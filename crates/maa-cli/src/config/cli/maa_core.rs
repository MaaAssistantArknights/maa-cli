use clap::Args;
use serde::Deserialize;

use super::{Channel, normalize_url, return_true};

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    channel: Channel,
    #[serde(default = "default_test_time")]
    test_time: u64,
    #[serde(default = "default_api_url")]
    api_url: String,
    #[serde(default)]
    components: Components,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            channel: Default::default(),
            test_time: default_test_time(),
            api_url: default_api_url(),
            components: Default::default(),
        }
    }
}

impl Config {
    pub fn channel(&self) -> Channel {
        self.channel
    }

    pub fn set_channel(&mut self, channel: Channel) -> &Self {
        self.channel = channel;
        self
    }

    pub fn test_time(&self) -> u64 {
        self.test_time
    }

    pub fn set_test_time(&mut self, test_time: u64) -> &Self {
        self.test_time = test_time;
        self
    }

    pub fn api_url(&self) -> String {
        format!("{}{}.json", normalize_url(&self.api_url), self.channel())
    }

    pub fn set_api_url(&mut self, api_url: impl ToString) -> &Self {
        self.api_url = api_url.to_string();
        self
    }

    pub fn components(&self) -> &Components {
        &self.components
    }

    pub fn set_components(&mut self, f: impl FnOnce(&mut Components)) -> &Self {
        f(&mut self.components);
        self
    }

    pub fn apply_args(mut self, args: &CommonArgs) -> Self {
        if let Some(channel) = args.channel {
            self.set_channel(channel);
        }
        if let Some(test_time) = args.test_time {
            self.set_test_time(test_time);
        }
        if let Some(api_url) = &args.api_url {
            self.set_api_url(api_url);
        }
        if args.no_resource {
            self.set_components(|components| components.resource = false);
        }
        self
    }
}

fn default_test_time() -> u64 {
    3
}

fn default_api_url() -> String {
    String::from("https://ota.maa.plus/MaaAssistantArknights/api/version/")
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct Components {
    #[serde(default = "return_true")]
    pub library: bool,
    #[serde(default = "return_true")]
    pub resource: bool,
}

impl Default for Components {
    fn default() -> Self {
        Components {
            library: true,
            resource: true,
        }
    }
}

#[derive(Args, Default)]
pub struct CommonArgs {
    /// Channel to download prebuilt package
    ///
    /// There are three channels of maa-core prebuilt packages,
    /// stable, beta and alpha.
    /// The default channel is stable, you can use this flag to change the channel.
    /// If you want to use the latest features of maa-core,
    /// you can use beta or alpha channel.
    /// You can also configure the default channel
    /// in the cli configure file `$MAA_CONFIG_DIR/cli.toml` with the key `maa_core.channel`.
    /// Note: the alpha channel is only available for windows.
    pub channel: Option<Channel>,
    /// Do not install resource
    ///
    /// By default, resources are shipped with maa-core,
    /// and we will install them when installing maa-core.
    /// If you do not want to install resource,
    /// you can use this flag to disable it.
    /// You can also configure the default value in the cli configure file
    /// `$MAA_CONFIG_DIR/cli.toml` with the key `maa_core.component.resource`;
    /// set it to false to disable installing resource by default.
    /// This is useful when you want to install maa-core only.
    /// For my own, I will use this flag to install maa-core,
    /// because I use the latest resource from github,
    /// and this flag can avoid the resource being overwritten.
    /// Note: if you use resources that too new or too old,
    /// you may encounter some problems.
    /// Use at your own risk.
    #[arg(long)]
    pub no_resource: bool,
    /// Time to test download speed
    ///
    /// There are several mirrors of maa-core prebuilt packages.
    /// This command will test the download speed of these mirrors,
    /// and choose the fastest one to download.
    /// This flag is used to set the time in seconds to test download speed.
    /// If test time is 0, speed test will be skipped.
    #[arg(short, long)]
    pub test_time: Option<u64>,
    /// URL of api to get version information
    ///
    /// This flag is used to set the URL of api to get version information.
    /// It can also be changed by environment variable `MAA_API_URL`.
    #[arg(long)]
    pub api_url: Option<String>,
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod tests {
    use std::sync::LazyLock;

    use super::*;

    pub fn example_config() -> Config {
        Config {
            channel: Channel::Beta,
            test_time: 0,
            api_url: "https://github.com/MaaAssistantArknights/MaaRelease/raw/main/MaaAssistantArknights/api/version/".to_string(),
            components: Components {
                library: true,
                resource: true,
            },
        }
    }

    static DEFAULT_CONFIG: LazyLock<Config> = LazyLock::new(Config::default);
    fn default_config() -> Config {
        DEFAULT_CONFIG.clone()
    }

    mod serde {
        use serde_test::{Token, assert_de_tokens};

        use super::*;

        #[test]
        fn deserialize_components() {
            assert_de_tokens(
                &Components {
                    library: true,
                    resource: true,
                },
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );
            assert_de_tokens(
                &Components {
                    library: false,
                    resource: false,
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("library"),
                    Token::Bool(false),
                    Token::Str("resource"),
                    Token::Bool(false),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn deserialize_config() {
            assert_de_tokens(
                &Config {
                    channel: Default::default(),
                    test_time: default_test_time(),
                    api_url: default_api_url(),
                    components: Components {
                        library: true,
                        resource: true,
                    },
                },
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            assert_de_tokens(&*DEFAULT_CONFIG, &[
                Token::Map { len: Some(0) },
                Token::MapEnd,
            ]);

            assert_de_tokens(
                &Config {
                    channel: Channel::Beta,
                    test_time: 10,
                    api_url: "https://foo.bar/api/".to_owned(),
                    components: Components {
                        library: false,
                        resource: false,
                    },
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("channel"),
                    Channel::Beta.to_token(),
                    Token::Str("test_time"),
                    Token::I64(10),
                    Token::Str("api_url"),
                    Token::Str("https://foo.bar/api/"),
                    Token::Str("components"),
                    Token::Map { len: Some(2) },
                    Token::Str("library"),
                    Token::Bool(false),
                    Token::Str("resource"),
                    Token::Bool(false),
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            );
        }
    }

    mod methods {
        use super::*;

        #[test]
        fn channel() {
            assert_eq!(DEFAULT_CONFIG.channel(), Channel::Stable);
            assert_eq!(
                default_config().set_channel(Channel::Beta).channel(),
                Channel::Beta
            );
            assert_eq!(
                default_config().set_channel(Channel::Alpha).channel(),
                Channel::Alpha
            );
        }

        #[test]
        fn test_time() {
            assert_eq!(DEFAULT_CONFIG.test_time(), 3);
            assert_eq!(default_config().set_test_time(5).test_time(), 5);
        }

        #[test]
        fn api_url() {
            assert_eq!(
                default_config().set_channel(Channel::Stable).api_url(),
                "https://ota.maa.plus/MaaAssistantArknights/api/version/stable.json"
            );
            assert_eq!(
                default_config().set_channel(Channel::Beta).api_url(),
                "https://ota.maa.plus/MaaAssistantArknights/api/version/beta.json"
            );
            assert_eq!(
                default_config().set_channel(Channel::Alpha).api_url(),
                "https://ota.maa.plus/MaaAssistantArknights/api/version/alpha.json"
            );
            assert_eq!(
                default_config()
                    .set_api_url("https://foo.bar/api/")
                    .api_url(),
                "https://foo.bar/api/stable.json"
            );
        }

        #[test]
        fn components() {
            assert!(matches!(
                default_config()
                    .set_components(|components| components.library = false)
                    .components(),
                &Components { library: false, .. }
            ));
            assert!(matches!(
                default_config()
                    .set_components(|components| components.resource = false)
                    .components(),
                &Components {
                    resource: false,
                    ..
                }
            ));
        }

        #[test]
        fn apply_args() {
            fn apply_to_default(args: &CommonArgs) -> Config {
                default_config().apply_args(args)
            }

            assert_eq!(apply_to_default(&CommonArgs::default()), default_config());

            assert_eq!(
                &apply_to_default(&CommonArgs {
                    channel: Some(Channel::Beta),
                    ..Default::default()
                }),
                default_config().set_channel(Channel::Beta)
            );

            assert_eq!(
                &apply_to_default(&CommonArgs {
                    test_time: Some(5),
                    ..Default::default()
                }),
                default_config().set_test_time(5)
            );

            assert_eq!(
                &apply_to_default(&CommonArgs {
                    api_url: Some("https://foo.bar/core/".to_string()),
                    ..Default::default()
                }),
                default_config().set_api_url("https://foo.bar/core/")
            );

            assert_eq!(
                &apply_to_default(&CommonArgs {
                    no_resource: true,
                    ..Default::default()
                }),
                default_config().set_components(|components| {
                    components.resource = false;
                })
            );

            assert_eq!(
                apply_to_default(&CommonArgs {
                    channel: Some(Channel::Beta),
                    test_time: Some(5),
                    api_url: Some("https://foo.bar/maa_core/".to_string()),
                    no_resource: true,
                }),
                Config {
                    channel: Channel::Beta,
                    test_time: 5,
                    api_url: "https://foo.bar/maa_core/".to_string(),
                    components: Components {
                        resource: false,
                        ..Default::default()
                    },
                }
            );
        }
    }
}
