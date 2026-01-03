use std::str::FromStr;

use anyhow::{Result, bail};
use clap::Subcommand;

pub mod archive;
mod meta;
mod package;

/// Release channel for maa-cli.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Subcommand)]
pub enum ReleaseCommands {
    /// Parse version and determine release metadata
    Meta,
    /// Update version.json files with release information
    Package,
}

pub fn run(command: ReleaseCommands) -> Result<()> {
    match command {
        ReleaseCommands::Meta => meta::run(),
        ReleaseCommands::Package => package::run(),
    }
}
