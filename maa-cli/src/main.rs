mod activity;
mod config;
mod consts;
mod dirs;
mod installer;
mod run;
mod value;

use crate::{
    config::cli, dirs::Ensure, installer::resource, run::preset,
    value::userinput::enable_batch_mode,
};

#[cfg(feature = "cli_installer")]
use crate::installer::maa_cli;
#[cfg(feature = "core_installer")]
use crate::installer::maa_core;

use std::{io::Write, path::PathBuf};

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
#[command(name = "maa", author, version, about = "A tool for Arknights.")]
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
    #[arg(long, global = true)]
    batch: bool,
    /// Redirect log to file instead of stderr
    ///
    /// If no log file is specified, the log will be written to
    /// `$(maa dir log)/YYYY/MM/DD/HH:MM:SS.log`.
    #[arg(long, global = true, require_equals = true)]
    log_file: Option<Option<PathBuf>>,
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
        task: String,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Startup Game and Enter Main Screen
    #[command(name = "startup")]
    StartUp {
        /// Client type of the game client
        ///
        /// The client type of the game client, used to launch the game client.
        /// If not specified, the client will not be launched.
        client: Option<config::task::ClientType>,
        /// Account name to switch to
        #[arg(long)]
        account: Option<String>,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Close game client
    #[command(name = "closedown")]
    CloseDown {
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Run fight task
    Fight {
        /// Stage to fight
        #[arg(default_value = "")]
        stage: String,
        /// medicine to use
        #[arg(short, long)]
        medicine: Option<i32>,
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
        #[arg(ignore_case = true)]
        theme: preset::RoguelikeTheme,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Convert file format between TOML, YAML and JSON
    ///
    /// This command will convert a file from TOML, YAML or JSON format to another format.
    /// This is useful when you want to write your infrastructure configuration
    /// in TOML or YAML format, and use it in MaaCore, which only supports JSON format.
    ///
    /// It may also be useful when you want to migrate your cli configuration from
    /// one format to another format.
    Convert {
        /// Path of the input file
        input: PathBuf,
        /// Path of the output file, if not specified, the output will be printed to stdout
        output: Option<PathBuf>,
        /// Format of the output file, can be one of "toml", "yaml" and "json"
        ///
        /// If not specified, the format will be guessed from the file extension of the output file.
        /// If output file is not specified, the output will be default to "json".
        #[arg(short, long)]
        format: Option<config::Filetype>,
    },
    /// Show stage activity of given client
    Activity {
        #[arg(default_value_t = config::task::ClientType::Official)]
        client: config::task::ClientType,
    },
    /// Get the remainder of given divisor and current date
    ///
    /// This command is used to calculate the value of remainder.
    /// Which is may helpful to fill remainder in task condition.
    Remainder {
        /// The value of divisor
        divisor: u32,
        /// Time zone of the date
        #[arg(long)]
        timezone: Option<i8>,
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

/// Whether or not to print log prefix [YYYY-MM-DD HH:MM:SS LEVEL]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Copy, Default)]
enum LogPrefix {
    /// Print log prefix if log to file, not print log prefix if log to stderr
    Auto,
    /// Always print log prefix
    #[default]
    Always,
    /// Never print log prefix
    Never,
}

impl LogPrefix {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "ALWAYS" | "Always" | "always" => Some(LogPrefix::Always),
            "NEVER" | "Never" | "never" => Some(LogPrefix::Never),
            "AUTO" | "Auto" | "auto" => Some(LogPrefix::Auto),
            _ => None,
        }
    }

    fn from_env() -> Self {
        std::env::var_os("MAA_LOG_PREFIX")
            .and_then(|s| s.to_str().and_then(LogPrefix::from_str))
            .unwrap_or_default()
    }

    fn format(
        &self,
        log_file: bool,
    ) -> fn(&mut env_logger::fmt::Formatter, &log::Record) -> std::io::Result<()> {
        match self {
            LogPrefix::Always => prefixed_format,
            LogPrefix::Never => plain_format,
            LogPrefix::Auto => {
                if log_file {
                    prefixed_format
                } else {
                    plain_format
                }
            }
        }
    }
}

fn prefixed_format(
    buf: &mut env_logger::fmt::Formatter,
    record: &log::Record,
) -> std::io::Result<()> {
    writeln!(
        buf,
        "[{} {:<5}] {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        buf.default_styled_level(record.level()),
        record.args()
    )
}

fn plain_format(buf: &mut env_logger::fmt::Formatter, record: &log::Record) -> std::io::Result<()> {
    writeln!(buf, "{}", record.args())
}

fn main() -> Result<()> {
    let cli = CLI::parse();

    let mut builder = env_logger::Builder::new();

    builder.filter_level(cli.verbose.log_level_filter());

    builder.format(LogPrefix::from_env().format(cli.log_file.is_some()));

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
            .open(log_file)?;

        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }

    builder.init();

    if cli.batch {
        enable_batch_mode()
    }

    match cli.command {
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
        SubCommand::StartUp {
            client,
            account,
            common,
        } => run::run(|_| preset::startup(client, account), common)?,
        SubCommand::CloseDown { common } => run::run(|_| preset::closedown(), common)?,
        SubCommand::Fight {
            stage,
            medicine,
            common,
        } => run::run(|_| preset::fight(stage, medicine), common)?,
        SubCommand::Copilot { uri, common } => run::run(
            |config| preset::copilot(uri, config.resource.base_dirs()),
            common,
        )?,
        SubCommand::Roguelike { theme, common } => run::run(|_| preset::roguelike(theme), common)?,
        SubCommand::Convert {
            input,
            output,
            format,
        } => config::convert(&input, output.as_deref(), format)?,
        SubCommand::Activity { client } => activity::display_stage_activity(client)?,
        SubCommand::Remainder { divisor, timezone } => {
            use crate::config::task::{remainder_of_day_mod, TimeOffset};
            println!(
                "{}",
                remainder_of_day_mod(
                    timezone.map(TimeOffset::TimeZone).unwrap_or_default(),
                    divisor
                )
            );
        }
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

    mod log_prefix {
        use super::*;

        #[test]
        fn from_str() {
            assert_eq!(LogPrefix::from_str("Always"), Some(LogPrefix::Always));
            assert_eq!(LogPrefix::from_str("always"), Some(LogPrefix::Always));
            assert_eq!(LogPrefix::from_str("NEVER"), Some(LogPrefix::Never));
            assert_eq!(LogPrefix::from_str("never"), Some(LogPrefix::Never));
            assert_eq!(LogPrefix::from_str("AUTO"), Some(LogPrefix::Auto));
            assert_eq!(LogPrefix::from_str("auto"), Some(LogPrefix::Auto));
            assert_eq!(LogPrefix::from_str("unknown"), None);
        }

        #[test]
        fn from_env() {
            std::env::remove_var("MAA_LOG_PREFIX");
            assert_eq!(LogPrefix::from_env(), LogPrefix::Always);

            std::env::set_var("MAA_LOG_PREFIX", "Always");
            assert_eq!(LogPrefix::from_env(), LogPrefix::Always);

            std::env::set_var("MAA_LOG_PREFIX", "Never");
            assert_eq!(LogPrefix::from_env(), LogPrefix::Never);

            std::env::set_var("MAA_LOG_PREFIX", "Auto");
            assert_eq!(LogPrefix::from_env(), LogPrefix::Auto);

            std::env::set_var("MAA_LOG_PREFIX", "unknown");
            assert_eq!(LogPrefix::from_env(), LogPrefix::Always);
        }

        #[test]
        fn format() {
            let pff = prefixed_format
                as fn(&mut env_logger::fmt::Formatter, &log::Record) -> std::io::Result<()>;
            let plf = plain_format
                as fn(&mut env_logger::fmt::Formatter, &log::Record) -> std::io::Result<()>;

            assert_eq!(LogPrefix::Always.format(true), pff);
            assert_eq!(LogPrefix::Always.format(false), pff);

            assert_eq!(LogPrefix::Never.format(true), plf);
            assert_eq!(LogPrefix::Never.format(false), plf);

            assert_eq!(LogPrefix::Auto.format(true), pff);
            assert_eq!(LogPrefix::Auto.format(false), plf);
        }
    }

    mod parser {
        use super::*;

        use crate::config::cli::Channel;
        use std::env;

        #[test]
        fn global_options() {
            let old = if let Some(val) = env::var_os("MAA_LOG") {
                env::remove_var("MAA_LOG");
                Some(val)
            } else {
                None
            };

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

            // Restore the environment variable
            if let Some(old) = old {
                env::set_var("MAA_LOG", old);
            }

            assert!(!CLI::parse_from(["maa", "list"]).batch);
            assert!(CLI::parse_from(["maa", "list", "--batch"]).batch);

            assert!(CLI::parse_from(["maa", "list"]).log_file.is_none());
            assert!(CLI::parse_from(["maa", "list", "--log-file"])
                .log_file
                .is_some_and(|x| x.is_none()));
            assert!(CLI::parse_from(["maa", "list", "--log-file=path"])
                .log_file
                .is_some_and(|x| x.is_some_and(|x| x == PathBuf::from("path"))));
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
        fn startup() {
            assert_matches!(
                CLI::parse_from(["maa", "startup"]).command,
                SubCommand::StartUp {
                    client: None,
                    account: None,
                    common: run::CommonArgs { .. },
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "startup", "YoStarEN"]).command,
                SubCommand::StartUp {
                    client: Some(client),
                    ..
                } if client == config::task::ClientType::YoStarEN
            );

            assert_matches!(
                CLI::parse_from(["maa", "startup", "YoStarEN", "--account", "account"]).command,
                SubCommand::StartUp {
                    client: Some(client),
                    account: Some(account),
                    ..
                } if client == config::task::ClientType::YoStarEN && account == "account"
            );
        }

        #[test]
        fn closedown() {
            assert_matches!(
                CLI::parse_from(["maa", "closedown"]).command,
                SubCommand::CloseDown {
                    common: run::CommonArgs { .. },
                }
            );
        }

        #[test]
        fn fight() {
            assert_matches!(
                CLI::parse_from(["maa", "fight", "1-7"]).command,
                SubCommand::Fight {
                    stage,
                    ..
                } if stage == "1-7"
            );

            assert_matches!(
                CLI::parse_from(["maa", "fight", "1-7", "-m", "1"]).command,
                SubCommand::Fight {
                    stage,
                    medicine: Some(medicine),
                    ..
                } if stage == "1-7" && medicine == 1
            );

            assert_matches!(
                CLI::parse_from(["maa", "fight", "1-7", "--medicine", "1"]).command,
                SubCommand::Fight {
                    stage,
                    medicine: Some(medicine),
                    ..
                } if stage == "1-7" && medicine == 1
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
                CLI::parse_from(["maa", "roguelike", "phantom"]).command,
                SubCommand::Roguelike {
                    theme,
                    ..
                } if matches!(theme, preset::RoguelikeTheme::Phantom)
            );
        }

        #[test]
        fn convert() {
            assert_matches!(
                CLI::parse_from(["maa", "convert", "input.toml"]).command,
                SubCommand::Convert {
                    input,
                    output: None,
                    format: None,
                } if input == PathBuf::from("input.toml")
            );

            assert_matches!(
                CLI::parse_from(["maa", "convert", "input.toml", "output.json"]).command,
                SubCommand::Convert {
                    output: Some(output),
                    ..
                } if output == PathBuf::from("output.json")
            );

            assert_matches!(
                CLI::parse_from(["maa", "convert", "input.toml", "--format", "json"]).command,
                SubCommand::Convert {
                    format: Some(config::Filetype::Json),
                    ..
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "convert", "input.toml", "output.json", "-fy"]).command,
                SubCommand::Convert {
                    output: Some(output),
                    format: Some(config::Filetype::Yaml),
                    ..
                } if output == PathBuf::from("output.json")
            );
        }

        #[test]
        fn activity() {
            assert_matches!(
                CLI::parse_from(["maa", "activity"]).command,
                SubCommand::Activity {
                    client: config::task::ClientType::Official,
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "activity", "YoStarEN"]).command,
                SubCommand::Activity {
                    client: config::task::ClientType::YoStarEN,
                }
            );
        }

        #[test]
        fn remainder() {
            assert_matches!(
                CLI::parse_from(["maa", "remainder", "3"]).command,
                SubCommand::Remainder {
                    divisor: 3,
                    timezone: None,
                }
            );

            assert_matches!(
                CLI::parse_from(["maa", "remainder", "3", "--timezone", "8"]).command,
                SubCommand::Remainder {
                    divisor: 3,
                    timezone: Some(8),
                }
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
