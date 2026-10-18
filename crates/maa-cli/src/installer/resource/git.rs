use anyhow::{Result, bail};
use log::{debug, warn};

use crate::{
    config::cli::resource::{Certificate, GitBackend, GitConfig},
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

pub(super) fn update(config: &GitConfig, warn_on_failure: bool) -> Result<()> {
    let backend = config.backend;
    let url = config.remote.url();
    let branch = config.remote.branch();
    let cert = config.remote.certificate();
    let dest = dirs::maa_resource();

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
    // Preserve legacy semantics: preflight errors above always fail; only
    // repository clone/pull failures may be downgraded to warnings.
    let result = update_core(backend, url, dest, branch, cert);
    if warn_on_failure {
        if let Err(error) = result {
            warn!("Failed to update resource repository: {error}");
        }
        Ok(())
    } else {
        result
    }
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
            GitBackend::Git => command::pull(dest, branch, cert)?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::pull(dest, branch, cert)?,
        }
    } else {
        debug!("Cloning resource repository...");
        match backend {
            GitBackend::Git => command::clone(url, branch, dest, cert)?,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2 => git2::clone(url, branch, dest, cert)?,
        }
    }

    Ok(())
}

mod command {
    use std::{path::Path, process::Command};

    use anyhow::{Context, Result, bail};

    use super::StatusExt;
    use crate::config::cli::{resource::Certificate, secret::Secret};

    fn supports_git_auth_prompt(passphrase: &Secret) -> bool {
        matches!(passphrase, Secret::None | Secret::Prompt)
    }

    fn setup_cert(cmd: &mut Command, cert: Option<&Certificate>) -> Result<()> {
        match cert {
            Some(Certificate::SshKey { path, passphrase }) => {
                if !supports_git_auth_prompt(passphrase) {
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

        cmd.args(["clone", "--depth=1", url]);
        cmd.arg(dest);

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

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;

    fn git_command(repo: &Path, args: &[&str]) {
        // Local fixture commands only; no global config, home files, or network.
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn missing_ssh_authentication_is_fatal_even_in_warning_mode() {
        // This fails validation before any repository access.
        let config: GitConfig =
            toml::from_str("url = \"git@example.invalid:resource.git\"").unwrap();
        for warn in [true, false] {
            assert!(
                update(&config, warn)
                    .unwrap_err()
                    .to_string()
                    .contains("Certificate is required")
            );
        }
    }

    #[test]
    fn clone_and_update_custom_branch_with_each_git_implementation() {
        // Unwraps assert isolated local repository fixture setup.
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        std::fs::create_dir(&source).unwrap();
        git_command(&source, &["init", "-b", "resource-test"]);
        let commit = || {
            git_command(&source, &["add", "."]);
            git_command(&source, &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-m",
                "resource fixture",
            ]);
        };
        std::fs::write(source.join("data"), "v1").unwrap();
        commit();
        let backends = [
            GitBackend::Git,
            #[cfg(feature = "git2")]
            GitBackend::Libgit2,
        ];
        let url = source.to_str().unwrap();
        for (index, backend) in backends.iter().enumerate() {
            let dest = tmp.path().join(format!("checkout-{index}"));
            update_core(*backend, url, &dest, Some("resource-test"), None).unwrap();
            assert_eq!(std::fs::read_to_string(dest.join("data")).unwrap(), "v1");
        }
        std::fs::write(source.join("data"), "v2").unwrap();
        commit();
        for (index, backend) in backends.iter().enumerate() {
            let dest = tmp.path().join(format!("checkout-{index}"));
            update_core(*backend, url, &dest, Some("resource-test"), None).unwrap();
            assert_eq!(std::fs::read_to_string(dest.join("data")).unwrap(), "v2");
        }
    }
}
