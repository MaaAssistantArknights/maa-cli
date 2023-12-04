mod message;
use message::callback;

mod playcover;
use playcover::PlayCoverApp;

mod fight;
pub use fight::fight;

use crate::{
    config::{
        asst::{AsstConfig, ConnectionConfig},
        task::{InitializedTaskConfig, TaskConfig},
        Error as ConfigError, FindFile,
    },
    consts::MAA_CORE_LIB,
    dirs::{self, Ensure},
    log::{set_level, LogLevel},
    {debug, warning},
};

use std::sync::{atomic, Arc};

use anyhow::{bail, Context, Result};
use clap::Parser;
use maa_sys::Assistant;
use signal_hook::consts::TERM_SIGNALS;

#[derive(Parser, Default)]
pub struct CommonArgs {
    /// ADB serial number of device or MaaTools address set in PlayCover
    ///
    /// By default, MaaCore connects to game with ADB,
    /// and this parameter is the serial number of the device
    /// (default to `emulator-5554` if not specified here and not set in config file).
    /// And if you want to use PlayCover,
    /// you need to set the connection type to PlayCover in the config file
    /// and then you can specify the address of MaaTools here.
    #[arg(short, long, verbatim_doc_comment)]
    pub addr: Option<String>,
    /// Load resources from the config directory
    ///
    /// By default, MaaCore loads resources from the resource installed with MaaCore.
    /// If you want to modify some configuration of MaaCore or you want to use your own resources,
    /// you can use this option to load resources from the `resource` directory,
    /// which is a subdirectory of the config directory.
    ///
    /// This option can also be enabled by setting the value of the key `user_resource` to true
    /// in the asst configure file `$MAA_CONFIG_DIR/asst.toml`.
    ///
    /// Note:
    /// CLI will load resources shipped with MaaCore firstly,
    /// then some client specific or platform specific when needed,
    /// lastly, it will load resources from the config directory.
    /// MaaCore will overwrite the resources loaded before,
    /// if there are some resources with the same name.
    /// Use at your own risk!
    #[arg(long, verbatim_doc_comment)]
    pub user_resource: bool,
    /// Parse the your config but do not connect to the game
    ///
    /// This option is useful when you want to check your config file.
    /// It will parse your config file and set the log level to debug.
    /// If there are some errors in your config file,
    /// it will print the error message and exit.
    #[arg(long, verbatim_doc_comment)]
    pub dry_run: bool,
}

pub struct AsstInstanceBuilder {
    asst_config: AsstConfig,
    task_config: InitializedTaskConfig,
}

impl AsstInstanceBuilder {
    fn new<T>(task: T, args: &CommonArgs) -> Result<Self>
    where
        T: TryInto<TaskConfig>,
        T::Error: std::error::Error + Send + Sync + 'static,
    {
        let asst_file = dirs::config().join("asst");
        debug!("Finding asst config file:", asst_file.display());
        let mut asst_config = match AsstConfig::find_file(&asst_file) {
            Ok(config) => config,
            Err(ConfigError::FileNotFound(_)) => {
                warning!("Failed to find asst config file, using default config!");
                AsstConfig::default()
            }
            Err(e) => return Err(e.into()),
        };

        if matches!(asst_config.connection, ConnectionConfig::PlayTools { .. }) {
            asst_config.resource.use_platform_diff_resource("iOS");
        }

        let task_config: TaskConfig = task.try_into()?;
        let task_config = task_config.init()?;

        if let Some(client_type) = task_config.client_type() {
            if let Some(resource) = client_type.resource() {
                asst_config.resource.use_global_resource(resource);
            }
        }

        if args.user_resource {
            asst_config.resource.use_user_resource();
        }

        if let Some(addr) = args.addr.as_ref() {
            asst_config.connection.set_address(addr);
        }

        Ok(Self {
            asst_config,
            task_config,
        })
    }

    fn build(self) -> Result<AsstInstance> {
        load_core();

        debug!("Setting user directory:", dirs::state().display());
        Assistant::set_user_dir(dirs::state().ensure()?)
            .context("Failed to set user directory!")?;

        self.asst_config.static_options.apply()?;

        self.asst_config.resource.load()?;

        let stop_bool = Arc::new(std::sync::atomic::AtomicBool::new(false));
        for sig in TERM_SIGNALS {
            signal_hook::flag::register_conditional_default(*sig, Arc::clone(&stop_bool))
                .context("Failed to register signal handler!")?;
            signal_hook::flag::register(*sig, Arc::clone(&stop_bool))
                .context("Failed to register signal handler!")?;
        }

        let asst = Assistant::new(Some(callback), None);

        let mut instance_options = self.asst_config.instance_options;
        if matches!(
            self.asst_config.connection,
            ConnectionConfig::PlayTools { .. }
        ) {
            instance_options.force_playtools();
        }

        instance_options.apply(&asst)?;

        let task_config = &self.task_config;

        for (task_type, params) in task_config.tasks() {
            // debug!(
            //     format!("Adding task {} with params", task_type.as_ref()),
            //     serde_json::to_string_pretty(params)?
            // );
            asst.append_task(task_type, serde_json::to_string(params)?)?;
        }

        Ok(AsstInstance {
            asst,
            stop_bool,
            playcover: PlayCoverAppConfig::new(task_config),
            connection: self.asst_config.connection,
        })
    }
}

struct PlayCoverAppConfig {
    app: PlayCoverApp<'static>,
    start_app: bool,
    close_app: bool,
}

impl PlayCoverAppConfig {
    pub fn new(task_config: &InitializedTaskConfig) -> Option<Self> {
        if task_config.start_app() || task_config.close_app() {
            let app = if let Some(client_type) = task_config.client_type() {
                let app = PlayCoverApp::from(client_type);
                debug!("PlayCover app:", app.name());
                app
            } else {
                let app = PlayCoverApp::default();
                warning!(
                    "No client type specified,",
                    format!("using default app name {}", app.name())
                );
                app
            };
            Some(Self {
                app,
                start_app: task_config.start_app(),
                close_app: task_config.close_app(),
            })
        } else {
            None
        }
    }

    pub fn open(&self) -> Result<()> {
        if self.start_app {
            self.app.open()?;
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        if self.close_app {
            self.app.close()?;
        }
        Ok(())
    }
}

pub struct AsstInstance {
    asst: Assistant,
    stop_bool: Arc<std::sync::atomic::AtomicBool>,
    playcover: Option<PlayCoverAppConfig>,
    connection: ConnectionConfig,
}

impl AsstInstance {
    pub fn connect(&self) -> Result<()> {
        if let Some(app) = self.playcover.as_ref() {
            app.open()?;
        }

        self.connection.connect(&self.asst)?;

        Ok(())
    }

    pub fn start(&self) -> Result<()> {
        self.asst.start()?;

        while self.asst.running() {
            if self.stop_bool.load(atomic::Ordering::Relaxed) {
                bail!("Interrupted by user!");
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        self.asst.stop()?;

        if let Some(app) = self.playcover.as_ref() {
            app.close()?;
        }

        Ok(())
    }
}

impl Drop for AsstInstance {
    fn drop(&mut self) {
        self.stop_bool.store(true, atomic::Ordering::Relaxed);
    }
}

pub fn run<T: TryInto<TaskConfig>>(task: T, args: CommonArgs) -> Result<()>
where
    T::Error: std::error::Error + Send + Sync + 'static,
{
    if args.dry_run {
        unsafe { set_level(LogLevel::Debug) };
    }

    let builder = AsstInstanceBuilder::new(task, &args)?;
    let instance = builder.build()?;

    if !args.dry_run {
        instance.connect()?;
        instance.start()?;
    }

    Ok(())
}

pub fn core_version<'a>() -> Result<&'a str> {
    load_core();

    Ok(Assistant::get_version()?)
}

fn load_core() {
    if let Some(lib_dir) = dirs::find_library() {
        debug!("Loading MaaCore from:", lib_dir.display());
        // Set DLL directory on Windows
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

            let lib_dir_w: Vec<u16> = lib_dir.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe { SetDllDirectoryW(lib_dir_w.as_ptr()) };
        }
        maa_sys::binding::load(lib_dir.join(MAA_CORE_LIB));
    } else {
        debug!("MaaCore not found, trying to load from system library path");
        maa_sys::binding::load(MAA_CORE_LIB);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod run {
        use std::env;

        use super::*;

        #[test]
        fn version() {
            if let Some(version) = env::var_os("MAA_CORE_VERSION") {
                assert_eq!(core_version().unwrap(), version);
            }
        }
    }
}
