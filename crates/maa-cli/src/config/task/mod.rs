mod client_type;
pub use client_type::ClientType;

mod condition;
use condition::Condition;
pub use condition::{TimeOffset, remainder_of_day_mod};

mod init;

use maa_types::TaskType;
use maa_value::MAAValue;
use serde::Deserialize;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TaskVariant {
    #[serde(default)]
    condition: Condition,
    #[serde(default)]
    params: MAAValue,
}

impl TaskVariant {
    // This constructor seems to be useless,
    // because predefined task always active and ask params from user.
    // Variant is only used in user-defined task.
    // pub fn new(condition: Condition, params: Value) -> Self {
    //     Self { condition, params }
    // }

    pub fn is_active(&self) -> bool {
        self.condition.is_active()
    }

    pub fn params(&self) -> &MAAValue {
        &self.params
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
/// How to select params from different variants
///
/// If the strategy is `First`, the params from the first active variant will be used.
/// If the strategy is `Merge`, the params from all active variants will be merged,
/// and the params from the later variants will override the params from the earlier variants.
/// The default strategy is `First`.
pub enum Strategy {
    #[default]
    First,
    Merge,
}

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    task_type: TaskType,
    #[serde(default)]
    params: MAAValue,
    #[serde(default)]
    strategy: Strategy,
    #[serde(default)]
    variants: Vec<TaskVariant>,
}

// Constructor for Task
impl Task {
    pub fn new(task_type: TaskType, params: MAAValue) -> Self {
        Self {
            name: None,
            task_type,
            strategy: Strategy::default(),
            params,
            variants: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    #[cfg(test)]
    pub fn with_strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    #[cfg(test)]
    pub fn with_variants(mut self, variants: Vec<TaskVariant>) -> Self {
        self.variants = variants;
        self
    }

    #[cfg(test)]
    pub fn push_variant(&mut self, variants: TaskVariant) -> &mut Self {
        self.variants.push(variants);
        self
    }

    pub fn task_type(&self) -> TaskType {
        self.task_type
    }

    pub fn is_active(&self) -> bool {
        if self.variants.is_empty() {
            return true;
        }
        for variant in self.variants.iter() {
            if variant.is_active() {
                return true;
            }
        }
        false
    }

    pub fn params(&self) -> MAAValue {
        let mut params = self.params.clone();
        for variant in &self.variants {
            if variant.is_active() {
                params.merge_from(variant.params());
                if matches!(self.strategy, Strategy::First) {
                    break;
                }
            }
        }
        params
    }
}

#[derive(Deserialize)]
pub struct TaskConfig {
    client_type: Option<ClientType>,
    startup: Option<bool>,
    closedown: Option<bool>,
    tasks: Vec<Task>,
}

impl TaskConfig {
    pub fn new_with_tasks(tasks: Vec<Task>) -> Self {
        Self {
            client_type: None,
            startup: None,
            closedown: None,
            tasks,
        }
    }
}


#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_value::object;

    use super::*;

    mod task {
        use super::*;

        #[test]
        fn is_active() {
            fn test_with_veriants(variants: Vec<TaskVariant>, expected: bool) {
                assert_eq!(
                    Task::new(TaskType::StartUp, object!())
                        .with_variants(variants)
                        .is_active(),
                    expected
                );
            }

            fn always_active() -> TaskVariant {
                TaskVariant {
                    condition: Condition::Always,
                    params: MAAValue::default(),
                }
            }

            fn never_active() -> TaskVariant {
                TaskVariant {
                    condition: Condition::Not {
                        condition: Box::new(Condition::Always),
                    },
                    params: MAAValue::default(),
                }
            }

            test_with_veriants(vec![always_active()], true);
            test_with_veriants(vec![never_active()], false);
            test_with_veriants(vec![always_active(), never_active()], true);
            test_with_veriants(vec![never_active(), always_active()], true);
            test_with_veriants(vec![never_active(), never_active()], false);
        }

        #[test]
        fn get_type() {
            assert_eq!(
                Task::new(TaskType::StartUp, object!()).task_type(),
                TaskType::StartUp,
            );
        }

        #[test]
        fn get_params() {
            fn test_with_variants(
                base: MAAValue,
                strategy: Strategy,
                variants: impl IntoIterator<Item = MAAValue>,
                expected: MAAValue,
            ) {
                let mut task = Task::new(TaskType::StartUp, base).with_strategy(strategy);
                for v in variants {
                    task.push_variant(TaskVariant {
                        condition: Condition::Always,
                        params: v,
                    });
                }

                assert_eq!(task.params(), expected);
            }

            test_with_variants(
                object!("a" => 1),
                Strategy::First,
                vec![],
                object!("a" => 1),
            );

            test_with_variants(
                object!("a" => 1),
                Strategy::First,
                vec![object!()],
                object!("a" => 1),
            );

            test_with_variants(
                object!(),
                Strategy::First,
                vec![object!("a" => 1)],
                object!("a" => 1),
            );

            test_with_variants(
                object!("a" => 1),
                Strategy::First,
                vec![object!("b" => 2)],
                object!("a" => 1, "b" => 2),
            );

            test_with_variants(
                object!("a" => 1),
                Strategy::First,
                vec![object!("a" => 2)],
                object!("a" => 2),
            );

            test_with_variants(
                object!("a" => 1),
                Strategy::First,
                vec![object!("a" => 2), object!("a" => 3)],
                object!("a" => 2),
            );

            test_with_variants(
                object!("a" => 1),
                Strategy::Merge,
                vec![object!("a" => 2), object!("a" => 3)],
                object!("a" => 3),
            );

            test_with_variants(
                object!("a" => 1),
                Strategy::First,
                vec![object!("a" => 2), object!("b" => 4)],
                object!("a" => 2),
            );

            test_with_variants(
                object!("a" => 1),
                Strategy::Merge,
                vec![object!("a" => 2), object!("b" => 4)],
                object!("a" => 2, "b" => 4),
            );

            assert_eq!(
                {
                    let mut task = Task::new(TaskType::StartUp, object!("a" => 1, "c" => 5))
                        .with_strategy(Strategy::First);
                    task.push_variant(TaskVariant {
                        condition: Condition::Not {
                            condition: Box::new(Condition::Always),
                        },
                        params: object!("a" => 2),
                    });
                    task.push_variant(TaskVariant {
                        condition: Condition::Always,
                        params: object!("a" => 3, "b" => 4),
                    });
                    task.params()
                },
                object!("a" => 3, "b" => 4, "c" => 5),
            );
        }
    }

    mod task_config {
        use TaskType::*;

        use super::*;

        mod serde {
            use chrono::{NaiveDateTime, NaiveTime, TimeZone, Weekday};
            use condition::TimeOffset;
            use maa_value::userinput::{BoolInput, Input, SelectD};

            use super::*;

            fn naive_local_datetime(
                y: i32,
                m: u32,
                d: u32,
                h: u32,
                mi: u32,
                s: u32,
            ) -> NaiveDateTime {
                chrono::Local
                    .with_ymd_and_hms(y, m, d, h, mi, s)
                    .unwrap()
                    .naive_local()
            }

            fn example_task_config() -> TaskConfig {
                use ClientType::*;

                let mut task_list = Vec::new();

                task_list.push(Task::new(
                    StartUp,
                    object!(
                        "start_game_enabled" => BoolInput::new(
                            Some(true),
                        ).with_description("start the game"),
                        "client_type" if "start_game_enabled" == true =>
                            SelectD::<String>::from_iter(
                                [
                                    Official.to_str(),
                                    YoStarEN.to_str(),
                                    YoStarJP.to_str(),
                                ],
                                None,
                            ).unwrap().with_description("a client type"),
                    ),
                ));

                task_list.push(
                    Task::new(Fight, object!())
                        .with_name("Fight Daily".to_string())
                        .with_strategy(Strategy::Merge)
                        .with_variants(vec![
                            TaskVariant {
                                condition: Condition::Weekday {
                                    weekdays: vec![Weekday::Sun],
                                    timezone: TimeOffset::Local,
                                },
                                params: object!("expiring_medicine" => 5),
                            },
                            TaskVariant {
                                condition: Condition::Always,
                                params: object!(
                                    "stage" => Input::new(
                                        Some("1-7".to_string()),
                                    ).with_description("a stage to fight"),
                                ),
                            },
                            TaskVariant {
                                condition: Condition::Weekday {
                                    weekdays: vec![Weekday::Tue, Weekday::Thu, Weekday::Sat],
                                    timezone: TimeOffset::Client(ClientType::Official),
                                },
                                params: object!("stage" => "CE-6"),
                            },
                            TaskVariant {
                                condition: Condition::DateTime {
                                    start: Some(naive_local_datetime(2023, 8, 1, 16, 0, 0)),
                                    end: Some(naive_local_datetime(2023, 8, 21, 3, 59, 59)),
                                    timezone: TimeOffset::TimeZone(8),
                                },
                                params: object!(
                                    "stage" => SelectD::<String>::from_iter(
                                        [
                                            "SL-6",
                                            "SL-7",
                                            "SL-8",
                                        ],
                                        std::num::NonZero::new(2),
                                    ).unwrap()
                                    .with_description("a stage to fight in summer event")
                                    .with_allow_custom(true),
                                ),
                            },
                        ]),
                );

                task_list.push(
                    Task::new(
                        Mall,
                        object!(
                            "shopping" => true,
                            "credit_fight" => true,
                            "buy_first" => [
                                "招聘许可",
                                "龙门币",
                            ],
                            "blacklist" => [
                                "碳",
                                "家具",
                                "加急许可",
                            ],
                        ),
                    )
                    .with_variants(vec![TaskVariant {
                        condition: Condition::Time {
                            start: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
                            end: None,
                            timezone: TimeOffset::Local,
                        },
                        params: object!(),
                    }]),
                );

                task_list.push(Task::new(CloseDown, object!()));

                TaskConfig::new_with_tasks(task_list)
            }

            #[test]
            fn json() {
                let task_config: TaskConfig = serde_json::from_reader(
                    std::fs::File::open("./config_examples/tasks/daily.json").unwrap(),
                )
                .unwrap();
                assert_eq!(task_config.tasks, example_task_config().tasks)
            }

            #[test]
            fn toml() {
                let task_config: TaskConfig = toml::from_str(
                    &std::fs::read_to_string("./config_examples/tasks/daily.toml").unwrap(),
                )
                .unwrap();
                assert_eq!(task_config.tasks, example_task_config().tasks)
            }

            #[test]
            fn yaml() {
                let task_config: TaskConfig = serde_yaml::from_reader(
                    std::fs::File::open("./config_examples/tasks/daily.yml").unwrap(),
                )
                .unwrap();
                assert_eq!(task_config.tasks, example_task_config().tasks)
            }
        }

    }
}
