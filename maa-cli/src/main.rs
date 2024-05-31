#[macro_use]
mod dirs;

mod log;

mod activity;
mod cleanup;
mod command;
mod config;
mod installer;
mod run;
mod value;

use crate::command::{Command, Component, Dir, CLI};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};

fn main() -> Result<()> {
    let cli = command::CLI::parse();

    cli.log.init_logger()?;

    if cli.batch {
        value::userinput::enable_batch_mode()
    }

    match cli.command {
        #[cfg(feature = "core_installer")]
        Command::Install { force, common } => {
            installer::maa_core::install(force, &common)?;
            installer::resource::update(false)?;
        }
        #[cfg(feature = "core_installer")]
        Command::Update { common } => {
            installer::maa_core::update(&common)?;
            installer::resource::update(false)?;
        }
        #[cfg(feature = "cli_installer")]
        Command::SelfC(self_c) => match self_c {
            command::SelfCommand::Update { common } => installer::maa_cli::update(&common)?,
        },
        Command::HotUpdate => installer::resource::update(false)?,
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
            Dir::HotUpdate => println!("{}", dirs::hot_update().display()),
            Dir::Config => println!("{}", dirs::config().display()),
            Dir::Cache => println!("{}", dirs::cache().display()),
            Dir::Log => println!("{}", dirs::log().display()),
        },
        Command::Version { component } => match component {
            Component::All => {
                println!("maa-cli v{}", env!("MAA_VERSION"));
                println!("MaaCore {}", run::core_version()?);
            }
            Component::MaaCLI => {
                println!("maa-cli v{}", env!("MAA_VERSION"));
            }
            Component::MaaCore => {
                println!("MaaCore {}", run::core_version()?);
            }
        },
        Command::Run { task, common } => run::run_custom(task, common)?,
        Command::StartUp {
            client,
            account,
            common,
        } => run::run(|_| run::preset::startup(client, account), common)?,
        Command::CloseDown { common } => run::run(|_| run::preset::closedown(), common)?,
        Command::Fight {
            stage,
            medicine,
            common,
        } => run::run(|_| run::preset::fight(stage, medicine), common)?,
        Command::Copilot { uri, common } => run::run(
            |config| run::preset::copilot(uri, config.resource.base_dirs()),
            common,
        )?,
        Command::Roguelike { theme, common } => {
            run::run(|_| run::preset::roguelike(theme), common)?
        }
        Command::Depot { common } => run::run(|_| run::preset::depot(), common)?,
        Command::Operbox { common } => run::run(|_| run::preset::oper_box(), common)?,
        Command::Convert {
            input,
            output,
            format,
        } => config::convert(&input, output.as_deref(), format)?,
        Command::Activity { client } => activity::display_stage_activity(client)?,
        Command::Remainder { divisor, timezone } => {
            use crate::config::task::{remainder_of_day_mod, TimeOffset};
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
            clap_complete::generate(shell, &mut CLI::command(), "maa", &mut std::io::stdout());
        }
        Command::Init {
            name,
            format,
            force,
        } => config::init::init(name, format, force)?,
        Command::Mangen { path } => {
            clap_mangen::generate_to(CLI::command(), path)?;
        }
    }

    Ok(())
}
