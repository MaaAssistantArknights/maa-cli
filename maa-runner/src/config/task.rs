use chrono::{Datelike, Local, NaiveDateTime, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Condition {
    Always,
    Weekday {
        weekdays: Vec<Weekday>,
    },
    Time {
        #[serde(default, deserialize_with = "deserialize_from_str")]
        start: Option<NaiveTime>,
        #[serde(default, deserialize_with = "deserialize_from_str")]
        end: Option<NaiveTime>,
    },
    DateTime {
        #[serde(default, deserialize_with = "deserialize_from_str")]
        start: Option<NaiveDateTime>,
        #[serde(default, deserialize_with = "deserialize_from_str")]
        end: Option<NaiveDateTime>,
    },
    Combined {
        conditions: Vec<Condition>,
    },
}

fn deserialize_from_str<'de, S, D>(deserializer: D) -> Result<Option<S>, D::Error>
where
    S: std::str::FromStr,
    S::Err: std::fmt::Display,
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => match s.parse::<S>() {
            Ok(t) => Ok(Some(t)),
            Err(e) => Err(serde::de::Error::custom(format!("Invalid format: {}", e))),
        },
        None => Ok(None),
    }
}

impl Condition {
    pub fn is_active(&self) -> bool {
        match self {
            Condition::Always => true,
            Condition::Weekday { weekdays } => {
                let now = Local::now();
                let weekday = now.date_naive().weekday();
                weekdays.contains(&weekday)
            }
            Condition::Time { start, end } => {
                let now = Local::now();
                let now_time = now.time();
                match (start, end) {
                    (Some(s), Some(e)) => now_time >= *s && now_time <= *e,
                    (Some(s), None) => now_time >= *s,
                    (None, Some(e)) => now_time <= *e,
                    (None, None) => true,
                }
            }
            Condition::DateTime { start, end } => {
                let now = Local::now().naive_local();
                match (start, end) {
                    (Some(s), Some(e)) => now >= *s && now <= *e,
                    (Some(s), None) => now >= *s,
                    (None, Some(e)) => now <= *e,
                    (None, None) => true,
                }
            }
            Condition::Combined { conditions } => {
                for condition in conditions {
                    if !condition.is_active() {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl Default for Condition {
    fn default() -> Self {
        Condition::Always
    }
}

fn empty_object() -> Value {
    json!({})
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct TaskVariant {
    #[serde(default)]
    pub condition: Condition,
    #[serde(default = "empty_object")]
    pub params: Value,
}

impl Default for TaskVariant {
    fn default() -> Self {
        TaskVariant {
            condition: Condition::Always,
            params: empty_object(),
        }
    }
}

fn default_variants() -> Vec<TaskVariant> {
    vec![TaskVariant::default()]
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum TaskType {
    StartUp,
    CloseDown,
    Fight,
    Recruit,
    Infrast,
    Mall,
    Award,
    Roguelike,
    Copilot,
    SSSCopilot,
    Depot,
    OperBox,
    ReclamationAlgorithm,
    Custom,
    SingleStep,
    VideoRecognition,
}

impl crate::maacore::ToCString for &TaskType {
    fn to_cstring(self) -> Result<std::ffi::CString, std::ffi::NulError> {
        match self {
            TaskType::StartUp => "StartUp",
            TaskType::CloseDown => "CloseDown",
            TaskType::Fight => "Fight",
            TaskType::Recruit => "Recruit",
            TaskType::Infrast => "Infrast",
            TaskType::Mall => "Mall",
            TaskType::Award => "Award",
            TaskType::Roguelike => "Roguelike",
            TaskType::Copilot => "Copilot",
            TaskType::SSSCopilot => "SSSCopilot",
            TaskType::Depot => "Depot",
            TaskType::OperBox => "OperBox",
            TaskType::ReclamationAlgorithm => "ReclamationAlgorithm",
            TaskType::Custom => "Custom",
            TaskType::SingleStep => "SingleStep",
            TaskType::VideoRecognition => "VideoRecognition",
        }
        .to_cstring()
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Task {
    #[serde(rename = "type")]
    task_type: TaskType,
    #[serde(default = "empty_object")]
    params: Value,
    #[serde(default = "default_variants")]
    variants: Vec<TaskVariant>,
}

fn merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (a @ &mut Value::Object(_), Value::Object(b)) => {
            let a = a.as_object_mut().unwrap();
            for (k, v) in b {
                merge(a.entry(k).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b.clone(),
    };
}

impl Task {
    pub fn is_active(&self) -> bool {
        for variant in &self.variants {
            if variant.condition.is_active() {
                return true;
            }
        }
        return false;
    }

    pub fn get_type(&self) -> &TaskType {
        return &self.task_type;
    }

    pub fn get_params(&self) -> Value {
        let mut params = self.params.clone();
        for variant in &self.variants {
            // Merge params from all active variants
            if variant.condition.is_active() {
                merge(&mut params, &variant.params);
                break;
            }
        }
        return params;
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

    use chrono::{Duration, TimeZone};

    fn naive_local_datetime(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        chrono::Local
            .with_ymd_and_hms(y, m, d, h, mi, s)
            .unwrap()
            .naive_local()
    }

    fn example_tasks() -> TaskList {
        return TaskList {
            tasks: vec![
                Task {
                    task_type: TaskType::StartUp,
                    params: json!({
                        "client_type": "Official",
                        "start_game_enabled": true,
                    }),
                    variants: vec![TaskVariant::default()],
                },
                Task {
                    task_type: TaskType::Fight,
                    params: empty_object(),
                    variants: vec![
                        TaskVariant {
                            condition: Condition::DateTime {
                                start: Some(naive_local_datetime(2023, 8, 1, 16, 0, 0)),
                                end: Some(naive_local_datetime(2023, 8, 21, 3, 59, 59)),
                            },
                            params: serde_json::json!({
                                "stage": "",
                            }),
                        },
                        TaskVariant {
                            condition: Condition::Weekday {
                                weekdays: vec![Weekday::Tue, Weekday::Thu, Weekday::Sat],
                            },
                            params: serde_json::json!({
                                "stage": "CE-6",
                            }),
                        },
                        TaskVariant {
                            condition: Condition::Always,
                            params: serde_json::json!({
                                "stage": "1-7",
                            }),
                        },
                    ],
                },
                Task {
                    task_type: TaskType::Mall,
                    params: empty_object(),
                    variants: vec![TaskVariant {
                        params: empty_object(),
                        condition: Condition::Time {
                            start: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
                            end: None,
                        },
                    }],
                },
                Task {
                    task_type: TaskType::CloseDown,
                    params: empty_object(),
                    variants: vec![TaskVariant::default()],
                },
            ],
        };
    }

    mod conditions {
        use super::*;

        #[test]
        fn always() {
            assert!(Condition::Always.is_active());
        }

        #[test]
        fn weekday() {
            let now = chrono::Local::now();
            let weekday = now.date_naive().weekday();

            assert!(Condition::Weekday {
                weekdays: vec![weekday]
            }
            .is_active());
            assert!(!Condition::Weekday {
                weekdays: vec![weekday.pred(), weekday.succ()]
            }
            .is_active());
        }

        #[test]
        fn time() {
            let now = chrono::Local::now();
            let now_time = now.time();

            assert!(Condition::Time {
                start: Some(now_time + Duration::minutes(-10)),
                end: Some(now_time + Duration::minutes(10)),
            }
            .is_active());
            assert!(!Condition::Time {
                start: Some(now_time + Duration::minutes(10)),
                end: Some(now_time + Duration::minutes(20)),
            }
            .is_active());
        }

        #[test]
        fn datetime() {
            let now = chrono::Local::now();
            let now_datetime = now.naive_local();

            assert!(Condition::DateTime {
                start: Some(now_datetime + Duration::minutes(-10)),
                end: Some(now_datetime + Duration::minutes(10)),
            }
            .is_active());
            assert!(!Condition::DateTime {
                start: Some(now_datetime + Duration::minutes(10)),
                end: Some(now_datetime + Duration::minutes(20)),
            }
            .is_active());
        }

        #[test]
        fn combined() {
            let now = chrono::Local::now();
            let now_time = now.time();
            let weekday = now.date_naive().weekday();

            assert!(Condition::Combined {
                conditions: vec![
                    Condition::Time {
                        start: Some(now_time + Duration::minutes(-10)),
                        end: Some(now_time + Duration::minutes(10)),
                    },
                    Condition::Weekday {
                        weekdays: vec![weekday]
                    },
                ]
            }
            .is_active());
            assert!(!Condition::Combined {
                conditions: vec![
                    Condition::Time {
                        start: Some(now_time + Duration::minutes(10)),
                        end: Some(now_time + Duration::minutes(20)),
                    },
                    Condition::Weekday {
                        weekdays: vec![weekday]
                    },
                ]
            }
            .is_active());
        }
    }

    mod cond_deserialize_json {
        use super::*;

        #[test]
        fn time() {
            let cond_str = r#"{
                "type": "Time",
                "start": "16:00:00",
                "end": "04:00:00"
            }"#;
            let cond: Condition = serde_json::from_str(cond_str).unwrap();
            assert_eq!(
                cond,
                Condition::Time {
                    start: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
                    end: Some(NaiveTime::from_hms_opt(4, 0, 0).unwrap()),
                }
            );
        }

        #[test]
        fn datatime() {
            let cond_str = r#"{
                "type": "DateTime",
                "start": "2021-08-01T16:00:00",
                "end": "2021-08-21T04:00:00"
            }"#;
            let cond: Condition = serde_json::from_str(cond_str).unwrap();
            assert_eq!(
                cond,
                Condition::DateTime {
                    start: Some(naive_local_datetime(2021, 8, 01, 16, 0, 0)),
                    end: Some(naive_local_datetime(2021, 8, 21, 4, 0, 0)),
                }
            );
        }
    }

    mod params {
        use super::*;

        #[test]
        fn empty() {
            let task: Task = toml::from_str(
                r#"
                    type = "StartUp"
                "#,
            )
            .unwrap();
            assert_eq!(task.get_params(), json!({}))
        }

        #[test]
        fn no_variants() {
            let task: Task = toml::from_str(
                r#"
                    type = "Fight"
                    params = { stage = "1-7" }
                "#,
            )
            .unwrap();
            assert_eq!(task.get_params(), json!({"stage": "1-7"}))
        }

        #[test]
        fn no_base() {
            let task: Task = toml::from_str(
                r#"
                    type = "Fight"
                    variants = [ { params = { stage= "1-7" } } ]
                "#,
            )
            .unwrap();
            assert_eq!(task.get_params(), json!({ "stage": "1-7" }))
        }

        #[test]
        fn merge() {
            let task: Task = toml::from_str(
                r#"
                    type = "Fight"
                    params = { stage = "1-7", times = 3 }
                    variants = [
                        { params = { stage = "CE-6" } },
                        { params = { stage = "" } },
                    ]
                "#,
            )
            .unwrap();
            assert_eq!(
                task.get_params(),
                json!({
                    "stage": "CE-6",
                    "times": 3,
                })
            )
        }
    }

    mod deserialize_example {
        use super::*;

        #[test]
        fn json() {
            let tasks: TaskList = serde_json::from_reader(
                std::fs::File::open("../example/tasks/daily.json").unwrap(),
            )
            .unwrap();
            assert_eq!(tasks, example_tasks());
        }

        #[test]
        fn toml() {
            let tasks: TaskList =
                toml::from_str(&std::fs::read_to_string("../example/tasks/daily.toml").unwrap())
                    .unwrap();
            assert_eq!(tasks, example_tasks())
        }
    }
}
