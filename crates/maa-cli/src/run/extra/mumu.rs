use anyhow::{Result, bail};

pub fn mumu_extra(emulator_path: &Option<String>, emulator_index: &Option<i32>) -> Result<String> {
    let Some(path) = emulator_path else {
        bail!("emulator_path is required for MuMu emulator");
    };
    let mut json_map = serde_json::Map::new();
    json_map.insert("path".to_string(), serde_json::Value::String(path.clone()));
    if let Some(index) = emulator_index {
        json_map.insert(
            "index".to_string(),
            serde_json::Value::Number(serde_json::Number::from(*index)),
        );
    }
    Ok(serde_json::to_string(&json_map)?)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn mumu_extra_requires_path() {
        assert!(mumu_extra(&None, &None).is_err());
    }

    #[test]
    fn mumu_extra_path_only() {
        let json = mumu_extra(&Some("/path/to/mumu".into()), &None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["path"], "/path/to/mumu");
        assert!(v.get("index").is_none());
    }

    #[test]
    fn mumu_extra_with_index() {
        let json = mumu_extra(&Some("/path/to/mumu".into()), &Some(2)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["path"], "/path/to/mumu");
        assert_eq!(v["index"], 2);
    }
}
