use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use log::{debug, trace};
use maa_sys::TaskType;
use maa_value::{
    MAAValue, insert, object,
    userinput::{BoolInput, UserInput},
};
use prettytable::{Table, format, row};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use ureq::http::StatusCode;

use super::{IntoParameters, TaskContext, ToTaskType};
use crate::{
    dirs::{self, Ensure},
    state::AGENT,
};

#[cfg(not(test))]
const COPILOT_API: &str = "https://prts.maa.plus/copilot/get/";
#[cfg(not(test))]
const COPILOT_SET_API: &str = "https://prts.maa.plus/set/get?id=";

#[cfg(test)]
const COPILOT_API: &str = "http://127.0.0.1:18080/copilot/get/";
#[cfg(test)]
const COPILOT_SET_API: &str = "http://127.0.0.1:18080/set/get?id=";

// Raid mode constants
const RAID_MODE_NORMAL: u8 = 0;
const RAID_MODE_RAID: u8 = 1;
const RAID_MODE_BOTH: u8 = 2;

#[cfg_attr(test, derive(Default))]
#[derive(clap::Args)]
pub struct CopilotParams {
    /// URI of the copilot task file
    ///
    /// It can be a maa URI or a local file path. Multiple URIs can be provided to fight multiple
    /// stages. For URI, it can be in the format of `maa://<code>`, `maa://<code>s`, `file://<path>`,
    /// which represents a single copilot task, a copilot task set, and a local file respectively,
    /// where `file://` prefix can be omitted.
    uri_list: Vec<String>,
    /// Whether to fight stage in raid mode
    ///
    /// `0` for normal, `1` for raid, `2` run twice for both normal and raid
    #[arg(long, default_value = "0")]
    raid: u8,
    /// Enable auto formation
    ///
    /// When multiple uri are provided or a copilot task set contains multiple stages, force to
    /// true. Otherwise, default to false.
    #[arg(long)]
    formation: bool,
    /// Select which formation to use (1-4)
    ///
    /// If not provided, use the current formation
    #[arg(long)]
    formation_index: Option<i32>,
    /// Fill empty slots by ascending trust value during auto formation.
    #[arg(long)]
    add_trust: bool,
    // Ignore operator attribute requirements during auto formation.
    #[arg(long)]
    ignore_requirements: bool,

    /// Use sanity potion to restore sanity when it's not enough
    #[arg(long)]
    use_sanity_potion: bool,

    /// Support operator usage mode.
    ///
    /// Effective only when formation is true. Available modes:
    ///
    /// - `0`: Do not use support operators (default).
    /// - `1`: Use support operator only if exactly one operator is missing; otherwise, do not use
    ///   support.
    /// - `2`: Use support operator if one is missing; otherwise, use the specified one.
    /// - `3`: Use support operator if one is missing; otherwise, use a random one.
    #[arg(long)]
    support_unit_usage: Option<i32>,

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

impl TryFrom<StageOpts> for MAAValue {
    type Error = maa_value::Error;

    fn try_from(opts: StageOpts) -> Result<Self, Self::Error> {
        Ok(object!(
            "filename" => opts.filename?,
            "stage_name" => opts.stage_name,
            "is_raid" => opts.is_raid,
        ))
    }
}

#[derive(Debug, serde::Deserialize)]
struct CopilotOperator {
    name: String,
    skill: i32,
}

#[derive(Debug, serde::Deserialize)]
struct CopilotGroup {
    name: String,
    opers: Vec<CopilotOperator>,
}

#[derive(Debug, serde::Deserialize)]
struct CopilotDoc {
    details: String,
}

#[derive(Debug, serde::Deserialize)]
struct CopilotTask {
    stage_name: String,
    #[serde(default)]
    opers: Vec<CopilotOperator>,
    #[serde(default)]
    groups: Vec<CopilotGroup>,
    #[serde(default)]
    doc: Option<CopilotDoc>,
    #[serde(rename = "type", default)]
    task_type: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct StageInfo {
    code: String,
}

impl ToTaskType for CopilotParams {
    fn to_task_type(&self) -> TaskType {
        TaskType::Copilot
    }
}

impl IntoParameters for CopilotParams {
    fn into_parameters_no_context(self) -> Result<MAAValue> {
        unreachable!("This method should not be called");
    }

    fn into_parameters(self, context: TaskContext<'_>) -> Result<MAAValue> {
        let base_dirs = context.config.resource.base_dirs();
        let default = context.default;

        let copilot_files = resolve_copilot_uris(self.uri_list)?;

        let is_task_list = copilot_files.len() > 1;
        let formation = self.formation || is_task_list || default.get_or("formation", false);

        let use_sanity_potion =
            self.use_sanity_potion || default.get_or("use_sanity_potion", false);
        let add_trust = self.add_trust || default.get_or("add_trust", false);
        let ignore_requirements =
            self.ignore_requirements || default.get_or("ignore_requirements", false);

        let mut stage_list = Vec::new();
        for (_, file, value) in copilot_files {
            let copilot_task = value.map(Ok).unwrap_or_else(|| json_from_file(&file))?;
            let stage_id = &copilot_task.stage_name;

            let stage_info = get_stage_info(stage_id, base_dirs.iter().map(|dir| dir.as_path()))?;
            let stage_code = &stage_info.code;

            if !formation {
                println!("Operators:\n{}", operator_table(&copilot_task)?);
                println!("Please set up your formation manually");
                while !BoolInput::new(Some(true))
                    .with_description("continue")
                    .value()?
                {
                    println!("Please confirm you have set up your formation");
                }
            }

            match self.raid {
                RAID_MODE_NORMAL | RAID_MODE_RAID => stage_list.push(StageOpts {
                    filename: file.to_path_buf(),
                    stage_name: stage_code.to_owned(),
                    is_raid: self.raid == RAID_MODE_RAID,
                }),
                RAID_MODE_BOTH => {
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

        // We also want all other parameters from overlay
        let mut params = default;
        insert!(params,
            "formation" => formation,
            "use_sanity_potion" => use_sanity_potion,
            "add_trust" => add_trust,
            "ignore_requirements" => ignore_requirements,
            "formation_index" =>? self.formation_index,
            "support_unit_usage" =>? self.support_unit_usage,
            "support_unit_name" =>? self.support_unit_name
        );

        // Use single file mode when there's only one stage in the list
        if stage_list.len() == 1 {
            let stage_opt = stage_list
                .into_iter()
                .next()
                .expect("single-file mode requires exactly one copilot stage");
            insert!(params,
                "filename" => stage_opt.filename.to_string_lossy().to_string()
            );
        } else {
            insert!(params,
                "copilot_list" => stage_list?
            );
        }

        Ok(params)
    }
}

fn get_stage_info<P, D>(stage_id: &str, base_dirs: D) -> Result<StageInfo>
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

impl IntoParameters for SSSCopilotParams {
    fn into_parameters_no_context(self) -> Result<MAAValue> {
        let copilot_dir = dirs::copilot().ensure()?;

        let copilot_file = CopilotFile::from_uri(&self.uri)?;
        let mut files = Vec::new();
        copilot_file.push_path_into::<CopilotTask>(0, copilot_dir, &mut files)?;

        if files.len() != 1 {
            bail!("SSS Copilot don't support task set");
        }

        let (_, file, value) = files.pop().expect("files have length 1");
        let task = value.map(Ok).unwrap_or_else(|| json_from_file(&file))?;

        if task.task_type.as_deref() != Some("SSS") {
            bail!("The given copilot file is not a SSS copilot file");
        }

        let stage_name = &task.stage_name;

        println!("Fight Stage: {stage_name}, please navigate to the stage manually");
        while !BoolInput::new(Some(true))
            .with_description("continue")
            .value()?
        {
            println!("Please confirm you have navigated to the stage");
        }
        println!("Core Operators:\n{}", operator_table(&task)?);
        // TODO: equipment, support unit, toolmans
        if let Some(doc) = &task.doc
            && BoolInput::new(Some(false))
                .with_description("show doc")
                .value()?
        {
            println!("{}", doc.details);
        }

        while !BoolInput::new(Some(true))
            .with_description("continue")
            .value()?
        {
            println!("Please confirm you have set up your formation");
        }

        let value = object!(
            "filename" => file?,
            "loop_times" => self.loop_times,
        );

        Ok(value)
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum CopilotFile {
    Remote(u64),
    RemoteSet(u64),
    Local(PathBuf),
}

impl CopilotFile {
    fn from_uri(uri: &str) -> Result<Self> {
        let trimmed = uri.trim();
        if let Some(code_str) = trimmed.strip_prefix("maa://") {
            if let Some(code_str) = code_str.strip_suffix('s') {
                Ok(CopilotFile::RemoteSet(
                    code_str.parse::<u64>().context("Invalid code")?,
                ))
            } else {
                Ok(CopilotFile::Remote(
                    code_str.parse::<u64>().context("Invalid code")?,
                ))
            }
        } else if let Some(code) = trimmed.strip_prefix("file://") {
            Ok(CopilotFile::Local(PathBuf::from(code)))
        } else {
            Ok(CopilotFile::Local(PathBuf::from(trimmed)))
        }
    }

    pub fn push_path_into<T>(
        self,
        index: usize,
        base_dir: impl AsRef<Path>,
        files: &mut Vec<(usize, PathBuf, Option<T>)>,
    ) -> Result<()>
    where
        T: serde::de::DeserializeOwned + Send,
    {
        let base_dir = base_dir.as_ref();

        #[derive(serde::Deserialize)]
        struct CopilotResponse<D> {
            status_code: u16,
            data: D,
        }

        #[derive(serde::Deserialize)]
        struct SingleData {
            content: String,
        }

        #[derive(serde::Deserialize)]
        struct SetData {
            copilot_ids: Vec<u64>,
        }

        match self {
            CopilotFile::Remote(code) => {
                let code = code.to_string();
                let json_file = base_dir.join(&code).with_extension("json");

                if json_file.is_file() {
                    debug!("Cache hit, using cached json file {}", json_file.display());
                    files.push((index, json_file, None));
                    return Ok(());
                }

                let url = format!("{COPILOT_API}{code}");
                debug!("Cache miss, downloading copilot from {url}");
                let mut response = AGENT
                    .get(&url)
                    .call()
                    .with_context(|| format!("Failed to send request to {url}"))?;

                let resp: CopilotResponse<SingleData> = response
                    .body_mut()
                    .read_json()
                    .with_context(|| {
                        format!("Failed to parse JSON response from {url}. The server may have disconnected or returned invalid data")
                    })?;

                if resp.status_code == StatusCode::OK {
                    let content = resp.data.content;

                    fs::File::create(&json_file)
                        .context("Failed to create json file")?
                        .write_all(content.as_bytes())
                        .context("Failed to write json file")?;

                    files.push((index, json_file, Some(serde_json::from_str(&content)?)));

                    Ok(())
                } else {
                    bail!("Request Error, code: {code}");
                }
            }
            CopilotFile::RemoteSet(code) => {
                let url = format!("{COPILOT_SET_API}{code}");
                debug!("Get copilot set from {url}");
                let mut response = AGENT
                    .get(&url)
                    .call()
                    .with_context(|| format!("Failed to send request to {url}"))?;

                let resp: CopilotResponse<SetData>= response
                    .body_mut()
                    .read_json()
                    .with_context(|| {
                        format!("Failed to parse JSON response from {url}. The server may have disconnected or returned invalid data")
                    })?;

                if resp.status_code == StatusCode::OK {
                    let ids = resp.data.copilot_ids;

                    // Download all copilot files in parallel
                    let sub_files = ids
                        .into_par_iter()
                        .try_fold(Vec::new, |mut files, id| {
                            CopilotFile::Remote(id)
                                .push_path_into::<T>(index, base_dir, &mut files)?;
                            Ok::<_, anyhow::Error>(files)
                        })
                        .try_reduce(Vec::new, |mut a, b| {
                            a.extend(b);
                            Ok(a)
                        })?;

                    files.extend(sub_files);

                    Ok(())
                } else {
                    bail!("Request Error, code: {}", code);
                }
            }
            CopilotFile::Local(file) => {
                let file = if file.is_absolute() {
                    file
                } else {
                    base_dir.join(file)
                };

                files.push((index, file, None));

                Ok(())
            }
        }
    }
}

fn json_from_file<V: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<V> {
    Ok(serde_json::from_reader(fs::File::open(path)?)?)
}

fn operator_table(task: &CopilotTask) -> Result<Table> {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["NAME", "SKILL"]);

    for operator in &task.opers {
        table.add_row(row![&operator.name, operator.skill]);
    }

    for group in &task.groups {
        let mut sub_table = Table::new();
        sub_table.set_format(*format::consts::FORMAT_NO_LINESEP);
        for operator in &group.opers {
            sub_table.add_row(row![&operator.name, operator.skill]);
        }

        let vertical_offset = (sub_table.len() + 2) >> 1;

        table.add_row(row![
            format!("{}[{}]", "\n".repeat(vertical_offset - 1), &group.name),
            sub_table
        ]);
    }

    Ok(table)
}

/// Resolve a list of URIs into local copilot file paths.
///
/// This function handles downloading remote files, expanding task sets,
/// and resolving local file paths. The returned list is sorted by the
/// original URI index to preserve order.
fn resolve_copilot_uris(
    uri_list: Vec<String>,
) -> Result<Vec<(usize, PathBuf, Option<CopilotTask>)>> {
    let copilot_dir = dirs::copilot().ensure()?;

    let mut copilot_files = uri_list
        .into_par_iter()
        .enumerate()
        .try_fold(Vec::new, |mut files, (index, uri)| {
            CopilotFile::from_uri(&uri)?.push_path_into::<CopilotTask>(
                index,
                copilot_dir,
                &mut files,
            )?;
            Ok::<_, anyhow::Error>(files)
        })
        .try_reduce(Vec::new, |mut a, b| {
            a.extend(b);
            Ok(a)
        })?;
    copilot_files.sort_by(|(index_a, ..), (index_b, ..)| index_a.cmp(index_b));

    Ok(copilot_files)
}

#[cfg_attr(test, derive(Default))]
#[derive(clap::Args)]
pub struct ParadoxCopilotParams {
    /// URI of the paradox copilot task file
    ///
    /// It can be a maa URI or a local file path. Multiple URIs can be provided.
    /// For URI, it can be in the format of `maa://<code>`, `maa://<code>s`, `file://<path>`,
    /// which represents a single copilot task, a copilot task set, and a local file respectively,
    /// where `file://` prefix can be omitted.
    uri_list: Vec<String>,
}

impl ToTaskType for ParadoxCopilotParams {
    fn to_task_type(&self) -> TaskType {
        TaskType::ParadoxCopilot
    }
}

impl IntoParameters for ParadoxCopilotParams {
    fn into_parameters_no_context(self) -> Result<MAAValue> {
        let copilot_files = resolve_copilot_uris(self.uri_list)?;

        let file_paths: Vec<PathBuf> = copilot_files.into_iter().map(|(_, file, _)| file).collect();

        Ok(object!("list" => file_paths?))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{env::temp_dir, fs, path::PathBuf, sync::Once, thread};

    use super::*;
    use crate::config::asst::AsstConfig;

    const TEST_SERVER_PORT: u16 = 18080;

    static INIT_SERVER: Once = Once::new();

    /// Ensures the test HTTP server is started.
    /// The server is started once and runs for the lifetime of the test process.
    fn ensure_test_server() {
        INIT_SERVER.call_once(|| {
            const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
            let fixtures_dir = PathBuf::from(MANIFEST_DIR).join("fixtures").join("copilot");

            // Start HTTP server in a background thread
            thread::spawn(move || {
                let server = tiny_http::Server::http(("127.0.0.1", TEST_SERVER_PORT))
                    .expect("Failed to bind test server");

                for request in server.incoming_requests() {
                    let url = request.url();

                    // Handle /copilot/get/{task_id}
                    if let Some(task_id) = url.strip_prefix("/copilot/get/") {
                        let file_path = fixtures_dir.join("tasks").join(format!("{task_id}.json"));

                        if let Ok(content) = fs::read_to_string(&file_path) {
                            let response = tiny_http::Response::from_string(content).with_header(
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
                                            r#"{"status_code": 404, "message": "Set not
found"}"#,
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

            // Wait a bit for server to start
            thread::sleep(std::time::Duration::from_millis(100));
        });
    }

    fn path_from_cache_dir(path: &str) -> String {
        use maa_str_ext::ToUtf8String;
        join!(maa_dirs::cache(), "copilot", path)
            .to_utf8_string()
            .expect("Cache directory path contains invalid UTF-8")
    }

    mod copilot_params {
        use super::*;

        mod to_task_type {
            use super::*;

            #[test]
            fn to_task_type() {
                let params = CopilotParams::default();
                assert_eq!(params.to_task_type(), TaskType::Copilot);
            }
        }

        mod into_parameters {
            use super::*;

            fn parse<I, T>(args: I) -> Result<MAAValue>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                parse_with_default(args, MAAValue::default())
            }

            fn parse_with_default<I, T>(args: I, default: MAAValue) -> Result<MAAValue>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                let config = AsstConfig::default();
                let command = crate::command::parse_from(args).command;
                match command {
                    crate::Command::Copilot { params, .. } => params.into_parameters(TaskContext {
                        default,
                        config: &config,
                    }),
                    _ => panic!("Not a Copilot command"),
                }
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn single_task() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let params = parse(["maa", "copilot", "maa://40051"]).unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => false,
                        "use_sanity_potion" => false,
                        "add_trust" => false,
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn with_all_options() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let params = parse([
                    "maa",
                    "copilot",
                    "maa://40051",
                    "--raid=1",
                    "--formation",
                    "--use-sanity-potion",
                    "--add-trust",
                    "--formation-index",
                    "4",
                    "--support-unit-name",
                    "维什戴尔",
                ])
                .unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => true,
                        "use_sanity_potion" => true,
                        "add_trust" => true,
                        "ignore_requirements" => false,
                        "formation_index" => 4,
                        "support_unit_name" => "维什戴尔",
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn with_formation_index() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let params =
                    parse(["maa", "copilot", "maa://40051", "--formation-index", "4"]).unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => false,
                        "use_sanity_potion" => false,
                        "add_trust" => false,
                        "ignore_requirements" => false,
                        "formation_index" => 4,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn raid_mode_both() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let params = parse([
                    "maa",
                    "copilot",
                    "maa://40051",
                    "--raid",
                    "2",
                    "--formation",
                ])
                .unwrap();

                assert_eq!(
                    params,
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
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn multiple_tasks() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let params = parse(["maa", "copilot", "maa://40051", "maa://40052"]).unwrap();

                assert_eq!(
                    params,
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
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn invalid_raid_mode() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let result = parse(["maa", "copilot", "maa://40051", "--raid", "3"]);

                assert!(result.is_err());
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("Invalid raid mode")
                );
            }

            #[test]
            fn non_existent_local_file() {
                let result = parse(["maa", "copilot", "non_existent_file.json"]);
                assert!(result.is_err());
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn with_default_formation() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let default = object!("formation" => true);
                let params =
                    parse_with_default(["maa", "copilot", "maa://40051"], default).unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => true,
                        "use_sanity_potion" => false,
                        "add_trust" => false,
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn with_default_use_sanity_potion() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let default = object!("use_sanity_potion" => true);
                let params =
                    parse_with_default(["maa", "copilot", "maa://40051"], default).unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => false,
                        "use_sanity_potion" => true,
                        "add_trust" => false,
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn with_default_add_trust() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let default = object!("add_trust" => true);
                let params =
                    parse_with_default(["maa", "copilot", "maa://40051"], default).unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => false,
                        "use_sanity_potion" => false,
                        "add_trust" => true,
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn with_extra_default() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                let default = object!("__extra_param___" => true);
                let params =
                    parse_with_default(["maa", "copilot", "maa://40051"], default).unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => false,
                        "use_sanity_potion" => false,
                        "add_trust" => false,
                        "ignore_requirements" => false,
                        "__extra_param___" => true,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn cli_overrides_default() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                // Default says formation is true, but CLI explicitly sets it to false
                // CLI should win
                let default = object!(
                    "formation" => true,
                    "use_sanity_potion" => true,
                );
                let params =
                    parse_with_default(["maa", "copilot", "maa://40051", "--formation"], default)
                        .unwrap();

                assert_eq!(
                    params,
                    object!(
                        "filename" => path_from_cache_dir("40051.json"),
                        "formation" => true,
                        "use_sanity_potion" => true,
                        "add_trust" => false,
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[ignore = "requires installed resources"]
            fn multiple_tasks_overrides_default_formation() {
                ensure_test_server();

                if std::env::var_os("SKIP_CORE_TEST").is_some() {
                    return;
                }

                // Default says formation is false, but multiple tasks force it to true
                let default = object!("formation" => false);
                let params =
                    parse_with_default(["maa", "copilot", "maa://40051", "maa://40052"], default)
                        .unwrap();

                assert_eq!(
                    params,
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
                        "ignore_requirements" => false,
                    ),
                );
            }

            #[test]
            #[should_panic]
            fn into_parameters_no_context_should_panic() {
                let cmd = crate::command::parse_from(["maa", "copilot", "maa://40051"]).command;
                let _ = match cmd {
                    crate::Command::Copilot { params, .. } => params.into_parameters_no_context(),
                    _ => panic!("Unexpected command"),
                };
            }
        }

        mod get_stage_info {
            use super::*;

            #[test]
            fn from_id() {
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

                let stage_info =
                    super::get_stage_info(stage_id, std::slice::from_ref(&resource_dir)).unwrap();

                assert_eq!(stage_info.code, "AS-EX-1");
            }
        }
    }

    mod sss_copilot_params {
        use super::*;

        mod to_task_type {
            use super::*;

            #[test]
            fn returns_sss_copilot() {
                let params = SSSCopilotParams {
                    uri: "maa://40051".to_string(),
                    loop_times: 2,
                };

                assert_eq!(params.to_task_type(), TaskType::SSSCopilot);
            }
        }

        mod into_parameters {
            use super::*;

            fn parse<I, T>(args: I) -> Result<MAAValue>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                let command = crate::command::parse_from(args).command;
                match command {
                    crate::Command::SSSCopilot { params, .. } => {
                        params.into_parameters_no_context()
                    }
                    _ => panic!("Not a SSSCopilot command"),
                }
            }

            #[test]
            fn rejects_non_sss_copilot() {
                ensure_test_server();

                let result = parse(["maa", "ssscopilot", "maa://40051"]);

                assert!(result.is_err());
                println!("Error: {:?}", result);
            }

            #[test]
            fn default_loop_times() {
                ensure_test_server();

                let params = parse(["maa", "ssscopilot", "maa://40451"]).unwrap();

                assert_eq!(
                    params,
                    object!("filename" => path_from_cache_dir("40451.json"), "loop_times" => 1)
                );
            }

            #[test]
            fn custom_loop_times() {
                ensure_test_server();

                let params =
                    parse(["maa", "ssscopilot", "maa://40451", "--loop-times", "2"]).unwrap();

                assert_eq!(
                    params,
                    object!("filename" => path_from_cache_dir("40451.json"), "loop_times" => 2)
                );
            }

            #[test]
            fn rejects_task_set() {
                ensure_test_server();

                let result = parse(["maa", "ssscopilot", "maa://23125s"]);

                assert!(result.is_err());
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("don't support task set")
                );
            }
        }
    }

    mod copilot_file {
        use super::*;

        mod from_uri {
            use super::*;

            #[test]
            fn invalid_code() {
                assert!(CopilotFile::from_uri("maa://xyz").is_err());
            }

            #[test]
            fn remote_set() {
                assert_eq!(
                    CopilotFile::from_uri("maa://20001s").unwrap(),
                    CopilotFile::RemoteSet(20001)
                );
            }

            #[test]
            fn remote() {
                assert_eq!(
                    CopilotFile::from_uri("maa://30001").unwrap(),
                    CopilotFile::Remote(30001)
                );
            }

            #[test]
            fn local_with_file_scheme() {
                assert_eq!(
                    CopilotFile::from_uri("file://file.json").unwrap(),
                    CopilotFile::Local(PathBuf::from("file.json"))
                );
            }

            #[test]
            fn local_without_scheme() {
                assert_eq!(
                    CopilotFile::from_uri("file.json").unwrap(),
                    CopilotFile::Local(PathBuf::from("file.json"))
                );
            }

            #[test]
            fn with_whitespace() {
                assert_eq!(
                    CopilotFile::from_uri("  maa://30001  ").unwrap(),
                    CopilotFile::Remote(30001)
                );

                assert_eq!(
                    CopilotFile::from_uri("  file.json  ").unwrap(),
                    CopilotFile::Local(PathBuf::from("file.json"))
                );
            }
        }

        mod push_path_into {
            use super::*;

            // Helper function to test push_path_into
            fn assert_push_path_into(
                uri: &str,
                index: usize,
                base_dir: &Path,
                expected_paths: &[PathBuf],
            ) {
                let mut files = Vec::new();
                CopilotFile::from_uri(uri)
                    .unwrap()
                    .push_path_into::<CopilotTask>(index, base_dir, &mut files)
                    .unwrap();

                assert_eq!(files.len(), expected_paths.len());
                for (i, expected_path) in expected_paths.iter().enumerate() {
                    assert_eq!(files[i].0, index);
                    assert_eq!(&files[i].1, expected_path);
                }
            }

            #[test]
            fn remote() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-remote");
                fs::create_dir_all(&test_root).unwrap();

                let _ = fs::remove_file(test_root.join("40051.json"));

                assert_push_path_into(
                    "maa://40051",
                    0,
                    &test_root,
                    &[test_root.join("40051.json")],
                );

                fs::remove_dir_all(&test_root).unwrap();
            }

            #[test]
            fn remote_uses_cache() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-cache");
                fs::create_dir_all(&test_root).unwrap();

                // First call downloads the file
                assert_push_path_into(
                    "maa://40051",
                    0,
                    &test_root,
                    &[test_root.join("40051.json")],
                );

                // Second call should use cache
                assert_push_path_into(
                    "maa://40051",
                    0,
                    &test_root,
                    &[test_root.join("40051.json")],
                );

                fs::remove_dir_all(&test_root).unwrap();
            }

            #[test]
            fn remote_set() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-set");
                fs::create_dir_all(&test_root).unwrap();

                // Clean up any cached files from previous runs
                for id in [40051, 40052, 40053, 40055, 40056, 40057, 40058, 40059] {
                    let _ = fs::remove_file(test_root.join(format!("{id}.json")));
                }

                let expected_paths: Vec<PathBuf> =
                    [40051, 40052, 40053, 40055, 40056, 40057, 40058, 40059]
                        .iter()
                        .map(|id| test_root.join(format!("{id}.json")))
                        .collect();

                assert_push_path_into("maa://23125s", 0, &test_root, &expected_paths);

                fs::remove_dir_all(&test_root).unwrap();
            }

            #[test]
            fn local_absolute() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-local-abs");
                fs::create_dir_all(&test_root).unwrap();

                let test_file = test_root.join("123234.json");
                let test_content = serde_json::json!({
                  "minimum_required": "v4.0.0",
                  "stage_name": "act25side_01",
                  "actions": [{ "type": "SpeedUp" }],
                  "groups": [],
                  "opers": [],
                });

                serde_json::to_writer(fs::File::create(&test_file).unwrap(), &test_content)
                    .unwrap();

                assert_push_path_into(
                    test_file.to_str().unwrap(),
                    0,
                    &test_root,
                    std::slice::from_ref(&test_file),
                );

                fs::remove_dir_all(&test_root).unwrap();
            }

            #[test]
            fn local_relative() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-local-rel");
                fs::create_dir_all(&test_root).unwrap();

                assert_push_path_into("file.json", 0, &test_root, &[test_root.join("file.json")]);

                fs::remove_dir_all(&test_root).unwrap();
            }

            #[test]
            fn preserves_index() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-index");
                fs::create_dir_all(&test_root).unwrap();

                let _ = fs::remove_file(test_root.join("40051.json"));

                // Test with different indices
                for index in [0, 1, 5, 100] {
                    let mut files = Vec::new();
                    CopilotFile::from_uri("maa://40051")
                        .unwrap()
                        .push_path_into::<CopilotTask>(index, &test_root, &mut files)
                        .unwrap();

                    assert_eq!(files[0].0, index);
                }

                fs::remove_dir_all(&test_root).unwrap();
            }

            #[test]
            fn non_existent_remote_task() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-non-existent-remote");
                fs::create_dir_all(&test_root).unwrap();

                let mut files = Vec::new();
                let result = CopilotFile::from_uri("maa://999999")
                    .unwrap()
                    .push_path_into::<CopilotTask>(0, &test_root, &mut files);

                assert!(result.is_err());
                assert!(files.is_empty());

                fs::remove_dir_all(&test_root).unwrap();
            }

            #[test]
            fn non_existent_remote_set() {
                ensure_test_server();

                let test_root = temp_dir().join("maa-test-push-path-into-non-existent-set");
                fs::create_dir_all(&test_root).unwrap();

                let mut files = Vec::new();
                let result = CopilotFile::from_uri("maa://999999s")
                    .unwrap()
                    .push_path_into::<CopilotTask>(0, &test_root, &mut files);

                assert!(result.is_err());
                assert!(files.is_empty());

                fs::remove_dir_all(&test_root).unwrap();
            }
        }
    }

    #[test]
    fn gen_operator_table() {
        let json = serde_json::json!({
            "stage_name": "test_stage",
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

        let task: CopilotTask = serde_json::from_value(json).unwrap();
        assert_eq!(operator_table(&task).unwrap(), expected_table);
    }

    mod paradox_copilot_params {
        use super::*;

        mod to_task_type {
            use super::*;

            #[test]
            fn returns_paradox_copilot() {
                let params = ParadoxCopilotParams::default();
                assert_eq!(params.to_task_type(), TaskType::ParadoxCopilot);
            }
        }

        mod into_parameters {
            use super::*;

            fn parse<I, T>(args: I) -> Result<MAAValue>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                let command = crate::command::parse_from(args).command;
                match command {
                    crate::Command::ParadoxCopilot { params, .. } => {
                        params.into_parameters_no_context()
                    }
                    _ => panic!("Not a ParadoxCopilot command"),
                }
            }

            #[test]
            fn single_file() {
                ensure_test_server();

                let params = parse(["maa", "paradoxcopilot", "maa://63896"]).unwrap();

                assert_eq!(
                    params,
                    object!("list" => [path_from_cache_dir("63896.json")])
                );
            }
        }
    }
}
