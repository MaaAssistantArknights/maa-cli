use std::path::PathBuf;

use maa_dirs::expand_tilde;
use serde::Deserialize;

use super::secret::Secret;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
pub struct Config {
    /// Automatically update resource every time
    #[serde(default)]
    auto_update: bool,
    /// Warn on update failure instead of exiting
    #[serde(default)]
    warn_on_update_failure: bool,
    /// Backend to use for resource update
    #[serde(default)]
    backend: GitBackend,
    /// Remote configuration for resource update
    #[serde(default)]
    remote: Remote,
}

impl Config {
    pub fn auto_update(&self) -> bool {
        self.auto_update
    }

    pub fn warn_on_update_failure(&self) -> bool {
        self.warn_on_update_failure
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
#[derive(Clone)]
pub struct Remote {
    /// URL to resource repository
    url: String,
    /// Branch of resource repository
    branch: Option<String>,
    /// Certificate to access resource repository
    certificate: Option<Certificate>,
}

impl<'de> Deserialize<'de> for Remote {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RemoteHelper {
            #[serde(default = "default_url")]
            url: String,
            #[serde(default)]
            branch: Option<String>,
            #[serde(default)]
            use_ssh_agent: bool,
            #[serde(default)]
            ssh_key: Option<PathBuf>,
            #[serde(default)]
            passphrase: Secret,
        }

        let helper = RemoteHelper::deserialize(deserializer)?;

        let certificate = match (helper.use_ssh_agent, helper.ssh_key, helper.passphrase) {
            (true, None, _) => Some(Certificate::SshAgent),
            (true, Some(_), _) => {
                log::warn!("Using ssh-agent to fetch certificate, no need to specify ssh_key");
                Some(Certificate::SshAgent)
            }
            (false, Some(path), passphrase) => Some(Certificate::SshKey { path, passphrase }),
            (false, None, _) => None,
        };

        Ok(Remote {
            url: helper.url,
            branch: helper.branch,
            certificate,
        })
    }
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            url: default_url(),
            branch: None,
            certificate: None,
        }
    }
}

fn default_url() -> String {
    String::from("https://github.com/MaaAssistantArknights/MaaResource.git")
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone)]
pub enum Certificate {
    /// Use certificate from ssh-agent
    SshAgent,
    /// Use given private key as certificate
    ///
    /// Note: when using git backend, the encrypted key will not work in batch mode
    /// because we can not pass the passphrase to git command and it will prompt for passphrase.
    /// Use ssh-agent in this case.
    SshKey { path: PathBuf, passphrase: Secret },
}

#[cfg(feature = "git2")]
impl Certificate {
    pub fn fetch(&self, username: &str) -> Result<git2::Cred, git2::Error> {
        match self {
            Certificate::SshAgent => git2::Cred::ssh_key_from_agent(username),
            Certificate::SshKey { path, passphrase } => git2::Cred::ssh_key(
                username,
                None,
                expand_tilde(path).as_ref(),
                passphrase
                    .get_with_desc("passphrase")
                    .map_err(|e| git2::Error::from_str(&format!("Failed to get passphrase {e}")))?
                    .as_deref(),
            ),
        }
    }
}

impl Remote {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn certificate(&self) -> Option<&Certificate> {
        self.certificate.as_ref()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod tests {
    use super::*;
    #[cfg(feature = "git2")]
    pub fn example_config() -> Config {
        Config {
            auto_update: true,
            warn_on_update_failure: true,
            backend: GitBackend::Libgit2,
            remote: Remote {
                url: String::from("https://github.com/MaaAssistantArknights/MaaResource.git"),
                branch: Some(String::from("main")),
                certificate: Some(Certificate::SshKey {
                    path: PathBuf::from("~/.ssh/id_ed25519"),
                    passphrase: Secret::Plain(String::from("password")),
                }),
            },
        }
    }

    #[test]
    fn default() {
        let config = Config::default();
        assert_eq!(config, Config {
            auto_update: false,
            warn_on_update_failure: false,
            backend: GitBackend::Git,
            remote: Remote {
                url: default_url(),
                branch: None,
                certificate: None,
            }
        });
    }

    #[test]
    fn getter() {
        let config = Config::default();
        assert!(!config.auto_update());
        assert!(!config.warn_on_update_failure());
        assert_eq!(config.backend(), GitBackend::Git);
        assert_eq!(config.remote().url(), default_url());
        assert_eq!(config.remote().branch(), None);
        assert_eq!(config.remote().certificate(), None);
    }

    mod serde {
        use serde_test::{Token, assert_de_tokens};

        use super::*;

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
            assert_de_tokens(&Remote::default(), &[
                Token::Map { len: Some(0) },
                Token::MapEnd,
            ]);

            assert_de_tokens(
                &Remote {
                    url: String::from("http://git.com/MaaMirror/Resource.git"),
                    branch: Some(String::from("main")),
                    certificate: None,
                },
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("url"),
                    Token::Str("http://git.com/MaaMirror/Resource.git"),
                    Token::Str("branch"),
                    Token::Some,
                    Token::Str("main"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Remote {
                    certificate: Some(Certificate::SshAgent),
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("use_ssh_agent"),
                    Token::Bool(true),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Remote {
                    certificate: None,
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("use_ssh_agent"),
                    Token::Bool(false),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Remote {
                    certificate: Some(Certificate::SshKey {
                        path: PathBuf::from("~/.ssh/id_ed25519"),
                        passphrase: Secret::None,
                    }),
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("ssh_key"),
                    Token::Some,
                    Token::Str("~/.ssh/id_ed25519"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Remote {
                    certificate: Some(Certificate::SshKey {
                        path: PathBuf::from("~/.ssh/id_ed25519"),
                        passphrase: Secret::Plain(String::from("password")),
                    }),
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("ssh_key"),
                    Token::Some,
                    Token::Str("~/.ssh/id_ed25519"),
                    Token::Str("passphrase"),
                    Token::Str("password"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Remote {
                    certificate: Some(Certificate::SshAgent),
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("use_ssh_agent"),
                    Token::Bool(true),
                    Token::Str("ssh_key"),
                    Token::Some,
                    Token::Str("~/.ssh/id_ed25519"),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn config() {
            assert_de_tokens(&Config::default(), &[
                Token::Map { len: Some(0) },
                Token::MapEnd,
            ]);

            assert_de_tokens(
                &Config {
                    auto_update: true,
                    warn_on_update_failure: true,
                    backend: GitBackend::Git,
                    remote: Remote {
                        url: String::from("git@github.com:MaaAssistantArknights/MaaResource.git"),
                        branch: Some(String::from("main")),
                        certificate: Some(Certificate::SshKey {
                            path: PathBuf::from("~/.ssh/id_ed25519"),
                            passphrase: Secret::Plain(String::from("password")),
                        }),
                    },
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("auto_update"),
                    Token::Bool(true),
                    Token::Str("warn_on_update_failure"),
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
                    Token::Str("passphrase"),
                    Token::Str("password"),
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            );
        }
    }

    #[test]
    fn url() {
        assert_eq!(Remote::default().url(), default_url());

        assert_eq!(
            Remote {
                url: String::from("http://git.com/MaaMirror/Resource.git"),
                ..Default::default()
            }
            .url(),
            "http://git.com/MaaMirror/Resource.git"
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
    fn certificate() {
        assert_eq!(Remote::default().certificate(), None);

        assert_eq!(
            Remote {
                certificate: Some(Certificate::SshAgent),
                ..Default::default()
            }
            .certificate(),
            Some(&Certificate::SshAgent)
        );
    }
}
