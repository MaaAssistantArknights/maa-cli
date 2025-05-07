use std::{borrow::Cow, path::PathBuf};

use maa_dirs::expand_tilde;
use serde::Deserialize;

use crate::value::userinput::{Input, UserInput};

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
            passphrase: Passphrase,
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
    SshKey {
        path: PathBuf,
        passphrase: Passphrase,
    },
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
                    .get()
                    .map_err(|e| git2::Error::from_str(&format!("Failed to get passphrase {e}")))?
                    .as_deref(),
            ),
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Default)]
pub enum Passphrase<Str = String> {
    /// No passphrase
    #[default]
    None,
    /// Prompt for passphrase
    ///
    /// This will not work in batch mode
    Prompt,
    /// Plain text passphrase
    ///
    /// This is not recommended for security reasons
    Plain(Str),
    /// From  Environment variable
    ///
    /// This is not recommended for security reasons
    Env(Str),
    /// Use a command to get passphrase
    ///
    /// A command that outputs the passphrase to stdout
    /// This is useful to fetch passphrase from password manager
    Command(Vec<Str>),
}

impl<'de> Deserialize<'de> for Passphrase {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        enum PassphraseField {
            Cmd,
            Env,
        }

        impl<'de> serde::Deserialize<'de> for PassphraseField {
            fn deserialize<D>(deserializer: D) -> Result<PassphraseField, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                struct PassphraseFieldVisitor;

                impl serde::de::Visitor<'_> for PassphraseFieldVisitor {
                    type Value = PassphraseField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`cmd` or `env`")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match v {
                            "cmd" => Ok(PassphraseField::Cmd),
                            "env" => Ok(PassphraseField::Env),
                            _ => Err(serde::de::Error::unknown_field(v, &["cmd", "env"])),
                        }
                    }
                }

                deserializer.deserialize_identifier(PassphraseFieldVisitor)
            }
        }

        struct PassphraseVisitor;

        impl<'de> serde::de::Visitor<'de> for PassphraseVisitor {
            type Value = Passphrase;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid passphrase, which must be a bool, string, or map")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v {
                    Ok(Passphrase::Prompt)
                } else {
                    Ok(Passphrase::None)
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Passphrase::Plain(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Passphrase::Plain(v))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let v = match map.next_key()? {
                    Some(PassphraseField::Cmd) => {
                        let cmd = map.next_value()?;
                        Passphrase::Command(cmd)
                    }
                    Some(PassphraseField::Env) => {
                        let name = map.next_value()?;
                        Passphrase::Env(name)
                    }
                    None => return Err(serde::de::Error::custom("`cmd` or `env` is required")),
                };

                if map.next_key::<PassphraseField>()?.is_some() {
                    return Err(serde::de::Error::custom("only one field is allowed"));
                }

                Ok(v)
            }
        }

        deserializer.deserialize_any(PassphraseVisitor)
    }
}

impl Passphrase {
    pub fn compatible_with_git(&self) -> bool {
        matches!(self, Passphrase::None | Passphrase::Prompt)
    }

    pub fn get(&self) -> std::io::Result<Option<Cow<str>>> {
        match self {
            Passphrase::None => Ok(None),
            Passphrase::Prompt => Input::<String>::new(None, Some("passphrase"))
                .value()
                .map(Cow::Owned)
                .map(Some),
            Passphrase::Plain(password) => Ok(Some(Cow::Borrowed(password))),
            Passphrase::Env(name) => std::env::var(name)
                .map(Cow::Owned)
                .map(Some)
                .map_err(std::io::Error::other),
            Passphrase::Command(cmd) => {
                let output = std::process::Command::new(&cmd[0])
                    .args(&cmd[1..])
                    .output()?;
                if output.status.success() {
                    let passphrase = std::str::from_utf8(&output.stdout)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    Ok(Some(Cow::Owned(passphrase.trim().to_owned())))
                } else {
                    Err(std::io::Error::other(
                        String::from_utf8(output.stderr).unwrap_or_default(),
                    ))
                }
            }
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
                    passphrase: Passphrase::Plain(String::from("password")),
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
        use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

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
                        passphrase: Passphrase::None,
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
                        passphrase: Passphrase::Plain(String::from("password")),
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
        fn passphrase() {
            assert_de_tokens(&Passphrase::Prompt, &[Token::Bool(true)]);

            assert_de_tokens(&Passphrase::None, &[Token::Bool(false)]);

            assert_de_tokens(&Passphrase::Plain(String::from("password")), &[Token::Str(
                "password",
            )]);

            assert_de_tokens(&Passphrase::Env(String::from("SSH_PASSPHRASE")), &[
                Token::Map { len: Some(1) },
                Token::Str("env"),
                Token::Str("SSH_PASSPHRASE"),
                Token::MapEnd,
            ]);

            assert_de_tokens(
                &Passphrase::Command(vec![String::from("get"), String::from("passphrase")]),
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("cmd"),
                    Token::Seq { len: Some(2) },
                    Token::Str("get"),
                    Token::Str("passphrase"),
                    Token::SeqEnd,
                    Token::MapEnd,
                ],
            );

            assert_de_tokens_error::<Passphrase>(
                &[Token::Map { len: Some(1) }, Token::Str("foo")],
                "unknown field `foo`, expected `cmd` or `env`",
            );

            assert_de_tokens_error::<Passphrase>(
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("cmd"),
                    Token::Seq { len: Some(2) },
                    Token::Str("get"),
                    Token::Str("passphrase"),
                    Token::SeqEnd,
                    Token::Str("env"),
                ],
                "only one field is allowed",
            );

            assert_de_tokens_error::<Passphrase>(
                &[Token::Map { len: Some(0) }, Token::MapEnd],
                "`cmd` or `env` is required",
            );

            assert_de_tokens_error::<Passphrase>(
                &[Token::I64(0)],
                "invalid type: integer `0`, \
                expected a valid passphrase, which must be a bool, string, or map",
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
                            passphrase: Passphrase::Plain(String::from("password")),
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

    #[test]
    fn passphrase() {
        assert!(!Passphrase::Plain(String::from("password")).compatible_with_git());
        assert!(Passphrase::Prompt.compatible_with_git());

        assert_eq!(Passphrase::None.get().unwrap(), None);

        assert_eq!(
            Passphrase::Plain(String::from("password")).get().unwrap(),
            Some(Cow::Borrowed("password"))
        );

        assert!(Passphrase::Env(String::from("MMA_TEST_SSH_PASSPHRASE"))
            .get()
            .is_err());

        std::env::set_var("MMA_TEST_SSH_PASSPHRASE", "password");
        assert_eq!(
            Passphrase::Env(String::from("MMA_TEST_SSH_PASSPHRASE"))
                .get()
                .unwrap()
                .unwrap(),
            "password"
        );
        std::env::remove_var("MMA_TEST_SSH_PASSPHRASE");

        assert_eq!(
            Passphrase::Command(vec![String::from("echo"), String::from("password")])
                .get()
                .unwrap()
                .unwrap(),
            "password"
        );
    }
}
