use anyhow::{Context, Result, bail};
use maa_value::prelude::*;

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
        "StartUpTask" => StartUp::convert_start_up_task(task),
        "FightTask" => Fight::convert_fight_task(task),
        _ => Ok(None),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{fs::File, path::Path};

    use super::*;

    const DEFAULT_PROFILE: &str = include_str!("../../fixtures/gui/default_profile.json");

    fn write_json(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn read_json(path: &Path) -> serde_json::Value {
        serde_json::from_reader(File::open(path).unwrap()).unwrap()
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
    fn json_to_json_default_profile() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_json(dir.path(), "profile.json", DEFAULT_PROFILE);
        let output = dir.path().join("tasks.json");

        let input: MAAValue = serde_json::from_reader(File::open(&input).unwrap()).unwrap();
        let value = convert(input).unwrap();
        let file = File::create(&output).unwrap();
        serde_json::to_writer_pretty(file, &value).unwrap();

        assert_eq!(
            read_json(&output),
            serde_json::json!({
                "tasks": [
                    {
                        "type": "StartUp",
                        "name": "",
                        "params": {
                            "start_game_enabled": true,
                            "account_name": ""
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
                    }
                ]
            })
        );
    }
}

mod StartUp {
    use anyhow::Result;
    use maa_value::prelude::*;

    pub(super) fn convert_start_up_task(task: &MAAValue) -> Result<Option<MAAValue>> {
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(false))) = task.get("IsEnable") {
            log::warn!("GUI StartUpTask is disabled but will still be converted");
        }

        let mut params = object!("start_game_enabled" => true);
        // -> account_name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(account))) =
            task.get("AccountName")
        {
            insert!(params, "account_name" => account.as_str());
        }

        let mut item = object!(
            "type" => "StartUp",
            "params" => params
        );
        // -> task name
        if let Some(MAAValue::Primitive(MAAPrimitive::String(name))) = task.get("Name") {
            insert!(item, "name" => name.as_str());
        }

        Ok(Some(item))
    }

    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    mod tests {
        use std::{fs::File, path::Path};

        use super::*;

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

        fn write_task_to_file(dir: &Path, name: &str, task_json: &str) -> std::path::PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, task_json).unwrap();
            path
        }

        fn read_task_from_file(path: &Path) -> MAAValue {
            serde_json::from_reader(File::open(path).unwrap()).unwrap()
        }

        fn write_output_json(output: &Path, value: &MAAValue) {
            let file = File::create(output).unwrap();
            serde_json::to_writer_pretty(file, value).unwrap();
        }

        fn read_json_file(path: &Path) -> serde_json::Value {
            serde_json::from_reader(File::open(path).unwrap()).unwrap()
        }

        #[test]
        fn basic() {
            let dir = tempfile::tempdir().unwrap();
            let input = write_task_to_file(dir.path(), "startup.json", STARTUP_TASK);
            let output = dir.path().join("task.json");

            let task = read_task_from_file(&input);
            let value = super::convert_start_up_task(&task).unwrap().unwrap();
            write_output_json(&output, &value);

            assert_eq!(
                read_json_file(&output),
                serde_json::json!({
                    "type": "StartUp",
                    "name": "",
                    "params": {
                        "start_game_enabled": true,
                        "account_name": ""
                    }
                })
            );
        }

        #[test]
        fn with_account_and_name() {
            let dir = tempfile::tempdir().unwrap();
            let input = write_task_to_file(dir.path(), "startup.json", STARTUP_TASK_WITH_ACCOUNT);
            let output = dir.path().join("task.json");

            let task = read_task_from_file(&input);
            let value = super::convert_start_up_task(&task).unwrap().unwrap();
            write_output_json(&output, &value);

            assert_eq!(
                read_json_file(&output),
                serde_json::json!({
                    "type": "StartUp",
                    "name": "启动游戏",
                    "params": {
                        "start_game_enabled": true,
                        "account_name": "123****4567"
                    }
                })
            );
        }
    }
}

mod Fight {
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
                    if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) = map.get(gui_day)
                    {
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
        if let Some(MAAValue::Primitive(MAAPrimitive::Bool(true))) =
            task.get("UseExpiringMedicine")
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

    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    mod tests {
        use std::{fs::File, path::Path};

        use super::*;

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

        fn write_task_to_file(dir: &Path, name: &str, task_json: &str) -> std::path::PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, task_json).unwrap();
            path
        }

        fn read_task_from_file(path: &Path) -> MAAValue {
            serde_json::from_reader(File::open(path).unwrap()).unwrap()
        }

        fn write_output_json(output: &Path, value: &MAAValue) {
            let file = File::create(output).unwrap();
            serde_json::to_writer_pretty(file, value).unwrap();
        }

        fn read_json_file(path: &Path) -> serde_json::Value {
            serde_json::from_reader(File::open(path).unwrap()).unwrap()
        }

        #[test]
        fn basic() {
            let dir = tempfile::tempdir().unwrap();
            let input = write_task_to_file(dir.path(), "fight.json", FIGHT_TASK);
            let output = dir.path().join("task.json");

            let task = read_task_from_file(&input);
            let value = super::convert_fight_task(&task).unwrap().unwrap();
            write_output_json(&output, &value);

            assert_eq!(
                read_json_file(&output),
                serde_json::json!({
                    "type": "Fight",
                    "name": "日常经验本",
                    "params": {
                        "stage": "LS-6"
                    }
                })
            );
        }

        #[test]
        fn when_disabled() {
            let dir = tempfile::tempdir().unwrap();
            let input = write_task_to_file(dir.path(), "fight.json", DISABLED_FIGHT_TASK);
            let output = dir.path().join("task.json");

            let task = read_task_from_file(&input);
            let value = super::convert_fight_task(&task).unwrap().unwrap();
            write_output_json(&output, &value);

            assert_eq!(
                read_json_file(&output),
                serde_json::json!({
                    "type": "Fight",
                    "name": "日常经验本",
                    "params": {
                        "stage": "LS-6"
                    }
                })
            );
        }

        #[test]
        fn weekly_schedule_ls6() {
            let dir = tempfile::tempdir().unwrap();
            let input = write_task_to_file(dir.path(), "fight.json", WEEKLY_LS6_TASK);
            let output = dir.path().join("task.json");

            let task = read_task_from_file(&input);
            let value = super::convert_fight_task(&task).unwrap().unwrap();
            write_output_json(&output, &value);

            assert_eq!(
                read_json_file(&output),
                serde_json::json!({
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
                })
            );
        }

        #[test]
        fn weekly_schedule_ce6() {
            let dir = tempfile::tempdir().unwrap();
            let input = write_task_to_file(dir.path(), "fight.json", WEEKLY_CE6_TASK);
            let output = dir.path().join("task.json");

            let task = read_task_from_file(&input);
            let value = super::convert_fight_task(&task).unwrap().unwrap();
            write_output_json(&output, &value);

            assert_eq!(
                read_json_file(&output),
                serde_json::json!({
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
                })
            );
        }
    }
}
