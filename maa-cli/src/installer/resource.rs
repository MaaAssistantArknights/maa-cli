use std::ffi::OsString;

use crate::{
    config::cli::{cli_config, resource::GitBackend},
    dirs,
};

use anyhow::{bail, Result};
use log::{debug, warn};

trait StatusExt {
    /// If error, return the error, otherwise return an error if the status is not successful
    fn check(self) -> std::io::Result<()>;
}

impl StatusExt for std::io::Result<std::process::ExitStatus> {
    fn check(self) -> std::io::Result<()> {
        self.and_then(|status| {
            if !status.success() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Command failed",
                ))
            } else {
                Ok(())
            }
        })
    }
}

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

    // check if git is available when using git backend
    let backend = match backend {
        GitBackend::Git
            if std::process::Command::new("git")
                .arg("--version")
                .stdout(std::process::Stdio::null()) // ignore normal output
                .status()
                .check()
                .is_err() =>
        {
            #[cfg(feature = "git2")]
            {
                warn!("Failed to execute git, falling back to libgit2 backend");
                GitBackend::Libgit2
            }

            #[cfg(not(feature = "git2"))]
            {
                bail!("Failed to execute git, please check your `git` installation");
            }
        }
        _ => backend,
    };

    // check if ssh key is available
    if url.starts_with("git@") && ssh_key.is_none() {
        bail!("SSH key is required for git repository with ssh url");
    }

    if dest.exists() {
        debug!("Fetching resource repository...");
        match backend {
            GitBackend::Git => git::pull(dest, branch, ssh_key.as_deref())?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::pull(dest, branch, ssh_key.as_deref())?,
        }
    } else {
        debug!("Cloning resource repository...");
        match backend {
            GitBackend::Git => git::clone(url, branch, dest, ssh_key.as_deref())?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::clone(url, branch, dest, ssh_key.as_deref())?,
        }
    }

    Ok(())
}

fn build_ssh_command(ssh_key: &std::path::Path) -> OsString {
    let mut cmd = OsString::from("ssh -i ");
    cmd.push(ssh_key);
    cmd
}

mod git {
    use super::{build_ssh_command, StatusExt};

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
            dest.to_str().context("Invalid path")?,
            "--depth=1",
        ]);

        if let Some(branch) = branch {
            cmd.args(["--branch", branch]);
        }

        if let Some(ssh_key) = ssh_key {
            cmd.env("GIT_SSH_COMMAND", build_ssh_command(ssh_key));
        }

        cmd.status()
            .check()
            .context("Failed to clone resource repository")?;

        Ok(())
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
                format!("ssh -i {}", ssh_key.to_str().context("Invalid path")?),
            );
        }

        cmd.current_dir(repo)
            .status()
            .check()
            .context("Failed to pull resource repository")?;

        Ok(())
    }
}

#[cfg(feature = "git2")]
mod git2 {
    use std::path::Path;

    use anyhow::{bail, Context, Result};
    use git2::{build::RepoBuilder, Repository};
    use log::debug;

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
            .context("Failed to clone resource repository")?;

        Ok(())
    }

    pub fn pull(repo: &Path, branch: Option<&str>, ssh_key: Option<&Path>) -> Result<()> {
        let repo = Repository::open(repo).context("Failed to open resource repository")?;

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
            .context("Failed to find remote 'origin'")?
            .fetch(&[branch], fetch_options.as_mut(), None)?;

        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .context("Failed to find reference 'FETCH_HEAD'")?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .context("Failed to find annotated commit")?;

        let (analysis, _) = repo
            .merge_analysis(&[&fetch_commit])
            .context("Failed to analyze merge")?;

        if analysis.is_fast_forward() {
            debug!("Fast-forwarding");

            let refname = format!("refs/heads/{}", branch);
            let mut reference = repo
                .find_reference(&refname)
                .context("Failed to find reference")?;

            reference
                .set_target(fetch_commit.id(), "Fast-Forward")
                .context("Failed to set target")?;

            repo.set_head(&refname).context("Failed to set HEAD")?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .context("Failed to checkout HEAD")?;
        } else if analysis.is_up_to_date() {
            debug!("Already up-to-date");
        } else {
            bail!("Failed to pull resource repository")
        }

        Ok(())
    }
}

mod gix {
    use super::build_ssh_command;

    use std::path::Path;

    use anyhow::{bail, Context, Result};
    use gix::{
        self,
        progress::Discard,
        reference::{self, set_target_id},
        remote,
        trace::warn,
    };

    pub fn clone(
        url: &str,
        branch: Option<&str>,
        dest: &Path,
        ssh_key: Option<&Path>,
    ) -> Result<()> {
        // gix use ssh command for ssh transport, so we need to set GIT_SSH_COMMAND like git
        let old_ssh_command = if let Some(ssh_key) = ssh_key {
            let old_ssh_command = std::env::var_os("GIT_SSH_COMMAND");
            std::env::set_var("GIT_SSH_COMMAND", build_ssh_command(ssh_key));
            old_ssh_command
        } else {
            None
        };

        let create_opts = gix::create::Options {
            destination_must_be_empty: true,
            ..Default::default()
        };

        // A copy of gix::open_opts_with_git_binary_config
        let open_opts = {
            use gix::sec::trust::DefaultForLevel;
            let mut opts = gix::open::Options::default_for_level(gix::sec::Trust::Full);
            opts.permissions.config.git_binary = true;
            opts
        };

        // In sync mode, the interrupt seems make no sense
        let interrupt = std::sync::atomic::AtomicBool::new(false);

        let mut prepare = gix::clone::PrepareFetch::new(
            url,
            dest,
            gix::create::Kind::WithWorktree,
            create_opts,
            open_opts,
        )
        .context("Failed to prepare fetch")?
        .with_shallow(remote::fetch::Shallow::DepthAtRemote(
            std::num::NonZeroU32::new(1).unwrap(),
        ));

        let (mut checkout, _) = prepare.fetch_then_checkout(Discard, &interrupt)?;

        checkout.main_worktree(Discard, &interrupt)?;

        if let Some(cmd) = old_ssh_command {
            std::env::set_var("GIT_SSH_COMMAND", cmd);
        }

        Ok(())
    }

    pub fn pull(repo: &Path, branch: Option<&str>, ssh_key: Option<&Path>) -> Result<()> {
        let old_ssh_command = if let Some(ssh_key) = ssh_key {
            let old_ssh_command = std::env::var_os("GIT_SSH_COMMAND");
            std::env::set_var("GIT_SSH_COMMAND", build_ssh_command(ssh_key));
            old_ssh_command
        } else {
            None
        };

        let repo = gix::open(repo).context("Failed to open resource repository")?;

        let branch = branch.unwrap_or("main");

        let remote = repo
            .find_fetch_remote(None)
            .context("Failed to find fetch remote")?;

        remote
            .connect(remote::Direction::Fetch)?
            .prepare_fetch(Discard, Default::default())?
            .with_shallow(remote::fetch::Shallow::DepthAtRemote(
                std::num::NonZeroU32::new(1).unwrap(),
            ))
            .receive(Discard, &std::sync::atomic::AtomicBool::new(false))?;

        let fetch_head = repo.find_reference("FETCH_HEAD")?;

        Ok(())
    }
}
