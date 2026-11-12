use std::{
    io::{BufRead, BufReader},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use log::{info, trace};

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct WaydroidApp;

impl WaydroidApp {
    pub fn new() -> Self {
        Self
    }
}

fn run_waydroid(args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("waydroid")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run `waydroid {}`", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();

    if !out.status.success() {
        bail!(
            "`waydroid {}` failed with status {}{}{}",
            args.join(" "),
            out.status,
            if stdout.is_empty() {
                String::new()
            } else {
                format!("\nstdout: {stdout}")
            },
            if stderr.is_empty() {
                String::new()
            } else {
                format!("\nstderr: {stderr}")
            },
        );
    }

    Ok(stdout)
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

const ADB_CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);

enum AdbWait {
    Connected,
    Closed(String),
}

fn wait_for_adb_connection(
    line_rx: &mpsc::Receiver<std::io::Result<String>>,
    timeout: Duration,
) -> Result<AdbWait> {
    let deadline = Instant::now() + timeout;
    let timeout_secs = timeout.as_secs();
    let mut last_line = String::new();

    loop {
        if Instant::now() >= deadline {
            bail!(
                "Waydroid did not establish an ADB connection within {timeout_secs}s \
                 (last line: {last_line:?})"
            );
        }

        let wait = deadline.saturating_duration_since(Instant::now());
        match line_rx.recv_timeout(wait) {
            Ok(Ok(line)) => {
                last_line.clear();
                last_line.push_str(line.trim_end());
                trace!("[waydroid] {}", line.trim_end());
                if line.contains("Established ADB connection") {
                    return Ok(AdbWait::Connected);
                }
            }
            Ok(Err(err)) => return Err(err).context("Failed to read waydroid stderr"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(AdbWait::Closed(last_line)),
        }
    }
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

    let (line_tx, line_rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });

    let AdbWait::Closed(last_line) = wait_for_adb_connection(&line_rx, ADB_CONNECTION_TIMEOUT)?
    else {
        return Ok(());
    };

    if let Some(status) = child
        .try_wait()
        .context("Failed to query Waydroid process status")?
    {
        if !status.success() {
            bail!("`waydroid session start` exited with {status}");
        }
    }

    let (running, _) = parse_status(&run_waydroid(&["status"])?);
    if !running {
        bail!("Waydroid session did not become ready. Last read line: {last_line}");
    }
    Ok(())
}

fn waydroid_adb_connect() -> Result<()> {
    let out = std::process::Command::new("waydroid")
        .args(["adb", "connect"])
        .output()
        .context("Failed to run `waydroid adb connect`")?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();

    trace!("[waydroid] stdout: {stdout}");
    trace!("[waydroid] stderr: {stderr}");

    if !out.status.success() {
        bail!(
            "`waydroid adb connect` failed with status {}{}{}",
            out.status,
            if stdout.is_empty() {
                String::new()
            } else {
                format!("\nstdout: {stdout}")
            },
            if stderr.is_empty() {
                String::new()
            } else {
                format!("\nstderr: {stderr}")
            },
        );
    }
    Ok(())
}

impl super::ExternalApp for WaydroidApp {
    fn open(&self, start_if_needed: bool) -> Result<Option<String>> {
        if start_if_needed {
            let (running, _) = parse_status(&run_waydroid(&["status"])?);
            if !running {
                info!("Starting Waydroid session");
                start_waydroid_session()?;
            } else {
                waydroid_adb_connect()?;
            }
        }

        let (_, address) = parse_status(&run_waydroid(&["status"])?);
        let address = address.map(|address| format!("{address}:5555"));

        // We always need an address
        if address.is_none() {
            Err(anyhow!("Waydroid failed to provide a device address"))
        } else {
            Ok(address)
        }
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
        assert_eq!(WaydroidApp::new(), WaydroidApp);
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

    #[test]
    fn wait_for_adb_connection_established() {
        let (line_tx, line_rx) = mpsc::channel();
        line_tx
            .send(Ok(
                "Established ADB connection to Waydroid device at 192.168.240.112.".to_string(),
            ))
            .unwrap();

        assert!(matches!(
            wait_for_adb_connection(&line_rx, Duration::from_secs(1)),
            Ok(AdbWait::Connected)
        ));
    }

    #[test]
    fn wait_for_adb_connection_times_out_with_last_line() {
        let (line_tx, line_rx) = mpsc::channel();
        line_tx
            .send(Ok("Android with user 0 is ready".to_string()))
            .unwrap();

        let err = match wait_for_adb_connection(&line_rx, Duration::from_millis(50)) {
            Err(err) => err,
            Ok(_) => panic!("expected a timeout"),
        };
        let message = err.to_string();
        assert!(
            message.contains("Android with user 0 is ready"),
            "{message}"
        );
    }

    #[test]
    fn wait_for_adb_connection_closed() {
        let (line_tx, line_rx) = mpsc::channel();
        line_tx
            .send(Ok("Android with user 0 is ready".to_string()))
            .unwrap();
        drop(line_tx);

        match wait_for_adb_connection(&line_rx, Duration::from_secs(1)) {
            Ok(AdbWait::Closed(line)) => assert_eq!(line, "Android with user 0 is ready"),
            _ => panic!("expected the closed outcome"),
        }
    }
}
