use anyhow::{Context, Result, bail};
use chrono::Weekday;
use log::warn;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, report_unknown_fields};
use crate::config::task::{ClientType, Condition, TimeOffset};

const HANDLED: &[&str] = &[
    "UseWeeklySchedule",
    "WeeklySchedule",
    "UseOptionalStage",
    "StagePlan",
    "UseCustomAnnihilation",
    "AnnihilationStage",
    "UseMedicine",
    "MedicineCount",
    "UseStone",
    "StoneCount",
    "EnableTimesLimit",
    "TimesLimit",
    "EnableTargetDrop",
    "DropId",
    "DropCount",
    "Series",
    "UseExpiringMedicine",
    "MedicineExpireDays",
];

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
    series: i32,
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
            && self.series == i32::default()
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
/// [`Value`] via [`TryFrom`]. Unmapped keys are collected in `unknown`.
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

    /// When StagePlan is solely `Annihilation`, pick [`Self::annihilation_stage`] instead.
    #[serde(default)]
    use_custom_annihilation: bool,
    /// Specific annihilation map (`Chernobog@Annihilation`, …); empty when unused.
    #[serde(default)]
    annihilation_stage: String,

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

impl WpfStagePlan {
    /// GUI annihilation-only plan: a single stage named `Annihilation`.
    fn is_sole_annihilation(&self) -> bool {
        match self {
            Self::Single(stage) => stage == "Annihilation",
            Self::Many(stages) => stages.len() == 1 && stages[0] == "Annihilation",
        }
    }
}

impl WpfFightTask {
    pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
        if !self.is_enable {
            summary.disable_task("FightTask", Some(self.name.clone()));
        }
        report_unknown_fields(
            summary,
            "FightTask",
            Some(self.name.clone()),
            &self.unknown,
            HANDLED,
        );
    }

    /// When StagePlan is solely `Annihilation`, honour UseCustomAnnihilation /
    /// AnnihilationStage; otherwise keep the StagePlan entry as-is.
    fn resolve_stage<'a>(&'a self, stage: &'a str) -> Result<&'a str> {
        if !self.stage_plan.is_sole_annihilation() {
            return Ok(stage);
        }
        if self.use_custom_annihilation {
            if self.annihilation_stage.is_empty() {
                bail!("FightTask UseCustomAnnihilation is true but AnnihilationStage is empty");
            }
            Ok(self.annihilation_stage.as_str())
        } else {
            Ok("Annihilation")
        }
    }

    /// Build the open-condition for a fight stage name.
    ///
    /// - Resource stages with a weekly rotation → `Weekday` (server timezone Official)
    /// - Permanent stages (mainline, annihilation, LS-6, OF-*) → `Always`
    /// - Side-story stages listed in `StageActivityV2.json` → `DateTime` with that activity's
    ///   expire time (and timezone) from the resource file
    ///
    /// If the activity resource cannot be read (even after a hot-update attempt), migration
    /// fails instead of inventing a condition. Stages absent from a readable activity file
    /// still fall back to `Always` with a warning (e.g. mainline).
    fn stage_condition(stage: &str) -> Result<Condition> {
        use Weekday::*;
        Ok(match stage {
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
            // 永久开启（含指定剿灭图）
            "LS-6"
            | "Annihilation"
            | "OF-1"
            | "OF-F3"
            | "Chernobog@Annihilation"
            | "LungmenOutskirts@Annihilation"
            | "LungmenDowntown@Annihilation" => Condition::Always,
            _ => match Self::activity_stage_schedule(stage)? {
                Some(schedule) => Condition::DateTime {
                    start: None,
                    end: Some(schedule.end),
                    timezone: TimeOffset::TimeZone(schedule.timezone),
                },
                None => {
                    warn!(
                        "FightTask stage `{stage}` has no known open schedule; treating as Always"
                    );
                    Condition::Always
                }
            },
        })
    }

    /// Resolve expire window for an activity stage from `StageActivityV2.json`.
    ///
    /// On read failure, attempts a hot update once and reloads. Persistent read failure
    /// is an error so migration does not invent dates.
    fn activity_stage_schedule(stage: &str) -> Result<Option<crate::activity::SideStorySchedule>> {
        match crate::activity::load_side_story_stage_schedule(ClientType::Official, stage) {
            Ok(schedule) => Ok(schedule),
            Err(load_err) => {
                let path = maa_dirs::activity();
                warn!(
                    "Failed to read activity resource {}: {load_err}; trying hot update",
                    path.display()
                );
                crate::installer::hot_update::update().with_context(|| {
                    format!(
                        "Failed to update hot-update resources after failing to read {}",
                        path.display()
                    )
                })?;
                crate::activity::load_side_story_stage_schedule(ClientType::Official, stage)
                    .with_context(|| {
                        format!(
                            "Cannot read expire time for FightTask stage `{stage}` from {}",
                            path.display()
                        )
                    })
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
            series: task.series,
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
                let stage = task.resolve_stage(stage)?;
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
                let stage = task.resolve_stage(stage)?;
                let weekly = Condition::from(task.weekly_schedule.as_ref().context(
                    "FightTask UseWeeklySchedule is true but WeeklySchedule is missing or invalid",
                )?);
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
                let mut variants = Vec::with_capacity(stages.len());
                for stage in stages {
                    let stage = task.resolve_stage(stage)?;
                    variants.push(CliFightVariant {
                        condition: WpfFightTask::stage_condition(stage)?,
                        params: CliFightParams::stage_only(stage),
                    });
                }
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
                let weekly = Condition::from(task.weekly_schedule.as_ref().context(
                    "FightTask UseWeeklySchedule is true but WeeklySchedule is missing or invalid",
                )?);
                let mut variants = Vec::with_capacity(stages.len());
                for stage in stages {
                    let stage = task.resolve_stage(stage)?;
                    variants.push(CliFightVariant {
                        condition: Condition::And {
                            conditions: vec![weekly.clone(), WpfFightTask::stage_condition(stage)?],
                        },
                        params: CliFightParams::stage_only(stage),
                    });
                }
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

impl TryFrom<&WpfFightTask> for Value {
    type Error = anyhow::Error;

    fn try_from(task: &WpfFightTask) -> Result<Self> {
        Ok(serde_json::to_value(CliFightTask::try_from(task)?)?)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::super::{WpfConfig, migrate};

    fn fight_profile(extra: serde_json::Value) -> WpfConfig {
        let mut task = serde_json::json!({
            "$type": "FightTask",
            "Name": "剿灭",
            "IsEnable": true,
            "UseMedicine": false,
            "MedicineCount": 0,
            "UseStone": false,
            "StoneCount": 0,
            "EnableTargetDrop": false,
            "DropId": "",
            "DropCount": 0,
            "EnableTimesLimit": false,
            "TimesLimit": 0,
            "Series": 0,
            "UseExpiringMedicine": false,
            "MedicineExpireDays": 2,
            "UseOptionalStage": false,
            "UseWeeklySchedule": false,
            "StagePlan": ["Annihilation"],
        });
        if let Some(obj) = extra.as_object()
            && let Some(task_obj) = task.as_object_mut()
        {
            for (key, value) in obj {
                task_obj.insert(key.clone(), value.clone());
            }
        }
        serde_json::from_value(serde_json::json!({
            "TaskQueue": [task],
            "Gui": {},
        }))
        .unwrap()
    }

    #[test]
    fn custom_annihilation_uses_annihilation_stage() {
        let config = fight_profile(serde_json::json!({
            "UseCustomAnnihilation": true,
            "AnnihilationStage": "LungmenDowntown@Annihilation",
        }));
        let (cli, summary) = migrate(config, None).unwrap();
        assert!(
            !summary
                .skipped_fields
                .iter()
                .any(|f| f.field == "UseCustomAnnihilation" || f.field == "AnnihilationStage")
        );
        assert_eq!(
            cli.tasks[0].get("params").and_then(|p| p.get("stage")),
            Some(&serde_json::json!("LungmenDowntown@Annihilation"))
        );
    }

    #[test]
    fn custom_annihilation_off_keeps_annihilation() {
        let config = fight_profile(serde_json::json!({
            "UseCustomAnnihilation": false,
            "AnnihilationStage": "LungmenDowntown@Annihilation",
        }));
        let (cli, _) = migrate(config, None).unwrap();
        assert_eq!(
            cli.tasks[0].get("params").and_then(|p| p.get("stage")),
            Some(&serde_json::json!("Annihilation"))
        );
    }

    #[test]
    fn custom_annihilation_true_requires_stage() {
        let config = fight_profile(serde_json::json!({
            "UseCustomAnnihilation": true,
            "AnnihilationStage": "",
        }));
        let err = migrate(config, None).unwrap_err();
        assert!(
            err.to_string()
                .contains("UseCustomAnnihilation is true but AnnihilationStage is empty"),
            "{err}"
        );
    }
}
