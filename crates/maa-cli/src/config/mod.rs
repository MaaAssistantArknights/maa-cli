use std::{
    fs::{self, File},
    path::Path,
};

use anyhow::{Context, Result, bail};
use maa_value::value::MAAValue;

fn file_not_found(path: impl AsRef<Path>) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        path.as_ref()
            .to_str()
            .map_or("File not found".to_owned(), |s| {
                format!("File not found: {s}")
            }),
    )
}

const SUPPORTED_EXTENSION: [&str; 4] = ["json", "yaml", "yml", "toml"];

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Filetype {
    #[clap(alias = "j")]
    Json,
    #[clap(alias = "y")]
    Yaml,
    #[clap(alias = "t")]
    Toml,
}

impl Filetype {
    pub(crate) fn is_valid_file(path: impl AsRef<Path>) -> bool {
        Self::parse_filetype(path).is_some()
    }

    fn parse_filetype(path: impl AsRef<Path>) -> Option<Self> {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::parse_extension)
    }

    fn parse_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_ref() {
            "json" => Some(Filetype::Json),
            "yaml" | "yml" => Some(Filetype::Yaml),
            "toml" => Some(Filetype::Toml),
            _ => None,
        }
    }

    fn read<T>(&self, path: impl AsRef<Path>) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use Filetype::*;
        Ok(match self {
            Json => serde_json::from_reader(File::open(path)?)?,
            Yaml => serde_yaml::from_reader(File::open(path)?)?,
            Toml => toml::from_str(&fs::read_to_string(path)?)?,
        })
    }

    fn write<T>(&self, path: &Path, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        crate::atomic_fs::write_with(path, |temp| self.write_to(temp, value))
    }

    fn write_to<W, T>(&self, mut writer: W, value: &T) -> Result<()>
    where
        W: std::io::Write,
        T: serde::Serialize,
    {
        use Filetype::*;
        match self {
            Json => serde_json::to_writer_pretty(writer, value)?,
            Yaml => serde_yaml::to_writer(writer, value)?,
            Toml => writer.write_all(toml::to_string_pretty(value)?.as_bytes())?,
        }
        Ok(())
    }

    fn to_str(self) -> &'static str {
        use Filetype::*;
        match self {
            Json => "json",
            Yaml => "yaml",
            Toml => "toml",
        }
    }
}

pub trait FromFile: Sized + serde::de::DeserializeOwned {
    fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            Filetype::parse_filetype(path)
                .with_context(|| format!("Unsupported or unknown filetype: {}", path.display()))?
                .read(path)
        } else {
            Err(file_not_found(path).into())
        }
    }
}

impl<T> FromFile for T where T: serde::de::DeserializeOwned {}

pub trait FindFile: FromFile {
    /// Find file with supported extension and deserialize it.
    ///
    /// The file should not have extension. If it has extension, it will be ignored.
    /// If file not found, return Ok(None).
    fn find_file_or_none(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        for filetype in SUPPORTED_EXTENSION.iter() {
            let path = path.with_extension(filetype);
            if path.exists() {
                return Ok(Some(Self::from_file(&path)?));
            }
        }
        Ok(None)
    }
    /// Find file with supported extension and deserialize it.
    ///
    /// The file should not have extension. If it has extension, it will be ignored.
    /// Return error if file not found.
    fn find_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Self::find_file_or_none(path)?.ok_or_else(|| file_not_found(path).into())
    }
}

pub trait FindFileOrDefault: FromFile + Default {
    fn find_file_or_default(path: impl AsRef<Path>) -> Result<Self> {
        Self::find_file_or_none(path).map(|opt| opt.unwrap_or_default())
    }
}

impl<T> FindFile for T where T: FromFile {}

impl<T> FindFileOrDefault for T where T: FromFile + Default {}

pub fn convert(file: &Path, out: Option<&Path>, ft: Option<Filetype>) -> Result<()> {
    use maa_dirs::Ensure;

    let ft = ft.or_else(|| {
        out.and_then(|path| path.extension())
            .and_then(|ext| ext.to_str())
            .and_then(Filetype::parse_extension)
    });

    let value = MAAValue::from_file(file)?;

    let Some(format) = ft else {
        bail!("Format not given")
    };

    if let Some(file) = out {
        let file = file.with_extension(format.to_str());
        if let Some(dir) = file.parent() {
            dir.ensure()?;
        }
        format
            .write(&file, &value)
            .with_context(|| format!("Failed to write converted file {}", file.display()))
    } else {
        format.write_to(std::io::stdout().lock(), &value)
    }
}

pub mod import;

pub mod asst;

pub mod cli;

pub mod task;

pub mod init;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    mod filetype {
        use maa_value::prelude::*;

        use super::super::*;
        use crate::assert_matches;

        #[test]
        fn parse() {
            use Filetype::*;
            assert_matches!(Filetype::parse_filetype("test.toml"), Some(Toml));
            assert!(Filetype::parse_filetype("test").is_none());

            assert_matches!(Filetype::parse_extension("toml"), Some(Toml));
            assert_matches!(Filetype::parse_extension("yml"), Some(Yaml));
            assert_matches!(Filetype::parse_extension("yaml"), Some(Yaml));
            assert_matches!(Filetype::parse_extension("json"), Some(Json));
            assert!(Filetype::parse_extension("txt").is_none());

            assert_eq!(Toml.to_str(), "toml");
            assert_eq!(Yaml.to_str(), "yaml");
            assert_eq!(Json.to_str(), "json");
        }

        #[test]
        fn write() {
            use Filetype::*;

            let dir = tempfile::tempdir().unwrap();
            let test_file = dir.path().join("test");

            let value = object!("z" => 1, "a" => "test", "m" => false);

            let test_json = test_file.with_extension("json");
            Json.write(&test_json, &value).unwrap();
            assert_eq!(
                std::fs::read_to_string(&test_json).unwrap(),
                "{\n  \"z\": 1,\n  \"a\": \"test\",\n  \"m\": false\n}"
            );

            let test_yaml = test_file.with_extension("yaml");
            Yaml.write(&test_yaml, &value).unwrap();

            let test_toml = test_file.with_extension("toml");
            Toml.write(&test_toml, &value).unwrap();
            assert_eq!(
                std::fs::read_to_string(&test_toml).unwrap(),
                "z = 1\na = \"test\"\nm = false\n"
            );
        }
    }

    mod find_file {
        use serde::Deserialize;

        use super::super::*;

        #[derive(Deserialize, PartialEq, Debug, Default)]
        struct TestConfig {
            a: i32,
            b: String,
        }

        #[test]
        fn not_found() {
            let dir = tempfile::tempdir().unwrap();
            let non_exist = dir.path().join("not_exist");
            assert!(TestConfig::find_file_or_none(&non_exist).unwrap().is_none());
            let err = TestConfig::find_file(&non_exist).unwrap_err();
            let io_err = err.downcast_ref::<std::io::Error>().unwrap();
            assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
        }

        #[test]
        fn found() {
            let dir = tempfile::tempdir().unwrap();
            let test_file = dir.path().join("test");
            std::fs::write(test_file.with_extension("json"), r#"{"a": 1, "b": "test"}"#).unwrap();

            assert_eq!(TestConfig::find_file(&test_file).unwrap(), TestConfig {
                a: 1,
                b: "test".into()
            });
        }

        #[test]
        fn or_default() {
            let dir = tempfile::tempdir().unwrap();
            let test_file = dir.path().join("test");
            let non_exist = dir.path().join("not_exist");

            std::fs::write(test_file.with_extension("json"), r#"{"a": 1, "b": "test"}"#).unwrap();

            assert_eq!(
                TestConfig::find_file_or_default(&test_file).unwrap(),
                TestConfig {
                    a: 1,
                    b: "test".into()
                }
            );
            assert_eq!(
                TestConfig::find_file_or_default(&non_exist).unwrap(),
                TestConfig::default()
            );
        }
    }

    mod convert {
        use maa_value::prelude::*;

        use super::super::*;

        #[test]
        fn basic() {
            use Filetype::*;

            let dir = tempfile::tempdir().unwrap();
            let input = dir.path().join("test.json");
            let toml = dir.path().join("test.toml");
            let yaml = dir.path().join("test.yaml");

            let value = object!("z" => 1, "a" => "test", "m" => false);
            Json.write(&input, &value).unwrap();

            super::super::convert(&input, None, Some(Json)).unwrap();
            super::super::convert(&input, Some(&toml), None).unwrap();
            super::super::convert(&input, Some(&toml), Some(Yaml)).unwrap();

            assert_eq!(
                std::fs::read_to_string(&toml).unwrap(),
                "z = 1\na = \"test\"\nm = false\n"
            );
            assert_eq!(
                std::fs::read_to_string(&yaml).unwrap(),
                "z: 1\na: test\nm: false\n"
            );
        }

        #[test]
        fn no_format_is_error() {
            let dir = tempfile::tempdir().unwrap();
            let input = dir.path().join("test.json");

            let value = object!("z" => 1);
            Filetype::Json.write(&input, &value).unwrap();

            assert!(super::super::convert(&input, None, None).is_err());
        }

        #[test]
        fn preserves_key_order() {
            let dir = tempfile::tempdir().unwrap();
            let input = dir.path().join("ordered.json");
            let yaml = dir.path().join("ordered.yaml");

            let value = object!(
                "z" => 1,
                "a" => object!("k2" => 2, "k1" => 1),
                "m" => 3,
            );
            Filetype::Json.write(&input, &value).unwrap();

            let input_value = MAAValue::from_file(&input).unwrap();
            let keys: Vec<_> = input_value
                .as_map()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(keys, ["z", "a", "m"]);
            let inner_keys: Vec<_> = input_value
                .get("a")
                .unwrap()
                .as_map()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(inner_keys, ["k2", "k1"]);

            super::super::convert(&input, Some(&yaml), None).unwrap();

            assert_eq!(
                std::fs::read_to_string(&yaml).unwrap(),
                "z: 1\na:\n  k2: 2\n  k1: 1\nm: 3\n"
            );
        }
    }
}
