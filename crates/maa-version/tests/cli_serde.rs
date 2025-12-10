use maa_version::cli::*;

#[test]
fn deserialize_from_json() {
    let json = r#"{
        "tag": "v0.1.0",
        "commit": "abc123def456",
        "assets": {
            "x86_64-apple-darwin": {
                "name": "maa_cli-0.1.0-x86_64-apple-darwin.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "x86_64-unknown-linux-gnu": {
                "name": "maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip",
                "size": 654321,
                "sha256sum": "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
            }
        }
    }"#;

    let details: Details = serde_json::from_str(json).unwrap();

    assert_eq!(details.tag, "v0.1.0");
    assert_eq!(details.commit, "abc123def456");
    assert_eq!(details.assets.len(), 2);

    let darwin_asset = &details.assets["x86_64-apple-darwin"];
    assert_eq!(darwin_asset.name, "maa_cli-0.1.0-x86_64-apple-darwin.zip");
    assert_eq!(darwin_asset.size, 123456);
    assert_eq!(
        darwin_asset.sha256sum,
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
    );

    let linux_asset = &details.assets["x86_64-unknown-linux-gnu"];
    assert_eq!(
        linux_asset.name,
        "maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip"
    );
    assert_eq!(linux_asset.size, 654321);
    assert_eq!(
        linux_asset.sha256sum,
        "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
    );

    // Test asset not found
    assert!(!details.assets.contains_key("nonexistent-platform"));
}

#[test]
fn deserialize_real_maa_cli_version_json() {
    // Real stable.json from version/stable.json
    let json = include_str!("../fixtures/cli_version.json");

    let version_json: maa_version::VersionManifest<Details> = serde_json::from_str(json).unwrap();

    // Test version
    assert_eq!(
        version_json.version,
        semver::Version::parse("0.5.9").unwrap()
    );

    // Test details
    let details = version_json.details;
    assert_eq!(details.tag, "v0.5.9");
    assert_eq!(details.commit, "f4e2418415b5cbf10d1d8e01514971c72f58cb50");
    assert_eq!(details.assets.len(), 7);

    // Test universal-apple-darwin
    let asset = &details.assets["universal-apple-darwin"];
    assert_eq!(asset.name, "maa_cli-v0.5.9-universal-apple-darwin.zip");
    assert_eq!(asset.size, 8692204);
    assert_eq!(
        asset.sha256sum,
        "a0a2aee6e01d2c60dc1be6295c3ba4eb7aeeecdd03e27072e15afdb5c8f69453"
    );

    // Test x86_64-apple-darwin
    let asset = &details.assets["x86_64-apple-darwin"];
    assert_eq!(asset.name, "maa_cli-v0.5.9-x86_64-apple-darwin.zip");
    assert_eq!(asset.size, 4290539);
    assert_eq!(
        asset.sha256sum,
        "4f77b84ef54db52373e420409e58b6300dc0b4b7babeb839675932b0e32bcb5b"
    );

    // Test x86_64-unknown-linux-gnu
    let asset = &details.assets["x86_64-unknown-linux-gnu"];
    assert_eq!(asset.name, "maa_cli-v0.5.9-x86_64-unknown-linux-gnu.tar.gz");
    assert_eq!(asset.size, 5121236);
    assert_eq!(
        asset.sha256sum,
        "f7bf07df03275b64018d789aabaa2628d062f9a6e56b7770589c6c6c1363f3b7"
    );

    // Test aarch64-unknown-linux-gnu
    let asset = &details.assets["aarch64-unknown-linux-gnu"];
    assert_eq!(
        asset.name,
        "maa_cli-v0.5.9-aarch64-unknown-linux-gnu.tar.gz"
    );
    assert_eq!(asset.size, 5301507);
    assert_eq!(
        asset.sha256sum,
        "6080419c2b3e09539bdabb04b0e7bcd5ee7fb93abd4e53cbddf87229285b7881"
    );

    // Test x86_64-pc-windows-msvc
    let asset = &details.assets["x86_64-pc-windows-msvc"];
    assert_eq!(asset.name, "maa_cli-v0.5.9-x86_64-pc-windows-msvc.zip");
    assert_eq!(asset.size, 3215593);
    assert_eq!(
        asset.sha256sum,
        "df1be3fbe297988f4fb27d1253c650e09beb0b1b330ce587a0bf7e5f7903fbad"
    );

    // Test aarch64-pc-windows-msvc
    let asset = &details.assets["aarch64-pc-windows-msvc"];
    assert_eq!(asset.name, "maa_cli-v0.5.9-aarch64-pc-windows-msvc.zip");
    assert_eq!(asset.size, 3006906);
    assert_eq!(
        asset.sha256sum,
        "0839ec03b0baff11142a9653af3dcbc58a4f4b28b9071e55f9f4d2cf9e7eac45"
    );

    // Test missing platform
    assert!(!details.assets.contains_key("unknown-platform"));
}
