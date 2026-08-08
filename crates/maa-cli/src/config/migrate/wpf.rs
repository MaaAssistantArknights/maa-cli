use std::{io::BufReader, num::NonZero, path::Path};

use anyhow::{Context, Result, bail, ensure};
use log::trace;
use maa_value::{
    prelude::*,
    userinput::{SelectD, UserInput},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::MigrationSummary;
use crate::config::Filetype;

/// Migrate a MAA WPF GUI profile into a maa-cli task config.
///
/// ```text
/// maa migrate wpf <input> [output]
/// ```
pub(crate) fn wpf(file: &Path, out: Option<&Path>, config: Option<String>) -> Result<()> {
    ensure!(
        file.extension() == Some("json".as_ref()),
        "`maa migrate wpf` expected a MAA GUI profile (typically .json); input {file:?} is not a JSON file"
    );

    let value = BufReader::new(std::fs::File::open(file).context("Trying to open wpf profile")?);
    let value = serde_json::from_reader(value)?;
    let value = select_configuration(value, config)?;
    let (value, summary) = migrate(value)?;

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

    summary.print();
    Ok(())
}

/// Pick one configuration from a multi-profile GUI export.
///
/// Legacy single-config profiles are returned unchanged.
///
/// When multiple configurations exist, `config_name` can be
/// used to select one without an interactive prompt.
pub(super) fn select_configuration(mut input: Value, config_name: Option<String>) -> Result<Value> {
    let current = input
        .get("Current")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_default();
    let Some(configurations) = input.get_mut("Configurations") else {
        bail!("GUI profile missing Configurations");
    };

    let config_name = match config_name {
        Some(name) => name,
        None => {
            let Some(object) = configurations.as_object() else {
                bail!("GUI profile Configurations is not an object");
            };
            match object.len() {
                0 => bail!("GUI profile has no configuration"),
                1 => object.keys().next().unwrap().to_string(),
                _ => resolve_configuration_name(object, current)?,
            }
        }
    };
    trace!("Selected configuration: {config_name}");
    let Some(profile) = configurations.get_mut(&config_name) else {
        bail!("GUI configuration {config_name} not found");
    };
    Ok(profile.take())
}

fn resolve_configuration_name(
    object: &serde_json::map::Map<String, Value>,
    current: String,
) -> Result<String> {
    let names: Vec<&str> = object.keys().map(String::as_str).collect();
    let default_index = object
        .iter()
        .position(|(name, _)| *name == current)
        .and_then(|i| NonZero::new(i + 1));

    SelectD::<String>::from_iter(names, default_index)
        .context("Failed to build configuration selection")?
        .with_description("a GUI configuration")
        .value()
        .context("Failed to select GUI configuration")
}

/// Migrate a GUI profile into maa-cli task config shape.
pub(super) fn migrate(input: Value) -> Result<(MAAValue, MigrationSummary)> {
    let mut summary = MigrationSummary::default();
    let config: WpfConfiguration =
        serde_json::from_value(input).context("Failed to deserialize GUI configuration")?;
    let value = config
        .migrate_tasks(&mut summary)?
        .context("GUI configuration produced no CLI config")?;
    Ok((value, summary))
}

/// Serialize a typed CLI config prototype into [`MAAValue`].
fn serialize_to_maa_value<T: Serialize>(value: &T) -> Result<MAAValue> {
    let json = serde_json::to_value(value).context("Failed to serialize CLI config prototype")?;
    serde_json::from_value(json).context("Failed to convert CLI config prototype to MAAValue")
}

/// Meta / structural keys that are never reported as skipped fields.
const META_FIELDS: &[&str] = &["$type", "TaskType", "Name", "IsEnable"];

fn report_unknown_fields(
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

fn split_semi_list(list: &str) -> Vec<String> {
    list.split(';')
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Serialize)]
struct CliConfig {
    tasks: Vec<MAAValue>,
}

#[derive(Debug, Deserialize)]
struct WpfConfiguration {
    #[serde(rename = "TaskQueue")]
    task_queue: Vec<WpfTask>,
    #[serde(rename = "Gui")]
    gui: WpfGuiSettings,
}

impl WpfConfiguration {
    fn migrate_tasks(&self, summary: &mut MigrationSummary) -> Result<Option<MAAValue>> {
        let mut tasks = Vec::new();
        for task in &self.task_queue {
            if let Some(item) = task.migrate_task(&self.gui, summary)? {
                tasks.push(item);
            }
        }
        Ok(Some(serialize_to_maa_value(&CliConfig { tasks })?))
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "$type")]
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
    #[serde(other)]
    Unsupported,
}

impl WpfTask {
    fn migrate_task(
        &self,
        gui: &WpfGuiSettings,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        match self {
            Self::StartUpTask(start_up) => {
                start_up.report_to(summary);
                Ok(Some(start_up.to_maa_value(gui)?))
            }
            Self::FightTask(fight) => {
                fight.report_disabled(summary);
                Ok(Some(MAAValue::try_from(fight)?))
            }
            Self::InfrastTask(task) => {
                task.report_to(summary);
                Ok(Some(task.to_maa_value()?))
            }
            Self::RecruitTask(task) => {
                task.report_to(summary);
                Ok(Some(task.to_maa_value()?))
            }
            Self::MallTask(task) => {
                task.report_to(summary);
                Ok(Some(task.to_maa_value()?))
            }
            Self::AwardTask(task) => {
                task.report_to(summary);
                Ok(Some(task.to_maa_value()?))
            }
            Self::RoguelikeTask(task) => {
                task.report_to(summary);
                Ok(Some(task.to_maa_value()?))
            }
            Self::ReclamationTask(task) => {
                task.report_to(summary);
                Ok(Some(task.to_maa_value()?))
            }
            Self::Unsupported => {
                summary.skip_task("Unsupported", None);
                Ok(None)
            }
        }
    }
}

/// WPF GUI `Gui` object for the selected configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WpfGuiSettings {
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

mod start_up {
    use anyhow::Result;
    use log::warn;
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, WpfGuiSettings, serialize_to_maa_value};
    use crate::config::task::ClientType;

    /// maa-cli StartUp task shape written by migration.
    #[derive(Debug, Serialize)]
    struct CliStartUpTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        #[serde(skip_serializing_if = "str::is_empty")]
        name: String,
        params: CliStartUpParams,
    }

    #[derive(Debug, Serialize)]
    struct CliStartUpParams {
        client_type: CliClientTypeParam,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_game_enabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        account_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    /// Fixed client type string, or an interactive select prompt.
    #[derive(Debug, Serialize)]
    #[serde(untagged)]
    enum CliClientTypeParam {
        Fixed(ClientType),
        Prompt {
            alternatives: &'static [&'static str],
        },
    }

    /// WPF GUI `StartUpTask` (`$type = "StartUpTask"`).
    ///
    /// Task-queue fields combine with [`WpfGuiSettings::runtime_settings`] when
    /// converting to [`CliStartUpTask`] / [`MAAValue`].
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfStartUpTask {
        name: String,
        is_enable: bool,
        account_switch_enabled: bool,
        /// Absent when the GUI export omits the key.
        #[serde(default)]
        account_name: Option<String>,
        #[serde(flatten)]
        #[allow(dead_code)]
        unknown: Map<String, Value>,
    }

    impl WpfStartUpTask {
        pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("StartUpTask", Some(self.name.clone()));
            }
            if self.account_switch_enabled && self.account_name.is_none() {
                summary.skip_field("StartUpTask", Some(self.name.clone()), "AccountName");
                warn!("AccountName is missing, but GUI enable account switching");
            }
        }

        pub(super) fn to_maa_value(&self, gui: &WpfGuiSettings) -> Result<MAAValue> {
            serialize_to_maa_value(&CliStartUpTask::try_from((self, gui))?)
        }
    }

    impl TryFrom<(&WpfStartUpTask, &WpfGuiSettings)> for CliStartUpTask {
        type Error = anyhow::Error;

        fn try_from((task, gui): (&WpfStartUpTask, &WpfGuiSettings)) -> Result<Self> {
            let runtime = gui.runtime_settings.as_ref();

            let client_type = match runtime.and_then(|r| r.client_type.as_deref()) {
                Some(s) if !s.is_empty() => match s.parse::<ClientType>() {
                    Ok(client) => CliClientTypeParam::Fixed(client),
                    Err(_) => CliClientTypeParam::Prompt {
                        alternatives: &ClientType::NAMES,
                    },
                },
                _ => CliClientTypeParam::Prompt {
                    alternatives: &ClientType::NAMES,
                },
            };

            let account_name = if task.account_switch_enabled {
                task.account_name.clone()
            } else {
                None
            };

            Ok(CliStartUpTask {
                task_type: TaskType::StartUp,
                name: task.name.clone(),
                params: CliStartUpParams {
                    client_type,
                    start_game_enabled: runtime.and_then(|r| r.start_game),
                    account_name,
                    enable: (!task.is_enable).then_some(false),
                },
            })
        }
    }
}

mod fight {
    use anyhow::{Context, Result, bail};
    use chrono::Weekday;
    use log::warn;
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, serialize_to_maa_value};
    use crate::config::task::{ClientType, Condition, TimeOffset};

    /// maa-cli Fight task shape written by migration.
    #[derive(Debug, Serialize)]
    struct CliFightTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<CliFightParams>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        variants: Vec<CliFightVariant>,
    }

    #[derive(Debug, Serialize)]
    struct CliFightVariant {
        condition: Condition,
        params: CliFightParams,
    }

    /// Fight params; field order matches historical `object!` / `insert!` output for parity.
    #[derive(Clone, Debug, Default, Serialize)]
    struct CliFightParams {
        #[serde(skip_serializing_if = "Option::is_none")]
        medicine: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stone: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        times: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        drops: Option<maa_value::map::StringMap<i32>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        series: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        medicine_expire_days: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stage: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    impl CliFightParams {
        fn is_empty(&self) -> bool {
            self.medicine.is_none()
                && self.stone.is_none()
                && self.times.is_none()
                && self.drops.is_none()
                && self.series.is_none()
                && self.medicine_expire_days.is_none()
                && self.stage.is_none()
                && self.enable.is_none()
        }

        fn stage_only(stage: &str) -> Self {
            Self {
                stage: Some(stage.to_string()),
                ..Self::default()
            }
        }
    }

    /// WPF GUI `FightTask` (`$type = "FightTask"`).
    ///
    /// Deserialized from the GUI task queue; converts into [`CliFightTask`] /
    /// [`MAAValue`] via [`TryFrom`]. Unmapped keys are collected in `unknown`.
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfFightTask {
        name: String,
        is_enable: bool,

        use_weekly_schedule: bool,
        /// Absent when `UseWeeklySchedule` is false in typical GUI exports.
        #[serde(default)]
        weekly_schedule: Option<WpfWeeklySchedule>,
        use_optional_stage: bool,
        stage_plan: WpfStagePlan,

        use_medicine: bool,
        medicine_count: i32,
        use_stone: bool,
        stone_count: i32,
        enable_times_limit: bool,
        times_limit: i32,
        enable_target_drop: bool,
        drop_id: String,
        drop_count: i32,
        series: i32,
        use_expiring_medicine: bool,
        medicine_expire_days: i32,

        #[serde(flatten)]
        #[allow(dead_code)]
        unknown: Map<String, Value>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct WpfWeeklySchedule {
        sunday: bool,
        monday: bool,
        tuesday: bool,
        wednesday: bool,
        thursday: bool,
        friday: bool,
        saturday: bool,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum WpfStagePlan {
        Single(String),
        Many(Vec<String>),
    }

    impl WpfFightTask {
        pub(super) fn report_disabled(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("FightTask", Some(self.name.clone()));
            }
        }

        /// Build the open-condition for a fight stage name.
        ///
        /// - Resource stages with a weekly rotation → `Weekday` (server timezone Official)
        /// - Permanent stages (mainline, annihilation, LS-6, OF-*) → `Always`
        /// - Side-story stages listed in `StageActivityV2.json` → `OnSideStory`
        fn stage_condition(stage: &str) -> Condition {
            use Weekday::*;
            match stage {
                // 资源本 / 芯片本（同开放日合并）
                "CE-6" => Condition::Weekday {
                    weekdays: vec![Tue, Thu, Sat, Sun],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                "AP-5" => Condition::Weekday {
                    weekdays: vec![Mon, Thu, Sat, Sun],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                "CA-5" => Condition::Weekday {
                    weekdays: vec![Tue, Wed, Fri, Sun],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                "SK-5" => Condition::Weekday {
                    weekdays: vec![Mon, Wed, Fri, Sat],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                "PR-A-1" | "PR-A-2" => Condition::Weekday {
                    weekdays: vec![Mon, Thu, Fri, Sun],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                "PR-B-1" | "PR-B-2" => Condition::Weekday {
                    weekdays: vec![Mon, Tue, Fri, Sat],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                "PR-C-1" | "PR-C-2" => Condition::Weekday {
                    weekdays: vec![Wed, Thu, Sat, Sun],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                "PR-D-1" | "PR-D-2" => Condition::Weekday {
                    weekdays: vec![Tue, Wed, Sat, Sun],
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                // 永久开启
                "LS-6" | "Annihilation" | "OF-1" | "OF-F3" => Condition::Always,
                _ => {
                    let side_story = crate::activity::side_story_stages(ClientType::Official);
                    if side_story.iter().any(|code| code == stage) {
                        return Condition::OnSideStory {
                            client: ClientType::Official,
                        };
                    }
                    warn!(
                        "FightTask stage `{stage}` has no known open schedule; treating as Always"
                    );
                    Condition::Always
                }
            }
        }
    }

    impl From<&WpfFightTask> for CliFightParams {
        fn from(task: &WpfFightTask) -> Self {
            CliFightParams {
                medicine: task.use_medicine.then_some(task.medicine_count),
                stone: task.use_stone.then(|| {
                    let count = task.stone_count;
                    warn!(
                        "FightTask enables stone={count}; this setting may consume Originite Prime (源石)"
                    );
                    count
                }),
                times: task.enable_times_limit.then_some(task.times_limit),
                drops: task.enable_target_drop.then(|| {
                    let mut drops = maa_value::map::StringMap::new();
                    drops.insert(task.drop_id.clone(), task.drop_count);
                    drops
                }),
                series: (task.series != 0).then_some(task.series),
                medicine_expire_days: task
                    .use_expiring_medicine
                    .then_some(task.medicine_expire_days),
                stage: None,
                enable: None,
            }
        }
    }

    impl From<&WpfWeeklySchedule> for Condition {
        fn from(schedule: &WpfWeeklySchedule) -> Self {
            let days = [
                (schedule.sunday, Weekday::Sun),
                (schedule.monday, Weekday::Mon),
                (schedule.tuesday, Weekday::Tue),
                (schedule.wednesday, Weekday::Wed),
                (schedule.thursday, Weekday::Thu),
                (schedule.friday, Weekday::Fri),
                (schedule.saturday, Weekday::Sat),
            ];

            let weekdays: Vec<Weekday> = days
                .into_iter()
                .filter_map(|(enabled, day)| enabled.then_some(day))
                .collect();

            Condition::Weekday {
                weekdays,
                timezone: TimeOffset::Local,
            }
        }
    }

    impl<'a> TryFrom<&'a WpfStagePlan> for &'a str {
        type Error = anyhow::Error;

        fn try_from(plan: &'a WpfStagePlan) -> Result<&'a str> {
            match plan {
                WpfStagePlan::Single(stage) => Ok(stage.as_str()),
                WpfStagePlan::Many(stages) if stages.len() == 1 => Ok(stages[0].as_str()),
                WpfStagePlan::Many(_) => {
                    bail!(
                        "FightTask StagePlan must be a single stage string when UseOptionalStage is false, \
                         got a string array; enable UseOptionalStage for multiple stages"
                    )
                }
            }
        }
    }

    impl<'a> TryFrom<&'a WpfStagePlan> for Vec<&'a str> {
        type Error = anyhow::Error;

        fn try_from(plan: &'a WpfStagePlan) -> Result<Vec<&'a str>> {
            match plan {
                WpfStagePlan::Many(stages) => {
                    if stages.is_empty() {
                        bail!(
                            "FightTask StagePlan must be a non-empty string array when UseOptionalStage is true"
                        );
                    }
                    Ok(stages.iter().map(String::as_str).collect())
                }
                WpfStagePlan::Single(_) => {
                    bail!(
                        "FightTask StagePlan must be a string array when UseOptionalStage is true, got a string"
                    )
                }
            }
        }
    }

    impl TryFrom<&WpfFightTask> for CliFightTask {
        type Error = anyhow::Error;

        fn try_from(task: &WpfFightTask) -> Result<Self> {
            // Shared fight options (medicine, stone, expire days, ...) belong on the task.
            // Variants only carry stage selection / open-condition params.
            let shared_params = CliFightParams::from(task);

            let mut item = match (task.use_weekly_schedule, task.use_optional_stage) {
                // No variants: StagePlan must be a single stage.
                (false, false) => {
                    let stage: &str = (&task.stage_plan).try_into()?;
                    let mut params = shared_params;
                    params.stage = Some(stage.to_string());
                    CliFightTask {
                        task_type: TaskType::Fight,
                        name: task.name.clone(),
                        params: Some(params),
                        variants: Vec::new(),
                    }
                }
                // One variant gated by weekly schedule; StagePlan must be a single stage.
                (true, false) => {
                    let stage: &str = (&task.stage_plan).try_into()?;
                    let weekly = Condition::from(
                        task.weekly_schedule.as_ref().context(
                            "FightTask UseWeeklySchedule is true but WeeklySchedule is missing or invalid",
                        )?,
                    );
                    CliFightTask {
                        task_type: TaskType::Fight,
                        name: task.name.clone(),
                        params: (!shared_params.is_empty()).then_some(shared_params),
                        variants: vec![CliFightVariant {
                            condition: weekly,
                            params: CliFightParams::stage_only(stage),
                        }],
                    }
                }
                // One variant per optional stage, each with its own open-condition.
                (false, true) => {
                    let stages: Vec<&str> = (&task.stage_plan).try_into()?;
                    let variants = stages
                        .into_iter()
                        .map(|stage| CliFightVariant {
                            condition: WpfFightTask::stage_condition(stage),
                            params: CliFightParams::stage_only(stage),
                        })
                        .collect();
                    CliFightTask {
                        task_type: TaskType::Fight,
                        name: task.name.clone(),
                        params: (!shared_params.is_empty()).then_some(shared_params),
                        variants,
                    }
                }
                // Weekly schedule AND each stage's open-condition.
                (true, true) => {
                    let stages: Vec<&str> = (&task.stage_plan).try_into()?;
                    let weekly = Condition::from(
                        task.weekly_schedule.as_ref().context(
                            "FightTask UseWeeklySchedule is true but WeeklySchedule is missing or invalid",
                        )?,
                    );
                    let variants = stages
                        .into_iter()
                        .map(|stage| CliFightVariant {
                            condition: Condition::And {
                                conditions: vec![
                                    weekly.clone(),
                                    WpfFightTask::stage_condition(stage),
                                ],
                            },
                            params: CliFightParams::stage_only(stage),
                        })
                        .collect();
                    CliFightTask {
                        task_type: TaskType::Fight,
                        name: task.name.clone(),
                        params: (!shared_params.is_empty()).then_some(shared_params),
                        variants,
                    }
                }
            };

            // MaaCore common switch: params.enable=false disables the task.
            if !task.is_enable {
                match &mut item.params {
                    Some(params) => params.enable = Some(false),
                    None => {
                        item.params = Some(CliFightParams {
                            enable: Some(false),
                            ..CliFightParams::default()
                        });
                    }
                }
            }

            Ok(item)
        }
    }

    impl TryFrom<&WpfFightTask> for MAAValue {
        type Error = anyhow::Error;

        fn try_from(task: &WpfFightTask) -> Result<Self> {
            serialize_to_maa_value(&CliFightTask::try_from(task)?)
        }
    }
}

mod infrast {
    use anyhow::{Result, bail};
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, report_unknown_fields, serialize_to_maa_value};

    const HANDLED: &[&str] = &[
        "Mode",
        "RoomList",
        "UsesOfDrones",
        "DormThreshold",
        "OriginiumShardAutoReplenishment",
        "DormFilterNotStationed",
        "DormTrustEnabled",
        "ReceptionMessageBoard",
        "ReceptionClueExchange",
        "SendClue",
        "Filename",
        "PlanSelect",
    ];

    #[derive(Debug, Serialize)]
    struct CliInfrastTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        #[serde(skip_serializing_if = "str::is_empty")]
        name: String,
        params: CliInfrastParams,
    }

    #[derive(Debug, Serialize)]
    struct CliInfrastParams {
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<i32>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        facility: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        drones: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        replenish: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dorm_notstationed_enabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dorm_trust_enabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reception_message_board: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reception_clue_exchange: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reception_send_clue: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct WpfInfrastRoom {
        room: String,
    }

    /// WPF GUI `InfrastTask` (`$type = "InfrastTask"`).
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfInfrastTask {
        name: String,
        is_enable: bool,
        mode: String,
        #[serde(default)]
        filename: String,
        room_list: Vec<WpfInfrastRoom>,
        uses_of_drones: String,
        dorm_threshold: i32,
        originium_shard_auto_replenishment: bool,
        dorm_filter_not_stationed: bool,
        dorm_trust_enabled: bool,
        reception_message_board: bool,
        reception_clue_exchange: bool,
        send_clue: bool,
        #[serde(flatten)]
        unknown: Map<String, Value>,
    }

    impl WpfInfrastTask {
        fn mapped_mode(&self) -> Option<i32> {
            match self.mode.as_str() {
                "Normal" => Some(0),
                "Rotation" => Some(20000),
                _ => None,
            }
        }

        pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("InfrastTask", Some(self.name.clone()));
            }
            if self.mode != "Custom" && self.mapped_mode().is_none() {
                summary.skip_field("InfrastTask", Some(self.name.clone()), "Mode");
            }
            report_unknown_fields(
                summary,
                "InfrastTask",
                Some(self.name.clone()),
                &self.unknown,
                HANDLED,
            );
        }

        pub(super) fn to_maa_value(&self) -> Result<MAAValue> {
            serialize_to_maa_value(&CliInfrastTask::try_from(self)?)
        }
    }

    impl TryFrom<&WpfInfrastTask> for CliInfrastTask {
        type Error = anyhow::Error;

        fn try_from(task: &WpfInfrastTask) -> Result<Self> {
            if task.mode == "Custom" {
                bail!("InfrastTask custom mode is not supported yet");
            }
            if !task.filename.is_empty() {
                bail!("InfrastTask custom plan (Filename) is not supported yet");
            }

            Ok(CliInfrastTask {
                task_type: TaskType::Infrast,
                name: task.name.clone(),
                params: CliInfrastParams {
                    mode: task.mapped_mode(),
                    facility: task.room_list.iter().map(|r| r.room.clone()).collect(),
                    drones: Some(task.uses_of_drones.clone()),
                    threshold: Some(task.dorm_threshold as f32 / 100.0),
                    replenish: Some(task.originium_shard_auto_replenishment),
                    dorm_notstationed_enabled: Some(task.dorm_filter_not_stationed),
                    dorm_trust_enabled: Some(task.dorm_trust_enabled),
                    reception_message_board: Some(task.reception_message_board),
                    reception_clue_exchange: Some(task.reception_clue_exchange),
                    reception_send_clue: Some(task.send_clue),
                    enable: (!task.is_enable).then_some(false),
                },
            })
        }
    }
}

mod recruit {
    use anyhow::Result;
    use log::warn;
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, report_unknown_fields, serialize_to_maa_value};

    const HANDLED: &[&str] = &[
        "MaxTimes",
        "ExtraTagMode",
        "RefreshLevel3",
        "ForceRefresh",
        "Level6Choose",
        "Level6Time",
        "Level5Choose",
        "Level5Time",
        "Level4Choose",
        "Level4Time",
        "Level3Choose",
        "Level3Time",
        "PreferTagEnabled",
        "Level3PreferTags",
        "PreserveTagEnabled",
        "PreserveTagList",
    ];

    #[derive(Debug, Serialize)]
    struct CliRecruitTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        #[serde(skip_serializing_if = "str::is_empty")]
        name: String,
        params: CliRecruitParams,
    }

    #[derive(Debug, Serialize)]
    struct CliRecruitParams {
        #[serde(skip_serializing_if = "Option::is_none")]
        times: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        extra_tags_mode: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refresh: Option<bool>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        select: Vec<i32>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        confirm: Vec<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        recruitment_time: Option<maa_value::map::StringMap<i32>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        first_tags: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        preserve_tags: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    /// WPF GUI `RecruitTask` (`$type = "RecruitTask"`).
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfRecruitTask {
        name: String,
        is_enable: bool,
        max_times: i32,
        extra_tag_mode: i32,
        refresh_level3: bool,
        force_refresh: bool,
        level3_choose: bool,
        #[serde(default)]
        level3_time: Option<i32>,
        level4_choose: bool,
        #[serde(default)]
        level4_time: Option<i32>,
        level5_choose: bool,
        #[serde(default)]
        level5_time: Option<i32>,
        level6_choose: bool,
        #[serde(default)]
        level6_time: Option<i32>,
        prefer_tag_enabled: bool,
        #[serde(default)]
        level3_prefer_tags: Vec<String>,
        preserve_tag_enabled: bool,
        #[serde(default)]
        preserve_tag_list: Vec<String>,
        #[serde(flatten)]
        unknown: Map<String, Value>,
    }

    impl WpfRecruitTask {
        pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("RecruitTask", Some(self.name.clone()));
            }
            // GUI ForceRefresh is not maa-cli/MaaCore `expedite`; maa-cli does not support it.
            if self.force_refresh {
                summary.skip_field("RecruitTask", Some(self.name.clone()), "ForceRefresh");
                warn!(
                    "RecruitTask ForceRefresh is not supported by maa-cli; this setting has no effect"
                );
            }
            if self.level6_choose {
                warn!(
                    "RecruitTask Level6Choose is enabled; recruitment will auto-confirm 6★ operators (dangerous)"
                );
            }
            report_unknown_fields(
                summary,
                "RecruitTask",
                Some(self.name.clone()),
                &self.unknown,
                HANDLED,
            );
        }

        pub(super) fn to_maa_value(&self) -> Result<MAAValue> {
            serialize_to_maa_value(&CliRecruitTask::try_from(self)?)
        }
    }

    impl TryFrom<&WpfRecruitTask> for CliRecruitTask {
        type Error = anyhow::Error;

        fn try_from(task: &WpfRecruitTask) -> Result<Self> {
            let levels = [
                (task.level6_choose, task.level6_time, 6),
                (task.level5_choose, task.level5_time, 5),
                (task.level4_choose, task.level4_time, 4),
                (task.level3_choose, task.level3_time, 3),
            ];

            let mut select = Vec::new();
            let mut confirm = Vec::new();
            let mut recruitment_time = maa_value::map::StringMap::new();
            for (choose, minutes, level) in levels {
                if choose {
                    select.push(level);
                    confirm.push(level);
                }
                if let Some(minutes) = minutes {
                    recruitment_time.insert(level.to_string(), minutes);
                }
            }

            let first_tags = (task.prefer_tag_enabled && !task.level3_prefer_tags.is_empty())
                .then(|| task.level3_prefer_tags.clone());
            let preserve_tags = (task.preserve_tag_enabled && !task.preserve_tag_list.is_empty())
                .then(|| task.preserve_tag_list.clone());

            Ok(CliRecruitTask {
                task_type: TaskType::Recruit,
                name: task.name.clone(),
                params: CliRecruitParams {
                    times: Some(task.max_times),
                    extra_tags_mode: Some(task.extra_tag_mode),
                    refresh: Some(task.refresh_level3),
                    select,
                    confirm,
                    recruitment_time: (!recruitment_time.is_empty()).then_some(recruitment_time),
                    first_tags,
                    preserve_tags,
                    enable: (!task.is_enable).then_some(false),
                },
            })
        }
    }
}

mod mall {
    use anyhow::Result;
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, report_unknown_fields, serialize_to_maa_value, split_semi_list};

    const HANDLED: &[&str] = &[
        "Shopping",
        "CreditFight",
        "CreditFightFormation",
        "VisitFriends",
        "FirstList",
        "BlackList",
        "ShoppingIgnoreBlackListWhenFull",
        "OnlyBuyDiscount",
        "ReserveMaxCredit",
    ];

    #[derive(Debug, Serialize)]
    struct CliMallTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        #[serde(skip_serializing_if = "str::is_empty")]
        name: String,
        params: CliMallParams,
    }

    #[derive(Debug, Serialize)]
    struct CliMallParams {
        #[serde(skip_serializing_if = "Option::is_none")]
        shopping: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        credit_fight: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        formation_index: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        visit_friends: Option<bool>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        buy_first: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        blacklist: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        force_shopping_if_credit_full: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        only_buy_discount: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reserve_max_credit: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    /// WPF GUI `MallTask` (`$type = "MallTask"`).
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfMallTask {
        name: String,
        is_enable: bool,
        shopping: bool,
        credit_fight: bool,
        credit_fight_formation: i32,
        visit_friends: bool,
        first_list: String,
        black_list: String,
        shopping_ignore_black_list_when_full: bool,
        only_buy_discount: bool,
        reserve_max_credit: bool,
        #[serde(flatten)]
        unknown: Map<String, Value>,
    }

    impl WpfMallTask {
        pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("MallTask", Some(self.name.clone()));
            }
            report_unknown_fields(
                summary,
                "MallTask",
                Some(self.name.clone()),
                &self.unknown,
                HANDLED,
            );
        }

        pub(super) fn to_maa_value(&self) -> Result<MAAValue> {
            serialize_to_maa_value(&CliMallTask::try_from(self)?)
        }
    }

    impl TryFrom<&WpfMallTask> for CliMallTask {
        type Error = anyhow::Error;

        fn try_from(task: &WpfMallTask) -> Result<Self> {
            Ok(CliMallTask {
                task_type: TaskType::Mall,
                name: task.name.clone(),
                params: CliMallParams {
                    shopping: Some(task.shopping),
                    credit_fight: Some(task.credit_fight),
                    formation_index: Some(task.credit_fight_formation),
                    visit_friends: Some(task.visit_friends),
                    buy_first: split_semi_list(&task.first_list),
                    blacklist: split_semi_list(&task.black_list),
                    force_shopping_if_credit_full: Some(task.shopping_ignore_black_list_when_full),
                    only_buy_discount: Some(task.only_buy_discount),
                    reserve_max_credit: Some(task.reserve_max_credit),
                    enable: (!task.is_enable).then_some(false),
                },
            })
        }
    }
}

mod award {
    use anyhow::Result;
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, report_unknown_fields, serialize_to_maa_value};

    const HANDLED: &[&str] = &[
        "Award",
        "Mail",
        "FreeGacha",
        "Orundum",
        "Mining",
        "SpecialAccess",
    ];

    #[derive(Debug, Serialize)]
    struct CliAwardTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        #[serde(skip_serializing_if = "str::is_empty")]
        name: String,
        params: CliAwardParams,
    }

    #[derive(Debug, Serialize)]
    struct CliAwardParams {
        #[serde(skip_serializing_if = "Option::is_none")]
        award: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mail: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        recruit: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        orundum: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mining: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        specialaccess: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    /// WPF GUI `AwardTask` (`$type = "AwardTask"`).
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfAwardTask {
        name: String,
        is_enable: bool,
        award: bool,
        mail: bool,
        free_gacha: bool,
        orundum: bool,
        mining: bool,
        special_access: bool,
        #[serde(flatten)]
        unknown: Map<String, Value>,
    }

    impl WpfAwardTask {
        pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("AwardTask", Some(self.name.clone()));
            }
            report_unknown_fields(
                summary,
                "AwardTask",
                Some(self.name.clone()),
                &self.unknown,
                HANDLED,
            );
        }

        pub(super) fn to_maa_value(&self) -> Result<MAAValue> {
            serialize_to_maa_value(&CliAwardTask::try_from(self)?)
        }
    }

    impl TryFrom<&WpfAwardTask> for CliAwardTask {
        type Error = anyhow::Error;

        fn try_from(task: &WpfAwardTask) -> Result<Self> {
            Ok(CliAwardTask {
                task_type: TaskType::Award,
                name: task.name.clone(),
                params: CliAwardParams {
                    award: Some(task.award),
                    mail: Some(task.mail),
                    recruit: Some(task.free_gacha),
                    orundum: Some(task.orundum),
                    mining: Some(task.mining),
                    specialaccess: Some(task.special_access),
                    enable: (!task.is_enable).then_some(false),
                },
            })
        }
    }
}

mod roguelike {
    use anyhow::Result;
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, report_unknown_fields, serialize_to_maa_value};

    const HANDLED: &[&str] = &[
        "Theme",
        "Mode",
        "Squad",
        "Roles",
        "CoreChar",
        "StartCount",
        "Difficulty",
        "Investment",
        "InvestCount",
        "InvestWithMoreScore",
        "StopWhenDepositFull",
        "StopAtFinalBoss",
        "StopWhenLevelMax",
        "UseSupport",
        "UseSupportNonFriend",
        "RefreshTraderWithDice",
        "StartWithEliteTwo",
        "StartWithEliteTwoOnly",
    ];

    #[derive(Debug, Serialize)]
    struct CliRoguelikeTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        #[serde(skip_serializing_if = "str::is_empty")]
        name: String,
        params: CliRoguelikeParams,
    }

    #[derive(Debug, Serialize)]
    struct CliRoguelikeParams {
        #[serde(skip_serializing_if = "Option::is_none")]
        theme: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        squad: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        roles: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        core_char: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        starts_count: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        difficulty: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        investment_enabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        investments_count: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        investment_with_more_score: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_when_investment_full: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_at_final_boss: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_at_max_level: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        use_support: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        use_nonfriend_support: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refresh_trader_with_dice: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_with_elite_two: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        only_start_with_elite_two: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    /// WPF GUI `RoguelikeTask` (`$type = "RoguelikeTask"`).
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfRoguelikeTask {
        name: String,
        is_enable: bool,
        theme: String,
        mode: String,
        squad: String,
        roles: String,
        core_char: String,
        start_count: i32,
        difficulty: i32,
        investment: bool,
        invest_count: i32,
        invest_with_more_score: bool,
        stop_when_deposit_full: bool,
        stop_at_final_boss: bool,
        stop_when_level_max: bool,
        use_support: bool,
        use_support_non_friend: bool,
        refresh_trader_with_dice: bool,
        start_with_elite_two: bool,
        start_with_elite_two_only: bool,
        #[serde(flatten)]
        unknown: Map<String, Value>,
    }

    impl WpfRoguelikeTask {
        fn mapped_mode(&self) -> Option<i32> {
            match self.mode.as_str() {
                "Exp" => Some(0),
                "Investment" => Some(1),
                "Collect" => Some(4),
                "CollapsalParadigms" => Some(5),
                "MonthlySquad" => Some(6),
                "DeepExploration" => Some(7),
                _ => None,
            }
        }

        pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("RoguelikeTask", Some(self.name.clone()));
            }
            if self.mapped_mode().is_none() {
                summary.skip_field("RoguelikeTask", Some(self.name.clone()), "Mode");
            }
            report_unknown_fields(
                summary,
                "RoguelikeTask",
                Some(self.name.clone()),
                &self.unknown,
                HANDLED,
            );
        }

        pub(super) fn to_maa_value(&self) -> Result<MAAValue> {
            serialize_to_maa_value(&CliRoguelikeTask::try_from(self)?)
        }
    }

    impl TryFrom<&WpfRoguelikeTask> for CliRoguelikeTask {
        type Error = anyhow::Error;

        fn try_from(task: &WpfRoguelikeTask) -> Result<Self> {
            Ok(CliRoguelikeTask {
                task_type: TaskType::Roguelike,
                name: task.name.clone(),
                params: CliRoguelikeParams {
                    theme: Some(task.theme.clone()),
                    mode: task.mapped_mode(),
                    squad: Some(task.squad.clone()),
                    roles: Some(task.roles.clone()),
                    core_char: Some(task.core_char.clone()),
                    starts_count: Some(task.start_count),
                    difficulty: (task.difficulty != i32::MAX).then_some(task.difficulty),
                    investment_enabled: Some(task.investment),
                    investments_count: Some(task.invest_count),
                    investment_with_more_score: Some(task.invest_with_more_score),
                    stop_when_investment_full: Some(task.stop_when_deposit_full),
                    stop_at_final_boss: Some(task.stop_at_final_boss),
                    stop_at_max_level: Some(task.stop_when_level_max),
                    use_support: Some(task.use_support),
                    use_nonfriend_support: Some(task.use_support_non_friend),
                    refresh_trader_with_dice: Some(task.refresh_trader_with_dice),
                    start_with_elite_two: Some(task.start_with_elite_two),
                    only_start_with_elite_two: Some(task.start_with_elite_two_only),
                    enable: (!task.is_enable).then_some(false),
                },
            })
        }
    }
}

mod reclamation {
    use anyhow::Result;
    use maa_types::TaskType;
    use maa_value::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    use super::{MigrationSummary, report_unknown_fields, serialize_to_maa_value};

    const HANDLED: &[&str] = &[
        "Theme",
        "Mode",
        "ToolToCraft",
        "IncrementMode",
        "MaxCraftCountPerRound",
    ];

    #[derive(Debug, Serialize)]
    struct CliReclamationTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        #[serde(skip_serializing_if = "str::is_empty")]
        name: String,
        params: CliReclamationParams,
    }

    #[derive(Debug, Serialize)]
    struct CliReclamationParams {
        #[serde(skip_serializing_if = "Option::is_none")]
        theme: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tools_to_craft: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        increment_mode: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        num_craft_batches: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enable: Option<bool>,
    }

    /// WPF GUI `ReclamationTask` (`$type = "ReclamationTask"`).
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct WpfReclamationTask {
        name: String,
        is_enable: bool,
        theme: String,
        mode: String,
        tool_to_craft: String,
        increment_mode: i32,
        max_craft_count_per_round: i32,
        #[serde(flatten)]
        unknown: Map<String, Value>,
    }

    impl WpfReclamationTask {
        fn mapped_mode(&self) -> Option<i32> {
            match self.mode.as_str() {
                "ProsperityNoSave" => Some(0),
                "ProsperityInSave" => Some(1),
                _ => None,
            }
        }

        pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
            if !self.is_enable {
                summary.disable_task("ReclamationTask", Some(self.name.clone()));
            }
            if self.mapped_mode().is_none() {
                summary.skip_field("ReclamationTask", Some(self.name.clone()), "Mode");
            }
            report_unknown_fields(
                summary,
                "ReclamationTask",
                Some(self.name.clone()),
                &self.unknown,
                HANDLED,
            );
        }

        pub(super) fn to_maa_value(&self) -> Result<MAAValue> {
            serialize_to_maa_value(&CliReclamationTask::try_from(self)?)
        }
    }

    impl TryFrom<&WpfReclamationTask> for CliReclamationTask {
        type Error = anyhow::Error;

        fn try_from(task: &WpfReclamationTask) -> Result<Self> {
            Ok(CliReclamationTask {
                task_type: TaskType::Reclamation,
                name: task.name.clone(),
                params: CliReclamationParams {
                    theme: Some(task.theme.clone()),
                    mode: task.mapped_mode(),
                    tools_to_craft: (!task.tool_to_craft.is_empty())
                        .then(|| vec![task.tool_to_craft.clone()]),
                    increment_mode: Some(task.increment_mode),
                    num_craft_batches: Some(task.max_craft_count_per_round),
                    enable: (!task.is_enable).then_some(false),
                },
            })
        }
    }
}
