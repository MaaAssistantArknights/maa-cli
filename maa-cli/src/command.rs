use crate::{cleanup, config, log, run};

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "maa", author, version = env!("MAA_VERSION"), about = "A tool for Arknights.")]
#[allow(clippy::upper_case_acronyms)]
pub(crate) struct CLI {
    #[command(subcommand)]
    pub(crate) command: Command,
    /// Enable batch mode
    ///
    /// If there are some input parameters in the task file,
    /// some prompts will be displayed to ask for input.
    /// In batch mode, the prompts will be skipped,
    /// and parameters will be set to default values.
    #[arg(long, global = true)]
    pub(crate) batch: bool,
    #[command(flatten)]
    pub(crate) log: log::Args,
}

#[derive(Subcommand)]
pub(crate) enum Command {
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
        common: config::cli::maa_core::CommonArgs,
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
        common: config::cli::maa_core::CommonArgs,
    },
    /// Manage maa-cli self
    ///
    /// This command is used to manage maa-cli self and maa-run.
    /// Note: If you want to install or update maa-core and resource,
    /// please use `maa-cli install` or `maa-cli update` instead.
    #[cfg(feature = "cli_installer")]
    #[command(subcommand, name = "self")]
    SelfC(SelfCommand),
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
        #[arg(default_value = "all")]
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
        /// A code copied from <https://prts.plus> or a json file,
        /// such as "maa://12345" or "/your/json/path.json".
        uri: String,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    /// Run rouge-like task
    Roguelike {
        /// Theme of the game
        #[arg(ignore_case = true)]
        theme: run::preset::RoguelikeTheme,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    Depot {
        #[command(flatten)]
        common: run::CommonArgs,
    },
    Operbox {
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
    /// Clearing the caches of maa-cli and maa core
    Cleanup {
        /// Specify the path for deletion
        targets: Vec<cleanup::CleanupTarget>,
    },
    /// List all available tasks
    List,
    /// Import configuration files
    Import {
        /// Path of the configuration file
        path: PathBuf,
        /// Force to import even if a file with the same name already exists
        #[arg(short, long)]
        force: bool,
        /// Type of the configuration file
        ///
        /// All possible values are listed below:
        ///
        /// - `task`: Task configuration file (default), used to define custom tasks;
        /// - `cli`: CLI configuration file, CLI related configuration;
        /// - `asst` or `profile`: MaaCore configuration file;
        /// - `infrast`: Infrastructure plan file;
        /// - `copilot` or `ssscopilot`: Copilot or SSSCopilot task file;
        /// - `resource`: user resource files.
        ///
        /// Other values are supported, but not recommended.
        /// It will be treated as a subdirectory of the config directory and show a warning message.
        /// If you think it is correct, please open an issue to let us know.
        #[arg(short = 't', long, default_value = "task", verbatim_doc_comment)]
        config_type: String,
    },
    /// Initialize configurations for maa-cli
    Init {
        /// Name of the profile
        ///
        /// The name of the profile to initialize.
        /// If not specified, the default profile will be initialized.
        #[arg(short, long)]
        name: Option<PathBuf>,
        /// Format of the configuration file
        ///
        /// The type of the configuration file to save can be one of "toml", "yaml" and "json".
        /// If not specified, default to "json".
        #[arg(short, long)]
        format: Option<config::Filetype>,
        /// Force to initialize even if the profile already exists
        #[arg(long)]
        force: bool,
    },
    /// Generate completion script for given shell
    Complete { shell: Shell },
    /// Generate man page
    Mangen {
        /// Path of the output file
        #[arg(long)]
        path: PathBuf,
    },
}

#[cfg(feature = "cli_installer")]
#[derive(Subcommand)]
#[command(name = "self")]
pub(crate) enum SelfCommand {
    /// Update maa-cli self
    ///
    /// This command will download prebuilt binary of maa-cli,
    /// and install them to it current directory.
    Update {
        #[command(flatten)]
        common: config::cli::maa_cli::CommonArgs,
    },
}

#[derive(ValueEnum, Clone, Default)]
pub(crate) enum Component {
    #[default]
    All,
    #[value(alias("cli"))]
    MaaCLI,
    #[value(alias("core"))]
    MaaCore,
}

#[derive(ValueEnum, Clone)]
pub(crate) enum Dir {
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

#[cfg(test)]
pub(crate) fn parse_from<I, T>(args: I) -> CLI
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    CLI::parse_from(args)
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::config::cli::Channel;

    #[macro_export]
    macro_rules! assert_matches {
        ($value:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {
            assert!(matches!($value, $pattern $(if $guard)?))
        };
    }

    #[test]
    fn batch() {
        assert!(!parse_from(["maa", "list"]).batch);
        assert!(parse_from(["maa", "list", "--batch"]).batch);
    }

    #[cfg(feature = "core_installer")]
    #[test]
    fn install() {
        assert_matches!(
            parse_from(["maa", "install"]).command,
            Command::Install {
                common: config::cli::maa_core::CommonArgs { .. },
                force: false,
            }
        );

        assert_matches!(
            parse_from(["maa", "install", "beta"]).command,
            Command::Install {
                common: config::cli::maa_core::CommonArgs {
                    channel: Some(Channel::Beta),
                    ..
                },
                ..
            }
        );

        assert_matches!(
            parse_from(["maa", "install", "--no-resource"]).command,
            Command::Install {
                common: config::cli::maa_core::CommonArgs {
                    no_resource: true,
                    ..
                },
                ..
            }
        );

        assert_matches!(
            parse_from(["maa", "install", "-t5"]).command,
            Command::Install {
                common: config::cli::maa_core::CommonArgs {
                    test_time: Some(5),
                    ..
                },
                ..
            }
        );

        assert_matches!(
            parse_from(["maa", "install", "--test-time", "5"]).command,
            Command::Install {
                common: config::cli::maa_core::CommonArgs {
                    test_time: Some(5),
                    ..
                },
                ..
            }
        );

        assert_matches!(
            parse_from(["maa", "install", "--api-url", "url"]).command,
            Command::Install {
                common: config::cli::maa_core::CommonArgs {
                    api_url: Some(url),
                    ..
                },
                ..
            } if url == "url"
        );

        assert!(matches!(
            parse_from(["maa", "install", "--force"]).command,
            Command::Install { force: true, .. }
        ));
    }

    #[cfg(feature = "core_installer")]
    #[test]
    fn update() {
        assert_matches!(
            parse_from(["maa", "update"]).command,
            Command::Update {
                common: config::cli::maa_core::CommonArgs { .. },
            }
        );
    }

    #[cfg(feature = "cli_installer")]
    #[test]
    fn self_command() {
        assert_matches!(
            parse_from(["maa", "self", "update"]).command,
            Command::SelfC(SelfCommand::Update { .. })
        );

        assert_matches!(
            parse_from(["maa", "self", "update", "beta"]).command,
            Command::SelfC(SelfCommand::Update {
                common: config::cli::maa_cli::CommonArgs {
                    channel: Some(Channel::Beta),
                    ..
                },
            })
        );

        assert_matches!(
            parse_from(["maa", "self", "update", "--api-url", "url"]).command,
            Command::SelfC(
                SelfCommand::Update {
                    common: config::cli::maa_cli::CommonArgs {
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
            parse_from(["maa", "dir", "data"]).command,
            Command::Dir { dir: Dir::Data }
        );
        assert_matches!(
            parse_from(["maa", "dir", "library"]).command,
            Command::Dir { dir: Dir::Library }
        );
        assert_matches!(
            parse_from(["maa", "dir", "lib"]).command,
            Command::Dir { dir: Dir::Library }
        );
        assert_matches!(
            parse_from(["maa", "dir", "config"]).command,
            Command::Dir { dir: Dir::Config }
        );
        assert_matches!(
            parse_from(["maa", "dir", "cache"]).command,
            Command::Dir { dir: Dir::Cache }
        );
        assert_matches!(
            parse_from(["maa", "dir", "resource"]).command,
            Command::Dir { dir: Dir::Resource }
        );
        assert_matches!(
            parse_from(["maa", "dir", "hot-update"]).command,
            Command::Dir {
                dir: Dir::HotUpdate
            }
        );
        assert_matches!(
            parse_from(["maa", "dir", "log"]).command,
            Command::Dir { dir: Dir::Log }
        );
    }

    #[test]
    fn version() {
        assert_matches!(
            parse_from(["maa", "version"]).command,
            Command::Version {
                component: Component::All
            }
        );
        assert_matches!(
            parse_from(["maa", "version", "all"]).command,
            Command::Version {
                component: Component::All
            }
        );
        assert_matches!(
            parse_from(["maa", "version", "maa-cli"]).command,
            Command::Version {
                component: Component::MaaCLI
            }
        );
        assert_matches!(
            parse_from(["maa", "version", "cli"]).command,
            Command::Version {
                component: Component::MaaCLI
            }
        );
        assert_matches!(
            parse_from(["maa", "version", "maa-core"]).command,
            Command::Version {
                component: Component::MaaCore
            }
        );
        assert_matches!(
            parse_from(["maa", "version", "core"]).command,
            Command::Version {
                component: Component::MaaCore
            }
        );
    }

    #[test]
    fn run() {
        assert_matches!(
            parse_from(["maa", "run", "task"]).command,
            Command::Run {
                task,
                common: run::CommonArgs { .. },
            } if task == "task"
        );

        assert!(matches!(
            parse_from(["maa", "run", "task", "-a", "addr"]).command,
            Command::Run {
                task,
                common: run::CommonArgs {
                    addr: Some(addr),
                    ..
                },
                ..
            } if task == "task" && addr == "addr"
        ));
        assert!(matches!(
            parse_from(["maa", "run", "task", "--addr", "addr"]).command,
            Command::Run {
                task,
                common: run::CommonArgs {
                    addr: Some(addr),
                    ..
                },
                ..
            } if task == "task" && addr == "addr"
        ));

        assert!(matches!(
            parse_from(["maa", "run", "task", "--user-resource"]).command,
            Command::Run {
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
            parse_from(["maa", "startup"]).command,
            Command::StartUp {
                client: None,
                account: None,
                common: run::CommonArgs { .. },
            }
        );

        assert_matches!(
            parse_from(["maa", "startup", "YoStarEN"]).command,
            Command::StartUp {
                client: Some(client),
                ..
            } if client == config::task::ClientType::YoStarEN
        );

        assert_matches!(
            parse_from(["maa", "startup", "YoStarEN", "--account", "account"]).command,
            Command::StartUp {
                client: Some(client),
                account: Some(account),
                ..
            } if client == config::task::ClientType::YoStarEN && account == "account"
        );
    }

    #[test]
    fn closedown() {
        assert_matches!(
            parse_from(["maa", "closedown"]).command,
            Command::CloseDown {
                common: run::CommonArgs { .. }
            }
        );
    }

    #[test]
    fn fight() {
        assert_matches!(
            parse_from(["maa", "fight", "1-7"]).command,
            Command::Fight {
                stage,
                ..
            } if stage == "1-7"
        );

        assert_matches!(
            parse_from(["maa", "fight", "1-7", "-m", "1"]).command,
            Command::Fight {
                stage,
                medicine: Some(medicine),
                ..
            } if stage == "1-7" && medicine == 1
        );

        assert_matches!(
            parse_from(["maa", "fight", "1-7", "--medicine", "1"]).command,
            Command::Fight {
                stage,
                medicine: Some(medicine),
                ..
            } if stage == "1-7" && medicine == 1
        );
    }

    #[test]
    fn copilot() {
        assert_matches!(
            parse_from(["maa", "copilot", "maa://12345"]).command,
            Command::Copilot {
                uri,
                ..
            } if uri == "maa://12345"
        );

        assert_matches!(
            parse_from(["maa", "copilot", "/your/json/path.json"]).command,
            Command::Copilot {
                uri,
                common: run::CommonArgs { .. }
            } if uri == "/your/json/path.json"
        );
    }

    #[test]
    fn rougelike() {
        assert_matches!(
            parse_from(["maa", "roguelike", "phantom"]).command,
            Command::Roguelike {
                theme,
                ..
            } if matches!(theme, run::preset::RoguelikeTheme::Phantom)
        );
    }

    #[test]
    fn depot() {
        assert_matches!(
            parse_from(["maa", "depot"]).command,
            Command::Depot {
                common: run::CommonArgs { .. }
            }
        );
    }

    #[test]
    fn operbox() {
        assert_matches!(
            parse_from(["maa", "operbox"]).command,
            Command::Operbox {
                common: run::CommonArgs { .. }
            }
        );
    }

    #[test]
    fn convert() {
        assert_matches!(
            parse_from(["maa", "convert", "input.toml"]).command,
            Command::Convert {
                input,
                output: None,
                format: None,
            } if input == PathBuf::from("input.toml")
        );

        assert_matches!(
            parse_from(["maa", "convert", "input.toml", "output.json"]).command,
            Command::Convert {
                output: Some(output),
                ..
            } if output == PathBuf::from("output.json")
        );

        assert_matches!(
            parse_from(["maa", "convert", "input.toml", "--format", "json"]).command,
            Command::Convert {
                format: Some(config::Filetype::Json),
                ..
            }
        );

        assert_matches!(
            parse_from(["maa", "convert", "input.toml", "output.json", "-fy"]).command,
            Command::Convert {
                output: Some(output),
                format: Some(config::Filetype::Yaml),
                ..
            } if output == PathBuf::from("output.json")
        );
    }

    #[test]
    fn activity() {
        assert_matches!(
            parse_from(["maa", "activity"]).command,
            Command::Activity {
                client: config::task::ClientType::Official,
            }
        );

        assert_matches!(
            parse_from(["maa", "activity", "YoStarEN"]).command,
            Command::Activity {
                client: config::task::ClientType::YoStarEN,
            }
        );
    }

    #[test]
    fn remainder() {
        assert_matches!(
            parse_from(["maa", "remainder", "3"]).command,
            Command::Remainder {
                divisor: 3,
                timezone: None,
            }
        );

        assert_matches!(
            parse_from(["maa", "remainder", "3", "--timezone", "8"]).command,
            Command::Remainder {
                divisor: 3,
                timezone: Some(8),
            }
        );
    }

    #[test]
    fn cleanup() {
        use cleanup::CleanupTarget::*;
        assert_matches!(
            parse_from(["maa", "cleanup"]).command,
            Command::Cleanup { .. }
        );

        assert_matches!(
            parse_from(["maa", "cleanup", "log"]).command,
            Command::Cleanup { targets } if targets == vec![Log]
        );

        assert_matches!(
            parse_from(["maa", "cleanup", "cli-cache", "log"]).command,
            Command::Cleanup { targets } if targets == vec![CliCache, Log]
        );
    }

    #[test]
    fn list() {
        assert_matches!(parse_from(["maa", "list"]).command, Command::List);
    }

    #[test]
    fn import() {
        assert_matches!(
            parse_from(["maa", "import", "path"]).command,
            Command::Import {
                path,
                force: false,
                config_type,
            } if path == PathBuf::from("path") && config_type == "task"
        );

        assert_matches!(
            parse_from(["maa", "import", "path", "--force"]).command,
            Command::Import { force: true, .. }
        );

        assert_matches!(
            parse_from(["maa", "import", "path", "-t", "cli"]).command,
            Command::Import {
                config_type,
                ..
            } if config_type == "cli"
        );
    }

    #[test]
    fn init() {
        assert_matches!(
            parse_from(["maa", "init"]).command,
            Command::Init {
                name: None,
                format: None,
                force: false,
            }
        );

        assert_matches!(
            parse_from(["maa", "init", "--name", "name"]).command,
            Command::Init {
                name: Some(name),
                ..
            } if name == PathBuf::from("name")
        );

        assert_matches!(
            parse_from(["maa", "init", "--format", "yaml"]).command,
            Command::Init {
                format: Some(config::Filetype::Yaml),
                ..
            }
        );

        assert_matches!(
            parse_from(["maa", "init", "-ft"]).command,
            Command::Init {
                format: Some(config::Filetype::Toml),
                ..
            }
        );

        assert_matches!(
            parse_from(["maa", "init", "--force"]).command,
            Command::Init { force: true, .. }
        );
    }

    #[test]
    fn complete() {
        assert_matches!(
            parse_from(["maa", "complete", "bash"]).command,
            Command::Complete { shell: Shell::Bash }
        );
    }

    #[test]
    fn mangen() {
        let pb = PathBuf::from(".");
        assert_matches!(
            parse_from(["maa", "mangen", "--path", "."]).command,
            Command::Mangen { path } if path == pb
        );
    }
}
