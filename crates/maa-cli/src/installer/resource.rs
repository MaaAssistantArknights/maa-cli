use anyhow::{Result, bail};
use log::{debug, warn};

use crate::{
    config::cli::{
        CLI_CONFIG,
        resource::{Certificate, GitBackend},
    },
    dirs,
};

trait StatusExt {
    /// If error, return the error, otherwise return an error if the status is not successful
    fn check(self) -> std::io::Result<()>;
}

impl StatusExt for std::io::Result<std::process::ExitStatus> {
    fn check(self) -> std::io::Result<()> {
        self.and_then(|status| {
            if !status.success() {
                Err(std::io::Error::other("Command failed"))
            } else {
                Ok(())
            }
        })
    }
}

pub fn update(is_auto: bool) -> Result<()> {
    let config = CLI_CONFIG.resource_config();

    // Skip auto update if auto update is disabled
    if is_auto && !config.auto_update() {
        return Ok(());
    }

    let backend = config.backend();
    let url = config.remote().url();
    let branch = config.remote().branch();
    let cert = config.remote().certificate();
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
    if url.starts_with("git@") && cert.is_none() {
        bail!("A Certificate is required to clone a repository using SSH");
    }

    let result = update_core(backend, url, dest, branch, cert);

    if config.warn_on_update_failure() {
        if let Err(err) = result {
            warn!("Failed to update resource repository: {err}");
        }
    } else {
        result?
    }

    Ok(())
}

fn update_core(
    backend: GitBackend,
    url: &str,
    dest: &std::path::Path,
    branch: Option<&str>,
    cert: Option<&Certificate>,
) -> Result<()> {
    if dest.exists() {
        debug!("Fetching resource repository...");
        match backend {
            GitBackend::Git => git::pull(dest, branch, cert)?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::pull(dest, branch, cert)?,
        }
    } else {
        debug!("Cloning resource repository...");
        match backend {
            GitBackend::Git => git::clone(url, branch, dest, cert)?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::clone(url, branch, dest, cert)?,
        }
    }

    Ok(())
}

mod git {
    use std::{path::Path, process::Command};

    use anyhow::{Context, Result, bail};

    use super::StatusExt;
    use crate::config::cli::resource::Certificate;

    fn setup_cert(cmd: &mut Command, cert: Option<&Certificate>) -> Result<()> {
        match cert {
            Some(Certificate::SshKey { path, passphrase }) => {
                if !passphrase.compatible_with_git() {
                    bail!(
                        "Pass passphrase to git is not supported,
                        you will also need to provide the passphrase to the terminal.
                        please use git2 backend or use ssh-agent to authenticate"
                    );
                }

                cmd.env(
                    "GIT_SSH_COMMAND",
                    format!("ssh -i {}", path.to_str().context("Invalid path")?),
                );
            }
            Some(Certificate::SshAgent) | None => {} // git uses ssh-agent by default
        }

        Ok(())
    }

    pub fn clone(
        url: &str,
        branch: Option<&str>,
        dest: &Path,
        cert: Option<&Certificate>,
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

        setup_cert(&mut cmd, cert)?;

        cmd.status()
            .check()
            .context("Failed to clone resource repository")?;

        Ok(())
    }

    pub fn pull(repo: &Path, branch: Option<&str>, cert: Option<&Certificate>) -> Result<()> {
        let mut cmd = std::process::Command::new("git");

        cmd.args(["pull", "origin"]);

        if let Some(branch) = branch {
            cmd.arg(branch);
        }

        cmd.arg("--ff-only");

        setup_cert(&mut cmd, cert)?;

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

    use anyhow::{Context, Result, bail};
    use git2::{Repository, build::RepoBuilder};
    use log::debug;

    use crate::config::cli::resource::Certificate;

    fn create_fetch_options(cert: &Certificate) -> git2::FetchOptions<'_> {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_, username, _| {
            username
                .map(|username| cert.fetch(username))
                .unwrap_or(Err(git2::Error::from_str("No username provided")))
        });
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        fetch_options
    }

    pub fn clone(
        url: &str,
        branch: Option<&str>,
        dest: &Path,
        cert: Option<&Certificate>,
    ) -> Result<()> {
        let mut builder = RepoBuilder::new();

        if let Some(branch) = branch {
            builder.branch(branch);
        }

        if let Some(cert) = cert {
            let fetch_options = create_fetch_options(cert);
            builder.fetch_options(fetch_options);
        }

        builder
            .clone(url, dest)
            .context("Failed to clone resource repository")?;

        Ok(())
    }

    pub fn pull(repo: &Path, branch: Option<&str>, cert: Option<&Certificate>) -> Result<()> {
        let repo = Repository::open(repo).context("Failed to open resource repository")?;

        let branch = branch.unwrap_or("main");

        let mut fetch_options = cert.map(create_fetch_options);

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

            let refname = format!("refs/heads/{branch}");
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
