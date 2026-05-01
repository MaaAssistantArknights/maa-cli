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
        std::process::Command::new("tasklist")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .is_some_and(|output| {
                output.contains("AndrowsVm.exe") || output.contains("ABoxHeadless.exe")
            })
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

