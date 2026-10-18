// This file is used to download and extract prebuilt packages of maa-core.

use std::{
    env::consts::{ARCH, DLL_PREFIX, DLL_SUFFIX, OS},
    path::{self, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use maa_dirs::{self, Ensure, MAA_CORE_LIB};
use maa_installer::{
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

/// GitHub Release 的下载前缀，用于拼接不在 version API 里的资产地址。
const CORE_RELEASE_DOWNLOAD_BASE: &str =
    "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download";

/// 单个版本的 core 组件包。
///
/// 桌面平台的包在 MAA 的 version API 里有条目（带镜像与尺寸）；Android 组件包只发在
/// GitHub Release 上，只能按命名规则拼地址，因此这里区分两种情况。
struct CoreAsset<'a> {
    /// 清单里列出的资产；按命名规则合成时为 None。
    listed: Option<&'a core::Asset>,
    name: String,
    url: String,
}

impl<'a> CoreAsset<'a> {
    fn from_listed(asset: &'a core::Asset) -> Self {
        Self {
            listed: Some(asset),
            name: asset.name.clone(),
            url: asset.browser_download_url.clone(),
        }
    }

    fn synthesized(version: &Version, name: String) -> Self {
        let url = format!("{CORE_RELEASE_DOWNLOAD_BASE}/v{version}/{name}");
        Self {
            listed: None,
            name,
            url,
        }
    }
}

impl CoreManifest {
    fn from_reader(file: std::fs::File) -> maa_installer::error::Result<Self> {
        use maa_installer::error::{Error, ErrorKind};
        let manifest: VersionManifest<core::Details> =
            serde_json::from_reader(file).map_err(|e| {
                Error::new(ErrorKind::Other)
                    .with_source(e)
                    .with_desc("Failed to parse core version manifest")
            })?;

        Ok(CoreManifest(manifest))
    }

    fn get_asset(&self, os: &str, arch: &str) -> Option<CoreAsset<'_>> {
        let asset_name = asset_name(self.version(), os, arch);
        if let Some(asset) = self
            .0
            .details
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
        {
            return Some(CoreAsset::from_listed(asset));
        }
        // Android 组件包只发布在 GitHub Release，version API 的资产列表里没有条目，
        // 按命名规则拼出地址即可；镜像与尺寸校验这两项在合成资产上不适用。
        (os == "android").then(|| CoreAsset::synthesized(self.version(), asset_name))
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
    type Verifier = Option<SizeVerifier>;

    fn name(&self) -> &str {
        &self.name
    }

    fn url(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.url)
    }

    fn mirror_opts(
        &self,
    ) -> Option<
        maa_installer::manifest::MirrorOptions<'_, impl Iterator<Item = std::borrow::Cow<'_, str>>>,
    > {
        let listed = self.listed?;
        Some(maa_installer::manifest::MirrorOptions::new(
            listed
                .mirrors
                .iter()
                .map(|m| std::borrow::Cow::Borrowed(m.as_str())),
            listed.size / 10,
        ))
    }

    fn verifier(&self) -> maa_installer::error::Result<Self::Verifier> {
        // 合成资产拿不到官方尺寸，只能不做校验；下载完整性由解包阶段的格式检查兜住。
        Ok(self.listed.map(|asset| SizeVerifier::new(asset.size)))
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
        // Android 组件包用 arm64 / x64 命名，与 Rust 的 aarch64 / x86_64 不同名
        "android" => match arch {
            "aarch64" => format!("MAAComponent-v{version}-android-arm64.tar.gz"),
            "x86_64" => format!("MAAComponent-v{version}-android-x64.tar.gz"),
            _ => panic!("Unsupported architecture: {arch}"),
        },
        _ => panic!("Unsupported OS: {os}"),
    }
}

/// Android 组件包里随附、但命令行侧用不到的库。
///
/// - `libMaaAndroidNativeControlUnit.so`：Android 原生控制单元，maa-cli 通过 adb 连接游戏，走不到它
/// - `libfastdeploy_ppocr.so`：桌面 OCR 后端，Android 用 ncnn 权重（由包内 ncnn overlay 提供）
const SKIPPED_LIBS: [&str; 2] = [
    "libMaaAndroidNativeControlUnit.so",
    "libfastdeploy_ppocr.so",
];

/// 是否跳过这个库。
///
/// 只在 Android 上跳过：桌面平台的 MaaCore 依赖 `libfastdeploy_ppocr.so` 做 OCR，
/// 少装它会让内核连加载都失败。
fn should_skip_lib(name: &str) -> bool {
    cfg!(target_os = "android") && SKIPPED_LIBS.contains(&name)
}

fn extract_mapper(
    src: &Path,
    lib_dir: &Path,
    resource_dir: &Path,
    config: &Components,
) -> Option<PathBuf> {
    // debug!("Extracting file: {}", src.display());
    if src
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(should_skip_lib)
    {
        return None;
    }
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
        CoreManifest::from_reader,
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

    // Android 的 OCR 走 ncnn，而 MAA 分发的 resource 只带 onnx；权重由打包阶段预转好随包分发，
    // 装完资源后叠加上去，否则识别直接不可用。
    #[cfg(target_os = "android")]
    let installer = if components.resource {
        installer.with_post_install_hook(|| apply_ncnn_overlay(resource_dir))
    } else {
        installer
    };

    installer
        .exec(
            maa_dirs::cache().ensure()?,
            &format!("core-manifest-{}.json", config.channel()),
        )
        .context("Failed to install MaaCore")?;

    Ok(())
}

/// 随包分发的 ncnn 权重目录，与 resource 平级。
#[cfg(any(target_os = "android", test))]
const NCNN_OVERLAY_DIR: &str = "ncnn-overlay";

/// 把 resource 目录 `version.json` 里的 `last_updated` 当作这份资源的身份戳。
///
/// MAA 的 resource 不写版本号，只在 `version.json` 里记活动与 `last_updated`；
/// ncnn 权重必须与生成它的那份 onnx 同源，所以用这个时间戳做配对依据。
#[cfg(any(target_os = "android", test))]
fn read_resource_stamp(dir: &Path) -> std::io::Result<Option<String>> {
    let path = dir.join("version.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(std::io::Error::other)?;
    Ok(value
        .get("last_updated")
        .and_then(|updated| updated.as_str())
        .map(str::to_owned))
}

/// 只叠加 ncnn 权重（`<name>.ncnn.param` / `<name>.ncnn.bin`），overlay 里的其余文件（含
/// `version.json`）不落地。
#[cfg(any(target_os = "android", test))]
fn copy_ncnn_weights(src_root: &Path, dst_root: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src_root)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst_root.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&to)?;
            copy_ncnn_weights(&from, &to)?;
        } else if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.contains(".ncnn."))
        {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Android 专用：把随包分发的 ncnn OCR 权重叠加到已安装的 resource 上。
///
/// 两份资源的 `last_updated` 不一致说明 overlay 与当前 resource 不同源（例如用户刚更新过 core），
/// 此时跳过并提示，宁可不套权重也不把错配的 ncnn 盖到新的 onnx 上。
#[cfg(any(target_os = "android", test))]
fn apply_ncnn_overlay(resource_dir: &Path) -> maa_installer::error::Result<()> {
    let overlay_dir = resource_dir.with_file_name(NCNN_OVERLAY_DIR);
    if !overlay_dir.is_dir() {
        return Ok(());
    }

    let overlay_stamp = read_resource_stamp(&overlay_dir)?;
    let resource_stamp = read_resource_stamp(resource_dir)?;
    if overlay_stamp.is_none() || overlay_stamp != resource_stamp {
        eprintln!(
            "Skip ncnn overlay: resource mismatch (overlay {overlay_stamp:?}, installed {resource_stamp:?}).\n\
             OCR needs matching ncnn weights; reinstall the matching maa-cli package to restore it."
        );
        return Ok(());
    }

    copy_ncnn_weights(&overlay_dir, resource_dir)?;
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
    // Android 的 OCR 依赖随包发布的 ncnn 权重，更新 resource 会让权重与新的 onnx 脱配，
    // 因此只更新 core 时要显式加 --no-resource。
    #[cfg(target_os = "android")]
    if components.resource {
        eprintln!(
            "Warning: updating resource replaces the bundled ncnn OCR weights; \
             pass --no-resource to update MaaCore only."
        );
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

        // 桌面必须照装 fastdeploy（MaaCore 靠它做 OCR），只有 Android 才跳过
        let desktop_ocr = format!("{DLL_PREFIX}fastdeploy_ppocr{DLL_SUFFIX}");
        let expected = if should_skip_lib("libfastdeploy_ppocr.so") {
            None
        } else {
            Some(lib_dir.join(&desktop_ocr))
        };
        assert_eq!(
            extract_mapper(Path::new(&desktop_ocr), lib_dir, resource_dir, &config),
            expected
        );
    }

    #[test]
    fn test_should_skip_lib() {
        let on_android = cfg!(target_os = "android");
        for name in SKIPPED_LIBS {
            assert_eq!(should_skip_lib(name), on_android, "{name}");
        }
        assert!(!should_skip_lib(MAA_CORE_LIB));
    }

    #[test]
    fn test_apply_ncnn_overlay() {
        let root = tempfile::tempdir().unwrap();
        let resource_dir = root.path().join("resource");
        let overlay_dir = root.path().join(NCNN_OVERLAY_DIR);
        let stamp = r#"{"last_updated": "2026-10-16 11:38:15.000"}"#;

        std::fs::create_dir_all(resource_dir.join("ocr/det")).unwrap();
        std::fs::write(resource_dir.join("version.json"), stamp).unwrap();
        std::fs::create_dir_all(overlay_dir.join("ocr/det")).unwrap();
        std::fs::write(overlay_dir.join("version.json"), stamp).unwrap();
        std::fs::write(overlay_dir.join("ocr/det/det.ncnn.param"), b"param").unwrap();
        std::fs::write(overlay_dir.join("ocr/det/det.ncnn.bin"), b"bin").unwrap();

        apply_ncnn_overlay(&resource_dir).unwrap();

        assert!(resource_dir.join("ocr/det/det.ncnn.param").is_file());
        assert!(resource_dir.join("ocr/det/det.ncnn.bin").is_file());
        // 只有权重落地，overlay 自身的 version.json 不能混进 resource
        assert_eq!(
            std::fs::read_to_string(resource_dir.join("version.json")).unwrap(),
            stamp
        );
    }

    #[test]
    fn test_apply_ncnn_overlay_skips_mismatched_resource() {
        let root = tempfile::tempdir().unwrap();
        let resource_dir = root.path().join("resource");
        let overlay_dir = root.path().join(NCNN_OVERLAY_DIR);

        std::fs::create_dir_all(&resource_dir).unwrap();
        std::fs::write(
            resource_dir.join("version.json"),
            r#"{"last_updated": "2026-10-17 00:00:00.000"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(&overlay_dir).unwrap();
        std::fs::write(
            overlay_dir.join("version.json"),
            r#"{"last_updated": "2026-10-16 11:38:15.000"}"#,
        )
        .unwrap();
        std::fs::write(overlay_dir.join("det.ncnn.param"), b"param").unwrap();

        apply_ncnn_overlay(&resource_dir).unwrap();

        assert!(!resource_dir.join("det.ncnn.param").exists());
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
        fn test_android() {
            let version = Version::new(1, 2, 3);
            assert_eq!(
                asset_name(&version, "android", "aarch64"),
                "MAAComponent-v1.2.3-android-arm64.tar.gz"
            );
            assert_eq!(
                asset_name(&version, "android", "x86_64"),
                "MAAComponent-v1.2.3-android-x64.tar.gz"
            );
        }

        #[test]
        #[should_panic(expected = "Unsupported architecture")]
        fn test_android_unsupported_arch() {
            let version = Version::new(1, 2, 3);
            asset_name(&version, "android", "armv7");
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

    const FIXTURE_JSON: &str = include_str!("../../fixtures/version/core_version.json");
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
        fn test_android_asset_is_synthesized() {
            // version API 不收录 Android 组件包，只能按命名规则拼 GitHub Release 地址
            let asset = MANIFEST
                .get_asset("android", "x86_64")
                .expect("android asset");
            assert_eq!(asset.name(), "MAAComponent-v4.26.1-android-x64.tar.gz");
            assert_eq!(
                asset.url(),
                "https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/v4.26.1/MAAComponent-v4.26.1-android-x64.tar.gz"
            );
            // 合成资产没有镜像，也拿不到官方尺寸
            assert!(asset.mirror_opts().is_none());
            assert!(asset.verifier().expect("verifier").is_none());
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

        #[test]
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        fn test_asset_linux_x86_64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "MAA-v4.26.1-linux-x86_64.tar.gz");
            assert!(asset.url().contains("linux-x86_64.tar.gz"));
            assert!(asset.verifier().is_ok());
            assert!(asset.mirror_opts().is_some());
        }

        #[test]
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        fn test_asset_linux_aarch64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "MAA-v4.26.1-linux-aarch64.tar.gz");
            assert!(asset.url().contains("linux-aarch64.tar.gz"));
            assert!(asset.verifier().is_ok());
            assert!(asset.mirror_opts().is_some());
        }

        #[test]
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        fn test_asset_windows_x86_64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "MAA-v4.26.1-win-x64.zip");
            assert!(asset.url().contains("win-x64.zip"));
            assert!(asset.verifier().is_ok());
            assert!(asset.mirror_opts().is_some());
        }

        #[test]
        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
        fn test_asset_windows_aarch64() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "MAA-v4.26.1-win-arm64.zip");
            assert!(asset.url().contains("win-arm64.zip"));
            assert!(asset.verifier().is_ok());
            assert!(asset.mirror_opts().is_some());
        }

        #[test]
        #[cfg(target_os = "macos")]
        fn test_asset_macos() {
            let asset = MANIFEST
                .asset()
                .expect("Asset should exist for current platform");
            assert_eq!(asset.name(), "MAA-v4.26.1-macos-runtime-universal.zip");
            assert!(asset.url().contains("macos-runtime-universal.zip"));
            assert!(asset.verifier().is_ok());
            assert!(asset.mirror_opts().is_some());
        }
    }
}
