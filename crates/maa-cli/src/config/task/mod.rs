mod client_type;
pub use client_type::ClientType;

mod condition;
use std::path::PathBuf;

use anyhow::Context;
use condition::Condition;
pub use condition::{TimeOffset, remainder_of_day_mod};
use maa_types::TaskType;
use maa_value::prelude::*;
use serde::Deserialize;

use crate::dirs;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TaskVariant {
    #[serde(default)]
    condition: Condition,
    #[serde(default)]
    params: MAAValueTemplate,
}

impl TaskVariant {
    pub fn is_active(&self) -> bool {
        self.condition.is_active()
    }

    pub fn params(&self) -> &MAAValueTemplate {
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
pub struct TaskTemplate {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    task_type: TaskType,
    #[serde(default)]
    params: MAAValueTemplate,
    #[serde(default)]
    strategy: Strategy,
    #[serde(default)]
    variants: Vec<TaskVariant>,
}

// Constructor for Task
impl TaskTemplate {
    #[cfg(test)]
    fn new(task_type: TaskType, params: MAAValueTemplate) -> Self {
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

    pub fn params(&self) -> MAAValueTemplate {
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
pub struct TaskConfigTemplate {
    client_type: Option<ClientType>,
    startup: Option<bool>,
    closedown: Option<bool>,
    tasks: Vec<TaskTemplate>,
}

impl TaskConfigTemplate {
    pub fn init(&self) -> anyhow::Result<TaskConfig> {
        let mut startup = self.startup;
        let mut closedown = self.closedown;
        let mut client_type = self.client_type;

        let mut tasks: Vec<Task> = Vec::new();
        let mut prepend_startup = startup.unwrap_or(false);
        let mut append_closedown = closedown.unwrap_or(false);

        use TaskType::*;

        for task in self.tasks.iter() {
            if !task.is_active() {
                continue;
            }

            let task_type = task.task_type();
            let mut params = task.params().resolve()?;

            // If startup task is not enabled, enable it automatically
            match task_type {
                StartUp => {
                    let start_game =
                        params.get_or("enable", true) && params.get_or("start_game_enabled", false);

                    match (start_game, startup) {
                        (true, None) => {
                            startup = Some(true);
                        }
                        (false, Some(true)) => {
                            insert!(params, "enable" => true, "start_game_enabled" => true);
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
                            insert!(params, "enable" => true);
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
                    if let Some(v) = params.get_mut("filename") {
                        let file = PathBuf::from(v.as_str().context("filename must be a string")?);
                        let sub_dir = task_type.to_str().to_lowercase();
                        if let Some(path) = dirs::abs_config(file, Some(sub_dir)) {
                            *v = path.try_into()?;
                        }
                    }
                }
            }

            let client_type_str = params.get("client_type").and_then(|v| v.as_str());

            let task_client_type = if let Some(s) = client_type_str {
                Some(s.parse()?)
            } else {
                None
            };

            // Get client type from task params
            match (task_client_type, client_type) {
                (Some(t), None) => client_type = Some(t),
                (Some(t1), Some(t2)) if t1 != t2 => {
                    log::warn!(
                        "Task {} has client_type {}, but the client type is set to {} in previous tasks or config",
                        task.name.as_deref().unwrap_or_else(|| task_type.to_str()),
                        t1,
                        t2,
                    )
                }
                _ => {}
            }

            let mut inited_task = Task::new(task_type, params);

            if let Some(name) = &task.name {
                inited_task = inited_task.with_name(name.to_owned());
            }

            tasks.push(inited_task)
        }

        let client_type = client_type.unwrap_or_default();

        // If client type is set in any task, set client type in all tasks automatically
        for task in tasks.iter_mut() {
            let task_type = task.task_type;
            let params = &mut task.params;

            // Set client type in task automatically
            if matches!(task_type, StartUp | Fight | CloseDown) {
                insert!(*params, "client_type" => client_type.to_str());
            }
        }

        if prepend_startup {
            tasks.insert(
                0,
                Task::new(
                    TaskType::StartUp,
                    object!(
                        "start_game_enabled" => true,
                        "client_type" => client_type.to_string(),
                    ),
                ),
            );
        }

        if append_closedown {
            tasks.push(Task::new(
                TaskType::CloseDown,
                object!("client_type" => client_type.to_string()),
            ));
        }

        Ok(TaskConfig {
            client_type,
            start_app: startup.unwrap_or(false),
            close_app: closedown.unwrap_or(false),
            tasks,
        })
    }
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct TaskConfig {
    pub client_type: ClientType,
    pub start_app: bool,
    pub close_app: bool,
    pub tasks: Vec<Task>,
}

impl TaskConfig {
    pub const fn new_with_tasks(tasks: Vec<Task>) -> Self {
        Self {
            client_type: ClientType::Official,
            start_app: false,
            close_app: false,
            tasks,
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct Task {
    pub name: Option<String>,
    pub task_type: TaskType,
    pub params: MAAValue,
}

impl Task {
    pub const fn new(task_type: TaskType, params: MAAValue) -> Self {
        Self {
            name: None,
            task_type,
            params,
        }
    }

    fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn name_or_default(&self) -> &str {
        self.name
            .as_deref()
            .unwrap_or_else(|| self.task_type.to_str())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    mod task {
        use super::*;

        #[test]
        fn is_active() {
            fn test_with_veriants(variants: Vec<TaskVariant>, expected: bool) {
                assert_eq!(
                    TaskTemplate::new(TaskType::StartUp, template!())
                        .with_variants(variants)
                        .is_active(),
                    expected
                );
            }

            fn always_active() -> TaskVariant {
                TaskVariant {
                    condition: Condition::Always,
                    params: MAAValueTemplate::default(),
                }
            }

            fn never_active() -> TaskVariant {
                TaskVariant {
                    condition: Condition::Not {
                        condition: Box::new(Condition::Always),
                    },
                    params: MAAValueTemplate::default(),
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
                TaskTemplate::new(TaskType::StartUp, template!()).task_type(),
                TaskType::StartUp,
            );
        }

        #[test]
        fn get_params() {
            fn test_with_variants(
                base: MAAValueTemplate,
                strategy: Strategy,
                variants: impl IntoIterator<Item = MAAValueTemplate>,
                expected: MAAValueTemplate,
            ) {
                let mut task = TaskTemplate::new(TaskType::StartUp, base).with_strategy(strategy);
                for v in variants {
                    task.push_variant(TaskVariant {
                        condition: Condition::Always,
                        params: v,
                    });
                }

                assert_eq!(task.params(), expected);
            }

            test_with_variants(
                template!("a" => 1),
                Strategy::First,
                vec![],
                template!("a" => 1),
            );

            test_with_variants(
                template!("a" => 1),
                Strategy::First,
                vec![template!()],
                template!("a" => 1),
            );

            test_with_variants(
                template!(),
                Strategy::First,
                vec![template!("a" => 1)],
                template!("a" => 1),
            );

            test_with_variants(
                template!("a" => 1),
                Strategy::First,
                vec![template!("b" => 2)],
                template!("a" => 1, "b" => 2),
            );

            test_with_variants(
                template!("a" => 1),
                Strategy::First,
                vec![template!("a" => 2)],
                template!("a" => 2),
            );

            test_with_variants(
                template!("a" => 1),
                Strategy::First,
                vec![template!("a" => 2), template!("a" => 3)],
                template!("a" => 2),
            );

            test_with_variants(
                template!("a" => 1),
                Strategy::Merge,
                vec![template!("a" => 2), template!("a" => 3)],
                template!("a" => 3),
            );

            test_with_variants(
                template!("a" => 1),
                Strategy::First,
                vec![template!("a" => 2), template!("b" => 4)],
                template!("a" => 2),
            );

            test_with_variants(
                template!("a" => 1),
                Strategy::Merge,
                vec![template!("a" => 2), template!("b" => 4)],
                template!("a" => 2, "b" => 4),
            );

            assert_eq!(
                {
                    let mut task =
                        TaskTemplate::new(TaskType::StartUp, template!("a" => 1, "c" => 5))
                            .with_strategy(Strategy::First);
                    task.push_variant(TaskVariant {
                        condition: Condition::Not {
                            condition: Box::new(Condition::Always),
                        },
                        params: template!("a" => 2),
                    });
                    task.push_variant(TaskVariant {
                        condition: Condition::Always,
                        params: template!("a" => 3, "b" => 4),
                    });
                    task.params()
                },
                template!("a" => 3, "b" => 4, "c" => 5),
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

            fn example_task_config() -> TaskConfigTemplate {
                use ClientType::*;

                let mut task_list = Vec::new();

                task_list.push(TaskTemplate::new(
                    StartUp,
                    template!(
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
                    TaskTemplate::new(Fight, template!())
                        .with_name("Fight Daily".to_string())
                        .with_strategy(Strategy::Merge)
                        .with_variants(vec![
                            TaskVariant {
                                condition: Condition::Weekday {
                                    weekdays: vec![Weekday::Sun],
                                    timezone: TimeOffset::Local,
                                },
                                params: template!("expiring_medicine" => 5),
                            },
                            TaskVariant {
                                condition: Condition::Always,
                                params: template!(
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
                                params: template!("stage" => "CE-6"),
                            },
                            TaskVariant {
                                condition: Condition::DateTime {
                                    start: Some(naive_local_datetime(2023, 8, 1, 16, 0, 0)),
                                    end: Some(naive_local_datetime(2023, 8, 21, 3, 59, 59)),
                                    timezone: TimeOffset::TimeZone(8),
                                },
                                params: template!(
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
                    TaskTemplate::new(
                        Mall,
                        template!(
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
                        params: template!(),
                    }]),
                );

                task_list.push(TaskTemplate::new(CloseDown, template!()));

                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: task_list,
                }
            }

            #[test]
            fn json() {
                let task_config: TaskConfigTemplate = serde_json::from_reader(
                    std::fs::File::open("./config_examples/tasks/daily.json").unwrap(),
                )
                .unwrap();
                assert_eq!(task_config.tasks, example_task_config().tasks)
            }

            #[test]
            fn toml() {
                let task_config: TaskConfigTemplate = toml::from_str(
                    &std::fs::read_to_string("./config_examples/tasks/daily.toml").unwrap(),
                )
                .unwrap();
                assert_eq!(task_config.tasks, example_task_config().tasks)
            }

            #[test]
            fn yaml() {
                let task_config: TaskConfigTemplate = serde_yaml::from_reader(
                    std::fs::File::open("./config_examples/tasks/daily.yml").unwrap(),
                )
                .unwrap();
                assert_eq!(task_config.tasks, example_task_config().tasks)
            }
        }

        #[test]
        fn init() {
            use ClientType::*;

            // Default client type is Official
            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: false,
                    close_app: false,
                    tasks: vec![],
                }
            );

            // No active tasks will be skipped
            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        TaskTemplate::new(StartUp, template!("start_game_enabled" => true))
                            .with_variants(vec![TaskVariant {
                                condition: Condition::Not {
                                    condition: Box::new(Condition::Always),
                                },
                                params: template!(),
                            }]),
                    ],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: false,
                    close_app: false,
                    tasks: vec![],
                }
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        TaskTemplate::new(
                            StartUp,
                            template!(
                                "start_game_enabled" => true,
                                "client_type" => "YoStarEN",
                            )
                        )
                        .with_name(String::from("StartUp"))
                    ],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: YoStarEN,
                    start_app: true,
                    close_app: false,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            template!(
                                "start_game_enabled" => true,
                                "client_type" => "YoStarEN",
                            )
                            .resolve()
                            .unwrap()
                        )
                        .with_name(String::from("StartUp"))
                    ]
                }
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![TaskTemplate::new(
                        StartUp,
                        template!(
                            "start_game_enabled" => false,
                            "client_type" => "YoStarEN",
                        )
                    )],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: YoStarEN,
                    start_app: false,
                    close_app: false,
                    tasks: vec![Task::new(
                        StartUp,
                        template!(
                            "start_game_enabled" => false,
                            "client_type" => "YoStarEN",
                        )
                        .resolve()
                        .unwrap()
                    )]
                }
            );

            // Process CloseDown task
            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![TaskTemplate::new(
                        CloseDown,
                        template!("client_type" => "YoStarEN")
                    )],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: YoStarEN,
                    start_app: false,
                    close_app: true,
                    tasks: vec![Task::new(
                        CloseDown,
                        template!("client_type" => "YoStarEN").resolve().unwrap()
                    )]
                }
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![TaskTemplate::new(
                        CloseDown,
                        template!(
                            "enable" => false,
                            "client_type" => "YoStarEN",
                        )
                    )],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: YoStarEN,
                    start_app: false,
                    close_app: false,
                    tasks: vec![Task::new(
                        CloseDown,
                        template!(
                            "enable" => false,
                            "client_type" => "YoStarEN",
                        )
                        .resolve()
                        .unwrap()
                    )]
                }
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![TaskTemplate::new(CloseDown, template!())],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: false,
                    close_app: true,
                    tasks: vec![Task::new(
                        CloseDown,
                        template!("client_type" => "Official").resolve().unwrap()
                    )]
                }
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![TaskTemplate::new(
                        Fight,
                        template!("client_type" => "YoStarEN")
                    )],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: YoStarEN,
                    start_app: false,
                    close_app: false,
                    tasks: vec![Task::new(
                        Fight,
                        template!("client_type" => "YoStarEN").resolve().unwrap()
                    )]
                }
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        TaskTemplate::new(
                            StartUp,
                            template!(
                                "start_game_enabled" => true,
                                "client_type" => "Official",
                            ),
                        ),
                        TaskTemplate::new(Fight, template!("stage" => "1-7")),
                        TaskTemplate::new(CloseDown, template!()),
                    ],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            template!(
                                "client_type" => "Official",
                                "start_game_enabled" => true,
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            Fight,
                            template!(
                                "stage" => "1-7",
                                "client_type" => "Official",
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            CloseDown,
                            template!("client_type" => "Official").resolve().unwrap()
                        ),
                    ]
                }
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: Some(Official),
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![
                        TaskTemplate::new(StartUp, template!( "start_game_enabled" => false)),
                        TaskTemplate::new(Fight, template!("stage" => "1-7")),
                        TaskTemplate::new(CloseDown, template!("enable" => false)),
                    ],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            template!(
                                "enable" => true,
                                "client_type" => "Official",
                                "start_game_enabled" => true,
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            Fight,
                            template!(
                                "stage" => "1-7",
                                "client_type" => "Official",
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            CloseDown,
                            template!(
                                "enable" => true,
                                "client_type" => "Official",
                            )
                            .resolve()
                            .unwrap()
                        ),
                    ]
                },
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![TaskTemplate::new(Fight, template!("stage" => "1-7"))],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            template!(
                                "client_type" => "Official",
                                "start_game_enabled" => true,
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            Fight,
                            template!(
                                "stage" => "1-7",
                                "client_type" => "Official",
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            CloseDown,
                            template!("client_type" => "Official").resolve().unwrap(),
                        ),
                    ]
                },
            );

            assert_eq!(
                TaskConfigTemplate {
                    client_type: Some(YoStarEN),
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![TaskTemplate::new(Fight, template!("stage" => "1-7"))],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: YoStarEN,
                    start_app: true,
                    close_app: true,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            template!(
                                "start_game_enabled" => true,
                                "client_type" => "YoStarEN",
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            Fight,
                            template!(
                                "stage" => "1-7",
                                "client_type" => "YoStarEN",
                            )
                            .resolve()
                            .unwrap()
                        ),
                        Task::new(
                            CloseDown,
                            template!("client_type" => "YoStarEN").resolve().unwrap(),
                        ),
                    ]
                }
            );

            // Conflicting client type
            assert_eq!(
                TaskConfigTemplate {
                    client_type: Some(Official),
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        TaskTemplate::new(StartUp, template!("client_type" => "YoStarEN")),
                        TaskTemplate::new(CloseDown, template!("client_type" => "YoStarJP")),
                    ],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: false,
                    close_app: true,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            template!("client_type" => "Official").resolve().unwrap()
                        ),
                        Task::new(
                            CloseDown,
                            template!("client_type" => "Official").resolve().unwrap()
                        ),
                    ]
                }
            );

            // Filename will be converted to absolute path
            #[cfg(unix)]
            assert_eq!(
                TaskConfigTemplate {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        TaskTemplate::new(Infrast, template!("filename" => "daily.json")),
                        TaskTemplate::new(Infrast, template!("filename" => "/tmp/daily.json")),
                    ],
                }
                .init()
                .unwrap(),
                TaskConfig {
                    client_type: Official,
                    start_app: false,
                    close_app: false,
                    tasks: vec![
                        Task::new(
                            Infrast,
                            template!("filename" => dirs::abs_config("daily.json", Some("infrast")).unwrap()??).resolve().unwrap(),
                        ),
                        Task::new(Infrast, template!("filename" => "/tmp/daily.json").resolve().unwrap())
                    ]
                }
            );
        }

        #[test]
        fn initialized_task() {
            let task = Task::new(Fight, template!("stage" => "1-7").resolve().unwrap())
                .with_name("Fight Daily".to_string());
            assert_eq!(task.name_or_default(), "Fight Daily");
            assert_eq!(task.task_type, Fight);
            assert_eq!(
                &task.params,
                &template!("stage" => "1-7").resolve().unwrap()
            );
            assert_eq!(task.name, Some(String::from("Fight Daily")));

            let task = Task::new(Fight, template!("stage" => "1-7").resolve().unwrap());
            assert_eq!(task.name_or_default(), "Fight");
            assert_eq!(task.task_type, Fight);
            assert_eq!(
                &task.params,
                &template!("stage" => "1-7").resolve().unwrap()
            );
            assert_eq!(task.name, None);
        }
    }
}
