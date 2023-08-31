use crate::installer::maa_core::Channel;

use serde::Deserialize;

/// Configuration for the CLI
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default)]
pub struct CLIConfig {
    /// MaaCore channel
    #[serde(default)]
    pub channel: Channel,
}

impl super::FromFile for CLIConfig {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_example() {
        let config: CLIConfig = toml::from_str(
            r#"
            channel = "beta"
            "#,
        )
        .unwrap();
        assert_eq!(
            config,
            CLIConfig {
                channel: Channel::Beta
            }
        );
    }

    #[test]
    fn deserialize_default() {
        let config: CLIConfig = toml::from_str("").unwrap();
        assert_eq!(
            config,
            CLIConfig {
                channel: Channel::default(),
            }
        );
    }
}
