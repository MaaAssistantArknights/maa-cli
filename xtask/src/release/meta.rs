use std::{fs, num::NonZeroU16, path::Path, str::FromStr};

use anyhow::{Context, Result, bail, ensure};
use maa_version::{VersionManifest, cli::Details};
use semver::{BuildMetadata, Prerelease, Version};
use serde::Deserialize;
use xshell::{Shell, cmd};

use super::Channel;
use crate::github;

#[derive(Deserialize)]
struct CargoToml {
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    version: Version,
}

pub fn run() -> Result<()> {
    let sh = Shell::new()?;

    let cargo_pkg_version = get_cargo_version()?;
    let commit_sha = get_commit_sha(&sh)?;
    let commit_short_sha = get_commit_short_sha(&sh)?;

    let event_name = github::EventName::from_env()?;

    let (channel, publish) = determine_channel_and_publish(event_name)?;

    // Check if version directory exists
    ensure!(Path::new("version").exists(), "version directory not found");

    let version_file = channel.version_file();

    // Skip if no new commits
    let manifest = read_version_manifest(&version_file)?;
    if manifest.details.commit == commit_sha {
        println!("No new commits, skipping all steps");
        github::set_output("skip", "true")?;
        return Ok(());
    }

    let published_version = manifest.version;

    // For stable releases triggered by push tag, validate tag matches version
    if event_name == github::EventName::Push {
        let github_ref = github::github_ref();
        let ref_version = github_ref.strip_prefix("refs/tags/v").unwrap_or("");
        ensure!(
            ref_version == cargo_pkg_version.to_string(),
            "Version tag not matched: expected v{cargo_pkg_version}, got {github_ref}"
        );
    }

    let (version, tag) = compute_version(
        channel,
        &cargo_pkg_version,
        &published_version,
        &commit_short_sha,
    )?;

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
        ("skip", "false"),
    ])?;

    Ok(())
}

fn get_cargo_version() -> Result<Version> {
    let content =
        fs::read_to_string("crates/maa-cli/Cargo.toml").context("Failed to read Cargo.toml")?;

    let cargo_toml: CargoToml = toml::from_str(&content).context("Failed to parse Cargo.toml")?;

    Ok(cargo_toml.package.version)
}

fn get_commit_sha(sh: &Shell) -> Result<String> {
    Ok(cmd!(sh, "git rev-parse HEAD").read()?)
}

fn get_commit_short_sha(sh: &Shell) -> Result<String> {
    Ok(cmd!(sh, "git rev-parse --short HEAD").read()?)
}

fn determine_channel_and_publish(event_name: github::EventName) -> Result<(Channel, bool)> {
    match event_name {
        github::EventName::PullRequest => {
            println!("PR detected");
            Ok((Channel::Alpha, false))
        }
        github::EventName::Schedule => {
            println!("Scheduled event detected");
            Ok((Channel::Alpha, true))
        }
        github::EventName::WorkflowDispatch => {
            println!("Workflow dispatch event detected");
            let event = github::WorkflowEvent::from_env()?;
            let channel = event.inputs.channel;
            Ok((channel, event.inputs.publish))
        }
        github::EventName::Push => {
            println!("New tag detected");
            Ok((Channel::Stable, true))
        }
    }
}

fn read_version_manifest(file: &str) -> Result<VersionManifest<Details>> {
    let content = fs::read_to_string(file).with_context(|| format!("Failed to read {}", file))?;

    serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", file))
}

fn compute_version(
    channel: Channel,
    cargo_version: &Version,
    published_version: &Version,
    commit_short_sha: &str,
) -> Result<(Version, String)> {
    match channel {
        Channel::Stable => {
            let tag = format!("v{}", cargo_version);
            Ok((cargo_version.clone(), tag))
        }
        Channel::Beta => {
            check_version_bumped(cargo_version, published_version)?;

            let mut version = cargo_version.clone();
            version.build = BuildMetadata::EMPTY;

            if is_same_core_version(cargo_version, published_version) {
                version.pre = PrereleaseVersion::from(&published_version.pre)
                    .bump_beta()
                    .into();
            } else {
                version.pre = Prerelease::new("beta.1")?;
            }

            let tag = format!("v{}", version);
            Ok((version, tag))
        }
        Channel::Alpha => {
            check_version_bumped(cargo_version, published_version)?;

            let mut version = cargo_version.clone();
            version.build = BuildMetadata::new(&format!("sha.{}", commit_short_sha))?;

            if is_same_core_version(cargo_version, published_version) {
                version.pre = PrereleaseVersion::from(&published_version.pre)
                    .bump_alpha()
                    .into();
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

impl FromStr for PrereleaseVersion {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::default());
        }

        if let Some(rest) = s.strip_prefix("beta.") {
            let parts: Vec<&str> = rest.split('.').collect();

            if parts.len() == 1 {
                // beta.N
                let beta = parts[0].parse().ok().and_then(NonZeroU16::new);
                Ok(PrereleaseVersion { beta, alpha: None })
            } else if parts.len() >= 3 && parts[1] == "alpha" {
                // beta.N.alpha.M
                let beta = parts[0].parse().ok().and_then(NonZeroU16::new);
                let alpha = parts[2].parse().ok().and_then(NonZeroU16::new);
                Ok(PrereleaseVersion { beta, alpha })
            } else {
                Ok(Self::default())
            }
        } else if let Some(rest) = s.strip_prefix("alpha.") {
            // alpha.N
            let alpha = rest.parse().ok().and_then(NonZeroU16::new);
            Ok(PrereleaseVersion { beta: None, alpha })
        } else {
            Ok(Self::default())
        }
    }
}

impl From<&Prerelease> for PrereleaseVersion {
    fn from(prerelease: &Prerelease) -> Self {
        prerelease.as_str().parse().unwrap()
    }
}

impl From<PrereleaseVersion> for Prerelease {
    fn from(version: PrereleaseVersion) -> Self {
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
    fn bump_beta(self) -> Self {
        let beta_num = self.beta.map(|n| n.get()).unwrap_or(0) + 1;
        PrereleaseVersion {
            beta: NonZeroU16::new(beta_num),
            alpha: None,
        }
    }

    fn bump_alpha(self) -> Self {
        match (self.beta, self.alpha) {
            (None, alpha) => {
                // None or alpha.N -> alpha.N+1
                let alpha_num = alpha.map(|n| n.get()).unwrap_or(0) + 1;
                PrereleaseVersion {
                    beta: None,
                    alpha: NonZeroU16::new(alpha_num),
                }
            }
            (Some(beta), alpha) => {
                // beta.N or beta.N.alpha.M -> beta.N.alpha.M+1
                let alpha_num = alpha.map(|n| n.get()).unwrap_or(0) + 1;
                PrereleaseVersion {
                    beta: Some(beta),
                    alpha: NonZeroU16::new(alpha_num),
                }
            }
        }
    }
}

fn is_same_core_version(v1: &Version, v2: &Version) -> bool {
    v1.major == v2.major && v1.minor == v2.minor && v1.patch == v2.patch
}

fn check_version_bumped(cargo_pkg_version: &Version, published_version: &Version) -> Result<()> {
    if cargo_pkg_version == published_version {
        println!("The version in Cargo.toml is the same as the published version");
        println!("No pre-release is allowed for the same version");
        github::set_output("skip", "true")?;
        bail!("Version not bumped");
    }
    Ok(())
}
