mod message;
use message::callback;

mod playcover;
use playcover::PlayCoverApp;

mod fight;
pub use fight::fight;

mod copilot;
pub use copilot::copilot;

use crate::{
    config::{
        asst::{with_asst_config, with_mut_asst_config, AsstConfig},
        task::TaskConfig,
    },
    consts::MAA_CORE_LIB,
    debug,
    dirs::{self, Ensure},
    installer::resource,
    log::{set_level, LogLevel},
};

use std::sync::{atomic, Arc};

use anyhow::{bail, Context, Result};
use clap::Args;
use maa_sys::Assistant;
use signal_hook::consts::TERM_SIGNALS;

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

pub fn run<F>(f: F, args: CommonArgs) -> Result<()>
where
    F: FnOnce(&AsstConfig) -> Result<TaskConfig>,
{
    if args.dry_run {
        unsafe { set_level(LogLevel::Debug) };
    }

    resource::update(true)?;

    // Prepare config
    with_mut_asst_config(|config| args.apply_to(config));
    let task = with_asst_config(f)?;
    let task_config = task.init()?;
    if let Some(client_type) = task_config.client_type {
        debug!("Detected client type:", client_type.as_ref());
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

    let asst = Assistant::new(Some(callback), None);

    with_asst_config(|config| config.instance_options.apply_to(&asst))?;

    for (task_type, params) in task_config.tasks.iter() {
        debug!(
            format!("Adding task {} with params:", task_type.as_ref()),
            serde_json::to_string_pretty(params)?
        );
        asst.append_task(task_type, serde_json::to_string(params)?)?;
    }

    let playcover = PlayCoverApp::from(&task_config);

    if let Some(app) = playcover.as_ref() {
        app.open()?;
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

    if let Some(app) = playcover.as_ref() {
        app.close()?;
    }

    // TODO: Better ways to restore signal handlers?
    stop_bool.store(true, atomic::Ordering::Relaxed);

    Ok(())
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

fn setup_core(config: &AsstConfig) -> Result<()> {
    debug!("Setting user directory:", dirs::state().display());
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
