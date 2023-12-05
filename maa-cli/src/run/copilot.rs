use super::run;
use crate::{
    config::task::{
        default_variants, task_type::TaskType, value::input::Input, Strategy, Task, TaskList, Value,
    },
    debug,
    dirs::Dirs,
    installer::maa_core::find_resource,
    normal, object, warning,
};
use anyhow::Result;
use anyhow::{anyhow, Error};
use anyhow::{Context, Ok};
use clipboard::{ClipboardContext, ClipboardProvider};
use prettytable::{format, row, Table};
use reqwest::blocking::Client;
use serde_json::{from_str, Value as JsonValue};
use std::{
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

pub fn copilot(
    dirs: &Dirs,
    uri_opt: Option<String>,
    paste: bool,
    addr: Option<String>,
    user_resource: bool,
    batch: bool,
) -> Result<()> {
    let uri = if paste {
        clipboard_reader()?
    } else {
        uri_opt.ok_or_else(|| anyhow!("No input"))?
    };
    debug!("uri: ", uri);
    let jpr = "JSON Prase Error";
    let results = json_reader(&uri, dirs)?;
    let value = results.0;

    // Determine type of stage
    let task_type = match value["type"].as_str() {
        Some("SSS") => "SSSCopilot",
        _ => "Copilot",
    };

    // Print stage info
    let mut stage_dir = find_resource(dirs).context("Failed to find resource!")?;
    stage_dir.push("Arknights-Tile-Pos");
    if task_type == "Copilot" {
        let stage_code_name = value["stage_name"].as_str().context(jpr)?;
        let result = find_json(stage_code_name, stage_dir).unwrap_or_else(|_| {
            warning!(
                "Unable to find target map. This may be because your Maacore version is too old."
            );
            let json_string = r#"{ "code" : " ", "name" : "Unknown" }"#;
            from_str(json_string).unwrap()
        });
        let stage_name = format!(
            "{} {}",
            result["code"].as_str().context(jpr)?,
            result["name"].as_str().context(jpr)?
        );
        normal!("Stage info:\n", stage_name);
    } else {
        let stage_name = value["stage_name"].as_str().context(jpr)?;
        normal!("Stage info:\n", stage_name);
    }

    // Print operators info
    let opers = value["opers"].as_array().context(jpr)?;
    let groups = value["groups"].as_array().context(jpr)?;

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["NAME", "SKILL"]);
    for operator in opers {
        let name = operator["name"].as_str().context(jpr)?;
        let skill = operator["skill"].as_u64().context(jpr)?;
        table.add_row(row![name, skill]);
    }

    for group in groups {
        let name: String = "[".to_string() + group["name"].as_str().context(jpr)? + "]";
        let skill = "X";
        table.add_row(row![name, skill]);
    }
    normal!("Operator lists:\n", table.to_string());

    // Get input of user
    // Note: Waiting for the new input type of bool to be completed
    let formation: Input<bool> =
        Input::new(Some(true), Some("whether to quick build(true/false):"));
    let loop_times: Input<i64> = Input::new(Some(1), Some("loop times:"));

    // Append task
    let mut task_list = Vec::new();
    let json_path_str = results.1.display().to_string();
    if task_type == "Copilot" {
        task_list.push(Task::new(
            TaskType::Copilot,
            object!(
                "filename" => json_path_str,
                "formation" => formation,
            ),
            Strategy::default(),
            default_variants(),
        ));
    } else {
        task_list.push(Task::new(
            TaskType::SSSCopilot,
            object!(
                "filename" => json_path_str,
                "loop_times" => loop_times,
            ),
            Strategy::default(),
            default_variants(),
        ));
    };

    let task = TaskList { tasks: task_list };
    run(dirs, task, addr, user_resource, batch, false)
}

fn json_reader(uri: &String, dirs: &Dirs) -> Result<(JsonValue, PathBuf)> {
    let api = "https://prts.maa.plus/copilot/get/";
    let jpr = "JSON Prase Error";
    let cache_dir = dirs.cache().display().to_string();

    let uri_ = {
        let trimed = uri.trim();
        if trimed.starts_with("maa://") && trimed.parse::<f64>().is_err() {
            (&uri[uri.len() - 5..], false)
        } else if Path::new(trimed).is_file() {
            (trimed, true)
        } else {
            return Err(anyhow!("Code Invalid"));
        }
    };

    if !uri_.1 {
        // Load via server's API.

        // Cache decision
        match find_json(uri_.0, PathBuf::from(&cache_dir)) {
            Result::Ok(value) => {
                let json_path = Path::new(&cache_dir).join(uri_.0).with_extension("json");
                return Ok((value, json_path));
            }
            Err(_error) => {
                debug!("Cache miss")
            }
        };

        let url = api.to_owned() + uri_.0;

        let client = Client::new();
        let response = client.get(url).send().context("Request Error")?;
        let json: JsonValue = response.json().context(jpr)?;
        let status_code = json["status_code"].clone().to_string();
        if status_code == "200" {
            let context = json["data"]["content"].as_str().context(jpr)?;
            let value: JsonValue = serde_json::from_str(context).context(jpr)?;

            // Save json file
            let json_path = Path::new(&cache_dir).join(uri_.0).with_extension("json");
            let mut file = File::create(json_path.clone())?;
            file.write_all(context.as_bytes())?;

            Ok((value, json_path))
        } else {
            Err(anyhow!("Request Error"))
        }
    } else {
        // Load via file.
        let content = fs::read_to_string(uri_.0)?;
        let json: JsonValue = serde_json::from_str(&content)?;
        Ok((json, PathBuf::from(uri_.0)))
    }
}

fn find_json(json_file_name: &str, mut dir_path: PathBuf) -> Result<JsonValue> {
    let json_file_name = fs::read_dir(dir_path.clone())
        .map_err(Error::msg)?
        .filter_map(|entry| {
            entry
                .ok()
                .and_then(|e| e.file_name().to_str().map(String::from))
        })
        .find(|file_name| file_name.starts_with(json_file_name))
        .ok_or(anyhow!("File not found"))?;

    dir_path.push(json_file_name);

    let file_result = File::open(dir_path).map_err(Error::msg)?;
    let reader = BufReader::new(file_result);
    let json_value: JsonValue = serde_json::from_reader(reader).map_err(Error::msg)?;

    Ok(json_value)
}

fn clipboard_reader() -> Result<String> {
    let mut ctx: ClipboardContext = ClipboardProvider::new().map_err(|err| anyhow!("{}", err))?;
    let content = ctx.get_contents().map_err(|err| anyhow!("{}", err))?;

    Ok(content)
}
