use crate::{
    activity, config, consts, dirs, installer,
    run::{self, preset},
};

use anyhow::Context;

#[derive(clap::Parser)]
#[command(
    name = "maa",
    author,
    version,
    disable_help_flag(true),
    disable_help_subcommand(true),
    disable_version_flag(true),
    localize(fl!("about")),
)]
#[allow(clippy::upper_case_acronyms)]
struct CLI {
    #[command(subcommand)]
    command: SubCommand,
    #[arg(long, global = true, help = fl!("batch-help"), long_help = fl!("batch-long-help"))]
    batch: bool,
    /// Log related options
    #[command(flatten)]
    log: crate::log::Args,
}

#[derive(clap::Subcommand)]
enum SubCommand {
    #[cfg(feature = "core_installer")]
    #[command(localize(fl!("install-about")))]
    Install {
        #[command(flatten)]
        common: config::cli::maa_core::CommonArgs,
        #[arg(short, long, help = fl!("install-force-help"),
              long_help = fl!("install-force-long-help"))]
        force: bool,
    },
    #[cfg(feature = "core_installer")]
    #[command(localize(fl!("update-about")))]
    Update {
        #[command(flatten)]
        common: config::cli::maa_core::CommonArgs,
    },
    #[cfg(feature = "cli_installer")]
    #[command(localize(fl!("self-update-about")))]
    SelfUpdate {
        #[command(flatten)]
        common: config::cli::maa_cli::CommonArgs,
    },
    #[command(localize(fl!("hot-update-about")))]
    HotUpdate,
    #[command(localize(fl!("dir-about")))]
    Dir {
        #[arg(hide_possible_values = true, help = fl!("dir-target-help"))]
        dir: DirTarget,
    },
    #[command(localize(fl!("version-about")))]
    Version {
        #[arg(hide_possible_values = true, hide_default_value = true,
              default_value_t = Component::All, help = fl!("version-component-help"))]
        component: Component,
    },
    #[command(localize(fl!("run-about")))]
    Run {
        #[arg(help = fl!("run-task-help"), long_help = fl!("run-task-long-help"))]
        task: String,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    #[command(localize(fl!("startup-about")))]
    Startup {
        #[arg(hide_possible_values = true, hide_default_value = true,
              help = fl!("startup-client-help"), long_help = fl!("startup-client-long-help"))]
        client: Option<config::task::ClientType>,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    #[command(localize(fl!("closedown-about")))]
    Closedown {
        #[command(flatten)]
        common: run::CommonArgs,
    },
    #[command(localize(fl!("fight-about")))]
    Fight {
        /// Stage to fight
        #[arg(default_value = "", hide_default_value = true,
              help = fl!("fight-stage-help"), long_help = fl!("fight-stage-long-help"))]
        stage: String,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    #[command(localize(fl!("copilot-about")))]
    Copilot {
        #[arg(help = fl!("copilot-uri-help"))]
        uri: String,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    #[command(localize(fl!("roguelike-about")))]
    Roguelike {
        #[arg(hide_possible_values = true, help = fl!("roguelike-theme-help"),
            long_help = fl!("roguelike-theme-long-help"))]
        theme: preset::RoguelikeTheme,
        #[command(flatten)]
        common: run::CommonArgs,
    },
    #[command(localize(fl!("convert-about")))]
    Convert {
        #[arg(help = fl!("convert-input-help"))]
        input: std::path::PathBuf,
        #[arg(help = fl!("convert-output-help"))]
        output: Option<std::path::PathBuf>,
        #[arg(short, long, hide_possible_values = true,
          help = fl!("convert-format-help"), long_help = fl!("convert-format-long-help"))]
        format: Option<config::Filetype>,
    },
    #[command(localize(fl!("activity-about")))]
    Activity {
        #[arg(
            hide_default_value = true,
            hide_possible_values = true,
            default_value_t = config::task::ClientType::Official,
            help = fl!("activity-client-help"),
        )]
        client: config::task::ClientType,
    },
    #[command(localize(fl!("list-about")))]
    List,
    #[command(localize(fl!("complete-about")))]
    Complete {
        #[arg(hide_possible_values = true, help = fl!("complete-shell-help"))]
        shell: clap_complete::Shell,
    },
}

#[derive(clap::ValueEnum, Clone, Default)]
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

#[derive(clap::ValueEnum, Clone)]
pub enum DirTarget {
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

lazy_static::lazy_static! {
    static ref HELP_TEMPLATE: String = {
        let usage_fl = fl!("Usage");
        format!(
            "{{about}}\n\n\
            {usage_fl}: {{usage}}\n\n\
            {{all-args}}\n\n\
            {{after-help}}",
        )
    };
}

fn build_command() -> clap::Command {
    use clap::CommandFactory;

    CLI::command().arg(
        clap::Arg::new("help")
            .global(true)
            .short('h')
            .long("help")
            .help(fl!("help-help"))
            .help_heading(fl!("Global-Options"))
            .action(clap::ArgAction::Help),
    )
}

trait LocalizeCommand {
    /// Set some localized options for the command and set about message
    fn localize(self, about: String) -> Self;
}

impl LocalizeCommand for clap::Command {
    fn localize(self, about: String) -> Self {
        self.about(about)
            .subcommand_value_name(fl!("SUBCOMMAND"))
            .subcommand_help_heading(fl!("Subcommands"))
            .mut_args(localize_help_heading)
            .help_template(HELP_TEMPLATE.as_str())
    }
}

fn localize_help_heading(arg: clap::Arg) -> clap::Arg {
    if arg.is_positional() {
        arg.help_heading(fl!("Arguments"))
    } else if arg.is_global_set() {
        arg.help_heading(fl!("Global-Options"))
    } else {
        arg.help_heading(fl!("Options"))
    }
}

pub fn process() -> anyhow::Result<()> {
    use clap::FromArgMatches;
    let cli = CLI::from_arg_matches_mut(&mut build_command().get_matches())?;

    cli.log.init_logger()?;

    if cli.batch {
        crate::value::userinput::enable_batch_mode()
    }

    let subcommand = cli.command;

    use SubCommand::*;

    match subcommand {
        #[cfg(feature = "core_installer")]
        Install { force, common } => {
            installer::maa_core::install(force, &common)?;
            installer::resource::update(false)?;
        }
        #[cfg(feature = "core_installer")]
        Update { common } => {
            installer::maa_core::update(&common)?;
            installer::resource::update(false)?;
        }
        #[cfg(feature = "cli_installer")]
        SelfUpdate { common } => installer::maa_cli::update(&common)?,
        HotUpdate => installer::resource::update(false)?,
        Dir { dir } => {
            use DirTarget::*;
            match dir {
                Data => println!("{}", dirs::data().display()),
                Library => {
                    println!(
                        "{}",
                        dirs::find_library()
                            .with_context(lfl!("maa-core-not-found"))?
                            .display()
                    )
                }
                Resource => {
                    println!(
                        "{}",
                        dirs::find_resource()
                            .with_context(lfl!("resource-directory-not-found"))?
                            .display()
                    )
                }
                HotUpdate => println!("{}", dirs::hot_update().display()),
                Config => println!("{}", dirs::config().display()),
                Cache => println!("{}", dirs::cache().display()),
                Log => println!("{}", dirs::log().display()),
            }
        }
        Version { component } => {
            use Component::*;
            match component {
                All => {
                    println!("maa-cli v{}", env!("CARGO_PKG_VERSION"));
                    println!("MaaCore {}", run::core_version()?);
                }
                MaaCLI => {
                    println!("maa-cli v{}", env!("CARGO_PKG_VERSION"));
                }
                MaaCore => {
                    println!("MaaCore {}", run::core_version()?);
                }
            }
        }
        Run { task, common } => run::run_custom(task, common)?,
        Startup { client, common } => run::run(|_| preset::startup(client), common)?,
        Closedown { common } => run::run(|_| preset::closedown(), common)?,
        Fight { stage, common } => run::run(|_| preset::fight(stage), common)?,
        Copilot { uri, common } => run::run(
            |config| preset::copilot(uri, config.resource.base_dirs()),
            common,
        )?,
        Roguelike {
            theme: args,
            common,
        } => run::run(|_| preset::roguelike(args), common)?,
        Convert {
            input,
            output,
            format,
        } => config::convert(&input, output.as_deref(), format)?,
        Activity { client } => activity::display_stage_activity(client)?,
        List => {
            let task_dir = dirs::config().join("tasks");
            if !task_dir.exists() {
                eprintln!(
                    "{}",
                    fl!(
                        "task-directory-not-exist",
                        path = task_dir.to_string_lossy()
                    )
                );
            } else {
                for entry in task_dir.read_dir()? {
                    let entry = entry?;
                    let path = entry.path();
                    // Ignore dot files
                    if path.is_file()
                        && path
                            .file_name()
                            .and_then(|x| x.to_str().map(|x| !x.starts_with('.')))
                            .unwrap_or(true)
                    {
                        println!("{}", path.file_stem().unwrap().to_str().unwrap());
                    }
                }
            }
        }
        Complete { shell } => {
            clap_complete::generate(
                shell,
                &mut build_command(),
                consts::MAA_CLI_NAME,
                &mut std::io::stdout(),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    use config::cli;

    #[macro_export]
    macro_rules! assert_matches {
        ($value:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {
            assert!(matches!($value, $pattern $(if $guard)?))
        };
    }

    fn parse_from<'a>(args: impl IntoIterator<Item = &'a str>) -> CLI {
        use clap::FromArgMatches;
        CLI::from_arg_matches_mut(&mut build_command().get_matches_from(args)).unwrap()
    }

    mod parser {
        use super::*;

        use config::cli::Channel;

        use std::env;

        use SubCommand::*;

        #[test]
        fn global_options() {
            use log::LevelFilter::*;

            // Test default values
            env::remove_var("MAA_LOG");
            assert_eq!(
                parse_from(["maa", "list", "--verbose"]).log.to_filter(),
                Info
            );
            assert_eq!(
                parse_from(["maa", "--verbose", "list"]).log.to_filter(),
                Info
            );
            assert_eq!(
                parse_from(["maa", "list", "--quiet"]).log.to_filter(),
                Error
            );
            assert_eq!(parse_from(["maa", "list", "-vv"]).log.to_filter(), Debug);
            assert_eq!(parse_from(["maa", "list", "-v"]).log.to_filter(), Info);
            assert_eq!(parse_from(["maa", "list"]).log.to_filter(), Warn);
            assert_eq!(parse_from(["maa", "list", "-vq"]).log.to_filter(), Warn);
            assert_eq!(parse_from(["maa", "list", "-q"]).log.to_filter(), Error);
            assert_eq!(parse_from(["maa", "list", "-qq"]).log.to_filter(), Off);
            assert_eq!(parse_from(["maa", "list", "-qqq"]).log.to_filter(), Off);

            // Test environment variable
            env::set_var("MAA_LOG", "Info");
            assert_eq!(
                parse_from(["maa", "list"]).log.to_filter(),
                log::LevelFilter::Info
            );
            env::remove_var("MAA_LOG");

            // Test log file
            use std::path::Path;
            assert!(parse_from(["maa", "list"]).log.log_file().is_none());
            assert!(parse_from(["maa", "list", "--log-file"])
                .log
                .log_file()
                .is_some_and(|x| {
                    let now = chrono::Local::now();
                    let dir = dirs::log()
                        .join(now.format("%Y").to_string())
                        .join(now.format("%m").to_string())
                        .join(now.format("%d").to_string());

                    // the file name is dependent on the current time, it's hard to test
                    x.starts_with(dir)
                }));
            assert!(parse_from(["maa", "list", "--log-file=path"])
                .log
                .log_file()
                .is_some_and(|x| x == Path::new("path")));

            // Test batch mode
            assert!(!parse_from(["maa", "list"]).batch);
            assert!(parse_from(["maa", "list", "--batch"]).batch);
        }

        #[cfg(feature = "core_installer")]
        #[test]
        fn install() {
            assert_matches!(
                parse_from(["maa", "install"]).command,
                Install {
                    common: cli::maa_core::CommonArgs { .. },
                    force: false,
                }
            );

            assert_matches!(
                parse_from(["maa", "install", "beta"]).command,
                Install {
                    common: cli::maa_core::CommonArgs {
                        channel: Some(Channel::Beta),
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                parse_from(["maa", "install", "--no-resource"]).command,
                Install {
                    common: cli::maa_core::CommonArgs {
                        no_resource: true,
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                parse_from(["maa", "install", "-t5"]).command,
                Install {
                    common: cli::maa_core::CommonArgs {
                        test_time: Some(5),
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                parse_from(["maa", "install", "--test-time", "5"]).command,
                Install {
                    common: cli::maa_core::CommonArgs {
                        test_time: Some(5),
                        ..
                    },
                    ..
                }
            );

            assert_matches!(
                parse_from(["maa", "install", "--api-url", "url"]).command,
                Install {
                    common: cli::maa_core::CommonArgs {
                        api_url: Some(url),
                        ..
                    },
                    ..
                } if url == "url"
            );

            assert!(matches!(
                parse_from(["maa", "install", "--force"]).command,
                Install { force: true, .. }
            ));
        }

        #[cfg(feature = "core_installer")]
        #[test]
        fn update() {
            assert_matches!(
                parse_from(["maa", "update"]).command,
                Update {
                    common: cli::maa_core::CommonArgs { .. },
                }
            );
        }

        #[cfg(feature = "cli_installer")]
        #[test]
        fn self_command() {
            assert_matches!(
                parse_from(["maa", "self-update"]).command,
                SelfUpdate { .. }
            );

            assert_matches!(
                parse_from(["maa", "self-update", "beta"]).command,
                SelfUpdate {
                    common: cli::maa_cli::CommonArgs {
                        channel: Some(Channel::Beta),
                        ..
                    },
                }
            );

            assert_matches!(
                parse_from(["maa", "self-update", "--api-url", "url"]).command,
                SelfUpdate{
                    common: cli::maa_cli::CommonArgs {
                        api_url: Some(url),
                        ..
                    }
                } if url == "url"
            );
        }

        #[test]
        fn dir() {
            use DirTarget::*;
            assert_matches!(
                parse_from(["maa", "dir", "data"]).command,
                Dir { dir: Data }
            );
            assert_matches!(
                parse_from(["maa", "dir", "library"]).command,
                Dir { dir: Library }
            );
            assert_matches!(
                parse_from(["maa", "dir", "lib"]).command,
                Dir { dir: Library }
            );
            assert_matches!(
                parse_from(["maa", "dir", "config"]).command,
                Dir { dir: Config }
            );
            assert_matches!(
                parse_from(["maa", "dir", "cache"]).command,
                Dir { dir: Cache }
            );
            assert_matches!(
                parse_from(["maa", "dir", "resource"]).command,
                Dir { dir: Resource }
            );
            assert_matches!(
                parse_from(["maa", "dir", "hot-update"]).command,
                Dir { dir: HotUpdate }
            );
            assert_matches!(parse_from(["maa", "dir", "log"]).command, Dir { dir: Log });
        }

        #[test]
        fn version() {
            use Component::*;
            assert_matches!(
                parse_from(["maa", "version"]).command,
                Version { component: All }
            );
            assert_matches!(
                parse_from(["maa", "version", "all"]).command,
                Version { component: All }
            );
            assert_matches!(
                parse_from(["maa", "version", "maa-cli"]).command,
                Version { component: MaaCLI }
            );
            assert_matches!(
                parse_from(["maa", "version", "cli"]).command,
                Version { component: MaaCLI }
            );
            assert_matches!(
                parse_from(["maa", "version", "maa-core"]).command,
                Version { component: MaaCore }
            );
            assert_matches!(
                parse_from(["maa", "version", "core"]).command,
                Version { component: MaaCore }
            );
        }

        #[test]
        fn run() {
            use run::CommonArgs;
            assert_matches!(
                parse_from(["maa", "run", "task"]).command,
                Run {
                    task,
                    common: CommonArgs { .. },
                } if task == "task"
            );

            assert!(matches!(
                parse_from(["maa", "run", "task", "-a", "addr"]).command,
                Run {
                    task,
                    common: CommonArgs {
                        addr: Some(addr),
                        ..
                    },
                    ..
                } if task == "task" && addr == "addr"
            ));
            assert!(matches!(
                parse_from(["maa", "run", "task", "--addr", "addr"]).command,
                Run {
                    task,
                    common: CommonArgs {
                        addr: Some(addr),
                        ..
                    },
                    ..
                } if task == "task" && addr == "addr"
            ));

            assert!(matches!(
                parse_from(["maa", "run", "task", "--user-resource"]).command,
                Run {
                    task,
                    common: CommonArgs {
                        user_resource: true,
                        ..
                    },
                    ..
                } if task == "task"
            ));
        }

        #[test]
        fn startup() {
            use config::task::ClientType::*;
            assert_matches!(
                parse_from(["maa", "startup"]).command,
                Startup { client: None, .. },
            );

            assert_matches!(
                parse_from(["maa", "startup", "Official"]).command,
                Startup {
                    client: Some(Official),
                    ..
                },
            );
        }

        #[test]
        fn fight() {
            assert_matches!(
                parse_from(["maa", "fight", "1-7"]).command,
                Fight {
                    stage,
                    ..
                } if stage == "1-7"
            );
        }

        #[test]
        fn copilot() {
            assert_matches!(
                parse_from(["maa", "copilot", "maa://12345"]).command,
                Copilot {
                    uri,
                    ..
                } if uri == "maa://12345"
            );

            assert_matches!(
                parse_from(["maa", "copilot", "/your/json/path.json"]).command,
                Copilot {
                    uri,
                    ..
                } if uri == "/your/json/path.json"
            );
        }

        #[test]
        fn rougelike() {
            use preset::RoguelikeTheme::*;
            assert_matches!(
                parse_from(["maa", "roguelike", "phantom"]).command,
                SubCommand::Roguelike { theme: Phantom, .. }
            );
        }

        #[test]
        fn convert() {
            use config::Filetype::*;
            use std::path::PathBuf;
            assert_matches!(
                parse_from(["maa", "convert", "input.toml"]).command,
                Convert {
                    input,
                    output: None,
                    format: None,
                } if input == PathBuf::from("input.toml")
            );

            assert_matches!(
                parse_from(["maa", "convert", "input.toml", "output.json"]).command,
                Convert {
                    output: Some(output),
                    ..
                } if output == PathBuf::from("output.json")
            );

            assert_matches!(
                parse_from(["maa", "convert", "input.toml", "--format", "json"]).command,
                Convert {
                    format: Some(Json),
                    ..
                }
            );

            assert_matches!(
                parse_from(["maa", "convert", "input.toml", "output.json", "-fy"]).command,
                Convert {
                    output: Some(output),
                    format: Some(Yaml),
                    ..
                } if output == PathBuf::from("output.json")
            );
        }

        #[test]
        fn activity() {
            use config::task::ClientType::*;

            assert_matches!(
                parse_from(["maa", "activity"]).command,
                Activity { client: Official }
            );

            assert_matches!(
                parse_from(["maa", "activity", "YoStarEN"]).command,
                Activity { client: YoStarEN }
            );
        }

        #[test]
        fn list() {
            assert_matches!(parse_from(["maa", "list"]).command, SubCommand::List);
        }

        #[test]
        fn complete() {
            use clap_complete::Shell::*;
            assert_matches!(
                parse_from(["maa", "complete", "bash"]).command,
                Complete { shell: Bash }
            );
        }
    }
}
