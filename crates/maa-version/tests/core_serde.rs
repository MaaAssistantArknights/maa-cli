use maa_version::core::Details;

#[test]
fn deserialize_from_json() {
    let json = r#"{
        "assets": [
            {
                "name": "MAA-v4.26.1-linux-x86_64.tar.gz",
                "size": 155241185,
                "browser_download_url": "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz",
                "mirrors": [
                    "https://mirror1.example.com/MAA-v4.26.1-linux-x86_64.tar.gz",
                    "https://mirror2.example.com/MAA-v4.26.1-linux-x86_64.tar.gz"
                ]
            },
            {
                "name": "MAA-v4.26.1-win-x64.zip",
                "size": 150092421,
                "browser_download_url": "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-x64.zip",
                "mirrors": [
                    "https://mirror1.example.com/MAA-v4.26.1-win-x64.zip"
                ]
            }
        ]
    }"#;

    let details: Details = serde_json::from_str(json).unwrap();

    assert_eq!(details.assets.len(), 2);

    let linux_asset = details
        .assets
        .iter()
        .find(|a| a.name.contains("linux-x86_64"))
        .unwrap();
    assert_eq!(linux_asset.name, "MAA-v4.26.1-linux-x86_64.tar.gz");
    assert_eq!(linux_asset.size, 155241185);
    assert_eq!(
        linux_asset.browser_download_url,
        "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz"
    );
    assert_eq!(linux_asset.mirrors.len(), 2);

    let win_asset = details
        .assets
        .iter()
        .find(|a| a.name.contains("win-x64"))
        .unwrap();
    assert_eq!(win_asset.name, "MAA-v4.26.1-win-x64.zip");
    assert_eq!(win_asset.size, 150092421);
    assert_eq!(win_asset.mirrors.len(), 1);

    // Test asset not found
    assert!(
        !details
            .assets
            .iter()
            .any(|a| a.name == "nonexistent-file.zip")
    );
}

#[test]
fn deserialize_real_maa_core_version_json() {
    // This is a stripped version of the real MaaCore version JSON
    let json_str = include_str!("../fixtures/core_version.json");

    let version_json: maa_version::VersionManifest<Details> =
        serde_json::from_str(json_str).expect("Failed to parse json");

    // Test version
    assert_eq!(
        version_json.version,
        semver::Version::parse("4.26.1").unwrap()
    );

    // Test details and assets
    let details = &version_json.details;
    assert_eq!(details.assets.len(), 5);

    // Test each platform asset
    let linux_x64 = details
        .assets
        .iter()
        .find(|a| a.name.contains("linux-x86_64"))
        .unwrap();
    assert_eq!(linux_x64.size, 155241185);
    assert_eq!(linux_x64.mirrors.len(), 3);

    let linux_aarch64 = details
        .assets
        .iter()
        .find(|a| a.name.contains("linux-aarch64"))
        .unwrap();
    assert_eq!(linux_aarch64.size, 152067668);
    assert_eq!(linux_aarch64.mirrors.len(), 3);

    let win_x64 = details
        .assets
        .iter()
        .find(|a| a.name.contains("win-x64"))
        .unwrap();
    assert_eq!(win_x64.size, 150092421);
    assert_eq!(win_x64.mirrors.len(), 3);

    let win_arm64 = details
        .assets
        .iter()
        .find(|a| a.name.contains("win-arm64"))
        .unwrap();
    assert_eq!(win_arm64.size, 148806502);
    assert_eq!(win_arm64.mirrors.len(), 3);

    let macos = details
        .assets
        .iter()
        .find(|a| a.name.contains("macos-runtime-universal"))
        .unwrap();
    assert_eq!(macos.size, 164012486);
    assert_eq!(macos.mirrors.len(), 3);
}
