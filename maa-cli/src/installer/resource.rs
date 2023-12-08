use crate::{
    config::{cli::resource::Config, installer_config},
    debug, dirs,
};

use anyhow::{Context, Result};
use git2::{build::RepoBuilder, Repository};

pub struct ResourceRepository {
    repo: Repository,
    branch: String,
    updated: bool,
}

impl ResourceRepository {
    pub fn new(config: Config) -> Result<Self> {
        let repo_path = dirs::hot_update();

        if repo_path.exists() {
            debug!("Resource repository found at", repo_path.display());
            Ok(Self {
                repo: Repository::open(repo_path)?,
                branch: config.remote().branch().to_owned(),
                updated: false,
            })
        } else {
            let remote = config.remote();
            let url = remote.url();
            let branch = remote.branch();

            let repo = RepoBuilder::new().branch(branch).clone(&url, repo_path)?;

            debug!("Resource repository not found, cloning from", url);
            Ok(Self {
                repo,
                branch: branch.to_owned(),
                updated: true,
            })
        }
    }

    pub fn update(&mut self) -> Result<()> {
        debug!("Updating resource repository");
        self.repo
            .find_remote("origin")
            .context("Failed to find remote 'origin'")?
            .fetch(&[&self.branch], None, None)?;
        self.updated = true;
        Ok(())
    }
}

pub fn update(is_auto: bool) -> Result<()> {
    let config = installer_config().resource_config();

    // Skip auto update if auto update is disabled
    if is_auto && !config.auto_update() {
        return Ok(());
    }

    let mut repo = ResourceRepository::new(config)?;
    repo.update()
}
