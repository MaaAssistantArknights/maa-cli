use maa_cli_config::{ClientType, ConnectionConfig, ProfileConfig};

#[test]
fn deserialize_profile_from_toml_fixture() {
    let profile: ProfileConfig =
        toml::from_str(include_str!("../fixtures/profile/general.toml")).unwrap();

    assert_eq!(profile.client_type, Some(ClientType::Official));
    assert_eq!(profile.inherits.as_deref(), Some("base"));
    assert!(matches!(
        profile.connection,
        ConnectionConfig::General { .. }
    ));
}

#[test]
fn deserialize_profile_from_yaml_fixture() {
    let profile: ProfileConfig =
        serde_yaml::from_str(include_str!("../fixtures/profile/playcover.yaml")).unwrap();

    assert!(matches!(
        profile.connection,
        ConnectionConfig::PlayCover { .. }
    ));
}

#[test]
fn deserialize_profile_from_json_fixture() {
    let profile: ProfileConfig =
        serde_json::from_str(include_str!("../fixtures/profile/mumupro.json")).unwrap();

    assert_eq!(profile.client_type, Some(ClientType::Official));
    assert!(matches!(
        profile.connection,
        ConnectionConfig::MuMuPro { .. }
    ));
}

#[test]
fn deserialize_profile_from_waydroid_yaml_fixture() {
    let profile: ProfileConfig =
        serde_yaml::from_str(include_str!("../fixtures/profile/waydroid.yaml")).unwrap();

    assert!(matches!(
        profile.connection,
        ConnectionConfig::Waydroid { .. }
    ));
}

#[test]
fn deserialize_profile_from_avd_toml_fixture() {
    let profile: ProfileConfig =
        toml::from_str(include_str!("../fixtures/profile/avd.toml")).unwrap();

    assert_eq!(profile.client_type, Some(ClientType::YoStarEN));
    assert!(matches!(profile.connection, ConnectionConfig::AVD { .. }));
}

#[test]
fn deserialize_profile_with_inherits_fixture() {
    let profile: ProfileConfig =
        serde_yaml::from_str(include_str!("../fixtures/profile/inherit.yaml")).unwrap();

    assert_eq!(profile.inherits.as_deref(), Some("cn-android"));
    assert_eq!(profile.client_type, Some(ClientType::Official));
    assert!(matches!(
        profile.connection,
        ConnectionConfig::General { .. }
    ));
}
