#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
//! A simple crate to provide serde struct of version api json for both maa-cli and MaaCore.

use semver::Version;
use serde::{Deserialize, Serialize};

/// Common used version manifest struct for both CLI and MaaCore.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct VersionManifest<D> {
    #[serde(deserialize_with = "deserialize_version")]
    pub version: Version,
    pub details: D,
}

fn deserialize_version<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Version, D::Error> {
    use serde::de::Error;
    let s = String::deserialize(de)?;
    let s = s.as_str();
    let s = s.strip_prefix('v').unwrap_or(s);
    Version::parse(s).map_err(D::Error::custom)
}

pub mod cli {
    pub use std::collections::BTreeMap as Map;

    use super::*;

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Details {
        pub tag: String,
        pub commit: String,
        pub assets: Map<String, Asset>,
    }

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Asset {
        pub name: String,
        pub size: u64,
        pub sha256sum: String,
    }
}

pub mod core {
    use super::*;

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Details {
        pub assets: Vec<Asset>,
    }

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Asset {
        pub name: String,
        pub size: u64,
        pub browser_download_url: String,
        pub mirrors: Vec<String>,
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_test::{Token, assert_tokens};

    use super::*;

    mod manifest {
        use super::*;

        #[test]
        fn with_unit_details() {
            let version_info = VersionManifest {
                version: Version::parse("1.2.3").unwrap(),
                details: (),
            };

            assert_tokens(&version_info, &[
                Token::Struct {
                    name: "VersionManifest",
                    len: 2,
                },
                Token::Str("version"),
                Token::BorrowedStr("1.2.3"),
                Token::Str("details"),
                Token::Unit,
                Token::StructEnd,
            ]);
        }

        #[test]
        fn with_string_details() {
            let version_info = VersionManifest {
                version: Version::parse("2.0.0-beta.1").unwrap(),
                details: "test details",
            };

            assert_tokens(&version_info, &[
                Token::Struct {
                    name: "VersionManifest",
                    len: 2,
                },
                Token::Str("version"),
                Token::BorrowedStr("2.0.0-beta.1"),
                Token::Str("details"),
                Token::BorrowedStr("test details"),
                Token::StructEnd,
            ]);
        }

        #[test]
        fn serialize_deserialize_roundtrip() {
            let original = VersionManifest {
                version: Version::parse("1.2.3-alpha.1+sha.123").unwrap(),
                details: "test",
            };

            let serialized = serde_json::to_string(&original).unwrap();
            let deserialized: VersionManifest<&str> = serde_json::from_str(&serialized).unwrap();

            assert_eq!(original, deserialized);
        }

        #[test]
        fn deserialize_version_with_v_prefix() {
            let version_info: VersionManifest<()> =
                serde_json::from_str(r#"{"version": "v1.2.3", "details": null}"#).unwrap();

            assert_eq!(version_info.version, Version::parse("1.2.3").unwrap());
        }

        #[test]
        fn deserialize_invalid_version() {
            let result: Result<VersionManifest<()>, _> =
                serde_json::from_str(r#"{"version": "invalid", "details": null}"#);

            assert!(result.is_err());
        }
    }

    mod cli {
        use super::*;
        use crate::cli::*;

        #[test]
        fn asset() {
            let asset = Asset {
                name: "maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip".to_string(),
                size: 123456,
                sha256sum: "abcdef1234567890".to_string(),
            };

            assert_tokens(&asset, &[
                Token::Struct {
                    name: "Asset",
                    len: 3,
                },
                Token::Str("name"),
                Token::BorrowedStr("maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip"),
                Token::Str("size"),
                Token::U64(123456),
                Token::Str("sha256sum"),
                Token::BorrowedStr("abcdef1234567890"),
                Token::StructEnd,
            ]);
        }

        #[test]
        fn details() {
            let mut assets = Map::new();
            assets.insert("x86_64-unknown-linux-gnu".to_string(), Asset {
                name: "maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip".to_string(),
                size: 123456,
                sha256sum: "abcdef1234567890".to_string(),
            });

            let details = Details {
                tag: "v0.1.0".to_string(),
                commit: "abc123".to_string(),
                assets,
            };

            assert_tokens(&details, &[
                Token::Struct {
                    name: "Details",
                    len: 3,
                },
                Token::Str("tag"),
                Token::BorrowedStr("v0.1.0"),
                Token::Str("commit"),
                Token::BorrowedStr("abc123"),
                Token::Str("assets"),
                Token::Map { len: Some(1) },
                Token::BorrowedStr("x86_64-unknown-linux-gnu"),
                Token::Struct {
                    name: "Asset",
                    len: 3,
                },
                Token::Str("name"),
                Token::BorrowedStr("maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip"),
                Token::Str("size"),
                Token::U64(123456),
                Token::Str("sha256sum"),
                Token::BorrowedStr("abcdef1234567890"),
                Token::StructEnd,
                Token::MapEnd,
                Token::StructEnd,
            ]);
        }

        #[test]
        fn detail_round_trip() {
            let mut assets = Map::new();
            assets.insert("x86_64-unknown-linux-gnu".to_string(), Asset {
                name: "maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip".to_string(),
                size: 123456,
                sha256sum: "abcdef".to_string(),
            });

            let details = Details {
                tag: "v0.1.0".to_string(),
                commit: "abc123".to_string(),
                assets,
            };

            let json = serde_json::to_string(&details).unwrap();
            let deserialized: Details = serde_json::from_str(&json).unwrap();

            assert_eq!(details, deserialized);
        }
    }

    mod core {
        use super::*;
        use crate::core::*;

        #[test]
        fn asset() {
            let asset = Asset {
                name: "MAA-v4.26.1-linux-x86_64.tar.gz".to_string(),
                size: 155241185,
                browser_download_url: "https://github.com/example/file.tar.gz".to_string(),
                mirrors: vec![
                    "https://mirror1.example.com/file.tar.gz".to_string(),
                    "https://mirror2.example.com/file.tar.gz".to_string(),
                ],
            };

            assert_tokens(&asset, &[
                Token::Struct {
                    name: "Asset",
                    len: 4,
                },
                Token::Str("name"),
                Token::BorrowedStr("MAA-v4.26.1-linux-x86_64.tar.gz"),
                Token::Str("size"),
                Token::U64(155241185),
                Token::Str("browser_download_url"),
                Token::BorrowedStr("https://github.com/example/file.tar.gz"),
                Token::Str("mirrors"),
                Token::Seq { len: Some(2) },
                Token::BorrowedStr("https://mirror1.example.com/file.tar.gz"),
                Token::BorrowedStr("https://mirror2.example.com/file.tar.gz"),
                Token::SeqEnd,
                Token::StructEnd,
            ]);
        }

        #[test]
        fn details() {
            let assets = vec![
                Asset {
                    name: "MAA-v4.26.1-linux-x86_64.tar.gz".to_string(),
                    size: 155241185,
                    browser_download_url: "https://github.com/example/linux.tar.gz".to_string(),
                    mirrors: vec!["https://mirror.example.com/linux.tar.gz".to_string()],
                },
                Asset {
                    name: "MAA-v4.26.1-win-x64.zip".to_string(),
                    size: 150092421,
                    browser_download_url: "https://github.com/example/win.zip".to_string(),
                    mirrors: vec!["https://mirror.example.com/win.zip".to_string()],
                },
            ];

            let details = Details { assets };

            assert_tokens(&details, &[
                Token::Struct {
                    name: "Details",
                    len: 1,
                },
                Token::Str("assets"),
                Token::Seq { len: Some(2) },
                Token::Struct {
                    name: "Asset",
                    len: 4,
                },
                Token::Str("name"),
                Token::BorrowedStr("MAA-v4.26.1-linux-x86_64.tar.gz"),
                Token::Str("size"),
                Token::U64(155241185),
                Token::Str("browser_download_url"),
                Token::BorrowedStr("https://github.com/example/linux.tar.gz"),
                Token::Str("mirrors"),
                Token::Seq { len: Some(1) },
                Token::BorrowedStr("https://mirror.example.com/linux.tar.gz"),
                Token::SeqEnd,
                Token::StructEnd,
                Token::Struct {
                    name: "Asset",
                    len: 4,
                },
                Token::Str("name"),
                Token::BorrowedStr("MAA-v4.26.1-win-x64.zip"),
                Token::Str("size"),
                Token::U64(150092421),
                Token::Str("browser_download_url"),
                Token::BorrowedStr("https://github.com/example/win.zip"),
                Token::Str("mirrors"),
                Token::Seq { len: Some(1) },
                Token::BorrowedStr("https://mirror.example.com/win.zip"),
                Token::SeqEnd,
                Token::StructEnd,
                Token::SeqEnd,
                Token::StructEnd,
            ]);
        }

        #[test]
        fn round_trip() {
            let details = Details {
                assets: vec![Asset {
                    name: "test.zip".to_string(),
                    size: 123456,
                    browser_download_url: "https://example.com/test.zip".to_string(),
                    mirrors: vec!["https://mirror.example.com/test.zip".to_string()],
                }],
            };

            let json = serde_json::to_string(&details).unwrap();
            let deserialized: Details = serde_json::from_str(&json).unwrap();

            assert_eq!(details, deserialized);
        }
    }
}
