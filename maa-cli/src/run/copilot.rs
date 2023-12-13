use crate::{
    config::task::{
        task_type::MAATask,
        value::input::{BoolInput, Input},
        Task, TaskConfig, MAAValue,
    },
    debug,
    dirs::{self, Ensure},
    info, object, warning,
};

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use prettytable::{format, row, Table};
use serde_json::Value as JsonValue;

const MAA_COPILOT_API: &str = "https://prts.maa.plus/copilot/get/";

pub fn copilot(uri: impl AsRef<str>, resource_dirs: &Vec<PathBuf>) -> Result<TaskConfig> {
    let (value, path) =
        CopilotJson::new(uri.as_ref())?.get_json_and_file(dirs::copilot().ensure()?)?;

    // Determine type of stage
    let task_type = match value["type"].as_str() {
        Some("SSS") => CopilotType::SSSCopilot,
        _ => CopilotType::Copilot,
    };

    // Print stage info
    let stage_id = value["stage_name"]
        .as_str()
        .context("Failed to get stage ID")?;
    let stage_name = task_type.get_stage_name(resource_dirs, stage_id)?;

    info!("Stage:", stage_name);

    // Print operators info
    info!("Operators:\n", operator_table(&value)?);

    // Append task
    let mut task_config = TaskConfig::new();

    task_config.push(task_type.to_task(path.to_str().context("Invalid path")?));

    Ok(task_config)
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum CopilotJson<'a> {
    Code(&'a str),
    File(&'a Path),
}

impl CopilotJson<'_> {
    pub fn new(uri: &str) -> Result<CopilotJson> {
        let trimed = uri.trim();
        if let Some(code_str) = trimed.strip_prefix("maa://") {
            // just check if it's a number
            if code_str.parse::<i64>().is_ok() {
                return Ok(CopilotJson::Code(code_str));
            } else {
                bail!("Invalid code: {}", code_str);
            }
        } else {
            Ok(CopilotJson::File(Path::new(trimed)))
        }
    }

    pub fn get_json_and_file(&self, dir: impl AsRef<Path>) -> Result<(JsonValue, PathBuf)> {
        match self {
            CopilotJson::Code(code) => {
                let json_file = dir.as_ref().join(code).with_extension("json");

                if json_file.is_file() {
                    debug!("Found cached json file:", json_file.display());
                    return Ok((json_from_file(&json_file)?, json_file));
                }

                let url = format!("{}{}", MAA_COPILOT_API, code);
                debug!("Cache miss, downloading from", url);
                let resp: JsonValue = reqwest::blocking::get(url)
                    .context("Failed to send request")?
                    .json()
                    .context("Failed to parse response")?;

                if resp["status_code"].as_i64().unwrap() == 200 {
                    let context = resp["data"]["content"]
                        .as_str()
                        .context("Failed to get copilot context")?;
                    let value: JsonValue =
                        serde_json::from_str(context).context("Failed to parse context")?;

                    // Save json file
                    fs::File::create(&json_file)
                        .context("Failed to create json file")?
                        .write_all(context.as_bytes())
                        .context("Failed to write json file")?;

                    Ok((value, json_file))
                } else {
                    bail!("Request Error, code: {}", code);
                }
            }
            CopilotJson::File(file) => {
                if file.is_absolute() {
                    Ok((json_from_file(file)?, file.to_path_buf()))
                } else {
                    let path = dirs::copilot().join(file);
                    Ok((json_from_file(&path)?, path))
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CopilotType {
    Copilot,
    SSSCopilot,
}

impl CopilotType {
    pub fn get_stage_name(self, base_dirs: &Vec<PathBuf>, stage_id: &str) -> Result<String> {
        match self {
            CopilotType::Copilot => {
                let stage_files = dirs::global_find(base_dirs, |dir| {
                    let dir = dir.join("Arknights-Tile-Pos");
                    debug!("Searching in", dir.display());
                    fs::read_dir(dir)
                        .map(|entries| {
                            entries
                                .filter_map(|entry| entry.map(|e| e.path()).ok())
                                .find(|file_path| {
                                    file_path.file_name().map_or(false, |file_name| {
                                        file_name.to_str().map_or(false, |file_name| {
                                            file_name.starts_with(stage_id)
                                                && file_name.ends_with("json")
                                        })
                                    })
                                })
                        })
                        .unwrap_or(None)
                });

                if let Some(stage_file) = stage_files.last() {
                    let stage_info = json_from_file(stage_file)?;
                    Ok(format!(
                        "{} {}",
                        get_str_key(&stage_info, "code")?,
                        get_str_key(&stage_info, "name")?
                    ))
                } else {
                    warning!("Failed to find stage file, maybe you resouces are outdated?");
                    Ok(stage_id.to_string())
                }
            }
            CopilotType::SSSCopilot => Ok(stage_id.to_string()),
        }
    }

    pub fn to_task(self, filename: impl AsRef<str>) -> Task {
        match self {
            CopilotType::Copilot => Task::new_with_default(
                MAATask::Copilot,
                object!(
                    "filename" => filename.as_ref(),
                    "formation" => BoolInput::new(Some(true), Some("auto formation"))
                ),
            ),
            CopilotType::SSSCopilot => Task::new_with_default(
                MAATask::SSSCopilot,
                object!(
                    "filename" => filename.as_ref(),
                    "loop_times" => Input::<i64>::new(Some(1), Some("loop times"))
                ),
            ),
        }
    }
}

impl AsRef<str> for CopilotType {
    fn as_ref(&self) -> &str {
        match self {
            CopilotType::Copilot => "Copilot",
            CopilotType::SSSCopilot => "SSSCopilot",
        }
    }
}

fn json_from_file(path: impl AsRef<Path>) -> Result<JsonValue> {
    Ok(serde_json::from_reader(fs::File::open(path)?)?)
}

fn operator_table(value: &JsonValue) -> Result<Table> {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["NAME", "SKILL"]);

    if let Some(opers) = value["opers"].as_array() {
        for operator in opers {
            table.add_row(row![get_str_key(operator, "name")?, operator["skill"]]);
        }
    }

    if let Some(groups) = value["groups"].as_array() {
        for group in groups.iter() {
            let opers = group["opers"].as_array().context("Failed to get opers")?;
            let mut sub_table = Table::new();
            sub_table.set_format(*format::consts::FORMAT_NO_LINESEP);
            for operator in opers {
                sub_table.add_row(row![get_str_key(operator, "name")?, operator["skill"]]);
            }

            let vertical_offset = (sub_table.len() + 2) >> 1;

            table.add_row(row![
                format!(
                    "{}[{}]",
                    "\n".repeat(vertical_offset - 1),
                    get_str_key(group, "name")?
                ),
                sub_table
            ]);
        }
    }

    Ok(table)
}

fn get_str_key(value: &JsonValue, key: impl AsRef<str>) -> Result<&str> {
    let key = key.as_ref();
    value[key]
        .as_str()
        .with_context(|| format!("Failed to get {}", key))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env::temp_dir;

    mod copilot_json {
        use super::*;

        #[test]
        fn new() {
            assert_eq!(
                CopilotJson::new("maa://123").unwrap(),
                CopilotJson::Code("123")
            );
            assert_eq!(
                CopilotJson::new("maa://123 ").unwrap(),
                CopilotJson::Code("123")
            );
            assert!(CopilotJson::new("maa:// 123").is_err());

            assert_eq!(
                CopilotJson::new("file.json").unwrap(),
                CopilotJson::File(Path::new("file.json"))
            );
        }

        #[test]
        fn get_json_and_file() {
            let test_root = temp_dir().join("maa-test-get-json-and-file");
            fs::create_dir_all(&test_root).unwrap();

            let test_file = test_root.join("123.json");
            fs::File::create(&test_file)
                .unwrap()
                .write_all(b"{\"type\":\"SSS\"}")
                .unwrap();

            // Remote file but cache hit
            assert_eq!(
                CopilotJson::new("maa://123")
                    .unwrap()
                    .get_json_and_file(&test_root)
                    .unwrap(),
                (serde_json::json!({"type": "SSS"}), test_file.clone())
            );

            // Local file
            assert_eq!(
                CopilotJson::new(test_file.to_str().unwrap())
                    .unwrap()
                    .get_json_and_file(&test_root)
                    .unwrap(),
                (serde_json::json!({"type": "SSS"}), test_file.clone())
            );

            fs::remove_dir_all(&test_root).unwrap();
        }
    }

    mod copilot_type {
        use super::*;

        #[test]
        fn get_stage_name() {
            let test_root = temp_dir().join("maa-test-get-stage-name");
            let arknights_tile_pos = test_root.join("Arknights-Tile-Pos");
            arknights_tile_pos.ensure().unwrap();

            let stage_id = "act30side_01";

            let test_file = arknights_tile_pos
                .join("act30side_01-activities-act30side-level_act30side_01.json");

            fs::File::create(test_file)
                .unwrap()
                .write_all(r#"{ "code": "RS-1", "name": "注意事项" }"#.as_bytes())
                .unwrap();

            assert_eq!(
                CopilotType::Copilot
                    .get_stage_name(&vec![test_root.clone()], stage_id)
                    .unwrap(),
                "RS-1 注意事项"
            );

            assert_eq!(
                CopilotType::Copilot
                    .get_stage_name(&vec![test_root.clone()], "act30side_02")
                    .unwrap(),
                "act30side_02"
            );

            fs::remove_dir_all(&test_root).unwrap();
        }

        #[test]
        fn to_task() {
            assert_eq!(
                CopilotType::Copilot.to_task("filename"),
                Task::new_with_default(
                    MAATask::Copilot,
                    object!(
                        "filename" => "filename",
                        "formation" => BoolInput::new(Some(true), Some("auto formation"))
                    )
                )
            );

            assert_eq!(
                CopilotType::SSSCopilot.to_task("filename"),
                Task::new_with_default(
                    MAATask::SSSCopilot,
                    object!(
                        "filename" => "filename",
                        "loop_times" => Input::<i64>::new(Some(1), Some("loop times"))
                    )
                )
            );
        }
    }

    #[test]
    fn gen_operator_table() {
        let json = serde_json::json!({
            "groups": [
              {
                "name": "行医",
                "opers": [
                  {
                    "name": "纯烬艾雅法拉",
                    "skill": 1,
                    "skill_usage": 0
                  },
                  {
                    "name": "蜜莓",
                    "skill": 1,
                    "skill_usage": 0
                  }
                ]
              }
            ],
            "opers": [
              {
                "name": "桃金娘",
                "skill": 1,
                "skill_usage": 1
              },
              {
                "name": "夜莺",
                "skill": 3,
                "skill_usage": 0
              }
            ]
        });

        let mut expected_table = Table::new();
        expected_table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        expected_table.set_titles(row!["NAME", "SKILL"]);
        expected_table.add_row(row!["桃金娘", 1]);
        expected_table.add_row(row!["夜莺", 3]);

        let mut sub_table = Table::new();
        sub_table.set_format(*format::consts::FORMAT_NO_LINESEP);
        sub_table.add_row(row!["纯烬艾雅法拉", 1]);
        sub_table.add_row(row!["蜜莓", 1]);
        expected_table.add_row(row!["\n[行医]", sub_table]);

        assert_eq!(operator_table(&json).unwrap(), expected_table);
    }
}
