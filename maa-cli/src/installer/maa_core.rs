// This file is used to download and extract prebuilt packages of maa-core.

use super::{
    download::{check_file_exists, download_mirrors},
    extract::Archive,
    version_json::VersionJSON,
};

use crate::{
    config::cli::{
        cli_config,
        maa_core::{CommonArgs, Components, Config},
    },
    consts::MAA_CORE_LIB,
    dirs::{self, Ensure},
    run,
};

use std::{
    env::consts::{ARCH, DLL_PREFIX, DLL_SUFFIX, OS},
    path::{self, Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use tokio::runtime::Runtime;

fn extract_mapper(
    src: &Path,
    lib_dir: &Path,
    resource_dir: &Path,
    config: &Components,
) -> Option<PathBuf> {
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
                    return Some(dest);
                } else if config.library && c
                    .to_str() // The DLL suffix may not the last part of the file name
                    .is_some_and(|s| s.starts_with(DLL_PREFIX) && s.contains(DLL_SUFFIX))
                {
                    let dest = lib_dir.join(src.file_name()?);
                    return Some(dest);
                } else {
                    continue;
                }
            }
            _ => continue,
        }
    }
    None
}

pub fn version() -> Result<Version> {
    let ver_str = run::core_version()?.trim();
    Version::parse(&ver_str[1..]).with_context(lfl!("failed-parse-version"))
}

pub fn install(force: bool, args: &CommonArgs) -> Result<()> {
    let config = cli_config().core_config().apply_args(args);

    let components = config.components();
    // Check if any component is specified
    if !(components.library || components.resource) {
        bailfl!("no-component-to-install");
    }

    let lib_dir = dirs::library();

    if lib_dir.join(MAA_CORE_LIB).exists() && !force {
        bailfl!("core-already-installed");
    }

    printlnfl!("fetching", name = "MaaCore", channel = config.channel());
    let version_json = get_version_json(&config)?;
    let asset_version = version_json.version().to_owned();
    let asset_name = name(&asset_version)?;

    let archive = version_json
        .details()
        .asset(&asset_name)?
        .download(config.test_time())?;

    printlnfl!(
        "installing",
        name = "maa-core",
        version = asset_version.to_string()
    );
    if components.library {
        warn!("deprecated-disable-library-option");
        lib_dir.ensure_clean()?;
    }
    let resource_dir = dirs::resource();
    if components.resource {
        warn!("deprecated-disable-resource-option");
        resource_dir.ensure_clean()?;
    }
    archive.extract(|path: &Path| extract_mapper(path, lib_dir, resource_dir, components))?;

    Ok(())
}

pub fn update(args: &CommonArgs) -> Result<()> {
    let config = cli_config().core_config().apply_args(args);

    let components = config.components();
    // Check if any component is specified
    if !(components.library || components.resource) {
        bailfl!("no-component-to-install");
    }
    // Check if MaaCore is installed and installed by maa
    let lib_dir = dirs::library();
    let resource_dir = dirs::resource();
    match (components.library, dirs::find_library()) {
        (true, Some(dir)) if dir != lib_dir => bailfl!(
            "library-installed-by-other",
            path = dir.to_str().unwrap_or("")
        ),
        (false, _) => {
            warn!("deprecated-disable-library-option");
        }
        _ => {}
    }
    match (components.resource, dirs::find_resource()) {
        (true, Some(dir)) if dir != resource_dir => bailfl!(
            "resource-installed-by-other",
            path = dir.to_str().unwrap_or("")
        ),
        (false, _) => {
            warn!("deprecated-disable-resource-option");
        }
        _ => {}
    }

    printlnfl!("fetching", name = "MaaCore", channel = config.channel());
    let version_json = get_version_json(&config)?;
    let asset_version = version_json.version().to_owned();
    let current_version = version()?;
    if !version_json.can_update("MaaCore", &current_version)? {
        return Ok(());
    }
    let asset_name = name(&asset_version)?;

    let archive = version_json
        .details()
        .asset(&asset_name)?
        .download(config.test_time())?;

    printlnfl!(
        "installing",
        name = "MaaCore",
        version = asset_version.to_string()
    );
    if components.library {
        lib_dir.ensure_clean()?;
    }
    if components.resource {
        resource_dir.ensure_clean()?;
    }
    archive.extract(|path| extract_mapper(path, lib_dir, resource_dir, components))?;

    Ok(())
}

fn get_version_json(config: &Config) -> Result<VersionJSON<Details>> {
    let api_url = config.api_url();
    let version_json = reqwest::blocking::get(&api_url)
        .with_context(lfl!("failed-fetch-version-json", url = api_url.as_str()))?
        .json()
        .with_context(lfl!("failed-parse-version-json"))?;

    Ok(version_json)
}

/// Get the name of the asset for the current platform
fn name(version: &Version) -> Result<String> {
    match OS {
        "macos" => Ok(format!("MAA-v{}-macos-runtime-universal.zip", version)),
        "linux" => match ARCH {
            "x86_64" => Ok(format!("MAA-v{}-linux-x86_64.tar.gz", version)),
            "aarch64" => Ok(format!("MAA-v{}-linux-aarch64.tar.gz", version)),
            _ => bailfl!("unsupported-architecture", arch = ARCH),
        },
        "windows" => match ARCH {
            "x86_64" => Ok(format!("MAA-v{}-win-x64.zip", version)),
            "aarch64" => Ok(format!("MAA-v{}-win-arm64.zip", version)),
            _ => bailfl!("unsupported-architecture", arch = ARCH),
        },
        _ => bailfl!("unsupported-platform", os = OS, arch = ARCH),
    }
}

#[derive(Deserialize)]
pub struct Details {
    assets: Vec<Asset>,
}

impl Details {
    pub fn asset(self, name: &str) -> Result<Asset> {
        self.assets
            .into_iter()
            .find(|asset| name == asset.name())
            .with_context(lfl!("asset-not-found", name = name))
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
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn download_links(&self) -> Vec<String> {
        let mut links = self.mirrors.clone();
        links.insert(0, self.browser_download_url.clone());
        links
    }

    fn download(self, test_time: u64) -> Result<Archive> {
        let file = self.name;
        let size = self.size;
        let cache_dir = dirs::cache().ensure()?;
        let path = cache_dir.join(&file);
        if check_file_exists(&path, size) {
            printlnfl!("package-cache-hit", file = file);
            return Archive::try_from(path);
        }

        let mut links = self.mirrors;
        links.insert(0, self.browser_download_url);

        printlnfl!("downloading", file = file);
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .build()
            .with_context(lfl!("failed-create-reqwest-client"))?;
        Runtime::new()
            .with_context(lfl!("failed-create-tokio-runtime"))?
            .block_on(download_mirrors(
                &client, links, &path, size, test_time, None,
            ))?;

        Archive::try_from(path)
    }
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

        let version = version_json.version().to_owned();
        let asset_name = name(&version).unwrap();
        let asset = version_json
            .details()
            .asset(&asset_name)
            .expect("Failed to get asset");

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
