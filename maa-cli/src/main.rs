mod config;
mod consts;
mod dirs;
mod installer;
mod log;
mod run;

use crate::{
    config::{
        cli::{self, Channel, InstallerConfig},
        FindFile,
    },
    log::{level, set_level},
};

#[cfg(feature = "cli_installer")]
use crate::installer::maa_cli;
#[cfg(feature = "core_installer")]
use crate::installer::maa_core;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};

#[derive(Parser)]
#[command(name = "maa", author, version)]
#[allow(clippy::upper_case_acronyms)]
struct CLI {
    #[command(subcommand)]
    command: SubCommand,
    /// Output more information, repeat to increase verbosity
    ///
    /// If you want to see more information, you can use this option to increase the log level.
    /// See documentation of log level for more information.
    #[arg(short, long, verbatim_doc_comment, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
    /// Output less information, repeat to increase quietness
    ///
    /// If you want to see less information, you can use this option to decrease the log level.
    /// See documentation of log level for more information.
    #[arg(short, long, verbatim_doc_comment, action = clap::ArgAction::Count, global = true)]
    quiet: u8,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Install maa maa_core and resources
    ///
    /// This command will install maa-core and resources
    /// by downloading prebuilt packages.
    /// Note: If the maa-core and resource are already installed,
    /// please update them by `maa-cli update`.
    /// Note: If you want to install maa-run, please use `maa-cli self install`.
    #[cfg(feature = "core_installer")]
    Install {
        #[command(flatten)]
        common: cli::maa_core::CommonArgs,
        /// Force to install even if the maa and resource already exists
        ///
        /// If the maa-core and resource already exists,
        /// we will not install them again by default.
        /// If you want to install them again, please use this flag.
        /// This flag is useful when the installation is failed,
        /// and you want to install them again.
        /// If you want to update the maa-core or resource,
        /// please use `maa-cli update` instead.
        #[arg(short, long)]
        force: bool,
    },
    /// Update maa maa_core and resources
    ///
    /// This command will update maa-core and resources
    /// by downloading prebuilt packages.
    /// If the version of maa-core is not newer,
    /// we will not update it.
    /// Note: If the maa-core and resource are not installed,
    /// please install them by `maa-cli install`.
    #[cfg(feature = "core_installer")]
    Update {
        #[command(flatten)]
        common: cli::maa_core::CommonArgs,
    },
    /// Manage maa-cli self and maa-run
    ///
    /// This command is used to manage maa-cli self and maa-run.
    /// Note: If you want to install or update maa-core and resource,
    /// please use `maa-cli install` or `maa-cli update` instead.
    #[cfg(feature = "cli_installer")]
    #[command(subcommand, name = "self")]
    SelfCommand(SelfCommand),
    /// Print path of maa directories
    ///
    /// This command will print the path used by maa-cli.
    /// Some of these paths are used by maa-core and maa-run.
    Dir { dir_type: Dir },
    /// Print version of given component
    ///
    /// This command will print the version of given component.
    /// If no component is given, it will print the version of all components.
    Version {
        #[arg(default_value_t = Component::All)]
        component: Component,
    },
    /// Run a predefined task
    ///
    /// All arguments will be passed to maa-run,
    /// type --help to get more information.
    /// The task is defined in the config directory of maa-cli,
    /// you can use `maa dir config` to get the path of config directory,
    /// and then create a directory named `tasks` in it.
    /// In the `tasks` directory, you can create a TOML or JSON file,
    /// to define a task. More information can be found in the README.
    /// You can also use `maa-cli list` to list all available tasks.
    Run {
        /// Name of the task to run
        ///
        /// The task name is the name of the task file without the extension.
        /// The task file must be in the `tasks` directory of the config directory.
        /// The task file must be in the TOML, YAML or JSON format.
        #[arg(verbatim_doc_comment)]
        task: String,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Run fight task
    Fight {
        /// Run startup task before the fight
        #[arg(long)]
        startup: bool,
        /// Close the game after the fight
        #[arg(long)]
        closedown: bool,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// List all available tasks
    List,
    /// Generate completion script for given shell
    Complete { shell: Shell },
}

#[derive(Subcommand)]
#[command(name = "self")]
enum SelfCommand {
    /// Update maa-cli self
    ///
    /// This command will download prebuilt binary of maa-cli,
    /// and install them to it current directory.
    Update {
        /// Channel to download prebuilt CLI binary
        ///
        /// There are two channels of maa-cli prebuilt binary,
        /// stable and alpha (which means nightly).
        channel: Option<Channel>,
        /// Url of api to get version information
        ///
        /// This flag is used to set the URL of api to get version information.
        /// Default to https://github.com/MaaAssistantArknights/maa-cli/raw/release/.
        #[arg(long)]
        api_url: Option<String>,
        /// Url of download to download prebuilt CLI binary
        ///
        /// This flag is used to set the URL of download to download prebuilt CLI binary.
        /// Default to https://github.com/MaaAssistantArknights/maa-cli/releases/download/.
        #[arg(long)]
        download_url: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Default)]
enum Component {
    #[default]
    All,
    MaaCLI,
    MaaCore,
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Component::All => write!(f, "all"),
            Component::MaaCLI => write!(f, "maa-cli"),
            Component::MaaCore => write!(f, "maa-core"),
        }
    }
}

#[derive(ValueEnum, Clone)]
pub enum Dir {
    /// Directory of maa-cli's data
    Data,
    /// Directory of maa-cli's dynamic library
    #[value(alias("lib"))]
    Library,
    /// Directory of maa-cli's config
    Config,
    /// Directory of maa-cli's cache
    Cache,
    /// Directory of MaaCore's resource
    Resource,
    /// Directory of MaaCore's log
    Log,
}

fn main() -> Result<()> {
    let cli = CLI::parse();

    let subcommand = cli.command;

    unsafe {
        set_level(level() as u8 + cli.verbose - cli.quiet);
    }

    match subcommand {
        #[cfg(feature = "core_installer")]
        SubCommand::Install { force, common } => {
            maa_core::install(force, &common)?;
        }
        #[cfg(feature = "core_installer")]
        SubCommand::Update { common } => {
            maa_core::update(&common)?;
        }
        #[cfg(feature = "cli_installer")]
        SubCommand::SelfCommand(self_command) => match self_command {
            SelfCommand::Update {
                channel,
                api_url,
                download_url,
            } => {
                let mut cli_config = InstallerConfig::find_file(&dirs::config().join("cli"))
                    .unwrap_or_default()
                    .cli_config();
                if let Some(channel) = channel {
                    cli_config.set_channel(channel);
                }
                if let Some(api_url) = api_url {
                    cli_config.set_api_url(api_url);
                }
                if let Some(download_url) = download_url {
                    cli_config.set_download_url(download_url);
                }
                maa_cli::update(&cli_config)?;
            }
        },
        SubCommand::Dir { dir_type } => match dir_type {
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
            Dir::Config => println!("{}", dirs::config().display()),
            Dir::Cache => println!("{}", dirs::cache().display()),
            Dir::Log => println!("{}", dirs::log().display()),
        },
        SubCommand::Version { component } => match component {
            Component::All => {
                println!("maa-cli v{}", env!("CARGO_PKG_VERSION"));
                println!("MaaCore {}", run::core_version()?);
            }
            Component::MaaCLI => {
                println!("maa-cli v{}", env!("CARGO_PKG_VERSION"));
            }
            Component::MaaCore => {
                println!("MaaCore {}", run::core_version()?);
            }
        },
        SubCommand::Run { task, common } => run::run(task, common)?,
        SubCommand::Fight {
            startup,
            closedown,
            common,
        } => run::fight(startup, closedown, common)?,
        SubCommand::List => {
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
        SubCommand::Complete { shell } => {
            generate(shell, &mut CLI::command(), "maa", &mut std::io::stdout());
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    mod parser {
        use super::*;

        #[test]
        fn log_level() {
            assert!(matches!(CLI::parse_from(["maa", "-v", "list"]).verbose, 1));
            assert!(matches!(CLI::parse_from(["maa", "list", "-v"]).verbose, 1));
            assert!(matches!(CLI::parse_from(["maa", "list", "-vv"]).verbose, 2));
            assert!(matches!(CLI::parse_from(["maa", "list", "-q"]).quiet, 1));
            assert!(matches!(CLI::parse_from(["maa", "list", "-qq"]).quiet, 2));
        }

        #[cfg(feature = "core_installer")]
        #[test]
        fn install() {
            assert!(matches!(
                CLI::parse_from(["maa", "install"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        channel: None,
                        test_time: None,
                        no_resource: false,
                        api_url: None,
                    },
                    force: false,
                }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "beta"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        channel: Some(Channel::Beta),
                        ..
                    },
                    ..
                }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "-t5"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        test_time: Some(5),
                        ..
                    },
                    ..
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "install", "--test-time", "5"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        test_time: Some(5),
                        ..
                    },
                    ..
                }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "--force"]).command,
                SubCommand::Install { force: true, .. }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "--no-resource"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        no_resource: true,
                        ..
                    },
                    ..
                }
            ));
        }

        #[cfg(feature = "core_installer")]
        #[test]
        fn update() {
            assert!(matches!(
                CLI::parse_from(["maa", "update"]).command,
                SubCommand::Update {
                    common: cli::maa_core::CommonArgs {
                        channel: None,
                        test_time: None,
                        no_resource: false,
                        api_url: None,
                    },
                }
            ));
        }

        #[cfg(feature = "cli_installer")]
        #[test]
        fn self_command() {
            assert!(matches!(
                CLI::parse_from(["maa", "self", "update"]).command,
                SubCommand::SelfCommand(SelfCommand::Update {
                    channel: None,
                    api_url: None,
                    download_url: None,
                })
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "self", "update", "beta"]).command,
                SubCommand::SelfCommand(SelfCommand::Update {
                    channel: Some(Channel::Beta),
                    ..
                })
            ));
        }

        #[test]
        fn dir() {
            assert!(matches!(
                CLI::parse_from(["maa", "dir", "data"]).command,
                SubCommand::Dir {
                    dir_type: Dir::Data
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "dir", "library"]).command,
                SubCommand::Dir {
                    dir_type: Dir::Library
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "dir", "lib"]).command,
                SubCommand::Dir {
                    dir_type: Dir::Library
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "dir", "config"]).command,
                SubCommand::Dir {
                    dir_type: Dir::Config
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "dir", "cache"]).command,
                SubCommand::Dir {
                    dir_type: Dir::Cache
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "dir", "resource"]).command,
                SubCommand::Dir {
                    dir_type: Dir::Resource
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "dir", "log"]).command,
                SubCommand::Dir { dir_type: Dir::Log }
            ));
        }

        #[test]
        fn version() {
            assert!(matches!(
                CLI::parse_from(["maa", "version"]).command,
                SubCommand::Version {
                    component: Component::All
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "version", "all"]).command,
                SubCommand::Version {
                    component: Component::All
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "version", "maa-cli"]).command,
                SubCommand::Version {
                    component: Component::MaaCLI
                }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "version", "maa-core"]).command,
                SubCommand::Version {
                    component: Component::MaaCore
                }
            ));
        }

        #[test]
        fn run() {
            assert!(matches!(
                CLI::parse_from(["maa", "run", "task"]).command,
                SubCommand::Run {
                    task,
                    common: run::CommonArgs {
                        addr: None,
                        user_resource: false,
                        batch: false,
                        dry_run: false,
                    },
                } if task == "task"
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "run", "task", "-a", "addr"]).command,
                SubCommand::Run {
                    task,
                    common: run::CommonArgs {
                        addr: Some(addr),
                        ..
                    },
                    ..
                } if task == "task" && addr == "addr"
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "run", "task", "--addr", "addr"]).command,
                SubCommand::Run {
                    task,
                    common: run::CommonArgs {
                        addr: Some(addr),
                        ..
                    },
                    ..
                } if task == "task" && addr == "addr"
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "run", "task", "--user-resource"]).command,
                SubCommand::Run {
                    task,
                    common: run::CommonArgs {
                        user_resource: true,
                        ..
                    },
                    ..
                } if task == "task"
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "run", "task", "--batch"]).command,
                SubCommand::Run {
                    task,
                    common: run::CommonArgs {
                        batch: true,
                        ..
                    },
                    ..
                } if task == "task"
            ));
        }

        #[test]
        fn fight() {
            assert!(matches!(
                CLI::parse_from(["maa", "fight"]).command,
                SubCommand::Fight {
                    startup: false,
                    closedown: false,
                    ..
                }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "fight", "--startup"]).command,
                SubCommand::Fight { startup: true, .. }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "fight", "--closedown"]).command,
                SubCommand::Fight {
                    closedown: true,
                    ..
                }
            ));
        }

        #[test]
        fn list() {
            assert!(matches!(
                CLI::parse_from(["maa", "list"]).command,
                SubCommand::List
            ));
        }

        #[test]
        fn complete() {
            assert!(matches!(
                CLI::parse_from(["maa", "complete", "bash"]).command,
                SubCommand::Complete { shell: Shell::Bash }
            ));
        }
    }
}
