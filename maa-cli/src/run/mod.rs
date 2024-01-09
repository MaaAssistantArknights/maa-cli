// mod message;
// use message::callback;
//
mod callback;
use callback::summary;

#[cfg(target_os = "macos")]
mod playcover;

pub mod preset;

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

use anyhow::{Context, Result};
use clap::Args;
use maa_sys::Assistant;
use signal_hook::consts::TERM_SIGNALS;

#[cfg(target_os = "macos")]
use tokio::runtime::Runtime;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Args, Default)]
pub struct CommonArgs {
    #[arg(short, long, help = fl!("run-addr-help"), long_help = fl!("run-addr-long-help"))]
    pub addr: Option<String>,
    #[arg(long, help = fl!("run-user-resource-help"), long_help = fl!("run-user-resource-long-help"))]
    pub user_resource: bool,
    #[arg(long, help = fl!("run-dry-run-help"), long_help = fl!("run-dry-run-long-help"))]
    pub dry_run: bool,
    #[arg(long, help = fl!("run-no-summary-help"), long_help = fl!("run-no-summary-long-help"))]
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
        debug!("detected-client-type", client = client_type);
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
            .with_context(lfl!("failed-register-signal-handler"))?;
        signal_hook::flag::register(*sig, Arc::clone(&stop_bool))
            .with_context(lfl!("failed-register-signal-handler"))?;
    }

    let asst = Assistant::new(Some(callback::default_callback), None);

    with_asst_config(|config| config.instance_options.apply_to(&asst))?;

    let mut summarys = (!args.no_summary).then(summary::Summary::new);
    for task in task_config.tasks.iter() {
        let name = task.name();
        let task_type = task.task_type();
        let params = task.params();

        if params.is_empty() {
            debug!(
                "append-task-no-param",
                task = name
                    .map(|s| s.to_owned())
                    .unwrap_or(task_type.to_fl_string()),
            );
        } else {
            debug!(
                "append-task-with-param",
                task = name
                    .map(|s| s.to_owned())
                    .unwrap_or(task_type.to_fl_string()),
                params = serde_json::to_string_pretty(params)?,
            );
        }
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
        let rt = Runtime::new().with_context(lfl!("failed-create-tokio-runtime"))?;

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
                bailfl!("interrupted");
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
            .with_context(lfl!("failed-find-task-file"))
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
        debug!("maa-core-already-loaded");
        return;
    }

    if let Some(lib_dir) = dirs::find_library() {
        debug!("load-maa-core", path = lib_dir.display().to_string());
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
        debug!("use-system-maa-core");
        maa_sys::binding::load(MAA_CORE_LIB);
    }
}

fn setup_core(config: &AsstConfig) -> Result<()> {
    debug!("set-user-directory", path = dirs::state().to_string_lossy());
    Assistant::set_user_dir(dirs::state().ensure()?).with_context(lfl!(
        "failed-set-user-directory",
        path = dirs::state().to_string_lossy()
    ))?;

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
