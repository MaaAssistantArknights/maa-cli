//! GitHub state around a release: frozen run artifacts and the release tag.

use std::{path::Path, process::Command, thread::sleep, time::Duration};

use anyhow::{Context, Result, bail, ensure};

use super::{git, plan::Plan};
use crate::{cmd::CommandExt, env, github};

const PLAN_ARTIFACT: &str = "release-plan";
const BUNDLE_ARTIFACT: &str = "release-bundle";

/// Report whether this run already froze its plan and bundle.
///
/// Queried on every job attempt, including retries that did not rerun the plan job,
/// so a retry reuses what was frozen instead of recalculating or rebuilding it.
pub fn artifacts() -> Result<()> {
    let run_id = env::var("GITHUB_RUN_ID")?;
    let listing = Command::new("gh")
        .args(["api", "--paginate"])
        .arg(format!(
            "repos/{{owner}}/{{repo}}/actions/runs/{run_id}/artifacts"
        ))
        .args([
            "--jq",
            ".artifacts[] | [.name, (.expired | tostring)] | @tsv",
        ])
        .read()?;
    let (plan, bundle) = frozen_artifacts(&listing)?;
    github::set_outputs(&[("plan", &plan.to_string()), ("bundle", &bundle.to_string())])
}

fn frozen_artifacts(listing: &str) -> Result<(bool, bool)> {
    let mut found = (false, false);
    for line in listing.lines() {
        let (name, expired) = line
            .split_once('\t')
            .with_context(|| format!("Unexpected artifact listing: {line}"))?;
        let slot = match name {
            PLAN_ARTIFACT => &mut found.0,
            BUNDLE_ARTIFACT => &mut found.1,
            _ => continue,
        };
        ensure!(
            expired != "true",
            "The {name} artifact of this run expired; start a new release run instead of retrying"
        );
        *slot = true;
    }
    Ok(found)
}

/// Make sure the release tag is free or already points at the release commit.
///
/// Version tags are never moved. The movable `nightly` tag and its release are
/// deleted so the release step can recreate them at the new commit.
pub fn tag(plan_dir: &Path) -> Result<()> {
    let plan = Plan::load(plan_dir)?;
    let tag = plan.tag.as_str();
    let refs = git(Path::new("."))
        .args(["ls-remote", "--tags", "origin"])
        .arg(format!("refs/tags/{tag}"))
        .arg(format!("refs/tags/{tag}^{{}}"))
        .read()?;
    let Some(existing) = tagged_commit(&refs) else {
        return Ok(());
    };
    if existing == plan.commit {
        return Ok(());
    }
    ensure!(
        tag == "nightly",
        "Tag {tag} targets {existing} instead of {}",
        plan.commit
    );
    delete_nightly()?;
    wait_until_deleted(tag)
}

/// The commit a tag points to, peeling annotated tags.
fn tagged_commit(ls_remote: &str) -> Option<String> {
    let mut direct = None;
    for line in ls_remote.lines() {
        let (commit, reference) = line.split_once('\t')?;
        if reference.ends_with("^{}") {
            return Some(commit.to_owned());
        }
        direct = Some(commit.to_owned());
    }
    direct
}

fn delete_nightly() -> Result<()> {
    let release = Command::new("gh")
        .args([
            "api",
            "repos/{owner}/{repo}/releases/tags/nightly",
            "--silent",
        ])
        .output()
        .context("Failed to run gh")?;
    if release.status.success() {
        Command::new("gh")
            .args(["release", "delete", "nightly", "--yes", "--cleanup-tag"])
            .run()
    } else if String::from_utf8_lossy(&release.stderr).contains("HTTP 404") {
        // The tag can outlive a failed release creation or a manually deleted release.
        Command::new("gh")
            .args([
                "api",
                "--method",
                "DELETE",
                "repos/{owner}/{repo}/git/refs/tags/nightly",
            ])
            .run()
    } else {
        bail!(
            "Failed to inspect the nightly release: {}",
            String::from_utf8_lossy(&release.stderr)
        )
    }
}

/// Recreating a release while GitHub still sees the old tag can leave it as a draft.
fn wait_until_deleted(tag: &str) -> Result<()> {
    for _ in 0..20 {
        let status = git(Path::new("."))
            .args(["ls-remote", "--exit-code", "--tags", "origin"])
            .arg(format!("refs/tags/{tag}"))
            .status()
            .context("Failed to run git ls-remote")?;
        match status.code() {
            Some(2) => {
                sleep(Duration::from_secs(5));
                return Ok(());
            }
            Some(0) => sleep(Duration::from_secs(3)),
            _ => bail!("git ls-remote failed while waiting for {tag} to be deleted"),
        }
    }
    bail!("Tag {tag} is still visible after deleting it")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_resolve_to_their_peeled_commit() {
        assert_eq!(tagged_commit(""), None);
        assert_eq!(
            tagged_commit("abc\trefs/tags/v0.8.0"),
            Some("abc".to_owned())
        );
        assert_eq!(
            tagged_commit("tagobj\trefs/tags/v0.8.0\ncommit\trefs/tags/v0.8.0^{}"),
            Some("commit".to_owned())
        );
    }

    #[test]
    fn frozen_artifacts_are_reused_unless_expired() -> Result<()> {
        assert_eq!(frozen_artifacts("")?, (false, false));
        assert_eq!(
            frozen_artifacts("release-plan\tfalse\nmaa_cli-x86_64\ttrue")?,
            (true, false)
        );
        assert_eq!(
            frozen_artifacts("release-plan\tfalse\nrelease-bundle\tfalse")?,
            (true, true)
        );
        assert!(frozen_artifacts("release-bundle\ttrue").is_err());
        assert!(frozen_artifacts("garbage").is_err());
        Ok(())
    }
}
