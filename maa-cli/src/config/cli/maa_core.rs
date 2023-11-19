use super::{normalize_url, return_true, Channel};

use std::env::var_os;

use serde::Deserialize;

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
}

fn default_test_time() -> u64 {
    3
}

fn default_api_url() -> String {
    if let Some(url) = var_os("MAA_API_URL") {
        url.to_str().unwrap().to_owned()
    } else {
        "https://ota.maa.plus/MaaAssistantArknights/api/version/".to_owned()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    impl Config {
        pub fn with_channel(mut self, channel: Channel) -> Self {
            self.channel = channel;
            self
        }

        pub fn with_test_time(mut self, test_time: u64) -> Self {
            self.test_time = test_time;
            self
        }

        pub fn with_api_url(mut self, api_url: impl ToString) -> Self {
            self.api_url = api_url.to_string();
            self
        }
    }

    mod default {
        use super::*;

        use std::env::{remove_var, set_var};

        #[test]
        fn api_url() {
            assert_eq!(
                default_api_url(),
                "https://ota.maa.plus/MaaAssistantArknights/api/version/"
            );

            set_var("MAA_API_URL", "https://foo.bar/core/");
            assert_eq!(default_api_url(), "https://foo.bar/core/");
            remove_var("MAA_API_URL");
        }
    }

    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, Token};

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
                    Token::Map { len: Some(3) },
                    Token::Str("channel"),
                    Channel::Beta.as_token(),
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
            assert_eq!(Config::default().channel(), Channel::Stable);
            assert_eq!(
                Config::default().set_channel(Channel::Beta).channel(),
                Channel::Beta
            );
            assert_eq!(
                Config::default().set_channel(Channel::Alpha).channel(),
                Channel::Alpha
            );
        }

        #[test]
        fn api_url() {
            assert_eq!(
                Config::default().api_url(),
                "https://ota.maa.plus/MaaAssistantArknights/api/version/stable.json"
            );
            assert_eq!(
                Config::default().set_channel(Channel::Beta).api_url(),
                "https://ota.maa.plus/MaaAssistantArknights/api/version/beta.json"
            );
            assert_eq!(
                Config::default().set_channel(Channel::Alpha).api_url(),
                "https://ota.maa.plus/MaaAssistantArknights/api/version/alpha.json"
            );
            assert_eq!(
                Config::default()
                    .set_api_url("https://foo.bar/api/")
                    .api_url(),
                "https://foo.bar/api/stable.json"
            );
        }

        #[test]
        fn components() {
            assert!(matches!(
                Config::default()
                    .set_components(|components| components.library = false)
                    .components(),
                &Components { library: false, .. }
            ));
            assert!(matches!(
                Config::default()
                    .set_components(|components| components.resource = false)
                    .components(),
                &Components {
                    resource: false,
                    ..
                }
            ));
        }
    }
}
