//! Manifest cache implementation using ETag for conditional requests.
//!
//! This module provides caching functionality to avoid re-downloading manifests
//! when they haven't changed, using HTTP ETag headers.
//!
//! Writers share a per-destination lock. Each file replacement is atomic.

use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::Path,
    time,
};

use fs2::FileExt;
use serde_json::{Map, Value};
use ureq::http::StatusCode;

use crate::error::{Error, ErrorKind, Result, WithDesc};

fn set_modified(path: &Path, modified: time::SystemTime) -> io::Result<()> {
    #[cfg(windows)]
    let file = fs::OpenOptions::new().write(true).open(path)?;
    #[cfg(not(windows))]
    let file = fs::File::open(path)?;

    file.set_modified(modified)
}

fn is_fresh(dest: &Path, etag_file: &Path, check_interval: Option<time::Duration>) -> bool {
    dest.is_file()
        && etag_file.is_file()
        && check_interval.is_some_and(|interval| {
            etag_file
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age < interval)
        })
}

fn read_etag(dest: &Path, etag_file: &Path) -> Option<String> {
    dest.is_file()
        .then(|| fs::read_to_string(etag_file).ok())
        .flatten()
        .filter(|etag| !etag.is_empty())
}

fn lock_cache(dest: &Path) -> io::Result<fs::File> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dest.with_added_extension("lock"))?;
    file.lock_exclusive()?;
    Ok(file)
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn cache_source(url: &str, fallback: Option<&str>) -> String {
    let mut hasher = DefaultHasher::new();
    (url, fallback).hash(&mut hasher);
    let hash = hasher.finish();
    format!("{hash:016x}")
}

pub fn download_with_etag(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    check_interval: Option<time::Duration>,
) -> Result<()> {
    let _lock = lock_cache(dest)?;
    let etag_file = dest.with_added_extension("etag");

    if is_fresh(dest, &etag_file, check_interval) {
        log::trace!("File {} is fresh", dest.display());
        return Ok(());
    }
    let etag = read_etag(dest, &etag_file);

    let mut request = agent.get(url);
    if let Some(etag) = etag {
        request = request.header("If-None-Match", &etag);
    }
    let response = request.call()?;

    match response.status() {
        StatusCode::OK => {
            log::trace!("Downloaded file {}", dest.display());
            // Split the response so the ETag can borrow the headers while the
            // body is consumed first; the ETag is only written after the body.
            let (parts, body) = response.into_parts();
            let etag = parts.headers.get("ETag").and_then(|v| v.to_str().ok());

            remove_if_exists(&dest.with_added_extension("source"))?;
            maa_atomic_fs::write_with(dest, |file| io::copy(&mut body.into_reader(), file))
                .then_with_desc(|| format!("Failed to update file at {}", dest.display()))?;

            if let Some(etag) = etag {
                log::trace!("Updated ETag {}", etag_file.display());
                maa_atomic_fs::write(&etag_file, etag).then_with_desc(|| {
                    format!("Failed to update ETag at {}", etag_file.display())
                })?;
            } else {
                remove_if_exists(&etag_file)
                    .then_with_desc(|| format!("Failed to remove {}", etag_file.display()))?;
            }

            Ok(())
        }
        StatusCode::NOT_MODIFIED => {
            log::trace!("File {} is up to date", dest.display());
            if set_modified(&etag_file, time::SystemTime::now()).is_ok() {
                log::trace!("Touched {}", etag_file.display());
            }
            Ok(())
        }
        s => Err(Error::new(ErrorKind::Network).with_desc(format!("unexpected status code {s}"))),
    }
}

/// Download a JSON object, optionally retrying a backup source without its ETag.
///
/// A per-destination lock serializes cache checks and body/ETag updates. Backup
/// responses use an empty ETag sidecar to retain freshness without validating
/// the primary source with the backup's ETag. A source fingerprint binds the
/// cache to its configured URLs without storing API credentials.
/// `validate` checks both cached and downloaded JSON objects before use.
pub fn download_json_with_fallback(
    agent: &ureq::Agent,
    url: &str,
    fallback: Option<&str>,
    dest: &Path,
    check_interval: Option<time::Duration>,
    validate: impl Fn(&Map<String, Value>) -> Result<()>,
) -> Result<()> {
    let _lock = lock_cache(dest)?;
    let etag_file = dest.with_added_extension("etag");
    let source_file = dest.with_added_extension("source");
    let source = cache_source(url, fallback);
    let valid_cache = fs::read_to_string(&source_file).is_ok_and(|cached| cached == source)
        && fs::read_to_string(dest).is_ok_and(|body| validate_json(&body, &validate).is_ok());
    if valid_cache && is_fresh(dest, &etag_file, check_interval) {
        return Ok(());
    }
    let etag = valid_cache.then(|| read_etag(dest, &etag_file)).flatten();
    let downloaded = match fetch_json(agent, url, etag.as_deref(), &validate) {
        Ok(downloaded) => downloaded,
        Err(primary) => {
            let Some(fallback) = fallback else {
                return Err(primary);
            };
            log::warn!("Primary hot-update API failed: {primary}; trying backup");
            fetch_json(agent, fallback, None, &validate)
                .map_err(|backup| {
                    let description = format!(
                        "Primary hot-update API failed: {primary}; backup failed: {backup}"
                    );
                    backup.with_desc(description)
                })?
                .map(|(body, _)| (body, None))
        }
    };

    if let Some((body, etag)) = downloaded {
        // Invalidate the old validator before publishing a body from either source.
        remove_if_exists(&source_file)?;
        remove_if_exists(&etag_file)?;
        maa_atomic_fs::write(dest, body)?;
        maa_atomic_fs::write(&etag_file, etag.as_deref().unwrap_or_default())?;
        maa_atomic_fs::write(&source_file, source)?;
    } else {
        // Timestamp refresh is best effort, including on Windows.
        let _ = set_modified(&etag_file, time::SystemTime::now());
    }
    Ok(())
}

fn validate_json(body: &str, validate: &impl Fn(&Map<String, Value>) -> Result<()>) -> Result<()> {
    let json = serde_json::from_str(body).map_err(|error| {
        let line = error.line();
        let column = error.column();
        Error::new(ErrorKind::Other).with_desc(format!(
            "Invalid hot-update JSON at line {line}, column {column}"
        ))
    })?;
    validate(&json)
}

fn request_error(error: ureq::Error) -> Error {
    let description = match error {
        ureq::Error::BadUri(_) | ureq::Error::RequireHttpsOnly(_) => "Invalid request URL".into(),
        error => error.to_string(),
    };
    // URL-bearing errors must not expose custom API credentials, even in their sources.
    Error::new(ErrorKind::Network).with_desc(description)
}

fn fetch_json(
    agent: &ureq::Agent,
    url: &str,
    etag: Option<&str>,
    validate: &impl Fn(&Map<String, Value>) -> Result<()>,
) -> Result<Option<(String, Option<String>)>> {
    let mut request = agent.get(url);
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    let mut response = request
        .config()
        .timeout_global(Some(time::Duration::from_secs(15)))
        .build()
        .call()
        .map_err(request_error)?;
    match response.status() {
        StatusCode::NOT_MODIFIED if etag.is_some() => Ok(None),
        StatusCode::OK => {
            let etag = response
                .headers()
                .get("ETag")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            // ureq limits read_to_string to 10 MiB by default.
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(request_error)?;
            validate_json(&body, validate)?;
            Ok(Some((body, etag)))
        }
        status => Err(Error::new(ErrorKind::Network)
            .with_desc(format!("Unexpected hot-update HTTP status: {status}"))),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        path::PathBuf,
        sync::mpsc,
        thread,
        time::{Duration, SystemTime},
    };

    use tempfile::{TempDir, tempdir};

    use super::*;

    // Fixture setup uses temporary files and loopback servers; failures should fail the test.
    /// A cached body and its ETag sidecar in a temporary directory.
    struct Cache {
        _dir: TempDir,
        dest: PathBuf,
        etag_file: PathBuf,
    }

    impl Cache {
        fn empty() -> Self {
            let dir = tempdir().unwrap();
            let dest = dir.path().join("manifest.json");
            let etag_file = dest.with_added_extension("etag");
            Self {
                _dir: dir,
                dest,
                etag_file,
            }
        }

        fn new(body: &str, etag: &str) -> Self {
            let cache = Self::empty();
            fs::write(&cache.dest, body).unwrap();
            fs::write(&cache.etag_file, etag).unwrap();
            cache
        }

        fn download_json(
            &self,
            url: &str,
            fallback: Option<&str>,
            interval: Option<Duration>,
        ) -> Result<()> {
            download_json_with_fallback(
                &ureq::Agent::new_with_defaults(),
                url,
                fallback,
                &self.dest,
                interval,
                |_| Ok(()),
            )
        }

        fn bind_json_source(&self, url: &str, fallback: Option<&str>) {
            fs::write(
                self.dest.with_added_extension("source"),
                cache_source(url, fallback),
            )
            .unwrap();
        }

        fn download(&self, url: &str, check_interval: Option<time::Duration>) -> Result<()> {
            download_with_etag(
                &ureq::Agent::new_with_defaults(),
                url,
                &self.dest,
                check_interval,
            )
        }

        fn body(&self) -> String {
            fs::read_to_string(&self.dest).unwrap()
        }

        fn etag(&self) -> String {
            fs::read_to_string(&self.etag_file).unwrap()
        }
    }

    fn ok_response(body: &str, etag: Option<&str>) -> Vec<u8> {
        let etag = etag.map_or_else(String::new, |etag| format!("ETag: {etag}\r\n"));
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n{etag}Connection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    /// Serves `response` to a single request and returns the raw request.
    fn serve_once(response: Vec<u8>) -> (String, thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0; 1024];
                let count = stream.read(&mut chunk).unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            stream.write_all(&response).unwrap();
            request
        });
        (format!("http://{address}/manifest"), handle)
    }

    #[test]
    fn successful_download_replaces_body_and_etag() {
        let cache = Cache::new("old manifest", "\"old\"");
        let (url, server) = serve_once(ok_response("new manifest", Some("\"new\"")));

        cache.download(&url, None).unwrap();
        server.join().unwrap();

        assert_eq!(cache.body(), "new manifest");
        assert_eq!(cache.etag(), "\"new\"");
    }

    #[test]
    fn interrupted_download_keeps_body_and_etag() {
        let cache = Cache::new("old manifest", "\"old\"");
        // The header promises 20 bytes but the connection closes after 7.
        let (url, server) = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nETag: \"new\"\r\nConnection: close\r\n\r\npartial"
                .to_vec(),
        );

        let result = cache.download(&url, None);
        server.join().unwrap();

        assert!(result.is_err());
        assert_eq!(cache.body(), "old manifest");
        assert_eq!(cache.etag(), "\"old\"");
    }

    #[test]
    fn body_is_committed_before_etag() {
        let cache = Cache::new("old manifest", "\"old\"");
        // A directory at the ETag path makes only the ETag write fail.
        fs::remove_file(&cache.etag_file).unwrap();
        fs::create_dir(&cache.etag_file).unwrap();
        let (url, server) = serve_once(ok_response("new manifest", Some("\"new\"")));

        let result = cache.download(&url, None);
        server.join().unwrap();

        assert!(result.is_err());
        assert_eq!(cache.body(), "new manifest");
    }

    #[test]
    fn response_without_etag_removes_stale_etag() {
        let cache = Cache::new("old manifest", "\"old\"");
        let (url, server) = serve_once(ok_response("new manifest", None));

        cache.download(&url, None).unwrap();
        server.join().unwrap();

        assert_eq!(cache.body(), "new manifest");
        assert!(!cache.etag_file.exists());
    }

    #[test]
    fn request_sends_cached_etag() {
        let cache = Cache::new("cached manifest", "\"current\"");
        let (url, server) =
            serve_once(b"HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n".to_vec());

        cache.download(&url, None).unwrap();
        let request = String::from_utf8(server.join().unwrap()).unwrap();

        assert!(
            request
                .to_ascii_lowercase()
                .contains("if-none-match: \"current\"\r\n")
        );
    }

    #[test]
    fn not_modified_keeps_cache_and_refreshes_check_time() {
        let cache = Cache::new("cached manifest", "\"current\"");
        // The ETag mtime records the last check; start from a known old time.
        set_modified(&cache.etag_file, time::SystemTime::UNIX_EPOCH).unwrap();
        let (url, server) =
            serve_once(b"HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n".to_vec());

        cache.download(&url, None).unwrap();
        server.join().unwrap();

        assert_eq!(cache.body(), "cached manifest");
        assert_eq!(cache.etag(), "\"current\"");
        let checked = fs::metadata(&cache.etag_file).unwrap().modified().unwrap();
        assert!(checked > time::SystemTime::UNIX_EPOCH);
    }

    #[test]
    fn recent_check_skips_request() {
        // The ETag file was just written, so the last check is within the interval.
        let cache = Cache::new("cached manifest", "\"current\"");

        // No server is running: any request would fail.
        cache
            .download(
                "not a valid URL",
                Some(time::Duration::from_secs(24 * 60 * 60)),
            )
            .unwrap();

        assert_eq!(cache.body(), "cached manifest");
        assert_eq!(cache.etag(), "\"current\"");
    }

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
                        .recv_timeout(Duration::from_secs(5))
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

    #[test]
    fn json_not_modified_preserves_cache_and_refreshes_check_time() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        set_modified(&cache.etag_file, SystemTime::UNIX_EPOCH).unwrap();
        let (url, server) =
            serve_once(b"HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n".to_vec());
        cache.bind_json_source(&url, None);
        cache.download_json(&url, None, None).unwrap();
        let request = String::from_utf8(server.join().unwrap()).unwrap();
        assert!(request.to_lowercase().contains("if-none-match: \"old\""));
        assert_eq!(cache.body(), r#"{"old":true}"#);
        assert_eq!(cache.etag(), "\"old\"");
        assert_ne!(
            cache.etag_file.metadata().unwrap().modified().unwrap(),
            SystemTime::UNIX_EPOCH
        );
    }

    #[test]
    fn fresh_cache_makes_no_requests() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        cache.bind_json_source(
            "http://127.0.0.1:1/primary",
            Some("http://127.0.0.1:1/backup"),
        );
        cache
            .download_json(
                "http://127.0.0.1:1/primary",
                Some("http://127.0.0.1:1/backup"),
                Some(Duration::from_secs(600)),
            )
            .unwrap();
    }

    #[test]
    fn http_504_uses_backup_without_sharing_etags() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let (primary, primary_server) = serve(vec![
            (504, "gateway timeout", None),
            (200, r#"{"primary":true}"#, Some("\"primary\"")),
        ]);
        let (backup, backup_server) = serve(vec![(200, r#"{"backup":true}"#, Some("\"backup\""))]);
        cache.bind_json_source(&primary, Some(&backup));
        cache.download_json(&primary, Some(&backup), None).unwrap();
        assert_eq!(cache.body(), r#"{"backup":true}"#);
        assert_eq!(cache.etag(), "");
        cache
            .download_json(&primary, Some(&backup), Some(Duration::from_secs(600)))
            .unwrap();
        assert!(
            !backup_server.join().unwrap()[0]
                .to_lowercase()
                .contains("if-none-match")
        );
        set_modified(&cache.etag_file, SystemTime::UNIX_EPOCH).unwrap();
        cache
            .download_json(&primary, Some(&backup), Some(Duration::from_secs(600)))
            .unwrap();
        let requests = primary_server.join().unwrap();
        assert!(
            requests[0]
                .to_lowercase()
                .contains("if-none-match: \"old\"")
        );
        assert!(!requests[1].to_lowercase().contains("if-none-match"));
        assert_eq!(cache.body(), r#"{"primary":true}"#);
        assert_eq!(cache.etag(), "\"primary\"");
    }

    #[test]
    fn both_failures_report_both_sources_and_preserve_cache() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let (primary, primary_server) = serve(vec![(504, "timeout", None)]);
        let (backup, backup_server) = serve(vec![(502, "bad gateway", None)]);
        let error = cache
            .download_json(&primary, Some(&backup), None)
            .unwrap_err();
        let message = format!("{error:#}");
        for expected in ["Primary", "backup", "504", "502"] {
            assert!(message.contains(expected), "{message}");
        }
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        assert_eq!(cache.body(), r#"{"old":true}"#);
        assert_eq!(cache.etag(), "\"old\"");
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
            let cache = Cache::empty();
            let (primary, primary_server) = serve(vec![(status, body, None)]);
            let (backup, backup_server) = serve(vec![(200, "{}", None)]);
            cache.download_json(&primary, Some(&backup), None).unwrap();
            assert_eq!(cache.body(), "{}");
            primary_server.join().unwrap();
            backup_server.join().unwrap();
        }
    }

    #[test]
    fn invalid_backup_json_preserves_cache() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let (primary, primary_server) = serve(vec![(504, "timeout", None)]);
        let (backup, backup_server) = serve(vec![(200, "not JSON", Some("\"bad\""))]);
        let error = cache
            .download_json(&primary, Some(&backup), None)
            .unwrap_err();
        assert!(format!("{error:#}").contains("Invalid hot-update JSON"));
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        assert_eq!(cache.body(), r#"{"old":true}"#);
        assert_eq!(cache.etag(), "\"old\"");
    }

    #[test]
    fn interrupted_body_uses_backup() {
        let (primary, primary_server) = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"partial\":"
                .to_vec(),
        );
        let (backup, backup_server) = serve(vec![(200, "{}", None)]);
        let cache = Cache::empty();
        cache.download_json(&primary, Some(&backup), None).unwrap();
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        assert_eq!(cache.body(), "{}");
    }

    #[test]
    fn write_failure_does_not_request_backup() {
        let cache = Cache::empty();
        let (url, server) = serve(vec![(200, "{}", None)]);
        let backup = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let backup_url = format!("http://{}/tasks.json", backup.server_addr());
        fs::create_dir(&cache.dest).unwrap();
        assert!(cache.download_json(&url, Some(&backup_url), None).is_err());
        server.join().unwrap();
        assert!(
            backup
                .recv_timeout(Duration::from_millis(10))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn failed_timestamp_refresh_is_non_fatal() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let etag_file = cache.etag_file.clone();
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/tasks.json", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            fs::remove_file(etag_file).unwrap();
            request.respond(tiny_http::Response::empty(304)).unwrap();
        });
        cache.bind_json_source(&url, None);
        cache.download_json(&url, None, None).unwrap();
        worker.join().unwrap();
        assert_eq!(cache.body(), r#"{"old":true}"#);
    }

    #[test]
    fn missing_cache_metadata_is_a_cache_miss() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        fs::remove_file(&cache.etag_file).unwrap();
        let (url, server) = serve(vec![(200, "{}", None)]);
        cache
            .download_json(&url, None, Some(Duration::from_secs(600)))
            .unwrap();
        server.join().unwrap();
        assert_eq!(cache.body(), "{}");
    }

    #[test]
    fn stalled_primary_times_out_before_the_connection_closes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let primary = format!("http://{}/tasks.json", listener.local_addr().unwrap());
        let (release, hold) = mpsc::channel();
        let primary_server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            hold.recv().unwrap();
        });
        let agent = ureq::Agent::config_builder()
            .timeout_recv_response(Some(Duration::from_millis(100)))
            .build()
            .into();
        let (backup, backup_server) = serve(vec![(200, "{}", None)]);
        let cache = Cache::empty();
        let (done, result) = mpsc::channel();
        let downloader = thread::spawn(move || {
            done.send(download_json_with_fallback(
                &agent,
                &primary,
                Some(&backup),
                &cache.dest,
                None,
                |_| Ok(()),
            ))
            .unwrap();
        });
        let result = result.recv_timeout(Duration::from_secs(2));
        release.send(()).unwrap();
        downloader.join().unwrap();
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        result
            .expect("download must complete before the primary is released")
            .unwrap();
    }

    #[test]
    fn concurrent_updates_share_the_destination_lock_and_recheck_freshness() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let (primary, primary_server) = serve(vec![(504, "timeout", None)]);
        let backup = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let backup_url = format!("http://{}/tasks.json", backup.server_addr());
        cache.bind_json_source(&primary, Some(&backup_url));
        set_modified(&cache.etag_file, SystemTime::UNIX_EPOCH).unwrap();
        let (received, ready) = mpsc::channel();
        let (release, hold) = mpsc::channel();
        let backup_server = thread::spawn(move || {
            let request = backup
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            received.send(()).unwrap();
            hold.recv_timeout(Duration::from_secs(5)).unwrap();
            request
                .respond(tiny_http::Response::from_string(r#"{"backup":true}"#))
                .unwrap();
        });
        let (completed, completion) = mpsc::channel();
        let (started, start) = mpsc::channel();
        thread::scope(|scope| {
            let first = scope.spawn(|| cache.download_json(&primary, Some(&backup_url), None));
            ready.recv_timeout(Duration::from_secs(5)).unwrap();
            let probe = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(cache.dest.with_added_extension("lock"))
                .unwrap();
            assert_eq!(
                probe.try_lock_exclusive().unwrap_err().kind(),
                fs2::lock_contended_error().kind()
            );
            let second = scope.spawn(|| {
                started.send(()).unwrap();
                let result = cache.download_json(
                    &primary,
                    Some(&backup_url),
                    Some(Duration::from_secs(600)),
                );
                completed.send(()).unwrap();
                result
            });
            start.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(completion.recv_timeout(Duration::from_millis(100)).is_err());
            release.send(()).unwrap();
            first.join().unwrap().unwrap();
            second.join().unwrap().unwrap();
        });
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        assert_eq!(cache.body(), r#"{"backup":true}"#);
        assert_eq!(cache.etag(), "");
    }

    #[test]
    fn oversized_primary_response_uses_backup() {
        let primary = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let primary_url = format!("http://{}/tasks.json", primary.server_addr());
        let primary_server = thread::spawn(move || {
            let request = primary
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            let body = format!(r#"{{"data":"{}"}}"#, "x".repeat(10 * 1024 * 1024));
            // The bounded reader may close the connection before the server finishes writing.
            let _ = request.respond(tiny_http::Response::from_string(body));
        });
        let (backup, backup_server) = serve(vec![(200, "{}", None)]);
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        cache
            .download_json(&primary_url, Some(&backup), None)
            .unwrap();
        primary_server.join().unwrap();
        backup_server.join().unwrap();
        assert_eq!(cache.body(), "{}");
    }

    #[test]
    fn custom_api_credentials_are_omitted_from_errors() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let (url, server) = serve(vec![(503, "unavailable", None)]);
        let url = format!("{url}?token=secret-token");
        let error = cache.download_json(&url, None, None).unwrap_err();
        let message = format!("{error:?}");
        assert!(message.contains("503"));
        assert!(!message.contains("secret-token"));
        server.join().unwrap();

        let (url, server) = serve(vec![(
            200,
            r#""http://user:secret-password@example.com/api?token=secret-token""#,
            None,
        )]);
        let error = cache.download_json(&url, None, None).unwrap_err();
        let message = format!("{error:?}");
        assert!(message.contains("Invalid hot-update JSON at line"));
        assert!(!message.contains("secret-password"));
        assert!(!message.contains("secret-token"));
        server.join().unwrap();

        let agent = ureq::Agent::config_builder()
            .https_only(true)
            .build()
            .into();
        let error = download_json_with_fallback(
            &agent,
            "http://user:secret-password@example.com/api?token=secret-token",
            None,
            &cache.dest,
            None,
            |_| Ok(()),
        )
        .unwrap_err();
        let message = format!("{error:?}");
        assert!(message.contains("Invalid request URL"));
        assert!(!message.contains("secret-password"));
        assert!(!message.contains("secret-token"));
    }

    #[test]
    fn invalid_cached_json_is_refetched_even_while_fresh() {
        let cache = Cache::new("invalid JSON", "\"old\"");
        let (url, server) = serve(vec![(200, r#"{"new":true}"#, Some("\"new\""))]);
        cache.bind_json_source(&url, None);
        cache
            .download_json(&url, None, Some(Duration::from_secs(600)))
            .unwrap();
        assert!(
            !server.join().unwrap()[0]
                .to_lowercase()
                .contains("if-none-match")
        );
        assert_eq!(cache.body(), r#"{"new":true}"#);
        assert_eq!(cache.etag(), "\"new\"");
    }

    #[test]
    fn invalid_cached_json_cannot_be_validated_by_304() {
        let cache = Cache::new("invalid JSON", "\"old\"");
        let (primary, primary_server) = serve(vec![(304, "", None)]);
        let (backup, backup_server) = serve(vec![(200, "{}", None)]);
        cache.bind_json_source(&primary, Some(&backup));
        cache.download_json(&primary, Some(&backup), None).unwrap();
        assert!(
            !primary_server.join().unwrap()[0]
                .to_lowercase()
                .contains("if-none-match")
        );
        backup_server.join().unwrap();
        assert_eq!(cache.body(), "{}");
    }

    #[test]
    fn source_changes_invalidate_fresh_cache_and_etag() {
        for etag in [Some("\"old\""), None] {
            let cache = Cache::empty();
            let (old_url, old_server) = serve(vec![(200, r#"{"old":true}"#, etag)]);
            cache.download_json(&old_url, None, None).unwrap();
            old_server.join().unwrap();
            let (new_url, new_server) = serve(vec![(200, r#"{"new":true}"#, None)]);
            let new_url = format!("{new_url}?token=secret-token");
            cache
                .download_json(&new_url, None, Some(Duration::from_secs(600)))
                .unwrap();
            assert!(
                !new_server.join().unwrap()[0]
                    .to_lowercase()
                    .contains("if-none-match")
            );
            assert_eq!(cache.body(), r#"{"new":true}"#);
            assert!(
                !fs::read_to_string(cache.dest.with_added_extension("source"))
                    .unwrap()
                    .contains("secret-token")
            );
        }
    }

    #[test]
    fn legacy_cache_is_revalidated_without_its_etag() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let (url, server) = serve(vec![(200, "{}", None)]);
        cache
            .download_json(&url, None, Some(Duration::from_secs(600)))
            .unwrap();
        assert!(
            !server.join().unwrap()[0]
                .to_lowercase()
                .contains("if-none-match")
        );
        assert_eq!(cache.body(), "{}");
    }

    #[test]
    fn resource_validation_applies_to_cache_and_both_sources() {
        let cache = Cache::new(r#"{"error":{}}"#, "\"old\"");
        let (primary, primary_server) = serve(vec![(200, r#"{"error":{}}"#, None)]);
        let (backup, backup_server) = serve(vec![(200, r#"{"Task":{}}"#, None)]);
        cache.bind_json_source(&primary, Some(&backup));
        download_json_with_fallback(
            &ureq::Agent::new_with_defaults(),
            &primary,
            Some(&backup),
            &cache.dest,
            Some(Duration::from_secs(600)),
            |json| {
                if json.contains_key("Task") {
                    Ok(())
                } else {
                    Err(Error::new(ErrorKind::Other).with_desc("Invalid resource structure"))
                }
            },
        )
        .unwrap();
        assert!(
            !primary_server.join().unwrap()[0]
                .to_lowercase()
                .contains("if-none-match")
        );
        backup_server.join().unwrap();
        assert_eq!(cache.body(), r#"{"Task":{}}"#);
    }

    #[test]
    fn basic_downloader_shares_lock_and_invalidates_json_source() {
        let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
        let (url, server) = serve_once(ok_response("new manifest", None));
        cache.bind_json_source(&url, None);
        let lock = lock_cache(&cache.dest).unwrap();
        let (completed, completion) = mpsc::channel();
        thread::scope(|scope| {
            let worker = scope.spawn(|| {
                let result = cache.download(&url, None);
                completed.send(()).unwrap();
                result
            });
            let blocked = completion.recv_timeout(Duration::from_millis(100)).is_err();
            drop(lock);
            worker.join().unwrap().unwrap();
            assert!(blocked);
        });
        server.join().unwrap();
        assert_eq!(cache.body(), "new manifest");
        assert!(!cache.dest.with_added_extension("source").exists());
    }

    #[test]
    fn invalid_cache_metadata_does_not_publish_or_request_backup() {
        for extension in ["source", "etag"] {
            let cache = Cache::new(r#"{"old":true}"#, "\"old\"");
            let path = cache.dest.with_added_extension(extension);
            remove_if_exists(&path).unwrap();
            fs::create_dir(&path).unwrap();
            let (url, server) = serve(vec![(200, "{}", None)]);
            let backup = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let backup_url = format!("http://{}/tasks.json", backup.server_addr());
            assert!(cache.download_json(&url, Some(&backup_url), None).is_err());
            server.join().unwrap();
            assert_eq!(cache.body(), r#"{"old":true}"#);
            assert!(
                backup
                    .recv_timeout(Duration::from_millis(10))
                    .unwrap()
                    .is_none()
            );
        }
    }
}
