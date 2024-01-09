use crate::config::task::ClientType;

use anyhow::{Context, Result};
use tokio::{io::AsyncWriteExt, net::TcpStream};

// M A A 0x00 0x00 0x04 T E R M
const TERMINATE: &[u8] = &[0x4d, 0x41, 0x41, 0x00, 0x00, 0x04, 0x54, 0x45, 0x52, 0x4d];

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct PlayCoverApp {
    client: ClientType,
    address: String,
    start: bool,
    close: bool,
}

impl PlayCoverApp {
    pub fn new(start: bool, close: bool, client: ClientType, address: String) -> Option<Self> {
        if start || close {
            Some(Self {
                client,
                address,
                start,
                close,
            })
        } else {
            None
        }
    }

    async fn connect(&self) -> Result<TcpStream> {
        let stream = TcpStream::connect(&self.address)
            .await
            .with_context(lfl!("failed-connect-game"))?;

        Ok(stream)
    }

    pub async fn open(&self) -> Result<()> {
        if !self.start {
            return Ok(());
        }

        if self.connect().await.is_ok() {
            info!("game-is-running");
            return Ok(());
        }

        let app = self.client.app();
        info!("start-game", name = app);
        std::process::Command::new("open")
            .arg("-a")
            .arg(app)
            .status()
            .with_context(lfl!("failed-start-game"))?;

        // Wait for game ready
        loop {
            if self.connect().await.is_ok() {
                info!("game-ready");
                break;
            }
            trace!("waiting-for-game");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        if !self.close {
            return Ok(());
        }

        if let Ok(mut stream) = self.connect().await {
            stream.write_all(TERMINATE).await?;
        }

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
            PlayCoverApp::new(true, true, Official, "localhost:1717".to_string(),),
            Some(PlayCoverApp {
                start: true,
                close: true,
                client: Official,
                address: "localhost:1717".to_string(),
            })
        );

        assert_eq!(
            PlayCoverApp::new(false, true, Official, "localhost:1717".to_string(),),
            Some(PlayCoverApp {
                start: false,
                close: true,
                client: Official,
                address: "localhost:1717".to_string(),
            })
        );

        assert_eq!(
            PlayCoverApp::new(false, false, Official, "localhost:1717".to_string(),),
            None
        );
    }
}
