//! GitHub Actions utilities for interacting with workflow context and outputs.

use std::{fs, io::Write};

use anyhow::{Context, Result};

use crate::env;

/// Set a GitHub Actions output variable.
pub fn set_output(key: &str, value: &str) -> Result<()> {
    let github_output = env::var("GITHUB_OUTPUT")?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&github_output)
        .with_context(|| format!("Failed to open {github_output}"))?;
    writeln!(file, "{key}={value}")?;
    Ok(())
}
