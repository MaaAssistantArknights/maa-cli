//! Module for managing the global state of the maa-cli.

use std::sync::LazyLock;

use semver::Version;
use ureq::{
    Agent, Body, SendBody,
    middleware::{Middleware, MiddlewareNext},
    tls::{RootCerts, TlsConfig},
};

use crate::{
    config::{FindFile, cli::CLIConfig},
    dirs,
};

pub const CLI_VERSION_STR: &str = env!("MAA_VERSION");

pub static CLI_VERSION: LazyLock<Version> =
    LazyLock::new(|| Version::parse(CLI_VERSION_STR).expect("CLI version string should be valid"));

pub static CORE_VERSION_STR: LazyLock<Option<String>> =
    LazyLock::new(|| crate::run::core_version().ok());

pub static CORE_VERSION: LazyLock<Option<Version>> = LazyLock::new(|| {
    CORE_VERSION_STR.as_deref().and_then(|version_str| {
        let version_str = version_str.strip_prefix("v").unwrap_or(version_str);
        Version::parse(version_str).ok()
    })
});

/// Rewrite a GitHub release download URL through a proxy.
///
/// Returns `Some(new_url)` if the URL is a GitHub release download
/// (`github.com/*/releases/download/*`), `None` otherwise.
/// Empty proxy is treated as no-op, returning `None`.
pub(crate) fn rewrite_github_url(url: &str, proxy: &str) -> Option<String> {
    let proxy = proxy.trim();
    if proxy.is_empty() {
        return None;
    }
    // Must be a github.com URL with /releases/download/ in the path
    if !url.contains("github.com/") || !url.contains("/releases/download/") {
        return None;
    }
    // Ensure it's the host, not just somewhere in the URL
    let rest = url.strip_prefix("https://github.com/")?;
    if !rest.contains("/releases/download/") {
        return None;
    }
    let proxy = proxy.trim_end_matches('/');
    Some(format!("{proxy}/{url}"))
}

/// GitHub proxy prefix, read lazily from the config file.
///
/// Unlike `CLI_CONFIG`, this never panics: if the config file fails to parse
/// (e.g. with a feature-gated field), the proxy is simply disabled.
static GITHUB_PROXY: LazyLock<Option<String>> = LazyLock::new(|| {
    CLIConfig::find_file_or_none(dirs::config().join("cli"))
        .ok()
        .flatten()
        .and_then(|cfg| cfg.github_proxy())
});

/// Apply the proxy prefix to a URI if it is a GitHub release download URL.
fn rewrite_uri(uri: &ureq::http::Uri, proxy: &str) -> Option<ureq::http::Uri> {
    rewrite_github_url(&uri.to_string(), proxy).and_then(|new_url| new_url.parse().ok())
}

/// Middleware that rewrites GitHub release download URLs through a proxy.
struct GitHubProxyMiddleware;

impl Middleware for GitHubProxyMiddleware {
    fn handle(
        &self,
        mut req: ureq::http::Request<SendBody>,
        next: MiddlewareNext,
    ) -> Result<ureq::http::Response<Body>, ureq::Error> {
        if let Some(proxy) = GITHUB_PROXY.as_deref()
            && let Some(uri) = rewrite_uri(req.uri(), proxy)
        {
            *req.uri_mut() = uri;
        }
        next.handle(req)
    }
}

pub static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    let mut config = Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent(format!("maa-cli/{CLI_VERSION_STR}"));

    if GITHUB_PROXY.is_some() {
        config = config.middleware(GitHubProxyMiddleware);
    }

    config.build().into()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_github_release_url() {
        let url = "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz";
        let result = rewrite_github_url(url, "https://hk.gh-proxy.org");
        assert_eq!(
            result,
            Some("https://hk.gh-proxy.org/https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz".to_string())
        );
    }

    #[test]
    fn rewrite_with_trailing_slash_proxy() {
        let url = "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz";
        let result = rewrite_github_url(url, "https://hk.gh-proxy.org/");
        assert_eq!(
            result,
            Some("https://hk.gh-proxy.org/https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz".to_string())
        );
    }

    #[test]
    fn non_github_url_unchanged() {
        let url = "https://example.com/file.tar.gz";
        assert_eq!(rewrite_github_url(url, "https://hk.gh-proxy.org"), None);
    }

    #[test]
    fn github_raw_url_unchanged() {
        let url = "https://github.com/MaaAssistantArknights/MaaRelease/raw/main/version.json";
        assert_eq!(rewrite_github_url(url, "https://hk.gh-proxy.org"), None);
    }

    #[test]
    fn empty_proxy_is_noop() {
        let url = "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz";
        assert_eq!(rewrite_github_url(url, ""), None);
    }

    #[test]
    fn rewrite_uri_github_release() {
        let uri: ureq::http::Uri = "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz"
            .parse()
            .unwrap();
        let result = rewrite_uri(&uri, "https://hk.gh-proxy.org");
        assert_eq!(
            result.map(|u| u.to_string()),
            Some("https://hk.gh-proxy.org/https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz".to_string())
        );
    }

    #[test]
    fn rewrite_uri_non_github() {
        let uri: ureq::http::Uri = "https://example.com/file.tar.gz".parse().unwrap();
        assert_eq!(rewrite_uri(&uri, "https://hk.gh-proxy.org"), None);
    }

    #[test]
    fn rewrite_uri_invalid_proxy_returns_none() {
        // Proxy that would produce an invalid URL (e.g. empty) yields no rewrite.
        let uri: ureq::http::Uri = "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz"
            .parse()
            .unwrap();
        assert_eq!(rewrite_uri(&uri, ""), None);
    }

    #[test]
    fn github_proxy_subprocess_parent() {
        // Run a child process with an isolated MAA_CONFIG_DIR so that the
        // process-wide GITHUB_PROXY/AGENT statics pick up a configured proxy.
        // Environment variables set here never leak into other tests.
        let exe = std::env::current_exe().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("cli.toml"),
            "github_proxy = \"https://hk.gh-proxy.org/\"\n",
        )
        .unwrap();
        let output = std::process::Command::new(exe)
            .env("MAA_GITHUB_PROXY_TEST", "1")
            .env("MAA_CONFIG_DIR", tmp.path())
            .args([
                "--exact",
                "state::tests::github_proxy_subprocess_child",
                "--nocapture",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "subprocess failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn github_proxy_subprocess_child() {
        // Only meaningful in the subprocess launched by the parent test;
        // in a normal test run this is a no-op.
        if std::env::var_os("MAA_GITHUB_PROXY_TEST").is_none() {
            return;
        }
        // GITHUB_PROXY resolves from the isolated config dir, and AGENT
        // installs the middleware when a proxy is configured.
        assert_eq!(GITHUB_PROXY.as_deref(), Some("https://hk.gh-proxy.org"));
        let _ = &*AGENT;
    }
}
