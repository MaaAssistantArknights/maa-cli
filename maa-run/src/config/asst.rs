use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct AsstConfig {
    pub connection: Option<Connection>,
    pub instance_options: Option<InstanceOption>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct InstanceOption {
    #[serde(default)]
    pub touch_mode: TouchMode,
    pub deployment_with_pause: Option<bool>,
    pub adb_lite_enabled: Option<bool>,
    pub kill_adb_on_exit: Option<bool>,
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum TouchMode {
    #[default]
    ADB,
    MiniTouch,
    MAATouch,
    MacPlayTools,
}

impl maa_sys::ToCString for TouchMode {
    fn to_cstring(self) -> maa_sys::Result<std::ffi::CString> {
        match self {
            TouchMode::ADB => "adb",
            TouchMode::MiniTouch => "minitouch",
            TouchMode::MAATouch => "maatouch",
            TouchMode::MacPlayTools => "MacPlayTools",
        }
        .to_cstring()
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
    PlayCover {
        #[serde(default = "default_playcover_address")]
        address: String,
        #[serde(default = "default_config")]
        config: String,
    },
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
                connection: Some(Connection::ADB {
                    adb_path: String::from("adb"),
                    device: String::from("emulator-5554"),
                    config: String::from("CompatMac"),
                }),
                instance_options: Some(InstanceOption {
                    touch_mode: TouchMode::MiniTouch,
                    deployment_with_pause: Some(false),
                    adb_lite_enabled: Some(false),
                    kill_adb_on_exit: Some(false),
                }),
            }
        );
    }
}
