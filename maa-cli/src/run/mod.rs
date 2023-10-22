mod message;
use message::callback;

use crate::{
    config::{
        asst::{self, AsstConfig, Connection, TouchMode},
        task::{
            task_type::{TaskOrUnknown, TaskType},
            value::input::enable_batch_mode,
            TaskList,
        },
        Error as ConfigError, FindFile,
    },
    dirs::{Dirs, Ensure},
    installer::maa_core::{find_lib_dir, find_resource, MAA_CORE_NAME},
    log::{set_level, LogLevel},
    {debug, normal, warning},
};

use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use maa_sys::Assistant;

pub fn run(
    dirs: &Dirs,
    task: String,
    addr: Option<String>,
    user_resource: bool,
    batch: bool,
    dryrun: bool,
) -> Result<()> {
    if dryrun {
        unsafe { set_level(LogLevel::Debug) };
        debug!("Dryrun mode!");
    }

    if batch {
        unsafe { enable_batch_mode() }
        debug!("Running in batch mode!");
    }

    // Get directories
    let state_dir = dirs.state().ensure()?;
    let config_dir = dirs.config().ensure()?;
    let base_resource_dir = find_resource(dirs).context("Failed to find resource!")?;
    debug!("State Directory:", state_dir.display());
    debug!("Config Directory:", config_dir.display());
    debug!("Base Resource Directory:", base_resource_dir.display());

    /*------------------- Process Asst Config ----------------------*/

    // Load asst config from config_dir/asst.(toml|yaml|json)
    let asst_file = config_dir.join("asst");
    debug!("Finding asst config file:", asst_file.display());
    let asst_config = match AsstConfig::find_file(&asst_file) {
        Ok(config) => config,
        Err(ConfigError::FileNotFound(_)) => {
            warning!("Failed to find asst config file, using default config!");
            AsstConfig::default()
        }
        Err(e) => return Err(e.into()),
    };

    // Process connection
    let mut playtools: bool = false;
    let (adb_path, address, config) = match asst_config.connection {
        Connection::ADB {
            adb_path,
            device,
            config,
        } => {
            let device = addr.unwrap_or(device);
            debug!("Connect to device via ADB");
            debug!("adb_path:", &adb_path);
            debug!("device:", &device);
            debug!("config:", &config);
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

    // Process static options
    let static_options = asst_config.static_options;
    if let Some(v) = static_options.cpu_ocr {
        debug!("Static Option `cpu_ocr`:", v);
    }
    if let Some(v) = static_options.gpu_ocr {
        debug!("Static Option `gpu_ocr`:", v);
    }

    // Process instance options
    let mut instance_options = asst_config.instance_options;
    if let Some(v) = instance_options.touch_mode {
        if playtools && v != asst::TouchMode::MacPlayTools {
            warning!("Force set `touch_mode` to `MacPlayTools` when using `PlayTools`");
            instance_options.touch_mode = Some(TouchMode::MacPlayTools);
        } else {
            debug!("Instance Option `touch_mode`:", v);
        }
    } else if playtools {
        let mode = asst::TouchMode::MacPlayTools;
        debug!("Instance Option `touch_mode`:", mode);
        instance_options.touch_mode = Some(mode);
    } else {
        let mode = asst::TouchMode::default();
        debug!("Instance Option `touch_mode`:", mode);
        instance_options.touch_mode = Some(mode);
    }

    if let Some(v) = instance_options.adb_lite_enabled {
        debug!("Instance Option `adb_lite_enabled`:", v);
    }
    if let Some(v) = instance_options.deployment_with_pause {
        debug!("Instance Option `deployment_with_pause`:", v);
    }
    if let Some(v) = instance_options.kill_adb_on_exit {
        debug!("Instance Option `kill_adb_on_exit`:", v);
    }

    /*----------------------- Process Task -------------------------*/

    // Load task from tasks/<task>.(toml|yaml|json)
    let task_file = config_dir.join("tasks").join(&task);
    debug!("Finding task file:", task_file.display());
    let task_list = TaskList::find_file(&task_file).with_context(|| {
        format!(
            "Failed to find task file {} in {}",
            task,
            task_file.display()
        )
    })?;

    let mut tasks: Vec<String> = Vec::new();
    let mut task_params: Vec<String> = Vec::new();

    let mut start_app: bool = false; // start iOS app before connect
    let mut close_app: bool = false; // close iOS app after disconnect
    let mut app_name: Option<&str> = None;

    let mut client_resource: Option<&str> = None;

    for task in task_list.tasks {
        if task.is_active() {
            let task_type = task.get_type();

            let mut params = task.get_params();
            params.init().context("Failed to init task params!")?;

            match task_type {
                TaskOrUnknown::Task(task_type) => match task_type {
                    TaskType::StartUp => {
                        if playtools
                            && params.get_or("enable", true)?
                            && params.get_or("start_game_enabled", false)?
                        {
                            start_app = true;
                        }

                        if let Some(client_type) = params.get("client_type") {
                            let client_name = String::try_from(client_type)?;
                            let client_type: ClientType = client_name.parse()?;
                            if playtools {
                                app_name = Some(client_type.app_name());
                            };
                            client_resource = client_type.resource();
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
                debug!("PlayCover app:", name);
                Some(PlayCoverApp::new(name))
            }
            None => {
                warning!(
                    "No client type specified,",
                    format!("using default app name {}", "明日方舟")
                );
                Some(PlayCoverApp::new("明日方舟"))
            }
        }
    } else {
        None
    };

    /*----------------------- Process Resource ---------------------*/
    // Resource directorys
    let mut resource_dirs = vec![base_resource_dir.parent().unwrap().to_path_buf()];

    // Client specific resource
    if let Some(resource) = client_resource {
        debug!("Client specific resource:", resource);
        resource_dirs.push(base_resource_dir.join("global").join(resource));
    }

    // Platform specific resource
    if playtools {
        debug!("Platform specific resource:", "iOS");
        resource_dirs.push(base_resource_dir.join("platform_diff/iOS"));
    }

    // User specified additional resource
    for resource in asst_config.resources.iter() {
        let path = PathBuf::from(resource);
        let path = if path.is_absolute() {
            debug!("User specified additional resource:", resource);
            path
        } else {
            base_resource_dir.join(resource)
        };
        if let Some(path) = process_resource_dir(path) {
            resource_dirs.push(path);
        }
    }

    // User resource in config directory
    if user_resource || asst_config.user_resource {
        if let Some(path) = process_resource_dir(config_dir.join("resource")) {
            resource_dirs.push(path);
        }
    }

    /*----------------------- Start Assistant ----------------------*/
    // Load MaaCore
    load_core(dirs);

    // Set user directory (some debug info and cache will be stored here)
    // Must be called any other function (set_static_option, load_resource, etc.)
    Assistant::set_user_dir(state_dir).context("Failed to set user directory!")?;

    // Set static option (this must be called before load_resource and after set_user_dir)
    if static_options.cpu_ocr.is_some_and(|v| v) {
        Assistant::set_static_option(1, true).context("Failed to set static option `cpu_ocr`!")?;
    }
    if let Some(v) = static_options.gpu_ocr {
        Assistant::set_static_option(2, v.to_string())
            .context("Failed to set static option `gpu_ocr`!")?;
    }

    // Load Resource
    for path in resource_dirs.iter() {
        Assistant::load_resource(path)
            .with_context(|| format!("Failed to load resource from {}", path.display()))?;
    }

    // Init Assistant
    let stop_bool = if cfg!(unix) {
        use signal_hook::consts::TERM_SIGNALS;
        use std::sync::Arc;
        let stop_bool = Arc::new(std::sync::atomic::AtomicBool::new(false));
        for sig in TERM_SIGNALS {
            signal_hook::flag::register_conditional_default(*sig, Arc::clone(&stop_bool))
                .context("Failed to register signal handler!")?;
            signal_hook::flag::register(*sig, Arc::clone(&stop_bool))
                .context("Failed to register signal handler!")?;
        }
        Some(stop_bool)
    } else {
        None
    };
    let assistant = Assistant::new(Some(callback), None);

    // Set instance options
    if let Some(v) = instance_options.touch_mode {
        assistant
            .set_instance_option(2, v)
            .context("Failed to set instance option `touch_mode`!")?;
    }
    if let Some(v) = instance_options.deployment_with_pause {
        assistant
            .set_instance_option(3, v)
            .context("Failed to set instance option `deployment_with_pause`!")?;
    }
    if let Some(v) = instance_options.adb_lite_enabled {
        assistant
            .set_instance_option(4, v)
            .context("Failed to set instance option `adb_lite_enabled`!")?;
    }
    if let Some(v) = instance_options.kill_adb_on_exit {
        assistant
            .set_instance_option(5, v)
            .context("Failed to set instance option `kill_adb_on_exit`!")?;
    }

    if !dryrun {
        if start_app {
            app.as_ref().unwrap().open()?;
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        assistant.async_connect(adb_path, address, config, true)?;

        for i in 0..tasks.len() {
            assistant.append_task(tasks[i].as_str(), task_params[i].as_str())?;
        }

        assistant.start()?;
        while assistant.running() {
            if let Some(stop_bool) = stop_bool.as_ref() {
                if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
                    bail!("Interrupted by user!");
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        assistant.stop()?;

        if close_app {
            app.as_ref().unwrap().close();
        }
    }

    // TODO: Better ways to restore signal handlers?
    if let Some(stop_bool) = stop_bool.as_ref() {
        stop_bool.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    Ok(())
}

pub fn core_version<'a>(dirs: &Dirs) -> Result<&'a str> {
    load_core(dirs);

    Ok(Assistant::get_version()?)
}

struct PlayCoverApp<'n> {
    name: &'n str,
}

impl<'n> PlayCoverApp<'n> {
    pub fn new(name: &'n str) -> Self {
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
            .arg(self.name)
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

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Copy)]
enum ClientType {
    Official,
    Bilibili,
    Txwy,
    YoStarEN,
    YoStarJP,
    YoStarKR,
}

impl ClientType {
    pub fn app_name(self) -> &'static str {
        match self {
            ClientType::Official | ClientType::Bilibili | ClientType::Txwy => "明日方舟",
            ClientType::YoStarEN => "Arknights",
            ClientType::YoStarJP => "アークナイツ",
            ClientType::YoStarKR => "명일방주",
        }
    }

    pub fn resource(self) -> Option<&'static str> {
        match self {
            ClientType::Txwy => Some("txwy"),
            ClientType::YoStarEN => Some("YoStarEN"),
            ClientType::YoStarJP => Some("YoStarJP"),
            ClientType::YoStarKR => Some("YoStarKR"),
            _ => None,
        }
    }
}

impl std::str::FromStr for ClientType {
    type Err = ParseClientTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Official" | "" => Ok(ClientType::Official),
            "Bilibili" => Ok(ClientType::Bilibili),
            "txwy" => Ok(ClientType::Txwy),
            "YoStarEN" => Ok(ClientType::YoStarEN),
            "YoStarJP" => Ok(ClientType::YoStarJP),
            "YoStarKR" => Ok(ClientType::YoStarKR),
            _ => Err(ParseClientTypeError::UnknownClientType),
        }
    }
}

#[derive(Debug)]
enum ParseClientTypeError {
    UnknownClientType,
}

impl std::fmt::Display for ParseClientTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseClientTypeError::UnknownClientType => write!(f, "Unknown client type!"),
        }
    }
}

impl std::error::Error for ParseClientTypeError {}

fn process_resource_dir(path: PathBuf) -> Option<PathBuf> {
    let path = if path.ends_with("resource") {
        path
    } else {
        path.join("resource")
    };
    if path.is_dir() {
        Some(path.parent().unwrap().to_path_buf())
    } else {
        warning!(format!("Resource directory {} not found!", path.display()));
        None
    }
}

fn load_core(dirs: &Dirs) {
    if let Some(lib_dir) = find_lib_dir(dirs) {
        // Set DLL directory on Windows
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

            let lib_dir_w: Vec<u16> = lib_dir.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe { SetDllDirectoryW(lib_dir_w.as_ptr()) };
        }
        maa_sys::binding::load(lib_dir.join(MAA_CORE_NAME));
    } else {
        maa_sys::binding::load(MAA_CORE_NAME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod client_type {
        use super::*;

        #[test]
        fn parse_client() {
            assert_eq!(ClientType::Official, "Official".parse().unwrap());
            assert_eq!(ClientType::Official, "".parse().unwrap());
            assert_eq!(ClientType::Bilibili, "Bilibili".parse().unwrap());
            assert_eq!(ClientType::Txwy, "txwy".parse().unwrap());
            assert_eq!(ClientType::YoStarEN, "YoStarEN".parse().unwrap());
            assert_eq!(ClientType::YoStarJP, "YoStarJP".parse().unwrap());
            assert_eq!(ClientType::YoStarKR, "YoStarKR".parse().unwrap());
        }

        #[test]
        fn client_to_app() {
            assert_eq!(ClientType::Official.app_name(), "明日方舟");
            assert_eq!(ClientType::Bilibili.app_name(), "明日方舟");
            assert_eq!(ClientType::Txwy.app_name(), "明日方舟");
            assert_eq!(ClientType::YoStarEN.app_name(), "Arknights");
            assert_eq!(ClientType::YoStarJP.app_name(), "アークナイツ");
            assert_eq!(ClientType::YoStarKR.app_name(), "명일방주");
        }

        #[test]
        fn client_to_resource() {
            assert_eq!(ClientType::Official.resource(), None);
            assert_eq!(ClientType::Bilibili.resource(), None);
            assert_eq!(ClientType::Txwy.resource(), Some("txwy"));
            assert_eq!(ClientType::YoStarEN.resource(), Some("YoStarEN"));
            assert_eq!(ClientType::YoStarJP.resource(), Some("YoStarJP"));
            assert_eq!(ClientType::YoStarKR.resource(), Some("YoStarKR"));
        }
    }
}
