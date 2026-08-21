use anyhow::Result;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, report_unknown_fields};

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
}

impl TryFrom<&WpfRoguelikeTask> for Value {
    type Error = anyhow::Error;

    fn try_from(task: &WpfRoguelikeTask) -> Result<Self> {
        Ok(serde_json::to_value(CliRoguelikeTask::try_from(task)?)?)
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_json::json;

    use super::super::migrate;

    fn roguelike_task(mode: &str, difficulty: i32, enabled: bool) -> serde_json::Value {
        json!({
            "$type": "RoguelikeTask",
            "Name": "肉鸽",
            "IsEnable": enabled,
            "Theme": "JieGarden",
            "Mode": mode,
            "Squad": "指挥分队",
            "Roles": "稳扎稳打",
            "CoreChar": "",
            "StartCount": 999,
            "Difficulty": difficulty,
            "Investment": true,
            "InvestCount": 999,
            "InvestWithMoreScore": false,
            "StopWhenDepositFull": false,
            "StopAtFinalBoss": false,
            "StopWhenLevelMax": false,
            "UseSupport": false,
            "UseSupportNonFriend": false,
            "RefreshTraderWithDice": false,
            "StartWithEliteTwo": false,
            "StartWithEliteTwoOnly": false,
        })
    }

    #[test]
    fn maps_roguelike_and_omits_max_difficulty() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [roguelike_task("Investment", i32::MAX, false)],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(summary.disabled_tasks.len(), 1);
        assert_eq!(summary.disabled_tasks[0].type_tag, "RoguelikeTask");
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "tasks": [{
                    "type": "Roguelike",
                    "name": "肉鸽",
                    "params": {
                        "theme": "JieGarden",
                        "mode": 1,
                        "squad": "指挥分队",
                        "roles": "稳扎稳打",
                        "core_char": "",
                        "starts_count": 999,
                        "investment_enabled": true,
                        "investments_count": 999,
                        "investment_with_more_score": false,
                        "stop_when_investment_full": false,
                        "stop_at_final_boss": false,
                        "stop_at_max_level": false,
                        "use_support": false,
                        "use_nonfriend_support": false,
                        "refresh_trader_with_dice": false,
                        "start_with_elite_two": false,
                        "only_start_with_elite_two": false,
                        "enable": false,
                    }
                }]
            })
        );
    }

    #[test]
    fn maps_known_roguelike_modes() {
        for (mode, expected) in [
            ("Exp", 0),
            ("Collect", 4),
            ("CollapsalParadigms", 5),
            ("MonthlySquad", 6),
            ("DeepExploration", 7),
        ] {
            let (config, summary) = migrate(
                serde_json::from_value(json!({
                    "TaskQueue": [roguelike_task(mode, 3, true)],
                    "Gui": {},
                }))
                .unwrap(),
                None,
            )
            .unwrap();
            assert!(summary.is_empty(), "{mode}");
            assert_eq!(
                config.tasks[0].pointer("/params/mode"),
                Some(&json!(expected)),
                "{mode}"
            );
            assert_eq!(
                config.tasks[0].pointer("/params/difficulty"),
                Some(&json!(3)),
                "{mode}"
            );
        }
    }

    #[test]
    fn unknown_roguelike_mode_is_skipped() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [roguelike_task("FutureMode", 1, true)],
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
