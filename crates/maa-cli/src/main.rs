#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use maa_dirs as dirs;
#[macro_use(join)]
extern crate maa_dirs;

mod state;

mod log;

mod activity;
mod cleanup;
mod command;
mod config;
mod installer;
mod run;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};

use crate::command::{Cli, Command, Component, Dir};

fn main() -> Result<()> {
    let cli = command::Cli::parse();

    cli.log.init_logger()?;

    if cli.batch {
        maa_value::userinput::enable_batch_mode()
    }

    match cli.command {
        #[cfg(feature = "core_installer")]
        Command::Install { force, common } => {
            installer::maa_core::install(force, &common)?;
            installer::hot_update::update()?;
            installer::resource::update(false)?;
        }
        #[cfg(feature = "core_installer")]
        Command::Update { common } => {
            installer::maa_core::update(&common)?;
            installer::hot_update::update()?;
            installer::resource::update(false)?;
        }
        #[cfg(feature = "cli_installer")]
        Command::SelfC(self_c) => match self_c {
            command::SelfCommand::Update { common } => installer::maa_cli::update(&common)?,
        },
        Command::HotUpdate => {
            installer::hot_update::update()?;
            installer::resource::update(false)?;
        }
        Command::Dir { dir } => match dir {
            Dir::Data => println!("{}", dirs::data().display()),
            Dir::Library => {
                println!(
                    "{}",
                    dirs::find_library().context("Library not found")?.display()
                )
            }
            Dir::Resource => {
                println!(
                    "{}",
                    dirs::find_resource()
                        .context("Resource not found")?
                        .display()
                )
            }
            Dir::HotUpdate => println!("{}", dirs::maa_resource().display()),
            Dir::Config => println!("{}", dirs::config().display()),
            Dir::Cache => println!("{}", dirs::cache().display()),
            Dir::Log => println!("{}", dirs::log().display()),
        },
        Command::Version { component } => {
            match component {
                Component::All | Component::MaaCLI => {
                    println!("maa-cli v{}", state::CLI_VERSION_STR)
                }
                _ => {}
            }
            match component {
                Component::All | Component::MaaCore => println!(
                    "MaaCore {}",
                    state::CORE_VERSION_STR
                        .as_deref()
                        .context("Failed to get MaaCore version")?
                ),
                _ => {}
            }
        }
        Command::Run { task, common } => run::run_custom(task, common)?,
        Command::StartUp { params, common } => run::run_preset(params, common)?,
        Command::CloseDown { params, common } => run::run_preset(params, common)?,
        Command::Fight { params, common } => run::run_preset(params, common)?,
        Command::Roguelike { params, common } => run::run_preset(params, common)?,
        Command::Copilot { params, common } => run::run_preset(params, common)?,
        Command::SSSCopilot { params, common } => run::run_preset(params, common)?,
        Command::Reclamation { params, common } => run::run_preset(params, common)?,
        Command::Convert {
            input,
            output,
            format,
        } => config::convert(&input, output.as_deref(), format)?,
        Command::Activity { client } => activity::display_stage_activity(client)?,
        Command::Remainder { divisor, timezone } => {
            use crate::config::task::{TimeOffset, remainder_of_day_mod};
            println!(
                "{}",
                remainder_of_day_mod(
                    timezone.map(TimeOffset::TimeZone).unwrap_or_default(),
                    divisor
                )
            );
        }
        Command::Cleanup { targets } => cleanup::cleanup(&targets)?,
        Command::List => {
            let task_dir = dirs::config().join("tasks");
            if !task_dir.exists() {
                eprintln!("No tasks found");
            } else {
                for entry in task_dir.read_dir()? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        println!("{}", path.file_stem().unwrap().to_str().unwrap());
                    }
                }
            }
        }
        Command::Import {
            path,
            force,
            config_type,
        } => config::import(&path, force, &config_type)?,
        Command::Complete { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "maa", &mut std::io::stdout());
        }
        Command::Init {
            name,
            format,
            force,
        } => config::init::init(name, format, force)?,
        Command::Mangen { path } => {
            clap_mangen::generate_to(Cli::command(), path)?;
        }
    }

    Ok(())
}
