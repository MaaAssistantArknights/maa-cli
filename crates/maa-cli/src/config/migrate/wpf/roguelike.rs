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
    "StartingOpers",
    "UseAdditionalStartingOpers",
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
    "SquadCollectible",
    "CollectibleStartAwards",
    "CollectibleShopping",
    "ExpectedCollapsalParadigms",
    "MonthlySquadAutoIterate",
    "MonthlySquadCheckComms",
    "DeepExplorationAutoIterate",
    "FindPlaytimeTarget",
    "BlackFlowCultivationTarget",
    "BlackFlowEnding",
    "BlackFlowStartAnimal1",
    "BlackFlowStartAnimal2",
    "BlackFlowStartRewards",
    "SamiFirstFloorFoldartal",
    "SamiFirstFloorFoldartals",
    "SamiNewSquad2StartingFoldartal",
    "SamiNewSquad2StartingFoldartals",
    "StartWithSeed",
    "Seed",
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
    core_char_list: Option<Vec<CliStartingOper>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collectible_mode_squad: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collectible_mode_shopping: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collectible_mode_start_list: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    monthly_squad_auto_iterate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    monthly_squad_check_comms: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deep_exploration_auto_iterate: Option<bool>,
    #[serde(
        rename = "find_playTime_target",
        skip_serializing_if = "Option::is_none"
    )]
    find_play_time_target: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blackflow_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blackflow_cultivation_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blackflow_start_animals: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_collapsal_paradigms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_floor_foldartal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_foldartal_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_with_seed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable: Option<bool>,
}

#[derive(Debug, Serialize)]
struct CliStartingOper {
    name: String,
    use_support: bool,
}

/// Newer GUI exports list starting operators here instead of flat `CoreChar` /
/// `UseSupport`. The full list maps to `core_char_list`; the first entry also
/// maps to `core_char` and `use_support`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WpfStartingOper {
    name: String,
    #[serde(default)]
    use_support: bool,
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
    /// Older exports; newer ones omit this in favor of [`Self::starting_opers`].
    #[serde(default)]
    core_char: Option<String>,
    #[serde(default)]
    starting_opers: Vec<WpfStartingOper>,
    start_count: i32,
    difficulty: i32,
    investment: bool,
    invest_count: i32,
    invest_with_more_score: bool,
    stop_when_deposit_full: bool,
    stop_at_final_boss: bool,
    stop_when_level_max: bool,
    /// Older exports; newer ones put support on each [`WpfStartingOper`].
    #[serde(default)]
    use_support: Option<bool>,
    use_support_non_friend: bool,
    refresh_trader_with_dice: bool,
    start_with_elite_two: bool,
    start_with_elite_two_only: bool,
    #[serde(default)]
    squad_collectible: String,
    #[serde(default)]
    collectible_start_awards: String,
    #[serde(default)]
    collectible_shopping: bool,
    #[serde(default)]
    expected_collapsal_paradigms: String,
    #[serde(default)]
    monthly_squad_auto_iterate: bool,
    #[serde(default)]
    monthly_squad_check_comms: bool,
    #[serde(default)]
    deep_exploration_auto_iterate: bool,
    #[serde(default)]
    find_playtime_target: String,
    #[serde(default)]
    black_flow_cultivation_target: String,
    #[serde(default)]
    black_flow_ending: String,
    #[serde(default)]
    black_flow_start_animal1: String,
    #[serde(default)]
    black_flow_start_animal2: String,
    #[serde(default)]
    black_flow_start_rewards: String,
    #[serde(default)]
    sami_first_floor_foldartal: bool,
    #[serde(default)]
    sami_first_floor_foldartals: String,
    #[serde(default)]
    sami_new_squad2_starting_foldartal: bool,
    #[serde(default)]
    sami_new_squad2_starting_foldartals: String,
    #[serde(default)]
    start_with_seed: bool,
    #[serde(default)]
    seed: String,
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
            "FindPlaytime" => Some(20001),
            "BlackFlowStartRun" => Some(30003),
            _ => None,
        }
    }

    /// Prefer `StartingOpers[0]`, then legacy `CoreChar` / `UseSupport`.
    fn starting_oper(&self) -> (Option<String>, bool) {
        if let Some(oper) = self.starting_opers.first() {
            let name = (!oper.name.is_empty()).then(|| oper.name.clone());
            return (name, oper.use_support);
        }
        (
            self.core_char
                .as_deref()
                .filter(|name| !name.is_empty())
                .map(str::to_string),
            self.use_support.unwrap_or(false),
        )
    }

    fn named_or_skip(
        &self,
        summary: &mut MigrationSummary,
        field: &str,
        raw: &str,
        mapped: Option<String>,
    ) {
        if !raw.is_empty() && raw != "None" && mapped.is_none() {
            summary.skip_field("RoguelikeTask", Some(self.name.clone()), field);
        }
    }

    fn report_unmapped(&self, summary: &mut MigrationSummary) {
        self.named_or_skip(
            summary,
            "FindPlaytimeTarget",
            &self.find_playtime_target,
            playtime_target(&self.find_playtime_target).map(|n| n.to_string()),
        );
        self.named_or_skip(
            summary,
            "BlackFlowCultivationTarget",
            &self.black_flow_cultivation_target,
            swaddled(&self.black_flow_cultivation_target).map(str::to_string),
        );
        self.named_or_skip(
            summary,
            "BlackFlowEnding",
            &self.black_flow_ending,
            ending_strategy(&self.black_flow_ending).map(str::to_string),
        );
        for (field, raw) in [
            (
                "BlackFlowStartAnimal1",
                self.black_flow_start_animal1.as_str(),
            ),
            (
                "BlackFlowStartAnimal2",
                self.black_flow_start_animal2.as_str(),
            ),
        ] {
            self.named_or_skip(summary, field, raw, swaddled(raw).map(str::to_string));
        }
        if !self.black_flow_start_rewards.is_empty() && self.black_flow_start_rewards != "None" {
            summary.skip_field(
                "RoguelikeTask",
                Some(self.name.clone()),
                "BlackFlowStartRewards",
            );
        }
        self.named_or_skip(
            summary,
            "CollectibleStartAwards",
            &self.collectible_start_awards,
            award_key(&self.collectible_start_awards).map(str::to_string),
        );
    }

    fn core_char_list(&self) -> Option<Vec<CliStartingOper>> {
        let list: Vec<CliStartingOper> = self
            .starting_opers
            .iter()
            .filter(|oper| !oper.name.is_empty())
            .map(|oper| CliStartingOper {
                name: oper.name.clone(),
                use_support: oper.use_support,
            })
            .collect();
        (!list.is_empty()).then_some(list)
    }

    fn start_animals(&self) -> Option<Vec<String>> {
        let animals: Vec<String> = [
            &self.black_flow_start_animal1,
            &self.black_flow_start_animal2,
        ]
        .into_iter()
        .filter_map(|name| swaddled(name))
        .map(str::to_string)
        .collect();
        (!animals.is_empty()).then_some(animals)
    }

    pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
        if !self.is_enable {
            summary.disable_task("RoguelikeTask", Some(self.name.clone()));
        }
        if self.mapped_mode().is_none() {
            summary.skip_field("RoguelikeTask", Some(self.name.clone()), "Mode");
        }
        self.report_unmapped(summary);
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
        let (core_char, use_support) = task.starting_oper();
        Ok(CliRoguelikeTask {
            task_type: TaskType::Roguelike,
            name: task.name.clone(),
            params: CliRoguelikeParams {
                theme: Some(task.theme.clone()),
                mode: task.mapped_mode(),
                squad: Some(task.squad.clone()),
                roles: Some(task.roles.clone()),
                core_char,
                starts_count: Some(task.start_count),
                difficulty: (task.difficulty != i32::MAX).then_some(task.difficulty),
                investment_enabled: Some(task.investment),
                investments_count: Some(task.invest_count),
                investment_with_more_score: Some(task.invest_with_more_score),
                stop_when_investment_full: Some(task.stop_when_deposit_full),
                stop_at_final_boss: Some(task.stop_at_final_boss),
                stop_at_max_level: Some(task.stop_when_level_max),
                use_support: Some(use_support),
                use_nonfriend_support: Some(task.use_support_non_friend),
                refresh_trader_with_dice: Some(task.refresh_trader_with_dice),
                start_with_elite_two: Some(task.start_with_elite_two),
                only_start_with_elite_two: Some(task.start_with_elite_two_only),
                core_char_list: task.core_char_list(),
                collectible_mode_squad: (!task.squad_collectible.is_empty())
                    .then(|| task.squad_collectible.clone()),
                collectible_mode_shopping: task.collectible_shopping.then_some(true),
                collectible_mode_start_list: award_key(&task.collectible_start_awards).map(|key| {
                    let mut list = Map::new();
                    list.insert(key.to_string(), Value::Bool(true));
                    list
                }),
                monthly_squad_auto_iterate: task.monthly_squad_auto_iterate.then_some(true),
                monthly_squad_check_comms: task.monthly_squad_check_comms.then_some(true),
                deep_exploration_auto_iterate: task.deep_exploration_auto_iterate.then_some(true),
                find_play_time_target: playtime_target(&task.find_playtime_target),
                blackflow_strategy: ending_strategy(&task.black_flow_ending).map(str::to_string),
                blackflow_cultivation_target: swaddled(&task.black_flow_cultivation_target)
                    .map(str::to_string),
                blackflow_start_animals: task.start_animals(),
                expected_collapsal_paradigms: split_list(&task.expected_collapsal_paradigms),
                first_floor_foldartal: (task.sami_first_floor_foldartal
                    && !task.sami_first_floor_foldartals.is_empty())
                .then(|| task.sami_first_floor_foldartals.clone()),
                start_foldartal_list: (task.sami_new_squad2_starting_foldartal)
                    .then(|| split_list(&task.sami_new_squad2_starting_foldartals))
                    .flatten(),
                start_with_seed: (task.start_with_seed && !task.seed.is_empty())
                    .then(|| task.seed.clone()),
                enable: (!task.is_enable).then_some(false),
            },
        })
    }
}

fn playtime_target(name: &str) -> Option<i32> {
    match name {
        "Ling" => Some(1),
        "Shu" => Some(2),
        "Nian" => Some(3),
        _ => None,
    }
}

fn swaddled(name: &str) -> Option<&'static str> {
    match name {
        "Cat" => Some("swaddled_cat"),
        "Dog" => Some("swaddled_dog"),
        "Cerberus" => Some("swaddled_cerberus"),
        "FeatheredSerpent" => Some("swaddled_feathered_serpent"),
        _ => None,
    }
}

fn ending_strategy(name: &str) -> Option<&'static str> {
    match name {
        "Ending1" => Some("ending1"),
        "Ending2" => Some("ending2"),
        "Ending3" => Some("ending3"),
        _ => None,
    }
}

fn award_key(name: &str) -> Option<&'static str> {
    match name {
        "HotWater" => Some("hot_water"),
        "Shield" => Some("shield"),
        "Ingot" => Some("ingot"),
        "Hope" => Some("hope"),
        "Random" => Some("random"),
        "Key" => Some("key"),
        "Dice" => Some("dice"),
        "Ideas" | "Idea" => Some("ideas"),
        _ => None,
    }
}

fn split_list(raw: &str) -> Option<Vec<String>> {
    let items: Vec<String> = raw
        .split([';', ',', '；'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    (!items.is_empty()).then_some(items)
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
            "CoreChar": "维什戴尔",
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
                        "core_char": "维什戴尔",
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
    fn maps_starting_opers_when_core_char_absent() {
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [{
                    "$type": "RoguelikeTask",
                    "Name": "黑流树海-刷源石锭",
                    "IsEnable": true,
                    "Theme": "BlackFlow",
                    "Mode": "Investment",
                    "Squad": "特勤分队",
                    "Roles": "稳扎稳打",
                    "StartCount": 99999,
                    "StartingOpers": [
                        { "Name": "机械师", "UseSupport": true },
                        { "Name": "额外干员", "UseSupport": false },
                    ],
                    "Difficulty": i32::MAX,
                    "Investment": true,
                    "InvestCount": 999,
                    "InvestWithMoreScore": false,
                    "StopWhenDepositFull": true,
                    "StopAtFinalBoss": false,
                    "StopWhenLevelMax": false,
                    "UseSupportNonFriend": false,
                    "RefreshTraderWithDice": false,
                    "StartWithEliteTwo": false,
                    "StartWithEliteTwoOnly": false,
                }],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert!(summary.skipped_fields.is_empty());
        assert_eq!(
            config.tasks[0].pointer("/params/core_char_list"),
            Some(&json!([
                { "name": "机械师", "use_support": true },
                { "name": "额外干员", "use_support": false },
            ]))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/core_char"),
            Some(&json!("机械师"))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/use_support"),
            Some(&json!(true))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/theme"),
            Some(&json!("BlackFlow"))
        );
    }

    #[test]
    fn accepts_roguelike_without_core_char_or_starting_opers() {
        let mut task = roguelike_task("Exp", 1, true);
        task.as_object_mut().unwrap().remove("CoreChar");
        task.as_object_mut().unwrap().remove("UseSupport");
        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [task],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();
        assert!(summary.is_empty());
        assert!(config.tasks[0].pointer("/params/core_char").is_none());
        assert_eq!(
            config.tasks[0].pointer("/params/use_support"),
            Some(&json!(false))
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
            ("FindPlaytime", 20001),
            ("BlackFlowStartRun", 30003),
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
    fn maps_blackflow_protocol_fields() {
        let mut task = roguelike_task("BlackFlowStartRun", i32::MAX, true);
        let obj = task.as_object_mut().unwrap();
        obj.insert("Theme".into(), json!("BlackFlow"));
        obj.insert("SquadCollectible".into(), json!("指挥分队"));
        obj.insert("CollectibleStartAwards".into(), json!("HotWater"));
        obj.insert("FindPlaytimeTarget".into(), json!("Ling"));
        obj.insert("BlackFlowCultivationTarget".into(), json!("Cerberus"));
        obj.insert("BlackFlowEnding".into(), json!("Ending1"));
        obj.insert("BlackFlowStartAnimal1".into(), json!("Cerberus"));
        obj.insert("BlackFlowStartAnimal2".into(), json!("None"));
        obj.insert("BlackFlowStartRewards".into(), json!("None"));
        obj.insert("MonthlySquadAutoIterate".into(), json!(true));
        obj.remove("CoreChar");
        obj.insert(
            "StartingOpers".into(),
            json!([{ "Name": "机械师", "UseSupport": false }]),
        );

        let (config, summary) = migrate(
            serde_json::from_value(json!({
                "TaskQueue": [task],
                "Gui": {},
            }))
            .unwrap(),
            None,
        )
        .unwrap();

        assert!(summary.skipped_fields.is_empty());
        assert_eq!(config.tasks[0].pointer("/params/mode"), Some(&json!(30003)));
        assert!(config.tasks[0].pointer("/params/difficulty").is_none());
        assert_eq!(
            config.tasks[0].pointer("/params/blackflow_start_animals"),
            Some(&json!(["swaddled_cerberus"]))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/blackflow_cultivation_target"),
            Some(&json!("swaddled_cerberus"))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/blackflow_strategy"),
            Some(&json!("ending1"))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/find_playTime_target"),
            Some(&json!(1))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/collectible_mode_squad"),
            Some(&json!("指挥分队"))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/collectible_mode_start_list/hot_water"),
            Some(&json!(true))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/monthly_squad_auto_iterate"),
            Some(&json!(true))
        );
        assert_eq!(
            config.tasks[0].pointer("/params/core_char_list/0/name"),
            Some(&json!("机械师"))
        );
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
