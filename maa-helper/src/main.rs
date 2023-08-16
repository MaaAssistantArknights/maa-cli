mod package;
use package::Channel;

use std::env::{current_exe, var_os};
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use paste::paste;

#[derive(Parser)]
#[command(author, version)]
#[allow(clippy::upper_case_acronyms)]
struct CLI {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Install or update maa core or resources
    ///
    /// This tool will download prebuilt packages of given channel (default is stable, can be beta or alpha)
    /// and extract them to given data directory used by `maa-cli`.
    /// The packages be extracted and installed to given data directory,
    /// the default data directory see [directories.rs](https://github.com/dirs-dev/directories-rs).
    /// If `MAA_DATA_DIR` is set, it will be used as data directory,
    /// or if `XDG_DATA_HOME` is set, `$XDG_DATA_HOME/maa` will be used as data directory,
    /// otherwise the default data directory will be used.
    /// Once you change the data directory by setting `MAA_DATA_DIR` or `XDG_DATA_HOME`,
    /// make sure you set them when using `maa-cli` too.
    Install {
        #[clap(default_value_t = Channel::default())]
        /// Channel to download prebuilt package
        channel: Channel,
        /// Do not extract resource files
        #[clap(long)]
        no_resource: bool,
        /// Time to test download speed
        #[clap(short, long, default_value_t = 3)]
        test_time: u64,
    },
    /// Print path of maa directories
    Dir { dir_type: Dir },
    /// Print version of maa-run and maa-core
    Version,
    /// Run a maa task
    ///
    /// All arguments will be passed to maa-run,
    /// type -h or --help to see help message of maa-run.
    Run {
        #[clap(name("ARGS"), trailing_var_arg(true))]
        args: Vec<String>,
    },
}

#[derive(ValueEnum, Clone)]
enum Dir {
    Config,
    Data,
    Library,
    Resource,
    Cache,
    Log,
}

macro_rules! matct_loc {
    (state, $dirs:ident) => {
        $dirs
            .state_dir()
            .unwrap_or_else(|| $dirs.data_dir())
            .to_path_buf()
    };
    (config, $dirs:ident) => {
        if cfg!(target_os = "macos") {
            $dirs.config_dir().join("config")
        } else {
            $dirs.config_dir().to_path_buf()
        }
    };
    ($loc:ident, $dirs:ident) => {
        paste! {
            $dirs.[<$loc _dir>]().to_path_buf()
        }
    };
}

macro_rules! get_dir {
    ($loc:ident) => {
        paste! {
            fn [<get_ $loc _dir>](proj: &Option<ProjectDirs>) -> PathBuf {
                if let Some(dir) = var_os(stringify!([<MAA_ $loc:upper _DIR>])) {
                    PathBuf::from(dir)
                } else if let Some(dir) = var_os(stringify!([<XDG_ $loc:upper _HOME>])) {
                    PathBuf::from(dir).join("maa")
                } else if let Some(dirs) = proj {
                    matct_loc!($loc, dirs)
                } else {
                    panic!("Failed to get {} directory!", stringify!($loc))
                }
            }
        }
    };
}

get_dir!(state);
get_dir!(data);
get_dir!(config);
get_dir!(cache);

fn find_maa_run() -> String {
    if let Ok(exe_path) = current_exe() {
        let exe_dir = exe_path.parent().unwrap();
        let maa_run_path = exe_dir.join("maa-run");
        if maa_run_path.exists() {
            return maa_run_path.to_str().unwrap().to_string();
        }
    }
    String::from("maa-run")
}

#[cfg(target_os = "linux")]
const LD_LIB_PATH_VAR: &'static str = "LD_LIBRARY_PATH";
#[cfg(target_os = "macos")]
const LD_LIB_PATH_VAR: &str = "DYLD_FALLBACK_LIBRARY_PATH";
#[cfg(target_os = "windows")]
const LD_LIB_PATH_VAR: &'static str = "PATH";

fn run<S, I>(args: I, dirs: &Option<ProjectDirs>) -> Result<ExitCode>
where
    S: AsRef<std::ffi::OsStr>,
    I: IntoIterator<Item = S>,
{
    let maa_run = find_maa_run();
    let ret = Command::new(maa_run)
        .args(args)
        .env(LD_LIB_PATH_VAR, get_data_dir(dirs).join("lib"))
        .status()
        .expect("failed to execute maa-run");
    if ret.success() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn main() -> Result<ExitCode> {
    let dirs = ProjectDirs::from("com", "loong", "maa");
    let cli = CLI::parse();

    match cli.subcmd {
        SubCommand::Install {
            channel,
            no_resource,
            test_time,
        } => {
            let data_dir = get_data_dir(&dirs);
            let cache_dir = get_cache_dir(&dirs);

            if !cache_dir.exists() {
                std::fs::create_dir_all(&cache_dir)?;
            }
            if !data_dir.exists() {
                std::fs::create_dir_all(&data_dir)?;
            }

            println!("Installing package (channel: {})...", channel);
            package::get_package(&channel)?
                .get_asset()?
                .download(&cache_dir, test_time)?
                .extract(&data_dir, !no_resource)?;
        }
        SubCommand::Dir { dir_type } => {
            let dir = match dir_type {
                Dir::Config => get_config_dir(&dirs),
                Dir::Data => get_data_dir(&dirs),
                Dir::Library => get_data_dir(&dirs).join("lib"),
                Dir::Resource => get_data_dir(&dirs).join("resource"),
                Dir::Cache => get_cache_dir(&dirs),
                Dir::Log => get_state_dir(&dirs).join("debug"),
            };
            println!("{}", dir.display());
        }
        SubCommand::Version => {
            return run(["version"], &dirs);
        }
        SubCommand::Run { args } => {
            return run(args, &dirs);
        }
    }

    Ok(ExitCode::SUCCESS)
}
