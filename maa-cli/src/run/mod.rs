use crate::config::{asst, task, Error as ConfigError, FindFile};
use crate::dirs::{Dirs, Ensure};
use crate::installer::maa_core::{find_maa_core, find_resource};
use crate::log::{level, set_level};
use crate::{debug, normal, warning};

use asst::{AsstConfig, Connection};
use task::{TaskList, TaskType};

mod message;
use message::callback;

use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use maa_sys::Assistant;
use serde_json::Value;

pub fn run(
    dirs: &Dirs,
    task: String,
    addr: Option<String>,
    user_resource: bool,
    verbose: u8,
    quiet: u8,
) -> Result<()> {
    let core_path = find_maa_core(dirs).context("Failed to find MaaCore!")?;

    maa_sys::binding::load(core_path);

    /*------------------- Setup global log level -------------------*/
    unsafe {
        set_level(level() as u8 + verbose - quiet);
    }

    /*--------------------- Setup MaaCore Dirs ---------------------*/
    let state_dir = dirs.state().ensure()?;
    debug!("State directory:", state_dir.display());
    Assistant::set_user_dir(state_dir).context("Failed to set user directory!")?;

    let data_dir = find_resource(dirs).context("Failed to find resource!")?;
    debug!("Data directory:", data_dir.display());
    Assistant::load_resource(data_dir.parent().unwrap()).context("Failed to load resource!")?;

    /*--------------------- Load Config Files ---------------------*/
    let config_dir = dirs.config();
    if !config_dir.exists() {
        bail!("Config directory not exists!");
    }
    debug!("Config directory:", config_dir.display());

    // asst.toml
    let asst_config = match AsstConfig::find_file(&config_dir.join("asst")) {
        Ok(config) => config,
        Err(ConfigError::FileNotFound(_)) => {
            warning!("Failed to find asst config file, using default config!");
            AsstConfig::default()
        }
        Err(e) => return Err(e.into()),
    };

    // tasks/<task>.toml
    let task_file = config_dir.join("tasks").join(&task);
    let task_list = TaskList::find_file(&task_file).with_context(|| {
        format!(
            "Failed to find task file {} in {}",
            task,
            task_file.display()
        )
    })?;

    /*------------------ Additional resource files -----------------*/
    for resource in asst_config.resources.iter() {
        let path = PathBuf::from(resource);
        let path = if path.is_absolute() {
            debug!("Loading additional resource:", path.display());
            path
        } else {
            debug!("Loading additional resource:", resource);
            data_dir.join(resource)
        };
        Assistant::load_resource(&path)
            .with_context(|| format!("Failed to load additional resource {}!", path.display()))?;
    }

    if user_resource {
        if config_dir.join("resource").exists() {
            debug!("Loading user resource:", config_dir.display());
            Assistant::load_resource(config_dir).context("Failed to load user resource!")?;
        } else {
            warning!("`--user-resource` is specified, but no user resource found!");
        }
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
                        let filename = v.as_str().ok_or(anyhow!("Filename must be string!"))?;
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
                serde_json::to_string(&params).map_or_else(|_| "Unknown".to_string(), |s| s)
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

    Ok(())
}

pub fn core_version<'a>(dirs: &Dirs) -> Result<&'a str> {
    let core_path = find_maa_core(dirs).context("Failed to find MaaCore!")?;
    maa_sys::binding::load(core_path);

    Ok(Assistant::get_version()?)
}
