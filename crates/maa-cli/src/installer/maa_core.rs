// This file is used to download and extract prebuilt packages of maa-core.

use std::{
    env::consts::{ARCH, DLL_PREFIX, DLL_SUFFIX, OS},
    path::{self, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use maa_dirs::{self, Ensure, MAA_CORE_LIB};
use maa_installer::{
    error::WithDesc,
    manifest::{Asset, Manifest},
    verify::SizeVerifier,
};
use maa_version::{VersionManifest, core};
use semver::Version;

// use super::reporter::StepReporter;
use crate::{
    config::cli::{
        CLI_CONFIG,
        maa_core::{CommonArgs, Components},
    },
    state::CORE_VERSION,
};

struct CoreManifest(VersionManifest<core::Details>);

struct CoreAsset<'a>(&'a core::Asset);

impl CoreManifest {
    fn from_body(mut body: ureq::Body) -> maa_installer::error::Result<Self> {
        let manifest = body.read_json().with_desc("Failed to parse manifest")?;

        Ok(CoreManifest(manifest))
    }

    fn get_asset(&self, os: &str, arch: &str) -> Option<CoreAsset<'_>> {
        let asset_name = asset_name(self.version(), os, arch);
        self.0
            .details
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .map(CoreAsset)
    }
}

impl Manifest for CoreManifest {
    type Asset<'a>
        = CoreAsset<'a>
    where
        Self: 'a;

    fn version(&self) -> &Version {
        &self.0.version
    }

    fn asset(&self) -> Option<Self::Asset<'_>> {
        self.get_asset(OS, ARCH)
    }
}

impl Asset for CoreAsset<'_> {
    type Verifier = SizeVerifier;

    fn name(&self) -> &str {
        &self.0.name
    }

    fn url(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.0.browser_download_url)
    }

    fn mirror_opts(
        &self,
    ) -> Option<
        maa_installer::manifest::MirrorOptions<'_, impl Iterator<Item = std::borrow::Cow<'_, str>>>,
    > {
        Some(maa_installer::manifest::MirrorOptions::new(
            self.0
                .mirrors
                .iter()
                .map(|m| std::borrow::Cow::Borrowed(m.as_str())),
            self.0.size / 10,
        ))
    }

    fn verifier(&self) -> maa_installer::error::Result<Self::Verifier> {
        Ok(SizeVerifier::new(self.0.size))
    }
}

/// Get the name of the asset for the current platform
pub(crate) fn this_asset_name(version: &Version) -> String {
    asset_name(version, OS, ARCH)
}

/// Get the name of the asset for given version, OS, and architecture
///
/// # Panics
///
/// This function panics if the OS or architecture is not supported.
fn asset_name(version: &Version, os: &str, arch: &str) -> String {
    // Once panic it means a bug or running on unsupported platform.
    match os {
        "macos" => format!("MAA-v{version}-macos-runtime-universal.zip"),
        "linux" => match arch {
            "x86_64" => format!("MAA-v{version}-linux-x86_64.tar.gz"),
            "aarch64" => format!("MAA-v{version}-linux-aarch64.tar.gz"),
            _ => panic!("Unsupported architecture: {arch}"),
        },
        "windows" => match arch {
            "x86_64" => format!("MAA-v{version}-win-x64.zip"),
            "aarch64" => format!("MAA-v{version}-win-arm64.zip"),
            _ => panic!("Unsupported architecture: {arch}"),
        },
        _ => panic!("Unsupported OS: {os}"),
    }
}

fn extract_mapper(
    src: &Path,
    lib_dir: &Path,
    resource_dir: &Path,
    config: &Components,
) -> Option<PathBuf> {
    // debug!("Extracting file: {}", src.display());
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
                }
                if config.library && c
                    .to_str() // The DLL suffix may not the last part of the file name
                    .is_some_and(|s| s.starts_with(DLL_PREFIX) && s.contains(DLL_SUFFIX))
                {
                    let dest = lib_dir.join(src.file_name()?);
                    return Some(dest);
                }
            }
            _ => continue,
        }
    }
    None
}

fn create_and_exec_installer(args: &CommonArgs, current_version: Option<&Version>) -> Result<()> {
    let config = CLI_CONFIG.core_config().apply_args(args);
    let lib_dir = maa_dirs::library();
    let resource_dir = maa_dirs::resource();
    let components = config.components();

    let installer = maa_installer::installer::Installer::new(
        crate::state::AGENT.clone(),
        config.api_url(),
        CoreManifest::from_body,
        |src| extract_mapper(src, lib_dir, resource_dir, components),
    )
    .with_test_duration(config.test_time())
    .with_pre_install_hook(move || {
        if components.library {
            lib_dir.ensure_clean()?;
        }
        if components.resource {
            resource_dir.ensure_clean()?;
        }
        Ok(())
    });

    let installer = if let Some(version) = current_version {
        installer.with_current_version(version)
    } else {
        installer
    };

    installer
        .exec(maa_dirs::cache().ensure()?)
        .context("Failed to install MaaCore")?;

    Ok(())
}

pub fn install(force: bool, args: &CommonArgs) -> Result<()> {
    let lib_dir = maa_dirs::library();
    let lib_name = MAA_CORE_LIB;

    if lib_dir.join(lib_name).exists() && !force {
        bail!(
            "MaaCore already exists, use `maa update` to update it or `maa install --force` to force reinstall"
        )
    }

    create_and_exec_installer(args, None)
}

pub fn update(args: &CommonArgs) -> Result<()> {
    let config = CLI_CONFIG.core_config().apply_args(args);

    let components = config.components();
    // Check if any component is specified
    if !(components.library || components.resource) {
        bail!("No component specified, aborting");
    }
    // Check if MaaCore is installed and installed by maa
    let lib_dir = maa_dirs::library();
    let resource_dir = maa_dirs::resource();
    if components.library
        && let Some(dir) = maa_dirs::find_library()
        && dir != lib_dir
    {
        bail!(
            "MaaCore found at {} but not installed by maa, aborting",
            dir.display()
        )
    }
    if components.resource
        && let Some(dir) = maa_dirs::find_resource()
        && dir != resource_dir
    {
        bail!(
            "MaaCore resource found at {} but not installed by maa, aborting",
            dir.display()
        )
    }

    create_and_exec_installer(args, CORE_VERSION.as_ref())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::LazyLock;

    use maa_dirs::MAA_CORE_LIB;

    use super::*;

    #[test]
    fn test_extract_mapper() {
        let config = Components::default();
        let lib_dir = Path::new("/home/user/.local/share/maa/lib");
        let resource_dir = Path::new("/home/user/.local/share/maa/resource");

        assert_eq!(
            extract_mapper(Path::new(MAA_CORE_LIB), lib_dir, resource_dir, &config),
            Some(lib_dir.join(MAA_CORE_LIB))
        );

        assert_eq!(
            extract_mapper(
                &Path::new("resource").join("config.json"),
                lib_dir,
                resource_dir,
                &config
            ),
            Some(resource_dir.join("config.json"))
        );
        assert_eq!(
            extract_mapper(Path::new("misc"), lib_dir, resource_dir, &config),
            None
        );
    }

    mod asset_name_tests {
        use super::*;

        #[test]
        fn test_macos() {
            let version = Version::new(1, 2, 3);
            assert_eq!(
                asset_name(&version, "macos", "x86_64"),
                "MAA-v1.2.3-macos-runtime-universal.zip"
            );
            assert_eq!(
                asset_name(&version, "macos", "aarch64"),
                "MAA-v1.2.3-macos-runtime-universal.zip"
            );
        }

        #[test]
        fn test_linux() {
            let version = Version::new(1, 2, 3);
            assert_eq!(
                asset_name(&version, "linux", "x86_64"),
                "MAA-v1.2.3-linux-x86_64.tar.gz"
            );
            assert_eq!(
                asset_name(&version, "linux", "aarch64"),
                "MAA-v1.2.3-linux-aarch64.tar.gz"
            );
        }

        #[test]
        #[should_panic(expected = "Unsupported architecture")]
        fn test_linux_unsupported_arch() {
            let version = Version::new(1, 2, 3);
            asset_name(&version, "linux", "armv7");
        }

        #[test]
        fn test_windows() {
            let version = Version::new(1, 2, 3);
            assert_eq!(
                asset_name(&version, "windows", "x86_64"),
                "MAA-v1.2.3-win-x64.zip"
            );
            assert_eq!(
                asset_name(&version, "windows", "aarch64"),
                "MAA-v1.2.3-win-arm64.zip"
            );
        }

        #[test]
        #[should_panic(expected = "Unsupported architecture")]
        fn test_windows_unsupported_arch() {
            let version = Version::new(1, 2, 3);
            asset_name(&version, "windows", "i686");
        }

        #[test]
        #[should_panic(expected = "Unsupported OS")]
        fn test_unsupported_os() {
            let version = Version::new(1, 2, 3);
            asset_name(&version, "freebsd", "x86_64");
        }

        #[test]
        fn test_with_prerelease() {
            let version = Version::parse("1.2.3-beta.1").unwrap();
            assert_eq!(
                asset_name(&version, "linux", "x86_64"),
                "MAA-v1.2.3-beta.1-linux-x86_64.tar.gz"
            );
        }

        #[test]
        fn test_this_asset_name() {
            let version = Version::new(1, 2, 3);
            let result = this_asset_name(&version);
            // Just verify it returns a non-empty string and contains the version
            assert!(!result.is_empty());
            assert!(result.contains("1.2.3"));
        }
    }

    const FIXTURE_JSON: &str = include_str!("../../fixtures/core_version.json");
    static MANIFEST: LazyLock<CoreManifest> = LazyLock::new(|| {
        let version_manifest: VersionManifest<core::Details> =
            serde_json::from_str(FIXTURE_JSON).expect("Failed to parse fixture");
        CoreManifest(version_manifest)
    });

    #[test]
    fn test_manifest() {
        assert_eq!(MANIFEST.version(), &Version::new(4, 26, 1));
        assert_eq!(MANIFEST.version().to_string(), "4.26.1");

        assert_eq!(MANIFEST.0.details.assets.len(), 5);
    }

    mod core_asset {
        use super::*;

        fn get_test_asset(os: &str, arch: &str) -> CoreAsset<'static> {
            MANIFEST.get_asset(os, arch).expect("Asset not found")
        }

        #[test]
        fn test_url() {
            let asset = get_test_asset("linux", "x86_64");
            assert_eq!(
                asset.url(),
                "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAA-v4.26.1-linux-x86_64.tar.gz"
            );
        }

        #[test]
        fn test_mirror_opts() {
            let asset = get_test_asset("linux", "x86_64");
            let mirror_opts = asset.mirror_opts();

            assert!(mirror_opts.is_some());
            let mirror_opts = mirror_opts.unwrap();
            let mirrors: Vec<_> = mirror_opts.mirrors.collect();
            assert_eq!(mirrors.len(), 3);
            assert!(mirrors[0].contains("s3.maa-org.net"));
            assert!(mirrors[1].contains("agent.imgg.dev"));
            assert!(mirrors[2].contains("maa.r2.imgg.dev"));
        }

        #[test]
        fn test_different_platforms() {
            let platforms = [
                ("linux", "x86_64", "MAA-v4.26.1-linux-x86_64.tar.gz"),
                ("linux", "aarch64", "MAA-v4.26.1-linux-aarch64.tar.gz"),
                ("windows", "x86_64", "MAA-v4.26.1-win-x64.zip"),
                ("windows", "aarch64", "MAA-v4.26.1-win-arm64.zip"),
                ("macos", "x86_64", "MAA-v4.26.1-macos-runtime-universal.zip"),
                (
                    "macos",
                    "aarch64",
                    "MAA-v4.26.1-macos-runtime-universal.zip",
                ),
            ];

            for (os, arch, expected_name) in platforms {
                let asset = get_test_asset(os, arch);

                assert_eq!(asset.name(), expected_name);
                assert!(asset.verifier().is_ok());
                assert!(asset.mirror_opts().is_some());
                let url = asset.url();
                assert!(url.contains(expected_name));
            }
        }
    }
}
