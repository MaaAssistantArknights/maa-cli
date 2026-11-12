use anyhow::Result;
use log::warn;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, WpfGuiSettings};
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
/// converting to [`CliStartUpTask`].
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

    pub(super) fn to_cli_task(&self, gui: &WpfGuiSettings) -> Result<impl Serialize> {
        CliStartUpTask::try_from((self, gui))
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_json::json;

    use super::super::migrate;
    use crate::config::task::ClientType;

    #[test]
    fn startup_uses_runtime_settings() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "StartUpTask",
                    "Name": "",
                    "IsEnable": true,
                    "AccountSwitchEnabled": false,
                }],
                "Gui": {
                    "RuntimeSettings": {
                        "ClientType": "Official",
                        "StartGame": true,
                    }
                },
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
                    "type": "StartUp",
                    "params": {
                        "client_type": "Official",
                        "start_game_enabled": true,
                    }
                }]
            })
        );
    }

    #[test]
    fn missing_or_invalid_client_type_uses_prompt() {
        for client_type in [json!(null), json!(""), json!("NotAClient")] {
            let mut gui = json!({ "Gui": { "RuntimeSettings": {} } });
            if !client_type.is_null() {
                gui["Gui"]["RuntimeSettings"]["ClientType"] = client_type.clone();
            }
            let (config, _) = migrate(
                serde_json::from_value(json!({
                    "TaskQueue": [{
                        "$type": "StartUpTask",
                        "Name": "启动",
                        "IsEnable": true,
                        "AccountSwitchEnabled": false,
                    }],
                    "Gui": gui["Gui"],
                }))
                .unwrap(),
                None,
            )
            .unwrap();

            assert_eq!(
                config.tasks[0].pointer("/params/client_type"),
                Some(&json!({ "alternatives": ClientType::NAMES })),
                "{client_type}"
            );
        }
    }

    #[test]
    fn account_switch_maps_name_or_reports_missing() {
        let (with_name, summary_ok) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "StartUpTask",
                    "Name": "",
                    "IsEnable": true,
                    "AccountSwitchEnabled": true,
                    "AccountName": "main",
                }],
                "Gui": {
                    "RuntimeSettings": { "ClientType": "Bilibili", "StartGame": false }
                },
            }))
            .unwrap(),
            None,
        )
        .unwrap();
        assert!(summary_ok.is_empty());
        assert_eq!(
            with_name.tasks[0].pointer("/params/account_name"),
            Some(&json!("main"))
        );

        let (without_name, summary_skip) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "StartUpTask",
                    "Name": "",
                    "IsEnable": false,
                    "AccountSwitchEnabled": true,
                }],
                "Gui": {
                    "RuntimeSettings": { "ClientType": "Official" }
                },
            }))
            .unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(summary_skip.disabled_tasks.len(), 1);
        assert_eq!(
            summary_skip
                .skipped_fields
                .iter()
                .map(|f| f.field.as_str())
                .collect::<Vec<_>>(),
            ["AccountName"]
        );
        assert!(
            without_name.tasks[0]
                .pointer("/params/account_name")
                .is_none()
        );
        assert_eq!(
            without_name.tasks[0].pointer("/params/enable"),
            Some(&json!(false))
        );
    }
}
