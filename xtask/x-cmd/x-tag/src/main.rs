//! Push the next Stable or Beta version tag for the latest remote release branch.

use std::{
    io::{self, Write},
    process::Command,
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use flags::Tag as TagOptions;

const REMOTE: &str = "origin";

fn main() -> Result<()> {
    x_lib::group::finish_prepare();
    let options = TagOptions::from_env_or_exit();

    git(&["fetch", "--prune", REMOTE])?;
    let branch = match options.channel {
        Channel::Stable => format!("{REMOTE}/main"),
        Channel::Beta => dev_branch()?,
    };
    let version = cargo_version(&branch)?;
    let tag = match options.channel {
        Channel::Stable => format!("v{version}"),
        Channel::Beta => next_beta(&version)?,
    };

    let subject = git(&["log", "-1", "--format=%h %s", &branch])?;
    println!("Push {tag} on {branch}: {subject}");
    if !options.yes && !confirm()? {
        bail!("Aborted");
    }
    git(&["push", REMOTE, &format!("{branch}:refs/tags/{tag}")])?;
    Ok(())
}

#[derive(Debug)]
enum Channel {
    Stable,
    Beta,
}

impl FromStr for Channel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            _ => Err(format!("expected stable or beta, got {s}")),
        }
    }
}

fn dev_branch() -> Result<String> {
    let refs = git(&[
        "branch",
        "-r",
        "--format=%(refname:short)",
        "--list",
        "origin/dev-*",
    ])?;
    let branches: Vec<_> = refs.lines().collect();
    match branches.as_slice() {
        [branch] => Ok((*branch).to_owned()),
        [] => bail!("No dev branch on {REMOTE}; Beta is released from dev-*"),
        _ => bail!(
            "Only one dev branch may exist, found: {}",
            branches.join(" ")
        ),
    }
}

fn cargo_version(rev: &str) -> Result<String> {
    let manifest = git(&["show", &format!("{rev}:crates/maa-cli/Cargo.toml")])?;
    let manifest: toml::Table = manifest.parse().context("Parse maa-cli Cargo.toml")?;
    manifest
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(|version| version.as_str())
        .map(str::to_owned)
        .context("maa-cli Cargo.toml has no package version")
}

fn next_beta(version: &str) -> Result<String> {
    let prefix = format!("v{version}-beta.");
    let last = git(&["tag", "--list", &format!("{prefix}*")])?
        .lines()
        .filter_map(|tag| tag.strip_prefix(&prefix)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    Ok(format!("{prefix}{}", last + 1))
}

fn confirm() -> Result<bool> {
    print!("Continue? [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to run git")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

mod flags {
    use super::Channel;

    xflags::xflags! {
        /// Push the next version tag for the latest remote release branch.
        cmd tag {
            /// Release channel: stable (main) or beta (the dev-* branch).
            required channel: Channel
            /// Push without asking for confirmation.
            optional -y, --yes
        }
    }
}
