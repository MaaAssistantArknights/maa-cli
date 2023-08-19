mod dirs;
mod installer;
mod maa_run;

use crate::{
    installer::{
        maa_cli::CLIComponent,
        maa_core::{Channel, MaaCore},
    },
    maa_run::SetLDLibPath,
};

use std::process::{ExitCode, ExitStatus};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use maa_run::command;

#[derive(Parser)]
#[command(author, version)]
#[allow(clippy::upper_case_acronyms)]
enum CLI {
    /// Install maa core or resources
    ///
    /// This command will install maa-core and resources
    /// by downloading prebuilt packages.
    /// Note: If the maa-core and resource are already installed,
    /// please update them by `maa-cli update`.
    /// Note: If you want to install maa-run, please use `maa-cli self install`.
    Install {
        #[arg(default_value_t = Channel::default())]
        /// Channel to download prebuilt package
        ///
        /// There are three channels of maa-core prebuilt packages,
        /// stable, beta and alpha.
        /// The default channel is stable, you can use this flag to change the channel.
        /// If you want to use the latest features of maa-core,
        /// you can use beta or alpha channel.
        /// Note: the alpha channel is only available for windows.
        channel: Channel,
        /// Time to test download speed
        ///
        /// There are several mirrors of maa-core prebuilt packages,
        /// we will test the download speed of these mirrors,
        /// and choose the fastest one to download.
        /// This flag is used to set the time to test download speed.
        /// If you want to increase the accuracy of the test,
        /// please increase the value of this flag.
        /// But if you think the test is too slow,
        /// you can decrease the value of this flag.
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
    },
    /// Update maa core or resources
    ///
    /// This command will update maa-core and resources
    /// by downloading prebuilt packages.
    /// If the version of maa-core is not newer,
    /// we will not update it.
    /// Note: If the maa-core and resource are not installed,
    /// please install them by `maa-cli install`.
    Update {
        #[arg(default_value_t = Channel::default())]
        /// Channel to download prebuilt package
        ///
        /// There are three channels of maa-core prebuilt packages,
        /// stable, beta and alpha.
        /// The default channel is stable, you can use this flag to change the channel.
        /// If you want to use the latest features of maa-core,
        /// you can use beta or alpha channel.
        /// Note: the alpha channel is only available for windows.
        /// Note: if the maa-core is not installed, we will install it.
        channel: Channel,
        /// Do not update resource
        ///
        /// By default, resources are shipped with maa-core,
        /// and we will update them when updating maa-core.
        /// If you do not want to update resource,
        /// you can use this flag to disable it.
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
        /// There are several mirrors of maa-core prebuilt packages,
        /// we will test the download speed of these mirrors,
        /// and choose the fastest one to download.
        /// This flag is used to set the time to test download speed.
        /// If you want to increase the accuracy of the test,
        /// please increase the value of this flag.
        /// But if you think the test is too slow,
        /// you can decrease the value of this flag.
        #[arg(short, long, default_value_t = 3)]
        test_time: u64,
    },
    /// Manage maa-cli self and maa-run
    ///
    /// This command is used to manage maa-cli self and maa-run.
    /// Note: If you want to install or update maa-core and resource,
    /// please use `maa-cli install` or `maa-cli update` instead.
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
    Version { component: Component },
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
        #[clap(name("ARGS"), trailing_var_arg(true))]
        args: Vec<String>,
    },
    /// List all available tasks
    List,
}

#[derive(Subcommand)]
enum SelfCommand {
    /// Install maa-run
    ///
    /// This command will download prebuilt binary of maa-run,
    /// and install it to the binary directory of maa-cli.
    /// Note: If the maa-run is already installed,
    /// please update it by `maa-cli self update`.
    Install,
    /// Update maa-cli self and maa-run
    ///
    /// This command will download prebuilt binary of maa-cli and maa-run,
    /// and install them to the binary directory of maa-cli.
    /// Note: we will check the version of maa-cli and maa-run,
    /// if the version is not newer, we will not update them.
    /// And if the maa-run is not installed, please install it firstly
    /// by `maa-cli self install`.
    Update,
}

#[derive(ValueEnum, Clone, Default)]
enum Component {
    #[default]
    All,
    MaaCLI,
    MaaRun,
    MaaCore,
}

#[derive(ValueEnum, Clone)]
pub enum Dir {
    Binary,
    Library,
    Config,
    Cache,
    Resource,
    Log,
}

fn main() -> Result<ExitCode> {
    let proj = ProjectDirs::from("com", "loong", "maa");
    let proj_dirs = dirs::Dirs::new(proj);

    let cli = CLI::parse();

    match cli {
        CLI::Install {
            channel,
            test_time,
            force,
        } => {
            MaaCore::new(channel).install(&proj_dirs, force, test_time)?;

            Ok(ExitCode::SUCCESS)
        }
        CLI::Update {
            channel,
            no_resource,
            test_time,
        } => {
            MaaCore::new(channel).update(&proj_dirs, no_resource, test_time)?;

            Ok(ExitCode::SUCCESS)
        }
        CLI::SelfCommand(self_command) => match self_command {
            SelfCommand::Install => {
                CLIComponent::MaaRun.install(&proj_dirs)?;
                Ok(ExitCode::SUCCESS)
            }
            SelfCommand::Update => {
                CLIComponent::MaaCLI.update(&proj_dirs)?;
                CLIComponent::MaaRun.update(&proj_dirs)?;
                Ok(ExitCode::SUCCESS)
            }
        },
        CLI::Dir { dir_type } => {
            let dir = match dir_type {
                Dir::Binary => proj_dirs.binary(),
                Dir::Library => proj_dirs.library(),
                Dir::Config => proj_dirs.config(),
                Dir::Cache => proj_dirs.cache(),
                Dir::Resource => proj_dirs.resource(),
                Dir::Log => proj_dirs.log(),
            };
            println!("{}", dir.display());

            Ok(ExitCode::SUCCESS)
        }
        CLI::Version { component } => match component {
            Component::All => {
                println!("maa-cli {}", env!("CARGO_PKG_VERSION"));
                command(&proj_dirs)?
                    .set_ld_lib_path(&proj_dirs)
                    .arg("--version")
                    .status()?
                    .to_code()?;
                command(&proj_dirs)?
                    .set_ld_lib_path(&proj_dirs)
                    .arg("version")
                    .status()?
                    .to_code()?;
                Ok(ExitCode::SUCCESS)
            }
            Component::MaaCLI => {
                println!("maa-cli {}", env!("CARGO_PKG_VERSION"));
                Ok(ExitCode::SUCCESS)
            }
            Component::MaaRun => command(&proj_dirs)?
                .set_ld_lib_path(&proj_dirs)
                .arg("--version")
                .status()?
                .to_code(),
            Component::MaaCore => command(&proj_dirs)?
                .set_ld_lib_path(&proj_dirs)
                .arg("version")
                .status()?
                .to_code(),
        },
        CLI::Run { args } => command(&proj_dirs)?
            .set_ld_lib_path(&proj_dirs)
            .arg("run")
            .args(&args)
            .status()?
            .to_code(),
        CLI::List => {
            let task_dir = proj_dirs.config().join("tasks");
            if !task_dir.exists() {
                println!("No tasks found");
                Ok(ExitCode::SUCCESS)
            } else {
                for entry in task_dir.read_dir()? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        println!("{}", path.file_stem().unwrap().to_str().unwrap());
                    }
                }
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

/// Convert `ExitStatus` to `ExitCode`
///
/// If the command is successful, return `ExitCode::SUCCESS`,
/// otherwise return an error with the exit code.
trait ToCode {
    fn to_code(&self) -> Result<ExitCode>;
}

impl ToCode for ExitStatus {
    fn to_code(&self) -> Result<ExitCode> {
        if self.success() {
            Ok(ExitCode::SUCCESS)
        } else {
            Err(anyhow!("Command failed with exit code {}", self))
        }
    }
}
