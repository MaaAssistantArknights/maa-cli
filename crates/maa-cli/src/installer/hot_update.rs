use anyhow::{Context, Result};
use log::info;
use maa_dirs::Ensure;
use maa_installer::error::{Error, ErrorKind};
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};
use serde_json::{Map, Value};

use crate::{
    activity::StageActivityJson,
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
            |json| validate_hot_update(json, dest == maa_dirs::activity()),
        )
        .with_context(|| format!("Failed to update hot-update file {}", dest.display()))?;

        Ok(())
    })?;

    Ok(())
}

fn validate_hot_update(
    json: &Map<String, Value>,
    activity: bool,
) -> maa_installer::error::Result<()> {
    let invalid =
        || Error::new(ErrorKind::Other).with_desc("Invalid hot-update resource structure");
    if activity {
        if json.is_empty() {
            return Err(invalid());
        }
        serde_json::from_value::<StageActivityJson>(Value::Object(json.clone()))
            .map_err(|_| invalid())?;
    } else if !json.values().all(Value::is_object) {
        return Err(invalid());
    }
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
    use std::time::Duration;

    use super::*;

    // Known JSON fixtures and temporary HTTP servers must initialize successfully.

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
    fn activity_validation_uses_the_existing_model_and_rejects_empty_metadata() {
        let activity = serde_json::from_str(include_str!("../../fixtures/activity.json")).unwrap();
        assert!(validate_hot_update(&activity, true).is_ok());
        for body in [
            "{}",
            r#"{"error":"http://user:secret-token@example.com"}"#,
            r#"{"Official":{}}"#,
        ] {
            let json = serde_json::from_str(body).unwrap();
            let error = validate_hot_update(&json, true).unwrap_err();
            assert!(!format!("{error:?}").contains("secret-token"));
        }
        for offset in [24, -24, i32::MAX, i32::MIN] {
            let mut invalid = Value::Object(activity.clone());
            invalid["Official"]["resourceCollection"]["TimeZone"] = Value::from(offset);
            assert!(validate_hot_update(invalid.as_object().unwrap(), true).is_err());
        }
    }

    #[test]
    fn task_patches_are_object_maps_and_can_be_empty() {
        for body in ["{}", r#"{"Task":{"next":["Next"]}}"#] {
            assert!(validate_hot_update(&serde_json::from_str(body).unwrap(), false).is_ok());
        }
        for body in [
            r#"{"error":"unavailable"}"#,
            r#"{"Task":[]}"#,
            r#"{"Task":null}"#,
        ] {
            assert!(validate_hot_update(&serde_json::from_str(body).unwrap(), false).is_err());
        }
    }

    #[test]
    fn cli_download_creates_parents_and_reports_destination_on_failure() {
        for status in [200, 503] {
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let url = format!("http://{}/tasks.json", server.server_addr());
            let worker = std::thread::spawn(move || {
                let request = server
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                    .unwrap();
                request
                    .respond(tiny_http::Response::from_string("{}").with_status_code(status))
                    .unwrap();
            });
            let temp = tempfile::tempdir().unwrap();
            let dest = temp.path().join("tasks/tasks.json");
            let result = download_with_etag(
                &ureq::Agent::new_with_defaults(),
                rayon::iter::once((&dest, &url)),
                None,
                &url,
            );
            worker.join().unwrap();
            if status == 200 {
                result.unwrap();
                assert_eq!(std::fs::read_to_string(&dest).unwrap(), "{}");
            } else {
                let error = result.unwrap_err();
                let message = format!("{error:#}");
                assert!(message.contains(&dest.display().to_string()));
                assert!(message.contains("503"));
            }
        }
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
                |json| validate_hot_update(json, true),
            )
            .unwrap();
        }
    }
}
