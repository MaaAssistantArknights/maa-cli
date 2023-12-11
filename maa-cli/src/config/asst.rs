use super::FindFileOrDefault;

use crate::{
    dirs::{self, global_path},
    {debug, info, warning},
};

use std::{path::PathBuf, sync::Mutex};

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use maa_sys::{Assistant, InstanceOptionKey, StaticOptionKey, TouchMode};
use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default, Clone)]
pub struct AsstConfig {
    pub connection: ConnectionConfig,
    pub resource: ResourceConfig,
    pub static_options: StaticOptions,
    pub instance_options: InstanceOptions,
}

impl<'de> Deserialize<'de> for AsstConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AsstConfigHelper {
            #[serde(default)]
            connection: ConnectionConfig,
            #[serde(default)]
            resource: ResourceConfig,
            #[serde(default)]
            static_options: StaticOptions,
            #[serde(default)]
            instance_options: InstanceOptions,
        }

        let mut config = AsstConfigHelper::deserialize(deserializer)?;

        if matches!(config.connection, ConnectionConfig::PlayTools { .. }) {
            info!("Detected connection with PlayTools");
            config.instance_options.force_playtools();
            config.resource.use_platform_diff_resource("iOS");
        }

        Ok(Self {
            connection: config.connection,
            resource: config.resource,
            static_options: config.static_options,
            instance_options: config.instance_options,
        })
    }
}

impl super::FromFile for AsstConfig {}

lazy_static! {
    static ref ASST_CONFIG: Mutex<AsstConfig> = Mutex::new(
        AsstConfig::find_file_or_default(&dirs::config().join("asst"))
            .expect("Failed to load asst config")
    );
}

pub fn with_asst_config<R>(f: impl FnOnce(&AsstConfig) -> R) -> R {
    let asst_config = ASST_CONFIG.lock().unwrap();
    f(&asst_config)
}

pub fn with_mut_asst_config<R>(f: impl FnOnce(&mut AsstConfig) -> R) -> R {
    let mut asst_config = ASST_CONFIG.lock().unwrap();
    f(&mut asst_config)
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub enum ConnectionConfig {
    ADB {
        #[serde(default = "default_adb_path")]
        adb_path: String,
        #[serde(default = "default_device")]
        device: String,
        #[serde(default = "default_config")]
        config: String,
    },
    #[serde(alias = "PlayCover")]
    PlayTools {
        #[serde(default = "default_playcover_address")]
        address: String,
        #[serde(default = "default_config")]
        config: String,
    },
}

impl ConnectionConfig {
    pub fn set_address(&mut self, addr: impl Into<String>) -> &Self {
        match self {
            ConnectionConfig::ADB { device, .. } => {
                *device = addr.into();
            }
            ConnectionConfig::PlayTools { address, .. } => {
                *address = addr.into();
            }
        }
        self
    }

    pub fn connect(&self, asst: &Assistant) -> maa_sys::Result<()> {
        match self {
            ConnectionConfig::ADB {
                adb_path,
                device,
                config,
            } => {
                debug!(format!(
                    "Connecting to {} with config {} via {}",
                    device, config, adb_path
                ));
                Assistant::async_connect(asst, adb_path, device, config, true)?;
            }
            ConnectionConfig::PlayTools { address, config } => {
                debug!(format!(
                    "Connecting to {} with config {} via PlayTools",
                    address, config
                ));
                Assistant::async_connect(asst, String::new(), address, config, true)?;
            }
        }
        Ok(())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone)]
pub struct ResourceConfig {
    /// Resources used by global arknights client, subdirectories of `resource_base_dirs`, e.g. `global/YostarEN`
    global_resource: Option<PathBuf>,
    /// Resources used by platform diff, subdirectories of `resource_base_dirs`, e.g. `platform_diff/iOS`
    platform_diff_resource: Option<PathBuf>,
    /// Whether to load resources from user config directory, when enabled, the `MAA_CONFIG_DIR/resource`
    /// will be appended to `resource_base_dirs` as the last element
    user_resource: bool,
    /// Resource base directories, a list of directories containing resource directories
    /// Not deserialized from config file
    resource_base_dirs: Vec<PathBuf>,
}

impl<'de> Deserialize<'de> for ResourceConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ResourceConfigHelper {
            #[serde(default)]
            global_resource: Option<PathBuf>,
            #[serde(default)]
            platform_diff_resource: Option<PathBuf>,
            #[serde(default)]
            user_resource: bool,
        }

        let helper = ResourceConfigHelper::deserialize(deserializer)?;

        let mut resource_base_dirs = default_resource_base_dirs();

        if helper.user_resource {
            push_user_resource(&mut resource_base_dirs);
        }

        Ok(Self {
            resource_base_dirs,
            global_resource: helper.global_resource,
            platform_diff_resource: helper.platform_diff_resource,
            user_resource: helper.user_resource,
        })
    }
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
        debug!("Found resource directory:", resource_dir.display());
        resource_dirs.push(resource_dir);
    } else {
        warning!("Resource directory not found!")
    }

    let hot_update_dir = dirs::hot_update();
    if hot_update_dir.exists() {
        debug!(
            "Found hot update resource directory:",
            hot_update_dir.display()
        );
        resource_dirs.push(hot_update_dir.join("resource"));
        resource_dirs.push(hot_update_dir.join("cache").join("resource"));
    } else {
        warning!("Hot update resource directory not found!");
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
                warning!(format!(
                    "Global resource {} already set, ignoring {}",
                    global_resource.display(),
                    resource.into().display(),
                ));
            }
            None => {
                let resource = resource.into();
                info!("Using global resource:", resource.display());
                self.global_resource = Some(resource);
            }
        }
        self
    }

    pub fn use_platform_diff_resource(&mut self, resource: impl Into<PathBuf>) -> &mut Self {
        match self.platform_diff_resource.as_ref() {
            Some(platform_diff_resource) => {
                warning!(format!(
                    "Platform diff resource {} already set, ignoring {}",
                    platform_diff_resource.display(),
                    resource.into().display(),
                ));
            }
            None => {
                let resource = resource.into();
                info!("Using platform diff resource:", resource.display());
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
    pub fn resource_dirs(&self) -> Result<Vec<PathBuf>> {
        let base_dirs = &self.resource_base_dirs;
        let mut resource_dirs = base_dirs.clone();
        if let Some(global_resource) = self.global_resource.as_ref() {
            let global_resource_dir = PathBuf::from("global")
                .join(global_resource)
                .join("resource");
            let full_paths = global_path(base_dirs, global_resource_dir);
            if full_paths.is_empty() {
                warning!(format!(
                    "Global resource {} not found",
                    global_resource.display(),
                ));
            } else {
                resource_dirs.extend(full_paths);
            }
        }
        if let Some(platform_diff_resource) = self.platform_diff_resource.as_ref() {
            let platform_diff_resource_dir = PathBuf::from("platform_diff")
                .join(platform_diff_resource)
                .join("resource");
            let full_paths = global_path(base_dirs, platform_diff_resource_dir);
            if full_paths.is_empty() {
                warning!(format!(
                    "Platform diff resource {} not found",
                    platform_diff_resource.display(),
                ));
            } else {
                resource_dirs.extend(full_paths);
            }
        }

        Ok(resource_dirs)
    }

    pub fn load(&self) -> Result<()> {
        let resource_dirs = self.resource_dirs()?;
        for resource_dir in resource_dirs {
            debug!("Loading resource from", resource_dir.display());
            Assistant::load_resource(resource_dir.parent().unwrap())?;
        }

        Ok(())
    }
}

fn push_user_resource(resource_dirs: &mut Vec<PathBuf>) -> &mut Vec<PathBuf> {
    let user_resource_dir = dirs::config().join("resource");
    if !user_resource_dir.exists() {
        warning!(format!(
            "User resource directory {} not found, ignoring",
            user_resource_dir.display(),
        ));
    } else {
        resource_dirs.push(user_resource_dir);
    }

    resource_dirs
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
pub struct StaticOptions {
    #[serde(default)]
    pub cpu_ocr: Option<bool>,
    #[serde(default)]
    pub gpu_ocr: Option<u32>,
}

impl StaticOptions {
    pub fn apply(&self) -> Result<()> {
        match (self.cpu_ocr, self.gpu_ocr) {
            (Some(cpu_ocr), Some(gpu_id)) => {
                if cpu_ocr {
                    warning!("Both CPU OCR and GPU OCR are enabled, CPU OCR will be ignored");
                }
                debug!(format!("Using GPU OCR with GPU ID {}", gpu_id));
                StaticOptionKey::GpuOCR
                    .apply(gpu_id)
                    .with_context(|| format!("Failed to enable GPU OCR with GPU ID {}", gpu_id))?;
            }
            (Some(cpu_core), None) if cpu_core => {
                debug!("Using CPU OCR");
                StaticOptionKey::CpuOCR
                    .apply(true)
                    .context("Failed to enable CPU OCR")?;
            }
            (_, _) => {}
        };

        Ok(())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
pub struct InstanceOptions {
    #[serde(default)]
    touch_mode: Option<TouchMode>,
    deployment_with_pause: Option<bool>,
    adb_lite_enabled: Option<bool>,
    kill_adb_on_exit: Option<bool>,
}

impl InstanceOptions {
    pub fn force_playtools(&mut self) -> &mut Self {
        match self.touch_mode {
            Some(touch_mode) if !matches!(touch_mode, TouchMode::MacPlayTools) => {
                warning!("Connect with PlayTools force touch mode to MacPlayTools");
                self.touch_mode = Some(TouchMode::MacPlayTools);
            }
            None => {
                info!("Connect with PlayTools set touch mode to MacPlayTools automatically");
                self.touch_mode = Some(TouchMode::MacPlayTools);
            }
            _ => {}
        }

        self
    }

    pub fn apply_to(&self, asst: &Assistant) -> Result<()> {
        if let Some(touch_mode) = self.touch_mode {
            debug!("Setting touch mode to", touch_mode);
            InstanceOptionKey::TouchMode
                .apply(asst, touch_mode)
                .with_context(|| format!("Failed to set touch mode to {}", touch_mode))?;
        }
        if let Some(deployment_with_pause) = self.deployment_with_pause {
            debug!("Setting deployment with pause to", deployment_with_pause);
            InstanceOptionKey::DeploymentWithPause
                .apply(asst, deployment_with_pause)
                .context("Failed to set deployment with pause")?;
        }
        if let Some(adb_lite_enabled) = self.adb_lite_enabled {
            debug!("Setting adb lite enabled to", adb_lite_enabled);
            InstanceOptionKey::AdbLiteEnabled
                .apply(asst, adb_lite_enabled)
                .context("Failed to set adb lite enabled")?;
        }
        if let Some(kill_adb_on_exit) = self.kill_adb_on_exit {
            debug!(format!("Setting kill adb on exit to {}", kill_adb_on_exit));
            InstanceOptionKey::KillAdbOnExit
                .apply(asst, kill_adb_on_exit)
                .context("Failed to set kill adb on exit")?;
        }
        Ok(())
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        ConnectionConfig::ADB {
            adb_path: default_adb_path(),
            device: default_device(),
            config: default_config(),
        }
    }
}

pub fn default_adb_path() -> String {
    String::from("adb")
}

pub fn default_device() -> String {
    String::from("emulator-5554")
}

pub fn default_playcover_address() -> String {
    String::from("localhost:1717")
}

pub fn default_config() -> String {
    if cfg!(target_os = "macos") {
        String::from("CompatMac")
    } else if cfg!(target_os = "linux") {
        String::from("CompatPOSIXShell")
    } else {
        String::from("General")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod serde {
        use super::*;

        #[test]
        fn deserialize_example() {
            let config: AsstConfig =
                toml::from_str(&std::fs::read_to_string("../config_examples/asst.toml").unwrap())
                    .unwrap();

            assert_eq!(
                config,
                AsstConfig {
                    connection: ConnectionConfig::ADB {
                        adb_path: String::from("adb"),
                        device: String::from("emulator-5554"),
                        config: String::from("CompatMac"),
                    },
                    resource: ResourceConfig {
                        resource_base_dirs: {
                            let mut base_dirs = default_resource_base_dirs();
                            push_user_resource(&mut base_dirs);
                            base_dirs
                        },
                        global_resource: Some(PathBuf::from("YoStarEN")),
                        platform_diff_resource: Some(PathBuf::from("iOS")),
                        user_resource: true,
                    },
                    static_options: StaticOptions {
                        cpu_ocr: Some(false),
                        gpu_ocr: Some(1),
                    },
                    instance_options: InstanceOptions {
                        touch_mode: Some(TouchMode::MaaTouch),
                        deployment_with_pause: Some(false),
                        adb_lite_enabled: Some(false),
                        kill_adb_on_exit: Some(false),
                    },
                }
            );
        }

        #[test]
        fn deserialize_empty() {
            let config: AsstConfig = toml::from_str("").unwrap();
            assert_eq!(
                config,
                AsstConfig {
                    connection: ConnectionConfig::ADB {
                        adb_path: String::from("adb"),
                        device: String::from("emulator-5554"),
                        config: if cfg!(target_os = "macos") {
                            String::from("CompatMac")
                        } else if cfg!(target_os = "linux") {
                            String::from("CompatPOSIXShell")
                        } else {
                            String::from("General")
                        },
                    },
                    resource: ResourceConfig {
                        resource_base_dirs: default_resource_base_dirs(),
                        global_resource: None,
                        platform_diff_resource: None,
                        user_resource: false,
                    },
                    static_options: StaticOptions {
                        cpu_ocr: None,
                        gpu_ocr: None,
                    },
                    instance_options: InstanceOptions {
                        touch_mode: None,
                        deployment_with_pause: None,
                        adb_lite_enabled: None,
                        kill_adb_on_exit: None,
                    },
                }
            );
        }
    }
}
