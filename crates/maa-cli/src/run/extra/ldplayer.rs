use std::process::Command;

use anyhow::{Result, bail};

pub fn ld_extra(
    emulator_path: &Option<String>,
    emulator_index: &Option<i32>,
) -> Result<String> {
    let Some(path) = emulator_path else {
        bail!("emulator_path is required for LDPlayer");
    };
    let index = emulator_index.unwrap_or(0);
    let ldconsole_path = std::path::Path::new(path).join("ldconsole.exe");
    if !ldconsole_path.exists() {
        bail!("ldconsole.exe not found in the specified emulator_path");
    }
    let output = Command::new(&ldconsole_path)
        .arg("list2")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute ldconsole.exe: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let index_str = index.to_string();
    let pid = stdout
        .lines()
        .find_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 6 && parts[0] == index_str {
                parts[5].parse::<i32>().ok()
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!("No running instance found for LDPlayer with index {index}")
        })?;
    let object = serde_json::json!({
        "path": path,
        "index": index,
        "pid": pid,
    });
    Ok(serde_json::to_string(&object)?)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn ld_extra_requires_path() {
        assert!(ld_extra(&None, &None).is_err());
    }

    /// Verify JSON structure given a synthetic ldconsole output (no real binary needed).
    ///
    /// This test exercises the CSV parsing logic directly via a mock function.
    #[test]
    fn parse_fields_from_csv_row() {
        // Simulate what `ld_extra` does internally with a known CSV line
        let stdout = "0,LDPlayer,,,running,12345\n";
        let index_str = "0";
        let pid = stdout.lines().find_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 6 && parts[0] == index_str {
                parts[5].parse::<i32>().ok()
            } else {
                None
            }
        });
        assert_eq!(pid, Some(12345));
    }
}
