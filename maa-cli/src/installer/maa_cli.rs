// This file is used to download and extract prebuilt packages of maa-cli.

use crate::{
    dirs::{Dirs, Ensure},
    maa_run::{command, SetLDLibPath},
};

use super::{
    download::{download, Checker},
    extract::Archive,
};

use std::env::{consts::EXE_SUFFIX, current_exe};
use std::str::from_utf8;
use std::{env::var_os, path::Path};

use anyhow::{bail, Context, Ok, Result};
use semver::Version;
use serde::Deserialize;
use tokio::runtime::Runtime;

const MAA_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Copy, Clone)]
pub enum CLIComponent {
    MaaCLI,
    MaaRun,
}

impl CLIComponent {
    pub fn name(self) -> String {
        match self {
            CLIComponent::MaaCLI => format!("maa{}", EXE_SUFFIX),
            CLIComponent::MaaRun => format!("maa-run{}", EXE_SUFFIX),
        }
    }

    pub fn version(self, dirs: &Dirs) -> Result<Version> {
        match self {
            CLIComponent::MaaCLI => {
                Version::parse(MAA_CLI_VERSION).context("Failed to parse maa-cli version")
            }
            CLIComponent::MaaRun => {
                let output = &command(dirs)?
                    .set_ld_lib_path(dirs)
                    .arg("--version")
                    .output()
                    .context("Failed to run maa-run")?
                    .stdout;
                // Remove "maa-run " prefix and "\n" suffix
                let ver_str = from_utf8(&output[8..output.len() - 1])
                    .context("Failed to parse maa-run output")?;
                Version::parse(ver_str).context("Failed to parse maa-run version")
            }
        }
    }

    pub fn install(self, dirs: &Dirs) -> Result<()> {
        let bin_dir = dirs.binary().ensure()?;
        let bin_name = self.name();
        let bin_path = bin_dir.join(&bin_name);
        if bin_path.exists() {
            bail!(
                "{} already exists, please run `maa self update` to update it",
                bin_path.display()
            );
        };

        let version_json = get_metadata()?;
        let asset = version_json.get_asset(self)?;

        let cache_dir = dirs.cache().ensure()?;

        asset.download(cache_dir)?.extract(|path| {
            if path.ends_with(&bin_name) {
                Some(bin_path.clone())
            } else {
                None
            }
        })
    }

    pub fn update(self, dirs: &Dirs) -> Result<()> {
        let version_json = get_metadata()?;
        let asset = version_json.get_asset(self)?;
        let version = asset.version();

        let cache_dir = dirs.cache().ensure()?;

        let last_version = self.version(dirs)?;
        if *version > last_version {
            println!(
                "Found newer {} version v{} (current: v{}), updating...",
                self.name(),
                version,
                last_version
            );
            let bin_name = self.name();
            let bin_path = match self {
                CLIComponent::MaaCLI => current_exe()?,
                CLIComponent::MaaRun => dirs.binary().join(&bin_name),
            };

            asset.download(cache_dir)?.extract(|path| {
                if path.ends_with(&bin_name) {
                    Some(bin_path.clone())
                } else {
                    None
                }
            })?;
        } else {
            println!("Up to date: {} v{}.", self.name(), last_version);
        }

        Ok(())
    }
}

fn get_metadata() -> Result<VersionJSON> {
    let metadata_url = if let Some(url) = var_os("MAA_CLI_API") {
        url.into_string().unwrap()
    } else {
        String::from("https://github.com/wangl-cc/maa-cli/raw/version/version.json")
    };
    let metadata: VersionJSON = reqwest::blocking::get(metadata_url)?.json()?;
    Ok(metadata)
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
/// The version.json file from the server.
///
/// Example:
/// ```json
/// {
///    "maa-cli": {
///      "universal-apple-darwin": {
///        "version": "0.1.0",
///        "name": "maa_cli-v0.1.0-universal-apple-darwin.tar.gz",
///        "size": 123456,
///        "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
///      },
///      "x86_64-unknown-linux-gnu": {
///        ...
///      }
///   },
///   "maa-run": {
///     "universal-apple-darwin": {
///       ...
///     },
///     ...
///   }
/// }
/// ```
struct VersionJSON {
    pub maa_cli: Targets,
    pub maa_run: Targets,
}

impl VersionJSON {
    pub fn get_asset(&self, compoment: CLIComponent) -> Result<&Asset> {
        let targets = match compoment {
            CLIComponent::MaaCLI => &self.maa_cli,
            CLIComponent::MaaRun => &self.maa_run,
        };

        if cfg!(target_os = "macos") {
            Ok(&targets.universal_macos)
        } else if cfg!(target_os = "linux")
            && cfg!(target_arch = "x86_64")
            && cfg!(target_env = "gnu")
        {
            Ok(&targets.x64_linux_gnu)
        } else {
            bail!("Unsupported platform")
        }
    }
}

#[derive(Deserialize)]
pub struct Targets {
    #[serde(rename = "universal-apple-darwin")]
    universal_macos: Asset,
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    x64_linux_gnu: Asset,
}

#[derive(Deserialize)]
pub struct Asset {
    version: Version,
    tag: String,
    name: String,
    size: u64,
    sha256sum: String,
}

impl Asset {
    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn download(&self, dir: &Path) -> Result<Archive> {
        let path = dir.join(&self.name);
        let size = self.size;

        if path.exists() {
            let file_size = path.metadata()?.len();
            if file_size == size {
                println!("Found existing file: {}", path.display());
                return Ok(Archive::try_from(path)?);
            }
        }

        let url = format_url(&self.tag, &self.name);

        let client = reqwest::Client::new();
        Runtime::new()
            .context("Failed to create tokio runtime")?
            .block_on(download(
                &client,
                &url,
                &path,
                size,
                Some(Checker::Sha256(&self.sha256sum)),
            ))
            .context("Failed to download maa-cli")?;

        Ok(Archive::try_from(path)?)
    }
}

fn format_url(tag: &str, name: &str) -> String {
    if let Some(url) = var_os("MAA_CLI_DOWNLOAD") {
        format!("{}/{}/{}", url.into_string().unwrap(), tag, name)
    } else {
        format!(
            "https://github.com/wangl-cc/maa-cli/releases/download/{}/{}",
            tag, name
        )
    }
}
