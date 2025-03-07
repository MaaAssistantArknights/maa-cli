pub use tonic;

pub mod task {
    tonic::include_proto!("task");

    pub use maa_types::TaskType;

    mod convert {
        use super::*;
        use maa_types::primitive::AsstTaskId;

        impl From<TaskId> for AsstTaskId {
            fn from(value: TaskId) -> Self {
                let TaskId { id } = value;
                id
            }
        }

        impl From<AsstTaskId> for TaskId {
            fn from(id: AsstTaskId) -> Self {
                Self { id }
            }
        }
    }

    mod utils {
        use super::*;
        use maa_types::TouchMode;

        impl new_connection_request::InstanceOptions {
            pub fn apply_to(self, asst: &maa_sys::Assistant) -> Result<(), String> {
                use maa_sys::InstanceOptionKey;
                if let Ok(touch_mode) = TryInto::<TouchMode>::try_into(self.touch_mode) {
                    tracing::debug!("Setting touch mode to {}", touch_mode.to_str());
                    asst.set_instance_option(InstanceOptionKey::TouchMode, touch_mode.to_str())
                        .map_err(|_| {
                            format!("Failed to set touch mode to {}", touch_mode.to_str())
                        })?;
                }
                if self.deployment_with_pause {
                    tracing::debug!(
                        "Setting deployment with pause to {}",
                        self.deployment_with_pause
                    );
                    asst.set_instance_option(
                        InstanceOptionKey::DeploymentWithPause,
                        self.deployment_with_pause,
                    )
                    .map_err(|_| "Failed to set deployment with pause")?;
                }
                if self.adb_lite_enabled {
                    tracing::debug!("Setting adb lite enabled to {}", self.adb_lite_enabled);
                    asst.set_instance_option(
                        InstanceOptionKey::AdbLiteEnabled,
                        self.adb_lite_enabled,
                    )
                    .map_err(|_| "Failed to set adb lite enabled")?;
                }
                if self.kill_adb_on_exit {
                    tracing::debug!("Setting kill adb on exit to {}", self.kill_adb_on_exit);
                    asst.set_instance_option(
                        InstanceOptionKey::KillAdbOnExit,
                        self.kill_adb_on_exit,
                    )
                    .map_err(|_| "Failed to set kill adb on exit")?;
                }
                Ok(())
            }
        }

        impl new_connection_request::ConnectionConfig {
            pub fn connect_args(self) -> (String, String, String) {
                let adb_path = self.adb_path;
                let address = self.address;
                let config = self.config;
                tracing::debug!(
                    "Connecting to {address} with config {config} via {}",
                    &adb_path
                );

                (adb_path, address, config)
            }
        }
    }

    impl NewConnectionRequest {
        #[tracing::instrument("Apply Instance Config", skip_all)]
        pub fn apply_to(self, asst: &maa_sys::Assistant) -> tonic::Result<()> {
            let Self { conncfg, instcfg } = self;

            if let Some(message) = instcfg.and_then(|cfg| cfg.apply_to(asst).err()) {
                return Err(tonic::Status::internal(message));
            }

            let (adb_path, address, config) = conncfg.unwrap().connect_args();
            asst.async_connect(adb_path.as_str(), address.as_str(), config.as_str(), true)
                .unwrap();

            Ok(())
        }
    }
}

pub mod core {
    tonic::include_proto!("core");

    impl core_config::StaticOptions {
        pub fn apply(self) -> tonic::Result<()> {
            use maa_sys::{Assistant, StaticOptionKey};

            match (self.cpu_ocr, self.gpu_ocr) {
                (cpu_ocr, Some(gpu_id)) => {
                    if cpu_ocr {
                        tracing::warn!(
                            "Both CPU OCR and GPU OCR are enabled, CPU OCR will be ignored"
                        );
                    }
                    tracing::debug!("Using GPU OCR with GPU ID {}", gpu_id);
                    if Assistant::set_static_option(StaticOptionKey::GpuOCR, gpu_id).is_err() {
                        return Err(tonic::Status::internal(format!(
                            "Failed to enable GPU OCR with GPU ID {}",
                            gpu_id
                        )));
                    }
                }
                (true, None) => {
                    tracing::debug!("Using CPU OCR");
                    if Assistant::set_static_option(StaticOptionKey::CpuOCR, true).is_err() {
                        return Err(tonic::Status::internal("Failed to enable CPU OCR"));
                    }
                }
                (false, None) => {}
            };
            Ok(())
        }
    }

    impl core_config::LogOptions {
        pub fn apply(self) -> tonic::Result<()> {
            let Self { level, path } = self;
            // Todo: set log level for tracing
            let _ = level;
            let path = std::path::PathBuf::from(path);
            if !path.exists() {
                std::fs::create_dir_all(&path).map_err(|e| {
                    tonic::Status::internal(format!("Unable to create dir due to {}", e))
                })?
            }
            if !path.is_dir() {
                Err(tonic::Status::invalid_argument("Not a valid dir"))?
            }
            maa_sys::Assistant::set_user_dir(path.as_path())
                .map_err(|e| tonic::Status::from_error(Box::new(e)))
        }
    }

    impl CoreConfig {
        pub fn apply(self) -> tonic::Result<()> {
            let Self {
                static_ops,
                log_ops,
                lib_path,
                resource_dirs,
            } = self;

            crate::utils::load_core(lib_path.into()).map_err(|e| {
                tonic::Status::invalid_argument(format!("Failed to load MaaCore due to {e}"))
            })?;
            if let Some(ops) = log_ops {
                ops.apply()?;
            }
            if let Some(ops) = static_ops {
                ops.apply()?;
            }
            crate::utils::load_resource(resource_dirs).map_err(|e| {
                tonic::Status::invalid_argument(format!("Failed to load Maa Resource due to {e}"))
            })?;
            Ok(())
        }
    }
}

mod utils {
    pub fn load_core(path_to_core: std::path::PathBuf) -> Result<(), String> {
        tracing::debug!("Loading MaaCore from: {}", path_to_core.display());
        // Set DLL directory on Windows
        #[cfg(target_os = "windows")]
        {
            use windows_strings::HSTRING;
            use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

            let code =
                unsafe { SetDllDirectoryW(HSTRING::from(path_to_core.parent().unwrap()).as_ptr()) };
            if code == 0 {
                return Err(anyhow::Error::new(windows_result::Error::from_win32())
                    .context("Failed to set DLL directory!"));
            }
        }
        maa_sys::binding::load(path_to_core).map_err(|e| e.to_string())
    }

    pub fn load_resource(resource_dirs: Vec<String>) -> maa_sys::Result<()> {
        for resource_dir in resource_dirs {
            let resource_dir = std::path::PathBuf::from(resource_dir);
            tracing::debug!("Loading resource from {}", resource_dir.display());
            maa_sys::Assistant::load_resource(resource_dir.parent().unwrap())?;
        }
        Ok(())
    }
}

pub mod callback;

pub mod types {
    pub type SessionID = [u8; 16];
    pub use maa_types::primitive::AsstTaskId as TaskId;
    pub use maa_types::TaskStateType;

    #[cfg(test)]
    mod tests {
        use std::str::FromStr;

        #[test]
        fn uuid_ffi() {
            let rust: [u8; 8] = [1, 7, 45, 31, 5, 21, 46, 1];

            let ptr = {
                let mut rust_copy = rust.to_vec();
                let ptr = rust_copy.as_mut_ptr();
                std::mem::forget(rust_copy);
                ptr as *mut std::ffi::c_void
            };
            let len = 8;

            let mut cffi = [0u8; 8];

            assert_ne!(rust, cffi);

            let ptr = ptr as *mut u8;
            cffi.copy_from_slice(unsafe { std::slice::from_raw_parts(ptr, len) });

            assert_eq!(rust, cffi);
        }

        #[test]
        fn uuid_string() {
            let uuid = uuid::Uuid::now_v7();
            let str = uuid.to_string();
            let bytes = uuid.to_bytes_le();

            let bytes_from_str = uuid::Uuid::from_str(&str).unwrap();

            assert_eq!(uuid, bytes_from_str);
            assert_eq!(bytes, bytes_from_str.to_bytes_le());
        }
    }
}

pub mod session;

pub mod server_impl;
