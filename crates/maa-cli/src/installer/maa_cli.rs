use anyhow::{Context, Result};
use maa_dirs::{Ensure, MAA_CLI_EXE};
use maa_installer::{
    error::WithDesc,
    manifest::{Asset, Manifest},
    verify::{SizeVerifier, digest::DigestVerifier},
};
use maa_version::{VersionManifest, cli::Details};
use semver::Version;
use sha2::Sha256;

// use super::reporter::StepReporter;
use crate::{
    config::cli::{CLI_CONFIG, maa_cli::CommonArgs},
    state::{AGENT, CLI_VERSION},
};

const PLATFORM: &str = env!("TARGET");

struct ManifestWithBaseUrl<'a> {
    manifest: maa_version::VersionManifest<maa_version::cli::Details>,
    url: &'a str,
}

struct AssetWithBaseUrl<'a> {
    base_url: &'a str,
    tag: &'a str,
    asset: &'a maa_version::cli::Asset,
}

impl Manifest for ManifestWithBaseUrl<'_> {
    type Asset<'a>
        = AssetWithBaseUrl<'a>
    where
        Self: 'a;

    fn version(&self) -> &Version {
        &self.manifest.version
    }

    fn asset(&self) -> Option<Self::Asset<'_>> {
        self.manifest
            .details
            .assets
            .get(PLATFORM)
            .map(|asset| AssetWithBaseUrl {
                base_url: self.url,
                tag: &self.manifest.details.tag,
                asset,
            })
    }
}

impl Asset for AssetWithBaseUrl<'_> {
    type Verifier = (SizeVerifier, DigestVerifier<Sha256>);

    fn name(&self) -> &str {
        &self.asset.name
    }

    fn url(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Owned(format!(
            "{}/{}/{}",
            self.base_url, self.tag, self.asset.name
        ))
    }

    fn verifier(&self) -> maa_installer::error::Result<Self::Verifier> {
        Ok((
            SizeVerifier::new(self.asset.size),
            DigestVerifier::<Sha256>::from_hex_str(&self.asset.sha256sum)?,
        ))
    }
}

pub fn update(args: &CommonArgs) -> Result<()> {
    let config = CLI_CONFIG.cli_config().with_args(args);

    // Check if binary component should be installed
    if !config.components().binary {
        println!("Binary component is disabled, skipping update");
        return Ok(());
    }

    let url = config.download_url();

    // Create a temp directory for extraction
    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let tmp_exe = tmp_dir.path().join(MAA_CLI_EXE);

    let installer = maa_installer::installer::Installer::new(
        AGENT.clone(),
        config.api_url(),
        |mut body| {
            let manifest: VersionManifest<Details> =
                body.read_json().with_desc("Failed to parse manifest")?;
            Ok(ManifestWithBaseUrl { manifest, url })
        },
        |src| {
            // Extract mapper for maa-cli binary to temp directory
            let file_name = src.file_name()?;
            if file_name == MAA_CLI_EXE {
                Some(tmp_exe.clone())
            } else {
                None
            }
        },
    )
    .with_current_version(&CLI_VERSION)
    .with_post_install_hook(|| {
        // Perform self-replacement
        self_replace::self_replace(&tmp_exe).with_desc("Failed to replace maa-cli binary")
    });

    installer
        .exec(maa_dirs::cache().ensure()?)
        .context("Failed to install maa-cli")?;

    Ok(())
}
