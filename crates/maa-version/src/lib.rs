#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
//! A simple crate to provide serde struct of version api json for both maa-cli and MaaCore.

use semver::Version;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, MapAccess, Visitor},
};

/// Common used version manifest struct for both CLI and MaaCore.
#[derive(Debug, PartialEq)]
pub struct VersionManifest<D> {
    pub version: Version,
    pub details: D,
}

impl<D: Serialize> Serialize for VersionManifest<D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("VersionManifest", 2)?;
        state.serialize_field("version", &self.version.to_string())?;
        state.serialize_field("details", &self.details)?;
        state.end()
    }
}

impl<'de, D: Deserialize<'de>> Deserialize<'de> for VersionManifest<D> {
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: Deserializer<'de>,
    {
        enum Field {
            Version,
            Details,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`version` or `details`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "version" => Ok(Field::Version),
                            "details" => Ok(Field::Details),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct VersionManifestVisitor<D> {
            marker: std::marker::PhantomData<D>,
        }

        impl<'de, D: Deserialize<'de>> Visitor<'de> for VersionManifestVisitor<D> {
            type Value = VersionManifest<D>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct VersionManifest")
            }

            fn visit_map<V>(self, mut map: V) -> Result<VersionManifest<D>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut version = None;
                let mut details = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Version => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            let s: String = map.next_value()?;
                            let s = s.strip_prefix('v').unwrap_or(&s);
                            version = Some(Version::parse(s).map_err(de::Error::custom)?);
                        }
                        Field::Details => {
                            if details.is_some() {
                                return Err(de::Error::duplicate_field("details"));
                            }
                            details = Some(map.next_value()?);
                        }
                    }
                }

                let version = version.ok_or_else(|| de::Error::missing_field("version"))?;
                let details = details.ok_or_else(|| de::Error::missing_field("details"))?;
                Ok(VersionManifest { version, details })
            }
        }

        const FIELDS: &[&str] = &["version", "details"];
        deserializer.deserialize_struct("VersionManifest", FIELDS, VersionManifestVisitor {
            marker: std::marker::PhantomData,
        })
    }
}

pub mod cli {
    pub use std::collections::BTreeMap as Map;

    use super::*;

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone)]
    pub struct Details {
        pub tag: String,
        pub commit: String,
        pub assets: Map<String, Asset>,
    }

    impl Serialize for Details {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("Details", 3)?;
            state.serialize_field("tag", &self.tag)?;
            state.serialize_field("commit", &self.commit)?;
            state.serialize_field("assets", &self.assets)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for Details {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            enum Field {
                Tag,
                Commit,
                Assets,
            }

            impl<'de> Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(
                            &self,
                            formatter: &mut std::fmt::Formatter,
                        ) -> std::fmt::Result {
                            formatter.write_str("`tag`, `commit`, or `assets`")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                        {
                            match value {
                                "tag" => Ok(Field::Tag),
                                "commit" => Ok(Field::Commit),
                                "assets" => Ok(Field::Assets),
                                _ => Err(de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct DetailsVisitor;

            impl<'de> Visitor<'de> for DetailsVisitor {
                type Value = Details;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("struct Details")
                }

                fn visit_map<V>(self, mut map: V) -> Result<Details, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut tag = None;
                    let mut commit = None;
                    let mut assets = None;

                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Tag => {
                                if tag.is_some() {
                                    return Err(de::Error::duplicate_field("tag"));
                                }
                                tag = Some(map.next_value()?);
                            }
                            Field::Commit => {
                                if commit.is_some() {
                                    return Err(de::Error::duplicate_field("commit"));
                                }
                                commit = Some(map.next_value()?);
                            }
                            Field::Assets => {
                                if assets.is_some() {
                                    return Err(de::Error::duplicate_field("assets"));
                                }
                                assets = Some(map.next_value()?);
                            }
                        }
                    }

                    let tag = tag.ok_or_else(|| de::Error::missing_field("tag"))?;
                    let commit = commit.ok_or_else(|| de::Error::missing_field("commit"))?;
                    let assets = assets.ok_or_else(|| de::Error::missing_field("assets"))?;
                    Ok(Details {
                        tag,
                        commit,
                        assets,
                    })
                }
            }

            const FIELDS: &[&str] = &["tag", "commit", "assets"];
            deserializer.deserialize_struct("Details", FIELDS, DetailsVisitor)
        }
    }

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone)]
    pub struct Asset {
        pub name: String,
        pub size: u64,
        pub sha256sum: String,
    }

    impl Serialize for Asset {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("Asset", 3)?;
            state.serialize_field("name", &self.name)?;
            state.serialize_field("size", &self.size)?;
            state.serialize_field("sha256sum", &self.sha256sum)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for Asset {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            enum Field {
                Name,
                Size,
                Sha256sum,
            }

            impl<'de> Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(
                            &self,
                            formatter: &mut std::fmt::Formatter,
                        ) -> std::fmt::Result {
                            formatter.write_str("`name`, `size`, or `sha256sum`")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                        {
                            match value {
                                "name" => Ok(Field::Name),
                                "size" => Ok(Field::Size),
                                "sha256sum" => Ok(Field::Sha256sum),
                                _ => Err(de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct AssetVisitor;

            impl<'de> Visitor<'de> for AssetVisitor {
                type Value = Asset;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("struct Asset")
                }

                fn visit_map<V>(self, mut map: V) -> Result<Asset, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut name = None;
                    let mut size = None;
                    let mut sha256sum = None;

                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Name => {
                                if name.is_some() {
                                    return Err(de::Error::duplicate_field("name"));
                                }
                                name = Some(map.next_value()?);
                            }
                            Field::Size => {
                                if size.is_some() {
                                    return Err(de::Error::duplicate_field("size"));
                                }
                                size = Some(map.next_value()?);
                            }
                            Field::Sha256sum => {
                                if sha256sum.is_some() {
                                    return Err(de::Error::duplicate_field("sha256sum"));
                                }
                                sha256sum = Some(map.next_value()?);
                            }
                        }
                    }

                    let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                    let size = size.ok_or_else(|| de::Error::missing_field("size"))?;
                    let sha256sum =
                        sha256sum.ok_or_else(|| de::Error::missing_field("sha256sum"))?;
                    Ok(Asset {
                        name,
                        size,
                        sha256sum,
                    })
                }
            }

            const FIELDS: &[&str] = &["name", "size", "sha256sum"];
            deserializer.deserialize_struct("Asset", FIELDS, AssetVisitor)
        }
    }
}

pub mod core {
    use super::*;

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone)]
    pub struct Details {
        pub assets: Vec<Asset>,
    }

    impl Serialize for Details {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("Details", 1)?;
            state.serialize_field("assets", &self.assets)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for Details {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            enum Field {
                Assets,
            }

            impl<'de> Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(
                            &self,
                            formatter: &mut std::fmt::Formatter,
                        ) -> std::fmt::Result {
                            formatter.write_str("`assets`")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                        {
                            match value {
                                "assets" => Ok(Field::Assets),
                                _ => Err(de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct DetailsVisitor;

            impl<'de> Visitor<'de> for DetailsVisitor {
                type Value = Details;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("struct Details")
                }

                fn visit_map<V>(self, mut map: V) -> Result<Details, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut assets = None;

                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Assets => {
                                if assets.is_some() {
                                    return Err(de::Error::duplicate_field("assets"));
                                }
                                assets = Some(map.next_value()?);
                            }
                        }
                    }

                    let assets = assets.ok_or_else(|| de::Error::missing_field("assets"))?;
                    Ok(Details { assets })
                }
            }

            const FIELDS: &[&str] = &["assets"];
            deserializer.deserialize_struct("Details", FIELDS, DetailsVisitor)
        }
    }

    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[derive(Debug, Clone)]
    pub struct Asset {
        pub name: String,
        pub size: u64,
        pub browser_download_url: String,
        pub mirrors: Vec<String>,
    }

    impl Serialize for Asset {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("Asset", 4)?;
            state.serialize_field("name", &self.name)?;
            state.serialize_field("size", &self.size)?;
            state.serialize_field("browser_download_url", &self.browser_download_url)?;
            state.serialize_field("mirrors", &self.mirrors)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for Asset {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            enum Field {
                Name,
                Size,
                BrowserDownloadUrl,
                Mirrors,
            }

            impl<'de> Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(
                            &self,
                            formatter: &mut std::fmt::Formatter,
                        ) -> std::fmt::Result {
                            formatter
                                .write_str("`name`, `size`, `browser_download_url`, or `mirrors`")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                        {
                            match value {
                                "name" => Ok(Field::Name),
                                "size" => Ok(Field::Size),
                                "browser_download_url" => Ok(Field::BrowserDownloadUrl),
                                "mirrors" => Ok(Field::Mirrors),
                                _ => Err(de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct AssetVisitor;

            impl<'de> Visitor<'de> for AssetVisitor {
                type Value = Asset;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("struct Asset")
                }

                fn visit_map<V>(self, mut map: V) -> Result<Asset, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut name = None;
                    let mut size = None;
                    let mut browser_download_url = None;
                    let mut mirrors = None;

                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Name => {
                                if name.is_some() {
                                    return Err(de::Error::duplicate_field("name"));
                                }
                                name = Some(map.next_value()?);
                            }
                            Field::Size => {
                                if size.is_some() {
                                    return Err(de::Error::duplicate_field("size"));
                                }
                                size = Some(map.next_value()?);
                            }
                            Field::BrowserDownloadUrl => {
                                if browser_download_url.is_some() {
                                    return Err(de::Error::duplicate_field("browser_download_url"));
                                }
                                browser_download_url = Some(map.next_value()?);
                            }
                            Field::Mirrors => {
                                if mirrors.is_some() {
                                    return Err(de::Error::duplicate_field("mirrors"));
                                }
                                mirrors = Some(map.next_value()?);
                            }
                        }
                    }

                    let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                    let size = size.ok_or_else(|| de::Error::missing_field("size"))?;
                    let browser_download_url = browser_download_url
                        .ok_or_else(|| de::Error::missing_field("browser_download_url"))?;
                    let mirrors = mirrors.ok_or_else(|| de::Error::missing_field("mirrors"))?;
                    Ok(Asset {
                        name,
                        size,
                        browser_download_url,
                        mirrors,
                    })
                }
            }

            const FIELDS: &[&str] = &["name", "size", "browser_download_url", "mirrors"];
            deserializer.deserialize_struct("Asset", FIELDS, AssetVisitor)
        }
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

        #[test]
        fn deserialize_missing_version_field() {
            let result: Result<VersionManifest<()>, _> =
                serde_json::from_str(r#"{"details": null}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `version`")
            );
        }

        #[test]
        fn deserialize_missing_details_field() {
            let result: Result<VersionManifest<()>, _> =
                serde_json::from_str(r#"{"version": "1.2.3"}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `details`")
            );
        }

        #[test]
        fn deserialize_duplicate_version_field() {
            let result: Result<VersionManifest<()>, _> = serde_json::from_str(
                r#"{"version": "1.2.3", "version": "1.2.4", "details": null}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("duplicate field"));
        }

        #[test]
        fn deserialize_duplicate_details_field() {
            let result: Result<VersionManifest<()>, _> =
                serde_json::from_str(r#"{"version": "1.2.3", "details": null, "details": null}"#);

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("duplicate field"));
        }

        #[test]
        fn deserialize_unknown_field() {
            let result: Result<VersionManifest<()>, _> = serde_json::from_str(
                r#"{"version": "1.2.3", "details": null, "unknown": "field"}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("unknown field"));
        }

        #[test]
        fn deserialize_wrong_version_type() {
            let result: Result<VersionManifest<()>, _> =
                serde_json::from_str(r#"{"version": 123, "details": null}"#);

            assert!(result.is_err());
        }

        #[test]
        fn deserialize_empty_version_string() {
            let result: Result<VersionManifest<()>, _> =
                serde_json::from_str(r#"{"version": "", "details": null}"#);

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

        #[test]
        fn asset_deserialize_missing_name() {
            let result: Result<Asset, _> =
                serde_json::from_str(r#"{"size": 123, "sha256sum": "abc"}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `name`")
            );
        }

        #[test]
        fn asset_deserialize_missing_size() {
            let result: Result<Asset, _> =
                serde_json::from_str(r#"{"name": "test.zip", "sha256sum": "abc"}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `size`")
            );
        }

        #[test]
        fn asset_deserialize_missing_sha256sum() {
            let result: Result<Asset, _> =
                serde_json::from_str(r#"{"name": "test.zip", "size": 123}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `sha256sum`")
            );
        }

        #[test]
        fn asset_deserialize_duplicate_field() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "name": "other.zip", "size": 123, "sha256sum": "abc"}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("duplicate field"));
        }

        #[test]
        fn asset_deserialize_unknown_field() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "size": 123, "sha256sum": "abc", "unknown": "field"}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("unknown field"));
        }

        #[test]
        fn asset_deserialize_wrong_size_type() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "size": "not_a_number", "sha256sum": "abc"}"#,
            );

            assert!(result.is_err());
        }

        #[test]
        fn asset_deserialize_negative_size() {
            let result: Result<Asset, _> =
                serde_json::from_str(r#"{"name": "test.zip", "size": -123, "sha256sum": "abc"}"#);

            assert!(result.is_err());
        }

        #[test]
        fn details_deserialize_missing_tag() {
            let result: Result<Details, _> =
                serde_json::from_str(r#"{"commit": "abc123", "assets": {}}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `tag`")
            );
        }

        #[test]
        fn details_deserialize_missing_commit() {
            let result: Result<Details, _> =
                serde_json::from_str(r#"{"tag": "v1.0.0", "assets": {}}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `commit`")
            );
        }

        #[test]
        fn details_deserialize_missing_assets() {
            let result: Result<Details, _> =
                serde_json::from_str(r#"{"tag": "v1.0.0", "commit": "abc123"}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `assets`")
            );
        }

        #[test]
        fn details_deserialize_duplicate_field() {
            let result: Result<Details, _> = serde_json::from_str(
                r#"{"tag": "v1.0.0", "tag": "v2.0.0", "commit": "abc", "assets": {}}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("duplicate field"));
        }

        #[test]
        fn details_deserialize_unknown_field() {
            let result: Result<Details, _> = serde_json::from_str(
                r#"{"tag": "v1.0.0", "commit": "abc", "assets": {}, "unknown": "field"}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("unknown field"));
        }

        #[test]
        fn details_deserialize_wrong_assets_type() {
            let result: Result<Details, _> = serde_json::from_str(
                r#"{"tag": "v1.0.0", "commit": "abc", "assets": "not_a_map"}"#,
            );

            assert!(result.is_err());
        }

        #[test]
        fn details_deserialize_invalid_asset_in_map() {
            let result: Result<Details, _> = serde_json::from_str(
                r#"{"tag": "v1.0.0", "commit": "abc", "assets": {"key": {"name": "test.zip"}}}"#,
            );

            assert!(result.is_err());
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

        #[test]
        fn asset_deserialize_missing_name() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"size": 123, "browser_download_url": "https://example.com", "mirrors": []}"#,
            );

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `name`")
            );
        }

        #[test]
        fn asset_deserialize_missing_size() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "browser_download_url": "https://example.com", "mirrors": []}"#,
            );

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `size`")
            );
        }

        #[test]
        fn asset_deserialize_missing_browser_download_url() {
            let result: Result<Asset, _> =
                serde_json::from_str(r#"{"name": "test.zip", "size": 123, "mirrors": []}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `browser_download_url`")
            );
        }

        #[test]
        fn asset_deserialize_missing_mirrors() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "size": 123, "browser_download_url": "https://example.com"}"#,
            );

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `mirrors`")
            );
        }

        #[test]
        fn asset_deserialize_duplicate_field() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "name": "other.zip", "size": 123, "browser_download_url": "https://example.com", "mirrors": []}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("duplicate field"));
        }

        #[test]
        fn asset_deserialize_unknown_field() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "size": 123, "browser_download_url": "https://example.com", "mirrors": [], "unknown": "field"}"#,
            );

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("unknown field"));
        }

        #[test]
        fn asset_deserialize_wrong_size_type() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "size": "not_a_number", "browser_download_url": "https://example.com", "mirrors": []}"#,
            );

            assert!(result.is_err());
        }

        #[test]
        fn asset_deserialize_negative_size() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "size": -123, "browser_download_url": "https://example.com", "mirrors": []}"#,
            );

            assert!(result.is_err());
        }

        #[test]
        fn asset_deserialize_wrong_mirrors_type() {
            let result: Result<Asset, _> = serde_json::from_str(
                r#"{"name": "test.zip", "size": 123, "browser_download_url": "https://example.com", "mirrors": "not_an_array"}"#,
            );

            assert!(result.is_err());
        }

        #[test]
        fn details_deserialize_missing_assets() {
            let result: Result<Details, _> = serde_json::from_str(r#"{}"#);

            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("missing field `assets`")
            );
        }

        #[test]
        fn details_deserialize_duplicate_field() {
            let result: Result<Details, _> =
                serde_json::from_str(r#"{"assets": [], "assets": []}"#);

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("duplicate field"));
        }

        #[test]
        fn details_deserialize_unknown_field() {
            let result: Result<Details, _> =
                serde_json::from_str(r#"{"assets": [], "unknown": "field"}"#);

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("unknown field"));
        }

        #[test]
        fn details_deserialize_wrong_assets_type() {
            let result: Result<Details, _> = serde_json::from_str(r#"{"assets": "not_an_array"}"#);

            assert!(result.is_err());
        }

        #[test]
        fn details_deserialize_invalid_asset_in_array() {
            let result: Result<Details, _> =
                serde_json::from_str(r#"{"assets": [{"name": "test.zip"}]}"#);

            assert!(result.is_err());
        }
    }
}
