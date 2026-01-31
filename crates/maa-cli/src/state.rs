//! Module for managing the global state of the maa-cli.

use std::sync::LazyLock;

use semver::Version;
use ureq::{
    Agent,
    tls::{RootCerts, TlsConfig},
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

pub static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent(format!("maa-cli/{CLI_VERSION_STR}"))
        .build()
        .into()
});
