use clap::{Args, ValueEnum};
use serde::Deserialize;

use super::{normalize_url, return_true};

/// Release channel of maa-cli itself, independent of MaaCore's channels.
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(ValueEnum, Clone, Copy, Default, Deserialize)]
pub enum Channel {
    #[default]
    #[serde(alias = "stable")]
    Stable,
    #[serde(alias = "beta")]
    Beta,
    /// Daily builds of the dev branch.
    // `alpha` is the former name, kept so existing configs keep working.
    #[serde(alias = "nightly", alias = "Alpha", alias = "alpha")]
    #[value(alias = "alpha")]
    Nightly,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::Stable => write!(f, "stable"),
            Channel::Beta => write!(f, "beta"),
            Channel::Nightly => write!(f, "nightly"),
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    channel: Channel,
    #[serde(default = "default_api_url")]
    api_url: String,
    #[serde(default = "default_download_url")]
    download_url: String,
    #[serde(default)]
    components: CLIComponents,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            channel: Default::default(),
            api_url: default_api_url(),
            download_url: default_download_url(),
            components: Default::default(),
        }
    }
}

impl Config {
    pub fn channel(&self) -> Channel {
        self.channel
    }

    pub fn set_channel(&mut self, channel: Channel) -> &mut Self {
        self.channel = channel;
        self
    }

    pub fn api_url(&self) -> String {
        format!("{}/{}.json", normalize_url(&self.api_url), self.channel())
    }

    pub fn set_api_url(&mut self, api_url: impl ToString) -> &mut Self {
        self.api_url = api_url.to_string();
        self
    }

    pub fn download_url(&self) -> &str {
        normalize_url(&self.download_url)
    }

    pub fn set_download_url(&mut self, download_url: impl ToString) -> &mut Self {
        self.download_url = download_url.to_string();
        self
    }

    pub fn components(&self) -> &CLIComponents {
        &self.components
    }

    pub fn with_args(mut self, args: &CommonArgs) -> Self {
        if let Some(channel) = args.channel {
            self.set_channel(channel);
        }
        if let Some(api_url) = args.api_url.as_ref() {
            self.set_api_url(api_url);
        }
        if let Some(download_url) = args.download_url.as_ref() {
            self.set_download_url(download_url);
        }
        self
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Args, Default)]
pub struct CommonArgs {
    /// Channel to download prebuilt CLI binary
    ///
    /// There are three channels of maa-cli prebuilt binary: stable, beta and nightly.
    pub channel: Option<Channel>,
    /// Url of api to get version information
    ///
    /// This flag is used to set the URL of api to get version information.
    /// Default to <https://github.com/MaaAssistantArknights/maa-cli/raw/version/>.
    #[arg(long)]
    pub api_url: Option<String>,
    /// Url of download to download prebuilt CLI binary
    ///
    /// This flag is used to set the URL of download to download prebuilt CLI binary.
    /// Default to <https://github.com/MaaAssistantArknights/maa-cli/releases/download/>.
    #[arg(long)]
    pub download_url: Option<String>,
}

fn default_api_url() -> String {
    String::from("https://github.com/MaaAssistantArknights/maa-cli/raw/version/")
}

fn default_download_url() -> String {
    String::from("https://github.com/MaaAssistantArknights/maa-cli/releases/download/")
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct CLIComponents {
    #[serde(default = "return_true")]
    pub binary: bool,
}

impl Default for CLIComponents {
    fn default() -> Self {
        Self { binary: true }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod tests {
    use super::*;

    pub fn example_config() -> Config {
        Config {
            channel: Channel::Nightly,
            api_url: "https://cdn.jsdelivr.net/gh/MaaAssistantArknights/maa-cli@vversion/"
                .to_string(),
            download_url: "https://github.com/MaaAssistantArknights/maa-cli/releases/download/"
                .to_string(),
            components: CLIComponents { binary: false },
        }
    }

    mod serde {
        use serde_test::{Token, assert_de_tokens};

        use super::*;

        #[test]
        fn deserialize_channel() {
            let channels: [Channel; 5] =
                serde_json::from_str(r#"["stable", "beta", "nightly", "alpha", "Alpha"]"#).unwrap();
            assert_eq!(channels, [
                Channel::Stable,
                Channel::Beta,
                Channel::Nightly,
                Channel::Nightly,
                Channel::Nightly,
            ]);
            assert_eq!(Channel::from_str("alpha", true), Ok(Channel::Nightly));
        }

        #[test]
        fn display_channel() {
            assert_eq!(Channel::Stable.to_string(), "stable");
            assert_eq!(Channel::Beta.to_string(), "beta");
            assert_eq!(Channel::Nightly.to_string(), "nightly");
        }

        #[test]
        fn deserialize_cli_components() {
            assert_de_tokens(&CLIComponents { binary: true }, &[
                Token::Map { len: Some(0) },
                Token::MapEnd,
            ]);
            assert_de_tokens(&CLIComponents { binary: false }, &[
                Token::Map { len: Some(1) },
                Token::Str("binary"),
                Token::Bool(false),
                Token::MapEnd,
            ]);
        }

        #[test]
        fn deserialize_config() {
            assert_de_tokens(
                &Config {
                    channel: Channel::Nightly,
                    api_url: "https://foo.bar/api/".to_owned(),
                    download_url: "https://foo.bar/download/".to_owned(),
                    components: CLIComponents { binary: false },
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("channel"),
                    Token::UnitVariant {
                        name: "Channel",
                        variant: "Nightly",
                    },
                    Token::Str("api_url"),
                    Token::Str("https://foo.bar/api/"),
                    Token::Str("download_url"),
                    Token::Str("https://foo.bar/download/"),
                    Token::Str("components"),
                    Token::Map { len: Some(1) },
                    Token::Str("binary"),
                    Token::Bool(false),
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(&Config::default(), &[
                Token::Map { len: Some(0) },
                Token::MapEnd,
            ]);
        }
    }

    mod methods {
        use super::*;

        #[test]
        fn channel() {
            assert_eq!(Config::default().channel(), Default::default());
            assert_eq!(
                Config::default().set_channel(Channel::Nightly).channel(),
                Channel::Nightly,
            );
        }

        #[test]
        fn api_url() {
            assert_eq!(
                Config::default().api_url(),
                "https://github.com/MaaAssistantArknights/maa-cli/raw/version/stable.json",
            );

            assert_eq!(
                Config::default()
                    .set_api_url("https://foo.bar/cli/")
                    .api_url(),
                "https://foo.bar/cli/stable.json",
            );

            assert_eq!(
                Config {
                    channel: Channel::Nightly,
                    api_url: "https://foo.bar/cli/".to_string(),
                    ..Default::default()
                }
                .api_url(),
                "https://foo.bar/cli/nightly.json",
            );
        }

        #[test]
        fn download_url() {
            assert_eq!(
                Config::default().download_url(),
                "https://github.com/MaaAssistantArknights/maa-cli/releases/download",
            );

            assert_eq!(
                Config::default()
                    .set_download_url("https://foo.bar/download/")
                    .download_url(),
                "https://foo.bar/download",
            );
        }

        #[test]
        fn components() {
            assert_eq!(Config::default().components(), &CLIComponents {
                binary: true
            },);

            assert_eq!(
                Config {
                    components: CLIComponents { binary: false },
                    ..Default::default()
                }
                .components(),
                &CLIComponents { binary: false },
            );
        }

        #[test]
        fn with_args() {
            assert_eq!(
                Config::default().with_args(&CommonArgs {
                    channel: None,
                    api_url: None,
                    download_url: None,
                }),
                Config::default(),
            );

            assert_eq!(
                Config::default().with_args(&CommonArgs {
                    channel: Some(Channel::Nightly),
                    api_url: Some("https://foo.bar/api/".to_string()),
                    download_url: Some("https://foo.bar/download/".to_string()),
                }),
                Config {
                    channel: Channel::Nightly,
                    api_url: "https://foo.bar/api/".to_string(),
                    download_url: "https://foo.bar/download/".to_string(),
                    ..Default::default()
                },
            );
        }
    }
}
