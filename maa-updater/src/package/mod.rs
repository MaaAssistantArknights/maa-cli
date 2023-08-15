mod download;
use download::download_package;

use super::arg_env_or_default;

use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::ValueEnum;
use serde::Deserialize;

#[derive(ValueEnum, Clone)]
pub enum Channel {
    Stable,
    Beta,
    Alpha,
}

impl Default for Channel {
    fn default() -> Self {
        Channel::Stable
    }
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

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
pub struct Package {
    pub version: String,
    pub details: VersionDetails,
}

impl Package {
    pub fn get_asset(&self) -> Result<&Asset> {
        let version = &self.version;
        let asset_name = match std::env::consts::OS {
            "linux" => format!(
                "MAA-{}-{}-{}.tar.gz",
                version,
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
            "macos" => format!("MAA-{}-macos-runtime-universal.zip", version),
            "windows" => format!("MAA-{}-{}.zip", version, std::env::consts::ARCH),
            _ => return Err(anyhow!("Unsupported OS")),
        };

        self.details
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or(anyhow!("Asset {} not found", asset_name))
    }
}

/// Get package information of the specified channel from API.
pub fn get_package(channel: &Channel, mirror: Option<String>) -> Result<Package> {
    let api_mirror = arg_env_or_default(
        mirror,
        "MAA_API_MIRROR",
        "https://ota.maa.plus/MaaAssistantArknights/api/version",
    );
    let channel: &str = channel.into();
    let url = format!("{}/{}.json", api_mirror, channel);
    let package: Package = reqwest::blocking::get(&url)?.json()?;
    Ok(package)
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
    pub fn download(&self, dir: &Path) -> Result<Archive> {
        let path = dir.join(&self.name);
        let size = self.size;

        if path.exists() {
            let metadata = path.metadata()?;
            if metadata.len() == size {
                println!("File {} already exists, skip download!", &self.name);
                return Ok(Archive { file: path });
            }
        }

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()?;
        let url = &self.browser_download_url;
        let mirrors = self.mirrors.clone();
        tokio::runtime::Runtime::new()?
            .block_on(download_package(&client, url, mirrors, &path, size))?;

        Ok(Archive { file: path })
    }
}

pub struct Archive {
    file: PathBuf,
}

impl Archive {
    #[cfg(target_os = "macos")]
    pub fn extract(&self, outdir: &Path) -> Result<()> {
        let file = File::open(&self.file)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let re = regex::Regex::new(r"lib.*\.dylib\.?.*").unwrap();
        println!("Extracting files...");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();

            let outpath = match file.enclosed_name() {
                Some(path) if re.is_match(path.to_str().unwrap()) => outdir.join("lib").join(path),
                Some(path) => outdir.join(path),
                None => continue,
            };

            if (*file.name()).ends_with('/') {
                if !outpath.exists() {
                    create_dir_all(&outpath)?;
                }
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        create_dir_all(p)?;
                    }
                }
                if outpath.exists() && file.size() == outpath.metadata()?.len() {
                    continue;
                } else {
                    let mut outfile = File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }

            {
                use std::fs::{set_permissions, Permissions};
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode() {
                    set_permissions(&outpath, Permissions::from_mode(mode))?;
                }
            }
        }

        println!("Done!");

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn extract(&self, outdir: &Path) -> Result<()> {
        let file = File::open(&self.file)?;
        let gz_decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz_decoder);
        let re = regex::Regex::new(r"lib.*\.so\..*").unwrap();

        println!("Extracting files...");

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = match entry.path() {
                Ok(path) if re.is_match(path.to_str().unwrap()) => outdir.join("lib").join(path),
                Ok(path) => outdir.join(path),
                Err(e) => return Err(e.into()),
            };

            if let Some(p) = path.parent() {
                if !p.exists() {
                    create_dir_all(p)?;
                }
            }
            entry.unpack(path)?;
        }

        println!("Done!");

        Ok(())
    }
}
