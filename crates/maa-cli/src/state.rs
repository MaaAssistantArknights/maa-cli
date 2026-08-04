//! Module for managing the global state of the maa-cli.

use std::sync::LazyLock;

use semver::Version;
use ureq::{
    Agent, Body, SendBody,
    middleware::{Middleware, MiddlewareNext},
    tls::{RootCerts, TlsConfig},
};

use crate::config::cli::CLI_CONFIG;

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

/// Middleware that rewrites GitHub release download URLs through a proxy.
struct GitHubProxyMiddleware {
    proxy: String,
}

impl Middleware for GitHubProxyMiddleware {
    fn handle(
        &self,
        mut req: ureq::http::Request<SendBody>,
        next: MiddlewareNext,
    ) -> Result<ureq::http::Response<Body>, ureq::Error> {
        let uri_str = req.uri().to_string();
        if let Some(new_url) = rewrite_github_url(&uri_str, &self.proxy)
            && let Ok(uri) = new_url.parse()
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

    if let Some(proxy) = CLI_CONFIG.github_proxy() {
        config = config.middleware(GitHubProxyMiddleware { proxy });
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
}
