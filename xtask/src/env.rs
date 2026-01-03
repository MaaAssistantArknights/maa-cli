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

/// Get an environment variable with a default value if not set.
///
/// # Example
/// ```
/// let value = env::var_or("MY_VAR", "default");
/// ```
pub fn var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}
