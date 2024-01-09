use crate::{
    config::task::{task_type::TaskType, Task, TaskConfig},
    dirs::{self, Ensure},
    object,
    value::{
        userinput::{BoolInput, Input},
        MAAValue,
    },
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
    let stage_name = value
        .get_as_str("stage_name")
        .with_context(lfl!("failed-get-stage-name"))?;
    let stage_name = task_type.get_stage_name(resource_dirs, stage_name)?;

    println!("{}{}", fl!("copilot-stage"), stage_name);
    println!("{}\n{}", fl!("copilot-operators"), operator_table(&value)?);

    // Append task
    let mut task_config = TaskConfig::new();

    task_config.push(task_type.to_task(path.to_str().with_context(lfl!("invalid-utf8-path"))?));

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
                    debug!("copilot-cache-hit", file = json_file.to_string_lossy());
                    return Ok((json_from_file(&json_file)?, json_file));
                }

                let url = format!("{}{}", MAA_COPILOT_API, code);
                let resp: JsonValue = reqwest::blocking::get(&url)
                    .with_context(lfl!("failed-download-copilot", url = url.clone()))?
                    .json()
                    .with_context(lfl!("failed-parse-copilot"))?;
                debug!("copilot-downloaded", url = url.clone());

                if resp["status_code"].as_i64().unwrap() == 200 {
                    let context = resp["data"]["content"]
                        .as_str()
                        .with_context(lfl!("failed-get-copilot-content"))?;
                    let value: JsonValue = serde_json::from_str(context)
                        .with_context(lfl!("failed-parse-copilot-content"))?;

                    // Save json file
                    fs::File::create(&json_file)
                        .with_context(lfl!("failed-open-file", file = json_file.to_string_lossy()))?
                        .write_all(context.as_bytes())
                        .with_context(lfl!(
                            "failed-write-file",
                            file = json_file.to_string_lossy()
                        ))?;

                    Ok((value, json_file))
                } else {
                    bailfl!(
                        "failed-response-status",
                        status = serde_json::to_string(&resp["status_code"])?
                    );
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
                    fs::read_dir(dir).ok().and_then(|entries| {
                        entries
                            .filter_map(|entry| entry.map(|e| e.path()).ok())
                            .find(|file_path| {
                                file_path
                                    .file_name()
                                    .and_then(|file_name| file_name.to_str())
                                    .map_or(false, |file_name| {
                                        file_name.starts_with(stage_id)
                                            && file_name.ends_with("json")
                                    })
                            })
                    })
                });

                stage_files
                    .last()
                    .with_context(lfl!("failed-find-stage-file", stage = stage_id))
                    .and_then(|stage_file| json_from_file(stage_file))
                    .and_then(|stage_info| {
                        match (stage_info.get_as_str("code"), stage_info.get_as_str("name")) {
                            (Some(code), Some(name)) => Ok(format!("{} {}", code, name)),
                            (Some(code), None) => Ok(code.to_string()),
                            (None, Some(name)) => Ok(name.to_string()),
                            (None, None) => Ok(stage_id.to_string()),
                        }
                    })
            }
            CopilotType::SSSCopilot => Ok(stage_id.to_string()),
        }
    }

    pub fn to_task(self, filename: impl AsRef<str>) -> Task {
        match self {
            CopilotType::Copilot => Task::new_with_default(
                TaskType::Copilot,
                object!(
                    "filename" => filename.as_ref(),
                    "formation" => BoolInput::new(Some(true), Some("auto formation"))
                ),
            ),
            CopilotType::SSSCopilot => Task::new_with_default(
                TaskType::SSSCopilot,
                object!(
                    "filename" => filename.as_ref(),
                    "loop_times" => Input::new(Some(1), Some("loop times"))
                ),
            ),
        }
    }

    // fn to_fl_string(&self) -> String {
    //     match self {
    //         CopilotType::Copilot => fl!("Copilot"),
    //         CopilotType::SSSCopilot => fl!("SSSCopilot"),
    //     }
    // }
}

fn json_from_file(path: impl AsRef<Path>) -> Result<JsonValue> {
    let path = path.as_ref();
    let file = fs::File::open(path)
        .with_context(lfl!("failed-read-file", file = path.to_string_lossy()))?;

    serde_json::from_reader(file).with_context(lfl!("failed-deserialize-json"))
}

fn operator_table(value: &JsonValue) -> Result<Table> {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["NAME", "SKILL"]);

    if let Some(opers) = value.get("opers").and_then(|v| v.as_array()) {
        for operator in opers {
            table.add_row(row![
                operator.get_as_str_or("name", "Unknown"),
                operator["skill"]
            ]);
        }
    }

    if let Some(groups) = value.get("groups").and_then(|v| v.as_array()) {
        for group in groups.iter() {
            let opers = if let Some(opers) = group.get("opers").and_then(|v| v.as_array()) {
                opers
            } else {
                continue;
            };
            let mut sub_table = Table::new();
            sub_table.set_format(*format::consts::FORMAT_NO_LINESEP);
            for operator in opers {
                sub_table.add_row(row![
                    operator.get_as_str_or("name", "Unknown"),
                    operator["skill"]
                ]);
            }

            let vertical_offset = (sub_table.len() + 2) >> 1;

            table.add_row(row![
                format!(
                    "{}[{}]",
                    "\n".repeat(vertical_offset - 1),
                    group.get_as_str_or("name", "Unknown")
                ),
                sub_table
            ]);
        }
    }

    Ok(table)
}

trait GetAsStr {
    fn get_as_str(&self, key: impl AsRef<str>) -> Option<&str>;

    fn get_as_str_or<'a>(&'a self, key: impl AsRef<str>, default: &'a str) -> &'a str {
        self.get_as_str(key).unwrap_or(default)
    }
}

impl GetAsStr for JsonValue {
    fn get_as_str(&self, key: impl AsRef<str>) -> Option<&str> {
        self.get(key.as_ref())?.as_str()
    }
}

#[repr(u8)]
#[derive(serde::Serialize, Default)]
enum Diretion {
    #[default]
    Right = 0,
    Down = 1,
    Left = 2,
    Up = 3,
    None = 4, // 没有方向，通常是无人机之类的
}

impl<'de> serde::Deserialize<'de> for Diretion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "Right" | "RIGHT" | "right" | "右" => Ok(Diretion::Right),
            "Left" | "LEFT" | "left" | "左" => Ok(Diretion::Left),
            "Up" | "UP" | "up" | "上" => Ok(Diretion::Up),
            "Down" | "DOWN" | "down" | "下" => Ok(Diretion::Down),
            "None" | "NONE" | "none" | "无" => Ok(Diretion::None),
            s => Err(serde::de::Error::custom(format!(
                "Invalid direction: {}",
                s
            ))),
        }
    }
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
                    TaskType::Copilot,
                    object!(
                        "filename" => "filename",
                        "formation" => BoolInput::new(Some(true), Some("auto formation"))
                    )
                )
            );

            assert_eq!(
                CopilotType::SSSCopilot.to_task("filename"),
                Task::new_with_default(
                    TaskType::SSSCopilot,
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
