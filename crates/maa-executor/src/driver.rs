use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use maa_core::Assistant;
use maa_types::StaticOptionKey;

use crate::{
    Error, ErrorKind, Result,
    config::ExecutorConfig,
    session::{MaaSessionRunner, RunnerConfig, SessionRunner},
};

pub(crate) trait CoreDriver: Send + Sync {
    fn connect(&self, config: RunnerConfig) -> Result<Box<dyn SessionRunner>>;
}

static RUNTIME_OWNED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct RuntimeLease {
    loaded: bool,
}

impl RuntimeLease {
    fn acquire() -> Result<Self> {
        RUNTIME_OWNED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                Error::new(ErrorKind::Bootstrap)
                    .with_desc("another Executor already owns the MaaCore runtime")
            })?;

        let lease = Self { loaded: false };
        if Assistant::loaded() {
            return Err(Error::new(ErrorKind::Bootstrap)
                .with_desc("MaaCore was loaded outside this Executor"));
        }
        Ok(lease)
    }
}

impl Drop for RuntimeLease {
    fn drop(&mut self) {
        if self.loaded
            && let Err(error) = Assistant::unload()
        {
            log::error!("Failed to unload MaaCore runtime: {error}");
        }
        RUNTIME_OWNED.store(false, Ordering::Release);
    }
}

pub(crate) struct MaaCoreDriver {
    _runtime: RuntimeLease,
}

impl MaaCoreDriver {
    pub(crate) fn build(config: &ExecutorConfig) -> Result<Self> {
        validate_config(config)?;
        let mut runtime = RuntimeLease::acquire()?;

        Assistant::load(&config.lib_path).map_err(|source| {
            let path = config.lib_path.display();
            Error::new(ErrorKind::Bootstrap)
                .with_source(source)
                .with_desc(format!("failed to load MaaCore shared library from {path}"))
        })?;
        runtime.loaded = true;

        Assistant::set_user_dir(config.state_dir.as_path()).map_err(|source| {
            let path = config.state_dir.display();
            Error::new(ErrorKind::Bootstrap)
                .with_source(source)
                .with_desc(format!("failed to set user directory to {path}"))
        })?;
        apply_static_options(config.static_options)?;
        load_resources(&config.resources)?;

        Ok(Self { _runtime: runtime })
    }
}

impl CoreDriver for MaaCoreDriver {
    fn connect(&self, config: RunnerConfig) -> Result<Box<dyn SessionRunner>> {
        Ok(Box::new(MaaSessionRunner::connect(config)?))
    }
}

fn validate_config(config: &ExecutorConfig) -> Result<()> {
    if config.lib_path.as_os_str().is_empty() {
        return Err(Error::new(ErrorKind::InvalidConfig)
            .with_desc("executor library path must not be empty"));
    }
    if config.state_dir.as_os_str().is_empty() {
        return Err(
            Error::new(ErrorKind::InvalidConfig).with_desc("executor user_dir must not be empty")
        );
    }
    if config.resources.is_empty() {
        return Err(
            Error::new(ErrorKind::InvalidConfig).with_desc("executor resources must not be empty")
        );
    }
    Ok(())
}

fn apply_static_options(options: crate::config::StaticOptions) -> Result<()> {
    match (options.cpu_ocr, options.gpu_ocr) {
        (cpu_ocr, Some(gpu_id)) => {
            if cpu_ocr.is_some_and(|cpu_ocr| cpu_ocr) {
                log::warn!("Both CPU OCR and GPU OCR are enabled, CPU OCR will be ignored");
            }
            Assistant::set_static_option(StaticOptionKey::GpuOCR, gpu_id).map_err(|source| {
                Error::new(ErrorKind::Bootstrap)
                    .with_source(source)
                    .with_desc(format!("failed to enable GPU OCR with GPU ID {gpu_id}"))
            })?;
        }
        (Some(cpu_ocr), None) => {
            Assistant::set_static_option(StaticOptionKey::CpuOCR, cpu_ocr).map_err(|source| {
                Error::new(ErrorKind::Bootstrap)
                    .with_source(source)
                    .with_desc("failed to configure CPU OCR")
            })?;
        }
        _ => {}
    }

    Ok(())
}

fn load_resources(resources: &[impl AsRef<Path>]) -> Result<()> {
    for resource in resources {
        let path = resource.as_ref();
        Assistant::load_resource(path).map_err(|source| {
            let path = path.display();
            Error::new(ErrorKind::Bootstrap)
                .with_source(source)
                .with_desc(format!("failed to load resource from {path}"))
        })?;
    }
    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn config_rejects_empty_lib_path() {
        let config = ExecutorConfig {
            lib_path: PathBuf::new(),
            state_dir: "/tmp/user".into(),
            resources: vec!["/tmp/resource".into()],
            static_options: crate::StaticOptions::default(),
        };
        let err = validate_config(&config).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidConfig);
    }

    #[test]
    fn runtime_lease_is_exclusive_and_released_on_drop() {
        let lease = RuntimeLease::acquire().unwrap();
        assert_eq!(
            RuntimeLease::acquire().unwrap_err().kind(),
            ErrorKind::Bootstrap
        );

        drop(lease);
        assert!(RuntimeLease::acquire().is_ok());
    }
}
