//! Resolve what a release run publishes, before anything is built.

use std::{fs, path::Path};

use anyhow::{Context, Result, ensure};
use maa_version::{VersionManifest, cli::Details};
use semver::Version;
use serde::{Deserialize, Serialize};

use super::{
    Channel, PlanOptions, SourceOptions, changelog, git,
    selection::{cargo_version, ensure_stable},
    version::{self, Identity, Target},
};
use crate::{cmd::CommandExt, github};

/// The frozen description of one release, shared by every job of a run and its retries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub channel: Channel,
    pub version: Version,
    pub tag: String,
    pub commit: String,
    pub publish: bool,
    pub targets: Vec<Target>,
}

const PLAN_FILE: &str = "plan.json";
const OUTPUTS_FILE: &str = "outputs";
const NOTES_FILE: &str = "notes.md";

impl Plan {
    pub fn load(dir: &Path) -> Result<Self> {
        let file = dir.join(PLAN_FILE);
        let content =
            fs::read(&file).with_context(|| format!("Failed to read {}", file.display()))?;
        serde_json::from_slice(&content)
            .with_context(|| format!("Failed to parse {}", file.display()))
    }

    pub fn identity(&self) -> Identity {
        Identity {
            version: self.version.clone(),
            commit: self.commit.clone(),
        }
    }

    fn outputs(&self) -> Vec<(&'static str, String)> {
        let targets: Vec<&str> = self.targets.iter().map(|t| t.channel.as_str()).collect();
        vec![
            ("skip", "false".into()),
            ("channel", self.channel.as_str().into()),
            ("version", self.version.to_string()),
            ("tag", self.tag.clone()),
            ("commit", self.commit.clone()),
            ("publish", self.publish.to_string()),
            (
                "profile",
                if self.publish { "release-lto" } else { "dev" }.into(),
            ),
            ("targets", targets.join(" ")),
            (
                "homebrew",
                version::homebrew_formulae(&self.targets).join(" "),
            ),
        ]
    }
}

/// Select the commit to build: the merged release PR for stable, the dev branch head otherwise.
pub fn source(options: SourceOptions) -> Result<()> {
    if options.channel == Channel::Stable {
        let commit = options
            .commit
            .context("A stable release needs the merged commit of its release PR")?;
        return github::set_outputs(&[("skip", "false"), ("commit", &commit), ("branch", "main")]);
    }
    ensure!(
        options.commit.is_none(),
        "Pre-releases always build the head of the dev branch"
    );
    let heads = git(Path::new("."))
        .args(["ls-remote", "--heads", "origin", "refs/heads/dev-*"])
        .read()?;
    match version::single_dev_branch(&heads)? {
        Some((branch, commit)) => {
            github::set_outputs(&[("skip", "false"), ("commit", &commit), ("branch", &branch)])
        }
        None if options.channel == Channel::Nightly => {
            println!("No dev branch exists; nightly only follows a dev-X.Y branch.");
            github::set_output("skip", "true")
        }
        None => anyhow::bail!("Beta releases need a dev-X.Y branch"),
    }
}

pub fn run(options: PlanOptions) -> Result<()> {
    fs::create_dir_all(&options.out)?;
    let outputs = options.out.join(OUTPUTS_FILE);
    let head = git(&options.source).args(["rev-parse", "HEAD"]).read()?;
    ensure!(
        head == options.commit,
        "Checked-out commit {head} does not match the selected commit {}",
        options.commit
    );
    let index = Index::read(&options.index)?;

    let Some((version, tag)) = resolve(&options, &index)? else {
        return write_outputs(&outputs, &[("skip", "true".into())]);
    };
    let release = Identity {
        version: version.clone(),
        commit: options.commit.clone(),
    };
    let targets = version::targets(options.channel, &release, |c| index.head(c).cloned())?;
    let plan = Plan {
        channel: options.channel,
        version,
        tag,
        commit: options.commit,
        publish: options.publish,
        targets,
    };

    let notes = match plan.channel {
        Channel::Stable => {
            // Check the actual merge once, before freezing anything or creating a tag.
            changelog::check(&options.source, &plan.version)?;
            changelog::stable_notes(&options.source, &plan.version)?
        }
        _ => changelog::prerelease_notes(&options.source, &plan.version, &plan.tag)?,
    };
    println!(
        "Release {} ({}) to {} from {}",
        plan.version,
        plan.tag,
        plan.targets
            .iter()
            .map(|t| t.channel.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        plan.commit
    );
    fs::write(options.out.join(NOTES_FILE), notes)?;
    fs::write(
        options.out.join(PLAN_FILE),
        serde_json::to_vec_pretty(&plan)?,
    )?;
    write_outputs(&outputs, &plan.outputs())
}

/// The version and tag to release, or `None` when there is nothing to release.
fn resolve(options: &PlanOptions, index: &Index) -> Result<Option<(Version, String)>> {
    let source = options.source.as_path();
    let cargo = cargo_version(source)?;
    let stable = index.head(Channel::Stable).map(|id| &id.version);
    match options.channel {
        Channel::Stable => {
            ensure_stable(&cargo)?;
            Ok(Some((cargo.clone(), format!("v{cargo}"))))
        }
        Channel::Beta => {
            let branch = options.branch.as_deref().context("Beta needs --branch")?;
            let base = version::dev_base(branch, &cargo, stable)?;
            let at_commit = git(source).args(["tag", "--points-at", "HEAD"]).read()?;
            if let Some(tag) = version::beta_at_commit(&base, &at_commit) {
                println!("{tag} already releases this commit");
                return Ok(None);
            }
            let tags = git(source)
                .args(["tag", "--list", &format!("v{base}-beta.*")])
                .read()?;
            let version = version::next_beta(&base, &tags)?;
            let tag = format!("v{version}");
            Ok(Some((version, tag)))
        }
        Channel::Nightly => {
            let branch = options
                .branch
                .as_deref()
                .context("Nightly needs --branch")?;
            let base = match version::dev_base(branch, &cargo, stable) {
                Ok(base) => base,
                Err(error) => {
                    // A scheduled run must not fail every night for a branch awaiting cleanup.
                    println!("Skipping nightly: {error:#}");
                    return Ok(None);
                }
            };
            let nightly = git(source)
                .args(["rev-parse", "-q", "--verify", "refs/tags/nightly^{commit}"])
                .read()
                .unwrap_or_default();
            if nightly == options.commit {
                println!("Nightly already builds this commit");
                return Ok(None);
            }
            let count = git(source)
                .args(["rev-list", "--count", "HEAD"])
                .read()?
                .parse()
                .context("Failed to count commits")?;
            let short = git(source).args(["rev-parse", "--short", "HEAD"]).read()?;
            Ok(Some((
                version::nightly(&base, count, &short)?,
                "nightly".into(),
            )))
        }
    }
}

fn write_outputs(file: &Path, outputs: &[(&str, String)]) -> Result<()> {
    let content: String = outputs
        .iter()
        .map(|(key, value)| format!("{key}={value}\n"))
        .collect();
    fs::write(file, content).with_context(|| format!("Failed to write {}", file.display()))
}

/// The published version index: the current entry of every channel.
pub struct Index {
    heads: Vec<(Channel, Option<Identity>)>,
}

impl Index {
    pub fn read(root: &Path) -> Result<Self> {
        let heads = Channel::ALL
            .into_iter()
            .map(|channel| Ok((channel, read_head(root, channel)?)))
            .collect::<Result<_>>()?;
        Ok(Self { heads })
    }

    pub fn head(&self, channel: Channel) -> Option<&Identity> {
        self.heads
            .iter()
            .find(|(c, _)| *c == channel)
            .and_then(|(_, head)| head.as_ref())
    }
}

/// The entry of a channel, from its primary index file.
///
/// Mirrors such as `alpha.json` are written but never read.
fn read_head(root: &Path, channel: Channel) -> Result<Option<Identity>> {
    let file = root.join(channel.index_files()[0]);
    Ok(read_manifest(&file)?.map(|manifest| Identity {
        version: manifest.version,
        commit: manifest.details.commit,
    }))
}

pub fn read_manifest(file: &Path) -> Result<Option<VersionManifest<Details>>> {
    match fs::read(file) {
        Ok(content) => serde_json::from_slice(&content)
            .map(Some)
            .with_context(|| format!("Failed to parse {}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Failed to read {}", file.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str, commit: &str) -> String {
        serde_json::json!({
            "version": version,
            "details": {"tag": "t", "commit": commit, "assets": {}}
        })
        .to_string()
    }

    #[test]
    fn heads_are_read_from_primary_index_files() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("stable.json"), manifest("0.7.5", "s"))?;
        fs::write(dir.path().join("alpha.json"), manifest("0.7.5", "s"))?;
        let index = Index::read(dir.path())?;
        assert_eq!(index.head(Channel::Stable).unwrap().commit, "s");
        assert_eq!(index.head(Channel::Beta), None);
        // The alpha mirror is not a nightly entry.
        assert_eq!(index.head(Channel::Nightly), None);

        fs::write(
            dir.path().join("nightly.json"),
            manifest("0.8.0-nightly.5+sha.n", "n"),
        )?;
        assert_eq!(
            Index::read(dir.path())?
                .head(Channel::Nightly)
                .unwrap()
                .commit,
            "n"
        );
        Ok(())
    }

    #[test]
    fn plan_outputs_drive_the_workflow() -> Result<()> {
        let plan = Plan {
            channel: Channel::Stable,
            version: Version::new(0, 8, 0),
            tag: "v0.8.0".into(),
            commit: "abc".into(),
            publish: true,
            targets: vec![
                Target {
                    channel: Channel::Stable,
                    expected: None,
                },
                Target {
                    channel: Channel::Beta,
                    expected: None,
                },
            ],
        };
        let outputs = plan.outputs();
        let get = |key: &str| outputs.iter().find(|(k, _)| *k == key).unwrap().1.clone();
        assert_eq!(get("targets"), "stable beta");
        assert_eq!(get("homebrew"), "maa-cli maa-cli-beta");
        assert_eq!(get("profile"), "release-lto");
        assert_eq!(get("skip"), "false");

        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join(PLAN_FILE), serde_json::to_vec(&plan)?)?;
        assert_eq!(Plan::load(dir.path())?, plan);
        Ok(())
    }

    use crate::release::test_repo::Repo;

    fn plan_in(repo: &Repo, index: &Path, channel: Channel, branch: &str) -> Result<Option<Plan>> {
        let out = tempfile::tempdir()?;
        run(PlanOptions {
            channel,
            commit: repo.git(&["rev-parse", "HEAD"]),
            branch: Some(branch.into()),
            source: repo.path().to_owned(),
            index: index.to_owned(),
            out: out.path().to_owned(),
            publish: true,
        })?;
        let outputs = fs::read_to_string(out.path().join(OUTPUTS_FILE))?;
        if outputs.contains("skip=true") {
            return Ok(None);
        }
        Plan::load(out.path()).map(Some)
    }

    #[test]
    fn dev_branch_prereleases() -> Result<()> {
        let repo = Repo::released("0.7.5");
        let index = tempfile::tempdir()?;
        fs::write(index.path().join("stable.json"), manifest("0.7.5", "s"))?;
        repo.git(&["checkout", "-qb", "dev-0.8"]);
        repo.commit("feat: config v2");

        // The branch must declare its version in Cargo first.
        assert!(plan_in(&repo, index.path(), Channel::Beta, "dev-0.8").is_err());
        assert!(plan_in(&repo, index.path(), Channel::Nightly, "dev-0.8")?.is_none());

        repo.write_version("0.8.0");
        repo.git(&["commit", "-qam", "chore: bump maa-cli to 0.8.0"]);
        let beta = plan_in(&repo, index.path(), Channel::Beta, "dev-0.8")?.unwrap();
        assert_eq!(beta.version.to_string(), "0.8.0-beta.1");
        assert_eq!(beta.tag, "v0.8.0-beta.1");
        // No nightly was published yet, so the beta also fills that channel.
        let channels: Vec<Channel> = beta.targets.iter().map(|t| t.channel).collect();
        assert_eq!(channels, [Channel::Beta, Channel::Nightly]);

        repo.git(&["tag", "v0.8.0-beta.1"]);
        assert!(plan_in(&repo, index.path(), Channel::Beta, "dev-0.8")?.is_none());
        repo.commit("fix: config v2 migration");
        let beta = plan_in(&repo, index.path(), Channel::Beta, "dev-0.8")?.unwrap();
        assert_eq!(beta.version.to_string(), "0.8.0-beta.2");

        let nightly = plan_in(&repo, index.path(), Channel::Nightly, "dev-0.8")?.unwrap();
        let count = repo.git(&["rev-list", "--count", "HEAD"]);
        let short = repo.git(&["rev-parse", "--short", "HEAD"]);
        assert_eq!(
            nightly.version.to_string(),
            format!("0.8.0-nightly.{count}+sha.{short}")
        );
        assert_eq!(nightly.tag, "nightly");
        repo.git(&["tag", "nightly"]);
        assert!(plan_in(&repo, index.path(), Channel::Nightly, "dev-0.8")?.is_none());

        // Once 0.8.0 is stable, the branch is done.
        fs::write(index.path().join("stable.json"), manifest("0.8.0", "m"))?;
        repo.commit("docs: after release");
        assert!(plan_in(&repo, index.path(), Channel::Beta, "dev-0.8").is_err());
        assert!(plan_in(&repo, index.path(), Channel::Nightly, "dev-0.8")?.is_none());
        Ok(())
    }
}
