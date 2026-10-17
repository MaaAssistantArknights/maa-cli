use std::path::PathBuf;

#[cfg(feature = "git2")]
use maa_dirs::expand_tilde;
use serde::Deserialize;

use super::secret::Secret;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
#[serde(try_from = "RawResourceConfig")]
pub struct Config {
    auto_update: bool,
    warn_on_update_failure: bool,
    source: ResourceSource,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResourceConfig {
    #[serde(default)]
    auto_update: bool,
    #[serde(default)]
    warn_on_update_failure: bool,
    backend: Option<GitBackend>,
    remote: Option<Remote>,
    source: Option<ResourceSource>,
}

impl TryFrom<RawResourceConfig> for Config {
    type Error = &'static str;

    fn try_from(raw: RawResourceConfig) -> Result<Self, Self::Error> {
        if raw.source.is_some() && (raw.backend.is_some() || raw.remote.is_some()) {
            return Err(
                "resource.source cannot be mixed with legacy resource.backend/resource.remote",
            );
        }
        let source = raw.source.unwrap_or_else(|| {
            ResourceSource::Git(GitConfig {
                backend: raw.backend.unwrap_or_default(),
                remote: raw.remote.unwrap_or_default(),
            })
        });
        Ok(Self {
            auto_update: raw.auto_update,
            warn_on_update_failure: raw.warn_on_update_failure,
            source,
        })
    }
}

impl Config {
    pub fn auto_update(&self) -> bool {
        self.auto_update
    }

    pub fn warn_on_update_failure(&self) -> bool {
        self.warn_on_update_failure
    }

    pub fn source(&self) -> &ResourceSource {
        &self.source
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ResourceSource {
    Git(GitConfig),
    MirrorChyan(MirrorChyanConfig),
}

impl Default for ResourceSource {
    fn default() -> Self {
        Self::Git(GitConfig::default())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
pub struct GitConfig {
    #[serde(default)]
    pub backend: GitBackend,
    #[serde(flatten)]
    pub remote: Remote,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
pub struct MirrorChyanConfig {
    pub cdk: Secret,
    #[serde(default = "default_mirror_check_interval")]
    pub check_interval: u64,
}

fn default_mirror_check_interval() -> u64 {
    600
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
    pub fn example_config() -> Config {
        Config {
            auto_update: true,
            warn_on_update_failure: true,
            source: ResourceSource::Git(GitConfig {
                backend: GitBackend::Git,
                remote: Remote {
                    url: String::from("https://github.com/MaaAssistantArknights/MaaResource.git"),
                    branch: Some(String::from("main")),
                    certificate: Some(Certificate::SshKey {
                        path: PathBuf::from("~/.ssh/id_ed25519"),
                        passphrase: Secret::Plain(String::from("password")),
                    }),
                },
            }),
        }
    }

    // Fixture parsing unwraps assert known test inputs, not user configuration.
    fn git(config: &Config) -> &GitConfig {
        match config.source() {
            ResourceSource::Git(git) => git,
            ResourceSource::MirrorChyan(_) => panic!("expected Git backend"),
        }
    }

    #[test]
    fn default() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
        assert!(!config.auto_update());
        assert!(!config.warn_on_update_failure());
        assert_eq!(git(&config).backend, GitBackend::Git);
        assert_eq!(git(&config).remote, Remote::default());
        assert_eq!(
            toml::from_str::<Config>("[source]\ntype = 'git'").unwrap(),
            config
        );
    }

    #[test]
    fn mirrorchyan_source() {
        let config: Config = toml::from_str("[source]\ntype = 'mirrorchyan'\ncdk = false").unwrap();
        let ResourceSource::MirrorChyan(mirror) = config.source() else {
            panic!("expected MirrorChyan source")
        };
        assert_eq!(mirror.cdk, Secret::None);
        assert_eq!(mirror.check_interval, 600);
        let config: Config = toml::from_str(
            r#"[source]
            type = "mirrorchyan"
            cdk = false
            check_interval = 0"#,
        )
        .unwrap();
        assert!(
            matches!(config.source(), ResourceSource::MirrorChyan(mirror) if mirror.check_interval == 0)
        );
    }

    #[test]
    fn legacy_and_new_git_preserve_remote_and_authentication() {
        let remote = r#"
            url = "git@example.org:custom/resources.git"
            branch = "custom"
            ssh_key = "~/.ssh/test-key"
            passphrase = "secret"
        "#;
        let legacy: Config =
            toml::from_str(&format!("backend = 'git'\n[remote]\n{remote}")).unwrap();
        let new: Config = toml::from_str(&format!(
            "[source]\ntype = 'git'\nbackend = 'git'\n{remote}"
        ))
        .unwrap();
        assert_eq!(new, legacy);
        assert_eq!(git(&new).remote, Remote {
            url: "git@example.org:custom/resources.git".into(),
            branch: Some("custom".into()),
            certificate: Some(Certificate::SshKey {
                path: "~/.ssh/test-key".into(),
                passphrase: Secret::Plain("secret".into()),
            }),
        });
    }

    #[test]
    fn source_conflicts_with_explicit_legacy_fields() {
        for source in ["type = 'git'", "type = 'mirrorchyan'\ncdk = false"] {
            for legacy in ["backend = 'git'\n", "[remote]\n"] {
                let error =
                    toml::from_str::<Config>(&format!("{legacy}[source]\n{source}")).unwrap_err();
                assert!(error.to_string().contains("cannot be mixed"));
            }
        }
        for source in ["", "type = 'unknown'", "type = 'mirrorchyan'"] {
            assert!(toml::from_str::<Config>(&format!("[source]\n{source}")).is_err());
        }
    }

    #[test]
    fn shared_options_apply_to_both_sources_and_legacy_format() {
        for source in [
            "",
            "[source]\ntype = 'git'",
            "[source]\ntype = 'mirrorchyan'\ncdk = false",
        ] {
            let config: Config = toml::from_str(&format!(
                "auto_update = true\nwarn_on_update_failure = true\n{source}"
            ))
            .unwrap();
            assert!(config.auto_update());
            assert!(config.warn_on_update_failure());
        }
    }

    #[test]
    #[cfg(feature = "git2")]
    fn legacy_and_new_libgit2_are_identical() {
        let old: Config = toml::from_str("backend = 'libgit2'\n[remote]\nbranch = 'main'").unwrap();
        let new: Config =
            toml::from_str("[source]\ntype = 'git'\nbackend = 'libgit2'\nbranch = 'main'").unwrap();
        assert_eq!(old, new);
        assert_eq!(git(&new).backend, GitBackend::Libgit2);
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
                    source: ResourceSource::Git(GitConfig {
                        backend: GitBackend::Git,
                        remote: Remote {
                            url: String::from(
                                "git@github.com:MaaAssistantArknights/MaaResource.git",
                            ),
                            branch: Some(String::from("main")),
                            certificate: Some(Certificate::SshKey {
                                path: PathBuf::from("~/.ssh/id_ed25519"),
                                passphrase: Secret::Plain(String::from("password")),
                            }),
                        },
                    }),
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("auto_update"),
                    Token::Bool(true),
                    Token::Str("warn_on_update_failure"),
                    Token::Bool(true),
                    Token::Str("backend"),
                    Token::Some,
                    GitBackend::Git.to_token(),
                    Token::Str("remote"),
                    Token::Some,
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
