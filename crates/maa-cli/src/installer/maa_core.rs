// This file is used to download and extract prebuilt packages of maa-core.

use std::{
    borrow::Cow,
    env::consts::{ARCH, DLL_PREFIX, DLL_SUFFIX, OS},
    path::{self, Path},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use log::debug;
use semver::Version;
use serde::Deserialize;
use tokio::runtime::Runtime;

use super::{
    download::{check_file_exists, download_mirrors},
    extract::Archive,
    version_json::VersionJSON,
};
use crate::{
    config::cli::{
        maa_core::{CommonArgs, Components, Config},
        CLI_CONFIG,
    },
    dirs::{self, Ensure},
    run,
};

fn extract_mapper(
    src: Cow<Path>,
    lib_dir: &Path,
    resource_dir: &Path,
    config: &Components,
) -> Option<std::path::PathBuf> {
    debug!("Extracting file: {}", src.display());
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
                    debug!("Extracting {} => {}", src.display(), dest.display());
                    return Some(dest);
                }
                if config.library && c
                    .to_str() // The DLL suffix may not the last part of the file name
                    .is_some_and(|s| s.starts_with(DLL_PREFIX) && s.contains(DLL_SUFFIX))
                {
                    let dest = lib_dir.join(src.file_name()?);
                    debug!("Extracting {} => {}", src.display(), dest.display());
                    return Some(dest);
                }
            }
            _ => continue,
        }
    }
    debug!("Ignored file {}", src.display());
    None
}

/// Get installed MaaCore version
pub fn version() -> Result<Version> {
    let v_str = run::core_version()?;
    let v_str = v_str.trim();

    v_str
        .strip_prefix('v')
        .unwrap_or(v_str)
        .parse()
        .context("Failed to get version")
}

pub fn install(force: bool, args: &CommonArgs) -> Result<()> {
    let config = CLI_CONFIG.core_config().apply_args(args);

    let lib_dir = dirs::library();
    let lib_name = format!("{}MaaCore{}", DLL_PREFIX, DLL_SUFFIX);

    if lib_dir.join(lib_name).exists() && !force {
        bail!("MaaCore already exists, use `maa update` to update it or `maa install --force` to force reinstall")
    }

    println!(
        "Fetching MaaCore version info (channel: {})...",
        config.channel()
    );
    let version_json = get_version_json(&config)?;
    let asset_version = version_json.version();
    let asset_name = name(asset_version)?;
    let asset = version_json.details().asset(&asset_name)?;

    println!("Downloading MaaCore {}...", asset_version);
    let cache_dir = dirs::cache().ensure()?;
    let archive = download(
        cache_dir.join(asset_name).into(),
        asset.size(),
        asset.download_links(),
        &config,
    )?;

    println!("Installing MaaCore...");
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
    archive.extract(|path| extract_mapper(path, lib_dir, resource_dir, components))?;

    Ok(())
}

pub fn update(args: &CommonArgs) -> Result<()> {
    let config = CLI_CONFIG.core_config().apply_args(args);

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

    println!(
        "Fetching MaaCore version info (channel: {})...",
        config.channel()
    );
    let version_json = get_version_json(&config)?;
    let asset_version = version_json.version();
    let current_version = version()?;
    if !version_json.can_update("MaaCore", &current_version)? {
        return Ok(());
    }
    let asset_name = name(asset_version)?;
    let asset = version_json.details().asset(&asset_name)?;

    println!("Downloading MaaCore {}...", asset_version);
    let cache_dir = dirs::cache().ensure()?;
    let asset_path = cache_dir.join(asset_name);
    let archive = download(
        asset_path.into(),
        asset.size(),
        asset.download_links(),
        &config,
    )?;

    println!("Installing MaaCore...");
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
pub fn name(version: &Version) -> Result<String> {
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

pub fn download<'p>(
    path: Cow<'p, Path>,
    size: u64,
    links: Vec<String>,
    config: &Config,
) -> Result<Archive<'p>> {
    if check_file_exists(&path, size) {
        println!("Already downloaded, skip downloading");
        return Archive::new(path);
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
            &path,
            size,
            config.test_time(),
            None,
        ))
        .context("Failed to download asset")?;

    Archive::new(path)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_json;

    use super::*;

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

    #[test]
    fn test_extract_mapper() {
        let config = Components::default();
        let lib_dir = Path::new("/home/user/.local/share/maa/lib");
        let resource_dir = Path::new("/home/user/.local/share/maa/resource");

        #[cfg(unix)]
        {
            #[cfg(target_os = "linux")]
            assert_eq!(
                extract_mapper(
                    Cow::Borrowed(Path::new("libMaaCore.so")),
                    lib_dir,
                    resource_dir,
                    &config
                ),
                Some(lib_dir.join("libMaaCore.so"))
            );
            #[cfg(target_os = "macos")]
            assert_eq!(
                extract_mapper(
                    Cow::Borrowed(Path::new("libMaaCore.dylib")),
                    lib_dir,
                    resource_dir,
                    &config
                ),
                Some(lib_dir.join("libMaaCore.dylib"))
            );
            #[cfg(target_os = "windows")]
            assert_eq!(
                extract_mapper(
                    Cow::Borrowed(Path::new("MaaCore.dll")),
                    lib_dir,
                    resource_dir,
                    &config
                ),
                Some(lib_dir.join("MaaCore.dll"))
            );
            assert_eq!(
                extract_mapper(
                    Cow::Borrowed(Path::new("resource/config.json")),
                    lib_dir,
                    resource_dir,
                    &config
                ),
                Some(resource_dir.join("config.json"))
            );
            assert_eq!(
                extract_mapper(
                    Cow::Borrowed(Path::new("misc")),
                    lib_dir,
                    resource_dir,
                    &config
                ),
                None
            );
        }
    }
}
