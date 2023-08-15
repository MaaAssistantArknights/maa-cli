mod download;
use download::download_package;

use std::env::var_os;
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use clap::ValueEnum;
use serde::Deserialize;

#[derive(ValueEnum, Clone, Default)]
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
pub fn get_package(channel: &Channel) -> Result<Package> {
    let api_url = if let Some(url) = var_os("MAA_API_URL") {
        url.to_str().unwrap().to_owned()
    } else {
        "https://ota.maa.plus/MaaAssistantArknights/api/version".to_owned()
    };
    let channel: &str = channel.into();
    let url = format!("{}/{}.json", api_url, channel);
    let package: Package = reqwest::blocking::get(url)?.json()?;
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
    pub fn download(&self, dir: &Path, t: u64) -> Result<ArchiveFile> {
        let path = dir.join(&self.name);
        let size = self.size;

        if path.exists() {
            let metadata = path.metadata()?;
            if metadata.len() == size {
                println!("File {} already exists, skip download!", &self.name);
                return Ok(ArchiveFile { file: path });
            }
        }

        let url = &self.browser_download_url;
        let mirrors = self.mirrors.clone();
        download_package(url, mirrors, &path, size, t)?;

        Ok(ArchiveFile { file: path })
    }
}

pub struct ArchiveFile {
    file: PathBuf,
}

impl ArchiveFile {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn extract(&self, outdir: &Path, resource: bool) -> Result<()> {
        let file = File::open(&self.file)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let re_lib = if cfg!(target_os = "macos") {
            regex::Regex::new(r"lib.*\.dylib\.?.*").unwrap()
        } else {
            regex::Regex::new(r".*\.dll").unwrap()
        };
        let re_resource = regex::Regex::new(r"resource/.*").unwrap();
        println!("Extracting files...");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();

            let outpath = match file.enclosed_name() {
                Some(path) if re_lib.is_match(path.to_str().unwrap()) => {
                    outdir.join("lib").join(path)
                }
                Some(path) if resource && re_resource.is_match(path.to_str().unwrap()) => {
                    outdir.join(path)
                }
                Some(_) => continue,
                None => continue,
            };

            if file.is_dir() {
                continue;
            }

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

            #[cfg(target_os = "macos")]
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
    pub fn extract(&self, outdir: &Path, resource: bool) -> Result<()> {
        let file = File::open(&self.file)?;
        let gz_decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz_decoder);
        let re_so = regex::Regex::new(r"lib.*\.so\.?.*").unwrap();
        let re_resource = regex::Regex::new(r"resource/.*").unwrap();

        println!("Extracting files...");

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = match entry.path() {
                Ok(path) if re_so.is_match(path.to_str().unwrap()) => outdir.join("lib").join(path),
                Ok(path) if resource && re_resource.is_match(path.to_str().unwrap()) => {
                    outdir.join(path)
                }
                Ok(_) => continue,
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
