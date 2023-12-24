mod config;
mod consts;
mod dirs;
mod installer;
mod run;

use crate::{
    config::{cli, task::value::input::enable_batch_mode},
    dirs::Ensure,
    installer::resource,
};

#[cfg(feature = "cli_installer")]
use crate::installer::maa_cli;
#[cfg(feature = "core_installer")]
use crate::installer::maa_core;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use clap_verbosity_flag::{LogLevel, Verbosity};

struct EnvLevel;

impl LogLevel for EnvLevel {
    fn default() -> Option<log::Level> {
        std::env::var_os("MAA_LOG")
            .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
            .or(Some(log::Level::Warn))
    }
}

#[derive(Parser)]
#[command(name = "maa", author, version)]
#[allow(clippy::upper_case_acronyms)]
struct CLI {
    #[command(subcommand)]
    command: SubCommand,
    /// Enable batch mode
    ///
    /// If there are some input parameters in the task file,
    /// some prompts will be displayed to ask for input.
    /// In batch mode, the prompts will be skipped,
    /// and parameters will be set to default values.
    #[arg(long, verbatim_doc_comment, global = true)]
    batch: bool,
    /// Redirect log to file instead of stderr
    ///
    /// If no log file is specified, the log will be written to
    /// `$(maa dir log)/YYYY/MM/DD/HH:MM:SS.log`.
    #[arg(long, verbatim_doc_comment, global = true, require_equals = true)]
    log_file: Option<Option<std::path::PathBuf>>,
    #[command(flatten)]
    verbose: Verbosity<EnvLevel>,
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
    /// Manage maa-cli self
    ///
    /// This command is used to manage maa-cli self and maa-run.
    /// Note: If you want to install or update maa-core and resource,
    /// please use `maa-cli install` or `maa-cli update` instead.
    #[cfg(feature = "cli_installer")]
    #[command(subcommand, name = "self")]
    SelfCommand(SelfCommand),
    /// Hot update for resource
    ///
    /// This command will update hot updateable resource by fetch git repository MaaResource.
    /// Note: the basic resource installed with maa-core will not be updated.
    ///
    /// The remote of can be configured in the config file of maa-cli.
    HotUpdate,
    /// Print path of maa directories
    ///
    /// This command will print the path used by maa-cli.
    /// Some of these paths are used by maa-core and maa-run.
    Dir { dir: Dir },
    /// Print version of given component
    ///
    /// This command will print the version of given component.
    /// If no component is given, it will print the version of all components.
    Version {
        #[arg(default_value_t = Component::All)]
        component: Component,
    },
    /// Run a custom task
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
        /// Stage to fight
        stage: Option<String>,
        /// Run startup task before the fight
        #[arg(long)]
        startup: bool,
        /// Close the game after the fight
        #[arg(long)]
        closedown: bool,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Run copilot task
    Copilot {
        /// A code copied from "https://prts.plus" or a json file,
        /// such as "maa://12345" or "/your/json/path.json".
        uri: String,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Run rouge-like task
    Roguelike {
        /// Theme of the game
        ///
        /// The theme of the game, can be one of "Phantom", "Mizuki" and "Sami".
        /// If not specified, it will be asked in the game.
        theme: Option<run::RoguelikeTheme>,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// List all available tasks
    List,
    /// Generate completion script for given shell
    Complete { shell: Shell },
}

#[cfg(feature = "cli_installer")]
#[derive(Subcommand)]
#[command(name = "self")]
enum SelfCommand {
    /// Update maa-cli self
    ///
    /// This command will download prebuilt binary of maa-cli,
    /// and install them to it current directory.
    Update {
        #[command(flatten)]
        common: cli::maa_cli::CommonArgs,
    },
}

#[derive(ValueEnum, Clone, Default)]
enum Component {
    #[default]
    All,
    #[value(alias("cli"))]
    MaaCLI,
    #[value(alias("core"))]
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
    /// Directory of MaaCore's hot update
    HotUpdate,
    /// Directory of MaaCore's log
    Log,
}

fn main() -> Result<()> {
    let cli = CLI::parse();

    let mut builder = env_logger::Builder::new();

    builder.filter_level(cli.verbose.log_level_filter());

    if let Some(opt) = cli.log_file {
        let now = chrono::Local::now();
        let log_file = opt.unwrap_or_else(|| {
            let dir = dirs::log()
                .join(now.format("%Y").to_string())
                .join(now.format("%m").to_string())
                .join(now.format("%d").to_string());
            dir.ensure().unwrap();
            dir.join(format!("{}.log", now.format("%H:%M:%S")))
        });

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }

    builder.init();

    if cli.batch {
        unsafe { enable_batch_mode() };
    }

    let subcommand = cli.command;

    match subcommand {
        #[cfg(feature = "core_installer")]
        SubCommand::Install { force, common } => {
            maa_core::install(force, &common)?;
            resource::update(false)?;
        }
        #[cfg(feature = "core_installer")]
        SubCommand::Update { common } => {
            maa_core::update(&common)?;
            resource::update(false)?;
        }
        #[cfg(feature = "cli_installer")]
        SubCommand::SelfCommand(self_command) => match self_command {
            SelfCommand::Update { common } => maa_cli::update(&common)?,
        },
        SubCommand::HotUpdate => resource::update(false)?,
        SubCommand::Dir { dir } => match dir {
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
        SubCommand::Run { task, common } => run::run_custom(task, common)?,
        SubCommand::Fight {
            stage,
            startup,
            closedown,
            common,
        } => run::run(|_| run::fight(stage, startup, closedown), common)?,
        SubCommand::Copilot { uri, common } => run::run(
            |config| run::copilot(uri, config.resource.base_dirs()),
            common,
        )?,
        SubCommand::Roguelike { theme, common } => run::run(|_| run::roguelike(theme), common)?,
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

    #[macro_export]
    macro_rules! assert_matches {
        ($value:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {
            assert!(matches!($value, $pattern $(if $guard)?))
        };
    }

    mod parser {
        use super::*;

        use crate::config::cli::Channel;
        use std::env;

        #[test]
        fn global_options() {
            env::remove_var("MAA_LOG");

            assert_eq!(
                CLI::parse_from(["maa", "list"]).verbose.log_level_filter(),
                log::LevelFilter::Warn
            );
            assert_eq!(
                CLI::parse_from(["maa", "-v", "list"])
                    .verbose
                    .log_level_filter(),
                log::LevelFilter::Info
            );
            assert_eq!(
                CLI::parse_from(["maa", "list", "-v"])
                    .verbose
                    .log_level_filter(),
                log::LevelFilter::Info
            );
            assert_eq!(
                CLI::parse_from(["maa", "list", "--verbose"])
                    .verbose
                    .log_level_filter(),
                log::LevelFilter::Info
            );

            assert_eq!(
                CLI::parse_from(["maa", "list", "-vv"])
                    .verbose
                    .log_level_filter(),
                log::LevelFilter::Debug
            );

            assert_eq!(
                CLI::parse_from(["maa", "list", "-q"])
                    .verbose
                    .log_level_filter(),
                log::LevelFilter::Error
            );

            env::set_var("MAA_LOG", "Info");
            assert_eq!(
                CLI::parse_from(["maa", "list"]).verbose.log_level_filter(),
                log::LevelFilter::Info
            );
            env::remove_var("MAA_LOG");

            assert!(!CLI::parse_from(["maa", "list"]).batch);
            assert!(CLI::parse_from(["maa", "list", "--batch"]).batch);

            assert!(CLI::parse_from(["maa", "list"]).log_file.is_none());
            assert!(CLI::parse_from(["maa", "list", "--log-file"])
                .log_file
                .is_some_and(|x| x.is_none()));
            assert!(CLI::parse_from(["maa", "list", "--log-file=path"])
                .log_file
                .is_some_and(|x| x.is_some_and(|x| x == std::path::PathBuf::from("path"))));
        }

        #[cfg(feature = "core_installer")]
        #[test]
        fn install() {
            assert_matches!(
                CLI::parse_from(["maa", "install"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs { .. },
                    force: false,
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "install", "beta"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        channel: Some(Channel::Beta),
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "install", "--no-resource"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        no_resource: true,
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "install", "-t5"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        test_time: Some(5),
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "install", "--test-time", "5"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        test_time: Some(5),
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "install", "--api-url", "url"]).command,
                SubCommand::Install {
                    common: cli::maa_core::CommonArgs {
                        api_url: Some(url),
                        ..
                    },
                    ..
                } if url == "url"
            );

            assert!(matches!(
                CLI::parse_from(["maa", "install", "--force"]).command,
                SubCommand::Install { force: true, .. }
            ));
        }

        #[cfg(feature = "core_installer")]
        #[test]
        fn update() {
            assert_matches!(
                CLI::parse_from(["maa", "update"]).command,
                SubCommand::Update {
                    common: cli::maa_core::CommonArgs { .. },
                }
            );
        }

        #[cfg(feature = "cli_installer")]
        #[test]
        fn self_command() {
            assert_matches!(
                CLI::parse_from(["maa", "self", "update"]).command,
                SubCommand::SelfCommand(SelfCommand::Update { .. })
            );

            assert_matches!(
                CLI::parse_from(["maa", "self", "update", "beta"]).command,
                SubCommand::SelfCommand(SelfCommand::Update {
                    common: cli::maa_cli::CommonArgs {
                        channel: Some(Channel::Beta),
                        ..
                    },
                })
            );

            assert_matches!(
                CLI::parse_from(["maa", "self", "update", "--api-url", "url"]).command,
                SubCommand::SelfCommand(
                    SelfCommand::Update {
                        common: cli::maa_cli::CommonArgs {
                            api_url: Some(url),
                            ..
                        }
                    }
                ) if url == "url"
            );
        }

        #[test]
        fn dir() {
            assert_matches!(
                CLI::parse_from(["maa", "dir", "data"]).command,
                SubCommand::Dir { dir: Dir::Data }
            );
            assert_matches!(
                CLI::parse_from(["maa", "dir", "library"]).command,
                SubCommand::Dir { dir: Dir::Library }
            );
            assert_matches!(
                CLI::parse_from(["maa", "dir", "lib"]).command,
                SubCommand::Dir { dir: Dir::Library }
            );
            assert_matches!(
                CLI::parse_from(["maa", "dir", "config"]).command,
                SubCommand::Dir { dir: Dir::Config }
            );
            assert_matches!(
                CLI::parse_from(["maa", "dir", "cache"]).command,
                SubCommand::Dir { dir: Dir::Cache }
            );
            assert_matches!(
                CLI::parse_from(["maa", "dir", "resource"]).command,
                SubCommand::Dir { dir: Dir::Resource }
            );
            assert_matches!(
                CLI::parse_from(["maa", "dir", "hot-update"]).command,
                SubCommand::Dir {
                    dir: Dir::HotUpdate
                }
            );
            assert_matches!(
                CLI::parse_from(["maa", "dir", "log"]).command,
                SubCommand::Dir { dir: Dir::Log }
            );
        }

        #[test]
        fn version() {
            assert_matches!(
                CLI::parse_from(["maa", "version"]).command,
                SubCommand::Version {
                    component: Component::All
                }
            );
            assert_matches!(
                CLI::parse_from(["maa", "version", "all"]).command,
                SubCommand::Version {
                    component: Component::All
                }
            );
            assert_matches!(
                CLI::parse_from(["maa", "version", "maa-cli"]).command,
                SubCommand::Version {
                    component: Component::MaaCLI
                }
            );
            assert_matches!(
                CLI::parse_from(["maa", "version", "cli"]).command,
                SubCommand::Version {
                    component: Component::MaaCLI
                }
            );
            assert_matches!(
                CLI::parse_from(["maa", "version", "maa-core"]).command,
                SubCommand::Version {
                    component: Component::MaaCore
                }
            );
            assert_matches!(
                CLI::parse_from(["maa", "version", "core"]).command,
                SubCommand::Version {
                    component: Component::MaaCore
                }
            );
        }

        #[test]
        fn run() {
            assert_matches!(
                CLI::parse_from(["maa", "run", "task"]).command,
                SubCommand::Run {
                    task,
                    common: run::CommonArgs { .. },
                } if task == "task"
            );

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
        }

        #[test]
        fn fight() {
            assert_matches!(
                CLI::parse_from(["maa", "fight"]).command,
                SubCommand::Fight {
                    stage: None,
                    startup: false,
                    closedown: false,
                    ..
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "fight", "1-7"]).command,
                SubCommand::Fight {
                    stage: Some(stage),
                    ..
                } if stage == "1-7"
            );

            assert_matches!(
                CLI::parse_from(["maa", "fight", "--startup"]).command,
                SubCommand::Fight { startup: true, .. }
            );
            assert_matches!(
                CLI::parse_from(["maa", "fight", "--closedown"]).command,
                SubCommand::Fight {
                    closedown: true,
                    ..
                }
            );
        }

        #[test]
        fn copilot() {
            assert_matches!(
                CLI::parse_from(["maa", "copilot", "maa://12345"]).command,
                SubCommand::Copilot {
                    uri,
                    ..
                } if uri == "maa://12345"
            );

            assert_matches!(
                CLI::parse_from(["maa", "copilot", "/your/json/path.json"]).command,
                SubCommand::Copilot {
                    uri,
                    common: run::CommonArgs { .. },
                } if uri == "/your/json/path.json"
            );
        }

        #[test]
        fn rougelike() {
            assert_matches!(
                CLI::parse_from(["maa", "roguelike"]).command,
                SubCommand::Roguelike { theme: None, .. }
            );

            assert_matches!(
                CLI::parse_from(["maa", "roguelike", "phantom"]).command,
                SubCommand::Roguelike {
                    theme: Some(theme),
                    ..
                } if matches!(theme, run::RoguelikeTheme::Phantom)
            );
        }

        #[test]
        fn list() {
            assert_matches!(CLI::parse_from(["maa", "list"]).command, SubCommand::List);
        }

        #[test]
        fn complete() {
            assert_matches!(
                CLI::parse_from(["maa", "complete", "bash"]).command,
                SubCommand::Complete { shell: Shell::Bash }
            );
        }
    }
}
