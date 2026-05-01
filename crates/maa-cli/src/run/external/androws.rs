use anyhow::{Result, bail};
use log::info;

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct AndrowsApp;

impl AndrowsApp {
    pub const fn new() -> Self {
        Self
    }

    /// Check if Androws is running by looking for AndrowsVm.exe or ABoxHeadless.exe processes.
    fn is_running() -> bool {
        // Try the full path first to avoid PATH lookup issues, then fall back to bare name.
        let output = std::process::Command::new(r"C:\Windows\System32\tasklist.exe")
            .output()
            .or_else(|_| std::process::Command::new("tasklist").output());

        match output {
            Ok(out) => {
                // tasklist may output GBK on non-UTF-8 Windows locales; use lossy decoding
                // so that non-UTF-8 bytes are replaced rather than causing a silent failure.
                let text = String::from_utf8_lossy(&out.stdout);
                text.contains("AndrowsVm.exe") || text.contains("ABoxHeadless.exe")
            }
            Err(_) => false,
        }
    }
}

impl super::ExternalApp for AndrowsApp {
    /// Check whether Androws is running.
    ///
    /// Androws is a standalone emulator managed by the user;
    /// maa-cli only verifies it is already running and does not start it.
    fn open(&self, _start_if_needed: bool) -> Result<Option<String>> {
        if Self::is_running() {
            info!("Androws is already running!");
            // Address is taken from the configured connection address.
            Ok(None)
        } else {
            bail!(
                "Androws does not appear to be running \
                 (AndrowsVm.exe / ABoxHeadless.exe not found in process list)"
            );
        }
    }

    fn close(&self) -> Result<()> {
        // Androws lifecycle is managed by the user; maa-cli does not close it.
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn new() {
        assert_eq!(AndrowsApp::new(), AndrowsApp);
    }
}

