use anyhow::Result;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, report_unknown_fields, split_semi_list};

const HANDLED: &[&str] = &[
    "Shopping",
    "CreditFight",
    "CreditFightFormation",
    "VisitFriends",
    "FirstList",
    "BlackList",
    "ShoppingIgnoreBlackListWhenFull",
    "OnlyBuyDiscount",
    "ReserveMaxCredit",
];

#[derive(Debug, Serialize)]
struct CliMallTask {
    #[serde(rename = "type")]
    task_type: TaskType,
    #[serde(skip_serializing_if = "str::is_empty")]
    name: String,
    params: CliMallParams,
}

#[derive(Debug, Serialize)]
struct CliMallParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    shopping: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credit_fight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formation_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visit_friends: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    buy_first: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    blacklist: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    force_shopping_if_credit_full: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    only_buy_discount: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reserve_max_credit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable: Option<bool>,
}

/// WPF GUI `MallTask` (`$type = "MallTask"`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct WpfMallTask {
    name: String,
    is_enable: bool,
    shopping: bool,
    credit_fight: bool,
    credit_fight_formation: i32,
    visit_friends: bool,
    first_list: String,
    black_list: String,
    shopping_ignore_black_list_when_full: bool,
    only_buy_discount: bool,
    reserve_max_credit: bool,
    #[serde(flatten)]
    unknown: Map<String, Value>,
}

impl WpfMallTask {
    pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
        if !self.is_enable {
            summary.disable_task("MallTask", Some(self.name.clone()));
        }
        report_unknown_fields(
            summary,
            "MallTask",
            Some(self.name.clone()),
            &self.unknown,
            HANDLED,
        );
    }
}

impl TryFrom<&WpfMallTask> for Value {
    type Error = anyhow::Error;

    fn try_from(task: &WpfMallTask) -> Result<Self> {
        Ok(serde_json::to_value(CliMallTask::try_from(task)?)?)
    }
}

impl TryFrom<&WpfMallTask> for CliMallTask {
    type Error = anyhow::Error;

    fn try_from(task: &WpfMallTask) -> Result<Self> {
        Ok(CliMallTask {
            task_type: TaskType::Mall,
            name: task.name.clone(),
            params: CliMallParams {
                shopping: Some(task.shopping),
                credit_fight: Some(task.credit_fight),
                formation_index: Some(task.credit_fight_formation),
                visit_friends: Some(task.visit_friends),
                buy_first: split_semi_list(&task.first_list),
                blacklist: split_semi_list(&task.black_list),
                force_shopping_if_credit_full: Some(task.shopping_ignore_black_list_when_full),
                only_buy_discount: Some(task.only_buy_discount),
                reserve_max_credit: Some(task.reserve_max_credit),
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
    fn maps_mall_fields_and_splits_semi_lists() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "MallTask",
                    "Name": "",
                    "IsEnable": true,
                    "Shopping": true,
                    "CreditFight": false,
                    "CreditFightFormation": 0,
                    "VisitFriends": true,
                    "FirstList": "加急许可;招聘许可",
                    "BlackList": "碳;家具;",
                    "ShoppingIgnoreBlackListWhenFull": true,
                    "OnlyBuyDiscount": false,
                    "ReserveMaxCredit": false,
                    "CreditFightLastTime": "2026/06/08 00:00:00",
                    "IsCreditFightAvailable": false,
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
            ["CreditFightLastTime"]
        );
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "tasks": [{
                    "type": "Mall",
                    "params": {
                        "shopping": true,
                        "credit_fight": false,
                        "formation_index": 0,
                        "visit_friends": true,
                        "buy_first": ["加急许可", "招聘许可"],
                        "blacklist": ["碳", "家具"],
                        "force_shopping_if_credit_full": true,
                        "only_buy_discount": false,
                        "reserve_max_credit": false,
                    }
                }]
            })
        );
    }

    #[test]
    fn empty_semi_lists_omit_arrays() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "MallTask",
                    "Name": "信用",
                    "IsEnable": false,
                    "Shopping": false,
                    "CreditFight": false,
                    "CreditFightFormation": 1,
                    "VisitFriends": false,
                    "FirstList": "",
                    "BlackList": "",
                    "ShoppingIgnoreBlackListWhenFull": false,
                    "OnlyBuyDiscount": true,
                    "ReserveMaxCredit": true,
                }],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(summary.disabled_tasks.len(), 1);
        assert_eq!(summary.disabled_tasks[0].type_tag, "MallTask");
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            json!({
                "tasks": [{
                    "type": "Mall",
                    "name": "信用",
                    "params": {
                        "shopping": false,
                        "credit_fight": false,
                        "formation_index": 1,
                        "visit_friends": false,
                        "force_shopping_if_credit_full": false,
                        "only_buy_discount": true,
                        "reserve_max_credit": true,
                        "enable": false,
                    }
                }]
            })
        );
    }
}
