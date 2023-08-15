mod package;
use package::Channel;

use std::env::var_os;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use directories::ProjectDirs;
use paste::paste;

#[derive(Parser)]
#[command(author, version)]
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
struct Updater {
    #[clap(default_value_t = Channel::default())]
    /// Channel to download prebuilt package
    channel: Channel,
    /// Do not extract resource files
    #[clap(long)]
    no_resource: bool,
    /// Time to test download speed
    #[clap(short, long, default_value_t = 3)]
    test_time: u64,
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

get_dir!(data);
get_dir!(cache);

fn main() -> Result<ExitCode> {
    let dirs = ProjectDirs::from("com", "loong", "maa");
    let cli = Updater::parse();

    let channel = cli.channel;

    let data_dir = get_data_dir(&dirs);
    let cache_dir = get_cache_dir(&dirs);

    println!("Updating package (channel: {})...", channel);

    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)?;
    }
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }

    let no_resource = cli.no_resource;
    let test_time = cli.test_time;
    package::get_package(&channel)?
        .get_asset()?
        .download(&cache_dir, test_time)?
        .extract(&data_dir, !no_resource)?;

    return Ok(ExitCode::SUCCESS);
}
