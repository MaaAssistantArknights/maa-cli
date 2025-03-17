use anyhow::{Context, Result};
use log::{info, trace};

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct WaydroidApp<'a> {
    address: &'a str,
}

impl<'a> WaydroidApp<'a> {
    pub const fn new(address: &'a str) -> Self {
        Self { address }
    }

    fn connect(&self) -> Result<bool> {
        let ret = String::from_utf8(
            std::process::Command::new("adb")
                .arg("devices")
                .output()?
                .stdout,
        )?;

        Ok(ret
            .lines()
            .filter(|line| line.contains(self.address))
            .filter(|line| line.contains("device"))
            .count()
            != 0)
    }
}

#[async_trait::async_trait]
impl super::ExternalApp for WaydroidApp<'_> {
    async fn open(&self) -> Result<()> {
        if self.connect().is_ok_and(|b| b) {
            info!("Game is already running!");
            return Ok(());
        }

        info!("Starting waydroid");
        let mut task = std::process::Command::new("waydroid")
            .arg("session")
            .arg("start")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start game!")?;

        let mut rdr = std::io::BufReader::new(task.stderr.take().unwrap());
        let mut buf = String::new();

        // Wait for game ready
        loop {
            use std::io::BufRead;

            rdr.read_line(&mut buf)?;
            trace!("{}", buf);
            if buf.contains("ADB") {
                info!("Game ready!");
                break;
            }
            trace!("Waiting for game ready...");
        }

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        info!("Closing waydroid");
        std::process::Command::new("waydroid")
            .arg("session")
            .arg("stop")
            .spawn()
            .context("Failed to start game!")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from() {
        assert_eq!(
            WaydroidApp::new("localhost:1717"),
            WaydroidApp {
                address: "localhost:1717",
            },
        );
    }
}
