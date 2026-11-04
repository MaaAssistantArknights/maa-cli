use anyhow::Result;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, report_unknown_fields};

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
}

impl TryFrom<&WpfReclamationTask> for Value {
    type Error = anyhow::Error;

    fn try_from(task: &WpfReclamationTask) -> Result<Self> {
        Ok(serde_json::to_value(CliReclamationTask::try_from(task)?)?)
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_json::json;

    use super::super::migrate;

    #[test]
    fn maps_reclamation_and_skips_clear_store() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "ReclamationTask",
                    "Name": "",
                    "Theme": "Tales",
                    "Mode": "ProsperityInSave",
                    "ToolToCraft": "",
                    "IncrementMode": 0,
                    "MaxCraftCountPerRound": 16,
                    "ClearStore": true,
                    "IsEnable": false,
                }],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(summary.disabled_tasks.len(), 1);
        assert_eq!(
            summary
                .skipped_fields
                .iter()
                .map(|f| f.field.as_str())
                .collect::<Vec<_>>(),
            ["ClearStore"]
        );
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "tasks": [{
                    "type": "Reclamation",
                    "params": {
                        "theme": "Tales",
                        "mode": 1,
                        "increment_mode": 0,
                        "num_craft_batches": 16,
                        "enable": false,
                    }
                }]
            })
        );
    }

    #[test]
    fn maps_tool_to_craft_and_prosperity_no_save() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "ReclamationTask",
                    "Name": "生息",
                    "Theme": "Tales",
                    "Mode": "ProsperityNoSave",
                    "ToolToCraft": "手电钻",
                    "IncrementMode": 1,
                    "MaxCraftCountPerRound": 8,
                    "IsEnable": true,
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
                    "type": "Reclamation",
                    "name": "生息",
                    "params": {
                        "theme": "Tales",
                        "mode": 0,
                        "tools_to_craft": ["手电钻"],
                        "increment_mode": 1,
                        "num_craft_batches": 8,
                    }
                }]
            })
        );
    }

    #[test]
    fn unknown_reclamation_mode_is_skipped() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "ReclamationTask",
                    "Name": "",
                    "Theme": "Tales",
                    "Mode": "UnknownMode",
                    "ToolToCraft": "",
                    "IncrementMode": 0,
                    "MaxCraftCountPerRound": 1,
                    "IsEnable": true,
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
            ["Mode"]
        );
        assert!(config.tasks[0].pointer("/params/mode").is_none());
    }
}
