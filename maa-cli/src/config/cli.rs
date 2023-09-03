use crate::installer::maa_core::Channel;

use serde::Deserialize;

/// Configuration for the CLI
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default)]
pub struct CLIConfig {
    /// DEPRECATED: Remove in the next breaking change
    #[serde(default)]
    pub channel: Option<Channel>,
    /// MaaCore configuration
    #[serde(default)]
    pub core: CoreConfig,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default)]
pub struct CoreConfig {
    #[serde(default)]
    channel: Channel,
    #[serde(default)]
    components: CoreComponents,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct CoreComponents {
    #[serde(default = "return_true")]
    resource: bool,
}

fn return_true() -> bool {
    true
}

impl Default for CoreComponents {
    fn default() -> Self {
        CoreComponents {
            resource: return_true(),
        }
    }
}

impl super::FromFile for CLIConfig {}

impl CLIConfig {
    pub fn channel(&self) -> Channel {
        if let Some(channel) = self.channel {
            println!(
                "\x1b[33mWARNING\x1b[0m: \
                The `channel` field in the CLI configuration is deprecated \
                and will be removed in the next breaking change. \
                Please use the `core.channel` field instead."
            );
            channel
        } else {
            println!(
                "OK: Using `core.channel` field in the CLI configuration: {}",
                self.core.channel
            );
            self.core.channel
        }
    }

    pub fn resource(&self) -> bool {
        self.core.components.resource
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_example() {
        let config: CLIConfig = toml::from_str(
            r#"
            [core]
            channel = "beta"
            [core.components]
            resource = false
            "#,
        )
        .unwrap();
        assert_eq!(
            config,
            CLIConfig {
                channel: None,
                core: CoreConfig {
                    channel: Channel::Beta,
                    components: CoreComponents { resource: false }
                }
            }
        );

        let config: CLIConfig = toml::from_str(
            r#"
            [core]
            channel = "beta"
            "#,
        )
        .unwrap();
        assert_eq!(
            config,
            CLIConfig {
                channel: None,
                core: CoreConfig {
                    channel: Channel::Beta,
                    components: CoreComponents { resource: true }
                }
            }
        );
    }

    #[test]
    fn deserialize_default() {
        let config: CLIConfig = toml::from_str("").unwrap();
        assert_eq!(
            config,
            CLIConfig {
                channel: None,
                core: CoreConfig {
                    channel: Channel::Stable,
                    components: CoreComponents { resource: true }
                }
            }
        );
    }

    #[test]
    fn get_channel() {
        let config = CLIConfig {
            channel: Some(Channel::Beta),
            core: CoreConfig {
                channel: Channel::Stable,
                components: CoreComponents { resource: true },
            },
        };
        assert_eq!(config.channel(), Channel::Beta);

        let config = CLIConfig {
            channel: None,
            core: CoreConfig {
                channel: Channel::Stable,
                components: CoreComponents { resource: true },
            },
        };
        assert_eq!(config.channel(), Channel::Stable);
    }
}
