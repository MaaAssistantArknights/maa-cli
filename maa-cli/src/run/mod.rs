// mod message;
// use message::callback;
//
mod callback;
use callback::summary;

#[cfg(target_os = "macos")]
mod playcover;

pub mod preset;

use crate::{
    config::{asst::AsstConfig, task::TaskConfig, FindFile},
    dirs::{self, maa_lib_name, Ensure},
    installer::resource,
};

use std::{
    path::Path,
    sync::{atomic, Arc},
};

use anyhow::{bail, Context, Result};
use clap::Args;
use log::{debug, warn};
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
    /// Profile (asst config file) name
    ///
    /// A profile is a config file that contains the configuration passed to MaaCore.
    /// By default, we will try to load the config file `$MAA_CONFIG_DIR/profiles/default.toml`.
    /// If the file does not exist, we will try to load the config file `$MAA_CONFIG_DIR/asst.toml`
    /// for backward compatibility, which is the old config file name.
    /// If you want to use another config file, you can specify the profile name here.
    /// The config file should be placed in the directory `$MAA_CONFIG_DIR/profiles/`.
    #[arg(short, long, verbatim_doc_comment)]
    pub profile: Option<String>,
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

fn find_profile(root: impl AsRef<Path>, profile: Option<&str>) -> Result<AsstConfig> {
    let root = root.as_ref();
    if let Some(profile) = profile {
        AsstConfig::find_file(join!(root, "profiles", profile))
            .context("Failed to find profile file!")
    } else if let Some(config) = AsstConfig::find_file_or_none(join!(root, "profiles", "default"))?
    {
        Ok(config)
    } else if let Some(config) = AsstConfig::find_file_or_none(join!(root, "asst"))? {
        warn!("The config file `asst.toml` is deprecated, please use `profiles/default.toml` instead!");
        Ok(config)
    } else {
        Ok(AsstConfig::default())
    }
}

fn run_core<F>(f: F, args: CommonArgs) -> Result<()>
where
    F: FnOnce(&AsstConfig) -> Result<TaskConfig>,
{
    // Auto update hot update resource
    resource::update(true)?;

    // Load asst config
    let mut asst_config = find_profile(dirs::config(), args.profile.as_deref())?;

    args.apply_to(&mut asst_config);

    let task = f(&asst_config)?;
    let task_config = task.init()?;
    if let Some(client_type) = task_config.client_type {
        debug!("Detected client type: {}", client_type);
        if let Some(resource) = client_type.resource() {
            asst_config.resource.use_global_resource(resource);
        }
    }

    // Load and setup MaaCore
    load_core().context("Failed to load MaaCore!")?;
    setup_core(&asst_config)?;

    // Register signal handlers
    let stop_bool = Arc::new(std::sync::atomic::AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        signal_hook::flag::register_conditional_default(*sig, Arc::clone(&stop_bool))
            .context("Failed to register signal handler!")?;
        signal_hook::flag::register(*sig, Arc::clone(&stop_bool))
            .context("Failed to register signal handler!")?;
    }

    // Create and setup Assistant
    let asst = Assistant::new(Some(callback::default_callback), None);
    asst_config.instance_options.apply_to(&asst)?;

    // Register tasks
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
            s.insert(id, name.map(|s| s.to_owned()), task_type);
        }
    }
    if let Some(s) = summarys {
        summary::init(s);
    }

    // Prepare connection
    let (adb, addr, config) = asst_config.connection.connect_args();

    // Launch external app like PlayCover or Emulator
    // Only support PlayCover on macOS now, may support more in the future
    #[cfg(target_os = "macos")]
    let app = match asst_config.connection.preset() {
        crate::config::asst::Preset::PlayCover => playcover::PlayCoverApp::new(
            task_config.start_app,
            task_config.close_app,
            task_config.client_type.unwrap_or_default(),
            addr.as_ref(),
        ),
        _ => None,
    };

    if !args.dry_run {
        // Startup external app
        #[cfg(target_os = "macos")]
        let rt = Runtime::new().context("Failed to create tokio runtime")?;
        #[cfg(target_os = "macos")]
        if let Some(app) = app.as_ref() {
            rt.block_on(app.open())?;
        }

        // Connect to game or emulator
        asst.async_connect(adb, addr.as_ref(), config, true)?;

        asst.start()?;

        while asst.running() {
            if stop_bool.load(atomic::Ordering::Relaxed) {
                bail!("Interrupted by user!");
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        asst.stop()?;

        // Close external app
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

    ret?;

    if callback::MAA_CORE_ERRORED.load(atomic::Ordering::Relaxed) {
        bail!("Some error occurred during running task!");
    }

    Ok(())
}

pub fn run_custom(path: impl AsRef<Path>, args: CommonArgs) -> Result<()> {
    run(
        |_| {
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

pub fn core_version<'a>() -> Result<&'a str> {
    load_core()?;

    Assistant::get_version().context("Failed to get MaaCore version!")
}

fn load_core() -> Result<()> {
    if maa_sys::binding::loaded() {
        debug!("MaaCore already loaded");
        return Ok(());
    }

    if let Some(lib_dir) = dirs::find_library() {
        debug!("Loading MaaCore from: {}", lib_dir.display());
        // Set DLL directory on Windows
        #[cfg(target_os = "windows")]
        {
            use windows::core::HSTRING;
            use windows::Win32::System::LibraryLoader::SetDllDirectoryW;

            unsafe { SetDllDirectoryW(&HSTRING::from(lib_dir.as_ref()))? };
        }
        maa_sys::binding::load(lib_dir.join(maa_lib_name()))
    } else {
        debug!("MaaCore not found, trying to load from system library path");
        maa_sys::binding::load(maa_lib_name())
    }
    .context("Failed to load MaaCore!")?;

    Ok(())
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

    use std::env::{self, temp_dir};

    #[test]
    fn version() {
        if let Some(version) = env::var_os("MAA_CORE_VERSION") {
            assert_eq!(core_version().unwrap(), version);
        }
    }

    #[test]
    fn test_find_profile() {
        let test_dir = temp_dir().join("maa_test_find_profile");
        test_dir.ensure_clean().unwrap();

        let sample_str = r#"
            [connection]
            address = "test_addr"
        "#;

        let sample_config = {
            let mut config = AsstConfig::default();
            config.connection.set_address("test_addr");
            config
        };

        assert_eq!(
            find_profile(&test_dir, None).unwrap(),
            AsstConfig::default()
        );

        let backcompat_path = test_dir.join("asst.toml");
        let default_path = test_dir.join("profiles").join("default.toml");
        let test_path = test_dir.join("profiles").join("test.toml");

        std::fs::write(&backcompat_path, sample_str).unwrap();
        assert_eq!(find_profile(&test_dir, None).unwrap(), sample_config);
        std::fs::remove_file(&backcompat_path).unwrap();

        std::fs::create_dir(test_dir.join("profiles")).unwrap();

        std::fs::write(&default_path, sample_str).unwrap();
        assert_eq!(find_profile(&test_dir, None).unwrap(), sample_config);
        std::fs::remove_file(&default_path).unwrap();

        std::fs::write(&test_path, sample_str).unwrap();
        assert_eq!(
            find_profile(&test_dir, None).unwrap(),
            AsstConfig::default()
        );
        assert_eq!(
            find_profile(&test_dir, Some("test")).unwrap(),
            sample_config
        );
        std::fs::remove_file(&test_path).unwrap();

        std::fs::remove_dir_all(&test_dir).unwrap();
    }
}
