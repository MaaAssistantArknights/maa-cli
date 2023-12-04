use crate::{config::task::ClientType, info, warning};

use anyhow::{Context, Result};

pub struct PlayCoverApp<'n> {
    name: &'n str,
}

impl<'n> PlayCoverApp<'n> {
    pub fn new(name: &'n str) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &'n str {
        self.name
    }

    fn is_running(&self) -> Result<bool> {
        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events" to count processes whose name is "Arknights""#)
            .output()
            .context("Failed to check if game is running!")?;
        let output = String::from_utf8_lossy(&output.stdout);
        Ok(output.trim() != "0")
    }

    pub fn open(&self) -> Result<()> {
        if self.is_running().unwrap_or(false) {
            info!("Game is already running!");
            return Ok(());
        }

        info!("Starting game...");
        std::process::Command::new("open")
            .arg("-a")
            .arg(self.name)
            .status()
            .context("Failed to start game!")?;
        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        if !self.is_running().unwrap_or(true) {
            warning!("Game is not running!");
            return Ok(());
        }

        info!("Closing game...");
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!("quit app \"{}\"", self.name))
            .status()
            .context("Failed to close game!")?;
        Ok(())
    }
}

impl Default for PlayCoverApp<'static> {
    fn default() -> Self {
        Self::new("明日方舟")
    }
}

impl From<ClientType> for PlayCoverApp<'static> {
    fn from(client: ClientType) -> Self {
        Self::new(match client {
            ClientType::Official | ClientType::Bilibili | ClientType::Txwy => "明日方舟",
            ClientType::YoStarEN => "Arknights",
            ClientType::YoStarJP => "アークナイツ",
            ClientType::YoStarKR => "명일방주",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_client_type() {
        assert_eq!(PlayCoverApp::from(ClientType::Official).name, "明日方舟");
        assert_eq!(PlayCoverApp::from(ClientType::Bilibili).name, "明日方舟");
        assert_eq!(PlayCoverApp::from(ClientType::Txwy).name, "明日方舟");
        assert_eq!(PlayCoverApp::from(ClientType::YoStarEN).name, "Arknights");
        assert_eq!(
            PlayCoverApp::from(ClientType::YoStarJP).name,
            "アークナイツ"
        );
        assert_eq!(PlayCoverApp::from(ClientType::YoStarKR).name, "명일방주");
    }
}
