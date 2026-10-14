//! Changelog and version files of a stable release.

use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result, bail, ensure};
use semver::Version;

use super::{
    PrepareOptions, cliff_config, git,
    selection::{self, CARGO_MANIFEST, VersionSelection},
    version::dev_branch_version,
};
use crate::{cmd::CommandExt, github};

const CHANGELOG: &str = "CHANGELOG.md";
/// The only files a release preparation may change.
const RELEASE_FILES: [&str; 3] = [CHANGELOG, "Cargo.lock", CARGO_MANIFEST];

/// Bump the Cargo version if needed and regenerate this version's changelog section.
pub fn prepare(options: PrepareOptions) -> Result<()> {
    let version = prepare_files(&options.source, &options.branch, &options.version)?;
    github::set_output("version", &version.to_string())
}

fn prepare_files(source: &Path, branch: &str, selection: &VersionSelection) -> Result<Version> {
    let (version, previous_tag) = selection::select(source, selection)?;
    ensure!(
        previous_tag != format!("v{version}"),
        "No changes require an automatic release since {previous_tag}; specify a version to release anyway"
    );
    if let Some(base) = dev_branch_version(branch) {
        ensure!(
            version == base,
            "{branch} can only release {base}, selected {version}"
        );
    } else {
        ensure!(
            branch == "main",
            "Stable releases are prepared on main or a dev-X.Y branch, not {branch}"
        );
    }

    let current = selection::cargo_version(source)?;
    if current != version {
        let manifest = source.join(CARGO_MANIFEST);
        let content = fs::read_to_string(&manifest)?;
        fs::write(&manifest, bump_manifest(&content, &current, &version)?)?;
        Command::new("cargo")
            .current_dir(source)
            .args(["update", "-p", "maa-cli", "--precise"])
            .arg(version.to_string())
            .arg("--offline")
            .run()?;
    }

    // A previous preparation may already be on the branch without a published
    // tag. Remove only its section; git-cliff owns generation and prepending.
    let changelog = source.join(CHANGELOG);
    let content = fs::read_to_string(&changelog).context("Failed to read CHANGELOG.md")?;
    fs::write(&changelog, remove_section(&content, &version))?;
    git_cliff(source, &version)?
        .args(["--prepend", CHANGELOG, &format!("{previous_tag}..HEAD")])
        .run()?;

    let changed = git(source).args(["diff", "--name-only"]).read()?;
    ensure!(
        !changed.is_empty(),
        "Release files already match v{version}; no preparation is needed"
    );
    for file in changed.lines() {
        ensure!(
            RELEASE_FILES.contains(&file),
            "Unexpected release preparation change: {file}"
        );
    }
    Ok(version)
}

/// Check that the changelog section of a stable release matches its commits.
pub fn check(source: &Path, version: &Version) -> Result<()> {
    let (_, previous_tag) = selection::previous_stable_tag(source, Some(version))?;
    let content = fs::read_to_string(source.join(CHANGELOG))?;
    let actual = section(&content, version)
        .with_context(|| format!("{CHANGELOG} has no section for {version}"))?;
    let expected = git_cliff(source, version)?
        .args(["--strip", "all", &format!("{previous_tag}..HEAD")])
        .read()?;
    if actual.trim_end() != expected.trim_end() {
        bail!(
            "The changelog of v{version} is stale. Run Prepare Stable Release again.\n--- {CHANGELOG}\n{actual}\n--- expected\n{expected}"
        );
    }
    Ok(())
}

/// Release notes of a stable release: its changelog section.
pub fn stable_notes(source: &Path, version: &Version) -> Result<String> {
    let content = fs::read_to_string(source.join(CHANGELOG))?;
    section(&content, version)
        .map(str::to_owned)
        .with_context(|| format!("{CHANGELOG} has no section for {version}"))
}

/// Release notes of a pre-release: everything since the latest reachable stable tag.
///
/// An explicit range is used because a dev branch that merges main would
/// otherwise list fixes already published in the merged stable tags.
pub fn prerelease_notes(source: &Path, version: &Version, tag: &str) -> Result<String> {
    let (_, previous_tag) = selection::previous_stable_tag(source, None)?;
    git_cliff(source, version)?
        .env("TAG", tag)
        .args(["--strip", "all", &format!("{previous_tag}..HEAD")])
        .read()
}

fn git_cliff(source: &Path, version: &Version) -> Result<Command> {
    let config = cliff_config();
    let mut command = Command::new("git-cliff");
    command
        .current_dir(source)
        .env("VERSION", version.to_string())
        .arg("--config")
        .arg(config);
    if cfg!(test) {
        // Tests run on throwaway repositories and must not query GitHub.
        command.arg("--offline");
    }
    if version.pre.is_empty() {
        command.args(["--tag", &format!("v{version}")]);
    }
    Ok(command)
}

fn bump_manifest(content: &str, current: &Version, version: &Version) -> Result<String> {
    let from = format!("version = \"{current}\"");
    let to = format!("version = \"{version}\"");
    let mut replaced = false;
    let lines: Vec<&str> = content
        .lines()
        .map(|line| {
            if !replaced && line == from {
                replaced = true;
                to.as_str()
            } else {
                line
            }
        })
        .collect();
    ensure!(replaced, "maa-cli Cargo.toml has no `{from}` line");
    Ok(lines.join("\n") + "\n")
}

fn heading(version: &Version) -> String {
    format!("## Release {version}")
}

/// The `## Release X.Y.Z` section of a changelog, without trailing blank lines.
fn section<'a>(content: &'a str, version: &Version) -> Option<&'a str> {
    let heading = heading(version);
    let start = content.find(&format!("{heading}\n"))?;
    let rest = &content[start..];
    let end = rest[heading.len()..]
        .find("\n## ")
        .map_or(rest.len(), |i| heading.len() + i + 1);
    Some(rest[..end].trim_end())
}

fn remove_section(content: &str, version: &Version) -> String {
    let heading = heading(version);
    let mut skip = false;
    let mut out = String::new();
    for line in content.split_inclusive('\n') {
        if line.starts_with("## ") {
            skip = line.trim_end() == heading;
        }
        if !skip {
            out.push_str(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "# Release Notes\n\n## Release 0.7.6\n\n### Bug Fixes\n\n- Fix a\n\n## Release 0.7.5\n\n- Old\n";

    #[test]
    fn sections_are_found_and_removed_by_exact_heading() {
        let v076 = Version::new(0, 7, 6);
        let v075 = Version::new(0, 7, 5);
        assert_eq!(
            section(LOG, &v076),
            Some("## Release 0.7.6\n\n### Bug Fixes\n\n- Fix a")
        );
        assert_eq!(section(LOG, &v075), Some("## Release 0.7.5\n\n- Old"));
        assert_eq!(section(LOG, &Version::new(0, 7, 0)), None);
        assert_eq!(
            remove_section(LOG, &v076),
            "# Release Notes\n\n## Release 0.7.5\n\n- Old\n"
        );
        assert_eq!(remove_section(LOG, &Version::new(0, 9, 0)), LOG);
    }

    #[test]
    fn manifest_bump_changes_only_the_package_version() -> Result<()> {
        let manifest = "[package]\nname = \"maa-cli\"\nversion = \"0.7.5\"\n\n[dependencies]\nfoo = { version = \"0.7.5\" }\n";
        let bumped = bump_manifest(manifest, &Version::new(0, 7, 5), &Version::new(0, 7, 6))?;
        assert_eq!(
            bumped,
            manifest.replacen("version = \"0.7.5\"\n", "version = \"0.7.6\"\n", 1)
        );
        assert!(bump_manifest(manifest, &Version::new(0, 7, 4), &Version::new(0, 7, 6)).is_err());
        Ok(())
    }

    use crate::release::test_repo::Repo;

    #[test]
    fn prepare_and_check_a_stable_release() -> Result<()> {
        let repo = Repo::released("0.7.5");
        let source = repo.path();
        repo.commit("docs: clarify usage");
        assert!(prepare_files(source, "main", &VersionSelection::Auto).is_err());

        // Before 1.0, features only advance the patch version.
        repo.commit("feat: add option A");
        repo.commit("Update README.md");
        repo.commit("chore(deps): bump example\n\nIncludes upstream security fixes.");
        let v076 = Version::new(0, 7, 6);
        assert_eq!(
            prepare_files(source, "main", &VersionSelection::Auto)?,
            v076
        );
        assert_eq!(selection::cargo_version(source)?, v076);
        let log = fs::read_to_string(source.join(CHANGELOG))?;
        assert!(log.contains("## Release 0.7.6"), "{log}");
        assert!(log.contains("Add option A"), "{log}");
        assert!(!log.to_lowercase().contains("bump example"), "{log}");
        assert!(log.contains("## Release 0.7.5"), "{log}");
        assert!(prepare_files(source, "dev-0.8", &VersionSelection::Auto).is_err());

        repo.git(&["commit", "-qam", "chore(release): prepare for v0.7.6"]);
        check(source, &v076)?;
        assert!(stable_notes(source, &v076)?.starts_with("## Release 0.7.6"));

        // Merged after preparation: the final check refuses to publish.
        repo.commit("fix: repair B");
        assert!(check(source, &v076).is_err());
        // Preparing again refreshes the section without bumping the version twice.
        assert_eq!(
            prepare_files(source, "main", &VersionSelection::Auto)?,
            v076
        );
        repo.git(&["commit", "-qam", "chore(release): prepare for v0.7.6"]);
        check(source, &v076)?;
        let log = fs::read_to_string(source.join(CHANGELOG))?;
        assert_eq!(log.matches("## Release 0.7.6").count(), 1, "{log}");
        assert!(log.contains("Repair B"), "{log}");

        // A replay after tagging still compares against the previous release.
        repo.git(&["tag", "v0.7.6"]);
        check(source, &v076)?;
        Ok(())
    }

    #[test]
    fn a_dev_branch_releases_its_declared_version() -> Result<()> {
        let repo = Repo::released("0.7.5");
        let source = repo.path();
        repo.git(&["checkout", "-qb", "dev-0.8"]);
        repo.write_version("0.8.0");
        repo.git(&["commit", "-qam", "chore: bump maa-cli to 0.8.0"]);
        repo.commit("feat: config v2");
        assert_eq!(
            prepare_files(source, "dev-0.8", &VersionSelection::Auto)?,
            Version::new(0, 8, 0)
        );
        let changed = repo.git(&["diff", "--name-only"]);
        assert_eq!(changed, CHANGELOG);
        assert!(
            prepare_files(
                source,
                "dev-0.8",
                &VersionSelection::Explicit(Version::new(0, 9, 0))
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn prerelease_notes_skip_changes_already_released_on_main() -> Result<()> {
        let repo = Repo::released("0.7.5");
        let source = repo.path();
        repo.git(&["checkout", "-qb", "dev-0.8"]);
        repo.commit("feat: config v2");
        repo.git(&["checkout", "-q", "main"]);
        repo.commit("fix: released on main");
        repo.git(&["tag", "v0.7.6"]);
        repo.git(&["checkout", "-q", "dev-0.8"]);
        repo.git(&["merge", "-q", "--no-edit", "main"]);
        let version = Version::parse("0.8.0-beta.1")?;
        let notes = prerelease_notes(source, &version, "v0.8.0-beta.1")?;
        assert!(notes.contains("Config v2"), "{notes}");
        assert!(!notes.contains("Released on main"), "{notes}");
        assert!(notes.contains("0.8.0-beta.1"), "{notes}");
        Ok(())
    }
}
