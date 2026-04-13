use maa_types::TouchMode;
use serde::{Deserialize, Serialize};

/// Payload struct for `ConnectionConfig::General`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GeneralConnectionConfig {
    pub address: Option<String>,
    pub adb_path: Option<String>,
    pub touch_mode: Option<TouchMode>,
    pub adb_lite: Option<bool>,
    pub kill_adb_on_exit: Option<bool>,
    pub config: Option<String>,
}

impl GeneralConnectionConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            address: other.address.or(self.address),
            adb_path: other.adb_path.or(self.adb_path),
            touch_mode: other.touch_mode.or(self.touch_mode),
            adb_lite: other.adb_lite.or(self.adb_lite),
            kill_adb_on_exit: other.kill_adb_on_exit.or(self.kill_adb_on_exit),
            config: other.config.or(self.config),
        }
    }
}

/// Payload struct for `ConnectionConfig::PlayCover`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct PlayCoverConnectionConfig {
    pub address: Option<String>,
    pub screencap_mode: Option<ScreencapMode>,
}

impl PlayCoverConnectionConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            address: other.address.or(self.address),
            screencap_mode: other.screencap_mode.or(self.screencap_mode),
        }
    }
}

/// Payload struct for `ConnectionConfig::Waydroid`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct WaydroidConnectionConfig {
    pub adb_path: Option<String>,
    pub touch_mode: Option<TouchMode>,
    pub adb_lite: Option<bool>,
}

impl WaydroidConnectionConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            adb_path: other.adb_path.or(self.adb_path),
            touch_mode: other.touch_mode.or(self.touch_mode),
            adb_lite: other.adb_lite.or(self.adb_lite),
        }
    }
}

/// Payload struct for `ConnectionConfig::MuMuPro`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct MuMuProConnectionConfig {
    pub address: Option<String>,
    pub touch_mode: Option<TouchMode>,
    pub adb_lite: Option<bool>,
    pub kill_adb_on_exit: Option<bool>,
}

impl MuMuProConnectionConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            address: other.address.or(self.address),
            touch_mode: other.touch_mode.or(self.touch_mode),
            adb_lite: other.adb_lite.or(self.adb_lite),
            kill_adb_on_exit: other.kill_adb_on_exit.or(self.kill_adb_on_exit),
        }
    }
}

/// Payload struct for `ConnectionConfig::AVD`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct AvdConnectionConfig {
    pub sdk_path: Option<String>,
    pub avd_name: Option<String>,
    pub touch_mode: Option<TouchMode>,
    pub adb_lite: Option<bool>,
    pub kill_adb_on_exit: Option<bool>,
}

impl AvdConnectionConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            sdk_path: other.sdk_path.or(self.sdk_path),
            avd_name: other.avd_name.or(self.avd_name),
            touch_mode: other.touch_mode.or(self.touch_mode),
            adb_lite: other.adb_lite.or(self.adb_lite),
            kill_adb_on_exit: other.kill_adb_on_exit.or(self.kill_adb_on_exit),
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum ConnectionConfig {
    General(GeneralConnectionConfig),
    PlayCover(PlayCoverConnectionConfig),
    Waydroid(WaydroidConnectionConfig),
    MuMuPro(MuMuProConnectionConfig),
    AVD(AvdConnectionConfig),
}

impl ConnectionConfig {
    /// Merge two connection configs.
    ///
    /// Same-variant: field-level `Option::or` merge.
    /// Cross-variant: child replaces parent.
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::General(p), Self::General(c)) => Self::General(p.merge(c)),
            (Self::PlayCover(p), Self::PlayCover(c)) => Self::PlayCover(p.merge(c)),
            (Self::Waydroid(p), Self::Waydroid(c)) => Self::Waydroid(p.merge(c)),
            (Self::MuMuPro(p), Self::MuMuPro(c)) => Self::MuMuPro(p.merge(c)),
            (Self::AVD(p), Self::AVD(c)) => Self::AVD(p.merge(c)),
            (_, child) => child,
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreencapMode {
    Default,
    BGR,
    SCK,
}

#[cfg(test)]
mod tests {
    use maa_types::TouchMode;

    use crate::profile::{
        ConnectionConfig,
        connection::{
            AvdConnectionConfig, GeneralConnectionConfig, MuMuProConnectionConfig,
            PlayCoverConnectionConfig, ScreencapMode, WaydroidConnectionConfig,
        },
    };

    fn general(config: GeneralConnectionConfig) -> ConnectionConfig {
        ConnectionConfig::General(config)
    }

    fn playcover(config: PlayCoverConnectionConfig) -> ConnectionConfig {
        ConnectionConfig::PlayCover(config)
    }

    fn waydroid(config: WaydroidConnectionConfig) -> ConnectionConfig {
        ConnectionConfig::Waydroid(config)
    }

    fn mumupro(config: MuMuProConnectionConfig) -> ConnectionConfig {
        ConnectionConfig::MuMuPro(config)
    }

    #[test]
    fn same_variant_general_partial_override() {
        let parent = general(GeneralConnectionConfig {
            address: Some("parent-address".into()),
            adb_path: Some("parent-adb".into()),
            touch_mode: Some(TouchMode::MaaTouch),
            adb_lite: Some(false),
            kill_adb_on_exit: Some(false),
            config: Some("parent-config".into()),
        });
        let child = general(GeneralConnectionConfig {
            address: Some("child-address".into()),
            ..Default::default()
        });

        assert_eq!(
            parent.merge(child),
            general(GeneralConnectionConfig {
                address: Some("child-address".into()),
                adb_path: Some("parent-adb".into()),
                touch_mode: Some(TouchMode::MaaTouch),
                adb_lite: Some(false),
                kill_adb_on_exit: Some(false),
                config: Some("parent-config".into()),
            })
        );
    }

    #[test]
    fn same_variant_general_full_override() {
        let parent = general(GeneralConnectionConfig {
            address: Some("parent-address".into()),
            adb_path: Some("parent-adb".into()),
            touch_mode: Some(TouchMode::Adb),
            adb_lite: Some(false),
            kill_adb_on_exit: Some(false),
            config: Some("parent-config".into()),
        });
        let child = general(GeneralConnectionConfig {
            address: Some("child-address".into()),
            adb_path: Some("child-adb".into()),
            touch_mode: Some(TouchMode::MiniTouch),
            adb_lite: Some(true),
            kill_adb_on_exit: Some(true),
            config: Some("child-config".into()),
        });

        assert_eq!(
            parent.merge(child),
            general(GeneralConnectionConfig {
                address: Some("child-address".into()),
                adb_path: Some("child-adb".into()),
                touch_mode: Some(TouchMode::MiniTouch),
                adb_lite: Some(true),
                kill_adb_on_exit: Some(true),
                config: Some("child-config".into()),
            })
        );
    }

    #[test]
    fn same_variant_general_empty_child() {
        let parent = general(GeneralConnectionConfig {
            address: Some("parent-address".into()),
            adb_path: Some("parent-adb".into()),
            touch_mode: Some(TouchMode::MaaTouch),
            adb_lite: Some(false),
            kill_adb_on_exit: Some(true),
            config: Some("parent-config".into()),
        });
        let child = general(GeneralConnectionConfig::default());

        assert_eq!(parent.clone().merge(child), parent);
    }

    #[test]
    fn same_variant_mumupro() {
        let parent = mumupro(MuMuProConnectionConfig {
            address: Some("127.0.0.1:16384".into()),
            touch_mode: Some(TouchMode::MaaTouch),
            adb_lite: Some(false),
            kill_adb_on_exit: Some(false),
        });
        let child = mumupro(MuMuProConnectionConfig {
            touch_mode: Some(TouchMode::MiniTouch),
            adb_lite: Some(true),
            ..Default::default()
        });

        assert_eq!(
            parent.merge(child),
            mumupro(MuMuProConnectionConfig {
                address: Some("127.0.0.1:16384".into()),
                touch_mode: Some(TouchMode::MiniTouch),
                adb_lite: Some(true),
                kill_adb_on_exit: Some(false),
            })
        );
    }

    #[test]
    fn same_variant_playcover() {
        let parent = playcover(PlayCoverConnectionConfig {
            address: Some("127.0.0.1:1717".into()),
            screencap_mode: Some(ScreencapMode::Default),
        });
        let child = playcover(PlayCoverConnectionConfig {
            screencap_mode: Some(ScreencapMode::SCK),
            ..Default::default()
        });

        assert_eq!(
            parent.merge(child),
            playcover(PlayCoverConnectionConfig {
                address: Some("127.0.0.1:1717".into()),
                screencap_mode: Some(ScreencapMode::SCK),
            })
        );
    }

    #[test]
    fn same_variant_waydroid() {
        let parent = waydroid(WaydroidConnectionConfig {
            adb_path: Some("parent-adb".into()),
            touch_mode: Some(TouchMode::Adb),
            adb_lite: Some(false),
        });
        let child = waydroid(WaydroidConnectionConfig {
            adb_path: Some("child-adb".into()),
            adb_lite: Some(true),
            ..Default::default()
        });

        assert_eq!(
            parent.merge(child),
            waydroid(WaydroidConnectionConfig {
                adb_path: Some("child-adb".into()),
                touch_mode: Some(TouchMode::Adb),
                adb_lite: Some(true),
            })
        );
    }

    #[test]
    fn same_variant_avd() {
        let parent = ConnectionConfig::AVD(AvdConnectionConfig {
            sdk_path: Some("/parent/sdk".into()),
            avd_name: Some("parent-avd".into()),
            touch_mode: Some(TouchMode::MaaTouch),
            adb_lite: Some(false),
            kill_adb_on_exit: Some(false),
        });
        let child = ConnectionConfig::AVD(AvdConnectionConfig {
            sdk_path: Some("/child/sdk".into()),
            touch_mode: Some(TouchMode::MiniTouch),
            adb_lite: Some(true),
            ..Default::default()
        });

        assert_eq!(
            parent.merge(child),
            ConnectionConfig::AVD(AvdConnectionConfig {
                sdk_path: Some("/child/sdk".into()),
                avd_name: Some("parent-avd".into()),
                touch_mode: Some(TouchMode::MiniTouch),
                adb_lite: Some(true),
                kill_adb_on_exit: Some(false),
            })
        );
    }

    #[test]
    fn cross_variant_replaces() {
        let parent = general(GeneralConnectionConfig {
            address: Some("parent-address".into()),
            adb_path: Some("parent-adb".into()),
            touch_mode: Some(TouchMode::MaaTouch),
            adb_lite: Some(false),
            kill_adb_on_exit: Some(false),
            config: Some("parent-config".into()),
        });
        let child = playcover(PlayCoverConnectionConfig {
            address: Some("127.0.0.1:1717".into()),
            screencap_mode: Some(ScreencapMode::BGR),
        });

        assert_eq!(parent.merge(child.clone()), child);
    }

    #[test]
    fn cross_variant_all() {
        let playcover = playcover(PlayCoverConnectionConfig {
            address: Some("127.0.0.1:1717".into()),
            screencap_mode: Some(ScreencapMode::Default),
        });
        let waydroid = waydroid(WaydroidConnectionConfig {
            adb_path: Some("adb".into()),
            touch_mode: Some(TouchMode::MaaTouch),
            adb_lite: Some(false),
        });
        let mumupro = mumupro(MuMuProConnectionConfig {
            address: Some("127.0.0.1:16384".into()),
            touch_mode: Some(TouchMode::MiniTouch),
            adb_lite: Some(true),
            kill_adb_on_exit: Some(true),
        });

        assert_eq!(playcover.clone().merge(waydroid.clone()), waydroid);
        assert_eq!(mumupro.merge(playcover.clone()), playcover);
    }
}
