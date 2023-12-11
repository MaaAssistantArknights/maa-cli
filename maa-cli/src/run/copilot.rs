use crate::{
    config::task::{
        task_type::MAATask,
        value::input::{BoolInput, Input},
        Task, TaskConfig, Value,
    },
    debug,
    dirs::{self, Ensure},
    info, object, warning,
};

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use prettytable::{format, row, Table};
use serde_json::Value as JsonValue;

const MAA_COPILOT_API: &str = "https://prts.maa.plus/copilot/get/";

pub fn copilot(uri: impl AsRef<str>, resource_dirs: &Vec<PathBuf>) -> Result<TaskConfig> {
    let (value, path) = CopilotJson::new(uri.as_ref())?.get_json_and_file()?;

    // Determine type of stage
    let task_type = match value["type"].as_str() {
        Some("SSS") => CopilotType::SSSCopilot,
        _ => CopilotType::Copilot,
    };

    // Print stage info
    let stage_id = value["stage_name"].as_str().unwrap();
    let stage_name = task_type.get_stage_name(resource_dirs, stage_id)?;

    info!("Stage:", stage_name);

    // Print operators info
    info!("Operators:\n", {
        let opers = value["opers"].as_array().unwrap();
        let groups = value["groups"].as_array().unwrap();

        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        table.set_titles(row!["NAME", "SKILL"]);
        for operator in opers {
            let name = operator["name"].as_str().unwrap();
            let skill = operator["skill"].as_u64().unwrap();
            table.add_row(row![name, skill]);
        }

        for group in groups {
            table.add_row(row![format!("[{}]", group["name"].as_str().unwrap()), "X"]);
        }
        table
    });

    // Append task
    let mut task_config = TaskConfig::new();

    task_config.push(task_type.to_task(path.to_str().unwrap()));

    Ok(task_config)
}

enum CopilotJson<'a> {
    URL(&'a str),
    File(&'a Path),
}

impl CopilotJson<'_> {
    pub fn new(uri: &str) -> Result<CopilotJson> {
        let trimed = uri.trim();
        if let Some(code_str) = trimed.strip_prefix("maa://") {
            // just check if it's a number
            if code_str.parse::<i64>().is_ok() {
                return Ok(CopilotJson::URL(code_str));
            } else {
                bail!("Invalid code: {}", code_str);
            }
        } else {
            Ok(CopilotJson::File(Path::new(trimed)))
        }
    }

    pub fn get_json_and_file(&self) -> Result<(JsonValue, PathBuf)> {
        match self {
            CopilotJson::URL(code) => {
                let json_file = dirs::copilot().ensure()?.join(code).with_extension("json");

                if json_file.is_file() {
                    debug!("Found cached json file:", json_file.display());
                    return Ok((copilot_json_from_file(&json_file)?, json_file));
                }

                let url = format!("{}{}", MAA_COPILOT_API, code);
                debug!("Cache miss, downloading from", url);
                let resp: JsonValue = reqwest::blocking::get(url)
                    .context("Failed to send request")?
                    .json()
                    .context("Failed to parse response")?;

                if resp["status_code"].as_i64().unwrap() == 200 {
                    let context = resp["data"]["content"].as_str().unwrap();
                    let value: JsonValue =
                        serde_json::from_str(context).context("Failed to parse context")?;

                    // Save json file
                    let mut file = File::create(&json_file).with_context(|| {
                        format!("Failed to create json file: {}", json_file.display())
                    })?;

                    file.write_all(context.as_bytes())
                        .context("Failed to write json file")?;

                    Ok((value, json_file))
                } else {
                    bail!("Request Error, code: {}", code);
                }
            }
            CopilotJson::File(file) => {
                if file.is_absolute() {
                    Ok((copilot_json_from_file(file)?, file.to_path_buf()))
                } else {
                    let path = dirs::copilot().join(file);
                    Ok((copilot_json_from_file(&path)?, path))
                }
            }
        }
    }
}

fn copilot_json_from_file(path: impl AsRef<Path>) -> Result<JsonValue> {
    Ok(serde_json::from_reader(File::open(path)?)?)
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
                    let stage_info: JsonValue = serde_json::from_reader(File::open(stage_file)?)?;
                    Ok(format!("{} {}", stage_info["code"], stage_info["name"]))
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
                    "formation" => BoolInput::new(Some(true), Some("self-formation?"))
                ),
            ),
            CopilotType::SSSCopilot => Task::new_with_default(
                MAATask::SSSCopilot,
                object!(
                    "filename" => filename.as_ref(),
                    "loop_times" => Input::<i64>::new(Some(1), Some("loop times:"))
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
