use super::{
    download::{download, Checker},
    extract::Archive,
    version_json::VersionJSON,
};

use crate::{
    config::cli::{cli_config, maa_cli::CommonArgs},
    consts::{MAA_CLI_EXE, MAA_CLI_VERSION},
    dirs::{self, Ensure},
};

use std::{
    env::{consts, current_exe},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use dunce::canonicalize;
use semver::Version;
use serde::Deserialize;
use tokio::runtime::Runtime;

pub fn version() -> Result<Version> {
    Version::parse(MAA_CLI_VERSION).context("Failed to parse maa-cli version")
}

pub fn update(args: &CommonArgs) -> Result<()> {
    let config = cli_config().cli_config().with_args(args);

    println!("Fetching maa-cli version info...");
    let version_json: VersionJSON<Details> = reqwest::blocking::get(config.api_url())
        .context("Failed to fetch version info")?
        .json()
        .context("Failed to parse version info")?;
    let current_version = version()?;
    if !version_json.can_update("maa-cli", &current_version)? {
        return Ok(());
    }

    let bin_path = canonicalize(current_exe()?)?;
    let details = version_json.details();
    let asset = details.asset()?;
    let asset_name = asset.name();
    let asset_size = asset.size();
    let asset_checksum = asset.checksum();
    let cache_path = dirs::cache().ensure()?.join(asset_name);

    if cache_path.exists() && cache_path.metadata()?.len() == asset_size {
        println!("Found existing file: {}", cache_path.display());
    } else {
        let url = config.download_url(details.tag(), asset_name);
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create reqwest client")?;
        Runtime::new()
            .context("Failed to create tokio runtime")?
            .block_on(download(
                &client,
                &url,
                &cache_path,
                asset_size,
                Some(Checker::Sha256(asset_checksum)),
            ))
            .context("Failed to download maa-cli")?;
    };

    Archive::new(cache_path.into())?.extract(|path| {
        if config.components().binary && path.ends_with(MAA_CLI_EXE) {
            Some(bin_path.clone())
        } else {
            None
        }
    })?;

    Ok(())
}

#[derive(Deserialize)]
struct Details {
    tag: String,
    assets: Assets,
}

impl Details {
    fn tag(&self) -> &str {
        &self.tag
    }

    fn asset(&self) -> Result<&Asset> {
        self.assets.asset()
    }
}

#[derive(Deserialize)]
struct Assets {
    #[serde(rename = "x86_64-apple-darwin")]
    x86_64_apple_darwin: Asset,
    #[serde(rename = "aarch64-apple-darwin")]
    aarch64_apple_darwin: Asset,
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    x86_64_unknown_linux_gnu: Asset,
    #[serde(rename = "aarch64-unknown-linux-gnu")]
    aarch64_unknown_linux_gnu: Asset,
    #[serde(rename = "x86_64-pc-windows-msvc")]
    x86_64_pc_windows_msvc: Asset,
}

impl Assets {
    fn asset(&self) -> Result<&Asset> {
        match consts::OS {
            "macos" => match consts::ARCH {
                "x86_64" => Ok(&self.x86_64_apple_darwin),
                "aarch64" => Ok(&self.aarch64_apple_darwin),
                _ => bail!("Unsupported architecture: {}", consts::ARCH),
            },
            "linux" => match consts::ARCH {
                "x86_64" => Ok(&self.x86_64_unknown_linux_gnu),
                "aarch64" => Ok(&self.aarch64_unknown_linux_gnu),
                _ => bail!("Unsupported architecture: {}", consts::ARCH),
            },
            "windows" if consts::ARCH == "x86_64" => Ok(&self.x86_64_pc_windows_msvc),
            _ => bail!("Unsupported platform: {} {}", consts::OS, consts::ARCH),
        }
    }
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    sha256sum: String,
}

impl Asset {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn checksum(&self) -> &str {
        &self.sha256sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json;

    #[test]
    fn deserialize_version_json() {
        let json = r#"
{
    "version": "0.1.0",
    "details": {
        "tag": "v0.1.0",
        "assets": {
            "x86_64-apple-darwin": {
                "name": "maa-cli.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "aarch64-apple-darwin": {
                "name": "maa-cli.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "x86_64-unknown-linux-gnu": {
                "name": "maa-cli.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "aarch64-unknown-linux-gnu": {
                "name": "maa-cli.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "x86_64-pc-windows-msvc": {
                "name": "maa-cli.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            }
        }
    }
}
        "#;

        let version_json: VersionJSON<Details> = serde_json::from_str(json).unwrap();
        let asset = version_json.details().asset().unwrap();

        assert_eq!(asset.name(), "maa-cli.zip");
        assert_eq!(asset.size(), 123456);
        assert_eq!(
            asset.checksum(),
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }
}
