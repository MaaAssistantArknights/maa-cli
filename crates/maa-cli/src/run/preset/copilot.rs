use std::{
    borrow::Cow,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
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
    state::AGENT,
    value::{
        MAAValue,
        userinput::{BoolInput, UserInput},
    },
};

#[cfg(not(test))]
const COPILOT_API: &str = "https://prts.maa.plus/copilot/get/";
#[cfg(not(test))]
const COPILOT_SET_API: &str = "https://prts.maa.plus/set/get?id=";

#[cfg(test)]
const COPILOT_API: &str = "http://127.0.0.1:18080/copilot/get/";
#[cfg(test)]
const COPILOT_SET_API: &str = "http://127.0.0.1:18080/set/get?id=";

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
    is_paradox: bool,
}

impl From<StageOpts> for MAAValue {
    fn from(opts: StageOpts) -> Self {
        object!(
            "filename" => opts.filename.to_string_lossy().to_string(),
            "stage_name" => opts.stage_name,
            "is_raid" => opts.is_raid,
            "is_paradox" => opts.is_paradox,
        )
    }
}

impl IntoTaskConfig for CopilotParams {
    fn into_task_config(self, config: &super::AsstConfig) -> Result<TaskConfig> {
        let copilot_dir = dirs::copilot().ensure()?;
        let base_dirs = config.resource.base_dirs();
        let default = MAAValue::find_file_or_default(super::default_file(TaskType::Copilot))
            .context("Failed to load default copilot task config")?;

        // Pre-allocate with uri_list capacity (may expand if RemoteSet is used)
        let mut copilot_files = Vec::with_capacity(self.uri_list.len());
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

        // Pre-allocate for common case (may need more if raid == 2)
        let mut stage_list = Vec::with_capacity(copilot_files.len());
        for (file, cached_json) in copilot_files {
            let copilot_info = match cached_json {
                Some(json) => json,
                None => json_from_file(&file)?,
            };
            let stage_id = copilot_info
                .get("stage_name")
                .with_context(|| {
                    format!("Missing 'stage_name' in copilot file: {}", file.display())
                })?
                .as_str()
                .with_context(|| {
                    format!(
                        "'stage_name' is not a string in copilot file: {}",
                        file.display()
                    )
                })?;

            let is_paradox = stage_id.starts_with("mem_");

            let stage_info = get_stage_info(stage_id, base_dirs.iter().map(|dir| dir.as_path()))?;
            let stage_code = get_str_key(&stage_info, "code")?;

            if !(is_paradox || formation) {
                println!("Operators:\n{}", operator_table(&copilot_info)?);
                println!("Please set up your formation manually");
                prompt_continue("Please confirm you have set up your formation")?;
            }

            let mut raid = self.raid;
            if is_paradox && raid != 0 {
                log::warn!(
                    "Paradox simulation is not supported in raid mode, force raid mode to 0"
                );
                raid = 0;
            }

            match raid {
                0 | 1 => stage_list.push(StageOpts {
                    filename: file.to_path_buf(),
                    stage_name: stage_code.to_owned(),
                    is_raid: raid == 1,
                    is_paradox,
                }),
                2 => {
                    stage_list.push(StageOpts {
                        filename: file.to_path_buf(),
                        stage_name: stage_code.to_owned(),
                        is_raid: false,
                        is_paradox,
                    });
                    stage_list.push(StageOpts {
                        filename: file.to_path_buf(),
                        stage_name: stage_code.to_owned(),
                        is_raid: true,
                        is_paradox,
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

        let (file, cached_json) = &paths[0];
        let value = match cached_json {
            Some(json) => json.clone(),
            None => json_from_file(file)?,
        };

        if get_str_key(&value, "type")? != "SSS" {
            bail!("The given copilot file is not a SSS copilot file");
        }

        let stage_name = get_str_key(&value, "stage_name")?;

        println!("Fight Stage: {stage_name}, please navigate to the stage manually");
        prompt_continue("Please confirm you have navigated to the stage")?;
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

        prompt_continue("Please confirm you have set up your formation")?;

        let value = object!(
            "filename" => file.to_str().context("Invalid file path")?,
            "loop_times" => params.loop_times,
        );

        Ok(value)
    }
}

/// Prompt user to continue with a confirmation message
fn prompt_continue(message: &str) -> Result<()> {
    while !BoolInput::new(Some(true), Some("continue")).value()? {
        println!("{}", message);
    }
    Ok(())
}

/// Download result containing path and optionally parsed JSON
type DownloadResult = Result<(PathBuf, Option<JsonValue>)>;

/// Download a single copilot file by code, returns (path, parsed_json)
fn download_copilot(code: i64, base_dir: &Path) -> DownloadResult {
    let code_str = code.to_string();
    let json_file = base_dir.join(&code_str).with_extension("json");

    // Check cache first
    if json_file.is_file() {
        debug!("Cache hit, using cached json file {}", json_file.display());
        return Ok((json_file, None));
    }

    let url = format!("{COPILOT_API}{code_str}");
    debug!("Cache miss, downloading copilot from {url}");
    let resp = fetch_api_response(&url)?;

    let content = resp
        .get("data")
        .context("No data in response")?
        .get("content")
        .context("No content in response data")?
        .as_str()
        .context("Content is not a string")?;

    // Parse JSON from content
    let json_value: JsonValue = serde_json::from_str(content)
        .with_context(|| format!("Failed to parse copilot JSON content for {}", code))?;

    // Save json file for FFI usage
    let mut file = fs::File::create(&json_file).context("Failed to create json file")?;
    file.write_all(content.as_bytes())
        .context("Failed to write json file")?;
    file.sync_all()
        .context("Failed to sync json file to disk")?;

    Ok((json_file, Some(json_value)))
}

/// Fetch JSON from API with status code validation
fn fetch_api_response(url: &str) -> Result<JsonValue> {
    debug!("Fetching from {url}");
    let mut response = AGENT
        .get(url)
        .call()
        .with_context(|| format!("Failed to send request to {url}"))?;

    let resp: JsonValue = response
        .body_mut()
        .read_json()
        .with_context(|| {
            format!("Failed to parse JSON response from {url}. The server may have disconnected or returned invalid data")
        })?;

    let status_code = resp["status_code"]
        .as_i64()
        .with_context(|| format!("Invalid status_code in response from {url}"))?;

    if status_code == 200 {
        Ok(resp)
    } else {
        bail!("Request failed with status code {}: {}", status_code, url);
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
        paths: &mut Vec<(Cow<'a, Path>, Option<JsonValue>)>,
        base_dir: impl AsRef<Path>,
    ) -> Result<()> {
        let base_dir = base_dir.as_ref();
        match self {
            CopilotFile::Remote(code) => {
                let (path, json) = download_copilot(code, base_dir)?;
                paths.push((path.into(), json));
                Ok(())
            }
            CopilotFile::RemoteSet(code) => {
                let url = format!("{COPILOT_SET_API}{code}");
                let resp = fetch_api_response(&url)?;

                let ids: Vec<i64> = resp
                    .get("data")
                    .context("No data in response")?
                    .get("copilot_ids")
                    .context("No copilot_ids in response data")?
                    .as_array()
                    .context("Copilot_ids is not an array")?
                    .iter()
                    .map(|id| id.as_i64().context("copilot_id is not an integer"))
                    .collect::<Result<_>>()?;

                // Download in parallel using threads, preserving order
                let base_dir = base_dir.to_path_buf();
                let (tx, rx) = mpsc::channel();

                // Spawn download threads
                let handles: Vec<_> = ids
                    .into_iter()
                    .enumerate()
                    .map(|(index, id)| {
                        let tx = tx.clone();
                        let base_dir = base_dir.clone();
                        thread::spawn(move || {
                            let result = download_copilot(id, &base_dir);
                            // Send index along with result to preserve order
                            let _ = tx.send((index, result));
                        })
                    })
                    .collect();

                // Drop the original sender so rx.iter() will end
                drop(tx);

                // Collect results
                let mut results: Vec<(usize, DownloadResult)> = rx.iter().collect();

                // Wait for all threads to complete
                for handle in handles {
                    let _ = handle.join();
                }

                // Sort by original index to preserve order
                results.sort_by_key(|(index, _)| *index);

                // Process results in order
                for (_, result) in results {
                    let (path, json) = result?;
                    paths.push((path.into(), json));
                }

                Ok(())
            }
            CopilotFile::Local(file) => {
                if file.is_absolute() {
                    paths.push((file.into(), None));
                } else {
                    paths.push((base_dir.join(file).into(), None));
                }
                Ok(())
            }
        }
    }
}

fn json_from_file(path: impl AsRef<Path>) -> Result<JsonValue> {
    let path = path.as_ref();
    let r: Result<JsonValue, _> = serde_json::from_reader(fs::File::open(path)?);
    r.with_context(|| format!("Failed to parse JSON file {}", path.display(),))
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
        .with_context(|| format!("Missing required field '{key}' in JSON: {value}"))?
        .as_str()
        .with_context(|| format!("Field '{key}' must be a string in JSON: {value}"))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{env::temp_dir, fs, path::PathBuf, sync::OnceLock, thread};

    use super::*;
    use crate::config::asst::AsstConfig;

    const TEST_SERVER_PORT: u16 = 18080;

    static SERVER_AVAILABLE: OnceLock<bool> = OnceLock::new();

    /// Ensures the test HTTP server is started.
    /// Returns true if server is available, false if binding failed.
    fn ensure_test_server() -> bool {
        *SERVER_AVAILABLE.get_or_init(|| {
            const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
            let fixtures_dir = PathBuf::from(MANIFEST_DIR).join("fixtures").join("copilot");

            // Start HTTP server in a background thread
            thread::spawn(move || {
                let server = match tiny_http::Server::http(("127.0.0.1", TEST_SERVER_PORT)) {
                    Ok(s) => s,
                    Err(_) => {
                        // Failed to bind - server unavailable
                        return;
                    }
                };

                for request in server.incoming_requests() {
                    let url = request.url();

                    // Handle /copilot/get/{task_id}
                    if let Some(task_id) = url.strip_prefix("/copilot/get/") {
                        let file_path = fixtures_dir.join("tasks").join(format!("{task_id}.json"));

                        if let Ok(content) = fs::read_to_string(&file_path) {
                            let response_body = serde_json::json!({
                                "status_code": 200,
                                "data": {
                                    "content": content
                                }
                            });

                            let response =
                                tiny_http::Response::from_string(response_body.to_string())
                                    .with_header(
                                        tiny_http::Header::from_bytes(
                                            &b"Content-Type"[..],
                                            &b"application/json"[..],
                                        )
                                        .unwrap(),
                                    );
                            let _ = request.respond(response);
                        } else {
                            let response = tiny_http::Response::from_string(
                                r#"{"status_code": 404, "message": "Task not found"}"#,
                            )
                            .with_status_code(404);
                            let _ = request.respond(response);
                        }
                        continue;
                    }

                    // Handle /set/get?id={set_id}
                    if url.starts_with("/set/get") {
                        if let Some(query) = url.split('?').nth(1) {
                            for param in query.split('&') {
                                if let Some((key, value)) = param.split_once('=')
                                    && key == "id"
                                {
                                    let file_path =
                                        fixtures_dir.join("sets").join(format!("{value}.json"));

                                    if let Ok(content) = fs::read_to_string(&file_path) {
                                        let response = tiny_http::Response::from_string(content)
                                            .with_header(
                                                tiny_http::Header::from_bytes(
                                                    &b"Content-Type"[..],
                                                    &b"application/json"[..],
                                                )
                                                .unwrap(),
                                            );
                                        let _ = request.respond(response);
                                    } else {
                                        let response = tiny_http::Response::from_string(
                                            r#"{"status_code": 404, "message": "Set not found"}"#,
                                        )
                                        .with_status_code(404);
                                        let _ = request.respond(response);
                                    }
                                    break;
                                }
                            }
                        }
                        continue;
                    }

                    // 404 for other paths
                    let response = tiny_http::Response::from_string(
                        r#"{"status_code": 404, "message": "Not found"}"#,
                    )
                    .with_status_code(404);
                    let _ = request.respond(response);
                }
            });

            // Wait for the server to start or timeout
            std::thread::sleep(std::time::Duration::from_millis(50));
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(5);
            loop {
                // Check if we can connect to the server
                if std::net::TcpStream::connect(("127.0.0.1", TEST_SERVER_PORT)).is_ok() {
                    return true;
                }

                // Timeout - assume server failed to start
                if start.elapsed() > timeout {
                    return false;
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        })
    }

    fn path_from_cache_dir(path: &str) -> String {
        join!(maa_dirs::cache(), "copilot", path)
            .to_string_lossy()
            .into_owned()
    }

    mod copilot_params {
        use super::*;
        use crate::config::task::InitializedTask;

        #[track_caller]
        fn parse_to_tasks<I, T>(args: I, config: &AsstConfig) -> Vec<InitializedTask>
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let command = crate::command::parse_from(args).command;
            let params = match command {
                crate::Command::Copilot { params, .. } => params,
                _ => panic!("Not a Copilot command"),
            };
            params
                .into_task_config(config)
                .expect("Failed to build task config")
                .init()
                .unwrap()
                .tasks
        }

        fn setup() -> Option<AsstConfig> {
            if std::env::var_os("SKIP_CORE_TEST").is_some() {
                return None;
            }

            if !ensure_test_server() {
                // Server failed to start (e.g., can't bind to port in restricted environment)
                return None;
            }

            Some(AsstConfig::default())
        }

        #[test]
        #[ignore = "requires local test server and installed resources"]
        fn basic_single_copilot() {
            let Some(config) = setup() else {
                return;
            };

            let tasks = parse_to_tasks(["maa", "copilot", "maa://40051"], &config);
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                        "is_paradox" => false,
                    )],
                    "formation" => false,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 0,
                ),
            );
        }

        #[test]
        #[ignore = "requires local test server and installed resources"]
        fn all_flags_enabled() {
            let Some(config) = setup() else {
                return;
            };

            let tasks = parse_to_tasks(
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
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => true,
                        "is_paradox" => false,
                    )],
                    "formation" => true,
                    "use_sanity_potion" => true,
                    "add_trust" => true,
                    "formation_index" => 4,
                    "support_unit_name" => "维什戴尔",
                ),
            );
        }

        #[test]
        #[ignore = "requires local test server and installed resources"]
        fn formation_index_with_deprecated_select_formation() {
            let Some(config) = setup() else {
                return;
            };

            let tasks = parse_to_tasks(
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
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                        "is_paradox" => false,
                    )],
                    "formation" => false,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 4,
                ),
            );
        }

        #[test]
        #[ignore = "requires local test server and installed resources"]
        fn raid_mode_2() {
            let Some(config) = setup() else {
                return;
            };

            let tasks = parse_to_tasks(
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
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                        "is_paradox" => false,
                    ),
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => true,
                        "is_paradox" => false,
                    )],
                    "formation" => true,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 0,
                ),
            );
        }

        #[test]
        #[ignore = "requires local test server and installed resources"]
        fn multiple_copilot_uris() {
            let Some(config) = setup() else {
                return;
            };

            let tasks = parse_to_tasks(["maa", "copilot", "maa://40051", "maa://40052"], &config);

            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "stage_name" => "AS-EX-1",
                        "is_raid" => false,
                        "is_paradox" => false,
                    ),
                    object!(
                        "filename" => path_from_cache_dir("40052.json"),
                        "stage_name" => "AS-EX-2",
                        "is_raid" => false,
                        "is_paradox" => false,
                    )],
                    "formation" => true,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 0,
                ),
            );
        }

        #[test]
        #[ignore = "requires local test server and installed resources"]
        fn paradox_simulation() {
            let Some(config) = setup() else {
                return;
            };

            let tasks = parse_to_tasks(["maa", "copilot", "maa://63896"], &config);
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("63896.json"),
                        "stage_name" => "mem_hsguma_1",
                        "is_raid" => false,
                        "is_paradox" => true,
                    )],
                    "formation" => false,
                    "use_sanity_potion" => false,
                    "add_trust" => false,
                    "formation_index" => 0,
                ),
            );
        }

        #[test]
        #[ignore = "requires local test server and installed resources"]
        fn paradox_with_raid_mode_forced_to_zero() {
            let Some(config) = setup() else {
                return;
            };

            // Raid mode should be forced to 0 for paradox simulation
            let tasks = parse_to_tasks(["maa", "copilot", "maa://63896", "--raid", "2"], &config);
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].task_type, TaskType::Copilot);
            assert_eq!(
                tasks[0].params,
                object!(
                    "copilot_list" => [object!(
                        "filename" => path_from_cache_dir("63896.json"),
                        "stage_name" => "mem_hsguma_1",
                        "is_raid" => false,
                        "is_paradox" => true,
                    )],
                    "formation" => false,
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

    mod copilot_file {
        use super::*;

        // from_uri tests
        #[test]
        fn from_uri_remote() {
            assert_eq!(
                CopilotFile::from_uri("maa://30001").unwrap(),
                CopilotFile::Remote(30001)
            );
        }

        #[test]
        fn from_uri_remote_set() {
            assert_eq!(
                CopilotFile::from_uri("maa://20001s").unwrap(),
                CopilotFile::RemoteSet(20001)
            );
        }

        #[test]
        fn from_uri_local_with_prefix() {
            assert_eq!(
                CopilotFile::from_uri("file://file.json").unwrap(),
                CopilotFile::Local(Path::new("file.json"))
            );
        }

        #[test]
        fn from_uri_local_without_prefix() {
            assert_eq!(
                CopilotFile::from_uri("file.json").unwrap(),
                CopilotFile::Local(Path::new("file.json"))
            );
        }

        #[test]
        fn from_uri_with_whitespace() {
            assert_eq!(
                CopilotFile::from_uri("  maa://12345  ").unwrap(),
                CopilotFile::Remote(12345)
            );
        }

        #[test]
        fn from_uri_empty() {
            assert_eq!(
                CopilotFile::from_uri("").unwrap(),
                CopilotFile::Local(Path::new(""))
            );
        }

        #[test]
        fn from_uri_whitespace_only() {
            assert_eq!(
                CopilotFile::from_uri("   ").unwrap(),
                CopilotFile::Local(Path::new(""))
            );
        }

        #[test]
        fn from_uri_invalid_maa_code() {
            let result = CopilotFile::from_uri("maa://not_a_number");
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Invalid code"),
                "Expected 'Invalid code' error, got: {err}"
            );
        }

        #[test]
        fn from_uri_invalid_maa_set_code() {
            let result = CopilotFile::from_uri("maa://not_a_numbers");
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Invalid code"),
                "Expected 'Invalid code' error, got: {err}"
            );
        }

        // push_path_to tests
        #[test]
        fn push_path_to_remote() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-remote");
            fs::create_dir_all(&test_root).unwrap();
            let _ = fs::remove_file(test_root.join("40051.json"));

            let mut paths = Vec::new();
            CopilotFile::Remote(40051)
                .push_path_to(&mut paths, &test_root)
                .unwrap();

            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].0, test_root.join("40051.json"));
            assert!(paths[0].1.is_some()); // Fresh download has cached JSON

            fs::remove_dir_all(&test_root).unwrap();
        }

        #[test]
        fn push_path_to_remote_set() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-remote-set");
            fs::create_dir_all(&test_root).unwrap();

            // Clean up any cached files
            for id in [40051, 40052, 40053, 40055, 40056, 40057, 40058, 40059] {
                let _ = fs::remove_file(test_root.join(format!("{id}.json")));
            }

            let mut paths = Vec::new();
            CopilotFile::RemoteSet(23125)
                .push_path_to(&mut paths, &test_root)
                .unwrap();

            let expected_ids = [40051, 40052, 40053, 40055, 40056, 40057, 40058, 40059];
            assert_eq!(paths.len(), expected_ids.len());
            for (i, id) in expected_ids.iter().enumerate() {
                assert_eq!(paths[i].0, test_root.join(format!("{id}.json")));
            }

            fs::remove_dir_all(&test_root).unwrap();
        }

        #[test]
        fn push_path_to_local_absolute() {
            let test_root = temp_dir().join("maa-test-push-path-local-abs");
            fs::create_dir_all(&test_root).unwrap();

            let test_file = test_root.join("test.json");
            fs::write(&test_file, "{}").unwrap();

            let mut paths = Vec::new();
            CopilotFile::Local(&test_file)
                .push_path_to(&mut paths, &test_root)
                .unwrap();

            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].0, test_file.as_path());
            assert!(paths[0].1.is_none()); // Local files don't have cached JSON

            fs::remove_dir_all(&test_root).unwrap();
        }

        #[test]
        fn push_path_to_local_relative() {
            let test_root = temp_dir().join("maa-test-push-path-local-rel");
            fs::create_dir_all(&test_root).unwrap();

            let mut paths = Vec::new();
            CopilotFile::Local(Path::new("file.json"))
                .push_path_to(&mut paths, &test_root)
                .unwrap();

            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].0, test_root.join("file.json"));
            assert!(paths[0].1.is_none());

            fs::remove_dir_all(&test_root).unwrap();
        }

        #[test]
        fn push_path_to_remote_not_found() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-remote-404");
            fs::create_dir_all(&test_root).unwrap();

            let mut paths = Vec::new();
            let result = CopilotFile::Remote(99999).push_path_to(&mut paths, &test_root);
            assert!(result.is_err());

            let _ = fs::remove_dir_all(&test_root);
        }

        #[test]
        fn push_path_to_remote_set_not_found() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-set-404");
            fs::create_dir_all(&test_root).unwrap();

            let mut paths = Vec::new();
            let result = CopilotFile::RemoteSet(99999).push_path_to(&mut paths, &test_root);
            assert!(result.is_err());

            let _ = fs::remove_dir_all(&test_root);
        }

        #[test]
        fn push_path_to_remote_set_missing_copilot_ids() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-set-missing-ids");
            fs::create_dir_all(&test_root).unwrap();

            let mut paths = Vec::new();
            let result = CopilotFile::RemoteSet(99801).push_path_to(&mut paths, &test_root);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("copilot_ids"),
                "Expected copilot_ids error, got: {err}"
            );

            let _ = fs::remove_dir_all(&test_root);
        }

        #[test]
        fn push_path_to_remote_set_ids_not_array() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-set-ids-not-array");
            fs::create_dir_all(&test_root).unwrap();

            let mut paths = Vec::new();
            let result = CopilotFile::RemoteSet(99802).push_path_to(&mut paths, &test_root);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("not an array"),
                "Expected 'not an array' error, got: {err}"
            );

            let _ = fs::remove_dir_all(&test_root);
        }

        #[test]
        fn push_path_to_remote_set_id_not_integer() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-set-id-not-int");
            fs::create_dir_all(&test_root).unwrap();

            let mut paths = Vec::new();
            let result = CopilotFile::RemoteSet(99803).push_path_to(&mut paths, &test_root);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("not an integer"),
                "Expected 'not an integer' error, got: {err}"
            );

            let _ = fs::remove_dir_all(&test_root);
        }

        #[test]
        fn push_path_to_remote_set_download_fails() {
            if !ensure_test_server() {
                return;
            }

            let test_root = temp_dir().join("maa-test-push-path-set-download-fails");
            fs::create_dir_all(&test_root).unwrap();

            let mut paths = Vec::new();
            // Set 99804 contains copilot_id 99999 which doesn't exist
            let result = CopilotFile::RemoteSet(99804).push_path_to(&mut paths, &test_root);
            assert!(result.is_err());

            let _ = fs::remove_dir_all(&test_root);
        }
    }

    mod operator_table {
        use super::*;

        #[test]
        fn success() {
            let json = serde_json::json!({
                "groups": [
                  {
                    "name": "行医",
                    "opers": [
                      { "name": "纯烬艾雅法拉", "skill": 1, "skill_usage": 0 },
                      { "name": "蜜莓", "skill": 1, "skill_usage": 0 }
                    ]
                  }
                ],
                "opers": [
                  { "name": "桃金娘", "skill": 1, "skill_usage": 1 },
                  { "name": "夜莺", "skill": 3, "skill_usage": 0 }
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

            assert_eq!(super::operator_table(&json).unwrap(), expected_table);
        }

        #[test]
        fn empty_operators_and_groups() {
            let json = serde_json::json!({
                "opers": [],
                "groups": []
            });
            let result = super::operator_table(&json);
            assert!(result.is_ok());
        }

        #[test]
        fn no_opers_or_groups_fields() {
            let json = serde_json::json!({});
            let result = super::operator_table(&json);
            assert!(result.is_ok());
        }

        #[test]
        fn operator_missing_name() {
            let json = serde_json::json!({
                "opers": [{"skill": 1}],
                "groups": []
            });
            let result = super::operator_table(&json);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Missing required field 'name'"),
                "Expected missing name error, got: {err}"
            );
        }

        #[test]
        fn operator_name_not_string() {
            let json = serde_json::json!({
                "opers": [{"name": 123, "skill": 1}],
                "groups": []
            });
            let result = super::operator_table(&json);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("must be a string"),
                "Expected 'must be a string' error, got: {err}"
            );
        }

        #[test]
        fn group_missing_name() {
            let json = serde_json::json!({
                "opers": [],
                "groups": [{"opers": [{"name": "test", "skill": 1}]}]
            });
            let result = super::operator_table(&json);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Missing required field 'name'"),
                "Expected missing name error, got: {err}"
            );
        }

        #[test]
        fn group_missing_opers() {
            let json = serde_json::json!({
                "opers": [],
                "groups": [{"name": "group1"}]
            });
            let result = super::operator_table(&json);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Failed to get opers"),
                "Expected missing opers error, got: {err}"
            );
        }

        #[test]
        fn group_operator_missing_name() {
            let json = serde_json::json!({
                "opers": [],
                "groups": [{"name": "group1", "opers": [{"skill": 1}]}]
            });
            let result = super::operator_table(&json);
            assert!(result.is_err());
        }
    }

    mod fetch_api_response {
        use super::*;

        #[test]
        fn success() {
            if !ensure_test_server() {
                return;
            }

            let result = super::fetch_api_response(&format!("{COPILOT_API}40051"));
            assert!(result.is_ok());
            let resp = result.unwrap();
            assert!(resp.get("data").is_some());
        }

        #[test]
        fn not_found() {
            if !ensure_test_server() {
                return;
            }

            let result = super::fetch_api_response(&format!("{COPILOT_API}99999"));
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("404") || err.contains("Failed to send request"),
                "Expected 404 or connection error, got: {err}"
            );
        }

        #[test]
        fn server_error_status() {
            if !ensure_test_server() {
                return;
            }

            // Request a set that returns status_code 500
            let result = super::fetch_api_response(&format!("{COPILOT_SET_API}99805"));
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("500"), "Expected 500 error, got: {err}");
        }

        #[test]
        fn connection_refused() {
            // Try to connect to a port that's not listening
            let result = super::fetch_api_response("http://127.0.0.1:19999/copilot/get/1");
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Failed to send request"),
                "Expected connection error, got: {err}"
            );
        }
    }

    mod download_copilot {
        use super::*;

        #[test]
        fn success() {
            if !ensure_test_server() {
                return;
            }

            let temp_dir = temp_dir().join("maa-test-download-success");
            fs::create_dir_all(&temp_dir).unwrap();
            let _ = fs::remove_file(temp_dir.join("40051.json"));

            let result = super::download_copilot(40051, &temp_dir);
            assert!(result.is_ok());
            let (path, json) = result.unwrap();
            assert_eq!(path, temp_dir.join("40051.json"));
            assert!(json.is_some());
            assert!(path.exists());

            fs::remove_dir_all(&temp_dir).unwrap();
        }

        #[test]
        fn cache_hit() {
            if !ensure_test_server() {
                return;
            }

            let temp_dir = temp_dir().join("maa-test-download-cache-hit");
            fs::create_dir_all(&temp_dir).unwrap();

            // Create a cached file
            let cached_file = temp_dir.join("12345.json");
            fs::write(&cached_file, r#"{"stage_name": "test"}"#).unwrap();

            // Should return the cached file without making a network request
            let result = super::download_copilot(12345, &temp_dir);
            assert!(result.is_ok());
            let (path, json) = result.unwrap();
            assert_eq!(path, cached_file);
            assert!(json.is_none()); // Cache hit returns None for JSON

            fs::remove_dir_all(&temp_dir).unwrap();
        }

        #[test]
        fn not_found() {
            if !ensure_test_server() {
                return;
            }

            let temp_dir = temp_dir().join("maa-test-download-not-found");
            fs::create_dir_all(&temp_dir).unwrap();

            let result = super::download_copilot(99999, &temp_dir);
            assert!(result.is_err());

            let _ = fs::remove_dir_all(&temp_dir);
        }

        #[test]
        fn invalid_json_content() {
            if !ensure_test_server() {
                return;
            }

            let temp_dir = temp_dir().join("maa-test-download-invalid-json");
            fs::create_dir_all(&temp_dir).unwrap();
            let _ = fs::remove_file(temp_dir.join("99903.json"));

            let result = super::download_copilot(99903, &temp_dir);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Failed to parse copilot JSON"),
                "Expected JSON parse error, got: {err}"
            );

            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    mod json_from_file {
        use super::*;

        #[test]
        fn success() {
            let temp_dir = temp_dir().join("maa-test-json-from-file-success");
            fs::create_dir_all(&temp_dir).unwrap();

            let valid_file = temp_dir.join("valid.json");
            fs::write(&valid_file, r#"{"key": "value"}"#).unwrap();

            let result = super::json_from_file(&valid_file);
            assert!(result.is_ok());
            let json = result.unwrap();
            assert_eq!(json["key"], "value");

            fs::remove_dir_all(&temp_dir).unwrap();
        }

        #[test]
        fn file_not_found() {
            let result = super::json_from_file("/nonexistent/path/to/file.json");
            assert!(result.is_err());
        }

        #[test]
        fn invalid_json() {
            let temp_dir = temp_dir().join("maa-test-json-from-file-invalid");
            fs::create_dir_all(&temp_dir).unwrap();

            let invalid_file = temp_dir.join("invalid.json");
            fs::write(&invalid_file, "not valid json").unwrap();

            let result = super::json_from_file(&invalid_file);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Failed to parse JSON"),
                "Expected JSON parse error, got: {err}"
            );

            let _ = fs::remove_dir_all(&temp_dir);
        }
    }

    mod get_str_key {
        #[test]
        fn success() {
            let json = serde_json::json!({"name": "test"});
            let result = super::get_str_key(&json, "name");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "test");
        }

        #[test]
        fn missing_key() {
            let json = serde_json::json!({"foo": "bar"});
            let result = super::get_str_key(&json, "missing");
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Missing required field 'missing'"),
                "Expected missing field error, got: {err}"
            );
        }

        #[test]
        fn key_not_string() {
            let json = serde_json::json!({"number": 123});
            let result = super::get_str_key(&json, "number");
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("must be a string"),
                "Expected 'must be a string' error, got: {err}"
            );
        }

        #[test]
        fn null_value() {
            let json = serde_json::json!({"null_key": null});
            let result = super::get_str_key(&json, "null_key");
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("must be a string"),
                "Expected 'must be a string' error, got: {err}"
            );
        }
    }

    mod sss_copilot_params {
        use super::*;

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

        #[test]
        fn to_task_type() {
            let params = SSSCopilotParams {
                uri: "maa://40051".to_string(),
                loop_times: 2,
            };

            assert_eq!(params.to_task_type(), TaskType::SSSCopilot);
        }

        #[test]
        fn try_into_maa_value_success() {
            if !ensure_test_server() {
                return;
            }

            assert_eq!(
                parse(["maa", "ssscopilot", "maa://40451"]).unwrap(),
                object!("filename" => path_from_cache_dir("40451.json"), "loop_times" => 1)
            );
            assert_eq!(
                parse(["maa", "ssscopilot", "maa://40451", "--loop-times", "2"]).unwrap(),
                object!("filename" => path_from_cache_dir("40451.json"), "loop_times" => 2)
            );
        }

        #[test]
        fn try_into_maa_value_not_sss_type() {
            if !ensure_test_server() {
                return;
            }

            // 40051 is a regular copilot, not SSS type
            assert!(parse(["maa", "ssscopilot", "maa://40051"]).is_err());
        }

        #[test]
        fn task_set_not_supported() {
            if !ensure_test_server() {
                return;
            }

            // SSSCopilot should fail when given a task set
            let mut paths = Vec::new();
            let temp_dir = temp_dir().join("maa-test-sss-copilot-set");
            fs::create_dir_all(&temp_dir).unwrap();

            // Clean up cached files
            for id in [40051, 40052, 40053, 40055, 40056, 40057, 40058, 40059] {
                let _ = fs::remove_file(temp_dir.join(format!("{id}.json")));
            }

            let copilot_file = CopilotFile::from_uri("maa://23125s").unwrap();
            let result = copilot_file.push_path_to(&mut paths, &temp_dir);

            // The push_path_to succeeds, but we have multiple paths
            assert!(result.is_ok());
            assert!(paths.len() > 1); // Task set returns multiple paths

            let _ = fs::remove_dir_all(&temp_dir);
        }
    }
}
