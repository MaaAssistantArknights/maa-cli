mod config;
use config::{asst, task, FindFile};

use asst::{AsstConfig, Connection};
use task::{TaskList, TaskType};

use maa_sys::Assistant;

mod message;
use message::callback;

mod log;

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
#[allow(clippy::upper_case_acronyms)]
enum CLI {
    /// Run a predefined task
    Run {
        /// Name of the task to run
        ///
        /// The task name is the name of the task file without the extension.
        /// The task file must be in the `tasks` directory of the config directory.
        /// The task file must be in the TOML or JSON format.
        task: String,
        /// ADB serial number of device or MaaTools address set in PlayCover
        ///
        /// By default, MaaCore connects to game with ADB,
        /// and this parameter is the serial number of the device
        /// (default to `emulator-5554` if not specified here and not set in config file).
        /// And if you want to use PlayCover,
        /// you need to set the connection type to PlayCover in the config file
        /// and then you can specify the address of MaaTools here.
        #[clap(short, long)]
        addr: Option<String>,
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
        #[clap(short, long, action = clap::ArgAction::Count)]
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
        #[clap(short, long, action = clap::ArgAction::Count)]
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
            addr,
            verbose,
            quiet,
        } => {
            /*------------------- Setup global log level -------------------*/
            unsafe {
                log::set_level(log::level() as u8 + verbose - quiet);
            }

            /*--------------------- Setup MaaCore Dirs ---------------------*/
            let state_dir = get_state_dir(&project).exist_or_create()?;
            debug!("State directory:", state_dir.display());
            Assistant::set_user_dir(&state_dir).context("Failed to set user directory!")?;

            let data_dir = get_data_dir(&project).exist_or_err()?;
            debug!("Data directory:", data_dir.display());
            Assistant::load_resource(&data_dir).context("Failed to load resource!")?;

            /*--------------------- Load Config Files ---------------------*/
            let config_dir = get_config_dir(&project).exist_or_err()?;
            debug!("Config directory:", config_dir.display());

            // asst.toml
            let asst_config =
                AsstConfig::find_file(&config_dir.join("asst")).unwrap_or_else(|_| {
                    warning!("Failed to find asst config file, using default config!");
                    AsstConfig::default()
                });

            // tasks/<task>.toml
            let task_file = config_dir.join("tasks").join(&task);
            let task_list = TaskList::find_file(&task_file).with_context(|| {
                format!(
                    "Failed to find task file {} in {}",
                    task,
                    task_file.display()
                )
            })?;

            /*------------------- Additional resource files ------------------*/
            for resource in asst_config.resources.iter() {
                let path = PathBuf::from(resource);
                let path = if path.is_absolute() {
                    debug!("Loading additional resource:", path.display());
                    path
                } else {
                    debug!("Loading additional resource:", resource);
                    data_dir.join("resource").join(resource)
                };
                Assistant::load_resource(&path).context("Failed to load resource!")?;
            }

            /*----------------------- Process Task --------------------------*/
            let mut task_typs: Vec<TaskType> = Vec::new();
            let mut task_params: Vec<String> = Vec::new();
            let mut start_app: u8 = 0;
            let mut close_app: u8 = 0;
            let mut app_name: &str = "";

            for task in task_list.tasks {
                if task.is_active() {
                    let task_type = task.get_type();
                    let params = &mut task.get_params();

                    match task_type {
                        TaskType::StartUp => {
                            let enable = match params.get("enable") {
                                Some(enable) => {
                                    enable.as_bool().ok_or(anyhow!("key enable must be bool"))?
                                }
                                None => true,
                            };
                            let client_type = match params.get("client_type") {
                                Some(client_type) => client_type
                                    .as_str()
                                    .ok_or(anyhow!("key client_type must be string"))?,
                                None => "",
                            };
                            let start_game = params
                                .get("start_game_enabled")
                                .unwrap_or(&Value::Bool(false))
                                .as_bool()
                                .ok_or(anyhow!("key enable must be bool"))?;
                            if enable && start_game {
                                start_app += 1;
                                app_name = match client_type {
                                    "Official" | "Bilibili" | "txwy" | "" => "明日方舟",
                                    "YoStarEN" => "Arknights",
                                    "YoStarJP" => "アークナイツ",
                                    "YoStarKR" => "명일방주",
                                    _ => {
                                        warning!(
                                            format!("Unknown client type: {}", client_type),
                                            "using default name: 明日方舟"
                                        );
                                        "明日方舟"
                                    }
                                };
                            }
                        }
                        TaskType::CloseDown => {
                            let enable = match params.get("enable") {
                                Some(enable) => {
                                    enable.as_bool().ok_or(anyhow!("key enable must be bool"))?
                                }
                                None => true,
                            };
                            if enable {
                                close_app += 1;
                            }
                        }
                        _ => {
                            // For any task that has a filename parameter
                            // and the filename parameter is not an absolute path,
                            // it will be treated as a relative path to the config directory
                            // and will be converted to an absolute path.
                            if let Some(v) = params.get_mut("filename") {
                                let filename =
                                    v.as_str().ok_or(anyhow!("Filename must be string!"))?;
                                let path = std::path::Path::new(filename);
                                if !path.is_absolute() {
                                    let type_name: &str = task_type.into();
                                    *v = Value::String(
                                        config_dir
                                            .join(type_name.to_lowercase())
                                            .join(path)
                                            .to_str()
                                            .ok_or(anyhow!("Invalid Path!"))?
                                            .to_string(),
                                    );
                                }
                            }
                        }
                    }

                    debug!("Task:", task_type);
                    debug!(
                        "Params:",
                        serde_json::to_string(&params)
                            .map_or_else(|_| "Unknown".to_string(), |s| s)
                    );

                    task_typs.push(task_type.clone());
                    task_params.push(serde_json::to_string(&params)?);
                }
            }

            /* ----------------------- Init Assistant ----------------------*/
            let assistant = Assistant::new(Some(callback), None);

            /* ----------------------- Setup Instance ----------------------*/
            let options = asst_config.instance_options;
            if let Some(v) = options.touch_mode {
                debug!("Setting touch_mode to", v);
                assistant
                    .set_instance_option(2, v)
                    .context("Failed to set touch mode!")?;
            }
            if let Some(v) = options.deployment_with_pause {
                debug!("Setting deployment_with_pause to", v);
                assistant
                    .set_instance_option(3, v)
                    .context("Failed to set deployment with pause!")?;
            }
            if let Some(v) = options.adb_lite_enabled {
                debug!("Setting adb_lite_enabled to", v);
                assistant.set_instance_option(4, v)?;
            }
            if let Some(v) = options.kill_adb_on_exit {
                debug!("Setting kill_adb_on_exit to", v);
                assistant.set_instance_option(5, v)?;
            }

            /*----------------------- Connect to Game ----------------------*/
            let connection = asst_config.connection;
            match connection {
                Connection::ADB {
                    adb_path,
                    device,
                    config,
                } => {
                    if options.touch_mode.is_none() {
                        warning!(
                            "No touch mode specified, set to",
                            asst::TouchMode::default()
                        );
                        assistant
                            .set_instance_option(2, asst::TouchMode::default())
                            .context("Failed to set touch mode!")?;
                    }

                    debug!("Setting adb_path to", &adb_path);
                    debug!("Setting device to", &device);
                    debug!("Setting config to", &config);
                    let adb_device = addr.unwrap_or(device);
                    assistant.async_connect(adb_path, adb_device, config, true)?;
                }
                Connection::PlayTools { address, config } => {
                    let address = addr.unwrap_or(address);
                    debug!("Setting address to", &address);
                    debug!("Setting config to", &config);

                    // NOTE:
                    // If the game is launched from terminal,
                    // there are some connection issues with server.
                    // Even launching the game from another app
                    // which can launch the game successfully,
                    // the connection issues still exist.
                    // I'm not sure if this is a bug of PlayCover or macOS.
                    // But it seems not bug of maa-cli
                    if start_app > 0 {
                        normal!("Starting game...");
                        std::process::Command::new("open")
                            .arg("-a")
                            .arg(app_name)
                            .spawn()
                            .context("Failed to start game!")?
                            .wait()
                            .context("Failed to start game!")?;
                    }
                    close_app += 1;

                    if let Some(v) = options.touch_mode {
                        if v != asst::TouchMode::MacPlayTools {
                            warning!(
                                "Wrong touch mode,",
                                "force set touch_mode to MacPlayTools when using PlayTools"
                            );
                            assistant
                                .set_instance_option(2, asst::TouchMode::MacPlayTools)
                                .context("Failed to set touch mode!")?;
                        }
                    } else {
                        assistant
                            .set_instance_option(2, asst::TouchMode::MacPlayTools)
                            .context("Failed to set touch mode!")?;
                    }

                    // Wait for the game to start
                    std::thread::sleep(std::time::Duration::from_secs(5));

                    assistant.async_connect("", address, config, true)?;
                }
            }

            /* ------------------------- Append Tasks ----------------------*/
            for (i, task_type) in task_typs.iter().enumerate() {
                assistant.append_task(task_type, task_params[i].as_str())?;
            }

            /* ------------------------ Run Assistant ----------------------*/
            assistant.start()?;
            while assistant.running() {
                std::thread::sleep(std::time::Duration::from_millis(5000));
            }
            assistant.stop()?;

            /* ------------------------- Close Game ------------------------*/
            if close_app > 1 {
                let app_name = match app_name {
                    "" => {
                        warning!(
                            "No app name specified",
                            "using default name: 明日方舟, please specify app name in startup task"
                        );
                        "明日方舟"
                    }
                    _ => app_name,
                };
                normal!("Closing game...");
                std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(format!("quit app \"{}\"", app_name))
                    .status()
                    .context("Failed to close game!")?;
            }
        }
        CLI::Version => {
            println!("MaaCore {}", Assistant::get_version()?);
        }
    }

    Ok(ExitCode::SUCCESS)
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
