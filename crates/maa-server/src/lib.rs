pub mod task {
    tonic::include_proto!("task");

    pub use maa_types::TaskType;

    mod convert {
        use maa_types::primitive::AsstTaskId;

        use super::*;

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
        use maa_types::TouchMode;

        use super::*;

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
            asst.async_connect(adb_path.as_str(), address.as_str(), config.as_str(), false)
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

pub mod prelude {
    pub use tonic;

    pub use crate::{
        core,
        server_impl::{core::gen_service as core_service, task::gen_service as task_service},
        task,
        types::HEADER_SESSION_ID,
    };
}

mod utils {
    pub fn load_core(path_to_core: std::path::PathBuf) -> Result<(), String> {
        tracing::debug!("Loading MaaCore from: {}", path_to_core.display());
        maa_sys::Assistant::load(path_to_core).map_err(|e| e.to_string())
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

mod callback;

mod types {
    pub use maa_types::{primitive::AsstTaskId as TaskId, TaskStateType};
    use uuid::Uuid;

    pub const HEADER_SESSION_ID: &str = "x-session-id";

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct SessionID(Uuid);

    impl SessionID {
        pub fn new() -> Self {
            Self(Uuid::now_v7())
        }

        /// Convert s SessionID to a raw pointer
        ///
        /// # Safety
        ///
        /// Remember to call [`SessionID::drop_ptr`] to free the memory
        pub fn to_ptr(self) -> *const u8 {
            let vec = self.0.into_bytes().to_vec();
            assert_eq!(vec.capacity(), 16);
            assert_eq!(vec.len(), 16);
            let ptr = vec.as_ptr();
            std::mem::forget(vec);
            ptr
        }

        /// Create a SessionID from a raw pointer via a byte array
        ///
        /// # Safety
        ///
        /// The pointer must be valid and point to a byte array of length 16.
        pub fn from_ptr(ptr: *const u8) -> Self {
            let mut bytes = const { [0; 16] };
            let slice = unsafe { std::slice::from_raw_parts(ptr, 16) };
            bytes.copy_from_slice(slice);
            Self(Uuid::from_bytes(bytes))
        }

        /// Free the pointer's memory
        ///
        /// # Safety
        ///
        /// The pointer must be created by [`SessionID::to_ptr`]
        pub fn drop_ptr(ptr: *const u8) {
            let ptr = ptr as *mut u8;
            let len = 16;
            let cap = 16;
            let _ = unsafe { Vec::from_raw_parts(ptr, len, cap) };
        }
    }

    impl std::fmt::Display for SessionID {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::str::FromStr for SessionID {
        type Err = uuid::Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Uuid::from_str(s).map(Self)
        }
    }

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
            let vec = unsafe {
                let len = 16;
                let cap = 16;
                Vec::from_raw_parts(ptr, len, cap)
            };
            drop(vec);
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

mod session;

mod server_impl;
