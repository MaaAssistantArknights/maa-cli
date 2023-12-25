use crate::config::task::InitializedTaskConfig;

use anyhow::{Context, Result};
use log::{info, warn};

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct PlayCoverApp<'a> {
    name: &'a str,
    start_app: bool,
    close_app: bool,
}

impl<'n> PlayCoverApp<'n> {
    pub fn from(task_config: &InitializedTaskConfig) -> Option<Self> {
        if task_config.start_app || task_config.close_app {
            Some(Self {
                name: task_config.client_type.unwrap_or_default().app(),
                start_app: task_config.start_app,
                close_app: task_config.close_app,
            })
        } else {
            None
        }
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
        if !self.start_app {
            return Ok(());
        }

        if self.is_running().unwrap_or(false) {
            info!("Game is already running!");
            return Ok(());
        }

        info!("Starting app: {}", self.name);
        std::process::Command::new("open")
            .arg("-a")
            .arg(self.name)
            .status()
            .context("Failed to start game!")?;

        // Wait for game ready
        // TODO: Find a way to detect if game is ready, so we can remove this sleep
        // The is_running() function is not enough
        // maybe we can launch the game by macOS API instead of open command?
        std::thread::sleep(std::time::Duration::from_secs(5));

        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        if !self.close_app {
            return Ok(());
        }

        if !self.is_running().unwrap_or(true) {
            warn!("Game is not running!");
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::task::ClientType;

    #[test]
    fn from() {
        assert_eq!(
            PlayCoverApp::from(&InitializedTaskConfig {
                start_app: true,
                close_app: true,
                client_type: Some(ClientType::Official),
                tasks: vec![],
            }),
            Some(PlayCoverApp {
                name: "明日方舟",
                start_app: true,
                close_app: true,
            })
        );

        assert_eq!(
            PlayCoverApp::from(&InitializedTaskConfig {
                start_app: true,
                close_app: false,
                client_type: None,
                tasks: vec![],
            }),
            Some(PlayCoverApp {
                name: "明日方舟",
                start_app: true,
                close_app: false,
            })
        );

        assert_eq!(
            PlayCoverApp::from(&InitializedTaskConfig {
                start_app: true,
                close_app: false,
                client_type: Some(ClientType::YoStarEN),
                tasks: vec![],
            }),
            Some(PlayCoverApp {
                name: "Arknights",
                start_app: true,
                close_app: false,
            })
        );

        assert_eq!(
            PlayCoverApp::from(&InitializedTaskConfig {
                start_app: false,
                close_app: false,
                client_type: Some(ClientType::Official),
                tasks: vec![],
            }),
            None
        );
    }
}
