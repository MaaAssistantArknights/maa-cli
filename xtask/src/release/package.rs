//! Release bundle and version index.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, ensure};
use maa_version::{
    VersionManifest,
    cli::{Asset, Details},
};

use super::{
    archive,
    archive::ArchiveFormat,
    plan::{Index, Plan, read_manifest},
    version::Target,
};

pub const BUNDLE: &str = "release-bundle";
const BUNDLE_INDEX: &str = "version";

/// Package build artifacts into archives and the index manifests of every target channel.
pub fn run(plan_dir: &Path) -> Result<()> {
    let plan = Plan::load(plan_dir)?;
    let version = plan.version.to_string();
    let bundle = Path::new(BUNDLE);
    fs::create_dir_all(bundle.join(BUNDLE_INDEX)).context("Failed to create release bundle")?;

    let mut manifest = VersionManifest {
        version: plan.version.clone(),
        details: Details {
            tag: plan.tag.clone(),
            commit: plan.commit.clone(),
            assets: BTreeMap::new(),
        },
    };

    let mut targets: Vec<String> = fs::read_dir(".")
        .context("Failed to read current directory")?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|name| name.starts_with("maa_cli-"))
        .collect();
    targets.sort();
    ensure!(!targets.is_empty(), "No build artifacts to package");

    for dir in targets {
        let target = &dir["maa_cli-".len()..];
        println!("Processing target: {target}");

        let tar_file = format!(
            "{dir}/{}.tar",
            target.strip_suffix("-winget").unwrap_or(target)
        );
        archive::extract_tar(&tar_file, &dir)?;
        fs::copy("licenses.md", format!("{dir}/licenses.md"))
            .context("Failed to copy licenses.md")?;

        let (archive_name, checksum_hash) = create_archive(bundle, target, &version, &dir)?;
        let size = fs::metadata(bundle.join(&archive_name))
            .context("Failed to get file metadata")?
            .len();
        println!("  Archive: {archive_name}");
        println!("  Size: {size} bytes");
        println!("  SHA256: {checksum_hash}");

        // WinGet archives are published but not indexed.
        if target.ends_with("winget") {
            continue;
        }
        manifest.details.assets.insert(target.to_string(), Asset {
            name: archive_name,
            size,
            sha256sum: checksum_hash,
        });
    }

    for target in &plan.targets {
        for file in target.channel.index_files() {
            let path = bundle.join(BUNDLE_INDEX).join(file);
            write_manifest(&path, &manifest)?;
            write_shell_format(&path, &manifest)?;
        }
    }
    println!("Release bundle created successfully");
    Ok(())
}

/// Check, before publishing anything, that the planned index updates still apply.
pub fn check_publication(plan_dir: &Path, index: &Path) -> Result<()> {
    let plan = Plan::load(plan_dir)?;
    let index = Index::read(index)?;
    for target in &plan.targets {
        check_target(&plan, &index, target)?;
    }
    Ok(())
}

/// Apply the bundle's manifests to the version branch checkout.
///
/// Each target channel must still hold the entry seen when the plan was
/// made, or already hold this release. Anything else means another release
/// was published in between, and nothing is written.
pub fn update_index(plan_dir: &Path, bundle: &Path, index_root: &Path) -> Result<()> {
    let plan = Plan::load(plan_dir)?;
    let index = Index::read(index_root)?;
    let bundle = bundle.join(BUNDLE_INDEX);

    let mut updates = Vec::new();
    for target in &plan.targets {
        check_target(&plan, &index, target)?;
        for file in target.channel.index_files() {
            let source = bundle.join(file);
            let manifest = read_manifest(&source)?
                .with_context(|| format!("{} is missing", source.display()))?;
            ensure!(
                manifest.version == plan.version
                    && manifest.details.tag == plan.tag
                    && manifest.details.commit == plan.commit,
                "{} does not describe {}",
                source.display(),
                plan.version
            );
            updates.push((index_root.join(file), manifest));
        }
    }
    for (destination, manifest) in updates {
        write_manifest(&destination, &manifest)?;
        write_shell_format(&destination, &manifest)?;
    }
    Ok(())
}

fn check_target(plan: &Plan, index: &Index, target: &Target) -> Result<()> {
    let current = index.head(target.channel);
    let release = plan.identity();
    ensure!(
        current == target.expected.as_ref() || current == Some(&release),
        "The {} index changed since this release was planned (expected {}, found {}); plan a new release instead of retrying this one",
        target.channel.as_str(),
        describe(target.expected.as_ref()),
        describe(current),
    );
    Ok(())
}

fn describe(identity: Option<&super::version::Identity>) -> String {
    identity.map_or_else(
        || "nothing".into(),
        |id| format!("{} at {}", id.version, id.commit),
    )
}

fn write_manifest(file: impl AsRef<Path>, manifest: &VersionManifest<Details>) -> Result<()> {
    let file = file.as_ref();
    let content = serde_json::to_string_pretty(manifest).context("Failed to serialize manifest")?;

    fs::write(file, content).with_context(|| format!("Failed to write {}", file.display()))
}

fn write_shell_format(file: impl AsRef<Path>, manifest: &VersionManifest<Details>) -> Result<()> {
    // Write a shell-friendly .txt format alongside the JSON
    let txt_path = file.as_ref().with_extension("txt");

    use std::io::Write;

    let mut txt_file = std::fs::File::create(&txt_path)
        .with_context(|| format!("Failed to create {}", txt_path.display()))?;

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
        .with_context(|| format!("Failed to sync {}", txt_path.display()))?;

    Ok(())
}

fn create_archive(
    output: &Path,
    target: &str,
    version: &str,
    dir: &str,
) -> Result<(String, String)> {
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

    let checksum_hash = format.create(output.join(&archive_name), &[
        (binary.as_str(), bin_name),
        (licenses.as_str(), "licenses.md"),
    ])?;

    Ok((archive_name, checksum_hash))
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::*;
    use crate::release::{Channel, version::Identity};

    fn manifest(version: &str, commit: &str) -> VersionManifest<Details> {
        VersionManifest {
            version: Version::parse(version).unwrap(),
            details: Details {
                tag: format!("v{version}"),
                commit: commit.into(),
                assets: BTreeMap::from([("aarch64-apple-darwin".into(), Asset {
                    name: format!("maa_cli-v{version}-aarch64-apple-darwin.tar.gz"),
                    size: 123,
                    sha256sum: "a".repeat(64),
                })]),
            },
        }
    }

    fn id(version: &str, commit: &str) -> Identity {
        Identity {
            version: Version::parse(version).unwrap(),
            commit: commit.into(),
        }
    }

    /// A temporary working directory with a plan, a bundle and a version checkout.
    struct Fixture {
        dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new(plan: Plan) -> Result<Self> {
            let dir = tempfile::tempdir()?;
            fs::create_dir_all(dir.path().join("plan"))?;
            fs::create_dir_all(dir.path().join("version"))?;
            fs::write(
                dir.path().join("plan/plan.json"),
                serde_json::to_vec(&plan)?,
            )?;
            let bundle = dir.path().join(BUNDLE).join(BUNDLE_INDEX);
            fs::create_dir_all(&bundle)?;
            let incoming = manifest(&plan.version.to_string(), &plan.commit);
            for target in &plan.targets {
                for file in target.channel.index_files() {
                    write_manifest(bundle.join(file), &incoming)?;
                }
            }
            Ok(Self { dir })
        }

        fn publish(&self, file: &str, version: &str, commit: &str) -> Result<()> {
            let path = self.dir.path().join("version").join(file);
            write_manifest(&path, &manifest(version, commit))?;
            write_shell_format(&path, &manifest(version, commit))
        }

        fn update(&self) -> Result<()> {
            let root = self.dir.path();
            update_index(
                &root.join("plan"),
                &root.join(BUNDLE),
                &root.join("version"),
            )
        }

        fn check(&self) -> Result<()> {
            check_publication(
                &self.dir.path().join("plan"),
                &self.dir.path().join("version"),
            )
        }

        fn read(&self, file: &str) -> Result<Option<String>> {
            Ok(fs::read_to_string(self.dir.path().join("version").join(file)).ok())
        }
    }

    fn plan(channel: Channel, version: &str, targets: Vec<Target>) -> Plan {
        Plan {
            channel,
            version: Version::parse(version).unwrap(),
            tag: format!("v{version}"),
            commit: "release".into(),
            publish: true,
            targets,
        }
    }

    #[test]
    fn index_writes_targets_and_replays_idempotently() -> Result<()> {
        let fixture = Fixture::new(plan(Channel::Stable, "0.8.0", vec![
            Target {
                channel: Channel::Stable,
                expected: Some(id("0.7.6", "old")),
            },
            Target {
                channel: Channel::Nightly,
                expected: None,
            },
        ]))?;
        fixture.publish("stable.json", "0.7.6", "old")?;
        fixture.publish("beta.json", "0.8.0-beta.1", "beta")?;
        fs::write(fixture.dir.path().join("version/unrelated.txt"), "keep")?;

        fixture.check()?;
        fixture.update()?;
        let stable = fixture.read("stable.json")?.unwrap();
        assert!(stable.contains("\"version\": \"0.8.0\""));
        assert!(
            fixture
                .read("stable.txt")?
                .unwrap()
                .starts_with("VERSION=0.8.0\nTAG=v0.8.0\nCOMMIT=release\n")
        );
        assert_eq!(fixture.read("nightly.json")?, fixture.read("alpha.json")?);
        assert!(fixture.read("nightly.json")?.is_some());
        // Not a target: left as it was.
        assert!(fixture.read("beta.json")?.unwrap().contains("0.8.0-beta.1"));
        assert_eq!(fixture.read("unrelated.txt")?.unwrap(), "keep");

        // A retry after the index was written changes nothing and succeeds.
        fs::write(fixture.dir.path().join("version/stable.txt"), "interrupted")?;
        fixture.check()?;
        fixture.update()?;
        assert_eq!(fixture.read("stable.json")?.unwrap(), stable);
        assert!(
            fixture
                .read("stable.txt")?
                .unwrap()
                .starts_with("VERSION=0.8.0")
        );
        Ok(())
    }

    #[test]
    fn index_refuses_to_write_after_another_release() -> Result<()> {
        let fixture = Fixture::new(plan(Channel::Beta, "0.8.0-beta.2", vec![Target {
            channel: Channel::Beta,
            expected: Some(id("0.8.0-beta.1", "b1")),
        }]))?;
        fixture.publish("beta.json", "0.8.0-beta.3", "b3")?;
        let before = fixture.read("beta.json")?;

        assert!(fixture.check().is_err());
        assert!(fixture.update().is_err());
        assert_eq!(fixture.read("beta.json")?, before);
        Ok(())
    }

    #[test]
    fn index_validates_the_whole_bundle_before_writing() -> Result<()> {
        let fixture = Fixture::new(plan(Channel::Stable, "0.8.0", vec![
            Target {
                channel: Channel::Stable,
                expected: None,
            },
            Target {
                channel: Channel::Beta,
                expected: None,
            },
        ]))?;
        write_manifest(
            fixture
                .dir
                .path()
                .join(BUNDLE)
                .join(BUNDLE_INDEX)
                .join("beta.json"),
            &manifest("0.9.0", "release"),
        )?;
        assert!(fixture.update().is_err());
        assert_eq!(fixture.read("stable.json")?, None);
        Ok(())
    }
}
