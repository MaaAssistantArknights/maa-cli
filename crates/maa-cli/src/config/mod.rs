use std::{
    fs::{self, File},
    path::Path,
};

use anyhow::{Context, Result, bail};
use serde_json::Value as JsonValue;

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

    fn write<T>(&self, mut writer: impl std::io::Write, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        use Filetype::*;
        match self {
            Json => serde_json::to_writer_pretty(writer, value)?,
            Yaml => serde_yaml::to_writer(writer, value)?,
            Toml => writer.write_all(toml::to_string_pretty(value)?.as_bytes())?,
        };
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

    let value = JsonValue::from_file(file)?;

    if let Some(format) = ft {
        if let Some(file) = out {
            let file = file.with_extension(format.to_str());
            if let Some(dir) = file.parent() {
                dir.ensure()?;
            }
            format.write(File::create(file)?, &value)
        } else {
            format.write(std::io::stdout().lock(), &value)
        }
    } else {
        bail!("Format not given")
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
    use std::env::temp_dir;

    use serde::Deserialize;
    use serde_json::{Value as JsonValue, json};

    use super::*;
    use crate::assert_matches;

    #[test]
    fn filetype() {
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

        let test_root = temp_dir().join("maa-test-filetype");
        std::fs::create_dir_all(&test_root).unwrap();

        let value = json!({
            "a": 1,
            "b": "test"
        });

        let test_file = test_root.join("test");
        let test_json = test_file.with_extension("json");
        Json.write(File::create(&test_json).unwrap(), &value)
            .unwrap();
        assert_eq!(Json.read::<JsonValue>(&test_json).unwrap(), value);

        let test_yaml = test_file.with_extension("yaml");
        Yaml.write(File::create(&test_yaml).unwrap(), &value)
            .unwrap();
        assert_eq!(Yaml.read::<JsonValue>(&test_yaml).unwrap(), value);

        let test_toml = test_file.with_extension("toml");
        Toml.write(File::create(&test_toml).unwrap(), &value)
            .unwrap();
        assert_eq!(Toml.read::<JsonValue>(&test_toml).unwrap(), value);
        std::fs::remove_dir_all(&test_root).unwrap();
    }

    #[test]
    fn find_file() {
        #[derive(Deserialize, PartialEq, Debug, Default)]
        struct TestConfig {
            a: i32,
            b: String,
        }

        let test_root = temp_dir().join("find_file");
        std::fs::create_dir_all(&test_root).unwrap();

        let test_file = test_root.join("test");
        let non_exist_file = test_root.join("not_exist");

        std::fs::write(
            test_file.with_extension("json"),
            r#"{
                "a": 1,
                "b": "test"
            }"#,
        )
        .unwrap();

        assert!(
            TestConfig::find_file_or_none(&non_exist_file)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            TestConfig::find_file_or_none(&test_file).unwrap().unwrap(),
            TestConfig {
                a: 1,
                b: "test".to_string()
            }
        );

        assert_eq!(TestConfig::find_file(&test_file).unwrap(), TestConfig {
            a: 1,
            b: "test".to_string()
        });

        // Test that find_file fails when file doesn't exist
        let err = TestConfig::find_file(&non_exist_file).unwrap_err();
        let io_err = err.downcast_ref::<std::io::Error>().unwrap();
        assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);

        assert_eq!(
            TestConfig::find_file_or_default(&test_file).unwrap(),
            TestConfig {
                a: 1,
                b: "test".to_string()
            }
        );

        assert_eq!(
            TestConfig::find_file_or_default(&non_exist_file).unwrap(),
            TestConfig::default()
        );

        std::fs::remove_dir_all(&test_root).unwrap();
    }

    #[test]
    fn test_convert() {
        use Filetype::*;

        let test_root = temp_dir().join("maa-test-convert");
        std::fs::create_dir_all(&test_root).unwrap();

        let input = test_root.join("test.json");
        let toml = test_root.join("test.toml");
        let yaml = test_root.join("test.yaml");

        let value = json!({
            "a": 1,
            "b": "test"
        });

        Json.write(File::create(&input).unwrap(), &value).unwrap();

        convert(&input, None, Some(Json)).unwrap();
        convert(&input, Some(&toml), None).unwrap();
        convert(&input, Some(&toml), Some(Yaml)).unwrap();

        assert_eq!(Toml.read::<JsonValue>(&toml).unwrap(), value);
        assert_eq!(Yaml.read::<JsonValue>(&yaml).unwrap(), value);

        // Test that convert fails when format is not given
        assert!(convert(&input, None, None).is_err());
    }
}
