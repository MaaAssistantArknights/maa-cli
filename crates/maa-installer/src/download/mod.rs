mod download_impl;
pub mod etag;
pub mod mirror;

use std::{borrow::Cow, path::Path, time::Duration};

use indicatif::MultiProgress;
pub use mirror::fastest_mirror;

use crate::{
    error::{ErrorKind, Result},
    installer::InstallerStyle,
    manifest::MirrorOptions,
    verify::Verifier,
};

pub struct DownloadOptions<'a, I: Iterator<Item = Cow<'a, str>>> {
    url: Cow<'a, str>,
    display_url: Option<Cow<'a, str>>,
    test_duration: u64,
    mirror_opts: Option<MirrorOptions<'a, I>>,
}

impl<'a, I: Iterator<Item = Cow<'a, str>>> DownloadOptions<'a, I> {
    pub fn new(
        url: Cow<'a, str>,
        test_duration: u64,
        mirror_opts: Option<MirrorOptions<'a, I>>,
    ) -> Self {
        Self {
            url,
            display_url: None,
            test_duration,
            mirror_opts,
        }
    }

    /// Use a label instead of displaying a potentially credential-bearing URL.
    pub fn with_display_url(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.display_url = Some(label.into());
        self
    }
}

/// Download a file to given path
///
/// If any mirrors are provided, it will perform a speed test to choose the fastest mirror.
/// If no mirrors are provided or the speed test is skipped, it will download from the default URL.
///
/// Progress bar is optional.
pub fn download<'a, V: Verifier>(
    agent: &ureq::Agent,
    opts: DownloadOptions<'a, impl Iterator<Item = Cow<'a, str>>>,
    dest: &Path,
    style: &InstallerStyle,
    mut verifier: V,
) -> Result<()> {
    let ui = MultiProgress::new();
    let download_main_ui = style.init_spinner();
    ui.add(download_main_ui.clone());

    if check_file_exists(dest, &mut verifier)? {
        download_main_ui.finish_with_message("File already exists and verified, skipping download");
        return Ok(());
    }

    let url = opts.url;
    let chosen_url = match (opts.test_duration, opts.mirror_opts) {
        (0, _) | (_, None) => url,
        (t, Some(opts)) => {
            download_main_ui.set_message("Speed testing to find fastest mirror...");
            let choose_mirror_ui = style.init_spinner();
            ui.add(choose_mirror_ui.clone());

            let max_time = Duration::from_secs(t);
            let url = fastest_mirror(agent, url, max_time, opts, choose_mirror_ui.clone());
            choose_mirror_ui.finish_and_clear();
            url
        }
    };

    let display_url = opts.display_url.as_deref().unwrap_or(chosen_url.as_ref());
    download_main_ui.set_message(format!("Downloading: {display_url}"));

    let download_progress_ui = style.init_bar();
    ui.add(download_progress_ui.clone());
    download_impl::download(
        agent,
        chosen_url.as_ref(),
        dest,
        download_progress_ui.clone(),
        verifier,
    )?;
    download_progress_ui.finish_and_clear();
    download_main_ui.finish_with_message("Download complete");

    Ok(())
}

/// Check if a file exists and verifies its integrity using the provided verifier.
fn check_file_exists<V: Verifier>(path: &Path, verifier: &mut V) -> Result<bool> {
    if path.exists() && path.is_file() {
        match verifier.verify_file(path) {
            Ok(()) => {
                log::debug!("File already exists and verified, skipping download");
                Ok(true)
            }
            Err(e) if e.kind() == ErrorKind::Verify => {
                log::warn!("File found but verification failed: {e}");
                Ok(false)
            }
            Err(e) => Err(e),
        }
    } else {
        Ok(false)
    }
}
