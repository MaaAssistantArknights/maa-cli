use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Context, Result};
use maa_version::{
    VersionManifest,
    cli::{Asset, Details},
};
use semver::Version;

use super::{Channel, archive, archive::ArchiveFormat};
use crate::env;

pub fn run() -> Result<()> {
    let channel: Channel = env::var("CHANNEL")?.parse()?;
    let version_str = env::var("VERSION")?;
    let tag = env::var("TAG")?;
    let commit = env::var("COMMIT")?;

    let version = Version::parse(&version_str)
        .with_context(|| format!("Failed to parse version: {}", version_str))?;

    // Determine which version files to update
    let version_files = channel.version_files();

    // Read existing manifests to preserve asset data structure
    let mut manifests: Vec<VersionManifest<Details>> = version_files
        .iter()
        .map(|file| {
            let manifest = read_or_create_manifest(file)?;
            Ok(manifest)
        })
        .collect::<Result<Vec<_>>>()?;

    // Update target-independent version info
    for manifest in &mut manifests {
        manifest.version = version.clone();
        manifest.details.tag = tag.clone();
        manifest.details.commit = commit.clone();
    }

    // Process each artifact directory
    let entries = fs::read_dir(".")
        .context("Failed to read current directory")?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|s| s.starts_with("maa_cli-"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    for entry in entries {
        let dir_name = entry.file_name();
        let dir_str = dir_name.to_str().context("Invalid directory name")?;
        let target = &dir_str[8..]; // Remove "maa_cli-" prefix

        println!("Processing target: {target}");

        // Extract tar file
        let tar_file = format!(
            "{dir_str}/{}.tar",
            target.strip_suffix("-winget").unwrap_or(target)
        );
        archive::extract_tar(&tar_file, dir_str)?;

        // Copy licenses.md
        fs::copy("licenses.md", format!("{dir_str}/licenses.md"))
            .context("Failed to copy licenses.md")?;

        // Create archive based on platform and get checksum
        let (archive_name, checksum_hash) = create_archive(target, &version_str, dir_str)?;
        let size = fs::metadata(&archive_name)
            .context("Failed to get file metadata")?
            .len();

        println!("  Archive: {archive_name}");
        println!("  Size: {size} bytes");
        println!("  SHA256: {checksum_hash}");

        // No need to update manifests for winget
        if target.ends_with("winget") {
            continue;
        }

        // Update version files with target-specific info
        let asset = Asset {
            name: archive_name,
            size,
            sha256sum: checksum_hash,
        };

        for manifest in &mut manifests {
            manifest
                .details
                .assets
                .insert(target.to_string(), asset.clone());
        }
    }

    // Write updated manifests back to files
    for (file, manifest) in version_files.iter().zip(&manifests) {
        write_manifest(file, manifest)?;
        write_shell_format(file, manifest)?;
    }

    println!("Version JSON files updated successfully");
    Ok(())
}

fn read_or_create_manifest(file: &str) -> Result<VersionManifest<Details>> {
    if fs::metadata(file).is_ok() {
        let content =
            fs::read_to_string(file).with_context(|| format!("Failed to read {}", file))?;

        serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", file))
    } else {
        // Create a new manifest with empty data
        Ok(VersionManifest {
            version: Version::new(0, 0, 0),
            details: Details {
                tag: String::new(),
                commit: String::new(),
                assets: BTreeMap::new(),
            },
        })
    }
}

fn write_manifest(file: &str, manifest: &VersionManifest<Details>) -> Result<()> {
    maa_atomic_fs::write_with(file, |writer| -> Result<()> {
        serde_json::to_writer_pretty(writer, manifest)?;
        Ok(())
    })
    .with_context(|| format!("Failed to write {file}"))
}

fn write_shell_format(file: &str, manifest: &VersionManifest<Details>) -> Result<()> {
    // Write a shell-friendly .txt format alongside the JSON
    let txt_path = PathBuf::from(file).with_extension("txt");

    use std::io::Write;

    maa_atomic_fs::write_with(&txt_path, |txt_file| -> Result<()> {
        writeln!(txt_file, "VERSION={}", manifest.version)?;
        writeln!(txt_file, "TAG={}", manifest.details.tag)?;
        writeln!(txt_file, "COMMIT={}", manifest.details.commit)?;
        writeln!(txt_file)?;

        // Write assets in a shell-friendly format
        for (target, asset) in &manifest.details.assets {
            let target_upper = target.to_uppercase().replace('-', "_");
            writeln!(txt_file, "# {target}")?;
            writeln!(txt_file, "{target_upper}_NAME={}", asset.name)?;
            writeln!(txt_file, "{target_upper}_SIZE={}", asset.size)?;
            writeln!(txt_file, "{target_upper}_SHA256={}", asset.sha256sum)?;
            writeln!(txt_file)?;
        }

        Ok(())
    })
    .with_context(|| format!("Failed to write {}", txt_path.display()))
}

fn create_archive(target: &str, version: &str, dir: &str) -> Result<(String, String)> {
    // Determine archive format and binary name based on target
    // Use tar.gz for Unix-like systems (Linux, macOS) and zip for Windows
    let (format, bin_name) = if target.contains("-windows-msvc-winget") {
        (ArchiveFormat::Zip, "maa-cli.exe")
    } else if target.contains("-windows-msvc") {
        (ArchiveFormat::Zip, "maa.exe")
    } else if target.contains("-linux-") || target.ends_with("-apple-darwin") {
        (ArchiveFormat::TarGz, "maa")
    } else {
        anyhow::bail!("Unknown target: {target}")
    };

    let ext = format.extension();
    let archive_name = format!("maa_cli-v{version}-{target}.{ext}");

    let binary = format!("{dir}/{bin_name}");
    let licenses = format!("{dir}/licenses.md");

    let checksum_hash = format.create(&archive_name, &[
        (binary.as_str(), bin_name),
        (licenses.as_str(), "licenses.md"),
    ])?;

    Ok((archive_name, checksum_hash))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn release_metadata_replaces_existing_files() {
        let temp_dir = tempdir().unwrap();
        let json_path = temp_dir.path().join("stable.json");
        let txt_path = temp_dir.path().join("stable.txt");
        fs::write(&json_path, "stale json").unwrap();
        fs::write(&txt_path, "stale text").unwrap();

        let manifest = VersionManifest {
            version: Version::new(1, 2, 3),
            details: Details {
                tag: "v1.2.3".to_string(),
                commit: "0123456789abcdef".to_string(),
                assets: BTreeMap::from([("aarch64-apple-darwin".to_string(), Asset {
                    name: "maa_cli-v1.2.3-aarch64-apple-darwin.tar.gz".to_string(),
                    size: 42,
                    sha256sum: "deadbeef".to_string(),
                })]),
            },
        };

        let json_path = json_path.to_string_lossy();
        write_manifest(&json_path, &manifest).unwrap();
        write_shell_format(&json_path, &manifest).unwrap();

        let persisted: VersionManifest<Details> =
            serde_json::from_reader(fs::File::open(&*json_path).unwrap()).unwrap();
        assert_eq!(persisted.version, Version::new(1, 2, 3));
        assert_eq!(persisted.details.tag, "v1.2.3");
        assert_eq!(persisted.details.commit, "0123456789abcdef");

        let shell = fs::read_to_string(txt_path).unwrap();
        assert!(shell.starts_with("VERSION=1.2.3\nTAG=v1.2.3\nCOMMIT=0123456789abcdef\n\n"));
        assert!(
            shell
                .contains("AARCH64_APPLE_DARWIN_NAME=maa_cli-v1.2.3-aarch64-apple-darwin.tar.gz\n")
        );
        assert!(!shell.contains("stale text"));
    }
}
