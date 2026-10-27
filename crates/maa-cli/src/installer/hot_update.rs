use anyhow::{Context, Result};
use log::info;
use maa_dirs::Ensure;
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};

use crate::{
    config::cli::{CLI_CONFIG, hot_update::DEFAULT_API_URL},
    state::AGENT,
};

const FALLBACK_API_URL: &str = "https://api2.maa.plus/MaaAssistantArknights/api";

pub fn update() -> Result<()> {
    let config = CLI_CONFIG.hot_update_config();

    info!("Updating hot update files...");

    let downloads = rayon::iter::once((maa_dirs::activity().to_owned(), config.activity_url()))
        .chain(config.resource_files().zip(config.resource_urls()));

    download_with_etag(&AGENT, downloads, config.check_interval(), config.api_url())?;

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
    api_url: &str,
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

        let fallback = fallback_url(api_url, url);
        maa_installer::download::etag::download_json_with_fallback(
            agent,
            url,
            fallback.as_deref(),
            dest,
            check_interval,
        )
        .with_context(|| format!("Failed to update hot-update file {}", dest.display()))?;

        Ok(())
    })?;

    Ok(())
}

fn fallback_url(api_url: &str, url: &str) -> Option<String> {
    if api_url != DEFAULT_API_URL {
        return None;
    }
    let path = url.strip_prefix(DEFAULT_API_URL)?.strip_prefix('/')?;
    Some(format!("{FALLBACK_API_URL}/{path}"))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn only_official_api_paths_have_a_backup() {
        let config = crate::config::cli::hot_update::Config::default();
        for url in
            std::iter::once(config.activity_url()).chain(config.resource_urls().collect::<Vec<_>>())
        {
            assert_eq!(
                fallback_url(config.api_url(), &url),
                Some(url.replacen(DEFAULT_API_URL, FALLBACK_API_URL, 1))
            );
        }
        for url in [
            "https://custom.example/api/tasks.json",
            "https://api2.maa.plus/MaaAssistantArknights/api/resource/tasks.json",
            "https://api.maa.plus/MaaAssistantArknights/api-other/tasks.json",
        ] {
            assert_eq!(fallback_url(DEFAULT_API_URL, url), None);
        }
        let custom_api = format!("{DEFAULT_API_URL}/custom");
        assert_eq!(
            fallback_url(
                &custom_api,
                &format!("{custom_api}/gui/StageActivityV2.json")
            ),
            None
        );
    }

    #[test]
    #[ignore = "requires access to the official primary and backup APIs"]
    fn official_api_responses_are_valid_json() {
        for base in [DEFAULT_API_URL, FALLBACK_API_URL] {
            let temp = tempfile::tempdir().unwrap();
            let url = format!("{base}/gui/StageActivityV2.json");
            maa_installer::download::etag::download_json_with_fallback(
                &AGENT,
                &url,
                None,
                &temp.path().join("activity.json"),
                None,
            )
            .unwrap();
        }
    }
}
