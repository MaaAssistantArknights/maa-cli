use std::str::FromStr;

use anyhow::{Result, bail};
use clap::Subcommand;
use serde::Deserialize;

pub mod archive;
mod meta;
mod package;

/// Release channel for maa-cli.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Stable release
    Stable,
    /// Beta pre-release
    Beta,
    /// Alpha pre-release (nightly)
    Alpha,
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
}

#[derive(Subcommand)]
pub enum ReleaseCommands {
    /// Parse version and determine release metadata
    Meta,
    /// Package release archives and write manifest.json and manifest.txt
    Package,
}

pub fn run(command: ReleaseCommands) -> Result<()> {
    match command {
        ReleaseCommands::Meta => meta::run(),
        ReleaseCommands::Package => package::run(),
    }
}
