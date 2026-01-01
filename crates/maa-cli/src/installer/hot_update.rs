use anyhow::Result;
use log::info;
use maa_dirs::Ensure;
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};

use crate::{config::cli::CLI_CONFIG, state::AGENT};

pub fn update() -> Result<()> {
    let config = CLI_CONFIG.hot_update_config();

    info!("Updating hot update files...");

    let downloads = rayon::iter::once((maa_dirs::activity().to_owned(), config.activity_url()))
        .chain(config.resource_files().zip(config.resource_urls()));

    download_with_etag(&AGENT, downloads, config.check_interval())?;

    info!("Hot update completed successfully");

    Ok(())
}

/// Download multiple files with ETag-based caching.
///
/// Creates parent directories as needed and downloads files in parallel.
pub fn download_with_etag<P, U>(
    agent: &ureq::Agent,
    downloads: impl ParallelIterator<Item = (P, U)>,
    check_interval: Option<std::time::Duration>,
) -> Result<()>
where
    P: AsRef<std::path::Path>,
    U: AsRef<str>,
{
    downloads.try_for_each(|(dest, url)| -> anyhow::Result<()> {
        let dest = dest.as_ref();
        let url = url.as_ref();

        if let Some(parent) = dest.parent() {
            parent.ensure()?;
        }

        maa_installer::download::etag::download_with_etag(agent, url, dest, check_interval)?;

        Ok(())
    })?;

    Ok(())
}
