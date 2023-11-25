mod config;
mod dirs;
mod installer;
mod log;
mod run;

use crate::{
    config::{cli::CLIConfig, FindFile},
    installer::maa_core::{self, Channel, MaaCore},
    log::{level, set_level},
};

#[cfg(feature = "self")]
use crate::installer::maa_cli;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use directories::ProjectDirs;

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
    /// Install maa core and resources
    ///
    /// This command will install maa-core and resources
    /// by downloading prebuilt packages.
    /// Note: If the maa-core and resource are already installed,
    /// please update them by `maa-cli update`.
    /// Note: If you want to install maa-run, please use `maa-cli self install`.
    Install {
        /// Channel to download prebuilt package
        ///
        /// There are three channels of maa-core prebuilt packages,
        /// stable, beta and alpha.
        /// The default channel is stable, you can use this flag to change the channel.
        /// If you want to use the latest features of maa-core,
        /// you can use beta or alpha channel.
        /// You can also configure the default channel
        /// in the cli configure file `$MAA_CONFIG_DIR/cli.toml` with the key `core.channel`.
        /// Note: the alpha channel is only available for windows.
        channel: Option<Channel>,
        /// Time to test download speed
        ///
        /// There are several mirrors of maa-core prebuilt packages.
        /// This command will test the download speed of these mirrors,
        /// and choose the fastest one to download.
        /// This flag is used to set the time in seconds to test download speed.
        /// If test time is 0, speed test will be skipped.
        #[arg(short, long, default_value_t = 3)]
        test_time: u64,
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
        /// Do not install resource
        ///
        /// By default, resources are shipped with maa-core,
        /// and we will install them when installing maa-core.
        /// If you do not want to install resource,
        /// you can use this flag to disable it.
        /// You can also configure the default value in the cli configure file
        /// `$MAA_CONFIG_DIR/cli.toml` with the key `core.component.resource`;
        /// set it to false to disable installing resource by default.
        /// This is useful when you want to install maa-core only.
        /// For my own, I will use this flag to install maa-core,
        /// because I use the latest resource from github,
        /// and this flag can avoid the resource being overwritten.
        /// Note: if you use resources that too new or too old,
        /// you may encounter some problems.
        /// Use at your own risk.
        #[arg(long)]
        no_resource: bool,
    },
    /// Update maa core and resources
    ///
    /// This command will update maa-core and resources
    /// by downloading prebuilt packages.
    /// If the version of maa-core is not newer,
    /// we will not update it.
    /// Note: If the maa-core and resource are not installed,
    /// please install them by `maa-cli install`.
    Update {
        /// Channel to download prebuilt package
        ///
        /// There are three channels of maa-core prebuilt packages,
        /// stable, beta and alpha.
        /// The default channel is stable, you can use this flag to change the channel.
        /// If you want to use the latest features of maa-core,
        /// you can use beta or alpha channel.
        /// You can also configure the default channel
        /// in the cli configure file `$MAA_CONFIG_DIR/cli.toml` with the key `core.channel`.
        /// Note: the alpha channel is only available for windows.
        /// Note: if the maa-core is not installed, please use `maa-cli install` instead.
        /// And if the core is broken, please use `maa-cli install --force` to reinstall it.
        channel: Option<Channel>,
        /// Do not update resource
        ///
        /// By default, resources are shipped with maa-core,
        /// and we will update them when updating maa-core.
        /// If you do not want to update resource,
        /// you can use this flag to disable it.
        /// You can also configure the default value in the cli configure file
        /// `$MAA_CONFIG_DIR/cli.toml` with the key `core.component.resource`;
        /// set it to false to disable updating resource by default.
        /// This is useful when you want to update maa-core only.
        /// For my own, I will use this flag to update maa-core,
        /// because I use the latest resource from github,
        /// and this flag can avoid the resource being overwritten.
        /// Note: if you use resources that too new or too old,
        /// you may encounter some problems.
        /// Use at your own risk.
        #[arg(long)]
        no_resource: bool,
        /// Time to test download speed
        ///
        /// There are several mirrors of maa-core prebuilt packages.
        /// This command will test the download speed of these mirrors,
        /// and choose the fastest one to download.
        /// This flag is used to set the time in seconds to test download speed.
        /// If test time is 0, speed test will be skipped.
        #[arg(short, long, default_value_t = 3)]
        test_time: u64,
    },
    /// Manage maa-cli self and maa-run
    ///
    /// This command is used to manage maa-cli self and maa-run.
    /// Note: If you want to install or update maa-core and resource,
    /// please use `maa-cli install` or `maa-cli update` instead.
    #[cfg(feature = "self")]
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
    Update,
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
    let proj = ProjectDirs::from("com", "loong", "maa");
    let proj_dirs = dirs::Dirs::new(proj);

    let cli = CLI::parse();

    let subcommand = cli.command;

    unsafe {
        set_level(level() as u8 + cli.verbose - cli.quiet);
    }

    match subcommand {
        SubCommand::Install {
            channel,
            no_resource,
            test_time,
            force,
        } => {
            let cli_config =
                CLIConfig::find_file(&proj_dirs.config().join("cli")).unwrap_or_default();
            let channel = channel.unwrap_or_else(|| cli_config.channel());
            let no_resource = no_resource || !cli_config.resource();
            MaaCore::new(channel).install(&proj_dirs, force, no_resource, test_time)?;
        }
        SubCommand::Update {
            channel,
            no_resource,
            test_time,
        } => {
            let cli_config =
                CLIConfig::find_file(&proj_dirs.config().join("cli")).unwrap_or_default();
            let channel = channel.unwrap_or_else(|| cli_config.channel());
            let no_resource = no_resource || !cli_config.resource();
            MaaCore::new(channel).update(&proj_dirs, no_resource, test_time)?;
        }
        #[cfg(feature = "self")]
        SubCommand::SelfCommand(self_command) => match self_command {
            SelfCommand::Update => {
                maa_cli::update(&proj_dirs)?;
            }
        },
        SubCommand::Dir { dir_type } => match dir_type {
            Dir::Data => println!("{}", proj_dirs.data().display()),
            Dir::Library => {
                println!(
                    "{}",
                    maa_core::find_lib_dir(&proj_dirs)
                        .context("Library not found")?
                        .display()
                )
            }
            Dir::Config => println!("{}", proj_dirs.config().display()),
            Dir::Cache => println!("{}", proj_dirs.cache().display()),
            Dir::Resource => {
                println!(
                    "{}",
                    maa_core::find_resource(&proj_dirs)
                        .context("Resource not found")?
                        .display()
                )
            }
            Dir::Log => println!("{}", proj_dirs.log().display()),
        },
        SubCommand::Version { component } => match component {
            Component::All => {
                println!("maa-cli v{}", env!("CARGO_PKG_VERSION"));
                println!("MaaCore {}", run::core_version(&proj_dirs)?);
            }
            Component::MaaCLI => {
                println!("maa-cli v{}", env!("CARGO_PKG_VERSION"));
            }
            Component::MaaCore => {
                println!("MaaCore {}", run::core_version(&proj_dirs)?);
            }
        },
        SubCommand::Run { task, common } => run::run(&proj_dirs, task, common)?,
        SubCommand::Fight {
            startup,
            closedown,
            common,
        } => run::fight(&proj_dirs, startup, closedown, common)?,
        SubCommand::List => {
            let task_dir = proj_dirs.config().join("tasks");
            if !task_dir.exists() {
                println!("No tasks found");
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
            assert!(matches!(CLI::parse_from(["maa", "-v", "help"]).verbose, 1));
            assert!(matches!(CLI::parse_from(["maa", "help", "-v"]).verbose, 1));
            assert!(matches!(CLI::parse_from(["maa", "help", "-vv"]).verbose, 2));
            assert!(matches!(CLI::parse_from(["maa", "help", "-q"]).quiet, 1));
            assert!(matches!(CLI::parse_from(["maa", "help", "-qq"]).quiet, 2));
        }

        #[test]
        fn install() {
            assert!(matches!(
                CLI::parse_from(["maa", "install"]).command,
                SubCommand::Install { .. }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "beta"]).command,
                SubCommand::Install {
                    channel: Some(Channel::Beta),
                    ..
                }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "-t5"]).command,
                SubCommand::Install { test_time: 5, .. }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "install", "--test-time", "5"]).command,
                SubCommand::Install { test_time: 5, .. }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "--force"]).command,
                SubCommand::Install { force: true, .. }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "install", "--no-resource"]).command,
                SubCommand::Install {
                    no_resource: true,
                    ..
                }
            ));
        }

        #[test]
        fn update() {
            assert!(matches!(
                CLI::parse_from(["maa", "update"]).command,
                SubCommand::Update {
                    channel: None,
                    test_time: 3,
                    no_resource: false,
                }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "update", "beta"]).command,
                SubCommand::Update {
                    channel: Some(Channel::Beta),
                    ..
                }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "update", "-t5"]).command,
                SubCommand::Update { test_time: 5, .. }
            ));
            assert!(matches!(
                CLI::parse_from(["maa", "update", "--test-time", "5"]).command,
                SubCommand::Update { test_time: 5, .. }
            ));

            assert!(matches!(
                CLI::parse_from(["maa", "update", "--no-resource"]).command,
                SubCommand::Update {
                    no_resource: true,
                    ..
                }
            ));
        }

        #[test]
        #[cfg(feature = "self")]
        fn self_command() {
            assert!(matches!(
                CLI::parse_from(["maa", "self", "update"]).command,
                SubCommand::SelfCommand(SelfCommand::Update)
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
