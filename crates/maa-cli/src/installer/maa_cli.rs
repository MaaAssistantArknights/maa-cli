use std::{
    collections::BTreeMap,
    env::{consts, current_exe},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use dunce::canonicalize;
use semver::Version;
use serde::Deserialize;
use tokio::runtime::Runtime;

use super::{
    download::{download, Checker},
    extract::Archive,
    version_json::VersionJSON,
};
use crate::{
    config::cli::{maa_cli::CommonArgs, CLI_CONFIG},
    dirs::{self, Ensure},
};

pub fn update(args: &CommonArgs) -> Result<()> {
    let config = CLI_CONFIG.cli_config().with_args(args);

    println!("Fetching maa-cli version info...");
    let version_json: VersionJSON<Details> = reqwest::blocking::get(config.api_url())
        .context("Failed to fetch version info")?
        .json()
        .context("Failed to parse version info")?;
    let current_version: Version = env!("MAA_VERSION").parse()?;
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

    let cli_exe = format!("maa{}", consts::EXE_SUFFIX);
    Archive::new(cache_path.into())?.extract(|path| {
        if config.components().binary && path.ends_with(&cli_exe) {
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
struct Assets(BTreeMap<String, Asset>);

const PLATFORM: &str = env!("TARGET");

impl Assets {
    fn asset(&self) -> Result<&Asset> {
        self.0
            .get(PLATFORM)
            .ok_or_else(|| anyhow!("No asset for platform: {}", PLATFORM))
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
    use serde_json;

    use super::*;

    #[test]
    fn deserialize_version_json() {
        let json = r#"
{
    "version": "0.1.0",
    "details": {
        "tag": "v0.1.0",
        "assets": {
            "x86_64-apple-darwin": {
                "name": "maa_cli-0.1.0-x86_64-apple-darwin.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "aarch64-apple-darwin": {
                "name": "maa_cli-0.1.0-aarch64-apple-darwin.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "x86_64-unknown-linux-gnu": {
                "name": "maa_cli-0.1.0-x86_64-unknown-linux-gnu.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "aarch64-unknown-linux-gnu": {
                "name": "maa_cli-0.1.0-aarch64-unknown-linux-gnu.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "x86_64-pc-windows-msvc": {
                "name": "maa_cli-0.1.0-x86_64-pc-windows-msvc.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            },
            "aarch64-pc-windows-msvc": {
                "name": "maa_cli-0.1.0-aarch64-pc-windows-msvc.zip",
                "size": 123456,
                "sha256sum": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            }
        }
    }
}
        "#;

        let version_json: VersionJSON<Details> = serde_json::from_str(json).unwrap();
        let asset = version_json.details().asset().unwrap();

        assert_eq!(asset.name(), format!("maa_cli-0.1.0-{}.zip", PLATFORM));
        assert_eq!(asset.size(), 123456);
        assert_eq!(
            asset.checksum(),
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }
}
