use crate::{
    config::cli::{cli_config, resource::GitBackend},
    dirs,
};

use anyhow::Result;

pub fn update(is_auto: bool) -> Result<()> {
    let config = cli_config().resource_config();

    // Skip auto update if auto update is disabled
    if is_auto && !config.auto_update() {
        return Ok(());
    }

    let backend = config.backend();
    let url = config.remote().url();
    let branch = config.remote().branch();
    let ssh_key = config.remote().ssh_key().map(dirs::expand_tilde);
    let dest = dirs::hot_update();

    if dest.exists() {
        info!("updating-resource-repository");
        match backend {
            GitBackend::Git => git::pull(dest, branch, ssh_key.as_deref())?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::pull(dest, branch, ssh_key.as_deref())?,
        }
    } else {
        info!("cloning-resource-repository");
        match backend {
            GitBackend::Git => git::clone(url, branch, dest, ssh_key.as_deref())?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::clone(url, branch, dest, ssh_key.as_deref())?,
        }
    }

    Ok(())
}

mod git {
    use std::path::Path;

    use anyhow::{Context, Result};

    pub fn clone(
        url: &str,
        branch: Option<&str>,
        dest: &Path,
        ssh_key: Option<&Path>,
    ) -> Result<()> {
        let mut cmd = std::process::Command::new("git");

        cmd.args([
            "clone",
            url,
            dest.to_str().with_context(lfl!("invalid-utf8-path"))?,
            "--depth=1",
        ]);

        if let Some(branch) = branch {
            cmd.args(["--branch", branch]);
        }

        if let Some(ssh_key) = ssh_key {
            cmd.env(
                "GIT_SSH_COMMAND",
                format!(
                    "ssh -i {}",
                    ssh_key.to_str().with_context(lfl!("invalid-utf8-path"))?
                ),
            );
        }

        cmd.status()
            .with_context(lfl!("failed-clone-resource-repository"))?
            .success()
            .then_some(())
            .with_context(lfl!("failed-clone-resource-repository"))
    }

    pub fn pull(repo: &Path, branch: Option<&str>, ssh_key: Option<&Path>) -> Result<()> {
        let mut cmd = std::process::Command::new("git");

        cmd.args(["pull", "origin"]);

        if let Some(branch) = branch {
            cmd.arg(branch);
        }

        cmd.arg("--ff-only");

        if let Some(ssh_key) = ssh_key {
            cmd.env(
                "GIT_SSH_COMMAND",
                format!(
                    "ssh -i {}",
                    ssh_key.to_str().with_context(lfl!("invalid-utf8-path"))?
                ),
            );
        }

        cmd.current_dir(repo)
            .status()
            .with_context(lfl!("failed-pull-resource-repository"))?
            .success()
            .then_some(())
            .with_context(lfl!("failed-pull-resource-repository"))
    }
}

#[cfg(feature = "git2")]
mod git2 {
    use std::path::Path;

    use anyhow::{Context, Result};
    use git2::{build::RepoBuilder, Repository};

    pub fn clone(
        url: &str,
        branch: Option<&str>,
        dest: &Path,
        ssh_key: Option<&Path>,
    ) -> Result<()> {
        let mut builder = RepoBuilder::new();

        if let Some(branch) = branch {
            builder.branch(branch);
        }

        if let Some(ssh_key) = ssh_key {
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_, username_from_url, _| {
                git2::Cred::ssh_key(username_from_url.unwrap(), None, ssh_key, None)
            });

            let mut fetch_options = git2::FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            builder.fetch_options(fetch_options);
        }

        builder
            .clone(url, dest)
            .with_context(lfl!("failed-clone-resource-repository"))?;

        Ok(())
    }

    pub fn pull(repo: &Path, branch: Option<&str>, ssh_key: Option<&Path>) -> Result<()> {
        let repo = Repository::open(repo).with_context(lfl!("failed-open-resource-repository"))?;

        let branch = branch.unwrap_or("main");

        let mut fetch_options = ssh_key.map(|ssh_key| {
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_, username_from_url, _| {
                git2::Cred::ssh_key(username_from_url.unwrap(), None, ssh_key, None)
            });

            let mut fetch_options = git2::FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            fetch_options
        });

        repo.find_remote("origin")
            .with_context(lfl!("failed-find-remote", name = "origin"))?
            .fetch(&[branch], fetch_options.as_mut(), None)?;

        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .with_context(lfl!("failed-find-reference", name = "FETCH_HEAD"))?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .with_context(lfl!(
                "failed-reference-to-annotated-commit",
                name = "FETCH_HEAD"
            ))?;

        let (analysis, _) = repo
            .merge_analysis(&[&fetch_commit])
            .with_context(lfl!("failed-merge-analysis"))?;

        if analysis.is_fast_forward() {
            debug!("fast-forward-merge");

            let refname = format!("refs/heads/{}", branch);
            let mut reference = repo
                .find_reference(&refname)
                .with_context(lfl!("failed-find-reference", name = refname.as_str()))?;

            reference
                .set_target(fetch_commit.id(), "Fast-Forward")
                .with_context(lfl!("failed-create-reference", name = refname.as_str()))?;

            repo.set_head(&refname)
                .with_context(lfl!("failed-set-head"))?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .with_context(lfl!("failed-checkout", name = "HEAD"))?;
        } else if analysis.is_up_to_date() {
            debug!("repo-up-to-date");
        } else {
            bailfl!("failed-pull-resource-repository");
        }

        Ok(())
    }
}
