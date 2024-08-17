use crate::config::task::ClientType;

use anyhow::{Context, Result};
use log::{info, trace};
use tokio::net::TcpStream;

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct PlayCoverApp<'a> {
    client: ClientType,
    address: &'a str,
}

impl<'a> PlayCoverApp<'a> {
    pub const fn new(client: ClientType, address: &'a str) -> Self {
        Self { client, address }
    }

    async fn connect(&self) -> Result<TcpStream> {
        let stream = TcpStream::connect(self.address)
            .await
            .context("Failed to connect to game!")?;

        Ok(stream)
    }
}

#[async_trait::async_trait]
impl<'a> super::ExternalApp for PlayCoverApp<'a> {
    async fn open(&self) -> Result<()> {
        if self.connect().await.is_ok() {
            info!("Game is already running!");
            return Ok(());
        }

        let app = self.client.app();
        info!("Starting app: {}", app);
        std::process::Command::new("open")
            .arg("-a")
            .arg(app)
            .status()
            .context("Failed to start game!")?;

        // Wait for game ready
        loop {
            if self.connect().await.is_ok() {
                info!("Game ready!");
                break;
            }
            trace!("Waiting for game ready...");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // MaaCore will close the game, so we don't need to do anything here
        Ok(())
    }
}

#[cfg(test)]
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
