use std::{collections::BTreeMap, num::NonZero, path::Path};

use anyhow::{Context, Result, bail, ensure};
use log::trace;
use maa_value::userinput::{SelectD, UserInput};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use super::MigrationSummary;
use crate::config::Filetype;

const SUPPORTED_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
pub(super) struct CliConfig {
    pub(super) tasks: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct WpfProfile {
    config_version: u32,
    #[serde(default)]
    current: Option<String>,
    configurations: BTreeMap<String, WpfConfig>,
    #[serde(flatten)]
    #[allow(dead_code)]
    unknown: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WpfConfig {
    #[serde(rename = "TaskQueue", deserialize_with = "deserialize_task_queue")]
    task_queue: Vec<WpfTask>,
    #[serde(rename = "Gui")]
    gui: WpfGuiSettings,
}

impl WpfConfig {
    fn migrate_tasks(
        &self,
        summary: &mut MigrationSummary,
        custom_schedule: Option<&Path>,
    ) -> Result<Option<CliConfig>> {
        let mut tasks = Vec::new();
        let mut schedule_applied = false;
        for task in &self.task_queue {
            if let Some(item) =
                task.migrate_task(&self.gui, summary, custom_schedule, &mut schedule_applied)?
            {
                tasks.push(item);
            }
        }
        if custom_schedule.is_some() && !schedule_applied {
            bail!(
                "`--custom-schedule` was provided, but the WPF profile is not using custom infrastructure scheduling"
            );
        }
        Ok(Some(CliConfig { tasks }))
    }
}

#[derive(Debug)]
enum WpfTask {
    StartUpTask(start_up::WpfStartUpTask),
    FightTask(fight::WpfFightTask),
    InfrastTask(infrast::WpfInfrastTask),
    RecruitTask(recruit::WpfRecruitTask),
    MallTask(mall::WpfMallTask),
    AwardTask(award::WpfAwardTask),
    RoguelikeTask(roguelike::WpfRoguelikeTask),
    ReclamationTask(reclamation::WpfReclamationTask),
    /// Unknown `$type` values are skipped during migration.
    Unsupported {
        type_tag: String,
        name: Option<String>,
    },
}

/// Migrate a MAA WPF GUI profile into a maa-cli task config.
///
/// ```text
/// maa migrate wpf <input> [output]
/// ```
pub(crate) fn wpf(
    file: &Path,
    out: Option<&Path>,
    profile_name: Option<String>,
    custom_schedule: Option<&Path>,
) -> Result<()> {
    ensure!(
        file.extension() == Some("json".as_ref()),
        "`maa migrate wpf` expected a MAA GUI profile (typically .json); input {file:?} is not a JSON file"
    );

    let profile: WpfProfile =
        serde_json::from_reader(std::fs::File::open(file).context("Trying to open wpf profile")?)?;
    ensure!(
        profile.config_version == SUPPORTED_CONFIG_VERSION,
        "Unsupported WPF ConfigVersion {} (expected {SUPPORTED_CONFIG_VERSION})",
        profile.config_version
    );
    let configuration = select_configuration(profile, profile_name)?;
    let (value, summary) = migrate(configuration, custom_schedule)?;

    let mut temp_path = Default::default();
    let out = out.unwrap_or_else(|| {
        temp_path = file.with_extension("toml");
        temp_path.as_path()
    });
    let ft = Filetype::parse_filetype(out).unwrap();
    if let Some(dir) = out.parent() {
        use maa_dirs::Ensure;
        dir.ensure()?;
    }
    ft.write(out, &value)
        .with_context(|| format!("Failed to write migrated file {}", out.display()))?;

    eprint!("{summary}");
    Ok(())
}

/// Pick one configuration from a multi-profile GUI export.
///
/// When multiple configurations exist, `profile_name` can be
/// used to select one without an interactive prompt.
pub(super) fn select_configuration(
    mut profile: WpfProfile,
    profile_name: Option<String>,
) -> Result<WpfConfig> {
    let current = profile.current.unwrap_or_default();
    let profile_name = match profile_name {
        Some(name) => name,
        None => match profile.configurations.len() {
            0 => bail!("GUI profile has no configuration"),
            1 => profile
                .configurations
                .keys()
                .next()
                .context("GUI profile has no configuration")?
                .clone(),
            _ => resolve_configuration_name(&profile.configurations, current)?,
        },
    };
    trace!("Selected configuration: {profile_name}");
    profile
        .configurations
        .remove(&profile_name)
        .with_context(|| format!("GUI configuration {profile_name} not found"))
}

fn resolve_configuration_name(
    configurations: &BTreeMap<String, WpfConfig>,
    current: String,
) -> Result<String> {
    let names: Vec<&str> = configurations.keys().map(String::as_str).collect();
    let default_index = configurations
        .iter()
        .position(|(name, _)| *name == current)
        .and_then(|i| NonZero::new(i + 1));

    SelectD::<String>::from_iter(names, default_index)
        .context("Failed to build configuration selection")?
        .with_description("a GUI configuration")
        .value()
        .context("Failed to select GUI configuration")
}

/// Migrate a GUI configuration into maa-cli task config shape.
pub(super) fn migrate(
    config: WpfConfig,
    custom_schedule: Option<&Path>,
) -> Result<(CliConfig, MigrationSummary)> {
    let mut summary = MigrationSummary::default();
    let value = config
        .migrate_tasks(&mut summary, custom_schedule)?
        .context("GUI configuration produced no CLI config")?;
    Ok((value, summary))
}

/// Meta / structural keys that are never reported as skipped fields.
const META_FIELDS: &[&str] = &["$type", "TaskType", "Name", "IsEnable"];

pub(super) fn report_unknown_fields(
    summary: &mut MigrationSummary,
    type_tag: &str,
    name: Option<String>,
    unknown: &serde_json::Map<String, Value>,
    handled: &[&str],
) {
    for (key, value) in unknown {
        if META_FIELDS.contains(&key.as_str()) || handled.contains(&key.as_str()) {
            continue;
        }
        if is_meaningful_json(value) {
            summary.skip_field(type_tag, name.clone(), key.clone());
        }
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Bool(v) => *v,
        Value::String(s) => !s.is_empty(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i != 0 && i != i64::from(i32::MAX)
            } else if let Some(f) = n.as_f64() {
                f != 0.0
            } else {
                true
            }
        }
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        Value::Null => false,
    }
}

pub(super) fn split_semi_list(list: &str) -> Vec<String> {
    list.split(';')
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn deserialize_task_queue<'de, D>(deserializer: D) -> Result<Vec<WpfTask>, D::Error>
where
    D: Deserializer<'de>,
{
    let items = Vec::<Value>::deserialize(deserializer)?;
    items
        .into_iter()
        .map(parse_wpf_task)
        .collect::<Result<Vec<_>, _>>()
        .map_err(serde::de::Error::custom)
}

fn parse_wpf_task(value: Value) -> Result<WpfTask> {
    let type_tag = value
        .get("$type")
        .and_then(Value::as_str)
        .context("GUI task missing $type")?;
    let name = value
        .get("Name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    match type_tag {
        "StartUpTask" => Ok(WpfTask::StartUpTask(serde_json::from_value(value)?)),
        "FightTask" => Ok(WpfTask::FightTask(serde_json::from_value(value)?)),
        "InfrastTask" => Ok(WpfTask::InfrastTask(serde_json::from_value(value)?)),
        "RecruitTask" => Ok(WpfTask::RecruitTask(serde_json::from_value(value)?)),
        "MallTask" => Ok(WpfTask::MallTask(serde_json::from_value(value)?)),
        "AwardTask" => Ok(WpfTask::AwardTask(serde_json::from_value(value)?)),
        "RoguelikeTask" => Ok(WpfTask::RoguelikeTask(serde_json::from_value(value)?)),
        "ReclamationTask" => Ok(WpfTask::ReclamationTask(serde_json::from_value(value)?)),
        other => Ok(WpfTask::Unsupported {
            type_tag: other.to_string(),
            name,
        }),
    }
}

impl WpfTask {
    fn migrate_task(
        &self,
        gui: &WpfGuiSettings,
        summary: &mut MigrationSummary,
        custom_schedule: Option<&Path>,
        schedule_applied: &mut bool,
    ) -> Result<Option<Value>> {
        match self {
            Self::StartUpTask(start_up) => {
                start_up.report_to(summary);
                Ok(Some(serde_json::to_value(start_up.to_cli_task(gui)?)?))
            }
            Self::FightTask(fight) => {
                fight.report_to(summary);
                Ok(Some(Value::try_from(fight)?))
            }
            Self::InfrastTask(task) => {
                task.report_to(summary);
                Ok(Some(serde_json::to_value(task.to_cli_task(
                    summary,
                    custom_schedule,
                    schedule_applied,
                )?)?))
            }
            Self::RecruitTask(task) => {
                task.report_to(summary);
                Ok(Some(Value::try_from(task)?))
            }
            Self::MallTask(task) => {
                task.report_to(summary);
                Ok(Some(Value::try_from(task)?))
            }
            Self::AwardTask(task) => {
                task.report_to(summary);
                Ok(Some(Value::try_from(task)?))
            }
            Self::RoguelikeTask(task) => {
                task.report_to(summary);
                Ok(Some(Value::try_from(task)?))
            }
            Self::ReclamationTask(task) => {
                task.report_to(summary);
                Ok(Some(Value::try_from(task)?))
            }
            Self::Unsupported { type_tag, name } => {
                summary.skip_task(type_tag, name.clone());
                Ok(None)
            }
        }
    }
}

/// WPF GUI `Gui` object for the selected configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct WpfGuiSettings {
    #[serde(default)]
    runtime_settings: Option<WpfRuntimeSettings>,
    #[serde(flatten)]
    #[allow(dead_code)]
    unknown: serde_json::Map<String, Value>,
}

/// `Gui.RuntimeSettings` fields used by StartUp migration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WpfRuntimeSettings {
    #[serde(default)]
    client_type: Option<String>,
    #[serde(default)]
    start_game: Option<bool>,
    #[serde(flatten)]
    #[allow(dead_code)]
    unknown: serde_json::Map<String, Value>,
}

mod award;
mod fight;
mod infrast;
mod mall;
mod reclamation;
mod recruit;
mod roguelike;
mod start_up;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn configuration_from_task_queue(task: Value) -> WpfConfig {
        serde_json::from_value(serde_json::json!({
            "TaskQueue": [task],
            "Gui": {},
        }))
        .unwrap()
    }

    #[test]
    fn unsupported_task_keeps_original_type_and_name() {
        let config = configuration_from_task_queue(serde_json::json!({
            "$type": "UserDataUpdateTask",
            "Name": "sync",
            "IsEnable": false,
        }));
        let (_, summary) = migrate(config, None).unwrap();
        assert_eq!(summary.skipped_tasks.len(), 1);
        assert_eq!(summary.skipped_tasks[0].type_tag, "UserDataUpdateTask");
        assert_eq!(summary.skipped_tasks[0].name.as_deref(), Some("sync"));
    }

    #[test]
    fn unsupported_config_version_is_rejected() {
        let file = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        std::fs::write(
            file.path(),
            serde_json::json!({
                "ConfigVersion": 2,
                "Current": "Default",
                "Configurations": {
                    "Default": {
                        "TaskQueue": [],
                        "Gui": {},
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
        let err = wpf(file.path(), None, None, None).unwrap_err();
        assert!(
            err.to_string().contains("Unsupported WPF ConfigVersion 2"),
            "{err}"
        );
    }

    #[test]
    fn split_semi_list_filters_empty_items() {
        assert_eq!(split_semi_list("加急许可;招聘许可;"), vec![
            "加急许可".to_string(),
            "招聘许可".to_string()
        ]);
        assert!(split_semi_list("").is_empty());
        assert!(split_semi_list(";;;").is_empty());
    }

    #[test]
    fn select_configuration_picks_named_or_sole_profile() {
        let sole: WpfProfile = serde_json::from_value(serde_json::json!({
            "ConfigVersion": 1,
            "Configurations": {
                "Only": {
                    "TaskQueue": [{
                        "$type": "AwardTask",
                        "Name": "",
                        "IsEnable": true,
                        "Award": true,
                        "Mail": false,
                        "FreeGacha": false,
                        "Orundum": false,
                        "Mining": false,
                        "SpecialAccess": false,
                    }],
                    "Gui": {},
                }
            }
        }))
        .unwrap();
        let cfg = select_configuration(sole, None).unwrap();
        assert_eq!(cfg.task_queue.len(), 1);

        let multi: WpfProfile = serde_json::from_value(serde_json::json!({
            "ConfigVersion": 1,
            "Current": "Default",
            "Configurations": {
                "Default": { "TaskQueue": [], "Gui": {} },
                "Alt": { "TaskQueue": [], "Gui": {} },
            }
        }))
        .unwrap();
        select_configuration(multi, Some("Alt".into())).unwrap();

        let missing: WpfProfile = serde_json::from_value(serde_json::json!({
            "ConfigVersion": 1,
            "Configurations": {
                "Default": { "TaskQueue": [], "Gui": {} },
            }
        }))
        .unwrap();
        let err = select_configuration(missing, Some("Nope".into())).unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");

        let empty: WpfProfile = serde_json::from_value(serde_json::json!({
            "ConfigVersion": 1,
            "Configurations": {},
        }))
        .unwrap();
        let err = select_configuration(empty, None).unwrap_err();
        assert!(err.to_string().contains("no configuration"), "{err}");
    }

    #[test]
    fn report_unknown_fields_ignores_non_meaningful_values() {
        let mut summary = MigrationSummary::default();
        let unknown = serde_json::json!({
            "FutureFlag": true,
            "EmptyString": "",
            "Zero": 0,
            "MaxInt": 2147483647,
            "EmptyArray": [],
            "NullField": null,
        })
        .as_object()
        .unwrap()
        .clone();
        report_unknown_fields(&mut summary, "DemoTask", Some("x".into()), &unknown, &[]);
        assert_eq!(
            summary
                .skipped_fields
                .iter()
                .map(|f| f.field.as_str())
                .collect::<Vec<_>>(),
            ["FutureFlag"]
        );
    }
}
