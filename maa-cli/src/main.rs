mod config;
mod dirs;
mod installer;
mod log;
mod run;

use crate::config::{cli::CLIConfig, FindFile};
#[cfg(feature = "self")]
use crate::installer::maa_cli;
use crate::installer::maa_core::{self, Channel, MaaCore};

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use directories::ProjectDirs;

#[derive(Parser)]
#[command(name = "maa", author, version)]
#[allow(clippy::upper_case_acronyms)]
enum CLI {
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
        /// ADB serial number of device or MaaTools address set in PlayCover
        ///
        /// By default, MaaCore connects to game with ADB,
        /// and this parameter is the serial number of the device
        /// (default to `emulator-5554` if not specified here and not set in config file).
        /// And if you want to use PlayCover,
        /// you need to set the connection type to PlayCover in the config file
        /// and then you can specify the address of MaaTools here.
        #[clap(short, long, verbatim_doc_comment)]
        addr: Option<String>,
        /// Load resources from the user config directory
        ///
        /// By default, MaaCore loads resources from the data directory,
        /// which is shipped with the program.
        /// If you want to load resources from the user config directory,
        /// you can use this option.
        /// The `resource` directory must be in the config directory
        /// and the resources must be in the `resource` directory.
        ///
        /// Note: user resources will be loaded at the end,
        /// so if there are resources with the same name,
        /// the user resources will overwrite the default resources.
        /// use at your own risk!
        #[clap(long, verbatim_doc_comment)]
        user_resource: bool,
        /// Output more information, repeat to increase verbosity
        ///
        /// This option is used to control the log level of this program and MaaCore.
        /// There are 6 levels of log:
        /// Error   // show only error messages
        /// Warning // show all error and warning messages
        /// normal  // show all above messages and basic information
        /// Info    // show all above messages and more detailed information
        /// Debug   // show all above messages and some information about configuration
        /// Trace   // show all above messages and trace information
        ///
        /// The default log level is normal.
        /// If you want to see more information, you can use this option to increase the log level.
        #[clap(short, long, action = clap::ArgAction::Count, verbatim_doc_comment)]
        verbose: u8,
        /// Output less information, repeat to increase quietness
        ///
        /// This option is used to control the log level of this program and MaaCore.
        /// There are 6 levels of log:
        /// Error   // show only error messages
        /// Warning // show all error and warning messages
        /// normal  // show all above messages and basic information
        /// Info    // show all above messages and more detailed information
        /// Debug   // show all above messages and some information about configuration
        /// Trace   // show all above messages and trace information
        /// The default log level is normal.
        /// If you want to see less information, you can use this option to decrease the log level.
        #[clap(short, long, action = clap::ArgAction::Count, verbatim_doc_comment)]
        quiet: u8,
        /// Run tasks in batch mode
        ///
        /// If there are some input parameters in the task file,
        /// some prompts will be displayed to ask for input.
        /// In batch mode, the prompts will be skipped,
        /// and parameters will be set to default values.
        #[clap(short, long, verbatim_doc_comment)]
        batch: bool,
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
    Library,
    /// Directory of maa-cli's dynamic library, alias of library
    Lib,
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

    match cli {
        CLI::Install {
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
        CLI::Update {
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
        CLI::SelfCommand(self_command) => match self_command {
            SelfCommand::Update => {
                maa_cli::update(&proj_dirs)?;
            }
        },
        CLI::Dir { dir_type } => match dir_type {
            Dir::Data => println!("{}", proj_dirs.data().display()),
            Dir::Library | Dir::Lib => {
                println!("{}", maa_core::find_lib_dir(&proj_dirs).unwrap().display())
            }
            Dir::Config => println!("{}", proj_dirs.config().display()),
            Dir::Cache => println!("{}", proj_dirs.cache().display()),
            Dir::Resource => {
                println!("{}", maa_core::find_resource(&proj_dirs).unwrap().display())
            }
            Dir::Log => println!("{}", proj_dirs.log().display()),
        },
        CLI::Version { component } => match component {
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
        CLI::Run {
            task,
            addr,
            user_resource,
            verbose,
            quiet,
            batch,
        } => run::run(&proj_dirs, task, addr, user_resource, verbose, quiet, batch)?,
        CLI::List => {
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
        CLI::Complete { shell } => {
            generate(shell, &mut CLI::command(), "maa", &mut std::io::stdout());
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {}
