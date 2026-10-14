//! The release tag on GitHub.

use std::{path::Path, process::Command, thread::sleep, time::Duration};

use anyhow::{Context, Result, bail, ensure};

use super::{git, plan::Plan};
use crate::cmd::CommandExt;

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
}
