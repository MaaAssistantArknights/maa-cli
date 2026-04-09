use maa_types::{ClientType, TouchMode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VersionedProfileConfig {
    pub version: crate::Version,
    #[serde(flatten)]
    pub config: ProfileConfig,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ProfileConfig {
    #[serde(default)]
    pub inherits: Option<String>,
    #[serde(default)]
    pub client_type: Option<ClientType>,
    pub connection: ConnectionConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum ConnectionConfig {
    General {
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        adb_path: Option<String>,
        #[serde(default)]
        touch_mode: Option<TouchMode>,
        #[serde(default)]
        adb_lite: Option<bool>,
        #[serde(default)]
        kill_adb_on_exit: Option<bool>,
        #[serde(default)]
        config: Option<String>,
    },
    PlayCover {
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        screencap_mode: Option<ScreencapMode>,
    },
    Waydroid {
        #[serde(default)]
        adb_path: Option<String>,
        #[serde(default)]
        touch_mode: Option<TouchMode>,
        #[serde(default)]
        adb_lite: Option<bool>,
    },
    MuMuPro {
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        touch_mode: Option<TouchMode>,
        #[serde(default)]
        adb_lite: Option<bool>,
        #[serde(default)]
        kill_adb_on_exit: Option<bool>,
    },
    AVD {
        sdk_path: String,
        #[serde(default)]
        avd_name: Option<String>,
        #[serde(default)]
        touch_mode: Option<TouchMode>,
        #[serde(default)]
        adb_lite: Option<bool>,
        #[serde(default)]
        kill_adb_on_exit: Option<bool>,
    },
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreencapMode {
    Default,
    BGR,
    SCK,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    #[serde(default)]
    pub auto_reconnect: Option<bool>,
    #[serde(default)]
    pub deployment_with_pause: Option<bool>,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdvancedConfig {
    #[serde(default)]
    pub inference_engine: Option<String>,
    #[serde(default)]
    pub user_resource: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_general_profile_from_toml() {
        let profile: ProfileConfig = toml::from_str(
            r#"
version = 2
client_type = "Official"

[connection]
type = "General"
address = "emulator-5554"
touch_mode = "MaaTouch"

[behavior]
auto_reconnect = true
"#,
        )
        .unwrap();

        assert_eq!(profile.client_type, Some(ClientType::Official));
        assert_eq!(profile.behavior.auto_reconnect, Some(true));
    }

    #[test]
    fn deserialize_playcover_profile_from_yaml() {
        let profile: ProfileConfig = serde_yaml::from_str(
            r#"
version: 2
connection:
  type: PlayCover
  screencap_mode: SCK
"#,
        )
        .unwrap();

        assert!(matches!(
            profile.connection,
            ConnectionConfig::PlayCover { .. }
        ));
    }
}
