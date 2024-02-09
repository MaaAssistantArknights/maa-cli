use crate::dirs::{self, global_path};

use std::path::PathBuf;

use anyhow::{Context, Result};
use log::{debug, info, warn};
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

impl AsstConfig {
    pub fn new(
        connection: ConnectionConfig,
        mut resource: ResourceConfig,
        static_options: StaticOptions,
        mut instance_options: InstanceOptions,
    ) -> Self {
        if matches!(connection.preset, Preset::PlayCover)
        {
            info!("Detected connection with PlayTools");
            instance_options.force_playtools();
            resource.use_platform_diff_resource("iOS");
        }

        Self {
            connection,
            resource,
            static_options,
            instance_options,
        }
    }
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

        let config = AsstConfigHelper::deserialize(deserializer)?;

        Ok(AsstConfig::new(
            config.connection,
            config.resource,
            config.static_options,
            config.instance_options,
        ))
    }
}

impl super::FromFile for AsstConfig {}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Clone, Default)]
pub struct ConnectionConfig {
    #[serde(default, alias = "type")]
    preset: Preset,
    #[serde(default)]
    adb_path: Option<String>,
    #[serde(default, alias = "device")]
    address: Option<String>,
    #[serde(default)]
    config: Option<String>,
}

impl ConnectionConfig {
    pub fn preset(&self) -> Preset {
        self.preset
    }

    pub fn set_address(&mut self, address: impl Into<String>) -> &mut Self {
        self.address = Some(address.into());
        self
    }

    pub fn connect_args(&self) -> (&str, &str, &str) {
        let adb_path = self.adb_path.as_deref().unwrap_or_else(|| self.preset.default_adb_path());
        let address = self.address.as_deref().unwrap_or_else(|| self.preset.default_address());
        let config = self.config.as_deref().unwrap_or_else(|| self.preset.default_config());
        debug!(
            "Connecting to {address} with config {config} via {}",
            if matches!(self.preset, Preset::PlayCover) { "PlayTools" } else { adb_path }
        );

        (adb_path, address, config)
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum Preset {
    MuMuPro,
    PlayCover,
    #[default]
    ADB,
}

impl<'de> Deserialize<'de> for Preset {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PresetVisitor;

        impl<'de> serde::de::Visitor<'de> for PresetVisitor {
            type Value = Preset;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a connection preset name")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Preset, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MuMuPro" => Ok(Preset::MuMuPro),
                    "PlayCover" | "PlayTools" => Ok(Preset::PlayCover),
                    "ADB" => Ok(Preset::ADB),
                    _ => {
                        warn!("Unknown connection preset: {}, ignoring", value);
                        Ok(Preset::ADB)
                    }
                }
            }
        }
        
        deserializer.deserialize_str(PresetVisitor)
    }
}

impl Preset {
    fn default_adb_path(self) -> &'static str {
        match self {
            Preset::MuMuPro => 
                "/Applications/MuMuPlayer.app/Contents/MacOS/MuMuEmulator.app/Contents/MacOS/tools/adb",
            Preset::PlayCover => "",
            Preset::ADB => "adb",
        }
    }

    fn default_address(self) -> &'static str {
        match self {
            Preset::MuMuPro => "127.0.0.1:16384",
            Preset::PlayCover => "localhost:1717",
            Preset::ADB => "emulator-5554",
        }
    }

    fn default_config(self) -> &'static str {
        // May be preset specific in the future
        config_based_on_os()
    }
}

fn config_based_on_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "CompatMac"
    } else if cfg!(target_os = "linux") {
        "CompatLinux"
    } else {
        "General"
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone)]
pub struct ResourceConfig {
    /// Resources used by global arknights client, e.g. `YostarEN`
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
        debug!("Found resource directory: {}", resource_dir.display());
        resource_dirs.push(resource_dir.into_owned());
    } else {
        warn!("Resource directory not found!")
    }

    let hot_update_dir = dirs::hot_update();
    if hot_update_dir.exists() {
        debug!(
            "Found hot update resource directory: {}",
            hot_update_dir.display()
        );
        resource_dirs.push(hot_update_dir.join("resource"));
        resource_dirs.push(hot_update_dir.join("cache").join("resource"));
    } else {
        warn!("Hot update resource directory not found!");
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
                warn!(
                    "Global resource {} already set, ignoring {}",
                    global_resource.display(),
                    resource.into().display(),
                );
            }
            None => {
                let resource = resource.into();
                info!("Using global resource: {}", resource.display());
                self.global_resource = Some(resource);
            }
        }
        self
    }

    pub fn use_platform_diff_resource(&mut self, resource: impl Into<PathBuf>) -> &mut Self {
        match self.platform_diff_resource.as_ref() {
            Some(platform_diff_resource) => {
                warn!(
                    "Platform diff resource {} already set, ignoring {}",
                    platform_diff_resource.display(),
                    resource.into().display(),
                );
            }
            None => {
                let resource = resource.into();
                info!("Using platform diff resource: {}", resource.display());
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
            let global_resource_dir = PathBuf::from("global")
                .join(global_resource)
                .join("resource");
            let full_paths = global_path(base_dirs, global_resource_dir);
            if full_paths.is_empty() {
                warn!("Global resource {} not found", global_resource.display(),);
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
                warn!(
                    "Platform diff resource {} not found",
                    platform_diff_resource.display(),
                );
            } else {
                resource_dirs.extend(full_paths);
            }
        }

        resource_dirs
    }

    pub fn load(&self) -> Result<()> {
        let resource_dirs = self.resource_dirs();
        for resource_dir in resource_dirs {
            debug!("Loading resource from {}", resource_dir.display());
            Assistant::load_resource(resource_dir.parent().unwrap())?;
        }

        Ok(())
    }
}

fn push_user_resource(resource_dirs: &mut Vec<PathBuf>) -> &mut Vec<PathBuf> {
    push_resource(resource_dirs, dirs::config().join("resource"))
}

fn push_resource(resource_dirs: &mut Vec<PathBuf>, dir: impl Into<PathBuf>) -> &mut Vec<PathBuf> {
    let dir = dir.into();
    if dir.exists() {
        resource_dirs.push(dir);
    } else {
        warn!("Resource directory {} not found, ignoring", dir.display(),);
    }

    resource_dirs
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default, Clone)]
pub struct StaticOptions {
    #[serde(default)]
    cpu_ocr: Option<bool>,
    #[serde(default)]
    gpu_ocr: Option<u32>,
}

impl StaticOptions {
    pub fn apply(&self) -> Result<()> {
        match (self.cpu_ocr, self.gpu_ocr) {
            (Some(cpu_ocr), Some(gpu_id)) => {
                if cpu_ocr {
                    warn!("Both CPU OCR and GPU OCR are enabled, CPU OCR will be ignored");
                }
                debug!("Using GPU OCR with GPU ID {}", gpu_id);
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
    fn force_playtools(&mut self) -> &mut Self {
        match self.touch_mode {
            Some(touch_mode) if !matches!(touch_mode, TouchMode::MacPlayTools) => {
                warn!("Connect with PlayTools force touch mode to MacPlayTools");
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
            debug!("Setting touch mode to {}", touch_mode);
            InstanceOptionKey::TouchMode
                .apply_to(asst, touch_mode)
                .with_context(|| format!("Failed to set touch mode to {}", touch_mode))?;
        }
        if let Some(deployment_with_pause) = self.deployment_with_pause {
            debug!("Setting deployment with pause to {}", deployment_with_pause);
            InstanceOptionKey::DeploymentWithPause
                .apply_to(asst, deployment_with_pause)
                .context("Failed to set deployment with pause")?;
        }
        if let Some(adb_lite_enabled) = self.adb_lite_enabled {
            debug!("Setting adb lite enabled to {}", adb_lite_enabled);
            InstanceOptionKey::AdbLiteEnabled
                .apply_to(asst, adb_lite_enabled)
                .context("Failed to set adb lite enabled")?;
        }
        if let Some(kill_adb_on_exit) = self.kill_adb_on_exit {
            debug!("Setting kill adb on exit to {}", kill_adb_on_exit);
            InstanceOptionKey::KillAdbOnExit
                .apply_to(asst, kill_adb_on_exit)
                .context("Failed to set kill adb on exit")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::assert_matches;

    use lazy_static::lazy_static;

    lazy_static! {
        static ref USER_RESOURCE_DIR: PathBuf = {
            let user_resource_dir = dirs::config().join("resource");
            if !user_resource_dir.exists() {
                std::fs::create_dir_all(&user_resource_dir).unwrap();
            }
            user_resource_dir
        };
    }

    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, Token};

        #[test]
        #[ignore = "attempt to create a directory in user space"]
        fn deserialize_example() {
            let user_resource_dir = USER_RESOURCE_DIR.clone();

            let config: AsstConfig =
                toml::from_str(&std::fs::read_to_string("./config_examples/asst.toml").unwrap())
                    .unwrap();

            assert_eq!(
                config,
                AsstConfig {
                    connection: ConnectionConfig {
                        preset: Preset::ADB,
                        adb_path: Some(String::from("adb")),
                        address: Some(String::from("emulator-5554")),
                        config: Some(String::from("CompatMac")),
                    },
                    resource: ResourceConfig {
                        resource_base_dirs: {
                            let mut base_dirs = default_resource_base_dirs();
                            base_dirs.push(user_resource_dir);
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
        fn connection_config() {
            assert_de_tokens(
                &ConnectionConfig::default(),
                &[
                    Token::Map { len: Some(0) },
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &ConnectionConfig {
                    preset: Preset::ADB,
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("ADB"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &ConnectionConfig {
                    preset: Preset::ADB,
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("preset"),
                    Token::Str("ADB"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &ConnectionConfig {
                    preset: Preset::MuMuPro,
                    ..Default::default()
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("preset"),
                    Token::Str("MuMuPro"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &ConnectionConfig {
                    preset: Preset::ADB,
                    adb_path: Some(String::from("/path/to/adb")),
                    address: Some(String::from("127.0.0.1:5555")),
                    config: Some(String::from("SomeConfig")),
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("type"),
                    Token::Str("ADB"),
                    Token::Str("adb_path"),
                    Token::Some,
                    Token::Str("/path/to/adb"),
                    Token::Str("device"),
                    Token::Some,
                    Token::Str("127.0.0.1:5555"),
                    Token::Str("config"),
                    Token::Some,
                    Token::Str("SomeConfig"),
                    Token::MapEnd,
                ],
            );

        }

        #[test]
        #[ignore = "attempt to create a directory in user space"]
        fn resource_config() {
            assert_de_tokens(
                &ResourceConfig {
                    resource_base_dirs: default_resource_base_dirs(),
                    global_resource: None,
                    platform_diff_resource: None,
                    user_resource: false,
                },
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            let user_resource_dir = USER_RESOURCE_DIR.clone();

            assert_de_tokens(
                &ResourceConfig {
                    resource_base_dirs: {
                        let mut base_dirs = default_resource_base_dirs();
                        base_dirs.push(user_resource_dir);
                        base_dirs
                    },
                    global_resource: Some(PathBuf::from("YoStarEN")),
                    platform_diff_resource: Some(PathBuf::from("iOS")),
                    user_resource: true,
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("global_resource"),
                    Token::Some,
                    Token::Str("YoStarEN"),
                    Token::Str("platform_diff_resource"),
                    Token::Some,
                    Token::Str("iOS"),
                    Token::Str("user_resource"),
                    Token::Bool(true),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn static_options() {
            assert_de_tokens(
                &StaticOptions {
                    cpu_ocr: None,
                    gpu_ocr: None,
                },
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            assert_de_tokens(
                &StaticOptions {
                    cpu_ocr: Some(false),
                    gpu_ocr: Some(1),
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("cpu_ocr"),
                    Token::Some,
                    Token::Bool(false),
                    Token::Str("gpu_ocr"),
                    Token::Some,
                    Token::U32(1),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn instance_options() {
            assert_de_tokens(
                &InstanceOptions {
                    touch_mode: None,
                    deployment_with_pause: None,
                    adb_lite_enabled: None,
                    kill_adb_on_exit: None,
                },
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            assert_de_tokens(
                &InstanceOptions {
                    touch_mode: Some(TouchMode::ADB),
                    deployment_with_pause: Some(false),
                    adb_lite_enabled: Some(false),
                    kill_adb_on_exit: Some(false),
                },
                &[
                    Token::Map { len: Some(4) },
                    Token::Str("touch_mode"),
                    Token::Some,
                    Token::UnitVariant {
                        name: "TouchMode",
                        variant: "ADB",
                    },
                    Token::Str("deployment_with_pause"),
                    Token::Some,
                    Token::Bool(false),
                    Token::Str("adb_lite_enabled"),
                    Token::Some,
                    Token::Bool(false),
                    Token::Str("kill_adb_on_exit"),
                    Token::Some,
                    Token::Bool(false),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn asst_config() {
            assert_de_tokens(
                &AsstConfig {
                    connection: ConnectionConfig::default(),
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
                },
                &[Token::Map { len: Some(0) }, Token::MapEnd],
            );

            // Auto load iOS resource and set touch mode to MacPlayTools
            assert_de_tokens(
                &AsstConfig {
                    connection: ConnectionConfig {
                        preset: Preset::PlayCover,
                        ..Default::default()
                    },
                    resource: ResourceConfig {
                        platform_diff_resource: Some(PathBuf::from("iOS")),
                        ..Default::default()
                    },
                    static_options: Default::default(),
                    instance_options: InstanceOptions {
                        touch_mode: Some(TouchMode::MacPlayTools),
                        ..Default::default()
                    },
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("connection"),
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("PlayTools"),
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            );
        }
    }

    mod connection_config {
        use super::*;

        #[test]
        fn default() {
            assert_matches!(
                ConnectionConfig::default(),
                ConnectionConfig {
                    preset: Preset::ADB,
                    adb_path: None,
                    address: None,
                    config: None,
                }
            );
        }

        #[test]
        fn connect_args() {
            assert_eq!(
                ConnectionConfig::default().connect_args(),
                ("adb", "emulator-5554", config_based_on_os()),
            );


            assert_eq!(
                ConnectionConfig {
                    preset: Preset::MuMuPro,
                    adb_path: None,
                    address: None,
                    config: None,
                }
                .connect_args(),
                (
                    "/Applications/MuMuPlayer.app/Contents/MacOS/MuMuEmulator.app/Contents/MacOS/tools/adb",
                    "127.0.0.1:16384",
                    config_based_on_os(),
                ),
            );



            assert_eq!(
                ConnectionConfig {
                    preset: Preset::ADB,
                    adb_path: Some("/path/to/adb".to_owned()),
                    address: Some("127.0.0.1:11111".to_owned()),
                    config: Some("SomeConfig".to_owned()),
                }
                .connect_args(),
                ("/path/to/adb", "127.0.0.1:11111", "SomeConfig"),
            );

        }
    }

    mod resource_config {
        use super::*;

        use crate::dirs::Ensure;

        use std::{env::temp_dir, fs};

        #[test]
        fn default() {
            assert_eq!(
                ResourceConfig::default(),
                ResourceConfig {
                    resource_base_dirs: default_resource_base_dirs(),
                    global_resource: None,
                    platform_diff_resource: None,
                    user_resource: false,
                }
            );
        }

        #[test]
        #[ignore = "attempt to create a directory in user space"]
        fn use_user_resource() {
            let user_resource_dir = USER_RESOURCE_DIR.clone();

            assert_eq!(
                *ResourceConfig::default().use_user_resource(),
                ResourceConfig {
                    resource_base_dirs: {
                        let mut base_dirs = default_resource_base_dirs();
                        base_dirs.push(user_resource_dir.to_path_buf());
                        base_dirs
                    },
                    global_resource: None,
                    platform_diff_resource: None,
                    user_resource: true,
                }
            );
        }

        #[test]
        fn use_global_resource() {
            assert_eq!(
                *ResourceConfig::default().use_global_resource("YoStarEN"),
                ResourceConfig {
                    resource_base_dirs: default_resource_base_dirs(),
                    global_resource: Some(PathBuf::from("YoStarEN")),
                    platform_diff_resource: None,
                    user_resource: false,
                }
            );

            assert_eq!(
                *ResourceConfig::default()
                    .use_global_resource("YoStarEN")
                    .use_global_resource("YostarJP"),
                ResourceConfig {
                    resource_base_dirs: default_resource_base_dirs(),
                    global_resource: Some(PathBuf::from("YoStarEN")),
                    platform_diff_resource: None,
                    user_resource: false,
                }
            );
        }

        #[test]
        fn use_platform_diff_resource() {
            assert_matches!(
                ResourceConfig::default().use_platform_diff_resource("iOS"),
                ResourceConfig {
                    platform_diff_resource: Some(path),
                    ..
                } if *path == PathBuf::from("iOS")
            );

            assert_matches!(
                ResourceConfig {
                    platform_diff_resource: Some(PathBuf::from("iOS")),
                    ..Default::default()
                }.use_platform_diff_resource("Other"),
                ResourceConfig {
                    platform_diff_resource: Some(path),
                    ..
                } if *path == PathBuf::from("iOS")
            );
        }

        #[test]
        fn base_dirs() {
            assert_eq!(
                *ResourceConfig {
                    resource_base_dirs: vec![PathBuf::from("resource")],
                    ..Default::default()
                }
                .base_dirs(),
                [PathBuf::from("resource")]
            );
        }

        #[test]
        fn test_push_resource() {
            let test_root = temp_dir().join("push_resource");

            let resource_dir = test_root.join("resource");
            let unexists_resource_dir = test_root.join("unexists_resource");

            resource_dir.ensure().unwrap();

            assert_eq!(
                push_resource(&mut Vec::new(), resource_dir.clone()),
                &[resource_dir.clone()]
            );

            assert_eq!(
                push_resource(&mut Vec::new(), unexists_resource_dir.clone()),
                &Vec::<PathBuf>::new()
            );

            fs::remove_dir_all(test_root).unwrap();
        }

        #[test]
        fn resource_dir() {
            let test_root = temp_dir().join("resource_config");
            let resource_dir = test_root.join("resource");
            let yostar_en_dir = resource_dir
                .join("global")
                .join("YoStarEN")
                .join("resource");
            let ios_dir = resource_dir
                .join("platform_diff")
                .join("iOS")
                .join("resource");

            yostar_en_dir.ensure().unwrap();
            ios_dir.ensure().unwrap();

            assert_eq!(
                ResourceConfig {
                    resource_base_dirs: vec![resource_dir.clone()],
                    ..Default::default()
                }
                .resource_dirs(),
                [resource_dir.clone()]
            );

            assert_eq!(
                ResourceConfig {
                    resource_base_dirs: vec![resource_dir.clone()],
                    global_resource: Some(PathBuf::from("YoStarEN")),
                    ..Default::default()
                }
                .resource_dirs(),
                [resource_dir.clone(), yostar_en_dir.clone()]
            );

            assert_eq!(
                ResourceConfig {
                    resource_base_dirs: vec![resource_dir.clone()],
                    global_resource: Some(PathBuf::from("NotExists")),
                    ..Default::default()
                }
                .resource_dirs(),
                [resource_dir.clone()]
            );

            assert_eq!(
                ResourceConfig {
                    resource_base_dirs: vec![resource_dir.clone()],
                    platform_diff_resource: Some(PathBuf::from("iOS")),
                    ..Default::default()
                }
                .resource_dirs(),
                [resource_dir.clone(), ios_dir.clone()]
            );

            assert_eq!(
                ResourceConfig {
                    resource_base_dirs: vec![resource_dir.clone()],
                    platform_diff_resource: Some(PathBuf::from("NotExists")),
                    ..Default::default()
                }
                .resource_dirs(),
                [resource_dir.clone()]
            );

            fs::remove_dir_all(test_root).unwrap();
        }
    }

    #[test]
    fn instance_options() {
        assert_matches!(
            InstanceOptions {
                touch_mode: None,
                ..Default::default()
            }
            .force_playtools(),
            InstanceOptions {
                touch_mode: Some(TouchMode::MacPlayTools),
                ..
            }
        );

        assert_matches!(
            InstanceOptions {
                touch_mode: Some(TouchMode::ADB),
                ..Default::default()
            }
            .force_playtools(),
            InstanceOptions {
                touch_mode: Some(TouchMode::MacPlayTools),
                ..
            }
        );
    }
}
