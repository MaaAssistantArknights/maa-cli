//! Opt-in MirrorChyan delivery of full MaaResource snapshots.
use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result, anyhow, bail};
use fs2::FileExt;
use maa_installer::{
    download::{DownloadOptions, download},
    extract::Archive,
    installer::InstallerStyle,
};
use serde::{Deserialize, Serialize};
use ureq::Agent;

use crate::{config::cli::resource::MirrorChyanConfig, state::AGENT};

const ENDPOINT: &str = "https://mirrorchyan.com/api/resources/MaaResource/latest";
const MARKER: &str = "mirrorchyan.json";

fn snapshot_root() -> PathBuf {
    maa_dirs::data().join("MirrorChyan").join("MaaResource")
}

pub fn resource_dir() -> PathBuf {
    active_snapshot(&snapshot_root())
}

/// Immutable generations let existing readers keep using their selected snapshot.
fn active_snapshot(root: &Path) -> PathBuf {
    let generation = fs::read(root.join("current.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<String>(&bytes).ok());
    if let Some(generation) = generation {
        let mut parts = Path::new(&generation).components();
        if matches!(parts.next(), Some(std::path::Component::Normal(_)))
            && parts.next().is_none()
            && generation.starts_with("snapshot-")
        {
            return root.join(generation);
        }
    }
    // Read snapshots created before the generation layout was introduced.
    root.to_path_buf()
}

pub fn usable_snapshot() -> bool {
    let dest = resource_dir();
    installed_version(&dest)
        .is_some_and(|installed| validate_snapshot(&dest, &installed.version).is_ok())
}

#[derive(Deserialize)]
struct Response {
    code: i64,
    data: Option<Release>,
}

#[derive(Deserialize)]
struct Release {
    version_name: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Installed {
    version: String,
}

fn latest(agent: &Agent, endpoint: &str, cdk: &str, current: &str) -> Result<Release> {
    let mut response = agent
        .get(endpoint)
        .query("current_version", current)
        .query("cdk", cdk)
        .query("user_agent", "maa-cli")
        .config()
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .call()
        // Do not retain an error chain containing a credential-bearing URI.
        .map_err(|_| anyhow!("MirrorChyan update request failed"))?;
    let status = response.status();
    if status.is_redirection() {
        bail!("MirrorChyan API redirects are not allowed");
    }
    let reply: Response = response
        .body_mut()
        .with_config()
        .limit(64 * 1024)
        .read_json()
        .map_err(|_| anyhow!("Invalid MirrorChyan response (HTTP {status})"))?;
    if reply.code != 0 {
        // Server messages can echo user input. Report only the documented code.
        bail!("MirrorChyan error code {} (HTTP {status})", reply.code);
    }
    if !status.is_success() {
        bail!("MirrorChyan request failed (HTTP {status})");
    }
    let release = reply
        .data
        .context("MirrorChyan response has no release data")?;
    if release.version_name.trim().is_empty() {
        bail!("MirrorChyan response has no resource version");
    }
    Ok(release)
}

fn download_url(release: &Release) -> Result<&str> {
    release
        .url
        .as_deref()
        .filter(|s| !s.is_empty())
        .context("MirrorChyan returned no download URL; check your CDK and download quota")
}

fn download_archive(agent: &Agent, url: &str, dest: &Path) -> Result<()> {
    let options = DownloadOptions::new(
        url.into(),
        0,
        None::<maa_installer::manifest::MirrorOptions<'_, std::iter::Empty<Cow<'_, str>>>>,
    )
    .with_display_url("MirrorChyan MaaResource");
    download(agent, options, dest, &InstallerStyle::default(), ())
        // Signed URLs may appear in the underlying error chain.
        .map_err(|error| anyhow!("MirrorChyan resource download failed: {}", error.kind()))
}

fn installed_version(dest: &Path) -> Option<Installed> {
    serde_json::from_slice(&fs::read(dest.join(MARKER)).ok()?).ok()
}

fn validate_snapshot(dest: &Path, version: &str) -> Result<()> {
    fn read_json(path: &Path) -> Result<serde_json::Value> {
        let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("Failed to parse JSON in {}", path.display()))
    }

    fn validate_task_file(path: &Path) -> Result<()> {
        let value = read_json(path)?;
        if !value.as_object().is_some_and(|tasks| {
            !tasks.is_empty() && tasks.values().all(serde_json::Value::is_object)
        }) {
            bail!("Invalid task data in {}", path.display());
        }
        Ok(())
    }

    fn validate_task_tree(path: &Path) -> Result<usize> {
        let mut count = 0;
        let entries = fs::read_dir(path)
            .with_context(|| format!("Failed to read task directory {}", path.display()))?;
        for entry in entries {
            let path = entry
                .with_context(|| format!("Failed to read entry in {}", path.display()))?
                .path();
            if path.is_dir() {
                count += validate_task_tree(&path)?;
            } else if path.extension().is_some_and(|ext| ext == "json") {
                validate_task_file(&path)?;
                count += 1;
            }
        }
        Ok(count)
    }

    fn has_template(path: &Path) -> Result<bool> {
        let entries = fs::read_dir(path)
            .with_context(|| format!("Failed to read template directory {}", path.display()))?;
        for entry in entries {
            let path = entry
                .with_context(|| format!("Failed to read entry in {}", path.display()))?
                .path();
            if path.is_dir() {
                if has_template(&path)? {
                    return Ok(true);
                }
            } else if path.extension().is_some_and(|ext| ext == "png") {
                let metadata = fs::metadata(&path)
                    .with_context(|| format!("Failed to read metadata for {}", path.display()))?;
                if metadata.len() > 0 {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    let resource = dest.join("resource");
    let version_path = resource.join("version.json");
    let metadata = read_json(&version_path)?;
    let actual = metadata
        .get("last_updated")
        .and_then(|value| value.as_str())
        .with_context(|| format!("Missing last_updated in {}", version_path.display()))?;
    if actual != version {
        bail!(
            "Resource version mismatch in {}: expected {version:?}, found {actual:?}",
            version_path.display()
        );
    }
    // MaaResource overlays the basic resources and may use split hotfix task files.
    if resource.join("tasks.json").is_file() {
        validate_task_file(&resource.join("tasks.json"))?;
    } else if validate_task_tree(&resource.join("tasks"))? == 0 {
        bail!("No task JSON files in {}", resource.join("tasks").display());
    }
    let templates = resource.join("template");
    if !has_template(&templates)? {
        bail!("No non-empty PNG templates in {}", templates.display());
    }
    let stages = resource.join("stages.json");
    if !read_json(&stages)?
        .as_array()
        .is_some_and(|items| !items.is_empty() && items.iter().all(serde_json::Value::is_object))
    {
        bail!("Invalid stage data in {}", stages.display());
    }
    for name in [
        "battle_data.json",
        "infrast.json",
        "recruitment.json",
        "item_index.json",
    ] {
        let path = resource.join(name);
        if !read_json(&path)?
            .as_object()
            .is_some_and(|object| !object.is_empty())
        {
            bail!("Expected a non-empty JSON object in {}", path.display());
        }
    }
    Ok(())
}

fn fresh(dest: &Path, interval: u64) -> bool {
    if interval == 0
        || !installed_version(dest)
            .is_some_and(|snapshot| validate_snapshot(dest, &snapshot.version).is_ok())
    {
        return false;
    }
    fs::metadata(dest.join(MARKER))
        .and_then(|m| m.modified())
        .ok()
        .and_then(|time| time.elapsed().ok())
        .is_some_and(|age| age < Duration::from_secs(interval))
}

/// Validate and stage a full archive before replacing only the owned snapshot.
fn install_archive(archive: &Path, dest: &Path, version: &str) -> Result<()> {
    fs::create_dir_all(dest)?;
    let staging = tempfile::Builder::new()
        .prefix("snapshot-")
        .tempdir_in(dest)?;
    let content = staging.path();
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    for index in 0..zip.len() {
        let entry = zip.by_index(index)?;
        let path = entry
            .enclosed_name()
            .context("Unsafe path in MirrorChyan archive")?;
        if entry.name()?.contains('\\')
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("MirrorChyan resource archive contains an unsafe path or symlink");
        }
        if path == Path::new("changes.json") {
            bail!("MirrorChyan returned an incremental archive; a full snapshot is required");
        }
    }
    zip.extract(InstallerStyle::default().init_spinner(), |path| {
        path.starts_with("resource").then(|| content.join(path))
    })?;
    validate_snapshot(content, version)?;
    fs::write(
        content.join(MARKER),
        serde_json::to_vec(&Installed {
            version: version.to_owned(),
        })?,
    )?;
    // Publish only after all files have been extracted and validated.
    let generation = staging.keep();
    let name = generation
        .file_name()
        .and_then(|name| name.to_str())
        .context("Invalid generation name")?;
    maa_atomic_fs::write(dest.join("current.json"), serde_json::to_vec(name)?)
        .map_err(|_| anyhow!("Failed to publish MirrorChyan snapshot"))?;
    Ok(())
}

struct UpdateLock(fs::File);

impl Drop for UpdateLock {
    fn drop(&mut self) {
        // A forked child may briefly retain the file descriptor before exec.
        let _ = FileExt::unlock(&self.0);
    }
}

fn acquire_lock(parent: &Path) -> Result<UpdateLock> {
    fs::create_dir_all(parent)?;
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(parent.join("update.lock"))?;
    file.try_lock_exclusive()
        .context("MirrorChyan update is already running")?;
    Ok(UpdateLock(file))
}

pub fn update(config: &MirrorChyanConfig, is_auto: bool) -> Result<()> {
    let agent = Agent::config_builder()
        .tls_config(AGENT.config().tls_config().clone())
        .user_agent(AGENT.config().user_agent().clone())
        .https_only(true)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(Duration::from_secs(600)))
        .build()
        .into();
    update_at(config, is_auto, &snapshot_root(), &agent, ENDPOINT)
}

fn update_at(
    config: &MirrorChyanConfig,
    is_auto: bool,
    dest: &Path,
    agent: &Agent,
    endpoint: &str,
) -> Result<()> {
    let parent = dest.parent().context("Resource snapshot has no parent")?;
    let _lock = acquire_lock(parent)?;
    let selected = active_snapshot(dest);
    let root = dest;
    let dest = selected.as_path();
    if is_auto && fresh(dest, config.check_interval) {
        return Ok(());
    }
    let cdk = config
        .cdk
        .get_with_desc("MirrorChyan CDK")
        .map_err(|_| anyhow!("Could not resolve MirrorChyan CDK"))?
        .filter(|s| !s.trim().is_empty())
        .context("MirrorChyan CDK is not configured")?;
    log::info!("Checking MaaResource updates via MirrorChyan...");
    let installed = installed_version(dest)
        .filter(|snapshot| validate_snapshot(dest, &snapshot.version).is_ok());
    let current = installed
        .as_ref()
        .map_or("0", |snapshot| snapshot.version.as_str());
    let release = latest(agent, endpoint, cdk.trim(), current)?;
    if current == release.version_name {
        fs::OpenOptions::new()
            .write(true)
            .open(dest.join(MARKER))?
            .set_modified(SystemTime::now())?;
        return Ok(());
    }
    // Avoid requesting a download URL when already up to date. If an update
    // exists, version 0 requests a full snapshot rather than a Git-incompatible delta.
    let release = if current == "0" {
        release
    } else {
        latest(agent, endpoint, cdk.trim(), "0")?
    };
    let url = download_url(&release)?;
    let temp = tempfile::tempdir_in(parent)?;
    let archive = temp.path().join("resource.zip");
    log::info!("Downloading MirrorChyan MaaResource...");
    download_archive(agent, url, &archive)?;
    install_archive(&archive, root, &release.version_name)?;
    log::info!("MirrorChyan MaaResource update completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    // Unwraps below assert that local fixtures and test server setup succeed;
    // they never process a user's credentials or production resource files.
    use std::{io::Write, thread};

    use super::*;

    fn api_reply(status: u16, body: &str, secret: &str) -> Result<Release> {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/latest", server.server_addr());
        let body = body.to_owned();
        let secret = secret.to_owned();
        let encoded_secret_test = secret == "a+b&c=d";
        let worker = thread::spawn(move || {
            let request = server.recv().unwrap();
            assert!(request.url().contains("current_version=0"));
            assert!(request.url().contains("user_agent=maa-cli"));
            assert!(request.url().contains("cdk="));
            if encoded_secret_test {
                assert!(request.url().contains("cdk=a%2Bb%26c%3Dd"));
                assert!(!request.url().contains("&c=d"));
            }
            request
                .respond(tiny_http::Response::from_string(body).with_status_code(status))
                .unwrap();
        });
        let result = latest(&Agent::new_with_defaults(), &endpoint, &secret, "0");
        worker.join().unwrap();
        result
    }

    fn snapshot_entries() -> Vec<(&'static str, &'static str)> {
        vec![
            ("resource/version.json", r#"{"last_updated":"v1"}"#),
            (
                "resource/tasks.json",
                r#"{"StartUp":{"algorithm":"JustReturn"}}"#,
            ),
            ("resource/battle_data.json", r#"{"operators":{}}"#),
            ("resource/infrast.json", r#"{"rooms":{}}"#),
            ("resource/recruitment.json", r#"{"operators":[]}"#),
            ("resource/item_index.json", r#"{"item":{}}"#),
            ("resource/stages.json", r#"[{"stageId":"test"}]"#),
            ("resource/template/example.png", "template"),
        ]
    }

    fn seed_snapshot(dest: &Path) {
        for (name, content) in snapshot_entries() {
            let path = dest.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        fs::write(dest.join(MARKER), r#"{"version":"v1"}"#).unwrap();
    }

    fn archive(path: &Path, entries: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, body) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn successful_api_and_encoded_cdk() {
        let release = api_reply(200, r#"{"code":0,"data":{"version_name":"2026-10-09","url":"https://mirrorchyan.com/package.zip"}}"#, "a+b&c=d").unwrap();
        assert_eq!(release.version_name, "2026-10-09");
        assert!(download_url(&release).is_ok());
    }

    #[test]
    fn api_errors_do_not_echo_secrets() {
        let error = api_reply(403, r#"{"code":7002,"msg":"SECRET"}"#, "SECRET")
            .err()
            .unwrap();
        assert!(error.to_string().contains("7002"));
        assert!(!format!("{error:?}").contains("SECRET"));
        assert!(api_reply(200, "not JSON SECRET", "SECRET").is_err());
        assert!(api_reply(302, "SECRET", "SECRET").is_err());
    }

    #[test]
    fn missing_download_url_is_rejected() {
        for url in [None, Some("")] {
            assert!(
                download_url(&Release {
                    version_name: "v1".into(),
                    url: url.map(str::to_owned),
                })
                .is_err()
            );
        }
    }

    #[test]
    fn replacing_full_snapshot_removes_obsolete_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        fs::create_dir_all(dest.join("resource")).unwrap();
        fs::write(dest.join("resource/obsolete"), "old").unwrap();
        let package = tmp.path().join("resource.zip");
        archive(&package, &snapshot_entries());
        install_archive(&package, &dest, "v1").unwrap();
        let selected = active_snapshot(&dest);
        assert!(!selected.join("resource/obsolete").exists());
        assert_eq!(installed_version(&selected).unwrap().version, "v1");
        assert!(fresh(&selected, 600));
        assert!(!fresh(&selected, 0));
    }

    #[test]
    fn incremental_archive_preserves_previous_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        seed_snapshot(&dest);
        let package = tmp.path().join("resource.zip");
        let mut entries = snapshot_entries();
        entries.push(("changes.json", "{}"));
        archive(&package, &entries);
        let error = install_archive(&package, &dest, "v1").unwrap_err();
        assert!(error.to_string().contains("incremental archive"));
        assert!(validate_snapshot(&dest, "v1").is_ok());
    }

    #[test]
    fn traversal_in_otherwise_valid_archive_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        seed_snapshot(&dest);
        let package = tmp.path().join("resource.zip");
        let mut entries = snapshot_entries();
        entries.push(("../../escape", "bad"));
        archive(&package, &entries);
        let error = install_archive(&package, &dest, "v1").unwrap_err();
        assert!(error.to_string().contains("Unsafe path"));
        assert!(validate_snapshot(&dest, "v1").is_ok());
        assert!(!tmp.path().join("escape").exists());
        assert!(!tmp.path().parent().unwrap().join("escape").exists());
    }

    #[test]
    fn incomplete_or_invalid_snapshot_preserves_previous_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        seed_snapshot(&dest);
        let package = tmp.path().join("resource.zip");
        for (index, invalid, context) in [
            (0, "not json", "Failed to parse JSON"),
            (0, "{}", "Missing last_updated"),
            (
                0,
                r#"{"last_updated":"different"}"#,
                "Resource version mismatch",
            ),
            (1, "{}", "Invalid task data"),
            (1, "not json", "Failed to parse JSON"),
            (1, r#"{"StartUp":null}"#, "Invalid task data"),
            (2, "{}", "non-empty JSON object"),
            (6, "{}", "Invalid stage data"),
            (7, "", "No non-empty PNG templates"),
        ] {
            let mut entries = snapshot_entries();
            entries[index].1 = invalid;
            archive(&package, &entries);
            let error = install_archive(&package, &dest, "v1").unwrap_err();
            assert!(error.to_string().contains(context), "{error:#}");
            let detail = format!("{error:#}").replace('\\', "/");
            let path = if index == 7 {
                "resource/template"
            } else {
                entries[index].0
            };
            assert!(detail.contains(path), "{error:#}");
            assert!(validate_snapshot(&dest, "v1").is_ok());
        }
        for missing in 0..snapshot_entries().len() {
            let mut entries = snapshot_entries();
            let (name, _) = entries.remove(missing);
            archive(&package, &entries);
            let error = install_archive(&package, &dest, "v1").unwrap_err();
            let path = if missing == 1 {
                "resource/tasks"
            } else if missing == 7 {
                "resource/template"
            } else {
                name
            };
            assert!(
                format!("{error:#}").replace('\\', "/").contains(path),
                "{error:#}"
            );
            assert!(validate_snapshot(&dest, "v1").is_ok());
        }
    }

    #[test]
    fn interrupted_publication_retains_previous_and_old_readers() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        let package = tmp.path().join("resource.zip");
        archive(&package, &snapshot_entries());
        install_archive(&package, &dest, "v1").unwrap();
        let old = active_snapshot(&dest);
        // A process can stop with an unpublished staging generation present.
        fs::create_dir(dest.join("snapshot-unpublished")).unwrap();
        assert_eq!(active_snapshot(&dest), old);
        install_archive(&package, &dest, "v1").unwrap();
        assert_ne!(active_snapshot(&dest), old);
        assert!(validate_snapshot(&old, "v1").is_ok());
        assert!(validate_snapshot(&active_snapshot(&dest), "v1").is_ok());
    }

    #[test]
    fn lock_excludes_concurrent_update_and_can_be_reacquired() {
        let tmp = tempfile::tempdir().unwrap();
        let lock = acquire_lock(tmp.path()).unwrap();
        assert!(acquire_lock(tmp.path()).is_err());
        let inherited = lock.0.try_clone().unwrap();
        drop(lock);
        assert!(acquire_lock(tmp.path()).is_ok());
        drop(inherited);
    }

    #[test]
    fn download_follows_redirects_and_installs_full_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let package = tmp.path().join("source.zip");
        archive(&package, &snapshot_entries());
        let bytes = fs::read(package).unwrap();
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let initial = format!("http://{}/download?token=SECRET", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(tiny_http::Response::empty(302).with_header(
                    tiny_http::Header::from_bytes("Location", "/resource.zip").unwrap(),
                ))
                .unwrap();
            let request = server.recv().unwrap();
            assert_eq!(request.url(), "/resource.zip");
            request
                .respond(tiny_http::Response::from_data(bytes))
                .unwrap();
        });
        let download = tmp.path().join("download.zip");
        download_archive(&Agent::new_with_defaults(), &initial, &download).unwrap();
        worker.join().unwrap();
        let dest = tmp.path().join("installed");
        install_archive(&download, &dest, "v1").unwrap();
        assert!(validate_snapshot(&active_snapshot(&dest), "v1").is_ok());
    }

    #[test]
    fn download_errors_do_not_echo_signed_urls() {
        let tmp = tempfile::tempdir().unwrap();
        let agent: Agent = Agent::config_builder().https_only(true).build().into();
        let error = download_archive(
            &agent,
            "http://example.org/archive?token=SECRET",
            &tmp.path().join("download"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("Network error"));
        assert_eq!(error.chain().count(), 1);
        assert!(!format!("{error:?}").contains("SECRET"));
    }

    #[test]
    fn download_io_errors_keep_category_without_urls() {
        let tmp = tempfile::tempdir().unwrap();
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/archive?token=SECRET", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(tiny_http::Response::from_string("archive"))
                .unwrap();
        });
        let error = download_archive(
            &Agent::new_with_defaults(),
            &url,
            &tmp.path().join("missing/archive"),
        )
        .unwrap_err();
        worker.join().unwrap();
        assert!(error.to_string().contains("I/O error"));
        assert_eq!(error.chain().count(), 1);
        assert!(!format!("{error:?}").contains("SECRET"));
    }

    #[test]
    fn stalled_body_read_is_bounded() {
        use std::{net::TcpListener, time::Instant};
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let initial = format!("http://{}/download", server.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (mut stream, _) = server.accept().unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nx")
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(300));
        });
        let tmp = tempfile::tempdir().unwrap();
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(50)))
            .build()
            .into();
        let start = Instant::now();
        assert!(download_archive(&agent, &initial, &tmp.path().join("download")).is_err());
        assert!(start.elapsed() < Duration::from_millis(250));
        worker.join().unwrap();
    }

    #[test]
    fn malformed_pointer_cannot_escape_snapshot_root() {
        let tmp = tempfile::tempdir().unwrap();
        for name in [
            "../snapshot-outside",
            "/snapshot-absolute",
            "snapshot-a/../../escape",
        ] {
            fs::write(
                tmp.path().join("current.json"),
                serde_json::to_vec(name).unwrap(),
            )
            .unwrap();
            assert_eq!(active_snapshot(tmp.path()), tmp.path());
        }
    }

    #[test]
    fn cdk_is_not_logged_at_trace() {
        use clap::Parser;
        const SECRET: &str = "TRACE-TEST-CDK-DO-NOT-LOG";
        if std::env::var_os("MAA_MIRROR_LOG_CHILD").is_some() {
            crate::command::Cli::parse_from(["maa", "list", "-vvv"])
                .log
                .init_logger()
                .unwrap();
            assert!(
                api_reply(
                    403,
                    r#"{"code":7002,"msg":"TRACE-TEST-CDK-DO-NOT-LOG"}"#,
                    SECRET
                )
                .is_err()
            );
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let url = format!(
                "http://{}/resource.zip?token={SECRET}",
                server.server_addr()
            );
            let worker = thread::spawn(move || {
                let request = server.recv().unwrap();
                request
                    .respond(tiny_http::Response::from_string("archive"))
                    .unwrap();
            });
            let tmp = tempfile::tempdir().unwrap();
            download_archive(
                &Agent::new_with_defaults(),
                &url,
                &tmp.path().join("archive"),
            )
            .unwrap();
            worker.join().unwrap();
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("cli.toml"),
            format!("[resource.source]\ntype = 'mirrorchyan'\ncdk = \"{SECRET}\"\n"),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "installer::resource::mirrorchyan::tests::cdk_is_not_logged_at_trace",
                "--nocapture",
            ])
            .env("MAA_MIRROR_LOG_CHILD", "1")
            .env("MAA_CONFIG_DIR", tmp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains(SECRET));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(SECRET));
    }

    #[test]
    fn missing_metadata_or_http_error_is_not_success() {
        assert!(api_reply(503, r#"{"code":0,"data":{"version_name":"v1"}}"#, "fake").is_err());
        assert!(api_reply(200, r#"{"code":0,"data":{"version_name":""}}"#, "fake").is_err());
        assert!(api_reply(200, r#"{"code":0}"#, "fake").is_err());
    }

    #[test]
    fn split_hotfix_layout_without_core_config_is_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        let package = tmp.path().join("resource.zip");
        let mut entries = snapshot_entries();
        entries[1].0 = "resource/tasks/hotfix/ocr_replace.json";
        archive(&package, &entries);
        install_archive(&package, &dest, "v1").unwrap();
        let selected = active_snapshot(&dest);
        assert!(validate_snapshot(&selected, "v1").is_ok());
        assert!(!selected.join("resource/config.json").exists());
        assert!(!selected.join("resource/tasks.json").exists());
        entries[1].1 = "not json";
        archive(&package, &entries);
        let error = install_archive(&package, &dest, "v1").unwrap_err();
        assert!(
            error
                .to_string()
                .replace('\\', "/")
                .contains("tasks/hotfix/ocr_replace.json")
        );
        assert_eq!(active_snapshot(&dest), selected);
    }

    #[test]
    fn corrupt_zip_preserves_previous_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        seed_snapshot(&dest);
        let archive = tmp.path().join("bad.zip");
        fs::write(&archive, b"truncated archive").unwrap();
        assert!(install_archive(&archive, &dest, "v1").is_err());
        assert!(validate_snapshot(&dest, "v1").is_ok());
    }

    #[test]
    fn symlink_archive_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("links.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        for (name, body) in snapshot_entries() {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.add_symlink(
            "resource/link",
            "../../outside",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.finish().unwrap();
        let dest = tmp.path().join("installed");
        seed_snapshot(&dest);
        let error = install_archive(&archive, &dest, "v1").unwrap_err();
        assert!(error.to_string().contains("symlink"));
        assert!(validate_snapshot(&dest, "v1").is_ok());
        assert!(!tmp.path().join("outside").exists());
    }

    #[test]
    fn up_to_date_check_does_not_request_full_download() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        seed_snapshot(&dest);
        let marker = dest.join(MARKER);
        let original_marker = fs::read(&marker).unwrap();
        let old_time = SystemTime::now() - Duration::from_secs(3600);
        fs::OpenOptions::new()
            .write(true)
            .open(&marker)
            .unwrap()
            .set_modified(old_time)
            .unwrap();
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/latest", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server.recv().unwrap();
            assert!(request.url().contains("current_version=v1"));
            request
                .respond(tiny_http::Response::from_string(
                    r#"{"code":0,"data":{"version_name":"v1"}}"#,
                ))
                .unwrap();
        });
        let config = MirrorChyanConfig {
            cdk: crate::config::cli::secret::Secret::Plain("test-key".into()),
            check_interval: 0,
        };
        update_at(
            &config,
            false,
            &dest,
            &Agent::new_with_defaults(),
            &endpoint,
        )
        .unwrap();
        worker.join().unwrap();
        assert_eq!(installed_version(&dest).unwrap().version, "v1");
        assert_eq!(fs::read(&marker).unwrap(), original_marker);
        assert!(fs::metadata(&marker).unwrap().modified().unwrap() > old_time);
        assert!(acquire_lock(tmp.path()).is_ok());
    }

    #[test]
    fn new_version_requests_full_snapshot_and_preserves_old_on_auth_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("installed");
        seed_snapshot(&dest);
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/latest", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server.recv().unwrap();
            assert!(request.url().contains("current_version=v1"));
            request.respond(tiny_http::Response::from_string(r#"{"code":0,"data":{"version_name":"v2","url":"https://example.org/delta.zip"}}"#)).unwrap();
            let request = server.recv().unwrap();
            assert!(request.url().contains("current_version=0"));
            request
                .respond(
                    tiny_http::Response::from_string(r#"{"code":7003,"msg":"test-key"}"#)
                        .with_status_code(403),
                )
                .unwrap();
        });
        let config = MirrorChyanConfig {
            cdk: crate::config::cli::secret::Secret::Plain("test-key".into()),
            check_interval: 0,
        };
        let error = update_at(
            &config,
            false,
            &dest,
            &Agent::new_with_defaults(),
            &endpoint,
        )
        .unwrap_err();
        worker.join().unwrap();
        assert!(!format!("{error:?}").contains("test-key"));
        assert_eq!(installed_version(&dest).unwrap().version, "v1");
        assert!(acquire_lock(tmp.path()).is_ok());
    }
}
