use std::{collections::BTreeMap, fs};

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
        let tar_file = format!("{dir_str}/{target}.tar");
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
    let content = serde_json::to_string_pretty(manifest).context("Failed to serialize manifest")?;

    fs::write(file, content).with_context(|| format!("Failed to write {file}"))
}

fn write_shell_format(file: &str, manifest: &VersionManifest<Details>) -> Result<()> {
    // Write a shell-friendly .txt format alongside the JSON
    let txt_file = file.replace(".json", ".txt");

    let mut content = String::new();
    content.push_str(&format!("VERSION={}\n", manifest.version));
    content.push_str(&format!("TAG={}\n", manifest.details.tag));
    content.push_str(&format!("COMMIT={}\n", manifest.details.commit));
    content.push('\n');

    // Write assets in a shell-friendly format
    for (target, asset) in &manifest.details.assets {
        let target_upper = target.to_uppercase().replace('-', "_");
        content.push_str(&format!("# {target}\n"));
        content.push_str(&format!("{target_upper}_NAME={}\n", asset.name));
        content.push_str(&format!("{target_upper}_SIZE={}\n", asset.size));
        content.push_str(&format!("{target_upper}_SHA256={}\n", asset.sha256sum));
        content.push('\n');
    }

    fs::write(&txt_file, content).with_context(|| format!("Failed to write {txt_file}"))
}

fn create_archive(target: &str, version: &str, dir: &str) -> Result<(String, String)> {
    // Determine archive format and binary name based on target
    // Use tar.gz for Unix-like systems (Linux, macOS) and zip for Windows
    let (format, bin_name) = if target.contains("-windows-msvc") {
        (ArchiveFormat::Zip, "maa.exe")
    } else if target.contains("-windows-msvc-winget") {
        (ArchiveFormat::Zip, "maa-cli.exe")
    } else if target.contains("-linux-") || target.contains("-apple-darwin") {
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
