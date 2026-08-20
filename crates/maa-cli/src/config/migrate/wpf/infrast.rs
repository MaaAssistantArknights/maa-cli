use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use chrono::{NaiveTime, Timelike};
use log::warn;
use maa_dirs::expand_tilde;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, report_unknown_fields};
use crate::config::task::{Condition, TimeOffset};

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
    "ContinueTraining",
    "CustomFileType",
    "Filename",
    "PlanSelect",
];

const MINUTES_PER_DAY: u32 = 24 * 60;
const CUSTOM_MODE: i32 = 10000;

#[derive(Debug, Serialize)]
struct CliInfrastTask {
    #[serde(rename = "type")]
    task_type: TaskType,
    #[serde(skip_serializing_if = "str::is_empty")]
    name: String,
    params: CliInfrastParams,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    variants: Vec<CliInfrastVariant>,
}

#[derive(Debug, Serialize)]
struct CliInfrastVariant {
    condition: Condition,
    params: CliInfrastVariantParams,
}

#[derive(Debug, Serialize)]
struct CliInfrastVariantParams {
    plan_index: i32,
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
    threshold: Option<f64>,
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
    continue_training: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_index: Option<i32>,
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
    #[serde(default = "default_plan_select")]
    plan_select: i32,
    #[serde(default)]
    continue_training: bool,
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

fn default_plan_select() -> i32 {
    -1
}

#[derive(Debug, Deserialize)]
struct CustomInfrastFile {
    plans: Vec<CustomInfrastPlan>,
}

#[derive(Debug, Deserialize)]
struct CustomInfrastPlan {
    #[serde(default)]
    period: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MergedSpan {
    /// `[start, end)` on a single day; `end == MINUTES_PER_DAY` means until midnight.
    NonWrap { start: u32, end: u32 },
    /// `[start, 24:00)` plus `[00:00, end)`.
    Wrap { start: u32, end: u32 },
}

impl WpfInfrastTask {
    fn mapped_mode(&self) -> Option<i32> {
        match self.mode.as_str() {
            "Normal" => Some(0),
            "Rotation" => Some(20000),
            _ => None,
        }
    }

    fn wants_custom(&self) -> bool {
        self.mode == "Custom" || (!self.filename.is_empty() && self.mode != "Rotation")
    }

    pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
        if !self.is_enable {
            summary.disable_task("InfrastTask", Some(self.name.clone()));
        }
        if !self.wants_custom() && self.mapped_mode().is_none() {
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

    pub(super) fn to_cli_task(
        &self,
        summary: &mut MigrationSummary,
        custom_schedule: Option<&Path>,
        schedule_applied: &mut bool,
    ) -> Result<impl Serialize> {
        if let Some(path) = custom_schedule
            && self.wants_custom()
        {
            *schedule_applied = true;
            return self.try_custom_task(&path.to_string_lossy());
        }

        if self.wants_custom() {
            match self.try_custom_task(&self.filename) {
                Ok(task) => return Ok(task),
                Err(err) => {
                    warn!(
                        "InfrastTask custom schedule skipped ({err}); writing default infrast mode"
                    );
                    summary.skip_field("InfrastTask", Some(self.name.clone()), "Mode");
                    if !self.filename.is_empty() {
                        summary.skip_field("InfrastTask", Some(self.name.clone()), "Filename");
                    }
                }
            }
        }
        Ok(self.default_task())
    }

    fn default_task(&self) -> CliInfrastTask {
        let mode = if self.wants_custom() {
            Some(0)
        } else {
            self.mapped_mode()
        };
        CliInfrastTask {
            task_type: TaskType::Infrast,
            name: self.name.clone(),
            params: self.shared_params(mode, None, None),
            variants: Vec::new(),
        }
    }

    fn shared_params(
        &self,
        mode: Option<i32>,
        filename: Option<String>,
        plan_index: Option<i32>,
    ) -> CliInfrastParams {
        let custom = mode == Some(CUSTOM_MODE);
        CliInfrastParams {
            mode,
            facility: self.room_list.iter().map(|r| r.room.clone()).collect(),
            drones: (!custom).then(|| self.uses_of_drones.clone()),
            // Prefer f64 so TOML/JSON emit `0.3` instead of the f32 binary expansion.
            threshold: Some(f64::from(self.dorm_threshold) / 100.0),
            replenish: Some(self.originium_shard_auto_replenishment),
            dorm_notstationed_enabled: Some(self.dorm_filter_not_stationed),
            dorm_trust_enabled: Some(self.dorm_trust_enabled),
            reception_message_board: Some(self.reception_message_board),
            reception_clue_exchange: Some(self.reception_clue_exchange),
            reception_send_clue: Some(self.send_clue),
            continue_training: Some(self.continue_training),
            filename,
            plan_index,
            enable: (!self.is_enable).then_some(false),
        }
    }

    fn try_custom_task(&self, filename: &str) -> Result<CliInfrastTask> {
        ensure!(!filename.is_empty(), "custom mode has empty Filename");
        let filename = resolve_infrast_filename(filename);
        let schedule = load_custom_schedule(&filename)?;
        let n_plans =
            i32::try_from(schedule.plans.len()).context("too many plans in custom infrast file")?;

        let time_rotation = self.plan_select < 0;
        let variants = if time_rotation {
            match variants_from_schedule(&schedule) {
                Ok(variants) => variants,
                Err(err) => {
                    warn!(
                        "InfrastTask custom schedule periods could not be converted ({err}); using plan_index"
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        let plan_index = if variants.is_empty() {
            warn!(
                "InfrastTask: custom schedule with {} plan(s); no period data, using plan_index",
                schedule.plans.len()
            );
            let in_range = self.plan_select >= 0 && self.plan_select < n_plans;
            Some(if in_range { self.plan_select } else { 0 })
        } else {
            let days = variants
                .iter()
                .filter_map(|v| match &v.condition {
                    Condition::And { conditions } | Condition::Or { conditions } => {
                        conditions.iter().find_map(|c| match c {
                            Condition::DayMod { divisor, .. } => Some(*divisor),
                            Condition::And { conditions } => {
                                conditions.iter().find_map(|inner| match inner {
                                    Condition::DayMod { divisor, .. } => Some(*divisor),
                                    _ => None,
                                })
                            }
                            _ => None,
                        })
                    }
                    Condition::DayMod { divisor, .. } => Some(*divisor),
                    _ => None,
                })
                .max()
                .unwrap_or(1);
            let times: Vec<&str> = schedule
                .plans
                .iter()
                .filter_map(|plan| plan.period.first())
                .filter_map(|pair| pair.first())
                .map(String::as_str)
                .collect();
            warn!(
                "InfrastTask: custom schedule ({days} day(s), {} shift(s)); suggested run times: {}",
                schedule.plans.len(),
                times.join(", ")
            );
            None
        };

        Ok(CliInfrastTask {
            task_type: TaskType::Infrast,
            name: self.name.clone(),
            params: self.shared_params(
                Some(CUSTOM_MODE),
                Some(filename.to_string_lossy().into_owned()),
                plan_index,
            ),
            variants,
        })
    }
}

fn resolve_infrast_filename(path: &str) -> PathBuf {
    expand_tilde(Path::new(path)).into_owned()
}

fn load_custom_schedule(path: &Path) -> Result<CustomInfrastFile> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read custom infrast file `{}`", path.display()))?;
    let file: CustomInfrastFile = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse custom infrast file `{}`", path.display()))?;
    ensure!(!file.plans.is_empty(), "custom infrast file has no plans");
    Ok(file)
}

fn variants_from_schedule(file: &CustomInfrastFile) -> Result<Vec<CliInfrastVariant>> {
    let has_period: Vec<bool> = file.plans.iter().map(|p| !p.period.is_empty()).collect();
    if has_period.iter().any(|has| !*has) {
        if has_period.iter().any(|has| *has) {
            bail!("some plans have period and some do not");
        }
        return Ok(Vec::new());
    }

    let spans: Vec<MergedSpan> = file
        .plans
        .iter()
        .map(|plan| merge_plan_periods(&plan.period))
        .collect::<Result<_>>()?;
    let remainders = assign_day_remainders(&spans);
    let days = remainders.iter().copied().max().unwrap_or(0) + 1;

    spans
        .iter()
        .zip(remainders)
        .enumerate()
        .map(|(index, (span, remainder))| {
            Ok(CliInfrastVariant {
                condition: span_condition(*span, days, remainder),
                params: CliInfrastVariantParams {
                    plan_index: i32::try_from(index)?,
                },
            })
        })
        .collect()
}

fn parse_clock(value: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
        .with_context(|| format!("invalid time `{value}` in custom infrast period"))
}

fn time_to_minutes(time: NaiveTime) -> u32 {
    time.num_seconds_from_midnight() / 60
}

fn minutes_to_time(minutes: u32) -> Option<NaiveTime> {
    if minutes >= MINUTES_PER_DAY {
        None
    } else {
        NaiveTime::from_hms_opt(minutes / 60, minutes % 60, 0)
    }
}

fn period_to_segments(start: NaiveTime, end: NaiveTime) -> Vec<(u32, u32)> {
    let start_m = time_to_minutes(start);
    let mut end_m = time_to_minutes(end);
    if end.hour() == 23 && end.minute() == 59 {
        end_m = MINUTES_PER_DAY;
    }
    if start_m == end_m {
        Vec::new()
    } else if start_m < end_m {
        vec![(start_m, end_m)]
    } else {
        vec![(start_m, MINUTES_PER_DAY), (0, end_m)]
    }
}

fn merge_plan_periods(period: &[Vec<String>]) -> Result<MergedSpan> {
    let mut segments = Vec::new();
    for entry in period {
        ensure!(
            entry.len() == 2,
            "period entry must be [start, end], got {entry:?}"
        );
        let start = parse_clock(&entry[0])?;
        let end = parse_clock(&entry[1])?;
        segments.extend(period_to_segments(start, end));
    }
    ensure!(!segments.is_empty(), "plan period is empty after parsing");
    segments.sort_by_key(|seg| seg.0);

    let mut merged: Vec<(u32, u32)> = Vec::new();
    for (start, end) in segments {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }

    match merged.as_slice() {
        [(start, end)] => Ok(MergedSpan::NonWrap {
            start: *start,
            end: *end,
        }),
        [(0, morning_end), (night_start, night_end)] if *night_end == MINUTES_PER_DAY => {
            Ok(MergedSpan::Wrap {
                start: *night_start,
                end: *morning_end,
            })
        }
        other => bail!("unsupported disjoint period segments: {other:?}"),
    }
}

fn span_coverage(span: MergedSpan) -> [bool; MINUTES_PER_DAY as usize] {
    let mut bits = [false; MINUTES_PER_DAY as usize];
    match span {
        MergedSpan::NonWrap { start, end } => {
            for minute in start..end {
                bits[minute as usize] = true;
            }
        }
        MergedSpan::Wrap { start, end } => {
            for minute in start..MINUTES_PER_DAY {
                bits[minute as usize] = true;
            }
            for minute in 0..end {
                bits[minute as usize] = true;
            }
        }
    }
    bits
}

fn coverages_overlap(left: &[bool], right: &[bool]) -> bool {
    left.iter().zip(right).any(|(a, b)| *a && *b)
}

fn assign_day_remainders(spans: &[MergedSpan]) -> Vec<u32> {
    let coverages: Vec<[bool; MINUTES_PER_DAY as usize]> =
        spans.iter().copied().map(span_coverage).collect();
    let mut occupied: Vec<[bool; MINUTES_PER_DAY as usize]> = Vec::new();
    let mut remainders = Vec::with_capacity(spans.len());
    for coverage in &coverages {
        let remainder = occupied
            .iter()
            .position(|day| !coverages_overlap(day, coverage))
            .unwrap_or_else(|| {
                occupied.push([false; MINUTES_PER_DAY as usize]);
                occupied.len() - 1
            });
        for (dst, src) in occupied[remainder].iter_mut().zip(coverage) {
            *dst |= src;
        }
        remainders.push(remainder as u32);
    }
    remainders
}

fn day_mod(divisor: u32, remainder: u32) -> Condition {
    Condition::DayMod {
        divisor,
        remainder,
        timezone: TimeOffset::Local,
    }
}

fn time_range(start: Option<NaiveTime>, end: Option<NaiveTime>) -> Condition {
    Condition::Time {
        start,
        end,
        timezone: TimeOffset::Local,
    }
}

fn span_condition(span: MergedSpan, days: u32, remainder: u32) -> Condition {
    let time = match span {
        MergedSpan::NonWrap { start, end } => {
            time_range(minutes_to_time(start), minutes_to_time(end))
        }
        MergedSpan::Wrap { start, end } => time_range(minutes_to_time(start), minutes_to_time(end)),
    };

    if days <= 1 {
        return time;
    }

    match span {
        MergedSpan::NonWrap { .. } => Condition::And {
            conditions: vec![day_mod(days, remainder), time],
        },
        MergedSpan::Wrap { start, end } => Condition::Or {
            conditions: vec![
                Condition::And {
                    conditions: vec![
                        day_mod(days, remainder),
                        time_range(minutes_to_time(start), None),
                    ],
                },
                Condition::And {
                    conditions: vec![
                        day_mod(days, (remainder + 1) % days),
                        time_range(None, minutes_to_time(end)),
                    ],
                },
            ],
        },
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{
        super::{WpfConfig, migrate},
        *,
    };
    use crate::config::migrate::MigrationSummary;

    fn sample_task(mode: &str, filename: &str, plan_select: i32) -> WpfInfrastTask {
        serde_json::from_value(serde_json::json!({
            "Name": "",
            "IsEnable": true,
            "Mode": mode,
            "Filename": filename,
            "PlanSelect": plan_select,
            "ContinueTraining": true,
            "RoomList": [{"Room": "Mfg"}, {"Room": "Trade"}],
            "UsesOfDrones": "Money",
            "DormThreshold": 30,
            "OriginiumShardAutoReplenishment": true,
            "DormFilterNotStationed": true,
            "DormTrustEnabled": true,
            "ReceptionMessageBoard": true,
            "ReceptionClueExchange": true,
            "SendClue": true,
        }))
        .unwrap()
    }

    fn write_schedule(contents: &str) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        std::fs::write(file.path(), contents).unwrap();
        file
    }

    fn to_json(task: &WpfInfrastTask, summary: &mut MigrationSummary) -> serde_json::Value {
        serde_json::to_value(task.to_cli_task(summary, None, &mut false).unwrap()).unwrap()
    }

    fn infrast_profile(mode: &str, filename: &str) -> WpfConfig {
        serde_json::from_value(serde_json::json!({
            "TaskQueue": [{
                "$type": "InfrastTask",
                "Name": "",
                "IsEnable": true,
                "Mode": mode,
                "Filename": filename,
                "PlanSelect": -1,
                "ContinueTraining": true,
                "RoomList": [{"Room": "Mfg"}, {"Room": "Trade"}],
                "UsesOfDrones": "Money",
                "DormThreshold": 30,
                "OriginiumShardAutoReplenishment": true,
                "DormFilterNotStationed": true,
                "DormTrustEnabled": true,
                "ReceptionMessageBoard": true,
                "ReceptionClueExchange": true,
                "SendClue": true,
            }],
            "Gui": {}
        }))
        .unwrap()
    }

    #[test]
    fn normal_mode_keeps_default_layout() {
        let task = sample_task("Normal", "", -1);
        let mut summary = MigrationSummary::default();
        let json = to_json(&task, &mut summary);
        assert!(summary.is_empty());
        assert_eq!(json["params"]["mode"], 0);
        assert!(json.get("variants").is_none() || json["variants"].as_array().unwrap().is_empty());
        assert!(json["params"].get("filename").is_none());
        assert_eq!(json["params"]["threshold"], 0.3);
    }

    #[test]
    fn missing_custom_file_falls_back_to_default() {
        let task = sample_task("Custom", "/no/such/infrast-plan.json", -1);
        let mut summary = MigrationSummary::default();
        let json = to_json(&task, &mut summary);
        assert_eq!(json["params"]["mode"], 0);
        assert!(json["params"].get("filename").is_none());
        assert!(summary.skipped_fields.iter().any(|f| f.field == "Filename"));
        assert!(summary.skipped_fields.iter().any(|f| f.field == "Mode"));
    }

    #[test]
    fn tilde_in_filename_expands_to_home() {
        let expanded = resolve_infrast_filename("~/一图流-153-一天两换-MAA.json");
        assert_eq!(
            expanded,
            expand_tilde(Path::new("~/一图流-153-一天两换-MAA.json")).into_owned()
        );
        assert!(expanded.is_absolute());
        assert!(!expanded.starts_with("~"));
    }

    #[test]
    fn one_day_two_shifts_generates_time_variants() {
        let schedule = r#"{
            "plans": [
                {"name": "A", "period": [["04:00", "16:00"]]},
                {"name": "B", "period": [["16:00", "23:59"], ["00:00", "04:00"]]}
            ]
        }"#;
        let file = write_schedule(schedule);
        let task = sample_task("Custom", file.path().to_str().unwrap(), -1);
        let mut summary = MigrationSummary::default();
        let json = to_json(&task, &mut summary);
        assert!(summary.is_empty());
        assert_eq!(json["params"]["mode"], CUSTOM_MODE);
        assert_eq!(json["params"]["filename"], file.path().to_str().unwrap());
        let variants = json["variants"].as_array().unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0]["params"]["plan_index"], 0);
        assert_eq!(variants[0]["condition"]["type"], "Time");
        assert_eq!(variants[0]["condition"]["start"], "04:00:00");
        assert_eq!(variants[0]["condition"]["end"], "16:00:00");
        assert_eq!(variants[1]["params"]["plan_index"], 1);
        assert_eq!(variants[1]["condition"]["type"], "Time");
        assert_eq!(variants[1]["condition"]["start"], "16:00:00");
        assert_eq!(variants[1]["condition"]["end"], "04:00:00");
    }

    #[test]
    fn two_day_six_shifts_uses_day_mod() {
        let schedule = r#"{
            "plans": [
                {"period": [["04:00", "12:00"]]},
                {"period": [["12:00", "20:00"]]},
                {"period": [["20:00", "23:59"], ["00:00", "04:00"]]},
                {"period": [["04:00", "12:00"]]},
                {"period": [["12:00", "20:00"]]},
                {"period": [["20:00", "23:59"], ["00:00", "04:00"]]}
            ]
        }"#;
        let file = write_schedule(schedule);
        let task = sample_task("Custom", file.path().to_str().unwrap(), -1);
        let mut summary = MigrationSummary::default();
        let json = to_json(&task, &mut summary);
        let variants = json["variants"].as_array().unwrap();
        assert_eq!(variants.len(), 6);
        assert_eq!(variants[0]["condition"]["type"], "And");
        assert_eq!(variants[0]["condition"]["conditions"][0]["type"], "DayMod");
        assert_eq!(variants[0]["condition"]["conditions"][0]["divisor"], 2);
        assert!(
            variants[0]["condition"]["conditions"][0]
                .get("remainder")
                .is_none()
        );
        assert_eq!(variants[3]["condition"]["conditions"][0]["remainder"], 1);
        assert_eq!(variants[2]["condition"]["type"], "Or");
        assert_eq!(variants[5]["condition"]["type"], "Or");
        assert_eq!(variants[5]["params"]["plan_index"], 5);
    }

    #[test]
    fn optional_shift_uses_plan_index() {
        let schedule = r#"{"plans": [{"name": "A"}, {"name": "B"}]}"#;
        let file = write_schedule(schedule);
        let task = sample_task("Custom", file.path().to_str().unwrap(), 1);
        let mut summary = MigrationSummary::default();
        let json = to_json(&task, &mut summary);
        assert_eq!(json["params"]["mode"], CUSTOM_MODE);
        assert_eq!(json["params"]["plan_index"], 1);
        assert!(json.get("variants").is_none() || json["variants"].as_array().unwrap().is_empty());
    }

    #[test]
    fn custom_schedule_overrides_wpf_filename() {
        let schedule = r#"{
            "plans": [
                {"name": "A", "period": [["04:00", "16:00"]]},
                {"name": "B", "period": [["16:00", "23:59"], ["00:00", "04:00"]]}
            ]
        }"#;
        let file = write_schedule(schedule);
        let profile = infrast_profile("Custom", "/original/missing.json");
        let (value, summary) = migrate(profile, Some(file.path())).unwrap();
        assert!(summary.is_empty());
        let json = serde_json::to_value(value).unwrap();
        let task = &json["tasks"][0];
        assert_eq!(task["params"]["mode"], CUSTOM_MODE);
        assert_eq!(
            task["params"]["filename"],
            resolve_infrast_filename(file.path().to_str().unwrap())
                .to_string_lossy()
                .as_ref()
        );
        assert_eq!(task["variants"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn custom_schedule_errors_when_profile_is_not_custom() {
        let profile = infrast_profile("Normal", "");
        let err = migrate(profile, Some(Path::new("plan.json"))).unwrap_err();
        assert!(err.to_string().contains("--custom-schedule"), "{err}");
    }

    #[test]
    fn custom_schedule_errors_when_override_file_missing() {
        let profile = infrast_profile("Custom", "/original/missing.json");
        let err = migrate(profile, Some(Path::new("/no/such/override.json"))).unwrap_err();
        assert!(
            err.to_string()
                .contains("failed to read custom infrast file"),
            "{err}"
        );
    }

    #[test]
    fn assign_days_packs_overlapping_clock_spans() {
        let spans = [
            MergedSpan::NonWrap {
                start: 4 * 60,
                end: 12 * 60,
            },
            MergedSpan::NonWrap {
                start: 12 * 60,
                end: 20 * 60,
            },
            MergedSpan::Wrap {
                start: 20 * 60,
                end: 4 * 60,
            },
            MergedSpan::NonWrap {
                start: 4 * 60,
                end: 12 * 60,
            },
            MergedSpan::NonWrap {
                start: 12 * 60,
                end: 20 * 60,
            },
            MergedSpan::Wrap {
                start: 20 * 60,
                end: 4 * 60,
            },
        ];
        assert_eq!(assign_day_remainders(&spans), vec![0, 0, 0, 1, 1, 1]);
    }
}
