mod config;
use config::{asst, task, Error as ConfigError, FindFile};

use asst::{AsstConfig, Connection};
use task::{TaskList, TaskType};

use maa_sys::Assistant;

mod message;
use message::create_callback;

mod log;
use log::Logger;

use std::env::var_os;
use std::path::PathBuf;
use std::process::ExitCode;

use directories::ProjectDirs;

use serde_json::Value;

use anyhow::{anyhow, Context, Result};

use clap::Parser;

use paste::paste;

#[derive(Parser)]
#[clap(author, about, version)]
enum CLI {
    #[clap(about = "Run a task defined by a config file")]
    Run {
        #[clap(help = "Task name")]
        task: String,
        #[clap(short, long, help = "ADB serial number of the device")]
        adb: Option<String>,
        #[clap(short, long, action = clap::ArgAction::Count, help = "Increase verbosity")]
        verbose: u8,
        #[clap(short, long, action = clap::ArgAction::Count, help = "Decrease verbosity")]
        quiet: u8,
    },
    #[clap(about = "Show version information")]
    Version,
}

trait DirExists: Sized {
    fn exist_or_create(self) -> Result<Self>;
    fn exist_or_err(self) -> Result<Self>;
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
                    panic!(concat!("Failed to get ", stringify!($dir), " directory!"));
                }
            }
        }
    };
}
get_dir!(state);
get_dir!(data);
get_dir!(config);

impl DirExists for PathBuf {
    fn exist_or_create(self) -> Result<Self> {
        if !self.exists() {
            std::fs::create_dir_all(&self)?;
        }
        Ok(self)
    }

    fn exist_or_err(self) -> Result<Self> {
        if !self.exists() {
            return Err(anyhow!("{} does not exist!", self.display()));
        }
        Ok(self)
    }
}

fn main() -> Result<std::process::ExitCode> {
    let project = ProjectDirs::from("com", "loong", "maa");

    let cli = CLI::parse();

    match cli {
        CLI::Run {
            task,
            adb,
            verbose,
            quiet,
        } => {
            let loglevel = 1u8 + verbose - quiet;

            let logger = Logger::from(loglevel);

            let state_dir = get_state_dir(&project);
            logger.debug("State directory:", || state_dir.display().to_string());
            Assistant::set_user_dir(state_dir).context("Failed to set user directory!")?;

            let data_dir = get_data_dir(&project);
            logger.debug("Data directory:", || data_dir.display().to_string());
            Assistant::load_resource(data_dir).context("Failed to load resource!")?;

            let config_dir = get_config_dir(&project);
            logger.debug("Config directory:", || config_dir.display().to_string());

            // This is not a good way to create a C callback with outter variables.
            // so we have to use a macro to create multiple callbacks.
            let callback = match loglevel {
                loglevel if loglevel == 0 => create_callback!(0),
                loglevel if loglevel == 1 => create_callback!(1),
                loglevel if loglevel == 2 => create_callback!(2),
                loglevel if loglevel == 3 => create_callback!(3),
                _ => create_callback!(4),
            };

            let assistant = Assistant::new(Some(callback), None);

            let asst_config = AsstConfig::find_file(&config_dir.join("asst"));
            match asst_config {
                Ok(asst_config) => {
                    if let Some(options) = asst_config.instance_options {
                        logger.debug("Setting touch_mode to", || {
                            format!("{:?}", options.touch_mode)
                        });
                        assistant
                            .set_instance_option(2, options.touch_mode)
                            .context("Failed to set touch mode!")?;
                        if let Some(v) = options.deployment_with_pause {
                            logger.debug("Setting deployment_with_pause to", || v);
                            assistant
                                .set_instance_option(3, v)
                                .context("Failed to set deployment with pause!")?;
                        }
                        if let Some(v) = options.adb_lite_enabled {
                            logger.debug("Setting adb_lite_enabled to", || v);
                            assistant.set_instance_option(4, v)?;
                        }
                        if let Some(v) = options.kill_adb_on_exit {
                            logger.debug("Setting kill_adb_on_exit to", || v);
                            assistant.set_instance_option(5, v)?;
                        }
                    }
                    if let Some(connection) = asst_config.connection {
                        match connection {
                            Connection::ADB {
                                adb_path,
                                device,
                                config,
                            } => {
                                logger.debug("Setting adb_path to", || &adb_path);
                                logger.debug("Setting device to", || &device);
                                logger.debug("Setting config to", || &config);
                                let adb_device = adb.unwrap_or(device);
                                assistant.async_connect(adb_path, adb_device, config, true)?;
                            }
                            Connection::Playcover {} => {
                                panic!("Playcover is not supported yet!");
                            }
                        }
                    }
                }
                Err(ConfigError::FileNotFound(_)) => {
                    logger.info("No asst config found, using default settings.", || "");

                    logger.debug("Setting touch_mode to", || {
                        format!("{:?}", asst::TouchMode::default())
                    });
                    assistant.set_instance_option(2, asst::TouchMode::default())?;

                    logger.debug("Set adb_path to", || asst::default_adb_path());
                    logger.debug("Set device to", || asst::default_device());
                    logger.debug("Set config to", || asst::default_config());
                    let adb_path = asst::default_adb_path();
                    let adb_device = adb.unwrap_or(asst::default_device());
                    let config = asst::default_config();
                    assistant.async_connect(adb_path, adb_device, config, true)?;
                }
                Err(err) => {
                    panic!("Failed to load connection config: {}", err);
                }
            }

            match TaskList::find_file(&config_dir.join("tasks").join(&task)) {
                Ok(task_list) => {
                    for task in task_list.tasks {
                        if task.is_active() {
                            let task_type = task.get_type();
                            let params = &mut task.get_params();
                            if *task_type == TaskType::Infrast {
                                if let Some(v) = params.get_mut("filename") {
                                    assert!(v.is_string());
                                    *v = Value::String(
                                        config_dir
                                            .join("infrast")
                                            .join(v.as_str().ok_or(anyhow!("Invalid filename!"))?)
                                            .to_str()
                                            .ok_or(anyhow!("Invalid filename!"))?
                                            .to_string(),
                                    );
                                }
                            }
                            logger.debug("Task:", || format!("{:?}", task_type));
                            logger.debug("Params:", || {
                                serde_json::to_string(&params)
                                    .map_or_else(|_| "Unknown".to_string(), |s| s)
                            });
                            assistant
                                .append_task(task.get_type(), serde_json::to_string(&params)?)?;
                        }
                    }
                }
                Err(err) => {
                    panic!("Failed to load task config: {}", err);
                }
            }

            assistant.start()?;

            while assistant.running() {
                std::thread::sleep(std::time::Duration::from_millis(5000));
            }

            assistant.stop()?;
        }
        CLI::Version => {
            let cli_version = env!("CARGO_PKG_VERSION");
            let core_version = Assistant::get_version()?;
            println!("maa-cli v{}\nMaaCore {}", cli_version, core_version);
        }
    }

    return Ok(ExitCode::SUCCESS);
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_dir {
        use super::*;
        use std::env;

        #[test]
        fn state_dir() {
            env::remove_var("XDG_STATE_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            let state_dir = get_state_dir(&project);
            if cfg!(target_os = "macos") {
                assert_eq!(
                    state_dir,
                    home_dir.join("Library/Application Support/com.loong.maa")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(state_dir, home_dir.join(".local/state/maa"));
            }

            env::set_var("XDG_STATE_HOME", "/tmp");
            let project = ProjectDirs::from("com", "loong", "maa");
            let state_dir = get_state_dir(&project);
            assert_eq!(state_dir, PathBuf::from("/tmp/maa"));
        }

        #[test]
        fn config_dir() {
            env::remove_var("XDG_CONFIG_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            let config_dir = get_config_dir(&project);
            if cfg!(target_os = "macos") {
                assert_eq!(
                    config_dir,
                    home_dir.join("Library/Application Support/com.loong.maa/config")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(config_dir, home_dir.join(".config/maa"));
            }

            env::set_var("XDG_CONFIG_HOME", "/tmp");
            let config_dir = get_config_dir(&project);
            assert_eq!(config_dir, PathBuf::from("/tmp/maa"));
        }

        #[test]
        fn data_dir() {
            env::remove_var("XDG_DATA_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            let data_dir = get_data_dir(&project);
            if cfg!(target_os = "macos") {
                assert_eq!(
                    data_dir,
                    home_dir.join("Library/Application Support/com.loong.maa")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(data_dir, home_dir.join(".local/share/maa"));
            }

            env::set_var("XDG_DATA_HOME", "/tmp");
            let project = ProjectDirs::from("com", "loong", "maa");
            let data_dir = get_data_dir(&project);
            assert_eq!(data_dir, PathBuf::from("/tmp/maa"));
        }
    }
}
