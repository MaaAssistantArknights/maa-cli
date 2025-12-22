//! High-level installer that orchestrates the download and extraction process.
//!
//! This module provides a builder-style API for setting up and executing
//! installations with progress reporting.

use std::{
    borrow::Cow,
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use semver::Version;
use ureq::Agent;

use crate::{
    download::{DownloadOptions, download, etag::dwonload_with_etag},
    error::{Result, WithDesc},
    extract::ArchiveFile,
    manifest::{Asset, Manifest},
};

pub struct Installer<'a, M, MP, E> {
    agent: Agent,
    manifest_url: Cow<'a, str>,
    manifest_processor: MP,
    test_duration: u64,
    current_version: Option<&'a Version>,
    extractor: E,
    progress_style: InstallerStyle,
    pre_install_hook: Option<Box<dyn FnOnce() -> Result<()> + 'a>>,
    post_install_hook: Option<Box<dyn FnOnce() -> Result<()> + 'a>>,
    min_check_interval: Option<Duration>,
    _marker: std::marker::PhantomData<M>,
}

pub struct InstallerStyle {
    spinner_style: ProgressStyle,
    bar_style: ProgressStyle,
}

impl Default for InstallerStyle {
    fn default() -> Self {
        const PROGRESS_BAR: &str =
            "{spinner} [{elapsed_precise}] [{bar:40}] {percent}% {bytes}/{total_bytes} ETA {eta}";
        const TICK_CHARS: &str = "⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈✔";
        const PROGRESS_CHARS: &str = "=> ";
        InstallerStyle {
            spinner_style: ProgressStyle::default_spinner().tick_chars(TICK_CHARS),
            bar_style: ProgressStyle::with_template(PROGRESS_BAR)
                .expect("static template string should be valid")
                .progress_chars(PROGRESS_CHARS)
                .tick_chars(TICK_CHARS),
        }
    }
}

impl InstallerStyle {
    pub fn new(spinner_style: ProgressStyle, bar_style: ProgressStyle) -> Self {
        InstallerStyle {
            spinner_style,
            bar_style,
        }
    }

    pub fn init_spinner(&self) -> ProgressBar {
        ProgressBar::no_length().with_style(self.spinner_style.clone())
    }

    pub fn init_bar(&self) -> ProgressBar {
        ProgressBar::no_length().with_style(self.bar_style.clone())
    }
}

impl<'a, M, MP, E> Installer<'a, M, MP, E>
where
    M: Manifest,
    MP: FnOnce(File) -> Result<M>,
    E: FnMut(&Path) -> Option<PathBuf>,
{
    pub fn new(
        agent: Agent,
        manifest_url: impl Into<Cow<'a, str>>,
        manifest_processor: MP,
        extractor: E,
    ) -> Self {
        Self {
            agent,
            manifest_url: manifest_url.into(),
            manifest_processor,
            extractor,
            test_duration: 0,
            current_version: None,
            progress_style: InstallerStyle::default(),
            pre_install_hook: None,
            post_install_hook: None,
            min_check_interval: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_pre_install_hook(mut self, hook: impl FnOnce() -> Result<()> + 'a) -> Self {
        self.pre_install_hook = Some(Box::new(hook));
        self
    }

    pub fn with_post_install_hook(mut self, hook: impl FnOnce() -> Result<()> + 'a) -> Self {
        self.post_install_hook = Some(Box::new(hook));
        self
    }

    pub fn with_progress_style(mut self, style: InstallerStyle) -> Self {
        self.progress_style = style;
        self
    }

    pub fn with_test_duration(mut self, test_duration: u64) -> Self {
        self.test_duration = test_duration;
        self
    }

    pub fn with_current_version(mut self, current_version: &'a Version) -> Self {
        self.current_version = Some(current_version);
        self
    }

    pub fn with_min_check_interval(mut self, interval: Duration) -> Self {
        self.min_check_interval = Some(interval);
        self
    }

    pub fn exec(self, cache_dir: &Path, manifest_name: &str) -> Result<()> {
        let ui = MultiProgress::new();

        let fetching_ui = self.progress_style.init_spinner();
        ui.add(fetching_ui.clone());

        // Fetch and process manifest
        fetching_ui.set_message("Fetching version manifest...");

        let manifest_path = cache_dir.join(manifest_name);
        dwonload_with_etag(
            &self.agent,
            &self.manifest_url,
            &manifest_path,
            self.min_check_interval,
        )
        .with_desc("Failed to fetch version manifest")?;

        let manifest_file = File::open(&manifest_path)?;
        let manifest = (self.manifest_processor)(manifest_file)?;

        // Check if we need update
        if let Some(current_version) = self.current_version {
            if current_version == manifest.version() {
                fetching_ui.finish_with_message("Fetched version manifest, already up-to-date!");
                return Ok(());
            } else {
                fetching_ui.set_message(format!(
                    "Fetched version manifest, update from v{current_version} to v{}",
                    manifest.version()
                ));
            }
        } else {
            fetching_ui.set_message(format!(
                "Fetched version manifest, found v{}",
                manifest.version()
            ));
        }

        // Check if asset exists
        let asset = manifest.asset();
        let asset = if let Some(asset) = asset {
            fetching_ui.finish();
            asset
        } else {
            fetching_ui.finish_with_message("No asset found for current platform");
            return Ok(());
        };

        // Download asset
        let dest = cache_dir.join(asset.name());
        let download_opts =
            DownloadOptions::new(asset.url(), self.test_duration, asset.mirror_opts());
        download(
            &self.agent,
            download_opts,
            &dest,
            ui.clone(),
            &self.progress_style,
            asset.verifier()?,
        )?;

        if let Some(pre_install_hook) = self.pre_install_hook {
            pre_install_hook()?;
        }

        // Extract asset (Install)
        let extract_ui = self.progress_style.init_spinner();
        let archive = ArchiveFile::new(&dest);
        archive.extract(extract_ui.clone(), self.extractor)?;
        if let Some(post_install_hook) = self.post_install_hook {
            post_install_hook()?;
        }
        extract_ui.finish_with_message("Installation completed successfully!");

        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_installer_style() {
        // Test that default style creates progress bars with valid configuration
        let style = InstallerStyle::default();
        let _ = style.init_spinner();
        let _ = style.init_bar();

        // Test custom style produces functional progress bars
        let custom_spinner = ProgressStyle::default_spinner().tick_chars("⠁⠂⠄");
        let custom_bar = ProgressStyle::with_template("{bar} {percent}%")
            .expect("valid template")
            .progress_chars("=> ");
        let _ = InstallerStyle::new(custom_spinner, custom_bar);
        let _ = style.init_spinner();
        let _ = style.init_bar();
    }

    // Test helper types
    struct TestManifest {
        version: Version,
    }

    impl Manifest for TestManifest {
        type Asset<'a>
            = TestAsset
        where
            Self: 'a;

        fn version(&self) -> &Version {
            &self.version
        }

        fn asset(&self) -> Option<Self::Asset<'_>> {
            None
        }
    }

    struct TestAsset;

    impl Asset for TestAsset {
        type Verifier = crate::verify::SizeVerifier;

        fn name(&self) -> &str {
            "test.zip"
        }

        fn url(&self) -> Cow<'_, str> {
            Cow::Borrowed("https://example.com/test.zip")
        }

        fn verifier(&self) -> Result<Self::Verifier> {
            Ok(crate::verify::SizeVerifier::new(1024))
        }
    }

    #[test]
    fn test_installer_builder() {
        use std::{cell::RefCell, rc::Rc};

        let agent = ureq::Agent::new_with_defaults();
        let version_1_2_3 = Version::new(1, 2, 3);
        let version_2_0_0 = Version::new(2, 0, 0);

        // Test default values after construction
        let installer = Installer::new(
            agent.clone(),
            "https://example.com/manifest.json",
            |_body| -> Result<TestManifest> { unreachable!() },
            |_path| None,
        );
        assert_eq!(installer.test_duration, 0);
        assert_eq!(installer.current_version, None);
        assert!(installer.min_check_interval.is_none());

        // Test with_test_duration sets the correct value
        let installer = Installer::new(
            agent.clone(),
            "https://example.com/manifest.json",
            |_body| -> Result<TestManifest> { unreachable!() },
            |_path| None,
        )
        .with_test_duration(10);
        assert_eq!(installer.test_duration, 10);

        // Test with_current_version sets the correct reference
        let installer = Installer::new(
            agent.clone(),
            "https://example.com/manifest.json",
            |_body| -> Result<TestManifest> { unreachable!() },
            |_path| None,
        )
        .with_current_version(&version_1_2_3);
        assert_eq!(installer.current_version, Some(&version_1_2_3));
        assert_eq!(installer.current_version.unwrap(), &Version::new(1, 2, 3));

        // Test that hooks can be set and are not called during construction
        let pre_hook_called = Rc::new(RefCell::new(false));
        let pre_hook_called_clone = pre_hook_called.clone();
        let post_hook_called = Rc::new(RefCell::new(false));
        let post_hook_called_clone = post_hook_called.clone();

        let _installer = Installer::new(
            agent.clone(),
            "https://example.com/manifest.json",
            |_body| -> Result<TestManifest> { unreachable!() },
            |_path| None,
        )
        .with_pre_install_hook(move || {
            *pre_hook_called_clone.borrow_mut() = true;
            Ok(())
        })
        .with_post_install_hook(move || {
            *post_hook_called_clone.borrow_mut() = true;
            Ok(())
        });

        // Hooks should not be called during construction
        assert!(!*pre_hook_called.borrow());
        assert!(!*post_hook_called.borrow());

        // Test method chaining preserves all configured values
        let installer = Installer::new(
            agent,
            "https://example.com/manifest.json",
            |_body| -> Result<TestManifest> { unreachable!() },
            |_path| None,
        )
        .with_test_duration(5)
        .with_current_version(&version_2_0_0)
        .with_progress_style(InstallerStyle::default())
        .with_min_check_interval(Duration::from_secs(300))
        .with_pre_install_hook(|| Ok(()))
        .with_post_install_hook(|| Ok(()));

        assert_eq!(installer.test_duration, 5);
        assert_eq!(installer.current_version, Some(&version_2_0_0));
        assert_eq!(installer.current_version.unwrap(), &Version::new(2, 0, 0));
        assert_eq!(installer.min_check_interval, Some(Duration::from_secs(300)));
    }
}
