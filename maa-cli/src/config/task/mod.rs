mod client_type;
pub use client_type::ClientType;

mod condition;
pub use condition::remainder_of_day_mod;
use condition::Condition;

use crate::{dirs, object, value::MAAValue};

use std::path::PathBuf;

use anyhow::Context;
use maa_sys::TaskType;
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

fn default_variants() -> Vec<TaskVariant> {
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
    #[serde(default = "default_variants")]
    variants: Vec<TaskVariant>,
}

impl Task {
    pub fn new<T, V, S>(
        name: Option<String>,
        task_type: T,
        params: V,
        strategy: Strategy,
        variants: S,
    ) -> Self
    where
        T: Into<TaskType>,
        V: Into<MAAValue>,
        S: IntoIterator<Item = TaskVariant>,
    {
        Self {
            name,
            task_type: task_type.into(),
            strategy,
            params: params.into(),
            variants: variants.into_iter().collect(),
        }
    }

    pub fn new_with_default<T, V>(task_type: T, params: V) -> Self
    where
        T: Into<TaskType>,
        V: Into<MAAValue>,
    {
        Self::new(
            None,
            task_type,
            params,
            Strategy::default(),
            default_variants(),
        )
    }

    pub fn is_active(&self) -> bool {
        for variant in self.variants.iter() {
            if variant.is_active() {
                return true;
            }
        }
        false
    }

    pub fn task_type(&self) -> TaskType {
        self.task_type
    }

    pub fn params(&self) -> MAAValue {
        let mut params = self.params.clone();
        match self.strategy {
            // Merge params from the first active variant
            Strategy::First => {
                for variant in &self.variants {
                    if variant.is_active() {
                        params.merge_mut(variant.params());
                        break;
                    }
                }
            }
            // Merge params from all active variants
            Strategy::Merge => {
                for variant in &self.variants {
                    if variant.is_active() {
                        params.merge_mut(variant.params());
                    }
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
    pub fn new() -> Self {
        Self {
            client_type: None,
            startup: None,
            closedown: None,
            tasks: Vec::new(),
        }
    }

    pub fn push(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn init(&self) -> anyhow::Result<InitializedTaskConfig> {
        let mut startup = self.startup;
        let mut closedown = self.closedown;
        let mut client_type = self.client_type;

        let mut prepend_startup = startup.is_some_and(|v| v);
        let mut append_closedown = closedown.is_some_and(|v| v);

        let mut tasks: Vec<InitializedTask> = Vec::new();

        for task in self.tasks.iter() {
            if task.is_active() {
                let task_type = task.task_type();
                let mut params = task.params().init()?;

                use TaskType::*;
                match task_type {
                    StartUp => {
                        let start_game = params.get_or("enable", true)
                            && params.get_or("start_game_enabled", false);

                        match (start_game, startup) {
                            (true, None) => {
                                startup = Some(true);
                            }
                            (false, Some(true)) => {
                                params.insert("enable", true);
                                params.insert("start_game_enabled", true);
                            }
                            _ => {}
                        }

                        match (params.get("client_type"), client_type) {
                            // If client_type in task is set, set client type in config automatically
                            (Some(t), None) => {
                                client_type = Some(
                                    t.as_str()
                                        .context("client_type must be a string")?
                                        .parse()?,
                                );
                            }
                            // If client type in config is set, set client_type in task automatically
                            (None, Some(t)) => {
                                params.insert("client_type", t.to_string());
                            }
                            _ => {}
                        }

                        prepend_startup = false;
                    }
                    CloseDown => {
                        match (params.get_or("enable", true), closedown) {
                            // If closedown task is enabled, enable closedown automatically
                            (true, None) => {
                                closedown = Some(true);
                            }
                            // If closedown is enabled manually, enable closedown task automatically
                            (false, Some(true)) => {
                                params.insert("enable", true);
                            }
                            _ => {}
                        }

                        append_closedown = false;
                    }
                    _ => {
                        // For any task that has a filename parameter
                        // and the filename parameter is not an absolute path,
                        // it will be treated as a relative path to the config directory
                        // and will be converted to an absolute path.
                        if let Some(v) = params.get("filename") {
                            let file: PathBuf =
                                v.as_str().context("filename must be a string")?.into();
                            let sub_dir = task_type.as_ref().to_lowercase();
                            if let Some(path) = dirs::abs_config(file, Some(sub_dir)) {
                                params.insert("filename", path.to_str().context("Invilid UTF-8")?)
                            }
                        }
                    }
                }
                tasks.push(InitializedTask::new(task.name.clone(), task_type, params));
            }
        }

        if prepend_startup {
            tasks.insert(
                0,
                InitializedTask::new_noname(
                    TaskType::StartUp,
                    object!(
                        "start_game_enabled" => true,
                        "client_type" => self.client_type.unwrap_or_default().to_string(),
                    ),
                ),
            );
        }

        if append_closedown {
            tasks.push(InitializedTask::new_noname(TaskType::CloseDown, object!()));
        }

        Ok(InitializedTaskConfig {
            client_type,
            start_app: startup.unwrap_or(false),
            close_app: closedown.unwrap_or(false),
            tasks,
        })
    }
}

impl super::FromFile for TaskConfig {}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct InitializedTaskConfig {
    pub client_type: Option<ClientType>,
    pub start_app: bool,
    pub close_app: bool,
    pub tasks: Vec<InitializedTask>,
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct InitializedTask {
    name: Option<String>,
    task_type: TaskType,
    params: MAAValue,
}

impl InitializedTask {
    pub fn new(name: Option<String>, task_type: impl Into<TaskType>, params: MAAValue) -> Self {
        Self {
            name,
            task_type: task_type.into(),
            params,
        }
    }

    pub fn new_noname(task_type: impl Into<TaskType>, params: MAAValue) -> Self {
        Self::new(None, task_type.into(), params)
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn task_type(&self) -> TaskType {
        self.task_type
    }

    pub fn params(&self) -> &MAAValue {
        &self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::object;

    impl TaskConfig {
        pub fn tasks(&self) -> &[Task] {
            &self.tasks
        }
    }

    mod task {
        use super::*;

        #[test]
        fn is_active() {
            fn test_with_veriants(variants: Vec<TaskVariant>, expected: bool) {
                assert_eq!(
                    Task::new(
                        None,
                        TaskType::StartUp,
                        object!(),
                        Strategy::default(),
                        variants
                    )
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
                Task::new_with_default(TaskType::StartUp, object!()).task_type(),
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
                assert_eq!(
                    Task::new(
                        None,
                        TaskType::StartUp,
                        base,
                        strategy,
                        variants.into_iter().map(|v| TaskVariant {
                            condition: Condition::Always,
                            params: v,
                        })
                    )
                    .params(),
                    expected
                );
            }

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
                Task::new(
                    None,
                    TaskType::StartUp,
                    object!("a" => 1, "c" => 5),
                    Strategy::First,
                    vec![
                        TaskVariant {
                            condition: Condition::Not {
                                condition: Box::new(Condition::Always),
                            },
                            params: object!("a" => 2),
                        },
                        TaskVariant {
                            condition: Condition::Always,
                            params: object!("a" => 3, "b" => 4),
                        },
                    ]
                )
                .params(),
                object!("a" => 3, "b" => 4, "c" => 5),
            );
        }
    }

    mod task_config {
        use super::*;

        use TaskType::*;

        mod serde {
            use super::*;

            use condition::TimeOffset;

            use crate::value::userinput::{BoolInput, Input, SelectD};

            use chrono::{NaiveDateTime, NaiveTime, TimeZone, Weekday};

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
                use crate::value::Map;
                use ClientType::*;
                use MAAValue::OptionalInput;

                let mut task_list = TaskConfig::new();

                task_list.push(Task::new_with_default(
                    StartUp,
                    object!(
                        "client_type" => OptionalInput {
                            deps: Map::from([("start_game_enabled".to_string(), true.into())]),
                            input: SelectD::<String>::new(
                                vec![
                                    Official.as_ref(),
                                    YoStarEN.as_ref(),
                                    YoStarJP.as_ref(),
                                ],
                                None,
                                Some("a client type"),
                                false
                            ).unwrap().into(),
                        },
                        "start_game_enabled" => BoolInput::new(
                            Some(true),
                            Some("start the game"),
                        ),
                    ),
                ));

                task_list.push(Task::new(
                    Some("Fight Daily".to_string()),
                    Fight,
                    object!(),
                    Strategy::Merge,
                    vec![
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
                                    Some("a stage to fight"),
                                ),
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
                                "stage" => SelectD::<String>::new(
                                    [
                                        "SL-6",
                                        "SL-7",
                                        "SL-8",
                                    ],
                                    Some(2),
                                    Some("a stage to fight in summer event"),
                                    true,
                                ).unwrap(),
                            ),
                        },
                    ],
                ));

                task_list.push(Task::new(
                    None,
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
                    Strategy::default(),
                    vec![TaskVariant {
                        condition: Condition::Time {
                            start: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
                            end: None,
                            timezone: TimeOffset::Local,
                        },
                        params: object!(),
                    }],
                ));

                task_list.push(Task::new_with_default(CloseDown, object!()));

                task_list
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

        #[test]
        fn init() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        Task::new_with_default(
                            StartUp,
                            object!(
                                "client_type" => "Official",
                                "start_game_enabled" => true,
                            ),
                        ),
                        Task::new_with_default(Fight, object!("stage" => "1-7")),
                        Task::new_with_default(CloseDown, object!()),
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Some(ClientType::Official),
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        InitializedTask::new_noname(
                            StartUp,
                            object!(
                                "client_type" => "Official",
                                "start_game_enabled" => true,
                            )
                        ),
                        InitializedTask::new_noname(Fight, object!("stage" => "1-7")),
                        InitializedTask::new_noname(CloseDown, object!()),
                    ]
                }
            );

            assert_eq!(
                TaskConfig {
                    client_type: Some(ClientType::Official),
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![
                        Task::new_with_default(StartUp, object!( "start_game_enabled" => false)),
                        Task::new_with_default(Fight, object!("stage" => "1-7")),
                        Task::new_with_default(CloseDown, object!("enable" => false)),
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Some(ClientType::Official),
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        InitializedTask::new_noname(
                            StartUp,
                            object!(
                                "enable" => true,
                                "client_type" => "Official",
                                "start_game_enabled" => true,
                            )
                        ),
                        InitializedTask::new_noname(Fight, object!("stage" => "1-7")),
                        InitializedTask::new_noname(CloseDown, object!("enable" => true)),
                    ]
                },
            );

            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![Task::new_with_default(Fight, object!("stage" => "1-7"))],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: None,
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        InitializedTask::new_noname(
                            StartUp,
                            object!(
                                "client_type" => "Official",
                                "start_game_enabled" => true,
                            )
                        ),
                        InitializedTask::new_noname(Fight, object!("stage" => "1-7")),
                        InitializedTask::new_noname(CloseDown, object!()),
                    ]
                },
            );

            assert_eq!(
                TaskConfig {
                    client_type: Some(ClientType::YoStarEN),
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![Task::new_with_default(Fight, object!("stage" => "1-7"))],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Some(ClientType::YoStarEN),
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        InitializedTask::new_noname(
                            StartUp,
                            object!(
                                "start_game_enabled" => true,
                                "client_type" => "YoStarEN",
                            )
                        ),
                        InitializedTask::new_noname(Fight, object!("stage" => "1-7")),
                        InitializedTask::new_noname(CloseDown, object!()),
                    ]
                }
            )
        }

        #[test]
        fn initialized_task() {
            let task = InitializedTask::new(
                Some("Fight Daily".to_string()),
                Fight,
                object!("stage" => "1-7"),
            );
            assert_eq!(task.name(), Some("Fight Daily"));
            assert_eq!(task.task_type(), Fight);
            assert_eq!(task.params(), &object!("stage" => "1-7"));
        }
    }
}
