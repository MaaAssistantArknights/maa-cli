// mod message;
// use message::callback;
//
mod callback;
use callback::summary::{self, SummarySubscriber};

mod external;

pub mod preset;

use std::{
    path::Path,
    sync::{atomic, Arc},
};

use anyhow::{bail, Context, Result};
use clap::Args;
use log::{debug, warn};
use maa_dirs::{self as dirs, Ensure, MAA_CORE_LIB};
use maa_sys::Assistant;
use signal_hook::consts::TERM_SIGNALS;

use crate::{
    config::{asst::AsstConfig, task::TaskConfig, FindFile},
    installer::resource,
};

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

fn run_core<F>(f: F, args: CommonArgs, rx: &mut SummarySubscriber) -> Result<()>
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
    if let Some(resource) = task_config.client_type.resource() {
        asst_config.resource.use_global_resource(resource);
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

    // Register tasks to Assistant and prepare summary
    let task_summary = !args.no_summary;
    for task in task_config.tasks {
        let task_type = task.task_type;
        let params = serde_json::to_string_pretty(&task.params)?;
        debug!(
            "Adding task [{}] with params: {params}",
            task.name_or_default(),
        );
        let id = asst
            .append_task(task_type, params.as_str())
            .with_context(|| {
                format!(
                    "Failed to add task {} with params: {params}",
                    task.name_or_default(),
                )
            })?;

        if task_summary {
            summary::insert(id, task.name, task_type);
        }
    }

    if !args.dry_run {
        // Prepare connection
        let (adb_path, address, config) = asst_config.connection.connect_args();

        // Launch external apps
        let app: Option<Box<dyn external::ExternalApp>> = match asst_config.connection.preset() {
            #[cfg(target_os = "macos")]
            crate::config::asst::Preset::PlayCover => Some(Box::new(external::PlayCoverApp::new(
                task_config.client_type,
                address.as_ref(),
            ))),
            #[cfg(target_os = "linux")]
            crate::config::asst::Preset::Waydroid => {
                Some(Box::new(external::WaydroidApp::new(address.as_ref())))
            }
            _ => None,
        };

        // Startup external app
        let need_reconfigure = if let (Some(app), true) = (app.as_deref(), task_config.start_app) {
            !app.open().context("Failed to open external app")?
        } else {
            false
        };

        let address = if need_reconfigure {
            debug!("Resetting address");
            asst_config.connection.connect_args().1
        } else {
            address.clone()
        };

        // Connect to game or emulator
        asst.async_connect(adb_path, address.as_ref(), config, true)?;

        asst.start()?;

        while asst.running() {
            if stop_bool.load(atomic::Ordering::Relaxed) {
                bail!("Interrupted by user!");
            }
            if let Some(updated) = rx.try_update() {
                print!("{}", updated)
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        asst.stop()?;

        // Close external app
        if let (Some(app), true) = (app.as_deref(), task_config.close_app) {
            app.close().context("Failed to close external app")?;
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
    let mut rx = summary::init_pipe();

    let ret = run_core(f, args, &mut rx);

    summary::display(rx);

    ret?;

    if callback::MAA_CORE_ERRORED.load(atomic::Ordering::Relaxed) {
        bail!("Some error occurred during running task!");
    }

    Ok(())
}

pub fn run_preset(params: impl preset::IntoTaskConfig, args: CommonArgs) -> Result<()> {
    run(|config| params.into_task_config(config), args)
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

pub fn core_version() -> Result<String> {
    load_core()?;

    let v_str = Assistant::get_version().context("Failed to get MaaCore version!")?;

    Assistant::unload()?;

    Ok(v_str)
}

fn load_core() -> Result<()> {
    if Assistant::loaded() {
        debug!("MaaCore already loaded");
        return Ok(());
    }

    if let Some(lib_dir) = dirs::find_library() {
        debug!("Loading MaaCore from: {}", lib_dir.display());
        Assistant::load(lib_dir.join(MAA_CORE_LIB))
    } else {
        debug!("MaaCore not found, trying to load from system library path");
        Assistant::load(MAA_CORE_LIB)
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
    use std::env::{self, temp_dir};

    use super::*;

    #[test]
    #[ignore = "need installed MaaCore"]
    fn basic_ffi() {
        if env::var_os("SKIP_CORE_TEST").is_some() {
            return;
        }
        let version = env::var_os("MAA_CORE_VERSION").unwrap();
        assert_eq!(core_version().unwrap().as_str(), version);

        assert!(!Assistant::loaded());
        load_core().unwrap();
        assert!(Assistant::loaded());
        load_core().unwrap();
        assert!(Assistant::loaded());
        Assistant::unload().unwrap();
        assert!(!Assistant::loaded());
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
