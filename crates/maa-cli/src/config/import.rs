use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use maa_dirs::Ensure;

use super::{Filetype, SUPPORTED_EXTENSION};
use crate::state::AGENT;

/// Represents the source of a configuration file to import
#[derive(Debug, Clone, Copy)]
enum ImportSource<'a> {
    /// A remote HTTP(S) URL
    Remote(&'a str),
    /// A local file path
    Local(&'a Path),
}

impl<'a> ImportSource<'a> {
    /// Parse a source string into an ImportSource
    fn from_str(src: &'a str) -> Self {
        let trimmed = src.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            ImportSource::Remote(trimmed)
        } else if let Some(path) = trimmed.strip_prefix("file://") {
            ImportSource::Local(Path::new(path))
        } else {
            ImportSource::Local(Path::new(trimmed))
        }
    }

    fn filename(self) -> Result<&'a str> {
        match self {
            ImportSource::Remote(url) => {
                let url_path = url.split('?').next().unwrap_or(url);
                url_path
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty())
                    .context("Cannot extract filename from URL")
            }
            ImportSource::Local(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .context("Cannot extract filename from local path"),
        }
    }

    /// Copy the source to the target path
    fn copy_to(self, target: &Path) -> Result<()> {
        match self {
            ImportSource::Remote(url) => {
                let response = AGENT.get(url).call()?;
                let mut file = fs::File::create(target)?;
                std::io::copy(&mut response.into_body().as_reader(), &mut file)?;
                Ok(())
            }
            ImportSource::Local(path) => {
                fs::copy(path, target)?;
                Ok(())
            }
        }
    }
}

/// Configuration type for import operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ConfigType {
    /// CLI configuration file
    Cli,
    /// Assistant/Profile configuration
    Profile,
    /// Task configuration
    Task,
    /// Infrastructure configuration
    Infrast,
    /// Resource configuration
    Resource,
    /// Copilot configuration
    Copilot,
    /// SSSCopilot configuration
    SSSCopilot,
    #[cfg(test)]
    /// Test configuration
    Test,
}

impl ConfigType {
    fn read_by_cli(self) -> bool {
        use ConfigType::*;
        matches!(self, Cli | Profile | Task)
    }

    fn config_dir(self, root: &Path) -> PathBuf {
        use ConfigType::*;
        match self {
            Cli => root.to_path_buf(),
            Profile => root.join("profiles"),
            Task => root.join("tasks"),
            Infrast => root.join("infrast"),
            Resource => root.join("resource"),
            Copilot => root.join("copilot"),
            SSSCopilot => root.join("ssscopilot"),
            #[cfg(test)]
            Test => root.join("__test__"),
        }
    }

    fn validate_file(self, filename: &Path) -> Result<()> {
        if matches!(self, ConfigType::Cli)
            && filename
                .file_stem()
                .is_none_or(|s| s.to_str() != Some("cli"))
        {
            bail!("A CLI configuration file should be named as `cli`.")
        }

        if self.read_by_cli() && !Filetype::is_valid_file(filename) {
            bail!(
                "File with unsupported extension: {}, supported extensions: {}",
                filename.extension().unwrap_or_default().to_string_lossy(),
                SUPPORTED_EXTENSION.join(", ")
            )
        }

        Ok(())
    }

    fn check_duplication(self, filename: &Path) -> bool {
        if self.read_by_cli() {
            SUPPORTED_EXTENSION
                .iter()
                .any(|ext| filename.with_extension(ext).exists())
        } else {
            filename.exists()
        }
    }

    fn clear_duplicate(self, filename: &Path) -> std::io::Result<()> {
        if self.read_by_cli() {
            for ext in SUPPORTED_EXTENSION {
                let path = filename.with_extension(ext);
                if path.exists() {
                    fs::remove_file(path)?;
                }
            }
        } else if filename.exists() {
            fs::remove_file(filename)?;
        }
        Ok(())
    }
}

#[derive(clap::Args)]
pub struct ImportOptions {
    /// Path or URL of the configuration file to be imported
    pub src: String,
    /// Name of the configuration file
    ///
    /// If not provided, the name of the source file will be used.
    #[arg(short, long)]
    pub name: Option<String>,
    /// Force to import even if a file with the same name already exists
    #[arg(short, long)]
    pub force: bool,
    /// Type of the configuration file
    #[arg(short = 't', long, default_value = "task")]
    pub config_type: ConfigType,
}

/// A thin shim over `import_to` that binds the default config directory.
///
/// This exists to keep the core import logic testable and free of
/// environment-specific concerns.
pub fn import(opts: ImportOptions) -> Result<()> {
    import_to(opts, maa_dirs::config())
}

fn import_to(opts: ImportOptions, dir: &Path) -> Result<()> {
    let ImportOptions {
        src,
        name,
        force,
        config_type,
    } = opts;

    // Parse the source
    let source = ImportSource::from_str(&src);

    // Determine the filename
    let filename = match name {
        Some(ref n) => n.as_str(),
        None => source.filename()?,
    };
    let file = Path::new(filename);

    // Validate the file for this config type
    config_type.validate_file(file)?;

    // Determine the target directory
    let target_dir = config_type.config_dir(dir);

    // Ensure directory exists
    target_dir.ensure()?;

    let dest = target_dir.join(file);

    // Check for duplicates
    if !force && config_type.check_duplication(&dest) {
        bail!(
            "Configuration file {} already exists, use --force to overwrite",
            dest.display()
        );
    }

    // Clear duplicates if force is enabled
    if force {
        config_type.clear_duplicate(&dest)?;
    }

    // Copy the file
    source.copy_to(&dest)?;

    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::assert_matches;

    mod config_type {
        use super::*;

        mod read_by_cli {
            use super::*;

            #[test]
            fn cli_is_read_by_cli() {
                assert!(ConfigType::Cli.read_by_cli());
            }

            #[test]
            fn profile_is_read_by_cli() {
                assert!(ConfigType::Profile.read_by_cli());
            }

            #[test]
            fn task_is_read_by_cli() {
                assert!(ConfigType::Task.read_by_cli());
            }

            #[test]
            fn infrast_is_not_read_by_cli() {
                assert!(!ConfigType::Infrast.read_by_cli());
            }

            #[test]
            fn resource_is_not_read_by_cli() {
                assert!(!ConfigType::Resource.read_by_cli());
            }

            #[test]
            fn copilot_is_not_read_by_cli() {
                assert!(!ConfigType::Copilot.read_by_cli());
            }

            #[test]
            fn ssscopilot_is_not_read_by_cli() {
                assert!(!ConfigType::SSSCopilot.read_by_cli());
            }
        }

        mod config_dir {
            use std::sync::LazyLock;

            use super::*;

            static ROOT: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/config"));

            #[test]
            fn cli_returns_root() {
                assert_eq!(ConfigType::Cli.config_dir(&ROOT), ROOT.to_path_buf());
            }

            #[test]
            fn profile_returns_profiles_subdir() {
                assert_eq!(ConfigType::Profile.config_dir(&ROOT), ROOT.join("profiles"));
            }

            #[test]
            fn task_returns_tasks_subdir() {
                assert_eq!(ConfigType::Task.config_dir(&ROOT), ROOT.join("tasks"));
            }

            #[test]
            fn infrast_returns_infrast_subdir() {
                assert_eq!(ConfigType::Infrast.config_dir(&ROOT), ROOT.join("infrast"));
            }

            #[test]
            fn resource_returns_resource_subdir() {
                assert_eq!(
                    ConfigType::Resource.config_dir(&ROOT),
                    ROOT.join("resource")
                );
            }

            #[test]
            fn copilot_returns_copilot_subdir() {
                assert_eq!(ConfigType::Copilot.config_dir(&ROOT), ROOT.join("copilot"));
            }

            #[test]
            fn ssscopilot_returns_ssscopilot_subdir() {
                assert_eq!(
                    ConfigType::SSSCopilot.config_dir(&ROOT),
                    ROOT.join("ssscopilot")
                );
            }

            #[test]
            fn test_returns_test_subdir() {
                assert_eq!(ConfigType::Test.config_dir(&ROOT), ROOT.join("__test__"));
            }
        }

        mod validate_file {
            use super::*;

            #[test]
            fn cli_accepts_cli_stem() {
                assert!(ConfigType::Cli.validate_file(Path::new("cli.json")).is_ok());
                assert!(ConfigType::Cli.validate_file(Path::new("cli.toml")).is_ok());
                assert!(ConfigType::Cli.validate_file(Path::new("cli.yml")).is_ok());
            }

            #[test]
            fn cli_rejects_non_cli_stem() {
                assert!(
                    ConfigType::Cli
                        .validate_file(Path::new("config.json"))
                        .is_err()
                );
                assert!(
                    ConfigType::Cli
                        .validate_file(Path::new("test.json"))
                        .is_err()
                );
            }

            #[test]
            fn task_accepts_valid_extensions() {
                assert!(
                    ConfigType::Task
                        .validate_file(Path::new("task.json"))
                        .is_ok()
                );
                assert!(
                    ConfigType::Task
                        .validate_file(Path::new("task.toml"))
                        .is_ok()
                );
                assert!(
                    ConfigType::Task
                        .validate_file(Path::new("task.yml"))
                        .is_ok()
                );
                assert!(
                    ConfigType::Task
                        .validate_file(Path::new("task.yaml"))
                        .is_ok()
                );
            }

            #[test]
            fn task_rejects_invalid_extensions() {
                assert!(
                    ConfigType::Task
                        .validate_file(Path::new("task.ini"))
                        .is_err()
                );
                assert!(
                    ConfigType::Task
                        .validate_file(Path::new("task.txt"))
                        .is_err()
                );
            }

            #[test]
            fn infrast_accepts_any_extension() {
                assert!(
                    ConfigType::Infrast
                        .validate_file(Path::new("plan.json"))
                        .is_ok()
                );
                assert!(
                    ConfigType::Infrast
                        .validate_file(Path::new("plan.ini"))
                        .is_ok()
                );
                assert!(
                    ConfigType::Infrast
                        .validate_file(Path::new("plan.txt"))
                        .is_ok()
                );
            }
        }

        mod check_duplication {
            use super::*;

            #[test]
            fn cli_checks_all_extensions() {
                let tmp_dir = tempfile::tempdir().unwrap();
                let file = tmp_dir.path().join("cli.json");
                fs::write(&file, "{}").unwrap();

                // Check with .toml extension but .json exists
                let check_path = tmp_dir.path().join("cli.toml");
                assert!(ConfigType::Cli.check_duplication(&check_path));
            }

            #[test]
            fn infrast_checks_exact_file() {
                let tmp_dir = tempfile::tempdir().unwrap();
                let file = tmp_dir.path().join("plan.json");
                fs::write(&file, "{}").unwrap();

                assert!(ConfigType::Infrast.check_duplication(&file));
                assert!(!ConfigType::Infrast.check_duplication(&tmp_dir.path().join("plan.toml")));
            }
        }

        mod clear_duplicate {
            use super::*;

            #[test]
            fn removes_file_and_all_extensions() {
                let tmp_dir = tempfile::tempdir().unwrap();
                let json_file = tmp_dir.path().join("test.json");
                let toml_file = tmp_dir.path().join("test.toml");
                let yml_file = tmp_dir.path().join("test.yml");

                fs::write(&json_file, "{}").unwrap();
                fs::write(&toml_file, "").unwrap();
                fs::write(&yml_file, "").unwrap();

                ConfigType::Task.clear_duplicate(&json_file).unwrap();

                assert!(!json_file.exists());
                assert!(!toml_file.exists());
                assert!(!yml_file.exists());
            }
        }
    }

    mod import_to {
        use super::*;

        fn setup() -> (tempfile::TempDir, PathBuf) {
            let tmp_dir = tempfile::tempdir().unwrap();
            let tmp_path = tmp_dir.path();
            let config_dir = tmp_path.join("config");

            fs::create_dir_all(&config_dir).unwrap();
            fs::create_dir_all(tmp_path.join("test")).unwrap();
            fs::write(tmp_path.join("cli.json"), "{}").unwrap();
            fs::write(tmp_path.join("test.json"), "{}").unwrap();
            fs::write(tmp_path.join("test.yml"), "").unwrap();
            fs::write(tmp_path.join("test.ini"), "").unwrap();

            (tmp_dir, config_dir)
        }

        mod cli {
            use super::*;

            #[test]
            fn rejects_non_cli_name() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir.path().join("test").to_str().unwrap().to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Cli,
                };
                assert!(import_to(opts, &config_dir).is_err());
            }

            #[test]
            fn rejects_wrong_stem() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Cli,
                };
                assert!(import_to(opts, &config_dir).is_err());
            }

            #[test]
            fn imports_valid_cli_config() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("cli.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Cli,
                };
                import_to(opts, &config_dir).unwrap();
                assert!(config_dir.join("cli.json").exists());
            }

            #[test]
            fn duplicate_fails_without_force() {
                let (tmp_dir, config_dir) = setup();
                let src = tmp_dir
                    .path()
                    .join("cli.json")
                    .to_str()
                    .unwrap()
                    .to_string();

                let opts1 = ImportOptions {
                    src: src.clone(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Cli,
                };
                import_to(opts1, &config_dir).unwrap();

                let opts2 = ImportOptions {
                    src,
                    name: None,
                    force: false,
                    config_type: ConfigType::Cli,
                };
                assert!(import_to(opts2, &config_dir).is_err());
            }

            #[test]
            fn duplicate_succeeds_with_force() {
                let (tmp_dir, config_dir) = setup();
                let src = tmp_dir
                    .path()
                    .join("cli.json")
                    .to_str()
                    .unwrap()
                    .to_string();

                let opts1 = ImportOptions {
                    src: src.clone(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Cli,
                };
                import_to(opts1, &config_dir).unwrap();

                let opts2 = ImportOptions {
                    src,
                    name: None,
                    force: true,
                    config_type: ConfigType::Cli,
                };
                import_to(opts2, &config_dir).unwrap();
            }
        }

        mod task {
            use super::*;

            #[test]
            fn imports_to_tasks_subdir() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Task,
                };
                import_to(opts, &config_dir).unwrap();
                assert!(config_dir.join("tasks").join("test.json").exists());
            }

            #[test]
            fn duplicate_stem_fails_without_force() {
                let (tmp_dir, config_dir) = setup();

                let opts1 = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Task,
                };
                import_to(opts1, &config_dir).unwrap();

                let opts2 = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.yml")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Task,
                };
                assert!(import_to(opts2, &config_dir).is_err());
            }

            #[test]
            fn force_removes_old_extension() {
                let (tmp_dir, config_dir) = setup();

                let opts1 = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Task,
                };
                import_to(opts1, &config_dir).unwrap();

                let opts2 = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.yml")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: true,
                    config_type: ConfigType::Task,
                };
                import_to(opts2, &config_dir).unwrap();

                assert!(config_dir.join("tasks").join("test.yml").exists());
                assert!(!config_dir.join("tasks").join("test.json").exists());
            }

            #[test]
            fn rejects_unsupported_extension() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.ini")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Task,
                };
                assert!(import_to(opts, &config_dir).is_err());
            }

            #[test]
            fn custom_name_overrides_filename() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: Some("custom.json".to_string()),
                    force: false,
                    config_type: ConfigType::Task,
                };
                import_to(opts, &config_dir).unwrap();
                assert!(config_dir.join("tasks").join("custom.json").exists());
                assert!(!config_dir.join("tasks").join("test.json").exists());
            }
        }

        mod infrast {
            use super::*;

            #[test]
            fn imports_to_infrast_subdir() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.json")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Infrast,
                };
                import_to(opts, &config_dir).unwrap();
                assert!(config_dir.join("infrast").join("test.json").exists());
            }

            #[test]
            fn accepts_any_extension() {
                let (tmp_dir, config_dir) = setup();
                let opts = ImportOptions {
                    src: tmp_dir
                        .path()
                        .join("test.ini")
                        .to_str()
                        .unwrap()
                        .to_string(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Infrast,
                };
                import_to(opts, &config_dir).unwrap();
                assert!(config_dir.join("infrast").join("test.ini").exists());
            }

            #[test]
            fn duplicate_fails_without_force() {
                let (tmp_dir, config_dir) = setup();
                let src = tmp_dir
                    .path()
                    .join("test.json")
                    .to_str()
                    .unwrap()
                    .to_string();

                let opts1 = ImportOptions {
                    src: src.clone(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Infrast,
                };
                import_to(opts1, &config_dir).unwrap();

                let opts2 = ImportOptions {
                    src,
                    name: None,
                    force: false,
                    config_type: ConfigType::Infrast,
                };
                assert!(import_to(opts2, &config_dir).is_err());
            }

            #[test]
            fn duplicate_succeeds_with_force() {
                let (tmp_dir, config_dir) = setup();
                let src = tmp_dir
                    .path()
                    .join("test.json")
                    .to_str()
                    .unwrap()
                    .to_string();

                let opts1 = ImportOptions {
                    src: src.clone(),
                    name: None,
                    force: false,
                    config_type: ConfigType::Infrast,
                };
                import_to(opts1, &config_dir).unwrap();

                let opts2 = ImportOptions {
                    src,
                    name: None,
                    force: true,
                    config_type: ConfigType::Infrast,
                };
                import_to(opts2, &config_dir).unwrap();
            }
        }

        #[test]
        fn import_uses_default_config_dir() {
            let tmp_dir = tempfile::tempdir().unwrap();
            fs::write(tmp_dir.path().join("test.json"), "{}").unwrap();

            let opts = ImportOptions {
                src: tmp_dir
                    .path()
                    .join("test.json")
                    .to_str()
                    .unwrap()
                    .to_string(),
                name: None,
                force: false,
                config_type: ConfigType::Test,
            };

            let result = import(opts);

            // Clean up
            if result.is_ok() {
                let test_dir = maa_dirs::config().join("__test__");
                if test_dir.exists() {
                    fs::remove_dir_all(&test_dir).unwrap();
                }
            }

            assert!(result.is_ok());
        }
    }

    mod import_source {
        use std::{sync::Once, thread};

        use super::*;

        const TEST_SERVER_PORT: u16 = 18081;
        static INIT_SERVER: Once = Once::new();

        /// Ensures the test HTTP server is started.
        fn ensure_test_server() {
            INIT_SERVER.call_once(|| {
                thread::spawn(|| {
                    let server = tiny_http::Server::http(("127.0.0.1", TEST_SERVER_PORT))
                        .expect("Failed to bind test server");

                    for request in server.incoming_requests() {
                        let url = request.url();

                        // Handle /config/{filename}
                        if let Some(filename) = url.strip_prefix("/config/") {
                            let content = match filename {
                                "task.json" => r#"{"tasks": ["task1", "task2"]}"#,
                                "cli.toml" => {
                                    r#"[maa]
user_resource = true"#
                                }
                                "test.yml" => r#"key: value"#,
                                _ => {
                                    let response = tiny_http::Response::from_string("Not found")
                                        .with_status_code(404);
                                    let _ = request.respond(response);
                                    continue;
                                }
                            };

                            let response = tiny_http::Response::from_string(content);
                            let _ = request.respond(response);
                            continue;
                        }

                        // 404 for other paths
                        let response =
                            tiny_http::Response::from_string("Not found").with_status_code(404);
                        let _ = request.respond(response);
                    }
                });

                // Wait for server to start
                thread::sleep(std::time::Duration::from_millis(100));
            });
        }

        mod from_str {
            use super::*;

            #[test]
            fn parses_http_url() {
                let source = ImportSource::from_str("http://example.com/file.json");
                assert_matches!(source, ImportSource::Remote(url) if url == "http://example.com/file.json");
            }

            #[test]
            fn parses_https_url() {
                let source = ImportSource::from_str("https://example.com/file.json");
                assert_matches!(source, ImportSource::Remote(url) if url == "https://example.com/file.json");
            }

            #[test]
            fn parses_local_path() {
                let source = ImportSource::from_str("/path/to/file.json");
                assert_matches!(source, ImportSource::Local(path) if path == Path::new("/path/to/file.json"));
            }

            #[test]
            fn parses_relative_path() {
                let source = ImportSource::from_str("./file.json");
                assert_matches!(source, ImportSource::Local(path) if path == Path::new("./file.json"));
            }

            #[test]
            fn trims_whitespace() {
                let source = ImportSource::from_str("  https://example.com/file.json  ");
                assert_matches!(source, ImportSource::Remote(url) if url == "https://example.com/file.json");
            }
        }

        mod filename {
            use super::*;

            #[test]
            fn extracts_from_url() {
                let source = ImportSource::Remote("https://example.com/path/to/file.json");
                assert_eq!(source.filename().unwrap(), "file.json");
            }

            #[test]
            fn extracts_from_url_with_query() {
                let source = ImportSource::Remote("https://example.com/file.json?version=1.0");
                assert_eq!(source.filename().unwrap(), "file.json");
            }

            #[test]
            fn extracts_from_local_path() {
                let source = ImportSource::Local(Path::new("/path/to/file.json"));
                assert_eq!(source.filename().unwrap(), "file.json");
            }

            #[test]
            fn fails_on_empty_url() {
                let source = ImportSource::Remote("https://example.com/");
                assert!(source.filename().is_err());
            }

            #[test]
            fn fails_on_invalid_local_path() {
                let source = ImportSource::Local(Path::new("/"));
                assert!(source.filename().is_err());
            }
        }

        mod copy_to {
            use super::*;

            #[test]
            fn copies_local_file() {
                let tmp_dir = tempfile::tempdir().unwrap();
                let src = tmp_dir.path().join("source.json");
                let dest = tmp_dir.path().join("dest.json");

                fs::write(&src, r#"{"test": "data"}"#).unwrap();

                let source = ImportSource::Local(&src);
                source.copy_to(&dest).unwrap();

                assert_eq!(fs::read_to_string(&dest).unwrap(), r#"{"test": "data"}"#);
            }

            #[test]
            fn downloads_remote_file() {
                ensure_test_server();

                let tmp_dir = tempfile::tempdir().unwrap();
                let dest = tmp_dir.path().join("downloaded.json");

                let url = format!("http://127.0.0.1:{TEST_SERVER_PORT}/config/task.json");
                let source = ImportSource::Remote(&url);
                source.copy_to(&dest).unwrap();

                assert_eq!(
                    fs::read_to_string(&dest).unwrap(),
                    r#"{"tasks": ["task1", "task2"]}"#
                );
            }

            #[test]
            fn downloads_different_file_types() {
                ensure_test_server();

                let tmp_dir = tempfile::tempdir().unwrap();

                // Test TOML
                let toml_dest = tmp_dir.path().join("cli.toml");
                let toml_url = format!("http://127.0.0.1:{TEST_SERVER_PORT}/config/cli.toml");
                ImportSource::Remote(&toml_url).copy_to(&toml_dest).unwrap();
                assert!(
                    fs::read_to_string(&toml_dest)
                        .unwrap()
                        .contains("user_resource")
                );

                // Test YAML
                let yaml_dest = tmp_dir.path().join("test.yml");
                let yaml_url = format!("http://127.0.0.1:{TEST_SERVER_PORT}/config/test.yml");
                ImportSource::Remote(&yaml_url).copy_to(&yaml_dest).unwrap();
                assert_eq!(fs::read_to_string(&yaml_dest).unwrap(), "key: value");
            }

            #[test]
            fn fails_on_404() {
                ensure_test_server();

                let tmp_dir = tempfile::tempdir().unwrap();
                let dest = tmp_dir.path().join("notfound.json");

                let url = format!("http://127.0.0.1:{TEST_SERVER_PORT}/config/notfound.json");
                let source = ImportSource::Remote(&url);

                assert!(source.copy_to(&dest).is_err());
            }

            #[test]
            fn fails_on_invalid_url() {
                let tmp_dir = tempfile::tempdir().unwrap();
                let dest = tmp_dir.path().join("file.json");

                let source = ImportSource::Remote(
                    "http://invalid.test.domain.that.does.not.exist/file.json",
                );

                assert!(source.copy_to(&dest).is_err());
            }
        }
    }
}
