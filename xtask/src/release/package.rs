use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result};
use maa_version::{
    VersionManifest,
    cli::{Asset, Details},
};
use semver::Version;

use super::{archive, archive::ArchiveFormat};
use crate::env;

pub fn run() -> Result<()> {
    let version_str = env::var("VERSION")?;
    let tag = env::var("TAG")?;
    let commit = env::var("COMMIT")?;

    let version = Version::parse(&version_str)
        .with_context(|| format!("Failed to parse version: {}", version_str))?;

    // Build manifests from this release's artifacts only, so targets that are no
    // longer built (e.g. universal-apple-darwin) don't keep pointing at an old release
    let mut manifest = VersionManifest {
        version,
        details: Details {
            tag,
            commit,
            assets: BTreeMap::new(),
        },
    };

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

        // Add target-specific info to the manifest
        manifest.details.assets.insert(target.to_string(), Asset {
            name: archive_name,
            size,
            sha256sum: checksum_hash,
        });
    }

    write_manifest("manifest.json", &manifest)?;
    write_shell_format("manifest.txt", &manifest)?;

    println!("Release manifests written successfully");
    Ok(())
}

fn write_manifest(file: &str, manifest: &VersionManifest<Details>) -> Result<()> {
    let content = serde_json::to_string_pretty(manifest).context("Failed to serialize manifest")?;

    fs::write(file, content).with_context(|| format!("Failed to write {file}"))
}

fn write_shell_format(file: &str, manifest: &VersionManifest<Details>) -> Result<()> {
    use std::io::Write;

    let mut txt_file =
        std::fs::File::create(file).with_context(|| format!("Failed to create {file}"))?;

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

    txt_file
        .sync_all()
        .with_context(|| format!("Failed to sync {file}"))?;

    Ok(())
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
