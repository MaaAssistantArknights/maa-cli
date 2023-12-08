use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    auto_update: bool,
    #[serde(default)]
    remote: Remote,
}

impl Config {
    pub fn auto_update(&self) -> bool {
        self.auto_update
    }

    pub fn remote(&self) -> &Remote {
        &self.remote
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct Remote {
    #[serde(default)]
    protocol: GitProtocol,
    /// Public key used for SSH protocol
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_owner")]
    owner: String,
    #[serde(default = "default_repo")]
    repo: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default = "default_branch")]
    branch: String,
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            protocol: GitProtocol::default(),
            host: default_host(),
            owner: default_owner(),
            repo: default_repo(),
            url: None,
            branch: default_branch(),
        }
    }
}

fn default_host() -> String {
    String::from("github.com")
}

fn default_owner() -> String {
    String::from("MaaAssistantArknights")
}

fn default_repo() -> String {
    String::from("MaaResource")
}

fn default_branch() -> String {
    String::from("main")
}

impl Remote {
    pub fn url(&self) -> String {
        self.url
            .as_ref()
            .map(|url| url.to_owned())
            .unwrap_or(self.protocol.url(&self.host, &self.owner, &self.repo))
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum GitProtocol {
    Http,
    #[default]
    Https,
    // Ssh, // there are some problems with certificate verification
}

impl GitProtocol {
    pub fn url(self, host: &str, owner: &str, repo: &str) -> String {
        match self {
            GitProtocol::Http => format!("http://{}/{}/{}.git", host, owner, repo),
            GitProtocol::Https => format!("https://{}/{}/{}.git", host, owner, repo),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn example_config() -> Config {
        Config {
            auto_update: true,
            remote: Remote {
                protocol: GitProtocol::Https,
                host: String::from("github.com"),
                owner: String::from("MaaAssistantArknights"),
                repo: String::from("MaaResource"),
                branch: String::from("main"),
                url: Some(String::from(
                    "git@github.com:MaaAssistantArknights/MaaResource.git",
                )),
            },
        }
    }

    #[test]
    fn default() {
        let config = Config::default();
        assert_eq!(
            config,
            Config {
                auto_update: false,
                remote: Remote {
                    protocol: GitProtocol::Https,
                    host: String::from("github.com"),
                    owner: String::from("MaaAssistantArknights"),
                    repo: String::from("MaaResource"),
                    url: None,
                    branch: String::from("main"),
                }
            }
        );
    }

    mod serde {
        use super::*;
        use serde_test::{assert_de_tokens, Token};

        impl GitProtocol {
            pub fn to_token(self) -> Token {
                Token::UnitVariant {
                    name: "GitProtocol",
                    variant: match self {
                        GitProtocol::Http => "http",
                        GitProtocol::Https => "https",
                    },
                }
            }
        }

        #[test]
        fn git_protocol() {
            assert_de_tokens(&GitProtocol::Http, &[GitProtocol::Http.to_token()]);
            assert_de_tokens(&GitProtocol::Https, &[GitProtocol::Https.to_token()]);
        }

        #[test]
        fn remote() {
            assert_de_tokens(
                &Remote::default(),
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            assert_de_tokens(
                &Remote {
                    protocol: GitProtocol::Http,
                    host: String::from("gitee.com"),
                    owner: String::from("MaaMirror"),
                    repo: String::from("Resource"),
                    url: Some(String::from("http://gitee.com/MaaMirror/Resource.git")),
                    branch: String::from("main"),
                },
                &[
                    Token::Map { len: Some(6) },
                    Token::Str("protocol"),
                    GitProtocol::Http.to_token(),
                    Token::Str("host"),
                    Token::Str("gitee.com"),
                    Token::Str("owner"),
                    Token::Str("MaaMirror"),
                    Token::Str("repo"),
                    Token::Str("Resource"),
                    Token::Str("url"),
                    Token::Some,
                    Token::Str("http://gitee.com/MaaMirror/Resource.git"),
                    Token::MapEnd,
                ],
            );
        }
    }

    #[test]
    fn url() {
        assert_eq!(
            Remote::default().url(),
            String::from("https://github.com/MaaAssistantArknights/MaaResource.git"),
        );

        assert_eq!(
            Remote {
                protocol: GitProtocol::Http,
                host: String::from("gitee.com"),
                owner: String::from("MaaMirror"),
                repo: String::from("Resource"),
                url: None,
                branch: String::from("main"),
            }
            .url(),
            String::from("http://gitee.com/MaaMirror/Resource.git")
        );

        assert_eq!(
            Remote {
                url: Some(String::from("http://gitee.com/MaaMirror/Resource.git")),
                ..Default::default()
            }
            .url(),
            String::from("http://gitee.com/MaaMirror/Resource.git")
        );
    }

    #[test]
    fn branch() {
        assert_eq!(Remote::default().branch(), "main");

        assert_eq!(
            Remote {
                branch: String::from("dev"),
                ..Default::default()
            }
            .branch(),
            "dev"
        );
    }
}
