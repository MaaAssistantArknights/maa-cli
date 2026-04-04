use std::path::PathBuf;

use anyhow::Context;
use maa_types::TaskType;
use maa_value::{MAAValue, insert, object};

use super::{ClientType, TaskConfig};
use crate::dirs;

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct InitializedTaskConfig {
    pub client_type: ClientType,
    pub startup_task: Option<InitializedTask>,
    pub tasks: Vec<InitializedTask>,
    pub closedown_task: Option<InitializedTask>,
}

#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct InitializedTask {
    pub name: Option<String>,
    pub task_type: TaskType,
    pub params: MAAValue,
}

impl InitializedTask {
    pub(super) const fn new(task_type: TaskType, params: MAAValue) -> Self {
        Self {
            name: None,
            task_type,
            params,
        }
    }

    pub(super) fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn name_or_default(&self) -> &str {
        self.name
            .as_deref()
            .unwrap_or_else(|| self.task_type.to_str())
    }
}

impl TaskConfig {
    pub fn init(&self) -> anyhow::Result<InitializedTaskConfig> {
        use TaskType::*;

        let mut startup = self.startup;
        let mut closedown = self.closedown;
        let mut client_type = self.client_type;
        let mut startup_candidate: Option<InitializedTask> = None;
        let mut tasks: Vec<InitializedTask> = Vec::new();
        let mut closedown_candidate: Option<InitializedTask> = None;

        for task in self.tasks.iter().filter(|t| t.is_active()) {
            let task_type = task.task_type();
            let mut params = task.params().init()?;

            match task_type {
                StartUp => startup = normalize_startup(&mut params, startup),
                CloseDown => closedown = normalize_closedown(&mut params, closedown),
                _ => resolve_filename(&mut params, task_type)?,
            }

            if let Some(s) = params.get("client_type").and_then(|v| v.as_str()) {
                let task_ct: ClientType = s.parse()?;
                match (task_ct, client_type) {
                    (t, None) => client_type = Some(t),
                    (t1, Some(t2)) if t1 != t2 => log::warn!(
                        "Task {} has client_type {}, but the client type is set to {} \
                         in previous tasks or config",
                        task.name.as_deref().unwrap_or_else(|| task_type.to_str()),
                        t1,
                        t2,
                    ),
                    _ => {}
                }
            }

            let mut inited = InitializedTask::new(task_type, params);
            if let Some(name) = &task.name {
                inited = inited.with_name(name.clone());
            }

            match task_type {
                StartUp if startup.unwrap_or(false) => startup_candidate = Some(inited),
                CloseDown => closedown_candidate = Some(inited),
                _ => tasks.push(inited),
            }
        }

        let client_type = client_type.unwrap_or_default();
        let ct = client_type.to_str();
        if let Some(t) = startup_candidate.as_mut() {
            insert!(t.params, "client_type" => ct);
        }
        for task in tasks.iter_mut() {
            if matches!(task.task_type, Fight) {
                insert!(task.params, "client_type" => ct);
            }
        }
        if let Some(t) = closedown_candidate.as_mut() {
            insert!(t.params, "client_type" => ct);
        }

        let startup_task = startup.unwrap_or(false).then(|| {
            startup_candidate.unwrap_or_else(|| {
                InitializedTask::new(
                    StartUp,
                    object!("start_game_enabled" => true, "client_type" => ct),
                )
            })
        });

        let closedown_task = closedown.unwrap_or(false).then(|| {
            closedown_candidate
                .unwrap_or_else(|| InitializedTask::new(CloseDown, object!("client_type" => ct)))
        });

        Ok(InitializedTaskConfig {
            client_type,
            startup_task,
            tasks,
            closedown_task,
        })
    }
}

fn normalize_startup(params: &mut MAAValue, startup: Option<bool>) -> Option<bool> {
    let start_game = params.get_or("enable", true) && params.get_or("start_game_enabled", false);
    match (start_game, startup) {
        (true, None) => Some(true),
        (false, Some(true)) => {
            insert!(*params, "enable" => true, "start_game_enabled" => true);
            startup
        }
        _ => startup,
    }
}

fn normalize_closedown(params: &mut MAAValue, closedown: Option<bool>) -> Option<bool> {
    match (params.get_or("enable", true), closedown) {
        (true, None) => Some(true),
        (false, Some(true)) => {
            insert!(*params, "enable" => true);
            closedown
        }
        _ => closedown,
    }
}

fn resolve_filename(params: &mut MAAValue, task_type: TaskType) -> anyhow::Result<()> {
    if let Some(v) = params.get_mut("filename") {
        let file = PathBuf::from(v.as_str().context("filename must be a string")?);
        let sub_dir = task_type.to_str().to_lowercase();
        if let Some(path) = dirs::abs_config(file, Some(sub_dir)) {
            *v = path.try_into()?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_types::TaskType::{self, *};
    use maa_value::{MAAValue, object};

    use super::{
        super::{ClientType::*, Task, TaskConfig, TaskVariant, condition::Condition},
        *,
    };
    use crate::dirs;

    // ── InitializedTask ────────────────────────────────────────────────────

    mod initialized_task {
        use super::*;

        #[test]
        fn name_or_default_with_name() {
            let task = InitializedTask::new(Fight, object!("stage" => "1-7"))
                .with_name("Fight Daily".to_string());
            assert_eq!(task.name_or_default(), "Fight Daily");
            assert_eq!(task.task_type, Fight);
            assert_eq!(&task.params, &object!("stage" => "1-7"));
            assert_eq!(task.name, Some(String::from("Fight Daily")));
        }

        #[test]
        fn name_or_default_without_name() {
            let task = InitializedTask::new(Fight, object!("stage" => "1-7"));
            assert_eq!(task.name_or_default(), "Fight");
            assert_eq!(task.task_type, Fight);
            assert_eq!(&task.params, &object!("stage" => "1-7"));
            assert_eq!(task.name, None);
        }
    }

    // ── normalize_startup ──────────────────────────────────────────────────

    mod normalize_startup_tests {
        use super::*;

        #[test]
        fn auto_enables_when_start_game_true() {
            let mut params = object!("start_game_enabled" => true);
            assert_eq!(normalize_startup(&mut params, None), Some(true));
            assert_eq!(params, object!("start_game_enabled" => true));
        }

        #[test]
        fn no_op_when_start_game_false_and_startup_none() {
            let mut params = object!("start_game_enabled" => false);
            assert_eq!(normalize_startup(&mut params, None), None);
            assert_eq!(params, object!("start_game_enabled" => false));
        }

        #[test]
        fn patches_params_when_startup_forced_but_task_disabled() {
            let mut params = object!("start_game_enabled" => false);
            assert_eq!(normalize_startup(&mut params, Some(true)), Some(true));
            assert_eq!(
                params,
                object!("start_game_enabled" => true, "enable" => true)
            );
        }

        #[test]
        fn patches_params_when_startup_forced_but_enable_false() {
            let mut params = object!("enable" => false, "start_game_enabled" => false);
            assert_eq!(normalize_startup(&mut params, Some(true)), Some(true));
            assert_eq!(
                params,
                object!("enable" => true, "start_game_enabled" => true)
            );
        }

        #[test]
        fn no_op_when_both_already_enabled() {
            let mut params = object!("start_game_enabled" => true);
            assert_eq!(normalize_startup(&mut params, Some(true)), Some(true));
            assert_eq!(params, object!("start_game_enabled" => true));
        }
    }

    // ── normalize_closedown ────────────────────────────────────────────────

    mod normalize_closedown_tests {
        use super::*;

        #[test]
        fn auto_enables_when_task_enabled() {
            let mut params = object!();
            assert_eq!(normalize_closedown(&mut params, None), Some(true));
            assert_eq!(params, object!());
        }

        #[test]
        fn no_op_when_task_disabled_and_closedown_none() {
            let mut params = object!("enable" => false);
            assert_eq!(normalize_closedown(&mut params, None), None);
            assert_eq!(params, object!("enable" => false));
        }

        #[test]
        fn patches_enable_when_closedown_forced_but_task_disabled() {
            let mut params = object!("enable" => false);
            assert_eq!(normalize_closedown(&mut params, Some(true)), Some(true));
            assert_eq!(params, object!("enable" => true));
        }

        #[test]
        fn no_op_when_both_already_enabled() {
            let mut params = object!("enable" => true);
            assert_eq!(normalize_closedown(&mut params, Some(true)), Some(true));
            assert_eq!(params, object!("enable" => true));
        }
    }

    // ── TaskConfig::init ──────────────────────────────────────────────────

    mod init_tests {
        use super::*;

        #[test]
        fn empty_config() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: None,
                    closedown_task: None,
                    tasks: vec![],
                }
            );
        }

        #[test]
        fn inactive_tasks_skipped() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        Task::new(StartUp, object!("start_game_enabled" => true)).with_variants(
                            vec![TaskVariant {
                                condition: Condition::Not {
                                    condition: Box::new(Condition::Always),
                                },
                                params: MAAValue::default(),
                            }]
                        )
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: None,
                    closedown_task: None,
                    tasks: vec![],
                }
            );
        }

        #[test]
        fn startup_extracted_with_name() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            object!("start_game_enabled" => true, "client_type" => "YoStarEN"),
                        )
                        .with_name(String::from("StartUp"))
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: YoStarEN,
                    startup_task: Some(
                        InitializedTask::new(
                            StartUp,
                            object!("start_game_enabled" => true, "client_type" => "YoStarEN"),
                        )
                        .with_name(String::from("StartUp")),
                    ),
                    closedown_task: None,
                    tasks: vec![],
                }
            );
        }

        #[test]
        fn startup_with_start_game_false_stays_in_tasks() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![Task::new(
                        StartUp,
                        object!("start_game_enabled" => false, "client_type" => "YoStarEN"),
                    )],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: YoStarEN,
                    startup_task: None,
                    closedown_task: None,
                    tasks: vec![InitializedTask::new(
                        StartUp,
                        object!("start_game_enabled" => false, "client_type" => "YoStarEN"),
                    )],
                }
            );
        }

        #[test]
        fn closedown_extracted_when_enabled() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![Task::new(CloseDown, object!("client_type" => "YoStarEN"))],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: YoStarEN,
                    startup_task: None,
                    closedown_task: Some(InitializedTask::new(
                        CloseDown,
                        object!("client_type" => "YoStarEN"),
                    )),
                    tasks: vec![],
                }
            );
        }

        #[test]
        fn closedown_disabled_not_materialised() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![Task::new(
                        CloseDown,
                        object!("enable" => false, "client_type" => "YoStarEN"),
                    )],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: YoStarEN,
                    startup_task: None,
                    closedown_task: None,
                    tasks: vec![],
                }
            );
        }

        #[test]
        fn closedown_gets_default_client_type() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![Task::new(CloseDown, object!())],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: None,
                    closedown_task: Some(InitializedTask::new(
                        CloseDown,
                        object!("client_type" => "Official"),
                    )),
                    tasks: vec![],
                }
            );
        }

        #[test]
        fn fight_gets_client_type_injected() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![Task::new(Fight, object!("client_type" => "YoStarEN"))],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: YoStarEN,
                    startup_task: None,
                    closedown_task: None,
                    tasks: vec![InitializedTask::new(
                        Fight,
                        object!("client_type" => "YoStarEN"),
                    )],
                }
            );
        }

        #[test]
        fn full_pipeline_startup_fight_closedown() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        Task::new(
                            StartUp,
                            object!("start_game_enabled" => true, "client_type" => "Official"),
                        ),
                        Task::new(Fight, object!("stage" => "1-7")),
                        Task::new(CloseDown, object!()),
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: Some(InitializedTask::new(
                        StartUp,
                        object!("start_game_enabled" => true, "client_type" => "Official"),
                    )),
                    closedown_task: Some(InitializedTask::new(
                        CloseDown,
                        object!("client_type" => "Official"),
                    )),
                    tasks: vec![InitializedTask::new(
                        Fight,
                        object!("stage" => "1-7", "client_type" => "Official"),
                    )],
                }
            );
        }

        #[test]
        fn config_flags_override_task_params() {
            assert_eq!(
                TaskConfig {
                    client_type: Some(Official),
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![
                        Task::new(StartUp, object!("start_game_enabled" => false)),
                        Task::new(Fight, object!("stage" => "1-7")),
                        Task::new(CloseDown, object!("enable" => false)),
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: Some(InitializedTask::new(
                        StartUp,
                        object!(
                            "enable" => true,
                            "start_game_enabled" => true,
                            "client_type" => "Official",
                        ),
                    )),
                    closedown_task: Some(InitializedTask::new(
                        CloseDown,
                        object!("enable" => true, "client_type" => "Official"),
                    )),
                    tasks: vec![InitializedTask::new(
                        Fight,
                        object!("stage" => "1-7", "client_type" => "Official"),
                    )],
                }
            );
        }

        #[test]
        fn auto_creates_startup_and_closedown() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![Task::new(Fight, object!("stage" => "1-7"))],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: Some(InitializedTask::new(
                        StartUp,
                        object!("start_game_enabled" => true, "client_type" => "Official"),
                    )),
                    closedown_task: Some(InitializedTask::new(
                        CloseDown,
                        object!("client_type" => "Official"),
                    )),
                    tasks: vec![InitializedTask::new(
                        Fight,
                        object!("stage" => "1-7", "client_type" => "Official"),
                    )],
                }
            );
        }

        #[test]
        fn auto_creates_with_explicit_client_type() {
            assert_eq!(
                TaskConfig {
                    client_type: Some(YoStarEN),
                    startup: Some(true),
                    closedown: Some(true),
                    tasks: vec![Task::new(Fight, object!("stage" => "1-7"))],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: YoStarEN,
                    startup_task: Some(InitializedTask::new(
                        StartUp,
                        object!("start_game_enabled" => true, "client_type" => "YoStarEN"),
                    )),
                    closedown_task: Some(InitializedTask::new(
                        CloseDown,
                        object!("client_type" => "YoStarEN"),
                    )),
                    tasks: vec![InitializedTask::new(
                        Fight,
                        object!("stage" => "1-7", "client_type" => "YoStarEN"),
                    )],
                }
            );
        }

        #[test]
        fn conflicting_client_type_uses_config_value() {
            assert_eq!(
                TaskConfig {
                    client_type: Some(Official),
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        Task::new(StartUp, object!("client_type" => "YoStarEN")),
                        Task::new(CloseDown, object!("client_type" => "YoStarJP")),
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: None,
                    closedown_task: Some(InitializedTask::new(
                        CloseDown,
                        object!("client_type" => "Official"),
                    )),
                    tasks: vec![InitializedTask::new(
                        StartUp,
                        object!("client_type" => "Official"),
                    )],
                }
            );
        }

        #[cfg(unix)]
        #[test]
        fn filename_resolved_to_absolute_path() {
            assert_eq!(
                TaskConfig {
                    client_type: None,
                    startup: None,
                    closedown: None,
                    tasks: vec![
                        Task::new(TaskType::Infrast, object!("filename" => "daily.json")),
                        Task::new(TaskType::Infrast, object!("filename" => "/tmp/daily.json")),
                    ],
                }
                .init()
                .unwrap(),
                InitializedTaskConfig {
                    client_type: Official,
                    startup_task: None,
                    closedown_task: None,
                    tasks: vec![
                        InitializedTask::new(
                            TaskType::Infrast,
                            object!("filename" => dirs::abs_config(
                                "daily.json",
                                Some("infrast")
                            )
                            .unwrap()??),
                        ),
                        InitializedTask::new(
                            TaskType::Infrast,
                            object!("filename" => "/tmp/daily.json"),
                        ),
                    ],
                }
            );
        }
    }
}
