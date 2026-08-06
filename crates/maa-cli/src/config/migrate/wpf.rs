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
    let config: Configuration =
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

#[derive(Debug, Serialize)]
struct CliConfig {
    tasks: Vec<MAAValue>,
}

#[derive(Debug, Deserialize)]
struct Configuration {
    #[serde(rename = "TaskQueue")]
    task_queue: Vec<WpfTask>,
    #[serde(rename = "Gui")]
    #[allow(dead_code)]
    gui: GuiSettings,
}

impl Configuration {
    fn migrate_tasks(&self, summary: &mut MigrationSummary) -> Result<Option<MAAValue>> {
        let mut tasks = Vec::new();
        for task in &self.task_queue {
            if let Some(item) = task.migrate_task(summary)? {
                tasks.push(item);
            }
        }
        Ok(Some(serialize_to_maa_value(&CliConfig { tasks })?))
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "$type")]
enum WpfTask {
    FightTask(fight::FightTask),
    /// Unknown `$type` values are skipped during migration.
    #[serde(other)]
    Unsupported,
}

impl WpfTask {
    fn migrate_task(&self, summary: &mut MigrationSummary) -> Result<Option<MAAValue>> {
        match self {
            Self::FightTask(fight) => {
                fight.report_disabled(summary);
                Ok(Some(MAAValue::try_from(fight)?))
            }
            Self::Unsupported => {
                summary.skip_task("Unsupported", None);
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct GuiSettings {}

mod start_up {
    // use anyhow::Result;
    // use log::warn;
    // use maa_value::prelude::*;

    // use super::MigrationSummary;

    // /// Valid GUI / maa-cli client type strings.
    // const VALID_CLIENT_TYPES: &[&str] = &[
    //     "Official", "Bilibili", "txwy", "YoStarEN", "YoStarJP", "YoStarKR",
    // ];

    #[allow(dead_code)]
    pub(super) struct StartUpTask {}

    // pub(super) fn migrate_start_up_task(
    //     task: &MAAValue,
    //     config: &MAAValue,
    //     summary: &mut MigrationSummary,
    // ) -> Result<Option<MAAValue>> {
    //     let runtime = config.get("Gui").and_then(|gui| gui.get("RuntimeSettings"));

    //     let mut params = MAAValue::default();

    //     // Gui.RuntimeSettings.ClientType -> client_type
    //     // Missing / invalid values fall back to an interactive prompt.
    //     match runtime
    //         .and_then(|settings| settings.get("ClientType"))
    //         .and_then(|value| value.as_str())
    //         .filter(|value| VALID_CLIENT_TYPES.contains(value))
    //     {
    //         Some(client_type) => {
    //             insert!(params, "client_type" => client_type);
    //         }
    //         None => {
    //             insert!(params, "client_type" => object!(
    //                 "alternatives" => VALID_CLIENT_TYPES.iter().map(|s|
    // s.to_string()).collect::<Vec<_>>()??,             ));
    //         }
    //     }

    //     // Gui.RuntimeSettings.StartGame -> start_game_enabled
    //     if let Some(MAAValue::Primitive(MAAPrimitive::Bool(start_game))) =
    //         runtime.and_then(|settings| settings.get("StartGame"))
    //     {
    //         insert!(params, "start_game_enabled" => *start_game);
    //     }

    //     // AccountSwitchEnabled + AccountName -> account_name
    //     // Only emit account_name when account switching is enabled.
    //     if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) =
    //         task.get("AccountSwitchEnabled")
    //     {
    //         if let Some(MAAValue::Primitive(MAAPrimitive::String(account_name))) =
    //             task.get("AccountName")
    //         {
    //             insert!(params, "account_name" => account_name.as_str());
    //         } else {
    //             summary.skip_field("StartUpTask", None, "AccountName");
    //             warn!("AccountName is missing, but GUI enable account switching");
    //         }
    //     }

    //     let item = object!(
    //         "type" => "StartUp",
    //         "params" => params
    //     );

    //     Ok(Some(item))
    // }
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
    struct FightCliTask {
        #[serde(rename = "type")]
        task_type: TaskType,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<FightParams>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        variants: Vec<FightVariant>,
    }

    #[derive(Debug, Serialize)]
    struct FightVariant {
        condition: Condition,
        params: FightParams,
    }

    /// Fight params; field order matches historical `object!` / `insert!` output for parity.
    #[derive(Clone, Debug, Default, Serialize)]
    struct FightParams {
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

    /// WPF GUI `FightTask` (`$type = "FightTask"`).
    ///
    /// Deserialized from the GUI task queue; converts into [`FightCliTask`] /
    /// [`MAAValue`] via [`TryFrom`]. Unmapped keys are collected in `unknown`.
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub(super) struct FightTask {
        name: String,
        is_enable: bool,

        use_weekly_schedule: bool,
        /// Absent when `UseWeeklySchedule` is false in typical GUI exports.
        #[serde(default)]
        weekly_schedule: Option<WeeklySchedule>,
        use_optional_stage: bool,
        stage_plan: StagePlan,

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
    struct WeeklySchedule {
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
    enum StagePlan {
        Single(String),
        Many(Vec<String>),
    }

    impl FightTask {
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

    impl From<&FightTask> for FightParams {
        fn from(task: &FightTask) -> Self {
            FightParams {
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

    impl From<&WeeklySchedule> for Condition {
        fn from(schedule: &WeeklySchedule) -> Self {
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

    impl<'a> TryFrom<&'a StagePlan> for &'a str {
        type Error = anyhow::Error;

        fn try_from(plan: &'a StagePlan) -> Result<&'a str> {
            match plan {
                StagePlan::Single(stage) => Ok(stage.as_str()),
                StagePlan::Many(stages) if stages.len() == 1 => Ok(stages[0].as_str()),
                StagePlan::Many(_) => {
                    bail!(
                        "FightTask StagePlan must be a single stage string when UseOptionalStage is false, \
                         got a string array; enable UseOptionalStage for multiple stages"
                    )
                }
            }
        }
    }

    impl<'a> TryFrom<&'a StagePlan> for Vec<&'a str> {
        type Error = anyhow::Error;

        fn try_from(plan: &'a StagePlan) -> Result<Vec<&'a str>> {
            match plan {
                StagePlan::Many(stages) => {
                    if stages.is_empty() {
                        bail!(
                            "FightTask StagePlan must be a non-empty string array when UseOptionalStage is true"
                        );
                    }
                    Ok(stages.iter().map(String::as_str).collect())
                }
                StagePlan::Single(_) => {
                    bail!(
                        "FightTask StagePlan must be a string array when UseOptionalStage is true, got a string"
                    )
                }
            }
        }
    }

    impl TryFrom<&FightTask> for FightCliTask {
        type Error = anyhow::Error;

        fn try_from(task: &FightTask) -> Result<Self> {
            let shared_params = FightParams::from(task);

            let mut item = match (task.use_weekly_schedule, task.use_optional_stage) {
                // No variants: StagePlan must be a single stage.
                (false, false) => {
                    let stage: &str = (&task.stage_plan).try_into()?;
                    let mut params = shared_params;
                    params.stage = Some(stage.to_string());
                    FightCliTask {
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
                    let mut params = shared_params;
                    params.stage = Some(stage.to_string());
                    FightCliTask {
                        task_type: TaskType::Fight,
                        name: task.name.clone(),
                        params: None,
                        variants: vec![FightVariant {
                            condition: weekly,
                            params,
                        }],
                    }
                }
                // One variant per optional stage, each with its own open-condition.
                (false, true) => {
                    let stages: Vec<&str> = (&task.stage_plan).try_into()?;
                    let mut variants = Vec::with_capacity(stages.len());
                    for stage in stages {
                        let mut params = shared_params.clone();
                        params.stage = Some(stage.to_string());
                        variants.push(FightVariant {
                            condition: FightTask::stage_condition(stage),
                            params,
                        });
                    }
                    FightCliTask {
                        task_type: TaskType::Fight,
                        name: task.name.clone(),
                        params: None,
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
                    let mut variants = Vec::with_capacity(stages.len());
                    for stage in stages {
                        let mut params = shared_params.clone();
                        params.stage = Some(stage.to_string());
                        variants.push(FightVariant {
                            condition: Condition::And {
                                conditions: vec![weekly.clone(), FightTask::stage_condition(stage)],
                            },
                            params,
                        });
                    }
                    FightCliTask {
                        task_type: TaskType::Fight,
                        name: task.name.clone(),
                        params: None,
                        variants,
                    }
                }
            };

            // MaaCore common switch: params.enable=false disables the task.
            if !task.is_enable {
                match &mut item.params {
                    Some(params) => params.enable = Some(false),
                    None => {
                        item.params = Some(FightParams {
                            enable: Some(false),
                            ..FightParams::default()
                        });
                    }
                }
            }

            Ok(item)
        }
    }

    impl TryFrom<&FightTask> for MAAValue {
        type Error = anyhow::Error;

        fn try_from(task: &FightTask) -> Result<Self> {
            serialize_to_maa_value(&FightCliTask::try_from(task)?)
        }
    }
}

// mod infrast {
//     use anyhow::{Result, bail};
//     use maa_value::prelude::*;

//     use super::{MigrationSummary, report_unhandled_fields};

//     pub(super) fn migrate_infrast_task(
//         task: &MAAValue,
//         summary: &mut MigrationSummary,
//     ) -> Result<Option<MAAValue>> {
//         // Custom infrastructure plans are not migrated yet.
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode")
//             && mode == "Custom"
//         {
//             bail!("InfrastTask custom mode is not supported yet");
//         }
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(filename))) = task.get("Filename")
//             && !filename.is_empty()
//         {
//             bail!("InfrastTask custom plan (Filename) is not supported yet");
//         }

//         let mut item = object!("type" => "Infrast");
//         // -> task name
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
//             insert!(item, "name" => name.as_str());
//         }

//         let mut params = MAAValue::default();
//         let mut handled = vec![
//             "Mode",
//             "RoomList",
//             "UsesOfDrones",
//             "DormThreshold",
//             "OriginiumShardAutoReplenishment",
//             "DormFilterNotStationed",
//             "DormTrustEnabled",
//             "ReceptionMessageBoard",
//             "ReceptionClueExchange",
//             "SendClue",
//             // Custom plan fields: rejected above when set; keep handled so defaults are quiet.
//             "Filename",
//             "PlanSelect",
//         ];
//         // Mode -> mode (Custom is rejected above)
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode") {
//             if let Some(mode) = match mode.as_str() {
//                 "Normal" => Some(0),
//                 "Rotation" => Some(20000),
//                 _ => None,
//             } {
//                 insert!(params, "mode" => mode);
//             } else {
//                 handled.retain(|field| *field != "Mode");
//             }
//         }
//         // RoomList -> facility
//         if let Some(MAAValue::Array(rooms)) = task.get("RoomList") {
//             let mut facility = Vec::new();
//             for room in rooms {
//                 if let MAAValue::Object(map) = room
//                     && let Some(MAAValue::Primitive(MAAPrimitive::String(room))) =
// map.get("Room")                 {
//                     facility.push(room.as_str());
//                 }
//             }
//             if !facility.is_empty() {
//                 insert!(params, "facility" => facility??);
//             }
//         }
//         // UsesOfDrones -> drones
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(drones))) = task.get("UsesOfDrones")
// {             insert!(params, "drones" => drones.as_str());
//         }
//         // DormThreshold -> threshold
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(threshold))) =
// task.get("DormThreshold") {             insert!(params, "threshold" => *threshold as f32 /
// 100.0);         }
//         // OriginiumShardAutoReplenishment -> replenish
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(replenish))) =
//             task.get("OriginiumShardAutoReplenishment")
//         {
//             insert!(params, "replenish" => *replenish);
//         }
//         // DormFilterNotStationed -> dorm_notstationed_enabled
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) =
//             task.get("DormFilterNotStationed")
//         {
//             insert!(params, "dorm_notstationed_enabled" => *enabled);
//         }
//         // DormTrustEnabled -> dorm_trust_enabled
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) =
// task.get("DormTrustEnabled")         {
//             insert!(params, "dorm_trust_enabled" => *enabled);
//         }
//         // ReceptionMessageBoard -> reception_message_board
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) =
//             task.get("ReceptionMessageBoard")
//         {
//             insert!(params, "reception_message_board" => *enabled);
//         }
//         // ReceptionClueExchange -> reception_clue_exchange
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) =
//             task.get("ReceptionClueExchange")
//         {
//             insert!(params, "reception_clue_exchange" => *enabled);
//         }
//         // SendClue -> reception_send_clue
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) = task.get("SendClue") {
//             insert!(params, "reception_send_clue" => *enabled);
//         }

//         insert!(item, "params" => params);
//         report_unhandled_fields(summary, task, "InfrastTask", &handled);
//         Ok(Some(item))
//     }
// }

// mod recruit {
//     use anyhow::Result;
//     use maa_value::prelude::*;

//     use super::{MigrationSummary, report_unhandled_fields};

//     pub(super) fn migrate_recruit_task(
//         task: &MAAValue,
//         summary: &mut MigrationSummary,
//     ) -> Result<Option<MAAValue>> {
//         let mut item = object!("type" => "Recruit");
//         // -> task name
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
//             insert!(item, "name" => name.as_str());
//         }

//         let mut params = MAAValue::default();
//         // MaxTimes -> times
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(times))) = task.get("MaxTimes") {
//             insert!(params, "times" => *times);
//         }
//         // ExtraTagMode -> extra_tags_mode
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(mode))) = task.get("ExtraTagMode") {
//             insert!(params, "extra_tags_mode" => *mode);
//         }
//         // RefreshLevel3 -> refresh
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(refresh))) = task.get("RefreshLevel3")
// {             insert!(params, "refresh" => *refresh);
//         }
//         // ForceRefresh -> expedite
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(expedite))) = task.get("ForceRefresh")
// {             insert!(params, "expedite" => *expedite);
//         }
//         // LevelXChoose -> select / confirm
//         let mut select = Vec::new();
//         let mut confirm = Vec::new();
//         let mut recruitment_time = MAAValue::default();
//         const LEVELS: [(&str, &str, i32); 4] = [
//             ("Level6Choose", "Level6Time", 6),
//             ("Level5Choose", "Level5Time", 5),
//             ("Level4Choose", "Level4Time", 4),
//             ("Level3Choose", "Level3Time", 3),
//         ];
//         for (choose_key, time_key, level) in LEVELS {
//             if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get(choose_key) {
//                 select.push(level);
//                 confirm.push(level);
//             }
//             if let Some(MAAValue::Primitive(MAAPrimitive::Int(minutes))) = task.get(time_key) {
//                 recruitment_time.insert(level.to_string(), (*minutes).into());
//             }
//         }
//         if !select.is_empty() {
//             insert!(params, "select" => select??);
//             insert!(params, "confirm" => confirm??);
//         }
//         if recruitment_time.as_map().is_some_and(|map| !map.is_empty()) {
//             insert!(params, "recruitment_time" => recruitment_time);
//         }
//         // PreferTagEnabled + Level3PreferTags -> first_tags
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get("PreferTagEnabled")
//             && let Some(MAAValue::Array(tags)) = task.get("Level3PreferTags")
//         {
//             let mut first_tags = Vec::new();
//             for tag in tags {
//                 if let MAAValue::Primitive(MAAPrimitive::String(tag)) = tag {
//                     first_tags.push(tag.as_str());
//                 }
//             }
//             if !first_tags.is_empty() {
//                 insert!(params, "first_tags" => first_tags??);
//             }
//         }
//         // PreserveTagEnabled + PreserveTagList -> preserve_tags
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) =
// task.get("PreserveTagEnabled")             && let Some(MAAValue::Array(tags)) =
// task.get("PreserveTagList")         {
//             let mut preserve_tags = Vec::new();
//             for tag in tags {
//                 if let MAAValue::Primitive(MAAPrimitive::String(tag)) = tag {
//                     preserve_tags.push(tag.as_str());
//                 }
//             }
//             if !preserve_tags.is_empty() {
//                 insert!(params, "preserve_tags" => preserve_tags??);
//             }
//         }

//         insert!(item, "params" => params);
//         report_unhandled_fields(summary, task, "RecruitTask", &[
//             "MaxTimes",
//             "ExtraTagMode",
//             "RefreshLevel3",
//             "ForceRefresh",
//             "Level6Choose",
//             "Level6Time",
//             "Level5Choose",
//             "Level5Time",
//             "Level4Choose",
//             "Level4Time",
//             "Level3Choose",
//             "Level3Time",
//             "PreferTagEnabled",
//             "Level3PreferTags",
//             "PreserveTagEnabled",
//             "PreserveTagList",
//         ]);
//         Ok(Some(item))
//     }
// }

// mod mall {
//     use anyhow::Result;
//     use maa_value::prelude::*;

//     use super::{MigrationSummary, report_unhandled_fields};

//     pub(super) fn migrate_mall_task(
//         task: &MAAValue,
//         summary: &mut MigrationSummary,
//     ) -> Result<Option<MAAValue>> {
//         let mut item = object!("type" => "Mall");
//         // -> task name
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
//             insert!(item, "name" => name.as_str());
//         }

//         let mut params = MAAValue::default();
//         // Shopping -> shopping
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(shopping))) = task.get("Shopping") {
//             insert!(params, "shopping" => *shopping);
//         }
//         // CreditFight -> credit_fight
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(credit_fight))) =
// task.get("CreditFight")         {
//             insert!(params, "credit_fight" => *credit_fight);
//         }
//         // CreditFightFormation -> formation_index
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(formation_index))) =
//             task.get("CreditFightFormation")
//         {
//             insert!(params, "formation_index" => *formation_index);
//         }
//         // VisitFriends -> visit_friends
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(visit_friends))) =
//             task.get("VisitFriends")
//         {
//             insert!(params, "visit_friends" => *visit_friends);
//         }
//         // FirstList -> buy_first
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(list))) = task.get("FirstList") {
//             let buy_first: Vec<_> = list.split(';').filter(|item| !item.is_empty()).collect();
//             if !buy_first.is_empty() {
//                 insert!(params, "buy_first" => buy_first??);
//             }
//         }
//         // BlackList -> blacklist
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(list))) = task.get("BlackList") {
//             let blacklist: Vec<_> = list.split(';').filter(|item| !item.is_empty()).collect();
//             if !blacklist.is_empty() {
//                 insert!(params, "blacklist" => blacklist??);
//             }
//         }
//         // ShoppingIgnoreBlackListWhenFull -> force_shopping_if_credit_full
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(force))) =
//             task.get("ShoppingIgnoreBlackListWhenFull")
//         {
//             insert!(params, "force_shopping_if_credit_full" => *force);
//         }
//         // OnlyBuyDiscount -> only_buy_discount
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(only_discount))) =
//             task.get("OnlyBuyDiscount")
//         {
//             insert!(params, "only_buy_discount" => *only_discount);
//         }
//         // ReserveMaxCredit -> reserve_max_credit
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(reserve))) =
// task.get("ReserveMaxCredit")         {
//             insert!(params, "reserve_max_credit" => *reserve);
//         }

//         insert!(item, "params" => params);
//         report_unhandled_fields(summary, task, "MallTask", &[
//             "Shopping",
//             "CreditFight",
//             "CreditFightFormation",
//             "VisitFriends",
//             "FirstList",
//             "BlackList",
//             "ShoppingIgnoreBlackListWhenFull",
//             "OnlyBuyDiscount",
//             "ReserveMaxCredit",
//         ]);
//         Ok(Some(item))
//     }
// }

// mod award {
//     use anyhow::Result;
//     use maa_value::prelude::*;

//     use super::{MigrationSummary, report_unhandled_fields};

//     pub(super) fn migrate_award_task(
//         task: &MAAValue,
//         summary: &mut MigrationSummary,
//     ) -> Result<Option<MAAValue>> {
//         let mut item = object!("type" => "Award");
//         // -> task name
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
//             insert!(item, "name" => name.as_str());
//         }

//         let mut params = MAAValue::default();
//         // Award -> award
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(award))) = task.get("Award") {
//             insert!(params, "award" => *award);
//         }
//         // Mail -> mail
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(mail))) = task.get("Mail") {
//             insert!(params, "mail" => *mail);
//         }
//         // FreeGacha -> recruit
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(recruit))) = task.get("FreeGacha") {
//             insert!(params, "recruit" => *recruit);
//         }
//         // Orundum -> orundum
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(orundum))) = task.get("Orundum") {
//             insert!(params, "orundum" => *orundum);
//         }
//         // Mining -> mining
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(mining))) = task.get("Mining") {
//             insert!(params, "mining" => *mining);
//         }
//         // SpecialAccess -> specialaccess
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(specialaccess))) =
//             task.get("SpecialAccess")
//         {
//             insert!(params, "specialaccess" => *specialaccess);
//         }

//         insert!(item, "params" => params);
//         report_unhandled_fields(summary, task, "AwardTask", &[
//             "Award",
//             "Mail",
//             "FreeGacha",
//             "Orundum",
//             "Mining",
//             "SpecialAccess",
//         ]);
//         Ok(Some(item))
//     }
// }

// mod roguelike {
//     use anyhow::Result;
//     use maa_value::prelude::*;

//     use super::{MigrationSummary, report_unhandled_fields};

//     pub(super) fn migrate_roguelike_task(
//         task: &MAAValue,
//         summary: &mut MigrationSummary,
//     ) -> Result<Option<MAAValue>> {
//         let mut item = object!("type" => "Roguelike");
//         // -> task name
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
//             insert!(item, "name" => name.as_str());
//         }

//         let mut params = MAAValue::default();
//         let mut handled = vec![
//             "Theme",
//             "Mode",
//             "Squad",
//             "Roles",
//             "CoreChar",
//             "StartCount",
//             "Difficulty",
//             "Investment",
//             "InvestCount",
//             "InvestWithMoreScore",
//             "StopWhenDepositFull",
//             "StopAtFinalBoss",
//             "StopWhenLevelMax",
//             "UseSupport",
//             "UseSupportNonFriend",
//             "RefreshTraderWithDice",
//             "StartWithEliteTwo",
//             "StartWithEliteTwoOnly",
//         ];
//         // Theme -> theme
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(theme))) = task.get("Theme") {
//             insert!(params, "theme" => theme.as_str());
//         }
//         // Mode -> mode
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode") {
//             if let Some(mode) = match mode.as_str() {
//                 "Exp" => Some(0),
//                 "Investment" => Some(1),
//                 "Collect" => Some(4),
//                 "CollapsalParadigms" => Some(5),
//                 "MonthlySquad" => Some(6),
//                 "DeepExploration" => Some(7),
//                 _ => None,
//             } {
//                 insert!(params, "mode" => mode);
//             } else {
//                 handled.retain(|field| *field != "Mode");
//             }
//         }
//         // Squad -> squad
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(squad))) = task.get("Squad") {
//             insert!(params, "squad" => squad.as_str());
//         }
//         // Roles -> roles
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(roles))) = task.get("Roles") {
//             insert!(params, "roles" => roles.as_str());
//         }
//         // CoreChar -> core_char
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(core_char))) = task.get("CoreChar")
// {             insert!(params, "core_char" => core_char.as_str());
//         }
//         // StartCount -> starts_count
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(starts_count))) =
// task.get("StartCount") {             insert!(params, "starts_count" => *starts_count);
//         }
//         // Difficulty -> difficulty
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(difficulty))) = task.get("Difficulty")
//             && *difficulty != i32::MAX
//         {
//             insert!(params, "difficulty" => *difficulty);
//         }
//         // Investment -> investment_enabled
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(investment_enabled))) =
//             task.get("Investment")
//         {
//             insert!(params, "investment_enabled" => *investment_enabled);
//         }
//         // InvestCount -> investments_count
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(investments_count))) =
//             task.get("InvestCount")
//         {
//             insert!(params, "investments_count" => *investments_count);
//         }
//         // InvestWithMoreScore -> investment_with_more_score
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(investment_with_more_score))) =
//             task.get("InvestWithMoreScore")
//         {
//             insert!(params, "investment_with_more_score" => *investment_with_more_score);
//         }
//         // StopWhenDepositFull -> stop_when_investment_full
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(stop_when_investment_full))) =
//             task.get("StopWhenDepositFull")
//         {
//             insert!(params, "stop_when_investment_full" => *stop_when_investment_full);
//         }
//         // StopAtFinalBoss -> stop_at_final_boss
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(stop_at_final_boss))) =
//             task.get("StopAtFinalBoss")
//         {
//             insert!(params, "stop_at_final_boss" => *stop_at_final_boss);
//         }
//         // StopWhenLevelMax -> stop_at_max_level
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(stop_at_max_level))) =
//             task.get("StopWhenLevelMax")
//         {
//             insert!(params, "stop_at_max_level" => *stop_at_max_level);
//         }
//         // UseSupport -> use_support
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(use_support))) =
// task.get("UseSupport") {             insert!(params, "use_support" => *use_support);
//         }
//         // UseSupportNonFriend -> use_nonfriend_support
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(use_nonfriend_support))) =
//             task.get("UseSupportNonFriend")
//         {
//             insert!(params, "use_nonfriend_support" => *use_nonfriend_support);
//         }
//         // RefreshTraderWithDice -> refresh_trader_with_dice
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(refresh_trader_with_dice))) =
//             task.get("RefreshTraderWithDice")
//         {
//             insert!(params, "refresh_trader_with_dice" => *refresh_trader_with_dice);
//         }
//         // StartWithEliteTwo -> start_with_elite_two
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(start_with_elite_two))) =
//             task.get("StartWithEliteTwo")
//         {
//             insert!(params, "start_with_elite_two" => *start_with_elite_two);
//         }
//         // StartWithEliteTwoOnly -> only_start_with_elite_two
//         if let Some(MAAValue::Primitive(MAAPrimitive::Bool(only_start_with_elite_two))) =
//             task.get("StartWithEliteTwoOnly")
//         {
//             insert!(params, "only_start_with_elite_two" => *only_start_with_elite_two);
//         }

//         insert!(item, "params" => params);
//         report_unhandled_fields(summary, task, "RoguelikeTask", &handled);
//         Ok(Some(item))
//     }
// }

// mod reclamation {
//     use anyhow::Result;
//     use maa_value::prelude::*;

//     use super::{MigrationSummary, report_unhandled_fields};

//     pub(super) fn migrate_reclamation_task(
//         task: &MAAValue,
//         summary: &mut MigrationSummary,
//     ) -> Result<Option<MAAValue>> {
//         let mut item = object!("type" => "Reclamation");
//         // -> task name
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
//             insert!(item, "name" => name.as_str());
//         }

//         let mut params = MAAValue::default();
//         let mut handled = vec![
//             "Theme",
//             "Mode",
//             "ToolToCraft",
//             "IncrementMode",
//             "MaxCraftCountPerRound",
//         ];
//         // Theme -> theme
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(theme))) = task.get("Theme") {
//             insert!(params, "theme" => theme.as_str());
//         }
//         // Mode -> mode
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode") {
//             if let Some(mode) = match mode.as_str() {
//                 "ProsperityNoSave" => Some(0),
//                 "ProsperityInSave" => Some(1),
//                 _ => None,
//             } {
//                 insert!(params, "mode" => mode);
//             } else {
//                 handled.retain(|field| *field != "Mode");
//             }
//         }
//         // ToolToCraft -> tools_to_craft
//         if let Some(MAAValue::Primitive(MAAPrimitive::String(tool))) = task.get("ToolToCraft")
//             && !tool.is_empty()
//         {
//             insert!(params, "tools_to_craft" => vec![tool.as_str()]??);
//         }
//         // IncrementMode -> increment_mode
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(increment_mode))) =
//             task.get("IncrementMode")
//         {
//             insert!(params, "increment_mode" => *increment_mode);
//         }
//         // MaxCraftCountPerRound -> num_craft_batches
//         if let Some(MAAValue::Primitive(MAAPrimitive::Int(num_craft_batches))) =
//             task.get("MaxCraftCountPerRound")
//         {
//             insert!(params, "num_craft_batches" => *num_craft_batches);
//         }

//         insert!(item, "params" => params);
//         report_unhandled_fields(summary, task, "ReclamationTask", &handled);
//         Ok(Some(item))
//     }
// }
