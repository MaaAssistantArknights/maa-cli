// This file is used to download and extract prebuilt packages of maa-core.

use super::{download::download_mirrors, extract::Archive};

use crate::{
    dirs::{Dirs, Ensure},
    run,
};

use std::{
    env::{
        consts::{DLL_PREFIX, DLL_SUFFIX},
        current_exe, var_os,
    },
    path::{Component, Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::ValueEnum;
use semver::Version;
use serde::Deserialize;
use tokio::runtime::Runtime;

pub struct MaaCore {
    channel: Channel,
}

pub const MAA_CORE_NAME: &str = if cfg!(target_os = "macos") {
    "libMaaCore.dylib"
} else if cfg!(target_os = "windows") {
    "MaaCore.dll"
} else {
    "libMaaCore.so"
};

fn extract_mapper(
    path: &Path,
    lib_dir: &Path,
    resource_dir: &Path,
    resource: bool,
) -> Option<PathBuf> {
    let mut components = path.components();
    for c in components.by_ref() {
        match c {
            Component::Normal(c) => {
                if resource && c == "resource" {
                    // The components.as_path() is not working
                    // because it return a path with / as separator on windows
                    // I don't know why
                    let mut path = resource_dir.to_path_buf();
                    for c in components.by_ref() {
                        path.push(c);
                    }
                    return Some(path);
                } else if c
                    .to_str() // The DLL suffix may not the last part of the file name
                    .is_some_and(|s| s.starts_with(DLL_PREFIX) && s.contains(DLL_SUFFIX))
                {
                    return Some(lib_dir.join(c));
                } else {
                    continue;
                }
            }
            _ => continue,
        }
    }
    None
}

impl MaaCore {
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }

    pub fn version(&self, dirs: &Dirs) -> Result<Version> {
        let ver_str = run::core_version(dirs)?.trim();
        Version::parse(&ver_str[1..]).context("Failed to parse version")
    }

    pub fn install(&self, dirs: &Dirs, force: bool, no_resource: bool, t: u64) -> Result<()> {
        let lib_dir = &dirs.library().ensure()?;

        if lib_dir.join(MAA_CORE_NAME).exists() && !force {
            bail!("MaaCore already exists, use `maa update` to update it or `maa install --force` to force reinstall")
        }

        println!(
            "Fetching MaaCore version info (channel: {})...",
            self.channel
        );
        let version_json = get_version_json(self.channel)?;
        let asset = version_json.asset()?;
        println!("Downloading MaaCore {}...", version_json.version_str());
        let cache_dir = &dirs.cache().ensure()?;
        let resource_dir = dirs.resource();
        if !no_resource {
            resource_dir.ensure_clean()?;
        }
        let archive = asset.download(cache_dir, t)?;
        archive.extract(|path: &Path| extract_mapper(path, lib_dir, resource_dir, !no_resource))?;

        Ok(())
    }

    pub fn update(&self, dirs: &Dirs, no_resource: bool, t: u64) -> Result<()> {
        println!(
            "Fetching MaaCore version info (channel: {})...",
            self.channel
        );
        let version_json = get_version_json(self.channel)?;
        let current_version = self.version(dirs)?;
        let last_version = version_json.version();
        if current_version >= last_version {
            println!("Up to data: MaaCore v{}.", current_version);
            return Ok(());
        }

        println!(
            "Found newer MaaCore version: v{} (current: v{}), downloading...",
            last_version, current_version
        );

        let cache_dir = &dirs.cache().ensure()?;
        let asset = version_json.asset()?;
        let archive = asset.download(cache_dir, t)?;
        // Clean dirs before extracting, but not before downloading
        // because the download may be interrupted
        let lib_dir = find_lib_dir(dirs).context("MaaCore not found")?;
        let resource_dir = find_resource(dirs).context("Resource dir not found")?;
        if !no_resource {
            resource_dir.ensure_clean()?;
        }
        archive
            .extract(|path: &Path| extract_mapper(path, &lib_dir, &resource_dir, !no_resource))?;

        Ok(())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(ValueEnum, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")] // Rename to kebab-case to match CLI option
pub enum Channel {
    #[default]
    Stable,
    Beta,
    Alpha,
}

impl From<&Channel> for &str {
    fn from(channel: &Channel) -> Self {
        match channel {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Alpha => "alpha",
        }
    }
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.into();
        write!(f, "{}", s)
    }
}

fn get_version_json(channel: Channel) -> Result<VersionJSON> {
    let api_url = if let Some(url) = var_os("MAA_API_URL") {
        url.to_str().unwrap().to_owned()
    } else {
        "https://ota.maa.plus/MaaAssistantArknights/api/version".to_owned()
    };

    let url = format!("{}/{}.json", api_url, channel);
    let version_json: VersionJSON = reqwest::blocking::get(url)
        .context("Failed to get version json")?
        .json()
        .context("Failed to parse version json")?;
    Ok(version_json)
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct VersionJSON {
    version: String,
    details: VersionDetails,
}

impl VersionJSON {
    pub fn version(&self) -> Version {
        Version::parse(&self.version[1..]).unwrap()
    }

    pub fn version_str(&self) -> &str {
        &self.version
    }

    pub fn name(&self) -> Result<String> {
        let version = self.version();
        if cfg!(target_os = "macos") {
            Ok(format!("MAA-v{}-macos-runtime-universal.zip", version))
        } else if cfg!(target_os = "linux") {
            if cfg!(target_arch = "x86_64") {
                Ok(format!("MAA-v{}-linux-x86_64.tar.gz", version))
            } else if cfg!(target_arch = "aarch64") {
                Ok(format!("MAA-v{}-linux-aarch64.tar.gz", version))
            } else {
                Err(anyhow!(
                    "Unsupported architecture: {}",
                    std::env::consts::ARCH
                ))
            }
        } else if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") {
                Ok(format!("MAA-v{}-win-x64.zip", version))
            } else if cfg!(target_arch = "aarch64") {
                Ok(format!("MAA-v{}-win-arm64.zip", version))
            } else {
                Err(anyhow!(
                    "Unsupported architecture: {}",
                    std::env::consts::ARCH
                ))
            }
        } else {
            Err(anyhow!("Unsupported platform"))
        }
    }

    pub fn asset(&self) -> Result<&Asset> {
        let asset_name = self.name()?;
        self.details
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| anyhow!("Asset not found"))
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct VersionDetails {
    pub assets: Vec<Asset>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
    pub mirrors: Vec<String>,
}

impl Asset {
    pub fn download(&self, dir: &Path, t: u64) -> Result<Archive> {
        let path = dir.join(&self.name);
        let size = self.size;

        if path.exists() {
            let file_size = match path.metadata() {
                Ok(metadata) => metadata.len(),
                Err(_) => 0,
            };
            if file_size == size {
                println!("File {} already exists, skip download!", &self.name);
                return Archive::try_from(path);
            }
        }

        let url = &self.browser_download_url;
        let mut mirrors = self.mirrors.clone();
        mirrors.push(url.to_owned());

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .build()
            .context("Failed to build reqwest client")?;
        Runtime::new()
            .context("Failed to create tokio runtime")?
            .block_on(download_mirrors(
                &client, url, mirrors, &path, size, t, None,
            ))?;

        Archive::try_from(path)
    }
}

pub fn find_lib_dir(dirs: &Dirs) -> Option<PathBuf> {
    let lib_dir = dirs.library();
    if lib_dir.join(MAA_CORE_NAME).exists() {
        return Some(lib_dir.to_path_buf());
    }

    if let Ok(path) = current_exe() {
        let path = path.canonicalize().unwrap();
        let exe_dir = path.parent().unwrap();
        if exe_dir.join(MAA_CORE_NAME).exists() {
            return Some(exe_dir.to_path_buf());
        }
        if let Some(dir) = exe_dir.parent() {
            let lib_dir = dir.join("lib");
            if lib_dir.join(MAA_CORE_NAME).exists() {
                return Some(lib_dir);
            }
        }
    }

    None
}

pub fn find_maa_core(dirs: &Dirs) -> Option<PathBuf> {
    let lib_dir = find_lib_dir(dirs)?;
    Some(lib_dir.join(MAA_CORE_NAME))
}

pub fn find_resource(dirs: &Dirs) -> Option<PathBuf> {
    let resource_dir = dirs.resource();
    if resource_dir.exists() {
        return Some(resource_dir.to_path_buf());
    }

    if let Ok(path) = current_exe() {
        let path = path.canonicalize().unwrap();
        let exe_dir = path.parent().unwrap();
        let resource_dir = exe_dir.join("resource");
        if resource_dir.exists() {
            return Some(resource_dir);
        }
        if let Some(dir) = exe_dir.parent() {
            let share_dir = dir.join("share");
            if let Some(extra_share) = option_env!("MAA_EXTRA_SHARE_NAME") {
                let resource_dir = share_dir.join(extra_share).join("resource");
                if resource_dir.exists() {
                    return Some(resource_dir);
                }
            }
            let resource_dir = share_dir.join("maa").join("resource");
            if resource_dir.exists() {
                return Some(resource_dir);
            }
        }
    }

    None
}
