pub mod value;
pub use value::Value;

pub mod task_type;
use task_type::TaskOrUnknown;

pub mod condition;
use condition::Condition;

use serde::Deserialize;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct TaskVariant {
    #[serde(default)]
    pub condition: Condition,
    #[serde(default)]
    pub params: Value,
}

pub fn default_variants() -> Vec<TaskVariant> {
    vec![Default::default()]
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

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Task {
    #[serde(rename = "type")]
    task_type: TaskOrUnknown,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    strategy: Strategy,
    #[serde(default = "default_variants")]
    variants: Vec<TaskVariant>,
}

impl Task {
    pub fn new<T, V, S>(task_type: T, params: V, strategy: Strategy, variants: S) -> Self
    where
        T: Into<TaskOrUnknown>,
        V: Into<Value>,
        S: IntoIterator<Item = TaskVariant>,
    {
        Self {
            task_type: task_type.into(),
            strategy,
            params: params.into(),
            variants: variants.into_iter().collect(),
        }
    }

    pub fn is_active(&self) -> bool {
        for variant in &self.variants {
            if variant.condition.is_active() {
                return true;
            }
        }
        false
    }

    pub fn get_type(&self) -> &TaskOrUnknown {
        &self.task_type
    }

    pub fn get_params(&self) -> Value {
        let mut params = self.params.clone();
        match self.strategy {
            // Merge params from the first active variant
            Strategy::First => {
                for variant in &self.variants {
                    if variant.condition.is_active() {
                        params.merge_mut(&variant.params);
                        break;
                    }
                }
            }
            // Merge params from all active variants
            Strategy::Merge => {
                for variant in &self.variants {
                    if variant.condition.is_active() {
                        params.merge_mut(&variant.params);
                    }
                }
            }
        }
        params
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct TaskList {
    pub tasks: Vec<Task>,
}

impl super::FromFile for TaskList {}

#[cfg(test)]
mod tests {
    use super::*;

    use task_type::TaskType;

    /// Create a object from a list of key-value pairs
    macro_rules! object {
        () => {
            Value::new()
        };
        ($($key:expr => $value:expr),* $(,)?) => {{
            let mut value = Value::new();
            $(value.insert($key, $value);)*
            value
        }};
    }

    mod task {
        use super::*;

        #[test]
        fn is_active() {
            assert!(Task::new(
                TaskType::StartUp,
                Value::default(),
                Strategy::default(),
                vec![TaskVariant {
                    condition: Condition::Always,
                    params: Value::default(),
                }]
            )
            .is_active());
            assert!(!Task::new(
                TaskType::StartUp,
                Value::default(),
                Strategy::default(),
                vec![TaskVariant {
                    condition: Condition::Never,
                    params: Value::default(),
                }]
            )
            .is_active());
            assert!(Task::new(
                TaskType::StartUp,
                Value::default(),
                Strategy::default(),
                vec![
                    TaskVariant {
                        condition: Condition::Never,
                        params: Value::default(),
                    },
                    TaskVariant {
                        condition: Condition::Always,
                        params: Value::default(),
                    },
                ]
            )
            .is_active());
            assert!(!Task::new(
                TaskType::StartUp,
                Value::default(),
                Strategy::default(),
                vec![
                    TaskVariant {
                        condition: Condition::Never,
                        params: Value::default(),
                    },
                    TaskVariant {
                        condition: Condition::Never,
                        params: Value::default(),
                    },
                ]
            )
            .is_active());
        }

        #[test]
        fn get_type() {
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    Value::default(),
                    Strategy::default(),
                    vec![]
                )
                .get_type(),
                &TaskType::StartUp.into()
            );
        }

        #[test]
        fn get_params() {
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1),
                    Strategy::First,
                    vec![TaskVariant {
                        condition: Condition::Always,
                        params: object!(),
                    }]
                )
                .get_params(),
                object!("a" => 1)
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1),
                    Strategy::First,
                    vec![TaskVariant {
                        condition: Condition::Always,
                        params: object!("b" => 2),
                    }]
                )
                .get_params(),
                object!("a" => 1, "b" => 2)
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    Value::default(),
                    Strategy::First,
                    vec![TaskVariant {
                        condition: Condition::Always,
                        params: object!("a" => 1),
                    }]
                )
                .get_params(),
                object!("a" => 1)
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1),
                    Strategy::First,
                    vec![TaskVariant {
                        condition: Condition::Always,
                        params: object!("a" => 2),
                    }]
                )
                .get_params(),
                object!("a" => 2)
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1),
                    Strategy::First,
                    vec![
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 2),
                        },
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 3),
                        },
                    ]
                )
                .get_params(),
                object!("a" => 2)
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1),
                    Strategy::Merge,
                    vec![
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 2),
                        },
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 3),
                        },
                    ]
                )
                .get_params(),
                object!("a" => 3)
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1),
                    Strategy::First,
                    vec![
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 2),
                        },
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("b" => 4),
                        },
                    ]
                )
                .get_params(),
                object!("a" => 2),
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1),
                    Strategy::Merge,
                    vec![
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 2),
                        },
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("b" => 4),
                        },
                    ]
                )
                .get_params(),
                object!("a" => 2, "b" => 4),
            );
            assert_eq!(
                Task::new(
                    TaskType::StartUp,
                    object!("a" => 1, "c" => 5),
                    Strategy::First,
                    vec![
                        TaskVariant {
                            condition: Condition::Never,
                            params: object!("a" => 2),
                        },
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 3, "b" => 4),
                        },
                    ]
                )
                .get_params(),
                object!("a" => 3, "b" => 4, "c" => 5),
            );
        }
    }

    mod deserialize_example {
        use super::*;

        use value::input::{Input, Select};

        use chrono::{NaiveDateTime, NaiveTime, TimeZone, Weekday};

        fn example_tasks() -> TaskList {
            TaskList {
                tasks: vec![
                    Task::new(
                        TaskType::StartUp,
                        object!(
                            "client_type" => "Official",
                            "start_game_enabled" => true,
                        ),
                        Strategy::default(),
                        vec![TaskVariant {
                            condition: Condition::Always,
                            params: object!(),
                        }],
                    ),
                    Task::new(
                        TaskType::Fight,
                        object!(),
                        Strategy::Merge,
                        vec![
                            TaskVariant {
                                condition: Condition::Weekday {
                                    weekdays: vec![Weekday::Sun],
                                },
                                params: object!("expiring_medicine" => 5),
                            },
                            TaskVariant {
                                condition: Condition::Always,
                                params: object!(
                                    "stage" => Value::InputString(
                                        Input{
                                            default: Some("1-7".to_string()),
                                            description: Some("a stage to fight".to_string()) }
                                        .into(),
                                    )
                                ),
                            },
                            TaskVariant {
                                condition: Condition::Weekday {
                                    weekdays: vec![Weekday::Tue, Weekday::Thu, Weekday::Sat],
                                },
                                params: object!("stage" => "CE-6"),
                            },
                            TaskVariant {
                                condition: Condition::DateTime {
                                    start: Some(naive_local_datetime(2023, 8, 1, 16, 0, 0)),
                                    end: Some(naive_local_datetime(2023, 8, 21, 3, 59, 59)),
                                },
                                params: object!("stage" => Value::InputString(
                                        Select {
                                            alternatives: vec![
                                                "SL-6".to_string(),
                                                "SL-7".to_string(),
                                                "SL-8".to_string(),
                                            ],
                                            description: Some("a stage to fight in summer event".to_string()),
                                        }
                                        .into(),
                                    )
                                ),
                            },
                        ],
                    ),
                    Task::new(
                        TaskType::Mall,
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
                        Strategy::default(),
                        vec![TaskVariant {
                            condition: Condition::Time {
                                start: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
                                end: None,
                            },
                            params: object!(),
                        }],
                    ),
                    Task::new(
                        TaskType::CloseDown,
                        object!(),
                        Strategy::default(),
                        vec![TaskVariant {
                            condition: Condition::Always,
                            params: object!(),
                        }],
                    ),
                ],
            }
        }

        fn naive_local_datetime(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
            chrono::Local
                .with_ymd_and_hms(y, m, d, h, mi, s)
                .unwrap()
                .naive_local()
        }

        #[test]
        fn json() {
            let tasks: TaskList = serde_json::from_reader(
                std::fs::File::open("../config_examples/tasks/daily.json").unwrap(),
            )
            .unwrap();
            assert_eq!(tasks, example_tasks());
        }

        #[test]
        fn toml() {
            let tasks: TaskList = toml::from_str(
                &std::fs::read_to_string("../config_examples/tasks/daily.toml").unwrap(),
            )
            .unwrap();
            assert_eq!(tasks, example_tasks())
        }

        #[test]
        fn yaml() {
            let tasks: TaskList = serde_yaml::from_reader(
                std::fs::File::open("../config_examples/tasks/daily.yml").unwrap(),
            )
            .unwrap();
            assert_eq!(tasks, example_tasks())
        }
    }
}
