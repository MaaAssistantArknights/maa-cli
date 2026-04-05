use std::{cell::RefCell, io::BufRead as _};

use anyhow::{Context, Result, bail};
use log::{info, trace};

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct WaydroidApp {
    address: RefCell<Option<String>>,
}

impl WaydroidApp {
    pub fn new() -> Self {
        Self {
            address: RefCell::new(None),
        }
    }
}

fn run_waydroid(args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("waydroid")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run `waydroid {}`", args.join(" ")))?;
    Ok(String::from_utf8(out.stdout)?)
}

fn parse_status(output: &str) -> (bool, Option<String>) {
    let running = output
        .lines()
        .any(|l| l.starts_with("Session:") && l.contains("RUNNING"));
    let address = output
        .lines()
        .find_map(|l| l.strip_prefix("IP address:").map(|v| v.trim().to_owned()));
    (running, address)
}

fn start_waydroid_session() -> Result<()> {
    let mut child = std::process::Command::new("waydroid")
        .args(["session", "start"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn `waydroid session start`")?;

    let stderr = child
        .stderr
        .take()
        .context("Failed to capture waydroid stderr")?;
    let mut rdr = std::io::BufReader::new(stderr);
    let mut line = String::new();

    loop {
        line.clear();
        match rdr.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                trace!("[waydroid] {}", line.trim_end());
                if line.contains("Established ADB connection") {
                    std::mem::forget(rdr);
                    std::mem::forget(child);
                    return Ok(());
                }
            }
            Err(e) => return Err(e).context("Failed to read waydroid startup output"),
        }
    }
    let status = child
        .wait()
        .context("Failed to wait for `waydroid session start`")?;
    if !status.success() {
        bail!("`waydroid session start` exited with {status}");
    }
    let (running, _) = parse_status(&run_waydroid(&["status"])?);
    if !running {
        bail!("Waydroid session did not become ready");
    }
    Ok(())
}

fn waydroid_adb_connect() -> Result<()> {
    let out = std::process::Command::new("waydroid")
        .args(["adb", "connect"])
        .output()
        .context("Failed to run `waydroid adb connect`")?;
    trace!("[waydroid] {}", String::from_utf8_lossy(&out.stderr).trim());
    Ok(())
}

impl super::ExternalApp for WaydroidApp {
    fn open(&self) -> Result<bool> {
        let (running, address) = parse_status(&run_waydroid(&["status"])?);
        if !running {
            info!("Starting Waydroid session");
            start_waydroid_session()?;
        } else {
            waydroid_adb_connect()?;
        }

        let ip = if running {
            address
        } else {
            parse_status(&run_waydroid(&["status"])?).1
        };
        *self.address.borrow_mut() = ip.map(|s| format!("{s}:5555"));

        Ok(false)
    }

    fn actual_address(&self) -> Result<Option<String>> {
        if let Some(addr) = self.address.borrow().clone() {
            return Ok(Some(addr));
        }
        let (_, ip) = parse_status(&run_waydroid(&["status"])?);
        Ok(ip.map(|s| format!("{s}:5555")))
    }

    fn close(&self) -> Result<()> {
        info!("Stopping Waydroid session");
        let status = std::process::Command::new("waydroid")
            .args(["session", "stop"])
            .status()
            .context("Failed to run `waydroid session stop`")?;
        if !status.success() {
            bail!("`waydroid session stop` exited with {status}");
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn from() {
        assert_eq!(WaydroidApp::new(), WaydroidApp {
            address: RefCell::new(None),
        });
    }

    #[test]
    fn status_running() {
        let (running, _) = parse_status("Session:        RUNNING\nContainer:      RUNNING\n");
        assert!(running);
    }

    #[test]
    fn status_stopped() {
        let (running, _) = parse_status("Session:        STOPPED\nVendor type:    MAINLINE\n");
        assert!(!running);
    }

    #[test]
    fn status_ip_address() {
        let (_, address) = parse_status(
            "Session:        RUNNING\nIP address:     192.168.240.112\nContainer:      RUNNING\n",
        );
        assert_eq!(address, Some("192.168.240.112".to_string()));
    }
}
