//! GitHub Actions output helpers.

use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::{Context, Result};

use crate::env;

fn open_github_output() -> Result<File> {
    let github_output = env::var("GITHUB_OUTPUT")?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&github_output)
        .with_context(|| format!("Failed to open {github_output}"))
}

fn set_output_to(file: &mut File, key: &str, value: &str) -> Result<()> {
    writeln!(file, "{key}={value}")?;
    Ok(())
}

/// Set a GitHub Actions output variable.
///
/// Writes to the file specified by GITHUB_OUTPUT environment variable.
pub fn set_output(key: &str, value: &str) -> Result<()> {
    let mut file = open_github_output()?;
    set_output_to(&mut file, key, value)
}

/// Set multiple GitHub Actions output variables at once.
pub fn set_outputs(outputs: &[(&str, &str)]) -> Result<()> {
    let mut file = open_github_output()?;
    for (key, value) in outputs {
        set_output_to(&mut file, key, value)?;
    }
    Ok(())
}
