// mod message;
// use message::callback;
//
mod callback;
use callback::summary;

#[cfg(target_os = "macos")]
mod playcover;

mod fight;
pub use fight::fight;

mod copilot;
pub use copilot::copilot;

mod roguelike;
pub use roguelike::{roguelike, Theme as RoguelikeTheme};

use crate::{
    config::{
        asst::{with_asst_config, with_mut_asst_config, AsstConfig},
        task::TaskConfig,
    },
    consts::MAA_CORE_LIB,
    dirs::{self, Ensure},
    installer::resource,
};

use std::sync::{atomic, Arc};

use anyhow::{bail, Context, Result};
use clap::Args;
use log::debug;
use maa_sys::Assistant;
use signal_hook::consts::TERM_SIGNALS;

#[cfg(target_os = "macos")]
use tokio::runtime::Runtime;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Args, Default)]
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
    /// Do not display task summary
    ///
    /// By default, maa will display task summary after all tasks are finished.
    /// If you want to disable this behavior, you can use this option.
    #[arg(long, verbatim_doc_comment)]
    pub no_summary: bool,
}

impl CommonArgs {
    pub fn apply_to(&self, config: &mut AsstConfig) {
        if let Some(addr) = self.addr.as_ref() {
            config.connection.set_address(addr);
        }

        if self.user_resource {
            config.resource.use_user_resource();
        }
    }
}

fn run_core<F>(f: F, args: CommonArgs) -> Result<()>
where
    F: FnOnce(&AsstConfig) -> Result<TaskConfig>,
{
    resource::update(true)?;

    // Prepare config
    with_mut_asst_config(|config| args.apply_to(config));
    let task = with_asst_config(f)?;
    let task_config = task.init()?;
    if let Some(client_type) = task_config.client_type {
        debug!("Detected client type: {}", client_type.as_ref());
        if let Some(resource) = client_type.resource() {
            with_mut_asst_config(|config| {
                config.resource.use_global_resource(resource);
            });
        }
    }

    load_core();
    with_asst_config(setup_core)?;

    let stop_bool = Arc::new(std::sync::atomic::AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        signal_hook::flag::register_conditional_default(*sig, Arc::clone(&stop_bool))
            .context("Failed to register signal handler!")?;
        signal_hook::flag::register(*sig, Arc::clone(&stop_bool))
            .context("Failed to register signal handler!")?;
    }

    let asst = Assistant::new(Some(callback::default_callback), None);

    with_asst_config(|config| config.instance_options.apply_to(&asst))?;

    let mut summarys = (!args.no_summary).then(summary::Summary::new);
    for task in task_config.tasks.iter() {
        let name = task.name();
        let task_type = task.task_type();
        let params = task.params();
        debug!(
            "Adding task [{}] with params: {}",
            name.unwrap_or(task_type.as_ref()),
            serde_json::to_string_pretty(params)?
        );
        let id = asst.append_task(task_type, serde_json::to_string(params)?)?;

        if let Some(s) = summarys.as_mut() {
            s.insert(id, name.map(|s| s.to_owned()), task_type.clone());
        }
    }
    if let Some(s) = summarys {
        summary::init(s);
    }

    #[cfg(target_os = "macos")]
    let app = with_asst_config(|config| {
        use crate::config::asst::ConnectionConfig::PlayTools;
        if let PlayTools { ref address, .. } = config.connection {
            playcover::PlayCoverApp::new(
                task_config.start_app,
                task_config.close_app,
                task_config.client_type.unwrap_or_default(),
                address.to_owned(),
            )
        } else {
            None
        }
    });

    if !args.dry_run {
        #[cfg(target_os = "macos")]
        let rt = Runtime::new().context("Failed to create tokio runtime")?;

        #[cfg(target_os = "macos")]
        if let Some(app) = app.as_ref() {
            rt.block_on(app.open())?;
        }

        with_asst_config(|config| {
            let (adb, addr, config) = config.connection.connect_args();
            asst.async_connect(adb, addr, config, true)
        })?;

        asst.start()?;

        while asst.running() {
            if stop_bool.load(atomic::Ordering::Relaxed) {
                bail!("Interrupted by user!");
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        asst.stop()?;

        #[cfg(target_os = "macos")]
        if let Some(app) = app.as_ref() {
            rt.block_on(app.close())?;
        }
    }

    // TODO: Better ways to restore signal handlers?
    stop_bool.store(true, atomic::Ordering::Relaxed);

    Ok(())
}

// Wrapper for run_core, always try to display summary even if error occurred
// It's safe to display summary even if summary is not initialized
pub fn run<F>(f: F, args: CommonArgs) -> Result<()>
where
    F: FnOnce(&AsstConfig) -> Result<TaskConfig>,
{
    let ret = run_core(f, args);
    summary::display();
    ret
}

pub fn run_custom(path: impl AsRef<std::path::Path>, args: CommonArgs) -> Result<()> {
    run(
        |_| {
            use crate::config::FindFile;

            let path = path.as_ref();
            if let Some(abs_path) = dirs::abs_config(path, Some("tasks")) {
                TaskConfig::find_file(abs_path)
            } else {
                TaskConfig::find_file(path)
            }
            .context("Failed to find task file!")
        },
        args,
    )
}

pub fn core_version<'a>() -> Result<&'a str, maa_sys::Error> {
    load_core();

    Assistant::get_version()

    // BUG:
    // if we call maa_sys::binding::unload() here,
    // program will crash with signal SIGSEGV (Address boundary error)
    // So we don't unload MaaCore
}

fn load_core() {
    if maa_sys::binding::loaded() {
        debug!("MaaCore already loaded");
        return;
    }

    if let Some(lib_dir) = dirs::find_library() {
        debug!("Loading MaaCore from: {}", lib_dir.display());
        // Set DLL directory on Windows
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::Win32::System::LibraryLoader::SetDllDirectoryW;

            let lib_dir_w: Vec<u16> = lib_dir.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe { SetDllDirectoryW(lib_dir_w.as_ptr()) };
        }
        maa_sys::binding::load(lib_dir.join(MAA_CORE_LIB));
    } else {
        debug!("MaaCore not found, trying to load from system library path");
        maa_sys::binding::load(MAA_CORE_LIB);
    }
}

fn setup_core(config: &AsstConfig) -> Result<()> {
    debug!("Setting user directory: {}", dirs::state().display());
    Assistant::set_user_dir(dirs::state().ensure()?).context("Failed to set user directory!")?;

    config.static_options.apply()?;
    config.resource.load()?;

    Ok(())
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
