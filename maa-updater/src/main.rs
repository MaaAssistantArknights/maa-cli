mod config;

use maa_utils::config::{Error as ConfigError, FindFile};
use maa_utils::dirs::{Dirs, ProjectDirs};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about = "Update maa core, resources or both")]
struct Updater {
    #[clap(subcommand)]
    target: Option<UpdateTarget>,
}

fn update_package(mirror: &str, cache_dir: &std::path::PathBuf, data_dir: &std::path::PathBuf) {
    println!("Warning: this is not implemented yet!");
    println!("Updating core and resources...");
    println!("Mirror: {}", mirror);
    println!("Download prebuilt packages to {}", cache_dir.display());
    println!("Extract core and resources to {}", data_dir.display());
}

fn update_core(mirror: &str, cache_dir: &std::path::PathBuf, data_dir: &std::path::PathBuf) {
    println!("Warning: this is not implemented yet!");
    println!("Updating core by building from source...");
    println!("Mirror: {}", mirror);
    println!("Clone source code to {}", cache_dir.display());
    println!("Build core and install to {}", data_dir.display());
}

fn update_resources(mirror: &str, cache_dir: &std::path::PathBuf, data_dir: &std::path::PathBuf) {
    println!("Warning: this is not implemented yet!");
    println!("Updating resources...");
    println!("Mirror: {}", mirror);
    println!("Clone resources to {}", cache_dir.display());
    println!("Build resources and install to {}", data_dir.display());
}

#[derive(Subcommand)]
enum UpdateTarget {
    #[clap(about = "Update both maa core and resources by downloading prebuilt packages")]
    Package {
        #[clap(short, long, help = "Mirror to download packages")]
        mirror: Option<String>,
    },
    #[clap(about = "Update maa core building from source")]
    Core {
        #[clap(short, long, help = "Mirror to download source code")]
        mirror: Option<String>,
    },
    #[clap(about = "Update maa resources")]
    Resources {
        #[clap(short, long, help = "Mirror to download resources")]
        mirror: Option<String>,
    },
}

fn main() {
    let project = ProjectDirs::from("maa");

    let date_dir = if let Some(dir) = project.data_dir() {
        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }
        dir
    } else {
        panic!("Failed to get data directory!");
    };

    let cache_dir = if let Some(dir) = project.cache_dir() {
        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }
        dir
    } else {
        panic!("Failed to get cache directory!");
    };

    let config_dir = if let Some(dir) = project.config_dir() {
        if !dir.exists() {
            panic!("Config directory not exists!");
        }
        dir
    } else {
        panic!("Failed to get config directory!");
    };

    let mut default_mirror = config::default_mirror();
    match config::Update::find_file(&config_dir.join("update")) {
        Ok(config) => {
            default_mirror = config.mirror;
        }
        Err(ConfigError::FileNotFound(_)) => {}
        Err(err) => {
            panic!("Failed to read config file: {}", err);
        }
    }

    let cli = Updater::parse();

    match cli.target {
        None => {
            println!("No target specified, updating both core and resources...");
            update_package(&default_mirror, &cache_dir, &date_dir);
        }
        Some(target) => match target {
            UpdateTarget::Core { mirror } => {
                let mirror = mirror.unwrap_or(default_mirror);
                update_core(&mirror, &cache_dir, &date_dir);
            }
            UpdateTarget::Resources { mirror } => {
                let mirror = mirror.unwrap_or(default_mirror);
                update_resources(&mirror, &cache_dir, &date_dir);
            }
            UpdateTarget::Package { mirror } => {
                let mirror = mirror.unwrap_or(default_mirror);
                update_package(&mirror, &cache_dir, &date_dir);
            }
        },
    }
}
