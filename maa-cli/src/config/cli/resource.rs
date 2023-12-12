use std::path::{Path, PathBuf};

use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    auto_update: bool,
    #[serde(default)]
    backend: GitBackend,
    #[serde(default)]
    remote: Remote,
}

impl Config {
    pub fn auto_update(&self) -> bool {
        self.auto_update
    }

    pub fn backend(&self) -> GitBackend {
        self.backend
    }

    pub fn remote(&self) -> &Remote {
        &self.remote
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum GitBackend {
    #[default]
    Git,
    #[cfg(feature = "git2")]
    Libgit2,
    // TODO: Backend gitoxide
    // The gitoxide don't not support merge yet
    // which is required to update resource
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct Remote {
    /// URL to resource repository
    #[serde(default = "default_url")]
    url: String,
    /// Branch of resource repository
    #[serde(default)]
    branch: Option<String>,
    /// SSH key to access resource repository when fetch from SSH
    #[serde(default)]
    ssh_key: Option<PathBuf>,
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            url: default_url(),
            branch: None,
            ssh_key: None,
        }
    }
}

fn default_url() -> String {
    String::from("https://github.com/MaaAssistantArknights/MaaResource.git")
}

impl Remote {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn ssh_key(&self) -> Option<&Path> {
        self.ssh_key.as_deref()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn example_config() -> Config {
        Config {
            auto_update: true,
            backend: GitBackend::Libgit2,
            remote: Remote {
                url: String::from("https://github.com/MaaAssistantArknights/MaaResource.git"),
                branch: Some(String::from("main")),
                ssh_key: None,
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
                backend: GitBackend::Git,
                remote: Remote {
                    url: default_url(),
                    branch: None,
                    ssh_key: None,
                }
            }
        );
    }

    mod serde {
        use super::*;
        use serde_test::{assert_de_tokens, Token};

        impl GitBackend {
            pub fn to_token(self) -> Token {
                Token::UnitVariant {
                    name: "GitBackend",
                    variant: match self {
                        GitBackend::Git => "git",
                        #[cfg(feature = "git2")]
                        GitBackend::Libgit2 => "libgit2",
                    },
                }
            }
        }

        #[test]
        fn backend() {
            assert_de_tokens(&GitBackend::Git, &[GitBackend::Git.to_token()]);

            #[cfg(feature = "git2")]
            assert_de_tokens(&GitBackend::Libgit2, &[GitBackend::Libgit2.to_token()]);
        }

        #[test]
        fn remote() {
            assert_de_tokens(
                &Remote::default(),
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            assert_de_tokens(
                &Remote {
                    url: String::from("http://gitee.com/MaaMirror/Resource.git"),
                    branch: Some(String::from("main")),
                    ssh_key: Some(PathBuf::from("~/.ssh/id_ed25519")),
                },
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("url"),
                    Token::Str("http://gitee.com/MaaMirror/Resource.git"),
                    Token::Str("branch"),
                    Token::Some,
                    Token::Str("main"),
                    Token::Str("ssh_key"),
                    Token::Some,
                    Token::Str("~/.ssh/id_ed25519"),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn config() {
            assert_de_tokens(
                &Config::default(),
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            assert_de_tokens(
                &Config {
                    auto_update: true,
                    backend: GitBackend::Git,
                    remote: Remote {
                        url: String::from("git@github.com:MaaAssistantArknights/MaaResource.git"),
                        branch: Some(String::from("main")),
                        ssh_key: Some(PathBuf::from("~/.ssh/id_ed25519")),
                    },
                },
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("auto_update"),
                    Token::Bool(true),
                    Token::Str("backend"),
                    GitBackend::Git.to_token(),
                    Token::Str("remote"),
                    Token::Map { len: Some(3) },
                    Token::Str("url"),
                    Token::Str("git@github.com:MaaAssistantArknights/MaaResource.git"),
                    Token::Str("branch"),
                    Token::Some,
                    Token::Str("main"),
                    Token::Str("ssh_key"),
                    Token::Some,
                    Token::Str("~/.ssh/id_ed25519"),
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            );
        }
    }

    #[test]
    fn url() {
        assert_eq!(
            Remote::default().url(),
            "https://github.com/MaaAssistantArknights/MaaResource.git",
        );

        assert_eq!(
            Remote {
                url: String::from("http://gitee.com/MaaMirror/Resource.git"),
                ..Default::default()
            }
            .url(),
            "http://gitee.com/MaaMirror/Resource.git"
        );
    }

    #[test]
    fn branch() {
        assert_eq!(Remote::default().branch(), None);

        assert_eq!(
            Remote {
                branch: Some(String::from("dev")),
                ..Default::default()
            }
            .branch()
            .unwrap(),
            "dev"
        );
    }

    #[test]
    fn ssh_key() {
        assert_eq!(Remote::default().ssh_key(), None);

        assert_eq!(
            Remote {
                ssh_key: Some(PathBuf::from("~/.ssh/id_ed25519")),
                ..Default::default()
            }
            .ssh_key()
            .unwrap(),
            Path::new("~/.ssh/id_ed25519")
        );
    }
}
