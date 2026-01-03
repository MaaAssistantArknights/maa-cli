use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
};

pub mod archive;
pub(crate) mod meta;
pub(crate) mod package;

/// Release channel for maa-cli.
#[derive(Debug, Clone, Copy)]
pub enum Channel {
    /// Stable release
    Stable,
    /// Beta pre-release
    Beta,
    /// Alpha pre-release (nightly)
    Alpha,
}

impl<'de> Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ChannelVisitor;

        impl<'de> Visitor<'de> for ChannelVisitor {
            type Value = Channel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a channel name: stable, beta, or alpha")
            }

            fn visit_str<E>(self, value: &str) -> Result<Channel, E>
            where
                E: de::Error,
            {
                match value {
                    "stable" => Ok(Channel::Stable),
                    "beta" => Ok(Channel::Beta),
                    "alpha" => Ok(Channel::Alpha),
                    _ => Err(de::Error::unknown_variant(value, &[
                        "stable", "beta", "alpha",
                    ])),
                }
            }
        }

        deserializer.deserialize_str(ChannelVisitor)
    }
}

impl FromStr for Channel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "stable" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            "alpha" => Ok(Channel::Alpha),
            _ => bail!("Unknown channel: {s}"),
        }
    }
}

impl Channel {
    /// Get the channel name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Alpha => "alpha",
        }
    }

    /// Get the version file name for this channel.
    pub fn version_file(&self) -> String {
        let channel = self.as_str();
        format!("version/{channel}.json")
    }

    /// Get the list of version files to update for this channel.
    ///
    /// - Alpha: updates alpha.json
    /// - Beta: updates alpha.json and beta.json
    /// - Stable: updates all three (alpha.json, beta.json, stable.json)
    pub fn version_files(&self) -> &[&'static str] {
        match self {
            Channel::Alpha => &["version/alpha.json"],
            Channel::Beta => &["version/alpha.json", "version/beta.json"],
            Channel::Stable => &[
                "version/alpha.json",
                "version/beta.json",
                "version/stable.json",
            ],
        }
    }
}
