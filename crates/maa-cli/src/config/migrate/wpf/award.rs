use anyhow::Result;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, report_unknown_fields};

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
}

impl TryFrom<&WpfAwardTask> for Value {
    type Error = anyhow::Error;

    fn try_from(task: &WpfAwardTask) -> Result<Self> {
        Ok(serde_json::to_value(CliAwardTask::try_from(task)?)?)
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_json::json;

    use super::super::migrate;

    #[test]
    fn maps_award_fields_including_renames() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "AwardTask",
                    "Name": "领奖",
                    "IsEnable": true,
                    "Award": true,
                    "Mail": true,
                    "FreeGacha": false,
                    "Orundum": true,
                    "Mining": true,
                    "SpecialAccess": true,
                }],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert!(summary.is_empty());
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "tasks": [{
                    "type": "Award",
                    "name": "领奖",
                    "params": {
                        "award": true,
                        "mail": true,
                        "recruit": false,
                        "orundum": true,
                        "mining": true,
                        "specialaccess": true,
                    }
                }]
            })
        );
    }

    #[test]
    fn disabled_task_sets_enable_false_and_is_reported() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "AwardTask",
                    "Name": "领奖",
                    "IsEnable": false,
                    "Award": true,
                    "Mail": true,
                    "FreeGacha": false,
                    "Orundum": true,
                    "Mining": true,
                    "SpecialAccess": true,
                }],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(summary.disabled_tasks.len(), 1);
        assert_eq!(summary.disabled_tasks[0].type_tag, "AwardTask");
        assert_eq!(summary.disabled_tasks[0].name.as_deref(), Some("领奖"));
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "tasks": [{
                    "type": "Award",
                    "name": "领奖",
                    "params": {
                        "award": true,
                        "mail": true,
                        "recruit": false,
                        "orundum": true,
                        "mining": true,
                        "specialaccess": true,
                        "enable": false,
                    }
                }]
            })
        );
    }

    #[test]
    fn unknown_fields_are_reported() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "AwardTask",
                    "Name": "领奖",
                    "IsEnable": true,
                    "Award": true,
                    "Mail": true,
                    "FreeGacha": false,
                    "Orundum": true,
                    "Mining": true,
                    "SpecialAccess": true,
                    "SomeFutureFlag": true,
                }],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(
            summary
                .skipped_fields
                .iter()
                .map(|f| f.field.as_str())
                .collect::<Vec<_>>(),
            ["SomeFutureFlag"]
        );
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "tasks": [{
                    "type": "Award",
                    "name": "领奖",
                    "params": {
                        "award": true,
                        "mail": true,
                        "recruit": false,
                        "orundum": true,
                        "mining": true,
                        "specialaccess": true,
                    }
                }]
            })
        );
    }
}
