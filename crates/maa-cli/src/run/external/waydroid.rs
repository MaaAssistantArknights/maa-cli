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

    fn adb_connect(&self) -> Result<bool> {
        std::process::Command::new("adb")
            .args(["disconnect", self.address])
            .output()?;

        let output = String::from_utf8(
            std::process::Command::new("waydroid")
                .args(["adb", "connect"])
                .output()?
                .stdout,
        )?;
        trace!("{}", output.trim());
        Ok(output.contains("Established ADB connection"))
    }
}

fn waydroid_session_running() -> Result<bool> {
    let output = String::from_utf8(
        std::process::Command::new("waydroid")
            .arg("status")
            .output()?
            .stdout,
    )?;
    Ok(output
        .lines()
        .any(|l| l.starts_with("Session:") && l.contains("RUNNING")))
}

fn start_waydroid_session() -> Result<()> {
    let mut task = std::process::Command::new("cage")
        .args(["-d", "waydroid", "session", "start"])
        .env("WAYLAND_DISPLAY", "wayland-0")
        .env("WLR_BACKENDS", "wayland")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start Waydroid!")?;

    let mut rdr = std::io::BufReader::new(task.stderr.take().expect("stderr is piped"));
    let mut line = String::new();

    loop {
        use std::io::BufRead;

        line.clear();
        if rdr.read_line(&mut line)? == 0 {
            break;
        }
        trace!("{}", line.trim_end());
        if line.contains("Android with user 0 is ready")
            || line.contains("Established ADB connection")
        {
            break;
        }
    }

    Ok(())
}

impl super::ExternalApp for WaydroidApp<'_> {
    fn open(&self) -> Result<bool> {
        if !waydroid_session_running()? {
            info!("Starting Waydroid session");
            start_waydroid_session()?;
        }

        self.adb_connect()
    }

    fn close(&self) -> Result<()> {
        info!("Closing Waydroid");
        std::process::Command::new("waydroid")
            .args(["session", "stop"])
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
        assert_eq!(WaydroidApp::new("localhost:1717"), WaydroidApp {
            address: "localhost:1717",
        });
    }
}
