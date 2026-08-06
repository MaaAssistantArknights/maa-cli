use std::{io::BufReader, num::NonZero, path::Path};

use anyhow::{Context, Result, bail, ensure};
use log::trace;
use maa_value::{
    map::StringMap,
    prelude::*,
    userinput::{SelectD, UserInput},
};
use serde_json::Value;

use super::MigrationSummary;
use crate::config::Filetype;

/// Meta / structural keys that are never reported as skipped fields.
const META_FIELDS: &[&str] = &["$type", "TaskType", "Name", "IsEnable"];

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
    ft.write(&out, &value)
        .with_context(|| format!("Failed to write migrated file {}", out.display()));

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
    let current = input.get("Current").and_then(|v| v.as_str()).map(str::to_string).unwrap_or_default();
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
                _ => resolve_configuration_name(object, current)?
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

/// Migrate a GUI profile `MAAValue` into maa-cli task config shape.
pub(super) fn migrate(
    input: Value,
) -> Result<(MAAValue, MigrationSummary)> {
    let summary = MigrationSummary::default();
    
    unimplemented!()
}

fn migrate_task(
    task: &MAAValue,
    config: &MAAValue,
    summary: &mut MigrationSummary,
) -> Result<Option<MAAValue>> {
    let type_tag = task
        .get("$type")
        .and_then(|v| v.as_str())
        .context("GUI task missing $type")?;
    let name = task_name(task);
    let disabled = matches!(
        task.get("IsEnable"),
        Some(MAAValue::Primitive(MAAPrimitive::Bool(false)))
    );

    let mut item = match type_tag {
        "StartUpTask" => start_up::migrate_start_up_task(task, config, summary)?,
        "FightTask" => fight::migrate_fight_task(task, summary)?,
        "InfrastTask" => infrast::migrate_infrast_task(task, summary)?,
        "RecruitTask" => recruit::migrate_recruit_task(task, summary)?,
        "MallTask" => mall::migrate_mall_task(task, summary)?,
        "AwardTask" => award::migrate_award_task(task, summary)?,
        "RoguelikeTask" => roguelike::migrate_roguelike_task(task, summary)?,
        "ReclamationTask" => reclamation::migrate_reclamation_task(task, summary)?,
        type_tag => {
            summary.skip_task(type_tag, name);
            return Ok(None);
        }
    };

    if disabled && let Some(item) = item.as_mut() {
        apply_disabled_condition(item);
        summary.disable_task(type_tag, name);
    }

    Ok(item)
}

fn task_name(task: &MAAValue) -> Option<String> {
    task.get("Name")
        .and_then(|v| v.as_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

/// Keep a disabled GUI task in the output, but ensure it never becomes active.
fn apply_disabled_condition(item: &mut MAAValue) {
    let never = never_condition();

    if let Some(MAAValue::Array(variants)) = item.get_mut("variants") {
        for variant in variants.iter_mut() {
            let new_cond = match variant.get("condition") {
                Some(cond) => object!(
                    "type" => "And",
                    "conditions" => vec![cond.clone(), never.clone()]??
                ),
                None => never.clone(),
            };
            variant.insert("condition", new_cond);
        }
        return;
    }

    let params = item.get("params").cloned().unwrap_or_default();
    if let Some(map) = item.as_mut_map() {
        map.shift_remove("params");
    }
    item.insert(
        "variants",
        MAAValue::Array(vec![object!(
            "condition" => never,
            "params" => params
        )]),
    );
}

fn never_condition() -> MAAValue {
    object!(
        "type" => "Not",
        "condition" => object!("type" => "Always")
    )
}

fn report_unhandled_fields(
    summary: &mut MigrationSummary,
    task: &MAAValue,
    task_type: &str,
    handled: &[&str],
) {
    let name = task_name(task);
    let Some(map) = task.as_map() else {
        return;
    };
    for (key, value) in map {
        if META_FIELDS.contains(&key.as_str()) || handled.contains(&key.as_str()) {
            continue;
        }
        if is_meaningful_unhandled(value) {
            summary.skip_field(task_type, name.clone(), key.clone());
        }
    }
}

/// Whether an unhandled GUI field likely affects runtime behavior if dropped.
fn is_meaningful_unhandled(value: &MAAValue) -> bool {
    match value {
        MAAValue::Primitive(MAAPrimitive::Bool(v)) => *v,
        MAAValue::Primitive(MAAPrimitive::String(s)) => !s.is_empty(),
        MAAValue::Primitive(MAAPrimitive::Int(i)) => *i != 0 && *i != i32::MAX,
        MAAValue::Primitive(MAAPrimitive::Float(f)) => *f != 0.0,
        MAAValue::Array(items) => !items.is_empty(),
        MAAValue::Object(map) => !map.is_empty(),
    }
}

mod start_up {
    use anyhow::Result;
    use log::warn;
    use maa_value::prelude::*;

    use super::MigrationSummary;

    /// Valid GUI / maa-cli client type strings.
    const VALID_CLIENT_TYPES: &[&str] = &[
        "Official", "Bilibili", "txwy", "YoStarEN", "YoStarJP", "YoStarKR",
    ];

    pub(super) fn migrate_start_up_task(
        task: &MAAValue,
        config: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        let runtime = config.get("Gui").and_then(|gui| gui.get("RuntimeSettings"));

        let mut params = MAAValue::default();

        // Gui.RuntimeSettings.ClientType -> client_type
        // Missing / invalid values fall back to an interactive prompt.
        match runtime
            .and_then(|settings| settings.get("ClientType"))
            .and_then(|value| value.as_str())
            .filter(|value| VALID_CLIENT_TYPES.contains(value))
        {
            Some(client_type) => {
                insert!(params, "client_type" => client_type);
            }
            None => {
                insert!(params, "client_type" => object!(
                    "alternatives" => VALID_CLIENT_TYPES.iter().map(|s| s.to_string()).collect::<Vec<_>>()??,
                ));
            }
        }

        // Gui.RuntimeSettings.StartGame -> start_game_enabled
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(start_game))) =
            runtime.and_then(|settings| settings.get("StartGame"))
        {
            insert!(params, "start_game_enabled" => *start_game);
        }

        // AccountSwitchEnabled + AccountName -> account_name
        // Only emit account_name when account switching is enabled.
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) =
            task.get("AccountSwitchEnabled")
        {
            if let Some(MAAValue::Primitive(MAAPrimitive::String(account_name))) =
                task.get("AccountName")
            {
                insert!(params, "account_name" => account_name.as_str());
            } else {
                summary.skip_field("StartUpTask", None, "AccountName");
                warn!("AccountName is missing, but GUI enable account switching");
            }
        }

        let item = object!(
            "type" => "StartUp",
            "params" => params
        );

        Ok(Some(item))
    }
}

mod fight {
    use anyhow::{Result, bail};
    use log::warn;
    use maa_value::prelude::*;

    use super::{MigrationSummary, report_unhandled_fields};
    use crate::config::task::ClientType;

    pub(super) fn migrate_fight_task(
        task: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        let mut item = object!("type" => "Fight");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let shared_params = shared_fight_params(task);

        let use_weekly = flag(task, "UseWeeklySchedule");
        let use_optional = flag(task, "UseOptionalStage");

        match (use_weekly, use_optional) {
            // No variants: StagePlan must be a single stage.
            (false, false) => {
                let stage = stage_plan_single(task)?;
                let mut params = shared_params;
                insert!(params, "stage" => stage);
                insert!(item, "params" => params);
            }
            // One variant gated by weekly schedule; StagePlan must be a single stage.
            (true, false) => {
                let stage = stage_plan_single(task)?;
                let weekly = weekly_schedule_condition(task)?;
                let mut params = shared_params;
                insert!(params, "stage" => stage);
                insert!(
                    item,
                    "variants" => vec![object!(
                        "condition" => weekly,
                        "params" => params
                    )]??
                );
            }
            // One variant per optional stage, each with its own open-condition.
            (false, true) => {
                let stages = stage_plan_array(task)?;
                let mut variants = Vec::with_capacity(stages.len());
                for stage in stages {
                    let mut params = shared_params.clone();
                    insert!(params, "stage" => stage);
                    variants.push(object!(
                        "condition" => stage_condition(stage),
                        "params" => params
                    ));
                }
                insert!(item, "variants" => variants??);
            }
            // Weekly schedule AND each stage's open-condition.
            (true, true) => {
                let stages = stage_plan_array(task)?;
                let weekly = weekly_schedule_condition(task)?;
                let mut variants = Vec::with_capacity(stages.len());
                for stage in stages {
                    let mut params = shared_params.clone();
                    insert!(params, "stage" => stage);
                    variants.push(object!(
                        "condition" => object!(
                            "type" => "And",
                            "conditions" => vec![
                                weekly.clone(),
                                stage_condition(stage)
                            ]??
                        ),
                        "params" => params
                    ));
                }
                insert!(item, "variants" => variants??);
            }
        }

        report_unhandled_fields(summary, task, "FightTask", &[
            "UseWeeklySchedule",
            "WeeklySchedule",
            "UseOptionalStage",
            "StagePlan",
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
        ]);

        Ok(Some(item))
    }

    fn flag(task: &MAAValue, key: &str) -> bool {
        matches!(
            task.get(key),
            Some(MAAValue::Primitive(MAAPrimitive::Bool(true)))
        )
    }

    /// Shared Fight params that do not depend on stage / condition branching.
    fn shared_fight_params(task: &MAAValue) -> MAAValue {
        let mut params = MAAValue::default();

        // UseMedicine + MedicineCount -> medicine
        if flag(task, "UseMedicine")
            && let Some(MAAValue::Primitive(MAAPrimitive::Int(count))) = task.get("MedicineCount")
        {
            insert!(params, "medicine" => *count);
        }

        // UseStone + StoneCount -> stone
        if flag(task, "UseStone")
            && let Some(MAAValue::Primitive(MAAPrimitive::Int(count))) = task.get("StoneCount")
        {
            warn!(
                "FightTask enables stone={count}; this setting may consume Originite Prime (源石)"
            );
            insert!(params, "stone" => *count);
        }

        // EnableTimesLimit + TimesLimit -> times
        if flag(task, "EnableTimesLimit")
            && let Some(MAAValue::Primitive(MAAPrimitive::Int(times))) = task.get("TimesLimit")
        {
            insert!(params, "times" => *times);
        }

        // EnableTargetDrop + DropId/DropCount -> drops = { <DropId> = <DropCount> }
        if flag(task, "EnableTargetDrop")
            && let (
                Some(MAAValue::Primitive(MAAPrimitive::String(drop_id))),
                Some(MAAValue::Primitive(MAAPrimitive::Int(drop_count))),
            ) = (task.get("DropId"), task.get("DropCount"))
        {
            let mut drop_map = maa_value::map::StringMap::new();
            drop_map.insert(drop_id.clone(), (*drop_count).into());
            insert!(params, "drops" => MAAValue::Object(drop_map));
        }

        // Series -> series (only when non-zero)
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(series))) = task.get("Series")
            && *series != 0
        {
            insert!(params, "series" => *series);
        }

        // UseExpiringMedicine + MedicineExpireDays -> medicine_expire_days
        if flag(task, "UseExpiringMedicine")
            && let Some(MAAValue::Primitive(MAAPrimitive::Int(days))) =
                task.get("MedicineExpireDays")
        {
            insert!(params, "medicine_expire_days" => *days);
        }

        params
    }

    /// `UseWeeklySchedule` + `WeeklySchedule` -> `Weekday` condition.
    fn weekly_schedule_condition(task: &MAAValue) -> Result<MAAValue> {
        let Some(MAAValue::Object(map)) = task.get("WeeklySchedule") else {
            bail!("FightTask UseWeeklySchedule is true but WeeklySchedule is missing or invalid");
        };

        const DAYS: [(&str, &str); 7] = [
            ("Sunday", "Sun"),
            ("Monday", "Mon"),
            ("Tuesday", "Tue"),
            ("Wednesday", "Wed"),
            ("Thursday", "Thu"),
            ("Friday", "Fri"),
            ("Saturday", "Sat"),
        ];

        let mut weekdays = Vec::new();
        for (gui_day, cli_day) in DAYS {
            if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = map.get(gui_day) {
                weekdays.push(cli_day);
            }
        }

        Ok(object!(
            "type" => "Weekday",
            "weekdays" => weekdays??
        ))
    }

    /// Parse `StagePlan` as a single stage name.
    ///
    /// GUI usually stores a single stage as a one-element array (`["CE-6"]`).
    /// A multi-element array is rejected when `UseOptionalStage` is false.
    fn stage_plan_single(task: &MAAValue) -> Result<&str> {
        match task.get("StagePlan") {
            Some(MAAValue::Primitive(MAAPrimitive::String(stage))) => Ok(stage.as_str()),
            Some(MAAValue::Array(stages)) if stages.len() == 1 => match &stages[0] {
                MAAValue::Primitive(MAAPrimitive::String(stage)) => Ok(stage.as_str()),
                _ => bail!("FightTask StagePlan must be a string when UseOptionalStage is false"),
            },
            Some(MAAValue::Array(_)) => {
                bail!(
                    "FightTask StagePlan must be a single stage string when UseOptionalStage is false, \
                     got a string array; enable UseOptionalStage for multiple stages"
                )
            }
            Some(_) => bail!("FightTask StagePlan must be a string when UseOptionalStage is false"),
            None => bail!("FightTask missing StagePlan"),
        }
    }

    /// Parse `StagePlan` as a non-empty string array (optional-stage mode).
    fn stage_plan_array(task: &MAAValue) -> Result<Vec<&str>> {
        match task.get("StagePlan") {
            Some(MAAValue::Array(stages)) => {
                let mut stage_list = Vec::with_capacity(stages.len());
                for stage in stages {
                    match stage {
                        MAAValue::Primitive(MAAPrimitive::String(stage)) => {
                            stage_list.push(stage.as_str());
                        }
                        _ => bail!(
                            "FightTask StagePlan array elements must be strings when UseOptionalStage is true"
                        ),
                    }
                }
                if stage_list.is_empty() {
                    bail!(
                        "FightTask StagePlan must be a non-empty string array when UseOptionalStage is true"
                    );
                }
                Ok(stage_list)
            }
            Some(MAAValue::Primitive(MAAPrimitive::String(_))) => {
                bail!(
                    "FightTask StagePlan must be a string array when UseOptionalStage is true, got a string"
                )
            }
            Some(_) => {
                bail!("FightTask StagePlan must be a string array when UseOptionalStage is true")
            }
            None => bail!("FightTask missing StagePlan"),
        }
    }

    /// Build the open-condition for a fight stage name.
    ///
    /// - Resource stages with a weekly rotation → `Weekday` (server timezone Official)
    /// - Permanent stages (mainline, annihilation, LS-6, OF-*) → `Always`
    /// - Side-story stages listed in `StageActivityV2.json` → `OnSideStory`
    fn stage_condition(stage: &str) -> MAAValue {
        match stage {
            // 资源本 / 芯片本（同开放日合并）
            "CE-6" => weekday_condition(&["Tue", "Thu", "Sat", "Sun"]),
            "AP-5" => weekday_condition(&["Mon", "Thu", "Sat", "Sun"]),
            "CA-5" => weekday_condition(&["Tue", "Wed", "Fri", "Sun"]),
            "SK-5" => weekday_condition(&["Mon", "Wed", "Fri", "Sat"]),
            "PR-A-1" | "PR-A-2" => weekday_condition(&["Mon", "Thu", "Fri", "Sun"]),
            "PR-B-1" | "PR-B-2" => weekday_condition(&["Mon", "Tue", "Fri", "Sat"]),
            "PR-C-1" | "PR-C-2" => weekday_condition(&["Wed", "Thu", "Sat", "Sun"]),
            "PR-D-1" | "PR-D-2" => weekday_condition(&["Tue", "Wed", "Sat", "Sun"]),
            // 永久开启
            "LS-6" | "Annihilation" | "OF-1" | "OF-F3" => always_condition(),
            _ => {
                let side_story = crate::activity::side_story_stages(ClientType::Official);
                if side_story.iter().any(|code| code == stage) {
                    return object!("type" => "OnSideStory");
                }
                warn!("FightTask stage `{stage}` has no known open schedule; treating as Always");
                always_condition()
            }
        }
    }

    fn always_condition() -> MAAValue {
        object!("type" => "Always")
    }

    fn weekday_condition(weekdays: &[&str]) -> MAAValue {
        object!(
            "type" => "Weekday",
            "weekdays" => weekdays.to_vec()??,
            "timezone" => "Official"
        )
    }
}

mod infrast {
    use anyhow::{Result, bail};
    use maa_value::prelude::*;

    use super::{MigrationSummary, report_unhandled_fields};

    pub(super) fn migrate_infrast_task(
        task: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        // Custom infrastructure plans are not migrated yet.
        if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode")
            && mode == "Custom"
        {
            bail!("InfrastTask custom mode is not supported yet");
        }
        if let Some(MAAValue::Primitive(MAAPrimitive::String(filename))) = task.get("Filename")
            && !filename.is_empty()
        {
            bail!("InfrastTask custom plan (Filename) is not supported yet");
        }

        let mut item = object!("type" => "Infrast");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
        let mut handled = vec![
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
            // Custom plan fields: rejected above when set; keep handled so defaults are quiet.
            "Filename",
            "PlanSelect",
        ];
        // Mode -> mode (Custom is rejected above)
        if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode") {
            if let Some(mode) = match mode.as_str() {
                "Normal" => Some(0),
                "Rotation" => Some(20000),
                _ => None,
            } {
                insert!(params, "mode" => mode);
            } else {
                handled.retain(|field| *field != "Mode");
            }
        }
        // RoomList -> facility
        if let Some(MAAValue::Array(rooms)) = task.get("RoomList") {
            let mut facility = Vec::new();
            for room in rooms {
                if let MAAValue::Object(map) = room
                    && let Some(MAAValue::Primitive(MAAPrimitive::String(room))) = map.get("Room")
                {
                    facility.push(room.as_str());
                }
            }
            if !facility.is_empty() {
                insert!(params, "facility" => facility??);
            }
        }
        // UsesOfDrones -> drones
        if let Some(MAAValue::Primitive(MAAPrimitive::String(drones))) = task.get("UsesOfDrones") {
            insert!(params, "drones" => drones.as_str());
        }
        // DormThreshold -> threshold
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(threshold))) = task.get("DormThreshold") {
            insert!(params, "threshold" => *threshold as f32 / 100.0);
        }
        // OriginiumShardAutoReplenishment -> replenish
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(replenish))) =
            task.get("OriginiumShardAutoReplenishment")
        {
            insert!(params, "replenish" => *replenish);
        }
        // DormFilterNotStationed -> dorm_notstationed_enabled
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) =
            task.get("DormFilterNotStationed")
        {
            insert!(params, "dorm_notstationed_enabled" => *enabled);
        }
        // DormTrustEnabled -> dorm_trust_enabled
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) = task.get("DormTrustEnabled")
        {
            insert!(params, "dorm_trust_enabled" => *enabled);
        }
        // ReceptionMessageBoard -> reception_message_board
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) =
            task.get("ReceptionMessageBoard")
        {
            insert!(params, "reception_message_board" => *enabled);
        }
        // ReceptionClueExchange -> reception_clue_exchange
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) =
            task.get("ReceptionClueExchange")
        {
            insert!(params, "reception_clue_exchange" => *enabled);
        }
        // SendClue -> reception_send_clue
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(enabled))) = task.get("SendClue") {
            insert!(params, "reception_send_clue" => *enabled);
        }

        insert!(item, "params" => params);
        report_unhandled_fields(summary, task, "InfrastTask", &handled);
        Ok(Some(item))
    }
}

mod recruit {
    use anyhow::Result;
    use maa_value::prelude::*;

    use super::{MigrationSummary, report_unhandled_fields};

    pub(super) fn migrate_recruit_task(
        task: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        let mut item = object!("type" => "Recruit");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
        // MaxTimes -> times
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(times))) = task.get("MaxTimes") {
            insert!(params, "times" => *times);
        }
        // ExtraTagMode -> extra_tags_mode
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(mode))) = task.get("ExtraTagMode") {
            insert!(params, "extra_tags_mode" => *mode);
        }
        // RefreshLevel3 -> refresh
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(refresh))) = task.get("RefreshLevel3") {
            insert!(params, "refresh" => *refresh);
        }
        // ForceRefresh -> expedite
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(expedite))) = task.get("ForceRefresh") {
            insert!(params, "expedite" => *expedite);
        }
        // LevelXChoose -> select / confirm
        let mut select = Vec::new();
        let mut confirm = Vec::new();
        let mut recruitment_time = MAAValue::default();
        const LEVELS: [(&str, &str, i32); 4] = [
            ("Level6Choose", "Level6Time", 6),
            ("Level5Choose", "Level5Time", 5),
            ("Level4Choose", "Level4Time", 4),
            ("Level3Choose", "Level3Time", 3),
        ];
        for (choose_key, time_key, level) in LEVELS {
            if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get(choose_key) {
                select.push(level);
                confirm.push(level);
            }
            if let Some(MAAValue::Primitive(MAAPrimitive::Int(minutes))) = task.get(time_key) {
                recruitment_time.insert(level.to_string(), (*minutes).into());
            }
        }
        if !select.is_empty() {
            insert!(params, "select" => select??);
            insert!(params, "confirm" => confirm??);
        }
        if recruitment_time.as_map().is_some_and(|map| !map.is_empty()) {
            insert!(params, "recruitment_time" => recruitment_time);
        }
        // PreferTagEnabled + Level3PreferTags -> first_tags
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get("PreferTagEnabled")
            && let Some(MAAValue::Array(tags)) = task.get("Level3PreferTags")
        {
            let mut first_tags = Vec::new();
            for tag in tags {
                if let MAAValue::Primitive(MAAPrimitive::String(tag)) = tag {
                    first_tags.push(tag.as_str());
                }
            }
            if !first_tags.is_empty() {
                insert!(params, "first_tags" => first_tags??);
            }
        }
        // PreserveTagEnabled + PreserveTagList -> preserve_tags
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get("PreserveTagEnabled")
            && let Some(MAAValue::Array(tags)) = task.get("PreserveTagList")
        {
            let mut preserve_tags = Vec::new();
            for tag in tags {
                if let MAAValue::Primitive(MAAPrimitive::String(tag)) = tag {
                    preserve_tags.push(tag.as_str());
                }
            }
            if !preserve_tags.is_empty() {
                insert!(params, "preserve_tags" => preserve_tags??);
            }
        }

        insert!(item, "params" => params);
        report_unhandled_fields(summary, task, "RecruitTask", &[
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
        ]);
        Ok(Some(item))
    }
}

mod mall {
    use anyhow::Result;
    use maa_value::prelude::*;

    use super::{MigrationSummary, report_unhandled_fields};

    pub(super) fn migrate_mall_task(
        task: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        let mut item = object!("type" => "Mall");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
        // Shopping -> shopping
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(shopping))) = task.get("Shopping") {
            insert!(params, "shopping" => *shopping);
        }
        // CreditFight -> credit_fight
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(credit_fight))) = task.get("CreditFight")
        {
            insert!(params, "credit_fight" => *credit_fight);
        }
        // CreditFightFormation -> formation_index
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(formation_index))) =
            task.get("CreditFightFormation")
        {
            insert!(params, "formation_index" => *formation_index);
        }
        // VisitFriends -> visit_friends
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(visit_friends))) =
            task.get("VisitFriends")
        {
            insert!(params, "visit_friends" => *visit_friends);
        }
        // FirstList -> buy_first
        if let Some(MAAValue::Primitive(MAAPrimitive::String(list))) = task.get("FirstList") {
            let buy_first: Vec<_> = list.split(';').filter(|item| !item.is_empty()).collect();
            if !buy_first.is_empty() {
                insert!(params, "buy_first" => buy_first??);
            }
        }
        // BlackList -> blacklist
        if let Some(MAAValue::Primitive(MAAPrimitive::String(list))) = task.get("BlackList") {
            let blacklist: Vec<_> = list.split(';').filter(|item| !item.is_empty()).collect();
            if !blacklist.is_empty() {
                insert!(params, "blacklist" => blacklist??);
            }
        }
        // ShoppingIgnoreBlackListWhenFull -> force_shopping_if_credit_full
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(force))) =
            task.get("ShoppingIgnoreBlackListWhenFull")
        {
            insert!(params, "force_shopping_if_credit_full" => *force);
        }
        // OnlyBuyDiscount -> only_buy_discount
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(only_discount))) =
            task.get("OnlyBuyDiscount")
        {
            insert!(params, "only_buy_discount" => *only_discount);
        }
        // ReserveMaxCredit -> reserve_max_credit
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(reserve))) = task.get("ReserveMaxCredit")
        {
            insert!(params, "reserve_max_credit" => *reserve);
        }

        insert!(item, "params" => params);
        report_unhandled_fields(summary, task, "MallTask", &[
            "Shopping",
            "CreditFight",
            "CreditFightFormation",
            "VisitFriends",
            "FirstList",
            "BlackList",
            "ShoppingIgnoreBlackListWhenFull",
            "OnlyBuyDiscount",
            "ReserveMaxCredit",
        ]);
        Ok(Some(item))
    }
}

mod award {
    use anyhow::Result;
    use maa_value::prelude::*;

    use super::{MigrationSummary, report_unhandled_fields};

    pub(super) fn migrate_award_task(
        task: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        let mut item = object!("type" => "Award");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
        // Award -> award
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(award))) = task.get("Award") {
            insert!(params, "award" => *award);
        }
        // Mail -> mail
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(mail))) = task.get("Mail") {
            insert!(params, "mail" => *mail);
        }
        // FreeGacha -> recruit
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(recruit))) = task.get("FreeGacha") {
            insert!(params, "recruit" => *recruit);
        }
        // Orundum -> orundum
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(orundum))) = task.get("Orundum") {
            insert!(params, "orundum" => *orundum);
        }
        // Mining -> mining
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(mining))) = task.get("Mining") {
            insert!(params, "mining" => *mining);
        }
        // SpecialAccess -> specialaccess
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(specialaccess))) =
            task.get("SpecialAccess")
        {
            insert!(params, "specialaccess" => *specialaccess);
        }

        insert!(item, "params" => params);
        report_unhandled_fields(summary, task, "AwardTask", &[
            "Award",
            "Mail",
            "FreeGacha",
            "Orundum",
            "Mining",
            "SpecialAccess",
        ]);
        Ok(Some(item))
    }
}

mod roguelike {
    use anyhow::Result;
    use maa_value::prelude::*;

    use super::{MigrationSummary, report_unhandled_fields};

    pub(super) fn migrate_roguelike_task(
        task: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        let mut item = object!("type" => "Roguelike");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
        let mut handled = vec![
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
        // Theme -> theme
        if let Some(MAAValue::Primitive(MAAPrimitive::String(theme))) = task.get("Theme") {
            insert!(params, "theme" => theme.as_str());
        }
        // Mode -> mode
        if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode") {
            if let Some(mode) = match mode.as_str() {
                "Exp" => Some(0),
                "Investment" => Some(1),
                "Collect" => Some(4),
                "CollapsalParadigms" => Some(5),
                "MonthlySquad" => Some(6),
                "DeepExploration" => Some(7),
                _ => None,
            } {
                insert!(params, "mode" => mode);
            } else {
                handled.retain(|field| *field != "Mode");
            }
        }
        // Squad -> squad
        if let Some(MAAValue::Primitive(MAAPrimitive::String(squad))) = task.get("Squad") {
            insert!(params, "squad" => squad.as_str());
        }
        // Roles -> roles
        if let Some(MAAValue::Primitive(MAAPrimitive::String(roles))) = task.get("Roles") {
            insert!(params, "roles" => roles.as_str());
        }
        // CoreChar -> core_char
        if let Some(MAAValue::Primitive(MAAPrimitive::String(core_char))) = task.get("CoreChar") {
            insert!(params, "core_char" => core_char.as_str());
        }
        // StartCount -> starts_count
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(starts_count))) = task.get("StartCount") {
            insert!(params, "starts_count" => *starts_count);
        }
        // Difficulty -> difficulty
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(difficulty))) = task.get("Difficulty")
            && *difficulty != i32::MAX
        {
            insert!(params, "difficulty" => *difficulty);
        }
        // Investment -> investment_enabled
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(investment_enabled))) =
            task.get("Investment")
        {
            insert!(params, "investment_enabled" => *investment_enabled);
        }
        // InvestCount -> investments_count
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(investments_count))) =
            task.get("InvestCount")
        {
            insert!(params, "investments_count" => *investments_count);
        }
        // InvestWithMoreScore -> investment_with_more_score
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(investment_with_more_score))) =
            task.get("InvestWithMoreScore")
        {
            insert!(params, "investment_with_more_score" => *investment_with_more_score);
        }
        // StopWhenDepositFull -> stop_when_investment_full
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(stop_when_investment_full))) =
            task.get("StopWhenDepositFull")
        {
            insert!(params, "stop_when_investment_full" => *stop_when_investment_full);
        }
        // StopAtFinalBoss -> stop_at_final_boss
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(stop_at_final_boss))) =
            task.get("StopAtFinalBoss")
        {
            insert!(params, "stop_at_final_boss" => *stop_at_final_boss);
        }
        // StopWhenLevelMax -> stop_at_max_level
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(stop_at_max_level))) =
            task.get("StopWhenLevelMax")
        {
            insert!(params, "stop_at_max_level" => *stop_at_max_level);
        }
        // UseSupport -> use_support
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(use_support))) = task.get("UseSupport") {
            insert!(params, "use_support" => *use_support);
        }
        // UseSupportNonFriend -> use_nonfriend_support
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(use_nonfriend_support))) =
            task.get("UseSupportNonFriend")
        {
            insert!(params, "use_nonfriend_support" => *use_nonfriend_support);
        }
        // RefreshTraderWithDice -> refresh_trader_with_dice
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(refresh_trader_with_dice))) =
            task.get("RefreshTraderWithDice")
        {
            insert!(params, "refresh_trader_with_dice" => *refresh_trader_with_dice);
        }
        // StartWithEliteTwo -> start_with_elite_two
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(start_with_elite_two))) =
            task.get("StartWithEliteTwo")
        {
            insert!(params, "start_with_elite_two" => *start_with_elite_two);
        }
        // StartWithEliteTwoOnly -> only_start_with_elite_two
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(only_start_with_elite_two))) =
            task.get("StartWithEliteTwoOnly")
        {
            insert!(params, "only_start_with_elite_two" => *only_start_with_elite_two);
        }

        insert!(item, "params" => params);
        report_unhandled_fields(summary, task, "RoguelikeTask", &handled);
        Ok(Some(item))
    }
}

mod reclamation {
    use anyhow::Result;
    use maa_value::prelude::*;

    use super::{MigrationSummary, report_unhandled_fields};

    pub(super) fn migrate_reclamation_task(
        task: &MAAValue,
        summary: &mut MigrationSummary,
    ) -> Result<Option<MAAValue>> {
        let mut item = object!("type" => "Reclamation");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
        let mut handled = vec![
            "Theme",
            "Mode",
            "ToolToCraft",
            "IncrementMode",
            "MaxCraftCountPerRound",
        ];
        // Theme -> theme
        if let Some(MAAValue::Primitive(MAAPrimitive::String(theme))) = task.get("Theme") {
            insert!(params, "theme" => theme.as_str());
        }
        // Mode -> mode
        if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode") {
            if let Some(mode) = match mode.as_str() {
                "ProsperityNoSave" => Some(0),
                "ProsperityInSave" => Some(1),
                _ => None,
            } {
                insert!(params, "mode" => mode);
            } else {
                handled.retain(|field| *field != "Mode");
            }
        }
        // ToolToCraft -> tools_to_craft
        if let Some(MAAValue::Primitive(MAAPrimitive::String(tool))) = task.get("ToolToCraft")
            && !tool.is_empty()
        {
            insert!(params, "tools_to_craft" => vec![tool.as_str()]??);
        }
        // IncrementMode -> increment_mode
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(increment_mode))) =
            task.get("IncrementMode")
        {
            insert!(params, "increment_mode" => *increment_mode);
        }
        // MaxCraftCountPerRound -> num_craft_batches
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(num_craft_batches))) =
            task.get("MaxCraftCountPerRound")
        {
            insert!(params, "num_craft_batches" => *num_craft_batches);
        }

        insert!(item, "params" => params);
        report_unhandled_fields(summary, task, "ReclamationTask", &handled);
        Ok(Some(item))
    }
}
