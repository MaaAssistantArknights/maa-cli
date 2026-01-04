use std::{
    fs::{self, File},
    path::Path,
};

use serde_json::Value as JsonValue;

use crate::dirs::Ensure;

#[derive(Debug)]
pub enum Error {
    UnsupportedFiletype,
    FormatNotGiven,
    Io(std::io::Error),
    Json(serde_json::Error),
    TomlDe(toml::de::Error),
    TomlSer(toml::ser::Error),
    Yaml(serde_yaml::Error),
}

type Result<T, E = Error> = std::result::Result<T, E>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::UnsupportedFiletype => write!(f, "Unsupported or unknown filetype"),
            Error::FormatNotGiven => write!(f, "Format not given"),
            Error::Io(e) => write!(f, "IO error, {e}"),
            Error::Json(e) => write!(f, "JSON parse error, {e}"),
            Error::TomlSer(e) => write!(f, "TOML serialize error, {e}"),
            Error::TomlDe(e) => write!(f, "TOML deserialize error, {e}"),
            Error::Yaml(e) => write!(f, "YAML parse error, {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::TomlDe(e)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Self {
        Error::TomlSer(e)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::Yaml(e)
    }
}

fn file_not_found(path: impl AsRef<Path>) -> Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        path.as_ref()
            .to_str()
            .map_or("File not found".to_owned(), |s| {
                format!("File not found: {s}")
            }),
    )
    .into()
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
    fn is_valid_file(path: impl AsRef<Path>) -> bool {
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
                .ok_or(Error::UnsupportedFiletype)?
                .read(path)
        } else {
            Err(file_not_found(path))
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
        Self::find_file_or_none(path)?.ok_or_else(|| file_not_found(path))
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
        Err(Error::FormatNotGiven)
    }
}

pub fn import(src: &Path, force: bool, config_type: &str) -> std::io::Result<()> {
    import_to(src, force, config_type, maa_dirs::config())
}

fn import_to(src: &Path, force: bool, config_type: &str, config_dir: &Path) -> std::io::Result<()> {
    use std::io::{Error as IOError, ErrorKind};

    if !src.is_file() {
        return Err(IOError::new(
            ErrorKind::InvalidInput,
            "Given path is not a file or not exists",
        ));
    };

    let file: &Path = src
        .file_name()
        .ok_or_else(|| IOError::new(ErrorKind::InvalidInput, "Invalid file path"))?
        .as_ref();

    // CLI configuration is unique, only one file is allowed
    if config_type == "cli" {
        // check if the file name is cli with supported extension for cli configuration
        if file
            .file_stem()
            .is_some_and(|stem| stem.to_str() == Some("cli"))
            && Filetype::is_valid_file(file)
        {
            let cli_path = config_dir.join("cli");
            if !force
                && SUPPORTED_EXTENSION
                    .iter()
                    .any(|ext| cli_path.with_extension(ext).exists())
            {
                return Err(IOError::new(
                    ErrorKind::AlreadyExists,
                    "CLI configuration file already exists, use --force to overwrite",
                ));
            }

            fs::copy(src, config_dir.join(file))?;
        } else {
            return Err(IOError::new(
                ErrorKind::InvalidInput,
                "A CLI configuration file should be named as `cli` with supported extension",
            ));
        }
    }

    let (read_by_cli, dir) = type_to_dir(config_type, config_dir);

    // check if the configuration file read by CLI is valid
    if read_by_cli && !Filetype::is_valid_file(file) {
        return Err(IOError::new(
            ErrorKind::InvalidInput,
            format!(
                "File with unsupported extension: {}, supported extensions: {}",
                file.display(),
                SUPPORTED_EXTENSION.join(", ")
            ),
        ));
    }

    let dest = dir.join(file);
    let mut tobe_removed = Vec::new();

    // Check if directory exists
    if dir.exists() {
        // Check if file with same name already exists
        if read_by_cli {
            for ext in SUPPORTED_EXTENSION.iter() {
                let path = dest.with_extension(ext);
                if path.exists() {
                    if force {
                        // Add file with same name but different extension
                        // to tobe_removed list to remove after copying
                        if path != dest {
                            tobe_removed.push(path);
                        }
                    } else {
                        return Err(IOError::new(
                            ErrorKind::AlreadyExists,
                            format!(
                                "File with same  name (`{}`) already exists, use --force to overwrite",
                                dest.display()
                            ),
                        ));
                    }
                }
            }
        } else if !force && dest.exists() {
            return Err(IOError::new(
                ErrorKind::AlreadyExists,
                format!(
                    "File {} already exists, use --force to overwrite",
                    dest.display()
                ),
            ));
        }
    } else {
        fs::create_dir_all(&dir)?;
    }

    fs::copy(src, dest)?;

    for path in tobe_removed {
        fs::remove_file(path)?;
    }

    Ok(())
}

/// Convert configuration type to directory path and whether it is a configuration read by CLI.
fn type_to_dir(config_type: &str, config_dir: &Path) -> (bool, std::path::PathBuf) {
    match config_type {
        // No need to check config_type == "cli" here, it is handled in import function
        "asst" | "profile" => (true, config_dir.join("profiles")),
        "task" => (true, config_dir.join("tasks")),
        "infrast" | "resource" | "copilot" | "ssscopilot" => (false, config_dir.join(config_type)),
        #[cfg(test)]
        "__test__" => (false, config_dir.join("__test__")),
        _ => {
            log::warn!("Unknown configuration type: {config_type}");
            (false, config_dir.join(config_type))
        }
    }
}

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

        assert_matches!(
            TestConfig::find_file(&non_exist_file).unwrap_err(),
            Error::Io(e) if e.kind() == std::io::ErrorKind::NotFound
        );

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

        assert_matches!(
            convert(&input, None, None).unwrap_err(),
            Error::FormatNotGiven
        );
    }

    mod import {
        use std::io::ErrorKind;

        use super::*;

        fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
            let tmp_dir = tempfile::tempdir().unwrap();
            let tmp_path = tmp_dir.path();
            let config_dir = tmp_path.join("config");

            std::fs::create_dir_all(&config_dir).unwrap();

            std::fs::create_dir_all(tmp_path.join("test")).unwrap();
            std::fs::write(tmp_path.join("cli.json"), "{}").unwrap();
            std::fs::write(tmp_path.join("test.json"), "{}").unwrap();
            std::fs::write(tmp_path.join("test.yml"), "").unwrap();
            std::fs::write(tmp_path.join("test.ini"), "").unwrap();

            (tmp_dir, config_dir)
        }

        #[test]
        fn cli_config_must_be_named_cli() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            assert_eq!(
                import_to(&tmp_path.join("test"), false, "cli", &config_dir)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidInput
            );
        }

        #[test]
        fn cli_config_file_must_have_stem_cli() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            assert_eq!(
                import_to(&tmp_path.join("test.json"), false, "cli", &config_dir)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidInput
            );
        }

        #[test]
        fn cli_config_import_succeeds() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            assert!(import_to(&tmp_path.join("cli.json"), false, "cli", &config_dir).is_ok());
        }

        #[test]
        fn cli_config_duplicate_fails_without_force() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("cli.json"), false, "cli", &config_dir).unwrap();
            assert_eq!(
                import_to(&tmp_path.join("cli.json"), false, "cli", &config_dir)
                    .unwrap_err()
                    .kind(),
                ErrorKind::AlreadyExists
            );
        }

        #[test]
        fn cli_config_duplicate_succeeds_with_force() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("cli.json"), false, "cli", &config_dir).unwrap();
            import_to(&tmp_path.join("cli.json"), true, "cli", &config_dir).unwrap();
        }

        #[test]
        fn task_import_succeeds() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("test.json"), false, "task", &config_dir).unwrap();
        }

        #[test]
        fn task_duplicate_name_fails_without_force() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("test.json"), false, "task", &config_dir).unwrap();
            assert_eq!(
                import_to(&tmp_path.join("test.yml"), false, "task", &config_dir)
                    .unwrap_err()
                    .kind(),
                ErrorKind::AlreadyExists
            );
        }

        #[test]
        fn task_duplicate_name_succeeds_with_force_and_removes_old_extension() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("test.json"), false, "task", &config_dir).unwrap();
            import_to(&tmp_path.join("test.yml"), true, "task", &config_dir).unwrap();

            assert!(config_dir.join("tasks").join("test.yml").exists());
            assert!(!config_dir.join("tasks").join("test.json").exists());
        }

        #[test]
        fn task_unsupported_extension_fails() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            assert_eq!(
                import_to(&tmp_path.join("test.ini"), false, "task", &config_dir)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidInput
            );
        }

        #[test]
        fn infrast_import_succeeds() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("test.json"), false, "infrast", &config_dir).unwrap();
        }

        #[test]
        fn infrast_duplicate_fails_without_force() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("test.json"), false, "infrast", &config_dir).unwrap();
            assert_eq!(
                import_to(&tmp_path.join("test.json"), false, "infrast", &config_dir)
                    .unwrap_err()
                    .kind(),
                ErrorKind::AlreadyExists
            );
        }

        #[test]
        fn infrast_duplicate_succeeds_with_force() {
            let (tmp_dir, config_dir) = setup();
            let tmp_path = tmp_dir.path();

            import_to(&tmp_path.join("test.json"), false, "infrast", &config_dir).unwrap();
            import_to(&tmp_path.join("test.json"), true, "infrast", &config_dir).unwrap();
        }

        #[test]
        #[ignore = "writes to real user config directory"]
        fn import_to_real_user_config_dir() {
            let tmp_dir = tempfile::tempdir().unwrap();
            let tmp_path = tmp_dir.path();

            std::fs::write(tmp_path.join("test.json"), "{}").unwrap();

            // Use __test__ type which maps to config_dir/__test__
            let result = import(&tmp_path.join("test.json"), false, "__test__");

            // Clean up if import succeeded
            if result.is_ok() {
                let test_dir = maa_dirs::config().join("__test__");
                if test_dir.exists() {
                    std::fs::remove_dir_all(&test_dir).unwrap();
                }
            }

            assert!(result.is_ok());
        }
    }

    mod type_to_dir_tests {
        use super::*;

        #[test]
        fn asst_maps_to_profiles() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("asst", config_dir),
                (true, config_dir.join("profiles"))
            );
        }

        #[test]
        fn profile_maps_to_profiles() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("profile", config_dir),
                (true, config_dir.join("profiles"))
            );
        }

        #[test]
        fn task_maps_to_tasks() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("task", config_dir),
                (true, config_dir.join("tasks"))
            );
        }

        #[test]
        fn infrast_maps_to_infrast() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("infrast", config_dir),
                (false, config_dir.join("infrast"))
            );
        }

        #[test]
        fn resource_maps_to_resource() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("resource", config_dir),
                (false, config_dir.join("resource"))
            );
        }

        #[test]
        fn copilot_maps_to_copilot() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("copilot", config_dir),
                (false, config_dir.join("copilot"))
            );
        }

        #[test]
        fn ssscopilot_maps_to_ssscopilot() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("ssscopilot", config_dir),
                (false, config_dir.join("ssscopilot"))
            );
        }

        #[test]
        fn unknown_type_maps_to_itself() {
            let config_dir = Path::new("/test/config");
            assert_eq!(
                type_to_dir("unknown", config_dir),
                (false, config_dir.join("unknown"))
            );
        }
    }
}
