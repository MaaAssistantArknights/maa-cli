use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result, bail};
use log::{info, warn};
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
        download_file(agent, url, fallback.as_deref(), dest, check_interval)
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

fn download_file(
    agent: &ureq::Agent,
    url: &str,
    fallback: Option<&str>,
    dest: &Path,
    check_interval: Option<Duration>,
) -> Result<()> {
    let etag_file = dest.with_added_extension("etag");
    let etag = if dest.is_file() && etag_file.is_file() {
        if let Some(interval) = check_interval
            && let Ok(modified) = etag_file.metadata()?.modified()
            && modified.elapsed().is_ok_and(|age| age < interval)
        {
            return Ok(());
        }
        fs::read_to_string(&etag_file).ok()
    } else {
        None
    };

    let (downloaded, from_fallback) = match fetch_json(agent, url, etag.as_deref()) {
        Ok(downloaded) => (downloaded, false),
        Err(primary) => {
            let Some(fallback) = fallback else {
                return Err(primary).with_context(|| format!("Hot-update request failed: {url}"));
            };
            warn!("Hot-update request failed at {url}: {primary:#}; trying {fallback}");
            let downloaded = fetch_json(agent, fallback, None).with_context(|| {
                format!("Hot-update backup failed at {fallback}; primary {url} failed: {primary:#}")
            })?;
            (downloaded, true)
        }
    };

    if let Some((body, new_etag)) = downloaded {
        // ETags belong to a URL. A backup response must not validate the primary cache.
        match fs::remove_file(&etag_file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        maa_atomic_fs::write(dest, body)?;
        if !from_fallback && let Some(etag) = new_etag {
            maa_atomic_fs::write(&etag_file, etag)?;
        }
    } else {
        fs::File::open(&etag_file)?.set_modified(SystemTime::now())?;
    }
    Ok(())
}

fn fetch_json(
    agent: &ureq::Agent,
    url: &str,
    etag: Option<&str>,
) -> Result<Option<(String, Option<String>)>> {
    let mut request = agent.get(url);
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    let mut response = request
        .config()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .call()
        .map_err(maa_installer::error::Error::from)?;
    match response.status().as_u16() {
        304 if etag.is_some() => Ok(None),
        200 => {
            let etag = response
                .headers()
                .get("ETag")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(maa_installer::error::Error::from)?;
            let json: serde_json::Value =
                serde_json::from_str(&body).context("Invalid hot-update JSON")?;
            if !json.is_object() {
                bail!("Hot-update JSON must be an object");
            }
            Ok(Some((body, etag)))
        }
        status => bail!("Unexpected hot-update HTTP status: {status}"),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use super::*;

    fn serve(
        responses: Vec<(u16, &str, Option<&str>)>,
    ) -> (String, thread::JoinHandle<Vec<String>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/tasks.json", server.server_addr());
        let responses: Vec<_> = responses
            .into_iter()
            .map(|(status, body, etag)| (status, body.to_owned(), etag.map(str::to_owned)))
            .collect();
        let worker = thread::spawn(move || {
            responses
                .into_iter()
                .map(|(status, body, etag)| {
                    let request = server
                        .recv_timeout(Duration::from_secs(2))
                        .unwrap()
                        .unwrap();
                    let headers = request
                        .headers()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("\n");
                    let mut response =
                        tiny_http::Response::from_string(body).with_status_code(status);
                    if let Some(etag) = etag {
                        response.add_header(tiny_http::Header::from_bytes("ETag", etag).unwrap());
                    }
                    request.respond(response).unwrap();
                    headers
                })
                .collect()
        });
        (url, worker)
    }

    fn download(url: &str, fallback: Option<&str>, dest: &Path) -> Result<()> {
        download_file(&ureq::Agent::new_with_defaults(), url, fallback, dest, None)
    }

    fn cache() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("tasks.json"), r#"{"old":true}"#).unwrap();
        fs::write(temp.path().join("tasks.json.etag"), "\"old\"").unwrap();
        temp
    }

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
    fn primary_success_and_304_preserve_etag_caching() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("tasks.json");
        let (url, server) = serve(vec![
            (200, r#"{"new":true}"#, Some("\"new\"")),
            (304, "", None),
        ]);
        let backup = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let backup_url = format!("http://{}/tasks.json", backup.server_addr());
        for attempt in 0..2 {
            if attempt == 1 {
                fs::File::open(dest.with_added_extension("etag"))
                    .unwrap()
                    .set_modified(SystemTime::UNIX_EPOCH)
                    .unwrap();
            }
            download(&url, Some(&backup_url), &dest).unwrap();
        }
        assert_ne!(
            dest.with_added_extension("etag")
                .metadata()
                .unwrap()
                .modified()
                .unwrap(),
            SystemTime::UNIX_EPOCH
        );
        let requests = server.join().unwrap();
        assert!(!requests[0].to_lowercase().contains("if-none-match"));
        assert!(
            requests[1]
                .to_lowercase()
                .contains("if-none-match: \"new\"")
        );
        assert!(
            backup
                .recv_timeout(Duration::from_millis(10))
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read_to_string(&dest).unwrap(), r#"{"new":true}"#);
        assert_eq!(
            fs::read_to_string(dest.with_added_extension("etag")).unwrap(),
            "\"new\""
        );
    }

    #[test]
    fn fresh_cache_makes_no_requests() {
        let temp = cache();
        download_file(
            &ureq::Agent::new_with_defaults(),
            "http://127.0.0.1:1/primary",
            Some("http://127.0.0.1:1/backup"),
            &temp.path().join("tasks.json"),
            Some(Duration::from_secs(600)),
        )
        .unwrap();
    }

    #[test]
    fn http_504_uses_backup_without_sharing_etags() {
        let temp = cache();
        let dest = temp.path().join("tasks.json");
        let (primary, primary_server) = serve(vec![
            (504, "gateway timeout", None),
            (200, r#"{"primary":true}"#, Some("\"primary\"")),
        ]);
        let (backup, backup_server) = serve(vec![(200, r#"{"backup":true}"#, Some("\"backup\""))]);
        download(&primary, Some(&backup), &dest).unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), r#"{"backup":true}"#);
        assert!(!dest.with_added_extension("etag").exists());
        assert!(
            !backup_server.join().unwrap()[0]
                .to_lowercase()
                .contains("if-none-match")
        );
        download(&primary, Some(&backup), &dest).unwrap();
        let requests = primary_server.join().unwrap();
        assert!(
            requests[0]
                .to_lowercase()
                .contains("if-none-match: \"old\"")
        );
        assert!(!requests[1].to_lowercase().contains("if-none-match"));
        assert_eq!(fs::read_to_string(&dest).unwrap(), r#"{"primary":true}"#);
    }

    #[test]
    fn both_failures_report_both_sources_and_preserve_cache() {
        let temp = cache();
        let dest = temp.path().join("tasks.json");
        let (primary, primary_server) = serve(vec![(504, "timeout", None)]);
        let (backup, backup_server) = serve(vec![(502, "bad gateway", None)]);
        let error = download(&primary, Some(&backup), &dest).unwrap_err();
        let message = format!("{error:#}");
        for expected in [&primary, &backup, "504", "502"] {
            assert!(message.contains(expected), "{message}");
        }
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), r#"{"old":true}"#);
        assert_eq!(
            fs::read_to_string(dest.with_added_extension("etag")).unwrap(),
            "\"old\""
        );
    }

    #[test]
    fn unusable_primary_response_uses_backup() {
        for (status, body) in [
            (200, "not JSON"),
            (200, ""),
            (200, "[]"),
            (304, ""),
            (204, ""),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let (primary, primary_server) = serve(vec![(status, body, None)]);
            let (backup, backup_server) = serve(vec![(200, "{}", None)]);
            let dest = temp.path().join("tasks.json");
            download(&primary, Some(&backup), &dest).unwrap();
            assert_eq!(fs::read_to_string(dest).unwrap(), "{}");
            primary_server.join().unwrap();
            backup_server.join().unwrap();
        }
    }

    #[test]
    fn invalid_backup_json_preserves_cache() {
        let temp = cache();
        let dest = temp.path().join("tasks.json");
        let (primary, primary_server) = serve(vec![(504, "timeout", None)]);
        let (backup, backup_server) = serve(vec![(200, "not JSON", Some("\"bad\""))]);
        let error = download(&primary, Some(&backup), &dest).unwrap_err();
        assert!(format!("{error:#}").contains("Invalid hot-update JSON"));
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), r#"{"old":true}"#);
        assert_eq!(
            fs::read_to_string(dest.with_added_extension("etag")).unwrap(),
            "\"old\""
        );
    }

    #[test]
    fn stalled_primary_uses_backup() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let primary = format!("http://{}/tasks.json", listener.local_addr().unwrap());
        let primary_server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(300));
        });
        let agent = ureq::Agent::config_builder()
            .timeout_recv_response(Some(Duration::from_millis(100)))
            .build()
            .into();
        let (backup, backup_server) = serve(vec![(200, "{}", None)]);
        let temp = tempfile::tempdir().unwrap();
        download_file(
            &agent,
            &primary,
            Some(&backup),
            &temp.path().join("tasks.json"),
            None,
        )
        .unwrap();
        primary_server.join().unwrap();
        backup_server.join().unwrap();
    }

    #[test]
    fn interrupted_body_uses_backup() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let primary = format!("http://{}/tasks.json", listener.local_addr().unwrap());
        let primary_server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
            }
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"partial\":").unwrap();
        });
        let (backup, backup_server) = serve(vec![(200, "{}", None)]);
        let temp = tempfile::tempdir().unwrap();
        download(&primary, Some(&backup), &temp.path().join("tasks.json")).unwrap();
        primary_server.join().unwrap();
        backup_server.join().unwrap();
    }

    #[test]
    fn custom_source_failure_is_returned() {
        let temp = cache();
        let (url, server) = serve(vec![(503, "unavailable", None)]);
        let error = download(&url, None, &temp.path().join("tasks.json")).unwrap_err();
        assert!(format!("{error:#}").contains("503"));
        server.join().unwrap();
    }

    #[test]
    fn write_failure_does_not_request_backup() {
        let temp = tempfile::tempdir().unwrap();
        let (url, server) = serve(vec![(200, "{}", None)]);
        let backup = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let backup_url = format!("http://{}/tasks.json", backup.server_addr());
        assert!(
            download_file(
                &ureq::Agent::new_with_defaults(),
                &url,
                Some(&backup_url),
                temp.path(),
                None
            )
            .is_err()
        );
        server.join().unwrap();
        assert!(
            backup
                .recv_timeout(Duration::from_millis(10))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    #[ignore = "requires access to the official primary and backup APIs"]
    fn official_api_responses_are_valid_json() {
        for base in [DEFAULT_API_URL, FALLBACK_API_URL] {
            let url = format!("{base}/gui/StageActivityV2.json");
            assert!(fetch_json(&AGENT, &url, None).unwrap().is_some());
        }
    }
}
