use anyhow::Result;
use clap::{Parser, Subcommand};

mod env;
mod github;
mod release;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Task automation for maa-cli", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Release automation tasks
    Release {
        #[command(subcommand)]
        command: release::ReleaseCommands,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release { command } => release::run(command),
    }
}
