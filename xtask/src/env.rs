//! Environment variable utilities.

use std::env;

use anyhow::{Context, Result};

/// Get an environment variable with context.
///
/// This is a helper that provides better error messages than `env::var()`.
///
/// # Example
/// ```
/// let value = env::var("MY_VAR")?;
/// ```
pub fn var(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("{key} environment variable not set"))
}
