use anyhow::Result;
use clap::Subcommand;

pub mod archive;
mod channels;
mod package;

#[derive(Subcommand)]
pub enum ReleaseCommands {
    /// Select index channels using CHANNEL, MANIFEST and INDEX_DIR
    Channels,
    /// Package release archives and write manifest.json and manifest.txt
    Package,
}

pub fn run(command: ReleaseCommands) -> Result<()> {
    match command {
        ReleaseCommands::Channels => channels::run(),
        ReleaseCommands::Package => package::run(),
    }
}
