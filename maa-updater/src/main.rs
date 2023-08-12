use std::env::var_os;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use paste::paste;

#[derive(Parser)]
#[command(author, version, about = "Update maa core or resources")]
struct Updater {
    #[clap(subcommand)]
    target: Option<UpdateTarget>,
}

#[derive(Subcommand)]
enum UpdateTarget {
    /// Update both maa core and resources by downloading prebuilt packages
    ///
    /// This is the default target if no target is specified.
    /// This target will download prebuilt packages of given channel (default is stable, can be beta or alpha)
    /// from given mirror (default is https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases).
    /// The mirror can be specified by command line argument or environment variable `MAA_PACKAGE_MIRROR`.
    /// The channel can be specified by command line argument or environment variable `MAA_PACKAGE_CHANNEL`.
    /// The packages will be extracted and installed to given data directory,
    /// the default see [directories.rs](https://github.com/dirs-dev/directories-rs).
    /// If `MAA_DATA_DIR` is set, it will be used as data directory,
    /// or if `XDG_DATA_HOME` is set, `$XDG_DATA_HOME/maa` will be used as data directory,
    Package {
        #[arg(short, long, help = "Mirror to download packages")]
        mirror: Option<String>,
        #[arg(short, long, help = "Update channel, it can be stable, beta or alpha")]
        channel: Option<String>,
    },
    /// Update maa core by building from source
    ///
    /// This target will clone the maa core repository and build maa core from source.
    /// The mirror can be specified by command line argument or environment variable `MAA_REPO_MIRROR`.
    /// The source code will be cloned to data directory,
    /// the default see [directories.rs](https://github.com/dirs-dev/directories-rs).
    /// If `MAA_DATA_DIR` is set, it will be used as data directory,
    /// or if `XDG_DATA_HOME` is set, `$XDG_DATA_HOME/maa` will be used as data directory,
    Core {
        #[arg(short, long, help = "Mirror to clone maa core repository")]
        mirror: Option<String>,
    },
    /// Update maa resources from maa core repository
    ///
    /// This target will clone the maa core repository and link resources to data directory.
    /// The mirror can be specified by command line argument or environment variable `MAA_REPO_MIRROR`.
    /// The repository will be cloned to data directory and linked to the same directory,
    /// the default see [directories.rs](https://github.com/dirs-dev/directories-rs).
    /// If `MAA_DATA_DIR` is set, it will be used as data directory,
    /// or if `XDG_DATA_HOME` is set, `$XDG_DATA_HOME/maa` will be used as data directory,
    Resources {
        #[arg(short, long, help = "Mirror to clone maa core repository")]
        mirror: Option<String>,
    },
}

macro_rules! get_dir {
    ($dir:ident) => {
        paste! {
            fn [<get_ $dir _dir>](proj: &Option<ProjectDirs>) -> PathBuf {
                if let Some(dir) = var_os(stringify!([<MAA_ $dir:upper _DIR>])) {
                    PathBuf::from(dir)
                } else if let Some(dir) = var_os(stringify!([<XDG_ $dir:upper _HOME>])) {
                    PathBuf::from(dir).join("maa")
                } else if let Some(dirs) = proj {
                    dirs.[< $dir _dir>]().to_path_buf()
                } else {
                    panic!(concat!("Failed to get ", stringify!($dir), " directory!"));
                }
            }
        }
    };
}

fn arg_env_or_default(arg: Option<String>, env: &str, default: &str) -> String {
    if let Some(arg) = arg {
        arg
    } else if let Some(env) = var_os(env) {
        env.to_str().unwrap_or(default).to_string()
    } else {
        default.into()
    }
}

get_dir!(data);
get_dir!(cache);

fn update_package(
    mirror: &str,
    channel: &str,
    cache_dir: &std::path::PathBuf,
    data_dir: &std::path::PathBuf,
) {
    println!("Warning: this is not implemented yet!");
    println!("Updating core and resources...");
    println!("Mirror: {}", mirror);
    println!("Channel: {}", channel);
    println!("Download prebuilt packages to {}", cache_dir.display());
    let resource_dir = data_dir.join("resource");
    println!("Extract resource to {}", resource_dir.display());
    let lib_dir = data_dir.join("lib");
    println!("Extract shared library to {}", lib_dir.display());
}

fn update_core(mirror: &str, data_dir: &std::path::PathBuf) {
    println!("Warning: this is not implemented yet!");
    println!("Updating core by building from source...");
    println!("Mirror: {}", mirror);
    println!("Clone source code to {}", data_dir.display());
    let lib_dir = data_dir.join("lib");
    println!("Build shared library and install to {}", lib_dir.display());
}

fn update_resources(mirror: &str, data_dir: &std::path::PathBuf) {
    println!("Warning: this is not implemented yet!");
    println!("Updating resources...");
    println!("Mirror: {}", mirror);
    println!("Clone resources to {}", data_dir.display());
    let resource_dir = data_dir.join("resource");
    println!("Link resources to {}", resource_dir.display());
}

fn main() -> Result<ExitCode> {
    let dirs = ProjectDirs::from("com", "loong", "maa");

    let data_dir = get_data_dir(&dirs);
    let cache_dir = get_cache_dir(&dirs);

    let cli = Updater::parse();

    match cli.target {
        None => {
            println!("No target specified, use default target: package");
            let package_mirror = arg_env_or_default(
                None,
                "MAA_PACKAGE_MIRROR",
                "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases",
            );
            let package_channel = arg_env_or_default(None, "MAA_PACKAGE_CHANNEL", "stable");
            update_package(&package_mirror, &package_channel, &cache_dir, &data_dir);
        }
        Some(target) => match target {
            UpdateTarget::Core { mirror } => {
                let repo_mirror = arg_env_or_default(
                    mirror,
                    "MAA_REPO_MIRROR",
                    "https://github.com/MaaAssistantArknights/MaaAssistantArknights",
                );
                update_core(&repo_mirror, &data_dir);
            }
            UpdateTarget::Resources { mirror } => {
                let repo_mirror = arg_env_or_default(
                    mirror,
                    "MAA_REPO_MIRROR",
                    "https://github.com/MaaAssistantArknights/MaaAssistantArknights",
                );
                update_resources(&repo_mirror, &data_dir);
            }
            UpdateTarget::Package { mirror, channel } => {
                let package_mirror = arg_env_or_default(
                    mirror,
                    "MAA_PACKAGE_MIRROR",
                    "https:://github.com/MaaAssistantArknights/MaaAssistantArknights/releases",
                );
                let package_channel = arg_env_or_default(channel, "MAA_PACKAGE_CHANNEL", "stable");
                update_package(&package_mirror, &package_channel, &cache_dir, &data_dir);
            }
        },
    }

    return Ok(ExitCode::SUCCESS);
}
