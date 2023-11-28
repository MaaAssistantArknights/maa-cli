#[cfg(feature = "cli_installer")]
pub mod maa_cli;
#[cfg(feature = "core_installer")]
pub mod maa_core;

use clap::ValueEnum;
use serde::Deserialize;

/// Configuration for the CLI (cli.toml)
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default)]
pub struct InstallerConfig {
    /// MaaCore configuration
    #[cfg(feature = "core_installer")]
    #[serde(default)]
    core: maa_core::Config,
    #[cfg(feature = "cli_installer")]
    #[serde(default)]
    cli: maa_cli::Config,
    // TODO: Add `resource` field for separate resource updater
}

impl InstallerConfig {
    #[cfg(feature = "core_installer")]
    pub fn core_config(self) -> maa_core::Config {
        self.core
    }

    #[cfg(feature = "cli_installer")]
    pub fn cli_config(self) -> maa_cli::Config {
        self.cli
    }
}

impl super::FromFile for InstallerConfig {}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(ValueEnum, Clone, Copy, Default, Deserialize)]
pub enum Channel {
    #[default]
    #[serde(alias = "stable")]
    Stable,
    #[serde(alias = "beta")]
    Beta,
    #[serde(alias = "alpha")]
    Alpha,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::Stable => write!(f, "stable"),
            Channel::Beta => write!(f, "beta"),
            Channel::Alpha => write!(f, "alpha"),
        }
    }
}

fn return_true() -> bool {
    true
}

fn normalize_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_owned()
    } else {
        format!("{}/", url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json;
    use serde_test::{assert_de_tokens, Token};
    use toml;

    // The serde_de_token cannot deserialize "beta" to Channel::Beta
    // But it works in real implementation (serde_json::from_str)
    // So we have to use this workaround
    impl Channel {
        pub fn as_token(self) -> Token {
            Token::UnitVariant {
                name: "Channel",
                variant: match self {
                    Channel::Stable => "Stable",
                    Channel::Beta => "Beta",
                    Channel::Alpha => "Alpha",
                },
            }
        }
    }

    #[test]
    fn deserialize_channel() {
        let channels: [Channel; 3] =
            serde_json::from_str(r#"["stable", "beta", "alpha"]"#).unwrap();
        assert_eq!(channels, [Channel::Stable, Channel::Beta, Channel::Alpha],);

        assert_de_tokens(&Channel::Stable, &[Channel::Stable.as_token()]);
        assert_de_tokens(&Channel::Beta, &[Channel::Beta.as_token()]);
        assert_de_tokens(&Channel::Alpha, &[Channel::Alpha.as_token()]);
    }

    #[test]
    fn deserialize_installer_config() {
        assert_de_tokens(
            &InstallerConfig::default(),
            &[Token::Map { len: Some(0) }, Token::MapEnd],
        );

        #[cfg(feature = "core_installer")]
        assert_de_tokens(
            &InstallerConfig {
                core: maa_core::Config::default().with_channel(Channel::Alpha),
                ..Default::default()
            },
            &[
                Token::Map { len: Some(3) },
                Token::Str("core"),
                Token::Map { len: Some(1) },
                Token::Str("channel"),
                Channel::Alpha.as_token(),
                Token::MapEnd,
                Token::MapEnd,
            ],
        );

        #[cfg(feature = "cli_installer")]
        assert_de_tokens(
            &InstallerConfig {
                cli: maa_cli::Config::default().with_channel(Channel::Alpha),
                ..Default::default()
            },
            &[
                Token::Map { len: Some(2) },
                Token::Str("cli"),
                Token::Map { len: Some(1) },
                Token::Str("channel"),
                Channel::Alpha.as_token(),
                Token::MapEnd,
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn deserialize_example() {
        let config: InstallerConfig =
            toml::from_str(&std::fs::read_to_string("../config_examples/cli.toml").unwrap())
                .unwrap();

        let expect = InstallerConfig {
            #[cfg(feature = "core_installer")]
            core: maa_core::Config::default()
                .with_channel(Channel::Beta)
                .with_test_time(0)
                .with_api_url(
                    "https://github.com/MaaAssistantArknights/MaaRelease/raw/main/MaaAssistantArknights/api/version/"
                ),
            #[cfg(feature = "cli_installer")]
            cli: maa_cli::Config::default()
                .with_channel(Channel::Alpha)
                .with_api_url("https://cdn.jsdelivr.net/gh/MaaAssistantArknights/maa-cli@vversion/")
                .with_download_url(
                    "https://github.com/MaaAssistantArknights/maa-cli/releases/download/",
                )
                .with_components(maa_cli::CLIComponents { binary: false }),
        };

        assert_eq!(config, expect);
    }

    #[cfg(feature = "core_installer")]
    #[test]
    fn get_core_config() {
        assert_eq!(
            InstallerConfig::default().core_config(),
            maa_core::Config::default()
        );

        assert_eq!(
            &InstallerConfig {
                core: {
                    let mut config = maa_core::Config::default();
                    config.set_channel(Channel::Beta);
                    config
                },
                ..Default::default()
            }
            .core_config(),
            maa_core::Config::default().set_channel(Channel::Beta)
        );
    }

    #[cfg(feature = "cli_installer")]
    #[test]
    fn get_cli_config() {
        assert_eq!(
            InstallerConfig {
                cli: Default::default(),
                ..Default::default()
            }
            .cli_config(),
            maa_cli::Config::default(),
        );

        assert_eq!(
            InstallerConfig {
                cli: maa_cli::Config::default().with_channel(Channel::Alpha),
                ..Default::default()
            }
            .cli_config(),
            maa_cli::Config::default().with_channel(Channel::Alpha),
        );
    }

    #[test]
    fn normalize_url_test() {
        assert_eq!(normalize_url("https://foo.bar"), "https://foo.bar/");
        assert_eq!(normalize_url("https://foo.bar/"), "https://foo.bar/");
    }
}
