//! Manifest cache implementation using ETag for conditional requests.
//!
//! This module provides caching functionality to avoid re-downloading manifests
//! when they haven't changed, using HTTP ETag headers.
//!
//! Note: The cache does not use file locking for simplicity and performance.
//! In rare concurrent write scenarios, some ETag updates may be lost, which is
//! acceptable as the cache will be refreshed on the next check.

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
            let etag = response
                .headers()
                .get("ETag")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);

            maa_atomic_fs::write_with(dest, |file| {
                io::copy(&mut response.into_body().as_reader(), file)
            })
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
                log::trace!("Touched {}", dest.display());
            }
            Ok(())
        }
        s => Err(Error::new(ErrorKind::Network).with_desc(format!("unexpected status code {s}"))),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use tempfile::tempdir;

    use super::*;

    fn serve_once(response: &'static [u8]) -> (String, thread::JoinHandle<Vec<u8>>) {
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
            stream.write_all(response).unwrap();
            request
        });
        (format!("http://{address}/manifest"), handle)
    }

    #[test]
    fn interrupted_download_preserves_cached_file_and_etag() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path().join("manifest.json");
        let etag_file = dest.with_added_extension("etag");
        fs::write(&dest, "old manifest").unwrap();
        fs::write(&etag_file, "\"old\"").unwrap();
        let (url, server) = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nETag: \"new\"\r\nConnection: close\r\n\r\npartial",
        );

        let result = download_with_etag(&ureq::Agent::new_with_defaults(), &url, &dest, None);
        drop(server.join().unwrap());

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "old manifest");
        assert_eq!(fs::read_to_string(&etag_file).unwrap(), "\"old\"");
    }

    #[test]
    fn successful_download_replaces_cached_file_and_etag() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path().join("manifest.json");
        let etag_file = dest.with_added_extension("etag");
        fs::write(&dest, "old manifest").unwrap();
        fs::write(&etag_file, "\"old\"").unwrap();
        let (url, server) = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nETag: \"new\"\r\nConnection: close\r\n\r\nnew manifest",
        );

        download_with_etag(&ureq::Agent::new_with_defaults(), &url, &dest, None).unwrap();
        drop(server.join().unwrap());

        assert_eq!(fs::read_to_string(&dest).unwrap(), "new manifest");
        assert_eq!(fs::read_to_string(&etag_file).unwrap(), "\"new\"");
    }

    #[test]
    fn etag_failure_occurs_after_file_commit() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path().join("manifest.json");
        let etag_file = dest.with_added_extension("etag");
        fs::write(&dest, "old manifest").unwrap();
        fs::create_dir(&etag_file).unwrap();
        let (url, server) = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nETag: \"new\"\r\nConnection: close\r\n\r\nnew manifest",
        );

        let result = download_with_etag(&ureq::Agent::new_with_defaults(), &url, &dest, None);
        drop(server.join().unwrap());

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new manifest");
    }

    #[test]
    fn response_without_etag_removes_stale_etag() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path().join("manifest.json");
        let etag_file = dest.with_added_extension("etag");
        fs::write(&dest, "old manifest").unwrap();
        fs::write(&etag_file, "\"old\"").unwrap();
        let (url, server) = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\nnew manifest",
        );

        download_with_etag(&ureq::Agent::new_with_defaults(), &url, &dest, None).unwrap();
        drop(server.join().unwrap());

        assert_eq!(fs::read_to_string(&dest).unwrap(), "new manifest");
        assert!(!etag_file.exists());
    }

    #[test]
    fn not_modified_uses_etag_preserves_cache_and_refreshes_timestamp() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path().join("manifest.json");
        let etag_file = dest.with_added_extension("etag");
        fs::write(&dest, "cached manifest").unwrap();
        fs::write(&etag_file, "\"current\"").unwrap();
        let old_modified = time::SystemTime::UNIX_EPOCH;
        set_modified(&etag_file, old_modified).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&etag_file, fs::Permissions::from_mode(0o444)).unwrap();
        let (url, server) = serve_once(b"HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n");

        download_with_etag(&ureq::Agent::new_with_defaults(), &url, &dest, None).unwrap();
        let request = String::from_utf8(server.join().unwrap())
            .unwrap()
            .to_ascii_lowercase();

        assert!(request.contains("if-none-match: \"current\"\r\n"));
        assert_eq!(fs::read_to_string(&dest).unwrap(), "cached manifest");
        assert_eq!(fs::read_to_string(&etag_file).unwrap(), "\"current\"");
        assert!(fs::metadata(etag_file).unwrap().modified().unwrap() > old_modified);
    }

    #[test]
    fn check_interval_skips_request() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path().join("manifest.json");
        let etag_file = dest.with_added_extension("etag");
        fs::write(&dest, "cached manifest").unwrap();
        fs::write(&etag_file, "\"current\"").unwrap();

        download_with_etag(
            &ureq::Agent::new_with_defaults(),
            "not a valid URL",
            &dest,
            Some(time::Duration::from_secs(24 * 60 * 60)),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&dest).unwrap(), "cached manifest");
        assert_eq!(fs::read_to_string(&etag_file).unwrap(), "\"current\"");
    }
}
