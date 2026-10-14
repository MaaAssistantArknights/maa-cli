//! Version rules of the release model, kept free of I/O so they can be tested directly.
//!
//! - Stable is the Cargo version on main, published by a release PR.
//! - Beta and nightly come only from a single `dev-X.Y` branch whose Cargo version is `X.Y.0`: beta
//!   is `X.Y.0-beta.N`, nightly is `X.Y.0-nightly.C+sha.<short>` where `C` is the commit count of
//!   the branch.
//! - A release updates its own channel and every less stable channel whose current version it
//!   supersedes.

use anyhow::{Context, Result, bail, ensure};
use semver::{BuildMetadata, Prerelease, Version};
use serde::{Deserialize, Serialize};

use super::Channel;

/// A published release: what an index entry points to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub version: Version,
    pub commit: String,
}

/// Version declared by a `dev-X.Y` branch name.
pub fn dev_branch_version(branch: &str) -> Option<Version> {
    let (major, minor) = branch.strip_prefix("dev-")?.split_once('.')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(major) || !digits(minor) {
        return None;
    }
    Some(Version::new(major.parse().ok()?, minor.parse().ok()?, 0))
}

/// The single development branch among `git ls-remote --heads` output, as `(branch, commit)`.
///
/// Returns `None` when there is no `dev-*` branch and fails when there are several,
/// since beta and nightly can only follow one line at a time.
pub fn single_dev_branch(ls_remote: &str) -> Result<Option<(String, String)>> {
    let branches: Vec<(String, String)> = ls_remote
        .lines()
        .filter_map(|line| {
            let (commit, reference) = line.split_once('\t')?;
            let branch = reference.strip_prefix("refs/heads/")?;
            branch
                .starts_with("dev-")
                .then(|| (branch.to_owned(), commit.to_owned()))
        })
        .collect();
    match branches.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(one.clone())),
        _ => {
            let names: Vec<&str> = branches.iter().map(|(name, _)| name.as_str()).collect();
            bail!(
                "Only one dev branch may exist at a time, found: {}",
                names.join(", ")
            )
        }
    }
}

/// Check that a dev branch follows the naming convention and declares an unreleased version.
pub fn dev_base(branch: &str, cargo: &Version, stable: Option<&Version>) -> Result<Version> {
    let base = dev_branch_version(branch)
        .with_context(|| format!("Development branch {branch} must be named dev-X.Y"))?;
    ensure!(
        *cargo == base,
        "{branch} must set the maa-cli Cargo version to {base}, found {cargo}"
    );
    if let Some(stable) = stable {
        ensure!(
            base > *stable,
            "{branch} declares {base}, which is not newer than stable {stable}; it has been released, delete the branch"
        );
    }
    Ok(base)
}

/// The beta tag already pointing at the release commit, if any.
pub fn beta_at_commit<'a>(base: &Version, tags_at_commit: &'a str) -> Option<&'a str> {
    tags_at_commit
        .lines()
        .find(|tag| beta_number(base, tag).is_some())
}

/// The next beta of `base`, numbered after the existing `vX.Y.Z-beta.N` tags.
pub fn next_beta(base: &Version, tags: &str) -> Result<Version> {
    let last = tags
        .lines()
        .filter_map(|tag| beta_number(base, tag))
        .max()
        .unwrap_or(0);
    let next = last.checked_add(1).context("beta counter exhausted")?;
    let mut version = base.clone();
    version.pre = Prerelease::new(&format!("beta.{next}"))?;
    Ok(version)
}

fn beta_number(base: &Version, tag: &str) -> Option<u64> {
    let version = Version::parse(tag.strip_prefix('v')?).ok()?;
    if (version.major, version.minor, version.patch) != (base.major, base.minor, base.patch)
        || !version.build.is_empty()
    {
        return None;
    }
    version.pre.as_str().strip_prefix("beta.")?.parse().ok()
}

/// The nightly version of `base` at a commit.
pub fn nightly(base: &Version, commit_count: u64, short_sha: &str) -> Result<Version> {
    let mut version = base.clone();
    version.pre = Prerelease::new(&format!("nightly.{commit_count}"))?;
    version.build = BuildMetadata::new(&format!("sha.{short_sha}"))?;
    Ok(version)
}

/// Index channels a release updates.
///
/// The release's own channel and every less stable channel are candidates.
/// A candidate is updated when it is empty, when the release supersedes its
/// current version, or when it already holds this very release (a replay).
/// The release must at least update its own channel.
pub fn targets(
    channel: Channel,
    release: &Identity,
    head: impl Fn(Channel) -> Option<Identity>,
) -> Result<Vec<Channel>> {
    let mut targets = Vec::new();
    for candidate in Channel::ALL.into_iter().filter(|c| *c >= channel) {
        let current = head(candidate);
        let update = match &current {
            None => true,
            Some(current) if current == release => true,
            Some(current) => release.version.cmp_precedence(&current.version).is_gt(),
        };
        if update {
            targets.push(candidate);
        } else if candidate == channel {
            let current = current.expect("an empty channel is always updated");
            bail!(
                "{} {} is not newer than the published {} ({})",
                channel.as_str(),
                release.version,
                current.version,
                current.commit
            );
        }
    }
    Ok(targets)
}

/// Homebrew formulae to bump for the given index targets.
pub fn homebrew_formulae(targets: &[Channel]) -> Vec<&'static str> {
    targets
        .iter()
        .filter_map(|channel| match channel {
            Channel::Stable => Some("maa-cli"),
            Channel::Beta => Some("maa-cli-beta"),
            Channel::Nightly => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(version: &str, commit: &str) -> Identity {
        Identity {
            version: Version::parse(version).unwrap(),
            commit: commit.into(),
        }
    }

    #[test]
    fn dev_branches_declare_a_minor_line() {
        assert_eq!(dev_branch_version("dev-0.8"), Some(Version::new(0, 8, 0)));
        assert_eq!(dev_branch_version("dev-1.12"), Some(Version::new(1, 12, 0)));
        for name in [
            "dev-0.8.1",
            "dev-x.8",
            "dev-0.",
            "dev",
            "main",
            "dev-0.8-fix",
        ] {
            assert_eq!(dev_branch_version(name), None, "{name}");
        }
    }

    #[test]
    fn only_one_dev_branch_is_followed() -> Result<()> {
        assert_eq!(single_dev_branch("")?, None);
        assert_eq!(
            single_dev_branch("abc\trefs/heads/dev-0.8\n")?,
            Some(("dev-0.8".into(), "abc".into()))
        );
        assert!(single_dev_branch("a\trefs/heads/dev-0.8\nb\trefs/heads/dev-0.9").is_err());
        Ok(())
    }

    #[test]
    fn dev_branch_must_match_cargo_and_be_unreleased() -> Result<()> {
        let stable = Version::new(0, 7, 5);
        assert_eq!(
            dev_base("dev-0.8", &Version::new(0, 8, 0), Some(&stable))?,
            Version::new(0, 8, 0)
        );
        assert_eq!(
            dev_base("dev-0.8", &Version::new(0, 8, 0), None)?,
            Version::new(0, 8, 0)
        );
        assert!(dev_base("dev-0.8", &Version::new(0, 7, 5), Some(&stable)).is_err());
        assert!(dev_base("feature", &Version::new(0, 8, 0), Some(&stable)).is_err());
        let released = Version::new(0, 8, 0);
        assert!(dev_base("dev-0.8", &Version::new(0, 8, 0), Some(&released)).is_err());
        Ok(())
    }

    #[test]
    fn betas_are_numbered_from_existing_tags() -> Result<()> {
        let base = Version::new(0, 8, 0);
        assert_eq!(next_beta(&base, "")?, Version::parse("0.8.0-beta.1")?);
        let tags = "v0.7.5\nv0.8.0-beta.1\nv0.8.0-beta.10\nv0.8.0-beta.2\nv0.9.0-beta.30\nnightly";
        assert_eq!(next_beta(&base, tags)?, Version::parse("0.8.0-beta.11")?);
        assert_eq!(
            beta_at_commit(&base, "nightly\nv0.8.0-beta.2"),
            Some("v0.8.0-beta.2")
        );
        assert_eq!(beta_at_commit(&base, "v0.9.0-beta.1\nnightly"), None);
        Ok(())
    }

    #[test]
    fn nightly_sorts_after_beta_and_before_stable() -> Result<()> {
        let base = Version::new(0, 8, 0);
        let nightly = nightly(&base, 1234, "abc1234")?;
        assert_eq!(nightly.to_string(), "0.8.0-nightly.1234+sha.abc1234");
        assert!(
            nightly
                .cmp_precedence(&Version::parse("0.8.0-beta.99")?)
                .is_gt()
        );
        assert!(nightly.cmp_precedence(&base).is_lt());
        // Older nightly formats are superseded on the same core version.
        for old in ["0.8.0-alpha.9+sha.x", "0.8.0-beta.3.alpha.9+sha.x"] {
            assert!(nightly.cmp_precedence(&Version::parse(old)?).is_gt());
        }
        Ok(())
    }

    #[test]
    fn stable_skips_newer_prerelease_lines() -> Result<()> {
        let release = id("0.7.6", "s");
        let heads = |c: Channel| match c {
            Channel::Stable => Some(id("0.7.5", "a")),
            Channel::Beta => Some(id("0.8.0-beta.1", "b")),
            Channel::Nightly => Some(id("0.8.0-nightly.120+sha.c", "c")),
        };
        let targets = targets(Channel::Stable, &release, heads)?;
        assert_eq!(targets, [Channel::Stable]);
        assert_eq!(homebrew_formulae(&targets), ["maa-cli"]);
        Ok(())
    }

    #[test]
    fn stable_updates_every_channel_it_supersedes() -> Result<()> {
        let release = id("0.8.0", "s");
        let heads = |c: Channel| match c {
            Channel::Stable => Some(id("0.7.6", "a")),
            Channel::Beta => Some(id("0.8.0-beta.2", "b")),
            Channel::Nightly => None,
        };
        let targets = targets(Channel::Stable, &release, heads)?;
        assert_eq!(targets, Channel::ALL);
        assert_eq!(homebrew_formulae(&targets), ["maa-cli", "maa-cli-beta"]);
        Ok(())
    }

    #[test]
    fn prereleases_never_touch_more_stable_channels() -> Result<()> {
        let release = id("0.8.0-beta.2", "b2");
        let heads = |c: Channel| match c {
            Channel::Stable => Some(id("0.7.6", "a")),
            Channel::Beta => Some(id("0.8.0-beta.1", "b1")),
            Channel::Nightly => Some(id("0.8.0-nightly.125+sha.c", "c")),
        };
        let beta = targets(Channel::Beta, &release, heads)?;
        assert_eq!(beta, [Channel::Beta]);
        assert_eq!(homebrew_formulae(&beta), ["maa-cli-beta"]);

        let release = id("0.8.0-nightly.130+sha.d", "d");
        assert_eq!(targets(Channel::Nightly, &release, heads)?, [
            Channel::Nightly
        ]);
        Ok(())
    }

    #[test]
    fn replays_are_targets_but_regressions_fail() -> Result<()> {
        let release = id("0.8.0", "s");
        let replay = targets(Channel::Stable, &release, |c| {
            (c == Channel::Stable).then(|| release.clone())
        })?;
        assert_eq!(replay, Channel::ALL);

        for head in [id("0.8.0", "other"), id("0.8.1", "x")] {
            let head = head.clone();
            assert!(
                targets(Channel::Stable, &release, |c| {
                    (c == Channel::Stable).then(|| head.clone())
                })
                .is_err()
            );
        }
        Ok(())
    }
}
