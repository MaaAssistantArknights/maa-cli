//! Manifest cache implementation using ETag for conditional requests.
//!
//! This module provides caching functionality to avoid re-downloading manifests
//! when they haven't changed, using HTTP ETag headers.
//!
//! Note: The cache does not use file locking or multi-file transactions. Rare
//! concurrent successful downloads may therefore leave a complete body paired
//! with an ETag from another response. This is an accepted cache consistency
//! tradeoff; each individual file replacement remains atomic.

use std::{fs, io, path::Path, time};

use ureq::http::StatusCode;

use crate::error::{Error, ErrorKind, Result, WithDesc};

fn set_modified(path: &Path, modified: time::SystemTime) -> io::Result<()> {
    #[cfg(windows)]
    let file = fs::OpenOptions::new().write(true).open(path)?;
    #[cfg(not(windows))]
    let file = fs::File::open(path)?;

    file.set_modified(modified)
}

pub fn download_with_etag(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    check_interval: Option<time::Duration>,
) -> Result<()> {
    let etag_file = dest.with_added_extension("etag");

    let etag = if dest.exists() && etag_file.exists() {
        let modified = etag_file.metadata().ok().and_then(|m| m.modified().ok());
        if let Some(check_interval) = check_interval
            && let Some(modified) = modified
            && let Ok(duration) = time::SystemTime::now().duration_since(modified)
            && duration < check_interval
        {
            log::trace!("File {} is fresh", dest.display());
            return Ok(());
        }

        fs::read_to_string(&etag_file).ok()
    } else {
        None
    };

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

            maa_atomic_fs::write_with(dest, |file| io::copy(&mut body.into_reader(), file))
                .then_with_desc(|| format!("Failed to update file at {}", dest.display()))?;

            if let Some(etag) = etag {
                log::trace!("Updated ETag {}", etag_file.display());
                maa_atomic_fs::write(&etag_file, etag).then_with_desc(|| {
                    format!("Failed to update ETag at {}", etag_file.display())
                })?;
            } else if let Err(error) = fs::remove_file(&etag_file)
                && error.kind() != io::ErrorKind::NotFound
            {
                return Err(error)
                    .then_with_desc(|| format!("Failed to remove {}", etag_file.display()));
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        path::PathBuf,
        thread,
    };

    use tempfile::{TempDir, tempdir};

    use super::*;

    /// A cached body and its ETag sidecar in a temporary directory.
    struct Cache {
        _dir: TempDir,
        dest: PathBuf,
        etag_file: PathBuf,
    }

    impl Cache {
        fn new(body: &str, etag: &str) -> Self {
            let dir = tempdir().unwrap();
            let dest = dir.path().join("manifest.json");
            let etag_file = dest.with_added_extension("etag");
            fs::write(&dest, body).unwrap();
            fs::write(&etag_file, etag).unwrap();
            Self {
                _dir: dir,
                dest,
                etag_file,
            }
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
}
