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

    fn check_adb_devices(&self) -> Result<bool> {
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

impl super::ExternalApp for WaydroidApp<'_> {
    /// Return true on success and address match
    ///
    /// Return false if given address not in `adb devices``
    fn open(&self) -> Result<bool> {
        if self.check_adb_devices().is_ok_and(|b| b) {
            info!("Waydroid is already running!");
            return Ok(true);
        }

        info!("Starting waydroid");
        let mut task = std::process::Command::new("waydroid")
            .arg("session")
            .arg("start")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start Waydroid!")?;

        let mut rdr = std::io::BufReader::new(task.stderr.take().unwrap());
        let mut buf = String::new();

        // Wait for game ready
        loop {
            use std::io::BufRead;

            rdr.read_line(&mut buf)?;
            trace!("{buf}");
            if buf.contains("ADB") {
                info!("Waydroid ready!");
                break;
            }
            trace!("Waiting for game ready...");
        }

        self.check_adb_devices()
    }

    fn close(&self) -> Result<()> {
        info!("Closing waydroid");
        std::process::Command::new("waydroid")
            .arg("session")
            .arg("stop")
            .spawn()
            .context("Failed to stop Waydroid!")?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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
