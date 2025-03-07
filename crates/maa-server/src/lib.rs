pub use tonic;

pub mod task {
    tonic::include_proto!("task");

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
        // use new_connection_request::instance_options::TouchMode;

        use super::*;

        impl new_connection_request::InstanceOptions {
            pub fn apply_to(self, asst: &maa_sys::Assistant) -> Result<(), String> {
                use maa_sys::InstanceOptionKey;
                let touch_mode = self.touch_mode;
                tracing::debug!("Setting touch mode to {}", touch_mode);
                asst.set_instance_option(InstanceOptionKey::TouchMode, touch_mode)
                    .map_err(|_| format!("Failed to set touch mode to {}", touch_mode))?;
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
            } = self;

            if let Some(ops) = log_ops {
                ops.apply()?;
            }
            if let Some(ops) = static_ops {
                ops.apply()?;
            }
            Ok(())
        }
    }
}

pub mod utils {
    pub fn load_core() -> Result<(), String> {
        use maa_dirs::MAA_CORE_LIB;
        if let Some(lib_dir) = maa_dirs::find_library() {
            tracing::debug!("Loading MaaCore from: {}", lib_dir.display());
            // Set DLL directory on Windows
            #[cfg(target_os = "windows")]
            {
                use windows_strings::HSTRING;
                use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

                let code = unsafe { SetDllDirectoryW(HSTRING::from(lib_dir.as_ref()).as_ptr()) };
                if code == 0 {
                    return Err(anyhow::Error::new(windows_result::Error::from_win32())
                        .context("Failed to set DLL directory!"));
                }
            }
            maa_sys::binding::load(lib_dir.join(MAA_CORE_LIB))
        } else {
            tracing::debug!("MaaCore not found, trying to load from system library path");
            maa_sys::binding::load(MAA_CORE_LIB)
        }
        .map_err(|e| e.to_string())
    }

    use std::path::PathBuf;

    use maa_dirs::{self as dirs, join};

    #[cfg_attr(test, derive(Debug, PartialEq))]
    #[derive(Clone)]
    pub struct ResourceConfig {
        /// Resources used by global arknights client, e.g. `YostarEN`
        global_resource: Option<PathBuf>,
        /// Resources used by platform diff, subdirectories of `resource_base_dirs`, e.g.
        /// `platform_diff/iOS`
        platform_diff_resource: Option<PathBuf>,
        /// Whether to load resources from user config directory, when enabled, the
        /// `MAA_CONFIG_DIR/resource` will be appended to `resource_base_dirs` as the last element
        user_resource: bool,
        /// Resource base directories, a list of directories containing resource directories
        /// Not deserialized from config file
        pub(crate) resource_base_dirs: Vec<PathBuf>,
    }

    impl Default for ResourceConfig {
        fn default() -> Self {
            Self {
                resource_base_dirs: default_resource_base_dirs(),
                global_resource: None,
                platform_diff_resource: None,
                user_resource: false,
            }
        }
    }

    fn default_resource_base_dirs() -> Vec<PathBuf> {
        let mut resource_dirs = Vec::new();

        if let Some(resource_dir) = dirs::find_resource() {
            tracing::debug!("Found resource directory: {}", resource_dir.display());
            resource_dirs.push(resource_dir.into_owned());
        } else {
            tracing::warn!("Resource directory not found!")
        }

        let hot_update_dir = dirs::hot_update();
        if hot_update_dir.exists() {
            tracing::debug!(
                "Found hot update resource directory: {}",
                hot_update_dir.display()
            );
            resource_dirs.push(join!(hot_update_dir, "resource"));
            resource_dirs.push(join!(hot_update_dir, "cache", "resource"));
        } else {
            tracing::warn!("Hot update resource directory not found!");
        }

        resource_dirs
    }

    impl ResourceConfig {
        pub fn use_user_resource(&mut self) -> &mut Self {
            if !self.user_resource {
                self.user_resource = true;
                push_user_resource(&mut self.resource_base_dirs);
            }
            self
        }

        pub fn use_global_resource(&mut self, resource: impl Into<PathBuf>) -> &mut Self {
            match self.global_resource.as_ref() {
                Some(global_resource) => {
                    tracing::warn!(
                        "Global resource {} already set, ignoring {}",
                        global_resource.display(),
                        resource.into().display(),
                    );
                }
                None => {
                    let resource = resource.into();
                    tracing::info!("Using global resource: {}", resource.display());
                    self.global_resource = Some(resource);
                }
            }
            self
        }

        pub fn use_platform_diff_resource(&mut self, resource: impl Into<PathBuf>) -> &mut Self {
            match self.platform_diff_resource.as_ref() {
                Some(platform_diff_resource) => {
                    tracing::warn!(
                        "Platform diff resource {} already set, ignoring {}",
                        platform_diff_resource.display(),
                        resource.into().display(),
                    );
                }
                None => {
                    // should not push to resource_base_dirs as this is not a base resource
                    // directory
                    let resource = resource.into();
                    tracing::info!("Using platform diff resource: {}", resource.display());
                    self.platform_diff_resource = Some(resource);
                }
            }
            self
        }

        /// Get base resource directories
        pub fn base_dirs(&self) -> &Vec<PathBuf> {
            &self.resource_base_dirs
        }

        /// Get all resource directories, including global and platform diff resources
        pub fn resource_dirs(&self) -> Vec<PathBuf> {
            let base_dirs = self.base_dirs();
            let mut resource_dirs = base_dirs.clone();
            if let Some(global_resource) = self.global_resource.as_ref() {
                let global_resource_dir = join!("global", global_resource, "resource");
                let full_paths = dirs::global_path(base_dirs, global_resource_dir);
                if full_paths.is_empty() {
                    tracing::warn!("Global resource {} not found", global_resource.display(),);
                } else {
                    resource_dirs.extend(full_paths);
                }
            }
            if let Some(platform_diff_resource) = self.platform_diff_resource.as_ref() {
                let platform_diff_resource_dir =
                    join!("platform_diff", platform_diff_resource, "resource");
                let full_paths = dirs::global_path(base_dirs, platform_diff_resource_dir);
                if full_paths.is_empty() {
                    tracing::warn!(
                        "Platform diff resource {} not found",
                        platform_diff_resource.display(),
                    );
                } else {
                    resource_dirs.extend(full_paths);
                }
            }

            resource_dirs
        }

        pub fn load(&self) -> Result<(), String> {
            let resource_dirs = self.resource_dirs();
            for resource_dir in resource_dirs {
                tracing::debug!("Loading resource from {}", resource_dir.display());
                maa_sys::Assistant::load_resource(resource_dir.parent().unwrap())
                    .map_err(|e| e.to_string())?;
            }

            Ok(())
        }
    }

    fn push_user_resource(resource_dirs: &mut Vec<PathBuf>) -> &mut Vec<PathBuf> {
        push_resource(resource_dirs, dirs::config().join("resource"))
    }

    fn push_resource(
        resource_dirs: &mut Vec<PathBuf>,
        dir: impl Into<PathBuf>,
    ) -> &mut Vec<PathBuf> {
        let dir = dir.into();
        if dir.exists() {
            resource_dirs.push(dir);
        } else {
            tracing::warn!("Resource directory {} not found, ignoring", dir.display(),);
        }

        resource_dirs
    }
}

pub mod callback;

pub mod types {
    pub type SessionID = [u8; 16];
    pub use maa_types::primitive::AsstTaskId as TaskId;

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
