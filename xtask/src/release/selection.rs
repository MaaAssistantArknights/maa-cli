use std::{path::Path, process::Command};

use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use semver::Version;
use serde::Deserialize;

use super::git;
use crate::cmd::CommandExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Bump {
    /// Keep a newer Cargo version, otherwise infer the bump from the commits
    Auto,
    Minor,
    Major,
}

/// Select the next stable version of `source` and the latest stable tag it builds on.
pub fn select(source: &Path, bump: Bump) -> Result<(Version, String)> {
    let (latest, previous_tag) = previous_stable_tag(source, None)?;
    let cargo = cargo_version(source)?;
    let version = choose(bump, &cargo, &latest, |level| {
        let range = format!("{previous_tag}..HEAD");
        let config = super::cliff_config();
        let config = config.to_str().context("cliff.toml path is not UTF-8")?;
        if level.is_none() {
            let context = Command::new("git-cliff")
                .current_dir(source)
                .args(["--config", config, "--offline", &range, "--context"])
                .read()?;
            if !has_version_changes(&context)? {
                return Ok(latest.clone());
            }
        }
        let mut command = Command::new("git-cliff");
        command.current_dir(source).args([
            "--config",
            config,
            "--offline",
            &range,
            "--bumped-version",
        ]);
        if let Some(level) = level {
            command.args(["--bump", level]);
        }
        let output = command.read()?;
        let version = Version::parse(output.strip_prefix('v').unwrap_or(&output))
            .context("git-cliff returned an invalid bumped version")?;
        ensure_stable(&version)?;
        Ok(version)
    })?;
    ensure!(
        version >= latest,
        "Selected version {version} is older than published stable {latest}"
    );
    if bump != Bump::Auto {
        ensure!(
            version > latest,
            "Requested bump must produce a version newer than published stable {latest}"
        );
    }
    Ok((version, previous_tag))
}

fn choose(
    bump: Bump,
    cargo: &Version,
    latest: &Version,
    infer: impl FnOnce(Option<&str>) -> Result<Version>,
) -> Result<Version> {
    ensure_stable(cargo)?;
    match bump {
        Bump::Auto if cargo > latest => Ok(cargo.clone()),
        Bump::Auto => infer(None),
        Bump::Minor => infer(Some("minor")),
        Bump::Major => infer(Some("major")),
    }
}

/// The latest stable tag reachable from HEAD of `source`, ignoring the tag of `excluding`.
///
/// A release that is replayed after its tag was created still compares against
/// the release before it.
pub fn previous_stable_tag(
    source: &Path,
    excluding: Option<&Version>,
) -> Result<(Version, String)> {
    let tags = git(source).args(["tag", "--merged", "HEAD"]).read()?;
    latest_stable_tag(&tags, excluding)
}

fn latest_stable_tag(tags: &str, excluding: Option<&Version>) -> Result<(Version, String)> {
    tags.lines()
        .filter_map(|tag| {
            let version = Version::parse(tag.strip_prefix('v')?).ok()?;
            (version.pre.is_empty() && version.build.is_empty() && Some(&version) != excluding)
                .then(|| (version, tag.to_owned()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .context("No reachable stable vX.Y.Z tag found")
}

#[derive(Deserialize)]
struct CliffRelease {
    commits: Vec<CliffCommit>,
}

#[derive(Deserialize)]
struct CliffCommit {
    breaking: bool,
    raw_message: String,
}

fn has_version_changes(context: &str) -> Result<bool> {
    let releases: Vec<CliffRelease> =
        serde_json::from_str(context).context("Failed to parse git-cliff release context")?;
    Ok(releases
        .iter()
        .flat_map(|release| &release.commits)
        .any(|commit| {
            let kind = commit
                .raw_message
                .split(['(', ':', '!'])
                .next()
                .unwrap_or_default();
            commit.breaking || matches!(kind, "feat" | "fix")
        }))
}

pub fn ensure_stable(version: &Version) -> Result<()> {
    ensure!(
        version.pre.is_empty() && version.build.is_empty(),
        "Expected a stable version without pre-release or build metadata: {version}"
    );
    Ok(())
}

pub const CARGO_MANIFEST: &str = "crates/maa-cli/Cargo.toml";

pub fn cargo_version(source: &Path) -> Result<Version> {
    let content = std::fs::read_to_string(source.join(CARGO_MANIFEST))
        .context("Failed to read maa-cli Cargo.toml")?;
    let manifest: toml::Value =
        toml::from_str(&content).context("Failed to parse maa-cli Cargo.toml")?;
    let version = manifest
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .context("maa-cli Cargo.toml has no package version")?;
    Version::parse(version).context("Failed to parse maa-cli Cargo version")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_keeps_a_newer_cargo_version() -> Result<()> {
        let cargo = Version::new(0, 8, 0);
        let version = choose(Bump::Auto, &cargo, &Version::new(0, 7, 5), |_| {
            anyhow::bail!("must not infer")
        })?;
        assert_eq!(version, cargo);
        Ok(())
    }

    #[test]
    fn auto_infers_when_cargo_is_released() -> Result<()> {
        let latest = Version::new(0, 7, 5);
        let version = choose(Bump::Auto, &latest, &latest, |level| {
            assert_eq!(level, None);
            Ok(Version::new(0, 7, 6))
        })?;
        assert_eq!(version, Version::new(0, 7, 6));
        Ok(())
    }

    #[test]
    fn explicit_bumps_are_delegated_to_git_cliff() -> Result<()> {
        for (bump, expected) in [(Bump::Minor, "minor"), (Bump::Major, "major")] {
            let version = choose(
                bump,
                &Version::new(0, 9, 0),
                &Version::new(0, 7, 5),
                |level| {
                    assert_eq!(level, Some(expected));
                    Ok(Version::new(1, 0, 0))
                },
            )?;
            assert_eq!(version, Version::new(1, 0, 0));
        }
        Ok(())
    }

    #[test]
    fn only_fixes_features_or_breaking_changes_trigger_auto_bumps() -> Result<()> {
        assert!(!has_version_changes(
            r#"[{"commits":[{"breaking":false,"raw_message":"docs: usage"},{"breaking":false,"raw_message":"refactor(core): simplify"},{"breaking":false,"raw_message":"chore: cleanup"}]}]"#
        )?);
        for message in ["fix: bug", "feat(cli): flag", "fix!: removed"] {
            assert!(has_version_changes(
                &serde_json::json!([{"commits":[{"breaking":false,"raw_message":message}]}])
                    .to_string()
            )?);
        }
        assert!(has_version_changes(
            r#"[{"commits":[{"breaking":true,"raw_message":"refactor!: remove API"}]}]"#
        )?);
        Ok(())
    }

    #[test]
    fn stable_tag_selection_ignores_prereleases_and_sorts_numerically() -> Result<()> {
        assert_eq!(
            latest_stable_tag("v0.9.0\nv0.10.0\nv1.0.0-beta.1\nnightly", None)?,
            (Version::new(0, 10, 0), "v0.10.0".into())
        );
        assert_eq!(
            latest_stable_tag("v0.9.0\nv0.10.0", Some(&Version::new(0, 10, 0)))?,
            (Version::new(0, 9, 0), "v0.9.0".into())
        );
        Ok(())
    }
}
