// This file is used to download and extract prebuilt packages of maa-core.

use super::{
    download::{check_file_exists, download_mirrors},
    extract::Archive,
    version_json::VersionJSON,
};

use crate::{
    config::{
        cli::{
            maa_core::{CommonArgs, Components, Config},
            InstallerConfig,
        },
        Error as ConfigError, FindFile,
    },
    consts::MAA_CORE_LIB,
    debug,
    dirs::{self, Ensure},
    normal, run,
};

use std::{
    env::consts::{ARCH, DLL_PREFIX, DLL_SUFFIX, OS},
    path::{self, Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use semver::Version;
use serde::Deserialize;
use tokio::runtime::Runtime;

fn extract_mapper(
    src: &Path,
    lib_dir: &Path,
    resource_dir: &Path,
    config: &Components,
) -> Option<PathBuf> {
    debug!("Extracting file:", src.display());
    let mut path_components = src.components();
    for c in path_components.by_ref() {
        match c {
            path::Component::Normal(c) => {
                if config.resource && c == "resource" {
                    // The components.as_path() is not working
                    // because it return a path with / as separator on windows
                    // I don't know why
                    let mut dest = resource_dir.to_path_buf();
                    for c in path_components.by_ref() {
                        dest.push(c);
                    }
                    debug!(
                        "Extracting",
                        format!("{} => {}", src.display(), dest.display())
                    );
                    return Some(dest);
                } else if config.library && c
                    .to_str() // The DLL suffix may not the last part of the file name
                    .is_some_and(|s| s.starts_with(DLL_PREFIX) && s.contains(DLL_SUFFIX))
                {
                    let dest = lib_dir.join(src.file_name()?);
                    debug!(
                        "Extracting",
                        format!("{} => {}", src.display(), dest.display())
                    );
                    return Some(dest);
                } else {
                    continue;
                }
            }
            _ => continue,
        }
    }
    debug!("Ignore file:", src.display());
    None
}

pub fn version() -> Result<Version> {
    let ver_str = run::core_version()?.trim();
    Version::parse(&ver_str[1..]).context("Failed to parse version")
}

fn get_config(args: &CommonArgs) -> Result<Config, ConfigError> {
    match InstallerConfig::find_file(&dirs::config().join("cli")) {
        Ok(config) => {
            let mut config = config.core_config();
            config.apply_args(args);
            Ok(config)
        }
        Err(ConfigError::FileNotFound(_)) => Ok(Config::default()),
        Err(e) => Err(e),
    }
}

pub fn install(force: bool, args: &CommonArgs) -> Result<()> {
    let config = get_config(args)?;

    let lib_dir = dirs::library();

    if lib_dir.join(MAA_CORE_LIB).exists() && !force {
        bail!("MaaCore already exists, use `maa update` to update it or `maa install --force` to force reinstall")
    }

    normal!(format!(
        "Fetching MaaCore version info (channel: {})...",
        config.channel()
    ));
    let version_json = get_version_json(&config)?;
    let asset_version = version_json.version();
    let asset_name = name(asset_version)?;
    let asset = version_json.details().asset(&asset_name)?;

    normal!(format!("Downloading MaaCore {}...", asset_version));
    let cache_dir = dirs::cache().ensure()?;
    let archive = download(
        &cache_dir.join(asset_name),
        asset.size(),
        asset.download_links(),
        &config,
    )?;

    normal!("Installing MaaCore...");
    let components = config.components();
    if components.library {
        debug!("Cleaning library directory");
        lib_dir.ensure_clean()?;
    }
    let resource_dir = dirs::resource();
    if components.resource {
        debug!("Cleaning resource directory");
        resource_dir.ensure_clean()?;
    }
    archive.extract(|path: &Path| extract_mapper(path, lib_dir, resource_dir, components))?;

    Ok(())
}

pub fn update(args: &CommonArgs) -> Result<()> {
    let config = get_config(args)?;

    let components = config.components();
    // Check if any component is specified
    if !(components.library || components.resource) {
        bail!("No component specified, aborting");
    }
    // Check if MaaCore is installed and installed by maa
    let lib_dir = dirs::library();
    let resource_dir = dirs::resource();
    match (components.library, dirs::find_library()) {
        (true, Some(dir)) if dir != lib_dir => bail!(
            "MaaCore found at {} but not installed by maa, aborting",
            dir.display()
        ),
        _ => {}
    }
    match (components.resource, dirs::find_resource()) {
        (true, Some(dir)) if dir != resource_dir => bail!(
            "MaaCore resource found at {} but not installed by maa, aborting",
            dir.display()
        ),
        _ => {}
    }

    normal!(format!(
        "Fetching MaaCore version info (channel: {})...",
        config.channel()
    ));
    let version_json = get_version_json(&config)?;
    let asset_version = version_json.version();
    let current_version = version()?;
    if !version_json.can_update("MaaCore", &current_version)? {
        return Ok(());
    }
    let asset_name = name(asset_version)?;
    let asset = version_json.details().asset(&asset_name)?;

    normal!(format!("Downloading MaaCore {}...", asset_version));
    let cache_dir = dirs::cache().ensure()?;
    let asset_path = cache_dir.join(asset_name);
    let archive = download(&asset_path, asset.size(), asset.download_links(), &config)?;

    normal!("Installing MaaCore...");
    if components.library {
        debug!("Cleaning library directory");
        lib_dir.ensure_clean()?;
    }
    if components.resource {
        debug!("Cleaning resource directory");
        resource_dir.ensure_clean()?;
    }
    archive.extract(|path| extract_mapper(path, lib_dir, resource_dir, components))?;

    Ok(())
}

fn get_version_json(config: &Config) -> Result<VersionJSON<Details>> {
    let url = config.api_url();
    let version_json = reqwest::blocking::get(&url)
        .with_context(|| format!("Failed to fetch version info from {}", url))?
        .json()
        .with_context(|| "Failed to parse version info")?;

    Ok(version_json)
}

/// Get the name of the asset for the current platform
fn name(version: &Version) -> Result<String> {
    match OS {
        "macos" => Ok(format!("MAA-v{}-macos-runtime-universal.zip", version)),
        "linux" => match ARCH {
            "x86_64" => Ok(format!("MAA-v{}-linux-x86_64.tar.gz", version)),
            "aarch64" => Ok(format!("MAA-v{}-linux-aarch64.tar.gz", version)),
            _ => Err(anyhow!("Unsupported architecture: {}", ARCH)),
        },
        "windows" => match ARCH {
            "x86_64" => Ok(format!("MAA-v{}-win-x64.zip", version)),
            "aarch64" => Ok(format!("MAA-v{}-win-arm64.zip", version)),
            _ => Err(anyhow!("Unsupported architecture: {}", ARCH)),
        },
        _ => Err(anyhow!("Unsupported platform: {}", OS)),
    }
}

#[derive(Deserialize)]
pub struct Details {
    assets: Vec<Asset>,
}

impl Details {
    pub fn asset(&self, name: &str) -> Result<&Asset> {
        self.assets
            .iter()
            .find(|asset| name == asset.name())
            .ok_or_else(|| anyhow!("Asset not found"))
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    mirrors: Vec<String>,
}

impl Asset {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn download_links(&self) -> Vec<String> {
        let mut links = self.mirrors.clone();
        links.insert(0, self.browser_download_url.clone());
        links
    }
}

pub fn download(path: &Path, size: u64, links: Vec<String>, config: &Config) -> Result<Archive> {
    if check_file_exists(path, size) {
        normal!("Already downloaded, skip downloading");
        return Archive::try_from(path);
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .build()
        .context("Failed to build reqwest client")?;
    Runtime::new()
        .context("Failed to create tokio runtime")?
        .block_on(download_mirrors(
            &client,
            links,
            path,
            size,
            config.test_time(),
            None,
        ))
        .context("Failed to download asset")?;

    Archive::try_from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json;

    #[test]
    fn deserialize_version_json() {
        // This is a stripped version of the real json
        let json_str = r#"
{
  "version": "v4.26.1",
  "details": {
    "tag_name": "v4.26.1",
    "name": "v4.26.1",
    "draft": false,
    "prerelease": false,
    "created_at": "2023-11-02T16:27:04Z",
    "published_at": "2023-11-02T16:50:51Z",
    "assets": [
      {
        "name": "MAA-v4.26.1-linux-aarch64.tar.gz",
        "size": 152067668,
        "browser_download_url": "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-aarch64.tar.gz",
        "mirrors": [
          "https://s3.maa-org.net:25240/maa-release/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-aarch64.tar.gz",
          "https://agent.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-aarch64.tar.gz",
          "https://maa.r2.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-aarch64.tar.gz"
        ]
      },
      {
        "name": "MAA-v4.26.1-linux-x86_64.tar.gz",
        "size": 155241185,
        "browser_download_url": "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz",
        "mirrors": [
          "https://s3.maa-org.net:25240/maa-release/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz",
          "https://agent.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz",
          "https://maa.r2.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz"
        ]
      },
      {
        "name": "MAA-v4.26.1-win-arm64.zip",
        "size": 148806502,
        "browser_download_url": "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-arm64.zip",
        "mirrors": [
          "https://s3.maa-org.net:25240/maa-release/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-arm64.zip",
          "https://agent.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-arm64.zip",
          "https://maa.r2.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-arm64.zip"
        ]
      },
      {
        "name": "MAA-v4.26.1-win-x64.zip",
        "size": 150092421,
        "browser_download_url": "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-x64.zip",
        "mirrors": [
          "https://s3.maa-org.net:25240/maa-release/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-x64.zip",
          "https://agent.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-x64.zip",
          "https://maa.r2.imgg.dev/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-win-x64.zip"
        ]
      },
      {
        "name": "MAA-v4.26.1-macos-runtime-universal.zip",
        "size": 164012486,
        "browser_download_url": "https://github.com/MaaAssistantArknights/MaaRelease/releases/download/v4.26.1/MAA-v4.26.1-macos-runtime-universal.zip",
        "mirrors": [
          "https://s3.maa-org.net:25240/maa-release/MaaAssistantArknights/MaaRelease/releases/download/v4.26.1/MAA-v4.26.1-macos-runtime-universal.zip",
          "https://agent.imgg.dev/MaaAssistantArknights/MaaRelease/releases/download/v4.26.1/MAA-v4.26.1-macos-runtime-universal.zip",
          "https://maa.r2.imgg.dev/MaaAssistantArknights/MaaRelease/releases/download/v4.26.1/MAA-v4.26.1-macos-runtime-universal.zip"
        ]
      }
    ],
    "tarball_url": "https://api.github.com/repos/MaaAssistantArknights/MaaAssistantArknights/tarball/v4.26.1",
    "zipball_url": "https://api.github.com/repos/MaaAssistantArknights/MaaAssistantArknights/zipball/v4.26.1"
  }
}
            "#;

        let version_json: VersionJSON<Details> =
            serde_json::from_str(json_str).expect("Failed to parse json");

        assert!(version_json
            .can_update("MaaCore", &Version::parse("4.26.0").unwrap())
            .unwrap());
        assert!(version_json
            .can_update("MaaCore", &Version::parse("4.26.1-beta.1").unwrap())
            .unwrap());
        assert!(!version_json
            .can_update("MaaCore", &Version::parse("4.27.0").unwrap())
            .unwrap());

        assert_eq!(
            version_json.version(),
            &Version::parse("4.26.1").expect("Failed to parse version")
        );

        let details = version_json.details();
        let asset_name = name(version_json.version()).unwrap();
        let asset = details.asset(&asset_name).unwrap();

        // Test asset name, size and download links
        match OS {
            "macos" => {
                assert_eq!(asset.name(), "MAA-v4.26.1-macos-runtime-universal.zip");
                assert_eq!(asset.size(), 164012486);
                assert_eq!(asset.download_links().len(), 4);
            }
            "linux" => match ARCH {
                "x86_64" => {
                    assert_eq!(asset.name(), "MAA-v4.26.1-linux-x86_64.tar.gz");
                    assert_eq!(asset.size(), 155241185);
                    assert_eq!(asset.download_links().len(), 4);
                }
                "aarch64" => {
                    assert_eq!(asset.name(), "MAA-v4.26.1-linux-aarch64.tar.gz");
                    assert_eq!(asset.size(), 152067668);
                    assert_eq!(asset.download_links().len(), 4);
                }
                _ => (),
            },
            "windows" => match ARCH {
                "x86_64" => {
                    assert_eq!(asset.name(), "MAA-v4.26.1-win-x64.zip");
                    assert_eq!(asset.size(), 150092421);
                    assert_eq!(asset.download_links().len(), 4);
                }
                "aarch64" => {
                    assert_eq!(asset.name(), "MAA-v4.26.1-win-arm64.zip");
                    assert_eq!(asset.size(), 148806502);
                    assert_eq!(asset.download_links().len(), 4);
                }
                _ => (),
            },
            _ => (),
        }
    }
}
