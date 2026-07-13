use std::num::NonZero;

use anyhow::{Context, Result, bail};
use maa_value::{
    map::StringMap,
    prelude::*,
    userinput::{SelectD, UserInput},
};

const GUI_CONFIG_ENV: &str = "MAA_GUI_CONFIG";

/// Pick one configuration from a multi-profile GUI export.
///
/// Legacy single-config profiles are returned unchanged.
///
/// When multiple configurations exist, `gui_config` or [`GUI_CONFIG_ENV`] can be
/// used to select one without an interactive prompt.
pub(crate) fn select_configuration(input: MAAValue, gui_config: Option<&str>) -> Result<MAAValue> {
    let Some(configurations_value) = input.get("Configurations") else {
        return Ok(input);
    };
    let Some(configurations) = configurations_value.as_map() else {
        bail!("GUI profile Configurations must be an object");
    };

    match configurations.len() {
        0 => bail!("GUI profile has no configuration"),
        1 => Ok(configurations.values().next().unwrap().clone()),
        _ => {
            let selected_name = resolve_configuration_name(&input, configurations, gui_config)?;
            configurations
                .get(&selected_name)
                .cloned()
                .with_context(|| format!("GUI configuration {selected_name} not found"))
        }
    }
}

fn resolve_configuration_name(
    input: &MAAValue,
    configurations: &StringMap<MAAValue>,
    gui_config: Option<&str>,
) -> Result<String> {
    if let Some(name) = gui_config {
        return Ok(name.to_string());
    }
    if let Ok(name) = std::env::var(GUI_CONFIG_ENV) {
        return Ok(name);
    }

    let names: Vec<&str> = configurations.keys().map(String::as_str).collect();
    let default_index = input
        .get("Current")
        .and_then(|v| v.as_str())
        .and_then(|current| names.iter().position(|name| *name == current))
        .and_then(|i| NonZero::new(i + 1));

    SelectD::<String>::from_iter(names, default_index)
        .context("Failed to build configuration selection")?
        .with_description("a GUI configuration")
        .value()
        .context("Failed to select GUI configuration")
}

/// Convert a GUI profile `MAAValue` into maa-cli task config shape.
pub(crate) fn convert(input: MAAValue) -> Result<MAAValue> {
    // GUI profile task list
    let queue = input
        .get("TaskQueue")
        .context("GUI profile missing TaskQueue")?;
    let MAAValue::Array(queue) = queue else {
        bail!("GUI profile TaskQueue must be an array");
    };

    let mut tasks = Vec::new();
    for task in queue {
        if let Some(item) = convert_task(task)? {
            tasks.push(item);
        }
    }

    Ok(object!("tasks" => tasks??))
}

fn convert_task(task: &MAAValue) -> Result<Option<MAAValue>> {
    // GUI task discriminator, e.g. "FightTask"
    let type_tag = task
        .get("$type")
        .and_then(|v| v.as_str())
        .context("GUI task missing $type")?;
    match type_tag {
        "StartUpTask" => start_up::convert_start_up_task(task),
        "FightTask" => fight::convert_fight_task(task),
        "InfrastTask" => infrast::convert_infrast_task(task),
        "RecruitTask" => recruit::convert_recruit_task(task),
        "MallTask" => mall::convert_mall_task(task),
        "AwardTask" => award::convert_award_task(task),
        "RoguelikeTask" => roguelike::convert_roguelike_task(task),
        "ReclamationTask" => reclamation::convert_reclamation_task(task),
        type_tag => {
            log::warn!("Skipping unsupported GUI task type: {type_tag}");
            Ok(None)
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    macro_rules! gui_task_test {
        ($name:ident, $converter:path, $task_json:expr, $($expected:tt)*) => {
            #[test]
            fn $name() {
                let task: MAAValue = serde_json::from_str($task_json).unwrap();
                let actual = $converter(&task).unwrap().unwrap();
                assert_eq!(
                    serde_json::to_value(&actual).unwrap(),
                    serde_json::json!($($expected)*)
                );
            }
        };
    }

    const DEFAULT_PROFILE: &str = include_str!("../../fixtures/gui/default_profile.json");

    #[test]
    fn select_configuration_passes_through_legacy_profile() {
        let input = object!("legacy" => true);
        assert_eq!(select_configuration(input.clone(), None).unwrap(), input);
    }

    #[test]
    fn select_configuration_picks_the_only_configuration() {
        let input = object!(
            "Configurations" => object!(
                "Default" => object!("name" => "only")
            )
        );
        assert_eq!(
            select_configuration(input, None).unwrap(),
            object!("name" => "only")
        );
    }

    #[test]
    fn select_configuration_uses_current_in_batch_mode() {
        let input = object!(
            "Current" => "Dev",
            "Configurations" => object!(
                "Default" => object!("name" => "default"),
                "Dev" => object!("name" => "dev"),
            )
        );
        assert_eq!(
            select_configuration(input, None).unwrap(),
            object!("name" => "dev")
        );
    }

    #[test]
    fn select_configuration_uses_gui_config_override() {
        let input = object!(
            "Current" => "Default",
            "Configurations" => object!(
                "Default" => object!("name" => "default"),
                "Dev" => object!("name" => "dev"),
            )
        );
        assert_eq!(
            select_configuration(input, Some("Dev")).unwrap(),
            object!("name" => "dev")
        );
    }

    #[test]
    fn select_configuration_uses_env_override() {
        let input = object!(
            "Configurations" => object!(
                "Default" => object!("name" => "default"),
                "Dev" => object!("name" => "dev"),
            )
        );

        // SAFETY: test-only env var, restored before return.
        unsafe { std::env::set_var(GUI_CONFIG_ENV, "Dev") };
        let result = select_configuration(input, None);
        unsafe { std::env::remove_var(GUI_CONFIG_ENV) };

        assert_eq!(result.unwrap(), object!("name" => "dev"));
    }

    #[test]
    fn missing_task_queue_is_error() {
        let input = object!("a" => 1);
        let err = convert(input).unwrap_err();
        assert!(err.to_string().contains("TaskQueue"));
    }

    #[test]
    fn task_queue_not_array_is_error() {
        let input = object!("TaskQueue" => "not-an-array");
        let err = convert(input).unwrap_err();
        assert!(err.to_string().contains("TaskQueue must be an array"));
    }

    #[test]
    fn unknown_task_type_is_skipped() {
        let input = object!(
            "TaskQueue" => vec![
                object!("$type" => "UserDataUpdateTask", "Name" => "update"),
                object!("$type" => "StartUpTask", "IsEnable" => true),
            ]??
        );
        let value = convert(input).unwrap();
        let MAAValue::Array(tasks) = value.get("tasks").unwrap() else {
            panic!("tasks should be an array");
        };
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].get("type").unwrap().as_str().unwrap(), "StartUp");
    }

    #[test]
    fn json_to_json_default_profile() {
        let input: MAAValue = serde_json::from_str(DEFAULT_PROFILE).unwrap();
        let value = convert(select_configuration(input, None).unwrap()).unwrap();
        let expected: MAAValue = serde_json::from_value(serde_json::json!({
                "tasks": [
                    {
                        "type": "StartUp",
                        "params": {
                            "client_type": {
                                "alternatives": ["Official", "YoStarEN", "YoStarJP"],
                                "description": "a client type",
                                "deps": {
                                    "start_game_enabled": true
                                }
                            },
                            "start_game_enabled": {
                                "default": true,
                                "description": "start the game"
                            }
                        }
                    },
                    {
                        "type": "Fight",
                        "name": "日常经验本",
                        "strategy": "merge",
                        "variants": [
                            {
                                "condition": {
                                    "type": "Weekday",
                                    "weekdays": ["Mon", "Wed", "Fri"]
                                },
                                "params": {
                                    "medicine_expire_days": 2,
                                    "stage": "LS-6"
                                }
                            }
                        ]
                    },
                    {
                        "type": "Fight",
                        "name": "日常龙门币",
                        "strategy": "merge",
                        "variants": [
                            {
                                "condition": {
                                    "type": "Weekday",
                                    "weekdays": ["Tue", "Thu", "Sat"]
                                },
                                "params": {
                                    "medicine_expire_days": 2,
                                    "stage": "CE-6"
                                }
                            }
                        ]
                    },
                    {
                        "type": "Infrast",
                        "name": "",
                        "params": {
                            "mode": 0,
                            "facility": [
                                "Mfg",
                                "Trade",
                                "Control",
                                "Power",
                                "Reception",
                                "Office",
                                "Dorm",
                                "Processing",
                                "Training"
                            ],
                            "drones": "Money",
                            "threshold": 0.3,
                            "replenish": true,
                            "dorm_notstationed_enabled": true,
                            "dorm_trust_enabled": true,
                            "reception_message_board": true,
                            "reception_clue_exchange": true,
                            "reception_send_clue": true
                        }
                    },
                    {
                        "type": "Recruit",
                        "name": "",
                        "params": {
                            "times": 4,
                            "extra_tags_mode": 0,
                            "refresh": true,
                            "expedite": true,
                            "select": [5, 4, 3],
                            "confirm": [5, 4, 3],
                            "recruitment_time": {
                                "3": 540,
                                "4": 540
                            }
                        }
                    },
                    {
                        "type": "Mall",
                        "name": "",
                        "params": {
                            "shopping": true,
                            "credit_fight": false,
                            "formation_index": 0,
                            "visit_friends": true,
                            "buy_first": ["加急许可", "招聘许可"],
                            "blacklist": ["碳", "家具"],
                            "force_shopping_if_credit_full": true,
                            "only_buy_discount": false,
                            "reserve_max_credit": false
                        }
                    },
                    {
                        "type": "Award",
                        "name": "",
                        "params": {
                            "award": true,
                            "mail": true,
                            "recruit": false,
                            "orundum": true,
                            "mining": true,
                            "specialaccess": true
                        }
                    },
                    {
                        "type": "Roguelike",
                        "name": "",
                        "params": {
                            "theme": "JieGarden",
                            "mode": 1,
                            "squad": "指挥分队",
                            "roles": "稳扎稳打",
                            "core_char": "维什戴尔",
                            "starts_count": 999999,
                            "investment_enabled": true,
                            "investments_count": 999,
                            "investment_with_more_score": false,
                            "stop_when_investment_full": true,
                            "stop_at_final_boss": false,
                            "stop_at_max_level": false,
                            "use_support": false,
                            "use_nonfriend_support": false,
                            "refresh_trader_with_dice": false,
                            "start_with_elite_two": false,
                            "only_start_with_elite_two": false
                        }
                    },
                    {
                        "type": "Reclamation",
                        "name": "",
                        "params": {
                            "theme": "Tales",
                            "mode": 1,
                            "increment_mode": 0,
                            "num_craft_batches": 16
                        }
                    }
                ]
            }))
            .unwrap();
        assert_eq!(value, expected);
    }

    const STARTUP_TASK: &str = r#"{
  "$type": "StartUpTask",
  "AccountName": "",
  "Name": "",
  "IsEnable": true,
  "TaskType": "StartUp"
}"#;

    const STARTUP_TASK_WITH_ACCOUNT: &str = r#"{
  "$type": "StartUpTask",
  "AccountName": "123****4567",
  "Name": "启动游戏",
  "IsEnable": true,
  "TaskType": "StartUp"
}"#;

    gui_task_test!(
        convert_start_up_task_basic,
        start_up::convert_start_up_task,
        STARTUP_TASK,
        {
            "type": "StartUp",
            "params": {
                "client_type": {
                    "alternatives": ["Official", "YoStarEN", "YoStarJP"],
                    "description": "a client type",
                    "deps": {
                        "start_game_enabled": true
                    }
                },
                "start_game_enabled": {
                    "default": true,
                    "description": "start the game"
                }
            }
        }
    );

    gui_task_test!(
        convert_start_up_task_with_account_and_name,
        start_up::convert_start_up_task,
        STARTUP_TASK_WITH_ACCOUNT,
        {
            "type": "StartUp",
            "params": {
                "client_type": {
                    "alternatives": ["Official", "YoStarEN", "YoStarJP"],
                    "description": "a client type",
                    "deps": {
                        "start_game_enabled": true
                    }
                },
                "start_game_enabled": {
                    "default": true,
                    "description": "start the game"
                }
            }
        }
    );

    const FIGHT_TASK: &str = r#"{
  "$type": "FightTask",
  "UseMedicine": false,
  "MedicineCount": 0,
  "UseStone": false,
  "StoneCount": 0,
  "EnableTargetDrop": false,
  "DropId": "",
  "DropCount": 0,
  "EnableTimesLimit": false,
  "TimesLimit": 2147483647,
  "Series": 0,
  "StagePlan": ["LS-6"],
  "IsDrGrandet": false,
  "Name": "日常经验本",
  "IsEnable": true,
  "TaskType": "Fight"
}"#;

    const DISABLED_FIGHT_TASK: &str = r#"{
  "$type": "FightTask",
  "UseMedicine": false,
  "MedicineCount": 0,
  "UseStone": false,
  "StoneCount": 0,
  "EnableTargetDrop": false,
  "DropId": "",
  "DropCount": 0,
  "EnableTimesLimit": false,
  "TimesLimit": 2147483647,
  "Series": 0,
  "StagePlan": ["LS-6"],
  "IsDrGrandet": false,
  "Name": "日常经验本",
  "IsEnable": false,
  "TaskType": "Fight"
}"#;

    const WEEKLY_LS6_TASK: &str = r#"{
  "$type": "FightTask",
  "UseMedicine": false,
  "MedicineCount": 0,
  "UseStone": false,
  "StoneCount": 0,
  "EnableTargetDrop": false,
  "DropId": "",
  "DropCount": 0,
  "IsInventoryTarget": false,
  "EnableTimesLimit": false,
  "TimesLimit": 2147483647,
  "Series": 0,
  "StagePlan": ["LS-6"],
  "IsDrGrandet": false,
  "UseExpiringMedicine": true,
  "MedicineExpireDays": 2,
  "UseExpireMedicineForActivity": false,
  "UseCustomAnnihilation": false,
  "AnnihilationStage": "Annihilation",
  "HideUnavailableStage": false,
  "IsStageManually": false,
  "UseOptionalStage": false,
  "UseStoneAllowSave": false,
  "HideSeries": false,
  "StageResetMode": "Ignore",
  "UseWeeklySchedule": true,
  "WeeklySchedule": {
    "Sunday": false,
    "Monday": true,
    "Tuesday": false,
    "Wednesday": true,
    "Thursday": false,
    "Friday": true,
    "Saturday": false
  },
  "Name": "日常经验本",
  "IsEnable": true,
  "TaskType": "Fight"
}"#;

    const WEEKLY_CE6_TASK: &str = r#"{
  "$type": "FightTask",
  "UseMedicine": false,
  "MedicineCount": 0,
  "UseStone": false,
  "StoneCount": 0,
  "EnableTargetDrop": false,
  "DropId": "",
  "DropCount": 0,
  "IsInventoryTarget": false,
  "EnableTimesLimit": false,
  "TimesLimit": 2147483647,
  "Series": 0,
  "StagePlan": ["CE-6"],
  "IsDrGrandet": false,
  "UseExpiringMedicine": true,
  "MedicineExpireDays": 2,
  "UseExpireMedicineForActivity": false,
  "UseCustomAnnihilation": false,
  "AnnihilationStage": "Annihilation",
  "HideUnavailableStage": false,
  "IsStageManually": false,
  "UseOptionalStage": false,
  "UseStoneAllowSave": false,
  "HideSeries": false,
  "StageResetMode": "Ignore",
  "UseWeeklySchedule": true,
  "WeeklySchedule": {
    "Sunday": false,
    "Monday": false,
    "Tuesday": true,
    "Wednesday": false,
    "Thursday": true,
    "Friday": false,
    "Saturday": true
  },
  "Name": "日常龙门币",
  "IsEnable": true,
  "TaskType": "Fight"
}"#;

    gui_task_test!(
        convert_fight_task_basic,
        fight::convert_fight_task,
        FIGHT_TASK,
        {
            "type": "Fight",
            "name": "日常经验本",
            "params": {
                "stage": "LS-6"
            }
        }
    );

    gui_task_test!(
        convert_fight_task_when_disabled,
        fight::convert_fight_task,
        DISABLED_FIGHT_TASK,
        {
            "type": "Fight",
            "name": "日常经验本",
            "params": {
                "stage": "LS-6"
            }
        }
    );

    gui_task_test!(
        convert_fight_task_weekly_schedule_ls6,
        fight::convert_fight_task,
        WEEKLY_LS6_TASK,
        {
            "type": "Fight",
            "name": "日常经验本",
            "strategy": "merge",
            "variants": [
                {
                    "condition": {
                        "type": "Weekday",
                        "weekdays": ["Mon", "Wed", "Fri"]
                    },
                    "params": {
                        "medicine_expire_days": 2,
                        "stage": "LS-6"
                    }
                }
            ]
        }
    );

    gui_task_test!(
        convert_fight_task_weekly_schedule_ce6,
        fight::convert_fight_task,
        WEEKLY_CE6_TASK,
        {
            "type": "Fight",
            "name": "日常龙门币",
            "strategy": "merge",
            "variants": [
                {
                    "condition": {
                        "type": "Weekday",
                        "weekdays": ["Tue", "Thu", "Sat"]
                    },
                    "params": {
                        "medicine_expire_days": 2,
                        "stage": "CE-6"
                    }
                }
            ]
        }
    );
}

mod start_up {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_start_up_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI StartUpTask is disabled but will still be converted");
        }

        let mut params = MAAValue::default();
        // -> client_type
        insert!(params, "client_type" => object!(
            "alternatives" => vec!["Official", "YoStarEN", "YoStarJP"]??,
            "description" => "a client type",
            "deps" => object!("start_game_enabled" => true)
        ));
        // -> start_game_enabled
        insert!(params, "start_game_enabled" => object!(
            "default" => true,
            "description" => "start the game"
        ));

        let item = object!(
            "type" => "StartUp",
            "params" => params
        );

        Ok(Some(item))
    }
}

mod fight {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_fight_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI FightTask is disabled but will still be converted");
        }

        let mut item = object!("type" => "Fight");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut weekday_condition = None;
        // UseWeeklySchedule + WeeklySchedule -> condition
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get("UseWeeklySchedule") {
            if let Some(MAAValue::Object(map)) = task.get("WeeklySchedule") {
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

                weekday_condition = Some(object!(
                    "type" => "Weekday",
                    "weekdays" => weekdays??
                ));
            }
        }

        let mut params = MAAValue::default();
        // StagePlan -> stage
        if let Some(MAAValue::Array(stages)) = task.get("StagePlan") {
            let mut stage_list = Vec::new();
            for stage in stages {
                if let MAAValue::Primitive(MAAPrimitive::String(stage)) = stage {
                    stage_list.push(stage.as_str());
                }
            }
            if stage_list.len() == 1 {
                insert!(params, "stage" => stage_list[0]);
            } else if !stage_list.is_empty() {
                insert!(params, "stage" => stage_list??);
            }
        }
        // UseExpiringMedicine + MedicineExpireDays -> medicine_expire_days
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get("UseExpiringMedicine")
        {
            if let Some(MAAValue::Primitive(MAAPrimitive::Int(days))) =
                task.get("MedicineExpireDays")
            {
                insert!(params, "medicine_expire_days" => *days);
            }
        }

        if let Some(condition) = weekday_condition {
            insert!(item, "strategy" => "merge");
            insert!(
                item,
                "variants" => vec![object!(
                    "condition" => condition,
                    "params" => params
                )]??
            );
        } else {
            insert!(item, "params" => params);
        }

        Ok(Some(item))
    }
}

mod infrast {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_infrast_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI InfrastTask is disabled but will still be converted");
        }

        let mut item = object!("type" => "Infrast");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
        // Mode -> mode
        if let Some(MAAValue::Primitive(MAAPrimitive::String(mode))) = task.get("Mode") {
            if let Some(mode) = match mode.as_str() {
                "Normal" => Some(0),
                "Custom" => Some(10000),
                "Rotation" => Some(20000),
                _ => None,
            } {
                insert!(params, "mode" => mode);
            }
        }
        // RoomList -> facility
        if let Some(MAAValue::Array(rooms)) = task.get("RoomList") {
            let mut facility = Vec::new();
            for room in rooms {
                if let MAAValue::Object(map) = room {
                    if let Some(MAAValue::Primitive(MAAPrimitive::String(room))) = map.get("Room") {
                        facility.push(room.as_str());
                    }
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
        // Filename -> filename
        if let Some(MAAValue::Primitive(MAAPrimitive::String(filename))) = task.get("Filename") {
            if !filename.is_empty() {
                insert!(params, "filename" => filename.as_str());
            }
        }
        // PlanSelect -> plan_index
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(plan_index))) = task.get("PlanSelect") {
            if *plan_index >= 0 {
                insert!(params, "plan_index" => *plan_index);
            }
        }

        insert!(item, "params" => params);
        Ok(Some(item))
    }
}

mod recruit {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_recruit_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI RecruitTask is disabled but will still be converted");
        }

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
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get("PreferTagEnabled") {
            if let Some(MAAValue::Array(tags)) = task.get("Level3PreferTags") {
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
        }
        // PreserveTagEnabled + PreserveTagList -> preserve_tags
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = task.get("PreserveTagEnabled")
        {
            if let Some(MAAValue::Array(tags)) = task.get("PreserveTagList") {
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
        }

        insert!(item, "params" => params);
        Ok(Some(item))
    }
}

mod mall {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_mall_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI MallTask is disabled but will still be converted");
        }

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
        Ok(Some(item))
    }
}

mod award {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_award_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI AwardTask is disabled but will still be converted");
        }

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
        Ok(Some(item))
    }
}

mod roguelike {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_roguelike_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI RoguelikeTask is disabled but will still be converted");
        }

        let mut item = object!("type" => "Roguelike");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
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
        if let Some(MAAValue::Primitive(MAAPrimitive::Int(difficulty))) = task.get("Difficulty") {
            if *difficulty != i32::MAX {
                insert!(params, "difficulty" => *difficulty);
            }
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
        Ok(Some(item))
    }
}

mod reclamation {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_reclamation_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI ReclamationTask is disabled but will still be converted");
        }

        let mut item = object!("type" => "Reclamation");
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        let mut params = MAAValue::default();
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
            }
        }
        // ToolToCraft -> tools_to_craft
        if let Some(MAAValue::Primitive(MAAPrimitive::String(tool))) = task.get("ToolToCraft") {
            if !tool.is_empty() {
                insert!(params, "tools_to_craft" => vec![tool.as_str()]??);
            }
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
        Ok(Some(item))
    }
}
