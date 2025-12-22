use std::fs;

use anyhow::Result;
use log::{debug, info};

use crate::{config::cli::CLI_CONFIG, state::AGENT};

pub fn update() -> Result<()> {
    let config = CLI_CONFIG.hot_update_config();

    info!("Updating hot update files...");

    let check_interval = config.check_interval();

    // Activity file and urls
    let activity_file = maa_dirs::activity().to_owned();
    let activity_url = config.activity_url();

    // Resource files
    let resource_files = config.resource_files();
    let resource_urls = config.resource_urls();

    // Download all files parallelly
    std::thread::scope(|s| {
        std::iter::once((activity_file, activity_url))
            .chain(resource_files.zip(resource_urls))
            .map(|(file, url)| {
                s.spawn(move || {
                    if let Some(parent) = file.parent()
                        && !parent.exists()
                    {
                        debug!("Creating parent directory {}", parent.display());
                        fs::create_dir_all(parent)?;
                    }
                    maa_installer::download::etag::download_with_etag(
                        &AGENT,
                        &url,
                        &file,
                        check_interval,
                    )
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().expect("The download should not panic"))
            .collect::<Result<Vec<_>, _>>()
    })?;

    info!("Hot update completed successfully");

    Ok(())
}
