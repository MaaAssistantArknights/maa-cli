use std::{env, net::TcpStream};

use anyhow::{Context, Result, bail};
use log::{info, trace, warn};

use crate::config::task::ClientType;

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct PlayCoverApp<'a> {
    client: ClientType,
    address: &'a str,
}

impl<'a> PlayCoverApp<'a> {
    pub const fn new(client: ClientType, address: &'a str) -> Self {
        Self { client, address }
    }

    fn connect(&self) -> Result<TcpStream> {
        let stream = TcpStream::connect(self.address).context("Failed to connect to game!")?;

        Ok(stream)
    }
}

impl super::ExternalApp for PlayCoverApp<'_> {
    fn open(&self, start_if_needed: bool) -> Result<Option<String>> {
        if !start_if_needed {
            return Ok(None);
        }

        if self.connect().is_ok() {
            info!("Game is already running!");
            return Ok(None);
        }

        let bundle_id = self
            .client
            .bundle_id()
            .with_context(|| format!("Client {} is not available on the App Store", self.client))?;

        let success = if let Some(home) = env::home_dir() {
            info!("Starting app: {bundle_id} by path");
            std::process::Command::new("open")
                .arg("-a")
                .arg(
                    home.join("Library")
                        .join("Containers")
                        .join("io.playcover.PlayCover")
                        .join("Applications")
                        .join(format!("{bundle_id}.app")),
                )
                .status()
                .context("Failed to start game!")?
                .success()
        } else {
            false
        };
        if !success {
            warn!("Failed to start game by path, fall back to bundle id");
            info!("Starting app: {bundle_id} by bundle id");
            let status = std::process::Command::new("open")
                .arg("-b")
                .arg(bundle_id)
                .status()
                .context("Failed to start game!")?;
            if !status.success() {
                bail!("Failed to start game with bundle identifier {bundle_id}: {status}");
            }
        }

        // Wait for game ready
        loop {
            if self.connect().is_ok() {
                info!("Game ready!");
                break;
            }
            trace!("Waiting for game ready...");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        Ok(None)
    }

    fn close(&self) -> Result<()> {
        // MaaCore will close the game, so we don't need to do anything here
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn from() {
        use crate::config::task::ClientType::*;
        assert_eq!(
            PlayCoverApp::new(Official, "localhost:1717"),
            PlayCoverApp {
                client: Official,
                address: "localhost:1717",
            },
        );
    }
}
