use crate::maacore::ToCString;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AsstConfig {
    pub connection: Option<Connection>,
    pub instance_options: Option<InstanceOption>,
}

#[derive(Deserialize)]
pub struct InstanceOption {
    #[serde(default)]
    pub touch_mode: TouchMode,
    pub deployment_with_pause: Option<bool>,
    pub adb_lite_enabled: Option<bool>,
    pub kill_adb_on_exit: Option<bool>,
}

impl ToCString for bool {
    fn to_cstring(self) -> Result<std::ffi::CString, std::ffi::NulError> {
        Ok(if self { "1" } else { "0" }.to_cstring()?)
    }
}

#[derive(Deserialize, Debug)]
pub enum TouchMode {
    Abd,
    Minitouch,
    Maatouch,
    MacPlayTools,
}

impl ToCString for TouchMode {
    fn to_cstring(self) -> Result<std::ffi::CString, std::ffi::NulError> {
        Ok(match self {
            TouchMode::Abd => "adb",
            TouchMode::Minitouch => "minitouch",
            TouchMode::Maatouch => "maatouch",
            TouchMode::MacPlayTools => "macplaytools",
        }
        .to_cstring()?)
    }
}

impl Default for TouchMode {
    fn default() -> Self {
        TouchMode::Minitouch
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
pub enum Connection {
    ADB {
        #[serde(default = "default_adb_path")]
        adb_path: String,
        #[serde(default = "default_device")]
        device: String,
        #[serde(default = "default_config")]
        config: String,
    },
    Playcover {},
}

pub fn default_adb_path() -> String {
    String::from("adb")
}

pub fn default_device() -> String {
    String::from("emulator-5554")
}

#[cfg(not(target_os = "macos"))]
pub fn default_config() -> String {
    String::from("General")
}

#[cfg(target_os = "macos")]
pub fn default_config() -> String {
    String::from("CompatMac")
}

impl super::FromFile for AsstConfig {}

#[cfg(test)]
mod tests {}
