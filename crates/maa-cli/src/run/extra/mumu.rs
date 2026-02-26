use anyhow::{Result, anyhow};
use serde_json;

pub fn mumu_extra(emulator_path: &Option<String>, emulator_index: &Option<i32>) -> Result<String> {
    let mut json_map = serde_json::Map::new();
    if let Some(path) = &emulator_path {
        json_map.insert("path".to_string(), serde_json::Value::String(path.clone()));
    } else {
        return Err(anyhow!("emulator_path is required for MuMu emulator"));
    }
    if let Some(index) = &emulator_index {
        json_map.insert(
            "index".to_string(),
            serde_json::Value::Number(serde_json::Number::from(*index)),
        );
    }
    Ok(serde_json::to_string(&json_map)?)
}
