//! Simple command execution helpers.

use std::process::Command;

use anyhow::{Context, Result};

/// Execute a command and return its stdout as a trimmed string.
///
/// # Example
/// ```ignore
/// let commit = run("git", &["rev-parse", "HEAD"])?;
/// ```
pub fn run(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Command failed: {program} {}:\n{}",
            args.join(" "),
            stderr.trim()
        );
    }

    let stdout = String::from_utf8(output.stdout).with_context(|| {
        format!(
            "Command output was not valid UTF-8: {program} {}",
            args.join(" ")
        )
    })?;

    Ok(stdout)
}
