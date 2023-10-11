use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default)]
pub struct AsstConfig {
    #[serde(default)]
    pub user_resource: bool,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub connection: Connection,
    #[serde(default)]
    pub instance_options: InstanceOption,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize, Default)]
pub struct InstanceOption {
    #[serde(default)]
    pub touch_mode: Option<TouchMode>,
    pub deployment_with_pause: Option<bool>,
    pub adb_lite_enabled: Option<bool>,
    pub kill_adb_on_exit: Option<bool>,
}

#[derive(Deserialize, Debug, Default, PartialEq, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum TouchMode {
    #[default]
    ADB,
    MiniTouch,
    MAATouch,
    MacPlayTools,
}

impl<'a> From<TouchMode> for &'a str {
    fn from(mode: TouchMode) -> Self {
        match mode {
            TouchMode::ADB => "adb",
            TouchMode::MiniTouch => "minitouch",
            TouchMode::MAATouch => "maatouch",
            TouchMode::MacPlayTools => "MacPlayTools",
        }
    }
}

impl std::fmt::Display for TouchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <&str>::from(*self))
    }
}

impl maa_sys::ToCString for TouchMode {
    fn to_cstring(self) -> maa_sys::Result<std::ffi::CString> {
        <&str>::from(self).to_cstring()
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
#[allow(clippy::upper_case_acronyms)]
pub enum Connection {
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

impl Default for Connection {
    fn default() -> Self {
        Connection::ADB {
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

impl super::FromFile for AsstConfig {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_example() {
        let config: AsstConfig =
            toml::from_str(&std::fs::read_to_string("../config_examples/asst.toml").unwrap())
                .unwrap();
        assert_eq!(
            config,
            AsstConfig {
                user_resource: true,
                resources: vec![String::from("platform_diff/macOS")],
                connection: Connection::ADB {
                    adb_path: String::from("adb"),
                    device: String::from("emulator-5554"),
                    config: String::from("CompatMac"),
                },
                instance_options: InstanceOption {
                    touch_mode: Some(TouchMode::MAATouch),
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
                user_resource: false,
                resources: vec![],
                connection: Connection::ADB {
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
                instance_options: InstanceOption {
                    touch_mode: None,
                    deployment_with_pause: None,
                    adb_lite_enabled: None,
                    kill_adb_on_exit: None,
                },
            }
        );
    }
}
