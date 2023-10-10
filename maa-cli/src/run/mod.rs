mod message;
use message::callback;

use crate::{
    config::{
        asst::{self, AsstConfig, Connection},
        task::{
            task_type::{TaskOrUnknown, TaskType},
            value::input::enable_batch_mode,
            TaskList, Value,
        },
        Error as ConfigError, FindFile,
    },
    dirs::{Dirs, Ensure},
    installer::maa_core::{find_maa_core, find_resource},
    log::{level, set_level},
    {debug, error, normal, warning},
};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use maa_sys::Assistant;
use signal_hook::consts::TERM_SIGNALS;

pub fn run(
    dirs: &Dirs,
    task: String,
    addr: Option<String>,
    user_resource: bool,
    verbose: u8,
    quiet: u8,
    batch: bool,
) -> Result<()> {
    let core_path = find_maa_core(dirs).context("Failed to find MaaCore!")?;

    maa_sys::binding::load(core_path);

    /*------------------- Setup global log level -------------------*/
    unsafe {
        if batch {
            enable_batch_mode();
        }
        set_level(level() as u8 + verbose - quiet);
    }

    /*--------------------- Setup MaaCore Dirs ---------------------*/
    let state_dir = dirs.state().ensure()?;
    debug!("State directory:", state_dir.display());
    Assistant::set_user_dir(state_dir).context("Failed to set user directory!")?;

    let resource_dir = find_resource(dirs).context("Failed to find resource!")?;
    debug!("Resources directory:", resource_dir.display());
    Assistant::load_resource(resource_dir.parent().unwrap()).context("Failed to load resource!")?;

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

    /*--------------------- Process Connection ---------------------*/
    let mut playtools: bool = false;
    let (adb_path, address, config) = match asst_config.connection {
        Connection::ADB {
            adb_path,
            device,
            config,
        } => {
            debug!("Setting adb_path to", &adb_path);
            debug!("Setting device to", &device);
            debug!("Setting config to", &config);
            (adb_path, device, config)
        }
        Connection::PlayTools { address, config } => {
            playtools = true;
            let address = addr.unwrap_or(address);
            debug!("Setting address to", &address);
            debug!("Setting config to", &config);
            (String::new(), address, config)
        }
    };

    /*----------------------- Process Task -------------------------*/
    let mut tasks: Vec<String> = Vec::new();
    let mut task_params: Vec<String> = Vec::new();

    let mut start_app: bool = false; // start iOS app before connect
    let mut close_app: bool = false; // close iOS app after disconnect
    let mut app_name: Option<String> = None;

    for task in task_list.tasks {
        if task.is_active() {
            let task_type = task.get_type();

            let mut params = task.get_params();
            params.init().context("Failed to init task params!")?;

            match task_type {
                TaskOrUnknown::Task(task_type) => match task_type {
                    TaskType::StartUp if playtools => {
                        if params.get_or("enable", true)?
                            && params.get_or("start_game_enabled", false)?
                        {
                            start_app = true;
                        }

                        if let Some(client_type) = params.get("client_type") {
                            app_name = Some(client_name(client_type, &resource_dir)?);
                        };
                    }
                    TaskType::CloseDown if playtools => {
                        close_app = params.get_or("enable", true)?;
                    }
                    _ => {
                        // For any task that has a filename parameter
                        // and the filename parameter is not an absolute path,
                        // it will be treated as a relative path to the config directory
                        // and will be converted to an absolute path.
                        if let Some(v) = params.get("filename") {
                            let filename = String::try_from(v)?;
                            let path = std::path::Path::new(&filename);
                            if !path.is_absolute() {
                                let type_name: &str = task_type.as_ref();
                                params.insert(
                                    "filename",
                                    config_dir
                                        .join(type_name.to_lowercase())
                                        .join(path)
                                        .to_str()
                                        .ok_or(anyhow!("Invalid Path!"))?,
                                );
                            }
                        }
                    }
                },
                TaskOrUnknown::Unknown(_) => (),
            }

            let task_str = task_type.as_ref();
            let param_str = serde_json::to_string(&params)?;

            debug!("Task:", task_str);
            debug!("Params:", param_str);

            tasks.push(task_str.into());
            task_params.push(param_str);
        }
    }

    let app = if start_app || close_app {
        match app_name {
            Some(name) => {
                debug!("Using PlayCover to launch app", name);
                Some(PlayCoverApp::new(name))
            }
            None => {
                warning!(
                    "No client type specified, ",
                    format!("using default app name {}", "明日方舟")
                );
                Some(PlayCoverApp::from("明日方舟"))
            }
        }
    } else {
        None
    };

    /*------------------- Load Additional resource -----------------*/
    if playtools {
        debug!("Load additional resource for PlayTools");
        Assistant::load_resource(resource_dir.join("platform_diff/iOS"))
            .context("Failed to load additional resource!")?;
    }

    for resource in asst_config.resources.iter() {
        let path = PathBuf::from(resource);
        let path = if path.is_absolute() {
            debug!("Loading additional resource:", path.display());
            path
        } else {
            debug!("Loading additional resource:", resource);
            resource_dir.join(resource)
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

    /*------------------------ Init Assistant ----------------------*/
    let stop_bool = Arc::new(std::sync::atomic::AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        signal_hook::flag::register_conditional_default(*sig, Arc::clone(&stop_bool))
            .context("Failed to register signal handler!")?;
        signal_hook::flag::register(*sig, Arc::clone(&stop_bool))
            .context("Failed to register signal handler!")?;
    }
    let assistant = Assistant::new(Some(callback), None);

    /*------------------------ Setup Instance ----------------------*/
    let options = asst_config.instance_options;
    if let Some(v) = options.touch_mode {
        if playtools && v != asst::TouchMode::MacPlayTools {
            warning!(
                "Wrong touch mode,",
                "force set touch_mode to MacPlayTools when using PlayTools"
            );
            assistant
                .set_instance_option(2, asst::TouchMode::MacPlayTools)
                .context("Failed to set touch mode!")?;
        } else {
            debug!("Setting touch_mode to", v);
            assistant
                .set_instance_option(2, v)
                .context("Failed to set touch mode!")?;
        }
    } else if playtools {
        debug!("Setting touch_mode to MacPlayTools");
        assistant
            .set_instance_option(2, asst::TouchMode::MacPlayTools)
            .context("Failed to set touch mode!")?;
    } else {
        let mode = asst::TouchMode::default();
        warning!(
            "No touch mode specified,",
            format!("using default touch mode {}.", mode)
        );
        assistant
            .set_instance_option(2, mode)
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
    if start_app {
        app.as_ref().unwrap().open()?;
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    assistant.async_connect(adb_path, address, config, true)?;

    /* ------------------------- Append Tasks ----------------------*/
    for i in 0..tasks.len() {
        assistant.append_task(tasks[i].as_str(), task_params[i].as_str())?;
    }

    /* ------------------------ Run Assistant ----------------------*/
    assistant.start()?;
    while assistant.running() {
        if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
            bail!("Interrupted by user!");
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    assistant.stop()?;

    // TODO: Better ways to restore signal handlers?
    stop_bool.store(true, std::sync::atomic::Ordering::Relaxed);

    /* ------------------------- Close Game ------------------------*/
    if close_app {
        app.as_ref().unwrap().close();
    }

    Ok(())
}

pub fn core_version<'a>(dirs: &Dirs) -> Result<&'a str> {
    let core_path = find_maa_core(dirs).context("Failed to find MaaCore!")?;

    maa_sys::binding::load(core_path);

    Ok(Assistant::get_version()?)
}

struct PlayCoverApp {
    name: String,
}

impl PlayCoverApp {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn open(&self) -> Result<()> {
        // NOTE:
        // If the game is launched from terminal,
        // there are some connection issues with server.
        // Even launching the game from another app
        // which can launch the game successfully,
        // the connection issues still exist.
        // I'm not sure if this is a bug of PlayCover or macOS.
        // But it seems not bug of maa-cli
        normal!("Starting game...");
        std::process::Command::new("open")
            .arg("-a")
            .arg(&self.name)
            .status()
            .context("Failed to start game!")?;
        Ok(())
    }

    pub fn close(&self) {
        normal!("Closing game...");
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!("quit app \"{}\"", self.name))
            .status()
            .expect("Failed to close game!");
    }
}

impl From<&str> for PlayCoverApp {
    fn from(name: &str) -> Self {
        Self::new(name.into())
    }
}

fn client_name(client: &Value, resource_dir: &Path) -> Result<String> {
    let client = String::try_from(client)?;

    let (resource, app) = match client.as_str() {
        "Official" | "Bilibili" | "" => (None, None),
        "txwy" => (Some("txwy"), None),
        "YoStarEN" => (Some("YoStarEN"), Some("Arknights")),
        "YoStarJP" => (Some("YoStarJP"), Some("アークナイツ")),
        "YoStarKR" => (Some("YoStarKR"), Some("명일방주")),
        _ => {
            error!("Unknown client type", client);
            (None, None)
        }
    };

    if let Some(resource) = resource {
        debug!("Loading additional resource for global client", resource);
        Assistant::load_resource(resource_dir.join("global").join(resource))
            .context("Failed to load additional resource!")?;
    }

    Ok(app.unwrap_or("明日方舟").to_string())
}
