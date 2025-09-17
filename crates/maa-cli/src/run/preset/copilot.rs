use std::{
    borrow::Cow,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use log::{debug, trace};
use maa_sys::TaskType;
use prettytable::{Table, format, row};
use serde_json::Value as JsonValue;

use super::{FindFileOrDefault, IntoTaskConfig, ToTaskType};
use crate::{
    config::task::{Task, TaskConfig},
    dirs::{self, Ensure},
    object,
    value::{
        MAAValue,
        userinput::{BoolInput, UserInput},
    },
};

#[cfg_attr(test, derive(Default))]
#[derive(clap::Args)]
pub struct CopilotParams {
    /// URI of the copilot task file
    ///
    /// It can be a maa URI or a local file path. Multiple URIs can be provided to fight multiple
    /// stages. For URI, it can be in the format of `maa://<code>`, `maa://<code>s`, `file://<path>`,
    /// which represents a single copilot task, a copilot task set, and a local file respectively.
    uri_list: Vec<String>,
    /// Whether to fight stage in raid mode
    ///
    /// 0 for normal, 1 for raid, 2 run twice for both normal and raid
    #[arg(long, default_value = "0")]
    raid: u8,
    /// Whether to auto formation
    ///
    /// When multiple uri are provided or a copilot task set contains multiple stages, force to
    /// true. Otherwise, default to false.
    #[arg(long)]
    formation: bool,
    /// Whether to use sanity potion to restore sanity when it's not enough
    ///
    /// When multiple uri are provided or the uri is a copilot task set, default to false.
    /// Otherwise, default to true.
    #[arg(long)]
    use_sanity_potion: bool,
    /// Whether to navigate to the stage [Deprecated, also navigate to the stage]
    ///
    /// When multiple uri are provided or the uri is a copilot task set, force to true.
    /// Otherwise, default to false.
    #[arg(long)]
    need_navigate: bool,
    /// Whether to add operators to empty slots in the formation to earn trust
    #[arg(long)]
    add_trust: bool,

    /// Deprecated, use `formation_index` instead
    #[arg(long)]
    select_formation: Option<i32>,

    /// Select which formation to use [1-4]
    ///
    /// If not provided, use the current formation
    #[arg(long)]
    formation_index: Option<i32>,

    /// Use given support unit name, don't use support unit if not provided
    #[arg(long)]
    support_unit_name: Option<String>,
}

#[derive(Debug)]
struct StageOpts {
    filename: PathBuf,
    stage_name: String,
    is_raid: bool,
}

impl From<StageOpts> for MAAValue {
    fn from(opts: StageOpts) -> Self {
        object!(
            "filename" => opts.filename.to_string_lossy().to_string(),
            "stage_name" => opts.stage_name,
            "is_raid" => opts.is_raid,
        )
    }
}

impl IntoTaskConfig for CopilotParams {
    fn into_task_config(self, config: &super::AsstConfig) -> Result<TaskConfig> {
        let copilot_dir = dirs::copilot().ensure()?;
        let base_dirs = config.resource.base_dirs();
        let default = MAAValue::find_file_or_default(super::default_file(TaskType::Copilot))
            .context("Failed to load default copilot task config")?;

        let mut copilot_files = Vec::new();
        for uri in &self.uri_list {
            let copilot_file = CopilotFile::from_uri(uri)?;

            copilot_file.push_path_to(&mut copilot_files, copilot_dir)?;
        }

        let is_task_list = copilot_files.len() > 1;
        let formation = self.formation || is_task_list || default.get_or("formation", false);

        let need_navigate = self.need_navigate || default.get_or("need_navigate", false);
        if need_navigate {
            log::warn!("`need_navigate` is deprecated and no longer required");
        }

        let use_sanity_potion =
            self.use_sanity_potion || default.get_or("use_sanity_potion", false);
        let add_trust = self.add_trust || default.get_or("add_trust", false);

        let select_formation = self
            .select_formation
            .or_else(|| default.get_typed("select_formation"));

        let formation_index = self
            .formation_index
            .or_else(|| default.get_typed("formation_index"));

        let formation_index = match (formation_index, select_formation) {
            (Some(formation_index), None) => formation_index,
            (formation_index, Some(select_formation)) => {
                log::warn!(
                    "`select_formation` is deprecated, please use `formation_index` instead"
                );
                if let Some(index) = formation_index {
                    log::warn!(
                        "Both `formation_index` and `select_formation` are provided, using `formation_index`"
                    );
                    index
                } else {
                    select_formation
                }
            }
            (None, None) => 0,
        };

        let mut stage_list = Vec::new();
        for file in copilot_files {
            let copilot_info = json_from_file(&file)?;
            let stage_id = copilot_info
                .get("stage_name")
                .context("No stage_name")?
                .as_str()
                .context("stage_name is not a string")?;

            let stage_info = get_stage_info(stage_id, base_dirs.iter().map(|dir| dir.as_path()))?;
            let stage_code = get_str_key(&stage_info, "code")?;

            if !formation {
                println!("Operators:\n{}", operator_table(&copilot_info)?);
                println!("Please set up your formation manually");
                while !BoolInput::new(Some(true), Some("continue")).value()? {
                    println!("Please confirm you have set up your formation");
                }
            }

            match self.raid {
                0 | 1 => stage_list.push(StageOpts {
                    filename: file.to_path_buf(),
                    stage_name: stage_code.to_owned(),
                    is_raid: self.raid == 1,
                }),
                2 => {
                    stage_list.push(StageOpts {
                        filename: file.to_path_buf(),
                        stage_name: stage_code.to_owned(),
                        is_raid: false,
                    });
                    stage_list.push(StageOpts {
                        filename: file.to_path_buf(),
                        stage_name: stage_code.to_owned(),
                        is_raid: true,
                    });
                }
                n => bail!("Invalid raid mode {n}, should be 0, 1 or 2"),
            }
        }

        let mut params = object!(
            "formation" => formation,
            "use_sanity_potion" => use_sanity_potion,
            "add_trust" => add_trust,
            "formation_index" => formation_index,
            "copilot_list" => stage_list,
        );
        params.maybe_insert("support_unit_name", self.support_unit_name.clone());

        let mut task_config = TaskConfig::new();

        task_config.push(Task::new(TaskType::Copilot, params));

        Ok(task_config)
    }
}

fn get_stage_info<P, D>(stage_id: &str, base_dirs: D) -> Result<JsonValue>
where
    P: AsRef<Path>,
    D: IntoIterator<Item = P>,
{
    let stage_files = dirs::global_find(base_dirs, |dir| {
        let dir = dir.join("Arknights-Tile-Pos");
        trace!("Searching stage file in {}", dir.display());
        fs::read_dir(dir).ok().and_then(|entries| {
            entries
                .filter_map(|entry| entry.map(|e| e.path()).ok())
                .find(|file_path| {
                    file_path
                        .file_name()
                        .and_then(|file_name| file_name.to_str())
                        .is_some_and(|file_name| {
                            file_name.starts_with(stage_id) && file_name.ends_with("json")
                        })
                })
        })
    });

    if let Some(stage_file) = stage_files.last() {
        json_from_file(stage_file)
    } else {
        bail!("Failed to find Tile-Pos file for {stage_id}, your resources may be outdated");
    }
}

#[derive(clap::Args)]
pub struct SSSCopilotParams {
    uri: String,
    /// Loop times
    #[arg(long, default_value = "1")]
    loop_times: i32,
}

impl ToTaskType for SSSCopilotParams {
    fn to_task_type(&self) -> TaskType {
        TaskType::SSSCopilot
    }
}

impl TryFrom<SSSCopilotParams> for MAAValue {
    type Error = anyhow::Error;

    fn try_from(params: SSSCopilotParams) -> std::result::Result<Self, Self::Error> {
        let copilot_dir = dirs::copilot().ensure()?;

        let copilot_file = CopilotFile::from_uri(&params.uri)?;
        let mut paths = Vec::new();
        copilot_file.push_path_to(&mut paths, copilot_dir)?;

        if paths.len() != 1 {
            bail!("SSS Copilot don't support task set");
        }

        let file = paths[0].as_ref();
        let value = json_from_file(file)?;

        if get_str_key(&value, "type")? != "SSS" {
            bail!("The given copilot file is not a SSS copilot file");
        }

        let stage_name = get_str_key(&value, "stage_name")?;

        println!("Fight Stage: {stage_name}, please navigate to the stage manually");
        while !BoolInput::new(Some(true), Some("continue")).value()? {
            println!("Please confirm you have navigated to the stage");
        }
        println!("Core Operators:\n{}", operator_table(&value)?);
        // TODO: equipment, support unit, toolmans
        if BoolInput::new(Some(false), Some("show doc")).value()? {
            let doc = value
                .get("doc")
                .context("No doc in copilot file")?
                .get("details")
                .context("No details in doc")?
                .as_str()
                .context("Details is not a string")?;
            println!("{doc}");
        }

        while !BoolInput::new(Some(true), Some("continue")).value()? {
            println!("Please confirm you have set up your formation");
        }

        let value = object!(
            "filename" => file.to_str().context("Invalid file path")?,
            "loop_times" => params.loop_times,
        );

        Ok(value)
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum CopilotFile<'a> {
    Remote(i64),
    RemoteSet(i64),
    Local(&'a Path),
}

impl<'a> CopilotFile<'a> {
    fn from_uri(uri: &'a str) -> Result<Self> {
        let trimmed = uri.trim();
        if let Some(code_str) = trimmed.strip_prefix("maa://") {
            if let Some(code_str) = code_str.strip_suffix('s') {
                Ok(CopilotFile::RemoteSet(
                    code_str.parse::<i64>().context("Invalid code")?,
                ))
            } else {
                Ok(CopilotFile::Remote(
                    code_str.parse::<i64>().context("Invalid code")?,
                ))
            }
        // } else if let Some(code) = trimmed.strip_prefix("maas://") {
        //     let code_num = code.parse::<i64>().context("Invalid code")?;
        //     Ok(CopilotFile::RemoteSet(code_num))
        } else if let Some(code) = trimmed.strip_prefix("file://") {
            Ok(CopilotFile::Local(Path::new(code)))
        } else {
            Ok(CopilotFile::Local(Path::new(trimmed)))
        }
    }

    pub fn push_path_to(
        self,
        paths: &mut Vec<Cow<'a, Path>>,
        base_dir: impl AsRef<Path>,
    ) -> Result<()> {
        let base_dir = base_dir.as_ref();
        match self {
            CopilotFile::Remote(code) => {
                let code = code.to_string();
                let json_file = base_dir.join(&code).with_extension("json");

                if json_file.is_file() {
                    debug!("Cache hit, using cached json file {}", json_file.display());
                    paths.push(json_file.into());
                    return Ok(());
                }

                const COPILOT_API: &str = "https://prts.maa.plus/copilot/get/";
                let url = format!("{COPILOT_API}{code}");
                debug!("Cache miss, downloading copilot from {url}");
                let resp: JsonValue = reqwest::blocking::get(url)
                    .context("Failed to send request")?
                    .json()
                    .context("Failed to parse response")?;

                if resp["status_code"].as_i64().unwrap() == 200 {
                    let content = resp
                        .get("data")
                        .context("No data in response")?
                        .get("content")
                        .context("No content in response data")?
                        .as_str()
                        .context("Content is not a string")?;

                    // Save json file
                    fs::File::create(&json_file)
                        .context("Failed to create json file")?
                        .write_all(content.as_bytes())
                        .context("Failed to write json file")?;

                    paths.push(json_file.into());

                    Ok(())
                } else {
                    bail!("Request Error, code: {}", code);
                }
            }
            CopilotFile::RemoteSet(code) => {
                const COPILOT_SET_API: &str = "https://prts.maa.plus/set/get?id=";
                let url = format!("{COPILOT_SET_API}{code}");
                debug!("Get copilot set from {url}");
                let resp: JsonValue = reqwest::blocking::get(url)
                    .context("Failed to send request")?
                    .json()
                    .context("Failed to parse response")?;

                if resp["status_code"].as_i64().unwrap() == 200 {
                    let ids = resp
                        .get("data")
                        .context("No data in response")?
                        .get("copilot_ids")
                        .context("No copilot_ids in response data")?
                        .as_array()
                        .context("Copilot_ids is not an array")?;

                    for id in ids {
                        let id = id.as_i64().context("copilot_id is not an integer")?;
                        CopilotFile::Remote(id).push_path_to(paths, base_dir)?;
                    }

                    Ok(())
                } else {
                    bail!("Request Error, code: {}", code);
                }
            }
            CopilotFile::Local(file) => {
                if file.is_absolute() {
                    paths.push(file.into());
                } else {
                    paths.push(base_dir.join(file).into());
                }
                Ok(())
            }
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
    value
        .get(key)
        .with_context(|| format!("{key} not found in {value}"))?
        .as_str()
        .with_context(|| format!("{key} is not a string in {value}"))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{env::temp_dir, path::PathBuf};

    use super::*;
    use crate::config::asst::AsstConfig;

    fn retry<T>(times: usize, f: impl Fn() -> Result<T>) -> T {
        for i in 0..times {
            match f() {
                Ok(x) => return x,
                Err(e) => {
                    eprintln!("Failed to run test: {e}, retry {i}");
                }
            }
        }

        panic!("Failed to run test after {times} retries");
    }

    fn path_from_cache_dir(path: &str) -> String {
        join!(maa_dirs::cache(), "copilot", path)
            .to_string_lossy()
            .to_string()
    }

    mod copilot_params {
        use super::*;

        #[test]
        #[ignore = "need to installed resources and network"]
        fn into_task_config() {
            if std::env::var_os("SKIP_CORE_TEST").is_some() {
                return; // Skip test if resource is not provided
            }

            let resource_dir = maa_dirs::find_resource().unwrap().into_owned();
            let hot_update_dir = maa_dirs::hot_update().to_owned();
            let resource_dirs = vec![resource_dir, hot_update_dir];

            let mut config = AsstConfig::default();
            config.resource.resource_base_dirs = resource_dirs;

            fn parse<I, T>(args: I, config: &AsstConfig) -> Result<TaskConfig>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                let command = crate::command::parse_from(args).command;
                match command {
                    crate::Command::Copilot { params, .. } => params.into_task_config(config),
                    _ => panic!("Not a Copilot command"),
                }
            }

            use crate::config::task::InitializedTask;
            fn parse_to_taskes<I, T>(args: I, config: &AsstConfig) -> Vec<InitializedTask>
            where
                I: AsRef<[T]>,
                T: Into<std::ffi::OsString> + Clone,
            {
                retry(3, || parse(args.as_ref().iter().cloned(), config))
                    .init()
                    .unwrap()
                    .tasks
            }

            let tasks = parse_to_taskes(["maa", "copilot", "maa://40051"], &config);
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                    )],
                    "formation" => false,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 0,
                ),
            );

            // Test no default values
            let tasks_no_default = parse_to_taskes(
                [
                    "maa",
                    "copilot",
                    "maa://40051",
                    "--raid=1",
                    "--formation",
                    "--use-sanity-potion",
                    "--need-navigate",
                    "--add-trust",
                    "--formation-index",
                    "4",
                    "--support-unit-name",
                    "维什戴尔",
                ],
                &config,
            );
            assert_eq!(tasks_no_default.len(), 1);
            assert_eq!(tasks_no_default[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks_no_default[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => true,
                    )],
                    "formation" => true,
                    "use_sanity_potion" => true,
                    "add_trust" => true,
                    "formation_index" => 4,
                    "support_unit_name" => "维什戴尔",
                ),
            );

            // Test formation index
            let tasks_formation_index = parse_to_taskes(
                [
                    "maa",
                    "copilot",
                    "maa://40051",
                    "--select-formation",
                    "2",
                    "--formation-index",
                    "4",
                ],
                &config,
            );
            assert_eq!(tasks_formation_index.len(), 1);
            assert_eq!(tasks_formation_index[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks_formation_index[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                    )],
                    "formation" => false,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 4,
                ),
            );

            // Test raid mode 2
            let tasks_raid_2 = parse_to_taskes(
                [
                    "maa",
                    "copilot",
                    "maa://40051",
                    "--raid",
                    "2",
                    "--formation",
                ],
                &config,
            );
            assert_eq!(tasks_raid_2.len(), 1);
            assert_eq!(tasks_raid_2[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks_raid_2[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                    ),
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => true,
                    )],
                    "formation" => true,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 0,
                ),
            );

            let tasks_multiple =
                parse_to_taskes(["maa", "copilot", "maa://40051", "maa://40052"], &config);

            assert_eq!(tasks_multiple.len(), 1);
            assert_eq!(tasks_multiple[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks_multiple[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                    ),
                    object!(
                        "filename" => path_from_cache_dir("40052.json"),
                        "stage_name" => "AS-EX-2",
                        "is_raid" => false,
                    )],
                    "formation" => true,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 0,
                ),
            );
        }

        #[test]
        fn get_stage_info_from_id() {
            // We don't use dirs::find_resource() here, because it is unreliable in tests
            // due to some tests may change return value of it.
            let resource_dir = if let Some(resource) = std::env::var_os("MAA_RESOURCE") {
                PathBuf::from(resource)
            } else {
                return; // Skip test if resource is not provided
            };

            let arknights_tile_pos = resource_dir.join("Arknights-Tile-Pos");
            arknights_tile_pos.ensure().unwrap();

            let stage_id = "act35side_ex01";

            let stage_info = get_stage_info(stage_id, std::slice::from_ref(&resource_dir)).unwrap();

            assert_eq!(stage_info["code"], "AS-EX-1");
            assert_eq!(stage_info["name"], "小偷与收款人");
        }
    }

    mod sss_copilot_params {
        use super::*;

        #[test]
        fn to_task_type() {
            let params = SSSCopilotParams {
                uri: "maa://40051".to_string(),
                loop_times: 2,
            };

            assert_eq!(params.to_task_type(), TaskType::SSSCopilot);
        }

        #[test]
        #[ignore = "need downloaded from internet"]
        fn try_into_maa_value() {
            fn parse<I, T>(args: I) -> Result<MAAValue>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                let command = crate::command::parse_from(args).command;
                match command {
                    crate::Command::SSSCopilot { params, .. } => params.try_into(),
                    _ => panic!("Not a SSSCopilot command"),
                }
            }

            assert!(parse(["maa", "ssscopilot", "maa://40051"]).is_err());
            assert_eq!(
                retry(3, || parse(["maa", "ssscopilot", "maa://40451"])),
                object!("filename" => path_from_cache_dir("40451.json"), "loop_times" => 1)
            );
            assert_eq!(
                retry(3, || parse([
                    "maa",
                    "ssscopilot",
                    "maa://40451",
                    "--loop-times",
                    "2"
                ])),
                object!("filename" => path_from_cache_dir("40451.json"), "loop_times" => 2)
            );
        }
    }

    mod copilot_file {
        use super::*;

        #[test]
        fn from_uri() {
            assert!(CopilotFile::from_uri("maa://xyz").is_err());

            assert_eq!(
                CopilotFile::from_uri("maa://20001s").unwrap(),
                CopilotFile::RemoteSet(20001)
            );

            assert_eq!(
                CopilotFile::from_uri("maa://30001").unwrap(),
                CopilotFile::Remote(30001)
            );

            assert_eq!(
                CopilotFile::from_uri("file://file.json").unwrap(),
                CopilotFile::Local(Path::new("file.json"))
            );

            assert_eq!(
                CopilotFile::from_uri("file.json").unwrap(),
                CopilotFile::Local(Path::new("file.json"))
            );
        }

        #[test]
        #[ignore = "need to download from internet"]
        fn push_path_to() {
            let test_root = temp_dir().join("maa-test-push-path-to");
            fs::create_dir_all(&test_root).unwrap();

            let test_file = test_root.join("123234.json");
            let test_content = serde_json::json!({
              "minimum_required": "v4.0.0",
              "stage_name": "act25side_01",
              "actions": [
                { "type": "SpeedUp" },
              ],
              "groups": [],
              "opers": [],
            });

            serde_json::to_writer(fs::File::create(&test_file).unwrap(), &test_content).unwrap();

            // Remote
            assert_eq!(
                retry(3, || {
                    let mut paths = Vec::new();
                    CopilotFile::from_uri("maa://40051")
                        .unwrap()
                        .push_path_to(&mut paths, &test_root)?;
                    Ok(paths)
                }),
                &[test_root.join("40051.json")],
            );

            // RemoteSet
            assert_eq!(
                retry(3, || {
                    let mut paths = Vec::new();
                    CopilotFile::from_uri("maa://23125s")
                        .unwrap()
                        .push_path_to(&mut paths, &test_root)?;
                    Ok(paths)
                }),
                {
                    let ids = [40051, 40052, 40053, 40055, 40056, 40057, 40058, 40059];

                    ids.iter()
                        .map(|id| test_root.join(format!("{id}.json")))
                        .collect::<Vec<PathBuf>>()
                }
            );

            // Local file (absolute)
            assert_eq!(
                {
                    let mut paths = Vec::new();
                    CopilotFile::from_uri(test_file.to_str().unwrap())
                        .unwrap()
                        .push_path_to(&mut paths, &test_root)
                        .unwrap();
                    paths
                },
                &[test_file.as_path()]
            );

            // Local file (relative)
            assert_eq!(
                {
                    let mut paths = Vec::new();
                    CopilotFile::from_uri("file.json")
                        .unwrap()
                        .push_path_to(&mut paths, &test_root)
                        .unwrap();
                    paths
                },
                &[test_root.join("file.json")]
            );

            fs::remove_dir_all(&test_root).unwrap();
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
