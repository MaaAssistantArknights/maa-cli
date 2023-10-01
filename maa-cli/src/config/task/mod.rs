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

fn default_variants() -> Vec<TaskVariant> {
    vec![Default::default()]
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Task {
    #[serde(rename = "type")]
    task_type: TaskOrUnknown,
    #[serde(default)]
    params: Value,
    #[serde(default = "default_variants")]
    variants: Vec<TaskVariant>,
}

impl Task {
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
        for variant in &self.variants {
            // Merge params from all active variants
            if variant.condition.is_active() {
                params.merge_mut(&variant.params);
                break;
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

    use value::input::{Input, Select};

    use chrono::{NaiveDateTime, NaiveTime, TimeZone, Weekday};
    use task_type::TaskType;

    fn naive_local_datetime(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        chrono::Local
            .with_ymd_and_hms(y, m, d, h, mi, s)
            .unwrap()
            .naive_local()
    }

    /// Create a object from a list of key-value pairs
    macro_rules! object {
        ($($key:expr => $value:expr),* $(,)?) => {{
            let mut map = std::collections::BTreeMap::new();
            $(map.insert($key.to_string(), $value.into());)*
            Value::Object(map)
        }};
    }

    impl Task {
        pub fn new<T, V, S>(task_type: T, params: V, variants: S) -> Self
        where
            T: Into<TaskOrUnknown>,
            V: Into<Value>,
            S: IntoIterator<Item = TaskVariant>,
        {
            Self {
                task_type: task_type.into(),
                params: params.into(),
                variants: variants.into_iter().collect(),
            }
        }
    }

    fn example_tasks() -> TaskList {
        TaskList {
            tasks: vec![
                Task::new(
                    TaskType::StartUp,
                    [
                        ("client_type", "Official".into()),
                        ("start_game_enabled", true.into()),
                    ],
                    vec![TaskVariant::default()],
                ),
                Task::new(
                    TaskType::Fight,
                    Value::default(),
                    vec![
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
                        TaskVariant {
                            condition: Condition::Weekday {
                                weekdays: vec![Weekday::Tue, Weekday::Thu, Weekday::Sat],
                            },
                            params: object!("stage" => "CE-6"),
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
                    ],
                ),
                Task::new(
                    TaskType::Mall,
                    Value::default(),
                    vec![TaskVariant {
                        condition: Condition::Time {
                            start: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
                            end: None,
                        },
                        params: Value::default(),
                    }],
                ),
                Task::new(
                    TaskType::CloseDown,
                    Value::default(),
                    vec![TaskVariant::default()],
                ),
            ],
        }
    }

    mod deserialize_example {
        use super::*;

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
