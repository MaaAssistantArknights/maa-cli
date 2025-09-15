use std::process::Command;

use serde_json;

pub fn ld_extra(
    emulator_path: &Option<String>,
    emulator_index: &Option<i32>,
) -> anyhow::Result<String> {
    let path = emulator_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("emulator_path is required for LDPlayer"))?;
    let index = emulator_index.unwrap_or(0);
    let mut ldconsole_path = std::path::PathBuf::from(path);
    ldconsole_path.push("ldconsole.exe");
    if !ldconsole_path.exists() {
        return Err(anyhow::anyhow!(
            "ldconsole.exe not found in the specified emulator_path"
        ));
    }
    let output = match Command::new(&ldconsole_path).arg("list2").output() {
        Ok(output) => output,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to execute ldconsole.exe: {}",
                e.to_string()
            ));
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pid_found: Option<i32> = None;
    let index_str = index.to_string();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 && parts[0] == index_str {
            pid_found = parts[5].parse::<i32>().ok();
            break;
        }
    }
    if let Some(pid) = pid_found {
        let object = serde_json::json!({
            "path": path,
            "index": index,
            "pid": pid,
        });
        Ok(serde_json::to_string(&object)?)
    } else {
        Err(anyhow::anyhow!(
            "No running instance found for LDPlayer with index {}",
            index
        ))
    }
}
