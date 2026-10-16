//! Minimal argument helpers for workspace automation commands.

use std::ffi::OsString;

use anyhow::{Context, Result};

/// Take the next argument as an option's required value.
pub fn next_value(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<OsString> {
    args.next()
        .filter(|value| !value.as_encoded_bytes().starts_with(b"-"))
        .with_context(|| format!("Missing value for {option}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_value_names_option() {
        let error = next_value(&mut std::iter::empty(), "--profile").unwrap_err();
        assert_eq!(error.to_string(), "Missing value for --profile");
        let error = next_value(
            &mut [OsString::from("--vendored-deps")].into_iter(),
            "--profile",
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "Missing value for --profile");
    }
}
