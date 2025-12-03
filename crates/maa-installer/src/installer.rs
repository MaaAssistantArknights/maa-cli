//! High-level installer that orchestrates the download and extraction process.
//!
//! This module provides a builder-style API for setting up and executing
//! installations with progress reporting.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use semver::Version;
use ureq::Agent;

use crate::{
    download::{DownloadOptions, download},
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
    MP: FnOnce(ureq::Body) -> Result<M>,
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

    pub fn exec(self, cache_dir: &Path) -> Result<()> {
        let ui = MultiProgress::new();

        let fetching_ui = self.progress_style.init_spinner();
        ui.add(fetching_ui.clone());

        // Fetch and process manifest
        fetching_ui.set_message("Fetching version manifest...");
        let raw_manifest = self
            .agent
            .get(&*self.manifest_url)
            .call()
            .with_desc("Failed to fetch manifest")?
            .into_body();
        let manifest = (self.manifest_processor)(raw_manifest)?;

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
