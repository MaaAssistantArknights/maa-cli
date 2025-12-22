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

impl ManifestWithBaseUrl<'_> {
    fn get_asset(&self, platform: &str) -> Option<AssetWithBaseUrl<'_>> {
        self.manifest
            .details
            .assets
            .get(platform)
            .map(|asset| AssetWithBaseUrl {
                base_url: self.url,
                tag: &self.manifest.details.tag,
                asset,
            })
    }
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
        self.get_asset(PLATFORM)
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
        |file| {
            use maa_installer::error::{Error, ErrorKind};
            let manifest: VersionManifest<Details> =
                serde_json::from_reader(file).map_err(|e| {
                    Error::new(ErrorKind::Other)
                        .with_source(e)
                        .with_desc("Failed to parse manifest")
                })?;
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
        .exec(
            maa_dirs::cache().ensure()?,
            &format!("cli-manifest-{}.json", config.channel()),
        )
        .context("Failed to install maa-cli")?;

    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    const FIXTURE_JSON: &str = include_str!("../../fixtures/cli_version.json");
    const BASE_URL: &str = "https://github.com/MaaAssistantArknights/maa-cli/releases/download";
    static MANIFEST: LazyLock<ManifestWithBaseUrl<'static>> = LazyLock::new(|| {
        let manifest: VersionManifest<Details> =
            serde_json::from_str(FIXTURE_JSON).expect("Failed to parse fixture");
        ManifestWithBaseUrl {
            manifest,
            url: BASE_URL,
        }
    });

    #[test]
    fn test_manifest() {
        assert_eq!(MANIFEST.version(), &Version::new(0, 5, 9));
        assert_eq!(MANIFEST.version().to_string(), "0.5.9");

        assert_eq!(MANIFEST.manifest.details.tag, "v0.5.9");
        assert_eq!(
            MANIFEST.manifest.details.commit,
            "f4e2418415b5cbf10d1d8e01514971c72f58cb50"
        );
        assert_eq!(MANIFEST.manifest.details.assets.len(), 7);
    }

    mod asset_tests {
        use super::*;

        fn get_test_asset(platform: &str) -> AssetWithBaseUrl<'static> {
            MANIFEST.get_asset(platform).expect("Asset not found")
        }

        #[test]
        fn test_url() {
            let asset = get_test_asset("x86_64-unknown-linux-gnu");
            assert_eq!(
                asset.url(),
                "https://github.com/MaaAssistantArknights/maa-cli/releases/download/v0.5.9/maa_cli-v0.5.9-x86_64-unknown-linux-gnu.tar.gz"
            );
        }

        #[test]
        fn test_different_platforms() {
            let platforms = [
                (
                    "x86_64-unknown-linux-gnu",
                    "maa_cli-v0.5.9-x86_64-unknown-linux-gnu.tar.gz",
                    5121236,
                    "f7bf07df03275b64018d789aabaa2628d062f9a6e56b7770589c6c6c1363f3b7",
                ),
                (
                    "aarch64-unknown-linux-gnu",
                    "maa_cli-v0.5.9-aarch64-unknown-linux-gnu.tar.gz",
                    5301507,
                    "6080419c2b3e09539bdabb04b0e7bcd5ee7fb93abd4e53cbddf87229285b7881",
                ),
                (
                    "x86_64-pc-windows-msvc",
                    "maa_cli-v0.5.9-x86_64-pc-windows-msvc.zip",
                    3215593,
                    "df1be3fbe297988f4fb27d1253c650e09beb0b1b330ce587a0bf7e5f7903fbad",
                ),
                (
                    "aarch64-pc-windows-msvc",
                    "maa_cli-v0.5.9-aarch64-pc-windows-msvc.zip",
                    3006906,
                    "0839ec03b0baff11142a9653af3dcbc58a4f4b28b9071e55f9f4d2cf9e7eac45",
                ),
                (
                    "universal-apple-darwin",
                    "maa_cli-v0.5.9-universal-apple-darwin.zip",
                    8692204,
                    "a0a2aee6e01d2c60dc1be6295c3ba4eb7aeeecdd03e27072e15afdb5c8f69453",
                ),
                (
                    "x86_64-apple-darwin",
                    "maa_cli-v0.5.9-x86_64-apple-darwin.zip",
                    4290539,
                    "4f77b84ef54db52373e420409e58b6300dc0b4b7babeb839675932b0e32bcb5b",
                ),
                (
                    "aarch64-apple-darwin",
                    "maa_cli-v0.5.9-aarch64-apple-darwin.zip",
                    4401174,
                    "19d90f7dda10ef28b6b9388862a4fcf647d83afb282d9d3103acff4144e287e2",
                ),
            ];

            for (platform, expected_name, expected_size, expected_sha256) in platforms {
                let asset = get_test_asset(platform);

                // Test name
                assert_eq!(asset.name(), expected_name);

                // Test size
                assert_eq!(asset.asset.size, expected_size);

                // Test sha256sum
                assert_eq!(asset.asset.sha256sum, expected_sha256);

                // Test verifier can be created (validates hex parsing)
                assert!(asset.verifier().is_ok());

                // Verify URL contains the asset name
                let url = asset.url();
                assert!(url.contains(expected_name));
            }
        }

        #[test]
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        fn test_asset_linux_x86_64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(
                asset.name(),
                "maa_cli-v0.5.9-x86_64-unknown-linux-gnu.tar.gz"
            );
            assert!(asset.url().contains("x86_64-unknown-linux-gnu.tar.gz"));
            assert!(asset.verifier().is_ok());
        }

        #[test]
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        fn test_asset_linux_aarch64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(
                asset.name(),
                "maa_cli-v0.5.9-aarch64-unknown-linux-gnu.tar.gz"
            );
            assert!(asset.url().contains("aarch64-unknown-linux-gnu.tar.gz"));
            assert!(asset.verifier().is_ok());
        }

        #[test]
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        fn test_asset_windows_x86_64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "maa_cli-v0.5.9-x86_64-pc-windows-msvc.zip");
            assert!(asset.url().contains("x86_64-pc-windows-msvc.zip"));
            assert!(asset.verifier().is_ok());
        }

        #[test]
        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
        fn test_asset_windows_aarch64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "maa_cli-v0.5.9-aarch64-pc-windows-msvc.zip");
            assert!(asset.url().contains("aarch64-pc-windows-msvc.zip"));
            assert!(asset.verifier().is_ok());
        }

        #[test]
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        fn test_asset_macos_x86_64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "maa_cli-v0.5.9-x86_64-apple-darwin.zip");
            assert!(asset.url().contains("x86_64-apple-darwin.zip"));
            assert!(asset.verifier().is_ok());
        }

        #[test]
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        fn test_asset_macos_aarch64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "maa_cli-v0.5.9-aarch64-apple-darwin.zip");
            assert!(asset.url().contains("aarch64-apple-darwin.zip"));
            assert!(asset.verifier().is_ok());
        }
    }
}
