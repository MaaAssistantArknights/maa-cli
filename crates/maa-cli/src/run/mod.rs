mod callback;
use callback::summary;

mod external;

mod window;

#[cfg(test)]
mod window_tests;

pub mod preset;

use std::{
    path::Path,
    sync::{Arc, atomic},
};

use anyhow::{Context, Result, bail};
use clap::Args;
use log::{debug, warn};
use maa_core::Assistant;
use maa_dirs::{self as dirs, Ensure, MAA_CORE_LIB};
use maa_types::InstanceOptionKey;
use signal_hook::consts::TERM_SIGNALS;

use crate::{
    config::{
        FindFile,
        asst::AsstConfig,
        task::{TaskConfig, TaskConfigTemplate},
    },
    installer,
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
    /// Do not reconnect when game loses connection to server
    ///
    /// By default, maa will automatically reconnect when the game client
    /// loses connection to the game server. Use this option to
    /// disable this behavior for this run.
    #[arg(long, verbatim_doc_comment)]
    pub no_auto_reconnect: bool,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Args, Default)]
pub struct ConnectionTestArgs {
    /// Profile (asst config file) name
    #[arg(short, long)]
    pub profile: Option<String>,
    /// Take one fresh screenshot after connecting
    #[arg(long)]
    pub screencap: bool,
    /// Print a machine-readable JSON result
    #[arg(long)]
    pub json: bool,
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
        warn!(
            "The config file `asst.toml` is deprecated, please use `profiles/default.toml` instead!"
        );
        Ok(config)
    } else {
        Ok(AsstConfig::default())
    }
}

fn ensure_connection_supported(_connection: &crate::config::asst::ConnectionConfig) -> Result<()> {
    #[cfg(not(windows))]
    if matches!(_connection.preset(), crate::config::asst::Preset::Win32) {
        bail!("Win32 connection is only supported on Windows");
    }
    Ok(())
}

fn connect_assistant(
    asst: &Assistant,
    connection: &crate::config::asst::ConnectionConfig,
    address_override: Option<&str>,
) -> Result<()> {
    match connection.connection_args()? {
        crate::config::asst::ConnectionArgs::Win32(args) => {
            #[cfg(windows)]
            {
                let library = dirs::find_library();
                window::validate_win32_control_unit_at(library.as_deref())?;
                let hwnd = window::resolve_window(&args.selector)?;
                asst.async_attach_window(
                    hwnd,
                    args.screencap_method,
                    args.mouse_method,
                    args.keyboard_method,
                    true,
                )?;
                Ok(())
            }
            #[cfg(not(windows))]
            {
                window::resolve_window(&args.selector)?;
                Ok(())
            }
        }
        crate::config::asst::ConnectionArgs::Adb {
            adb_path,
            address,
            config,
        } => {
            let address = address_override.unwrap_or(address.as_ref());
            asst.async_connect(adb_path.as_ref(), address, config, true)?;
            Ok(())
        }
    }
}

fn require_screenshot_bytes(image: Option<Vec<u8>>) -> Result<usize> {
    let image = image.context("Connection succeeded but MaaCore returned no screenshot")?;
    if image.is_empty() {
        bail!("Connection succeeded but MaaCore returned an empty screenshot");
    }
    Ok(image.len())
}

fn connection_label(preset: crate::config::asst::Preset) -> &'static str {
    use crate::config::asst::Preset;

    match preset {
        Preset::Adb => "ADB",
        Preset::MuMuPro => "MuMuPro",
        Preset::PlayCover => "PlayCover",
        Preset::Waydroid => "Waydroid",
        Preset::Androws => "Androws",
        Preset::Win32 => "Win32",
    }
}

pub fn test_connection(args: ConnectionTestArgs) -> Result<()> {
    let asst_config = find_profile(dirs::config(), args.profile.as_deref())?;
    ensure_connection_supported(&asst_config.connection)?;
    load_core().context("Failed to load MaaCore!")?;
    setup_core(&asst_config)?;

    let asst = Assistant::new().context("Failed to create Assistant")?;
    asst_config.instance_options.apply_to(&asst)?;
    connect_assistant(&asst, &asst_config.connection, None)?;
    let screenshot_bytes = if args.screencap {
        require_screenshot_bytes(asst.get_fresh_image()?)?
    } else {
        0
    };
    let connection = connection_label(asst_config.connection.preset());
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "connection": connection,
                "screenshot_bytes": screenshot_bytes,
            })
        );
    } else {
        println!("Connection test succeeded ({connection}, screenshot bytes: {screenshot_bytes})");
    }
    Ok(())
}

fn run_core<F>(f: F, args: CommonArgs) -> Result<()>
where
    F: FnOnce(&AsstConfig) -> Result<TaskConfig>,
{
    // Auto update hot update resource
    installer::hot_update::update()?;
    installer::resource::update(true)?;

    // Load asst config
    let mut asst_config = find_profile(dirs::config(), args.profile.as_deref())?;

    args.apply_to(&mut asst_config);
    ensure_connection_supported(&asst_config.connection)?;

    let mut task_config = f(&asst_config)?;
    if matches!(
        asst_config.connection.preset(),
        crate::config::asst::Preset::Win32
    ) {
        task_config.prepare_for_win32();
    }
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
    let auto_reconnect = asst_config.behavior.auto_reconnect && !args.no_auto_reconnect;
    let (maa_callback, offline_stop) = callback::MaaCallback::new(auto_reconnect);
    let asst = Assistant::new_with_callback(maa_callback)
        .context("Failed to create Assistant: resources may not be loaded")?;
    asst_config.instance_options.apply_to(&asst)?;
    debug!("Setting client type to {}", task_config.client_type);
    asst.set_instance_option(
        InstanceOptionKey::ClientType,
        task_config.client_type.to_str(),
    )
    .context("Failed to set client type")?;

    // Register tasks to Assistant and prepare summary
    let mut task_summary = (!args.no_summary).then(summary::Summary::new);
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

        if let Some(s) = task_summary.as_mut() {
            s.insert(id, task.name, task_type);
        }
    }
    if let Some(s) = task_summary {
        summary::init(s);
    }

    if !args.dry_run {
        #[cfg(target_os = "macos")]
        let playcover_address = matches!(
            asst_config.connection.preset(),
            crate::config::asst::Preset::PlayCover
        )
        .then(|| asst_config.connection.connect_args().1.into_owned());

        // Launch external apps
        let app: Option<Box<dyn external::ExternalApp>> = match asst_config.connection.preset() {
            #[cfg(target_os = "macos")]
            crate::config::asst::Preset::PlayCover => Some(Box::new(external::PlayCoverApp::new(
                task_config.client_type,
                playcover_address
                    .as_deref()
                    .context("PlayCover address is unavailable")?,
            ))),
            #[cfg(target_os = "linux")]
            crate::config::asst::Preset::Waydroid => Some(Box::new(external::WaydroidApp::new())),
            _ => None,
        };

        // Startup external app or query its runtime address if available
        let runtime_address = app
            .as_deref()
            .map(|app| app.open(task_config.start_app))
            .transpose()?
            .flatten();

        // Connect to game or emulator
        connect_assistant(&asst, &asst_config.connection, runtime_address.as_deref())?;

        debug!("Starting MAA...");
        asst.start()?;

        while asst.running() {
            if stop_bool.load(atomic::Ordering::Relaxed) {
                bail!("Interrupted by user!");
            }
            if offline_stop.load(atomic::Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        debug!("Stopping MAA...");
        asst.stop()?;

        // Close external app
        if let (Some(app), true) = (app.as_deref(), task_config.close_app) {
            debug!("Closing external app...");
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
    let ret = run_core(f, args);

    summary::display();

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
            let config = if let Some(abs_path) = dirs::abs_config(path, Some("tasks")) {
                TaskConfigTemplate::find_file(abs_path)
            } else {
                TaskConfigTemplate::find_file(path)
            }
            .context("Failed to find task file!")?;

            config.init().context("Failed to initialize task config!")
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::env::{self, temp_dir};

    use super::*;

    #[test]
    fn screenshot_probe_requires_a_non_empty_image() {
        assert!(require_screenshot_bytes(None).is_err());
        assert!(require_screenshot_bytes(Some(Vec::new())).is_err());
        assert_eq!(require_screenshot_bytes(Some(vec![1, 2, 3])).unwrap(), 3);
    }

    #[test]
    fn connection_probe_reports_the_configured_preset() {
        use crate::config::asst::Preset;

        assert_eq!(connection_label(Preset::Adb), "ADB");
        assert_eq!(connection_label(Preset::MuMuPro), "MuMuPro");
        assert_eq!(connection_label(Preset::PlayCover), "PlayCover");
        assert_eq!(connection_label(Preset::Waydroid), "Waydroid");
        assert_eq!(connection_label(Preset::Androws), "Androws");
        assert_eq!(connection_label(Preset::Win32), "Win32");
    }

    #[cfg(not(windows))]
    #[test]
    fn win32_is_rejected_before_loading_core_on_non_windows() {
        let config: crate::config::asst::ConnectionConfig = toml::from_str(
            r#"
                preset = "Win32"
                window_title = "Arknights"
            "#,
        )
        .unwrap();

        assert_eq!(
            ensure_connection_supported(&config)
                .unwrap_err()
                .to_string(),
            "Win32 connection is only supported on Windows"
        );
    }

    #[test]
    #[ignore = "need installed MaaCore"]
    fn basic_ffi() {
        if env::var_os("SKIP_CORE_TEST").is_some() {
            return;
        }
        core_version().unwrap();

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
