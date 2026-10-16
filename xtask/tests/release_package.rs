use std::{collections::BTreeMap, fs, io::Read, path::Path, process::Command};

use anyhow::Result;
use flate2::read::GzDecoder;
use maa_version::{VersionManifest, cli::Details};
use sha2::{Digest, Sha256};

const BINARY: &[u8] = b"release binary\n";
const VERSION: &str = "0.7.0-beta.1";
const TAG: &str = "v0.7.0-beta.1";
const COMMIT: &str = "0123456789abcdef";

#[test]
fn package_emits_manifests_without_version_checkout() -> Result<()> {
    check_package(false)
}

#[test]
fn package_does_not_modify_existing_version_indexes() -> Result<()> {
    check_package(true)
}

fn check_package(with_version_checkout: bool) -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    if with_version_checkout {
        fs::write(root.join("licenses.md"), "unrelated root licenses")?;
    }
    if with_version_checkout {
        fs::create_dir(root.join("version"))?;
        for channel in ["stable", "beta", "alpha"] {
            for extension in ["json", "txt"] {
                fs::write(
                    root.join(format!("version/{channel}.{extension}")),
                    "previous release",
                )?;
            }
        }
    }

    let targets = [
        ("x86_64-unknown-linux-gnu", "maa", "tar.gz"),
        ("aarch64-apple-darwin", "maa", "tar.gz"),
        ("x86_64-pc-windows-msvc", "maa.exe", "zip"),
        ("x86_64-pc-windows-msvc-winget", "maa-cli.exe", "zip"),
    ];
    for (target, binary, _) in targets {
        let dir = root.join(format!("maa_cli-{target}"));
        fs::create_dir(&dir)?;
        let tar_name = target.strip_suffix("-winget").unwrap_or(target);
        let mut tar = tar::Builder::new(fs::File::create(dir.join(format!("{tar_name}.tar")))?);
        let mut header = tar::Header::new_gnu();
        header.set_size(BINARY.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, binary, BINARY)?;
        let licenses = format!("licenses for {target}\n");
        header.set_size(licenses.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "licenses.md", licenses.as_bytes())?;
        tar.finish()?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["release", "package"])
        .env_remove("CHANNEL")
        .env("VERSION", VERSION)
        .env("TAG", TAG)
        .env("COMMIT", COMMIT)
        .current_dir(root)
        .output()?;
    assert!(
        output.status.success(),
        "packaging failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest: VersionManifest<Details> =
        serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    assert_eq!(manifest.version.to_string(), VERSION);
    assert_eq!(manifest.details.tag, TAG);
    assert_eq!(manifest.details.commit, COMMIT);
    assert_eq!(manifest.details.assets.len(), 3);
    let mut shell_assets = BTreeMap::new();
    for (target, binary, extension) in targets {
        let archive_name = format!("maa_cli-v{VERSION}-{target}.{extension}");
        let archive_path = root.join(&archive_name);
        let files = read_archive(&archive_path, extension)?;
        assert_eq!(
            files,
            BTreeMap::from([
                (binary.to_owned(), BINARY.to_vec()),
                (
                    "licenses.md".to_owned(),
                    format!("licenses for {target}\n").into_bytes()
                ),
            ])
        );
        if target.ends_with("-winget") {
            assert!(!manifest.details.assets.contains_key(target));
            continue;
        }
        let asset = &manifest.details.assets[target];
        let bytes = fs::read(&archive_path)?;
        let checksum = format!(
            "{:x}",
            base16ct::HexDisplay(Sha256::digest(&bytes).as_slice())
        );
        assert_eq!(asset.name, archive_name);
        assert_eq!(asset.size, bytes.len() as u64);
        assert_eq!(asset.sha256sum, checksum);
        let prefix = target.to_uppercase().replace('-', "_");
        shell_assets.insert(
            target,
            format!(
                "# {target}\n{prefix}_NAME={archive_name}\n{prefix}_SIZE={}\n{prefix}_SHA256={checksum}\n\n",
                bytes.len()
            ),
        );
    }
    assert_eq!(
        fs::read_to_string(root.join("manifest.txt"))?,
        format!(
            "VERSION={VERSION}\nTAG={TAG}\nCOMMIT={COMMIT}\n\n{}",
            shell_assets.into_values().collect::<String>()
        )
    );
    if with_version_checkout {
        assert_eq!(fs::read_dir(root.join("version"))?.count(), 6);
        for entry in fs::read_dir(root.join("version"))? {
            assert_eq!(fs::read_to_string(entry?.path())?, "previous release");
        }
    } else {
        assert!(!root.join("version").exists());
    }
    Ok(())
}

fn read_archive(path: &Path, extension: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let file = fs::File::open(path)?;
    if extension == "zip" {
        let mut archive = zip::ZipArchive::new(file)?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            files.insert(entry.name().to_owned(), bytes);
        }
    } else {
        let mut archive = tar::Archive::new(GzDecoder::new(file));
        for entry in archive.entries()? {
            let mut entry = entry?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            files.insert(entry.path()?.to_string_lossy().into_owned(), bytes);
        }
    }
    Ok(files)
}
