use maa_types::ClientType;
use serde::Deserialize;

use crate::profile::{AdvancedConfig, BehaviorConfig, ConnectionConfig};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VersionedProfileConfig {
    pub version: crate::Version,
    #[serde(flatten)]
    pub config: ProfileConfig,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct ProfileConfig {
    pub inherits: Option<String>,
    pub client_type: Option<ClientType>,
    #[serde(default)]
    pub connection: Option<ConnectionConfig>,
    #[serde(default)]
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

/// A resolved/final profile configuration after inheritance is resolved and validated.
///
/// Unlike [`ProfileConfig`], this type:
/// - Has no `inherits` field (inheritance is already resolved)
/// - Has a required `connection` field (validation ensures it's present)
/// - Represents a fully validated configuration ready for use
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedProfileConfig {
    pub client_type: Option<ClientType>,
    pub connection: ConnectionConfig,
    pub behavior: BehaviorConfig,
    pub advanced: AdvancedConfig,
}

impl ProfileConfig {
    fn validate_connection(connection: &ConnectionConfig) -> Result<(), crate::ValidationError> {
        match connection {
            ConnectionConfig::AVD(crate::profile::connection::AvdConnectionConfig {
                sdk_path: Some(path),
                ..
            }) if path.trim().is_empty() => Err(crate::ValidationError::EmptyAvdSdkPath),
            _ => Ok(()),
        }
    }

    /// Merge a parent (self) and child (other) profile.
    ///
    /// For `connection`:
    /// - Both `Some`: same-variant merges field-by-field; cross-variant child replaces parent.
    /// - Child `None`: inherits parent's connection.
    /// - Parent `None`, child `Some`: uses child's connection.
    /// - Both `None`: result is `None` (will fail validation).
    pub fn merge(self, other: Self) -> Self {
        let connection = match (self.connection, other.connection) {
            (Some(parent), Some(child)) => Some(parent.merge(child)),
            (Some(parent), None) => Some(parent),
            (None, Some(child)) => Some(child),
            (None, None) => None,
        };

        Self {
            inherits: None, // Inheritance resolved by caller
            client_type: other.client_type.or(self.client_type),
            connection,
            behavior: self.behavior.merge(other.behavior),
            advanced: self.advanced.merge(other.advanced),
        }
    }

    /// Validate that a resolved profile has all required fields.
    ///
    /// After inheritance is resolved, `connection` must be present.
    pub fn validate(&self) -> Result<(), crate::ValidationError> {
        match self.connection.as_ref() {
            None => Err(crate::ValidationError::MissingConnection),
            Some(connection) => Self::validate_connection(connection),
        }
    }

    /// Resolve this profile into a [`ResolvedProfileConfig`].
    ///
    /// This validates the profile and converts `Option<ConnectionConfig>` to
    /// the required `ConnectionConfig`, returning an error if connection is missing
    /// or if AVD has an empty/whitespace SDK path.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::MissingConnection`] if `connection` is `None`.
    /// Returns [`ValidationError::EmptyAvdSdkPath`] if AVD connection has an empty SDK path.
    pub fn resolve(self) -> Result<ResolvedProfileConfig, crate::ValidationError> {
        let connection = self
            .connection
            .ok_or(crate::ValidationError::MissingConnection)?;

        Self::validate_connection(&connection)?;

        Ok(ResolvedProfileConfig {
            client_type: self.client_type,
            connection,
            behavior: self.behavior,
            advanced: self.advanced,
        })
    }
}

impl TryFrom<ProfileConfig> for ResolvedProfileConfig {
    type Error = crate::ValidationError;

    fn try_from(config: ProfileConfig) -> Result<Self, Self::Error> {
        config.resolve()
    }
}

#[cfg(test)]
mod tests {
    use maa_types::{ClientType, TouchMode};

    use crate::profile::{
        AdvancedConfig, BehaviorConfig, ConnectionConfig, ProfileConfig, ResolvedProfileConfig,
        connection::{AvdConnectionConfig, GeneralConnectionConfig},
    };

    fn profile(connection: Option<ConnectionConfig>) -> ProfileConfig {
        ProfileConfig {
            connection,
            ..Default::default()
        }
    }

    fn inherited_profile(
        inherits: &str,
        client_type: Option<ClientType>,
        connection: Option<ConnectionConfig>,
    ) -> ProfileConfig {
        ProfileConfig {
            inherits: Some(inherits.into()),
            client_type,
            connection,
            ..Default::default()
        }
    }

    fn avd_profile(sdk_path: Option<&str>) -> ProfileConfig {
        profile(Some(ConnectionConfig::AVD(AvdConnectionConfig {
            sdk_path: sdk_path.map(Into::into),
            ..Default::default()
        })))
    }

    fn general_connection(
        address: Option<&str>,
        touch_mode: Option<TouchMode>,
    ) -> ConnectionConfig {
        ConnectionConfig::General(GeneralConnectionConfig {
            address: address.map(Into::into),
            adb_path: Some("adb".into()),
            touch_mode,
            adb_lite: Some(false),
            kill_adb_on_exit: Some(false),
            config: Some("General".into()),
        })
    }

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
        assert!(matches!(
            profile.connection,
            Some(ConnectionConfig::General(..))
        ));
        assert_eq!(profile.behavior.auto_reconnect, Some(true));
    }

    #[test]
    fn deserialize_playcover_profile_from_yaml() {
        use crate::profile::connection::{PlayCoverConnectionConfig, ScreencapMode};

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
            Some(ConnectionConfig::PlayCover(PlayCoverConnectionConfig {
                screencap_mode: Some(ScreencapMode::SCK),
                ..
            }))
        ));
    }

    #[test]
    fn deserialize_inheritance_only_profile_without_connection() {
        let profile: ProfileConfig = toml::from_str(
            r#"
version = 2
inherits = "default"
client_type = "YoStarEN"
"#,
        )
        .unwrap();

        assert_eq!(profile.inherits.as_deref(), Some("default"));
        assert_eq!(profile.client_type, Some(ClientType::YoStarEN));
        assert_eq!(profile.connection, None);
    }

    #[test]
    fn child_overrides_client_type() {
        let parent = inherited_profile(
            "base",
            Some(ClientType::Official),
            Some(general_connection(
                Some("parent-address"),
                Some(TouchMode::MaaTouch),
            )),
        );
        let child = inherited_profile(
            "child-base",
            Some(ClientType::YoStarEN),
            Some(general_connection(None, None)),
        );

        assert_eq!(parent.merge(child).client_type, Some(ClientType::YoStarEN));
    }

    #[test]
    fn child_inherits_client_type() {
        let parent = inherited_profile(
            "base",
            Some(ClientType::Official),
            Some(general_connection(
                Some("parent-address"),
                Some(TouchMode::MaaTouch),
            )),
        );
        let child = inherited_profile("base", None, Some(general_connection(None, None)));

        assert_eq!(parent.merge(child).client_type, Some(ClientType::Official));
    }

    #[test]
    fn inherits_set_to_none() {
        let parent = inherited_profile(
            "base",
            Some(ClientType::Official),
            Some(general_connection(
                Some("parent-address"),
                Some(TouchMode::MaaTouch),
            )),
        );
        let child = inherited_profile("derived", None, Some(general_connection(None, None)));

        assert_eq!(parent.merge(child).inherits, None);
    }

    #[test]
    fn full_merge() {
        let parent = ProfileConfig {
            inherits: Some("base".into()),
            client_type: Some(ClientType::Official),
            connection: Some(ConnectionConfig::General(GeneralConnectionConfig {
                address: Some("parent-address".into()),
                adb_path: Some("parent-adb".into()),
                touch_mode: Some(TouchMode::Adb),
                adb_lite: Some(false),
                kill_adb_on_exit: Some(false),
                config: Some("parent-config".into()),
            })),
            behavior: BehaviorConfig {
                auto_reconnect: Some(true),
                deployment_with_pause: Some(false),
            },
            advanced: AdvancedConfig {
                inference_engine: Some("cpu".into()),
                user_resource: Some(false),
            },
        };
        let child = ProfileConfig {
            inherits: Some("base".into()),
            client_type: Some(ClientType::YoStarEN),
            connection: Some(ConnectionConfig::General(GeneralConnectionConfig {
                adb_path: Some("child-adb".into()),
                touch_mode: Some(TouchMode::MaaTouch),
                adb_lite: Some(true),
                ..Default::default()
            })),
            behavior: BehaviorConfig {
                auto_reconnect: None,
                deployment_with_pause: Some(true),
            },
            advanced: AdvancedConfig {
                inference_engine: Some("gpu:0".into()),
                user_resource: None,
            },
        };

        assert_eq!(parent.merge(child), ProfileConfig {
            inherits: None,
            client_type: Some(ClientType::YoStarEN),
            connection: Some(ConnectionConfig::General(GeneralConnectionConfig {
                address: Some("parent-address".into()),
                adb_path: Some("child-adb".into()),
                touch_mode: Some(TouchMode::MaaTouch),
                adb_lite: Some(true),
                kill_adb_on_exit: Some(false),
                config: Some("parent-config".into()),
            })),
            behavior: BehaviorConfig {
                auto_reconnect: Some(true),
                deployment_with_pause: Some(true),
            },
            advanced: AdvancedConfig {
                inference_engine: Some("gpu:0".into()),
                user_resource: Some(false),
            },
        });
    }

    #[test]
    fn realistic_inherit_scenario() {
        let parent = profile(Some(ConnectionConfig::General(GeneralConnectionConfig {
            address: Some("emulator-5554".into()),
            touch_mode: Some(TouchMode::MaaTouch),
            ..Default::default()
        })));
        let child = inherited_profile(
            "default",
            Some(ClientType::YoStarEN),
            Some(ConnectionConfig::General(GeneralConnectionConfig::default())),
        );

        assert_eq!(parent.merge(child), ProfileConfig {
            inherits: None,
            client_type: Some(ClientType::YoStarEN),
            connection: Some(ConnectionConfig::General(GeneralConnectionConfig {
                address: Some("emulator-5554".into()),
                touch_mode: Some(TouchMode::MaaTouch),
                ..Default::default()
            })),
            behavior: BehaviorConfig::default(),
            advanced: AdvancedConfig::default(),
        });
    }

    #[test]
    fn child_without_connection_inherits_parent_connection() {
        let parent = profile(Some(general_connection(
            Some("parent-address"),
            Some(TouchMode::MaaTouch),
        )));
        let child = inherited_profile("default", Some(ClientType::YoStarEN), None);

        assert_eq!(
            parent.merge(child).connection,
            Some(general_connection(
                Some("parent-address"),
                Some(TouchMode::MaaTouch),
            ))
        );
    }

    #[test]
    fn validate_requires_connection_after_merge() {
        let profile = inherited_profile("default", Some(ClientType::Official), None);

        assert_eq!(
            profile.validate(),
            Err(crate::ValidationError::MissingConnection)
        );
    }

    #[test]
    fn validate_avd_sdk_path() {
        for (sdk_path, expected) in [
            // `None` is valid in config — upper layers resolve it later.
            (None, Ok(())),
            (Some("/opt/android-sdk"), Ok(())),
            (Some(""), Err(crate::ValidationError::EmptyAvdSdkPath)),
            (Some("  \t  "), Err(crate::ValidationError::EmptyAvdSdkPath)),
        ] {
            assert_eq!(avd_profile(sdk_path).validate(), expected);
        }
    }

    #[test]
    fn resolve_success_general() {
        let profile = ProfileConfig {
            client_type: Some(ClientType::Official),
            connection: Some(general_connection(
                Some("emulator-5554"),
                Some(TouchMode::MaaTouch),
            )),
            behavior: BehaviorConfig {
                auto_reconnect: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };

        let resolved = profile.clone().resolve().unwrap();
        assert_eq!(resolved.client_type, Some(ClientType::Official));
        assert_eq!(resolved.behavior.auto_reconnect, Some(true));
        assert!(matches!(resolved.connection, ConnectionConfig::General(..)));
    }

    #[test]
    fn resolve_missing_connection_error() {
        let profile = ProfileConfig {
            connection: None,
            ..Default::default()
        };

        assert_eq!(
            profile.resolve(),
            Err(crate::ValidationError::MissingConnection)
        );
    }

    #[test]
    fn resolve_avd_empty_sdk_path_error() {
        let profile = avd_profile(Some(""));
        assert_eq!(
            profile.resolve(),
            Err(crate::ValidationError::EmptyAvdSdkPath)
        );
    }

    #[test]
    fn resolve_avd_whitespace_sdk_path_error() {
        let profile = avd_profile(Some("  \t  "));
        assert_eq!(
            profile.resolve(),
            Err(crate::ValidationError::EmptyAvdSdkPath)
        );
    }

    #[test]
    fn resolve_avd_none_sdk_path_ok() {
        // AVD with None sdk_path is valid for resolution (checked at usage time)
        let profile = avd_profile(None);
        let resolved = profile.resolve().unwrap();
        assert!(matches!(resolved.connection, ConnectionConfig::AVD(..)));
    }

    #[test]
    fn try_from_profile_success() {
        let profile = ProfileConfig {
            client_type: Some(ClientType::YoStarEN),
            connection: Some(general_connection(Some("127.0.0.1:5555"), None)),
            ..Default::default()
        };

        let resolved: ResolvedProfileConfig = profile.try_into().unwrap();
        assert_eq!(resolved.client_type, Some(ClientType::YoStarEN));
    }

    #[test]
    fn try_from_profile_missing_connection() {
        let profile = ProfileConfig::default();
        let result: Result<ResolvedProfileConfig, _> = profile.try_into();
        assert_eq!(result, Err(crate::ValidationError::MissingConnection));
    }

    #[test]
    fn resolved_profile_no_inherits_field() {
        // Ensure ResolvedProfileConfig doesn't have an inherits field
        // by checking the struct definition matches expected fields
        let resolved = ResolvedProfileConfig {
            client_type: None,
            connection: general_connection(None, None),
            behavior: BehaviorConfig::default(),
            advanced: AdvancedConfig::default(),
        };

        // The fact this compiles proves there's no `inherits` field required
        assert!(resolved.client_type.is_none());
    }
}
