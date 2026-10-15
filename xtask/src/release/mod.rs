use std::{
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

pub mod archive;
mod changelog;
mod package;
mod plan;
mod publish;
mod selection;
mod version;

/// Release channel for maa-cli, ordered from the most to the least stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Stable release from main
    Stable,
    /// Beta pre-release from the dev branch
    Beta,
    /// Nightly snapshot of the dev branch
    Nightly,
}

impl FromStr for Channel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "stable" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            "nightly" => Ok(Channel::Nightly),
            _ => bail!("Unknown channel: {s}"),
        }
    }
}

impl Channel {
    pub const ALL: [Channel; 3] = [Channel::Stable, Channel::Beta, Channel::Nightly];

    /// Get the channel name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Nightly => "nightly",
        }
    }

    /// Index files of this channel, relative to the version branch checkout.
    ///
    /// The client and install scripts read `alpha.json`, so it mirrors nightly. The first
    /// file is the one release planning reads.
    pub fn index_files(&self) -> &'static [&'static str] {
        match self {
            Channel::Stable => &["stable.json"],
            Channel::Beta => &["beta.json"],
            Channel::Nightly => &["nightly.json", "alpha.json"],
        }
    }
}

#[derive(Subcommand)]
pub enum ReleaseCommands {
    /// Find the commit a release is built from
    Source(SourceOptions),
    /// Resolve the release plan, its index targets and release notes
    Plan(PlanOptions),
    /// Prepare the version and changelog for a stable release
    Prepare(PrepareOptions),
    /// Package archives and index manifests into release-bundle
    Package(PlanFile),
    /// Make sure the release tag is free or already points at the release commit
    Tag(PlanFile),
    /// Apply the packaged manifests to the version branch checkout
    Index(IndexOptions),
}

#[derive(Args)]
pub struct SourceOptions {
    /// Channel to release
    #[arg(long)]
    channel: Channel,
    /// Merged commit of the release PR (stable only)
    #[arg(long)]
    commit: Option<String>,
}

#[derive(Args)]
pub struct PlanOptions {
    /// Channel to release
    #[arg(long)]
    channel: Channel,
    /// Commit selected by `release source`
    #[arg(long)]
    commit: String,
    /// Development branch selected by `release source` (beta and nightly)
    #[arg(long)]
    branch: Option<String>,
    /// Checkout of the release commit
    #[arg(long, default_value = ".")]
    source: PathBuf,
    /// Checkout of the version branch
    #[arg(long, default_value = "version")]
    index: PathBuf,
    /// Directory to write the plan, its outputs and the release notes into
    #[arg(long)]
    out: PathBuf,
    /// Publish an optimized release instead of building a preview
    #[arg(long)]
    publish: bool,
}

#[derive(Args)]
pub struct PrepareOptions {
    /// Checkout of main to prepare
    #[arg(long, default_value = ".")]
    source: PathBuf,
    /// Version bump to release
    #[arg(long, value_enum, default_value_t = selection::Bump::Auto)]
    bump: selection::Bump,
}

#[derive(Args)]
pub struct PlanFile {
    /// Directory of the release plan
    #[arg(long, default_value = "release-plan")]
    plan: PathBuf,
}

#[derive(Args)]
pub struct IndexOptions {
    #[command(flatten)]
    plan: PlanFile,
    /// Checkout of the version branch
    #[arg(long, default_value = "version")]
    index: PathBuf,
}

pub fn run(command: ReleaseCommands) -> Result<()> {
    match command {
        ReleaseCommands::Source(options) => plan::source(options),
        ReleaseCommands::Plan(options) => plan::run(options),
        ReleaseCommands::Prepare(options) => changelog::prepare(options),
        ReleaseCommands::Package(options) => package::run(&options.plan),
        ReleaseCommands::Tag(options) => publish::tag(&options.plan),
        ReleaseCommands::Index(options) => package::update_index(
            &options.plan.plan,
            Path::new(package::BUNDLE),
            &options.index,
        ),
    }
}

/// The changelog configuration of the release tooling, applied to any source checkout.
fn cliff_config() -> PathBuf {
    Path::new(crate::workspace_root()).join("cliff.toml")
}

/// A git command running inside `dir`.
fn git(dir: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(dir);
    command
}

/// Throwaway git repositories shaped like this workspace, for release tests.
#[cfg(test)]
mod test_repo {
    use std::{fs, path::Path};

    use crate::cmd::CommandExt;

    pub struct Repo {
        dir: tempfile::TempDir,
    }

    impl Repo {
        /// A repository on `main` whose maa-cli package is released as `v{version}`.
        pub fn released(version: &str) -> Self {
            let repo = Self {
                dir: tempfile::tempdir().unwrap(),
            };
            repo.git(&["init", "-q", "-b", "main"]);
            repo.git(&["config", "user.name", "Release Test"]);
            repo.git(&["config", "user.email", "release@example.invalid"]);
            repo.git(&["config", "commit.gpgsign", "false"]);
            repo.git(&["config", "tag.gpgsign", "false"]);
            let root = repo.path();
            fs::create_dir_all(root.join("crates/maa-cli/src")).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[workspace]\nmembers = [\"crates/maa-cli\"]\nresolver = \"2\"\n",
            )
            .unwrap();
            repo.write_version(version);
            fs::write(
                root.join("crates/maa-cli/src/lib.rs"),
                "pub fn example() {}\n",
            )
            .unwrap();
            fs::write(
                root.join("CHANGELOG.md"),
                format!("# Release Notes\n\n## Release {version}\n\nPrevious release\n"),
            )
            .unwrap();
            std::process::Command::new("cargo")
                .current_dir(root)
                .args(["generate-lockfile", "--offline"])
                .run()
                .unwrap();
            repo.git(&["add", "."]);
            repo.git(&["commit", "-qm", "chore: initial"]);
            repo.git(&["tag", &format!("v{version}")]);
            repo
        }

        pub fn path(&self) -> &Path {
            self.dir.path()
        }

        pub fn git(&self, args: &[&str]) -> String {
            super::git(self.path()).args(args).read().unwrap()
        }

        pub fn commit(&self, message: &str) -> String {
            self.git(&["commit", "-q", "--allow-empty", "-m", message]);
            self.git(&["rev-parse", "HEAD"])
        }

        pub fn write_version(&self, version: &str) {
            fs::write(
                self.path().join("crates/maa-cli/Cargo.toml"),
                format!(
                    "[package]\nname = \"maa-cli\"\nversion = \"{version}\"\nedition = \"2024\"\n"
                ),
            )
            .unwrap();
        }
    }
}
