use std::{num::NonZeroU16, path::Path, process::Command};

use anyhow::{Context, Result, bail, ensure};
use maa_version::{VersionManifest, cli::Details};
use semver::{BuildMetadata, Prerelease, Version};

use super::{Channel, ReleaseVersionOptions, selection};
use crate::{cmd::CommandExt, github};

pub fn run(options: ReleaseVersionOptions) -> Result<()> {
    let commit_sha = get_commit_sha()?;
    ensure!(
        commit_sha == options.commit,
        "Checked-out commit {commit_sha} does not match expected commit {}",
        options.commit
    );
    let commit_short_sha = get_commit_short_sha()?;
    let channel = options.channel;
    let publish = options.publish;
    let candidate = if channel == Channel::Stable {
        selection::cargo_version()?
    } else {
        let (candidate, previous_tag) = selection::select(&options.version)?;
        let commits_since = Command::new("git")
            .args(["rev-list", "--count", &format!("{previous_tag}..HEAD")])
            .read()?
            .parse()
            .context("Failed to count commits since the latest stable tag")?;
        // A stable tag can be published before the index job completes. Do not
        // create an older prerelease just because stable.json still lags behind.
        let Some(base) = prerelease_base(channel, candidate, &previous_tag, commits_since) else {
            println!("No releasable changes since {previous_tag}");
            github::set_output("skip", "true")?;
            return Ok(());
        };
        base
    };
    selection::ensure_stable(&candidate)?;

    // Check if version directory exists
    ensure!(Path::new("version").exists(), "version directory not found");

    let version_file = channel.version_file();
    let stable_manifest = read_version_manifest(Channel::Stable.version_file())?;
    ensure!(
        stable_manifest.version.pre.is_empty() && stable_manifest.version.build.is_empty(),
        "version/stable.json contains an invalid stable version: {}",
        stable_manifest.version
    );
    ensure!(
        candidate >= stable_manifest.version,
        "Selected stable v{} is older than version/stable.json v{}",
        candidate,
        stable_manifest.version
    );

    if channel != Channel::Stable && candidate == stable_manifest.version {
        println!("No releasable changes since v{}", stable_manifest.version);
        github::set_output("skip", "true")?;
        return Ok(());
    }
    let manifest = read_version_manifest(&version_file)?;
    if channel == Channel::Stable {
        validate_stable_release(
            &candidate,
            &manifest.version,
            &commit_sha,
            &manifest.details.commit,
        )?;
    }
    if channel == Channel::Beta {
        let tags = Command::new("git").args(["tag", "--list", "v*"]).read()?;
        ensure_beta_index_current(&manifest.version, &tags)?;
    }
    if channel == Channel::Alpha {
        check_nightly_index(&manifest)?;
    }
    if skip_existing_prerelease(
        channel,
        &candidate,
        &manifest.version,
        &commit_sha,
        &manifest.details.commit,
    ) {
        println!("No new commits, skipping all steps");
        github::set_output("skip", "true")?;
        return Ok(());
    }

    let published_version = manifest.version;

    let (version, tag) =
        compute_version(channel, &candidate, &published_version, &commit_short_sha)?;

    let channel_str = channel.as_str();
    println!(
        "Release version {version} with tag {tag} to channel {channel_str} (publish: {publish})"
    );

    github::set_outputs(&[
        ("commit", &commit_sha),
        ("channel", channel.as_str()),
        ("version", &version.to_string()),
        ("tag", &tag),
        ("publish", if publish { "true" } else { "false" }),
        ("profile", if publish { "release-lto" } else { "dev" }),
        ("skip", "false"),
    ])?;

    Ok(())
}

/// Choose the base version of a prerelease, or `None` to skip it.
///
/// Without a version-bumping change since the latest stable tag, beta has
/// nothing to release, while nightly still builds every new commit against
/// the next patch version.
fn prerelease_base(
    channel: Channel,
    candidate: Version,
    previous_tag: &str,
    commits_since: u64,
) -> Option<Version> {
    if previous_tag != format!("v{candidate}") {
        return Some(candidate);
    }
    (channel == Channel::Alpha && commits_since > 0)
        .then(|| Version::new(candidate.major, candidate.minor, candidate.patch + 1))
}

fn get_commit_sha() -> Result<String> {
    Command::new("git").args(["rev-parse", "HEAD"]).read()
}

fn get_commit_short_sha() -> Result<String> {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .read()
}

fn validate_stable_release(
    candidate: &Version,
    published: &Version,
    commit: &str,
    published_commit: &str,
) -> Result<()> {
    ensure!(
        candidate > published || (candidate == published && commit == published_commit),
        "Stable version {candidate} must be newer than {published}, or replay the same release commit"
    );
    Ok(())
}

fn skip_existing_prerelease(
    channel: Channel,
    candidate: &Version,
    published: &Version,
    commit: &str,
    published_commit: &str,
) -> bool {
    channel != Channel::Stable
        && is_same_core_version(candidate, published)
        && commit == published_commit
}

fn ensure_beta_index_current(indexed: &Version, tags: &str) -> Result<()> {
    for tag in tags.lines() {
        let Some(version) = tag
            .strip_prefix('v')
            .and_then(|version| Version::parse(version).ok())
        else {
            continue;
        };
        if version
            .pre
            .as_str()
            .strip_prefix("beta.")
            .is_some_and(|counter| counter.parse::<u16>().is_ok())
        {
            ensure!(
                version <= *indexed,
                "Beta tag {tag} is newer than version/beta.json ({indexed}); rerun the previous release's index job before allocating another beta"
            );
        }
    }
    Ok(())
}

fn check_nightly_index(indexed: &VersionManifest<Details>) -> Result<()> {
    if Command::new("git")
        .args(["tag", "--list", "nightly"])
        .read()?
        .is_empty()
    {
        return Ok(());
    }
    // The movable nightly tag alone cannot tell us which counter was published.
    // Its release name records the full version even when the index push failed.
    let release = Command::new("gh")
        .args(["api", "repos/{owner}/{repo}/releases/tags/nightly"])
        .read()
        .context("Cannot inspect the existing nightly release; repair it before allocating another alpha")?;
    #[derive(serde::Deserialize)]
    struct NightlyRelease {
        name: String,
    }
    let release: NightlyRelease = serde_json::from_str(&release)?;
    let published = Version::parse(release.name.strip_prefix('v').unwrap_or(&release.name))
        .context("The nightly release name must contain its published version")?;
    let commit = Command::new("git")
        .args(["rev-parse", "refs/tags/nightly^{}"])
        .read()?;
    ensure_nightly_index_current(indexed, &published, &commit)
}

fn ensure_nightly_index_current(
    indexed: &VersionManifest<Details>,
    published: &Version,
    commit: &str,
) -> Result<()> {
    use std::cmp::Ordering;

    ensure!(
        match published.cmp_precedence(&indexed.version) {
            Ordering::Less => true, // A later beta or stable also advances the alpha index.
            Ordering::Equal => published == &indexed.version && commit == indexed.details.commit,
            Ordering::Greater => false,
        },
        "Nightly {published} ({commit}) is not recorded in version/alpha.json; rerun its index job before allocating another alpha"
    );
    Ok(())
}

fn read_version_manifest(file: impl AsRef<Path>) -> Result<VersionManifest<Details>> {
    let file = file.as_ref();
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read {}", file.display()))?;

    serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", file.display()))
}

fn compute_version(
    channel: Channel,
    stable_version: &Version,
    published_version: &Version,
    commit_short_sha: &str,
) -> Result<(Version, String)> {
    match channel {
        Channel::Stable => {
            let tag = format!("v{stable_version}");
            Ok((stable_version.clone(), tag))
        }
        Channel::Beta => {
            ensure_candidate_not_older(stable_version, published_version)?;
            let published_prerelease = PrereleaseVersion::try_from(&published_version.pre)?;
            let mut version = stable_version.clone();
            version.build = BuildMetadata::EMPTY;

            if is_same_core_version(stable_version, published_version) {
                version.pre = published_prerelease.bump_beta()?.into();
            } else {
                version.pre = Prerelease::new("beta.1")?;
            }

            let tag = format!("v{}", version);
            Ok((version, tag))
        }
        Channel::Alpha => {
            ensure_candidate_not_older(stable_version, published_version)?;
            let published_prerelease = PrereleaseVersion::try_from(&published_version.pre)?;
            let mut version = stable_version.clone();
            version.build = BuildMetadata::new(&format!("sha.{}", commit_short_sha))?;

            if is_same_core_version(stable_version, published_version) {
                version.pre = published_prerelease.bump_alpha()?.into();
            } else {
                version.pre = Prerelease::new("alpha.1")?;
            }

            Ok((version, "nightly".to_string()))
        }
    }
}

// Pre-release helper

#[derive(Debug, Clone, Default)]
struct PrereleaseVersion {
    beta: Option<NonZeroU16>,
    alpha: Option<NonZeroU16>,
}

impl PrereleaseVersion {
    fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Ok(Self::default());
        }

        if let Some(rest) = s.strip_prefix("beta.") {
            let parts: Vec<&str> = rest.split('.').collect();

            if parts.len() == 1 {
                let beta = parse_counter(parts[0], "beta")?;
                Ok(Self {
                    beta: Some(beta),
                    alpha: None,
                })
            } else if parts.len() == 3 && parts[1] == "alpha" {
                let beta = parse_counter(parts[0], "beta")?;
                let alpha = parse_counter(parts[2], "alpha")?;
                Ok(PrereleaseVersion {
                    beta: Some(beta),
                    alpha: Some(alpha),
                })
            } else {
                bail!("Unsupported pre-release version: {s}")
            }
        } else if let Some(rest) = s.strip_prefix("alpha.") {
            let alpha = parse_counter(rest, "alpha")?;
            Ok(Self {
                beta: None,
                alpha: Some(alpha),
            })
        } else {
            bail!("Unsupported pre-release version: {s}")
        }
    }
}

impl TryFrom<&Prerelease> for PrereleaseVersion {
    type Error = anyhow::Error;

    fn try_from(prerelease: &Prerelease) -> Result<Self> {
        Self::parse(prerelease.as_str())
    }
}

impl From<PrereleaseVersion> for Prerelease {
    fn from(version: PrereleaseVersion) -> Self {
        // Fixed identifiers and nonzero integer counters always form valid semver prereleases.
        match (version.beta, version.alpha) {
            (None, None) => Prerelease::EMPTY,
            (None, Some(alpha)) => Prerelease::new(&format!("alpha.{}", alpha.get())).unwrap(),
            (Some(beta), None) => Prerelease::new(&format!("beta.{}", beta.get())).unwrap(),
            (Some(beta), Some(alpha)) => {
                Prerelease::new(&format!("beta.{}.alpha.{}", beta.get(), alpha.get())).unwrap()
            }
        }
    }
}

impl PrereleaseVersion {
    fn bump_beta(self) -> Result<Self> {
        Ok(PrereleaseVersion {
            beta: Some(next_counter(self.beta, "beta")?),
            alpha: None,
        })
    }

    fn bump_alpha(self) -> Result<Self> {
        Ok(PrereleaseVersion {
            beta: self.beta,
            alpha: Some(next_counter(self.alpha, "alpha")?),
        })
    }
}

fn next_counter(current: Option<NonZeroU16>, name: &str) -> Result<NonZeroU16> {
    current
        .map_or(0, NonZeroU16::get)
        .checked_add(1)
        .and_then(NonZeroU16::new)
        .with_context(|| format!("{name} release counter exhausted"))
}

fn is_same_core_version(v1: &Version, v2: &Version) -> bool {
    v1.major == v2.major && v1.minor == v2.minor && v1.patch == v2.patch
}

fn ensure_candidate_not_older(candidate: &Version, published: &Version) -> Result<()> {
    let candidate_core = (candidate.major, candidate.minor, candidate.patch);
    let published_core = (published.major, published.minor, published.patch);
    ensure!(
        candidate_core >= published_core,
        "Derived stable v{candidate} is older than published pre-release v{published}"
    );
    Ok(())
}

fn parse_counter(value: &str, name: &str) -> Result<NonZeroU16> {
    value
        .parse::<u16>()
        .ok()
        .and_then(NonZeroU16::new)
        .with_context(|| format!("Invalid {name} counter: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nightly_builds_new_commits_without_a_version_bump() {
        let stable = Version::new(0, 8, 0);
        assert_eq!(
            prerelease_base(Channel::Alpha, stable.clone(), "v0.8.0", 2),
            Some(Version::new(0, 8, 1))
        );
        assert_eq!(
            prerelease_base(Channel::Alpha, stable.clone(), "v0.8.0", 0),
            None
        );
        assert_eq!(
            prerelease_base(Channel::Beta, stable.clone(), "v0.8.0", 2),
            None
        );
        assert_eq!(
            prerelease_base(Channel::Beta, Version::new(0, 9, 0), "v0.8.0", 2),
            Some(Version::new(0, 9, 0))
        );
    }

    #[test]
    fn beta_uses_the_derived_stable_version_without_changing_it() -> Result<()> {
        let stable = Version::new(0, 8, 0);
        let published = Version::parse("0.8.0-beta.2")?;

        let (version, tag) = compute_version(Channel::Beta, &stable, &published, "abc1234")?;

        assert_eq!(version, Version::parse("0.8.0-beta.3")?);
        assert_eq!(tag, "v0.8.0-beta.3");
        assert_eq!(stable, Version::new(0, 8, 0));
        Ok(())
    }

    #[test]
    fn alpha_starts_again_when_the_stable_candidate_changes() -> Result<()> {
        let stable = Version::new(0, 9, 0);
        let published = Version::parse("0.8.0-beta.2.alpha.4+sha.old")?;

        let (version, tag) = compute_version(Channel::Alpha, &stable, &published, "abc1234")?;

        assert_eq!(version, Version::parse("0.9.0-alpha.1+sha.abc1234")?);
        assert_eq!(tag, "nightly");
        Ok(())
    }

    #[test]
    fn beta_starts_again_when_the_base_changes() -> Result<()> {
        let (version, tag) = compute_version(
            Channel::Beta,
            &Version::new(0, 9, 0),
            &Version::parse("0.8.0-beta.2")?,
            "abc1234",
        )?;
        assert_eq!(version, Version::parse("0.9.0-beta.1")?);
        assert_eq!(tag, "v0.9.0-beta.1");
        Ok(())
    }

    #[test]
    fn same_commit_only_skips_the_same_prerelease_base() -> Result<()> {
        let published = Version::parse("0.8.0-beta.2")?;
        assert!(skip_existing_prerelease(
            Channel::Beta,
            &Version::new(0, 8, 0),
            &published,
            "same",
            "same"
        ));
        assert!(!skip_existing_prerelease(
            Channel::Beta,
            &Version::new(0, 9, 0),
            &published,
            "same",
            "same"
        ));
        assert!(!skip_existing_prerelease(
            Channel::Stable,
            &Version::new(0, 8, 0),
            &Version::new(0, 8, 0),
            "same",
            "same"
        ));
        Ok(())
    }

    #[test]
    fn stable_replay_requires_the_original_commit() -> Result<()> {
        let published = Version::new(0, 8, 0);
        validate_stable_release(&published, &published, "same", "same")?;
        assert!(validate_stable_release(&published, &published, "new", "old").is_err());
        assert!(
            validate_stable_release(&Version::new(0, 7, 6), &published, "same", "same").is_err()
        );
        validate_stable_release(&Version::new(0, 9, 0), &published, "new", "old")?;
        Ok(())
    }

    #[test]
    fn newer_beta_tags_require_index_recovery() -> Result<()> {
        let indexed = Version::parse("0.8.0-beta.2")?;
        ensure_beta_index_current(&indexed, "v0.8.0-beta.1\nv0.8.0-beta.2\nnightly\nv0.9.0")?;
        assert!(ensure_beta_index_current(&indexed, "v0.8.0-beta.3").is_err());
        assert!(ensure_beta_index_current(&indexed, "v0.9.0-beta.1").is_err());
        Ok(())
    }

    #[test]
    fn nightly_requires_index_recovery_before_allocating_another_alpha() -> Result<()> {
        let mut indexed = VersionManifest {
            version: Version::parse("0.8.0-alpha.1+sha.old")?,
            details: Details {
                tag: "nightly".into(),
                commit: "old".into(),
                assets: Default::default(),
            },
        };
        let published = Version::parse("0.8.0-alpha.2+sha.new")?;
        assert!(ensure_nightly_index_current(&indexed, &published, "new").is_err());
        indexed.version = published.clone();
        assert!(ensure_nightly_index_current(&indexed, &published, "new").is_err());
        indexed.details.commit = "new".into();
        ensure_nightly_index_current(&indexed, &published, "new")?;
        assert!(
            ensure_nightly_index_current(
                &indexed,
                &Version::parse("0.8.0-alpha.2+sha.other")?,
                "other"
            )
            .is_err()
        );
        for newer in ["0.8.0-beta.1", "0.8.0", "0.9.0-alpha.1+sha.next"] {
            indexed.version = Version::parse(newer)?;
            ensure_nightly_index_current(&indexed, &published, "new")?;
        }
        Ok(())
    }

    #[test]
    fn exhausted_prerelease_counters_return_errors() -> Result<()> {
        for (channel, version) in [
            (Channel::Beta, "0.8.0-beta.65535"),
            (Channel::Alpha, "0.8.0-alpha.65535"),
            (Channel::Alpha, "0.8.0-beta.2.alpha.65535"),
        ] {
            assert!(
                compute_version(
                    channel,
                    &Version::new(0, 8, 0),
                    &Version::parse(version)?,
                    "abc1234"
                )
                .is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn prerelease_state_rejects_unknown_or_malformed_versions() -> Result<()> {
        for version in ["0.8.0-rc.1", "0.8.0-beta.bad", "0.8.0-beta.1.alpha"] {
            let published = Version::parse(version)?;
            assert!(
                compute_version(Channel::Beta, &Version::new(0, 8, 0), &published, "abc1234")
                    .is_err(),
                "{version} should be rejected"
            );
        }
        Ok(())
    }

    #[test]
    fn prerelease_candidate_must_not_move_backwards() -> Result<()> {
        let error = compute_version(
            Channel::Beta,
            &Version::new(0, 8, 0),
            &Version::parse("0.9.0-beta.1")?,
            "abc1234",
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("older than published pre-release")
        );
        Ok(())
    }
}
