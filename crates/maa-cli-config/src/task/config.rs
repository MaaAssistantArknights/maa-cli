use maa_value::prelude::*;
use nonempty_vec::{NonEmptyVec, nevec};
use serde::Deserialize;

use super::{ConditionContext, Session, SessionTemplate, TaskTemplate};
use crate::ValidationError;

#[cfg(feature = "schema")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VersionedTaskConfig {
    pub version: crate::Version,
    #[serde(flatten)]
    pub config: TaskConfigTemplate,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(try_from = "RawTaskConfig")]
pub struct TaskConfigTemplate {
    pub manage_environment_lifecycle: bool,
    pub manage_game_lifecycle: bool,
    pub sessions: NonEmptyVec<SessionTemplate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskConfig {
    pub manage_environment_lifecycle: bool,
    pub manage_game_lifecycle: bool,
    pub sessions: NonEmptyVec<Session>,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize)]
struct RawTaskConfig {
    #[serde(default = "default_true")]
    manage_environment_lifecycle: bool,
    #[serde(default = "default_true")]
    manage_game_lifecycle: bool,
    #[serde(default)]
    account_name: Option<String>,
    #[serde(default)]
    tasks: Option<NonEmptyVec<TaskTemplate>>,
    #[serde(default)]
    sessions: Option<NonEmptyVec<SessionTemplate>>,
}

impl TryFrom<RawTaskConfig> for TaskConfigTemplate {
    type Error = ValidationError;

    fn try_from(value: RawTaskConfig) -> Result<Self, Self::Error> {
        let sessions = match (value.tasks, value.sessions) {
            (Some(_), Some(_)) => return Err(ValidationError::ConflictingTaskModes),
            (None, None) => return Err(ValidationError::MissingTaskMode),
            (Some(tasks), None) => {
                nevec![SessionTemplate {
                    account_name: value.account_name,
                    tasks
                }]
            }
            (None, Some(sessions)) => {
                if value.account_name.is_some() {
                    return Err(ValidationError::AccountNameWithSessions);
                }
                sessions
            }
        };

        Ok(Self {
            manage_environment_lifecycle: value.manage_environment_lifecycle,
            manage_game_lifecycle: value.manage_game_lifecycle,
            sessions,
        })
    }
}

impl TaskConfigTemplate {
    pub fn resolve(
        self,
        context: &ConditionContext,
        resolver: &mut impl MAAInputResolver,
    ) -> Result<Option<TaskConfig>, maa_value::error::Error> {
        let Self {
            manage_environment_lifecycle,
            manage_game_lifecycle,
            sessions,
        } = self;

        let sessions = sessions
            .into_vec()
            .into_iter()
            .map(|session| session.resolve(context, resolver))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let Some(sessions) = NonEmptyVec::new(sessions) else {
            return Ok(None);
        };

        Ok(Some(TaskConfig {
            manage_environment_lifecycle,
            manage_game_lifecycle,
            sessions,
        }))
    }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_task_config_error(content: &str, expected: &str) {
        let err = serde_yaml::from_str::<TaskConfigTemplate>(content).unwrap_err();
        assert!(err.to_string().contains(expected), "{err}");
    }

    #[test]
    fn deserialize_tasks_mode() {
        let task_config: TaskConfigTemplate = serde_yaml::from_str(
            r#"
version: 2
account_name: main
tasks:
  - type: Fight
    params:
      stage: 1-7
"#,
        )
        .unwrap();

        assert!(task_config.manage_environment_lifecycle);
        assert!(task_config.manage_game_lifecycle);
        assert_eq!(task_config.sessions.len(), 1);
        assert_eq!(
            task_config.sessions[0].account_name.as_deref(),
            Some("main")
        );
    }

    #[test]
    fn deserialize_sessions_mode() {
        let task_config: TaskConfigTemplate = serde_yaml::from_str(
            r#"
version: 2
sessions:
  - account_name: main
    tasks:
      - type: Fight
        params:
          stage: 1-7
"#,
        )
        .unwrap();

        assert_eq!(task_config.sessions.len(), 1);
        assert_eq!(task_config.sessions[0].tasks.len(), 1);
    }

    #[test]
    fn reject_conflicting_task_modes() {
        let err = serde_yaml::from_str::<TaskConfigTemplate>(
            r#"
version: 2
tasks:
  - type: Fight
    params:
      stage: 1-7
sessions:
  - tasks:
      - type: Fight
        params:
          stage: CE-6
"#,
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("`tasks` and `sessions` cannot be used together")
        );
    }

    #[test]
    fn reject_account_name_with_sessions() {
        let err = serde_yaml::from_str::<TaskConfigTemplate>(
            r#"
version: 2
account_name: main
sessions:
  - tasks:
      - type: Fight
        params:
          stage: 1-7
"#,
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("`account_name` is only allowed in `tasks` mode")
        );
    }

    #[test]
    fn reject_empty_tasks() {
        let err = serde_yaml::from_str::<TaskConfigTemplate>(
            r#"
version: 2
tasks: []
"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("invalid length 0"));
    }

    #[test]
    fn reject_empty_sessions() {
        let err = serde_yaml::from_str::<TaskConfigTemplate>(
            r#"
version: 2
sessions: []
"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("invalid length 0"));
    }

    #[test]
    fn deserialize_tasks_mode_from_toml_fixture() {
        let task_config: TaskConfigTemplate =
            toml::from_str(include_str!("../../fixtures/task/tasks.toml")).unwrap();

        assert!(task_config.manage_environment_lifecycle);
        assert!(!task_config.manage_game_lifecycle);
        assert_eq!(task_config.sessions.len(), 1);
        assert_eq!(task_config.sessions[0].tasks.len(), 2);
    }

    #[test]
    fn deserialize_tasks_mode_from_json_fixture() {
        let task_config: TaskConfigTemplate =
            serde_json::from_str(include_str!("../../fixtures/task/tasks.json")).unwrap();

        assert!(!task_config.manage_environment_lifecycle);
        assert!(!task_config.manage_game_lifecycle);
        assert_eq!(task_config.sessions.len(), 1);
        assert_eq!(task_config.sessions[0].tasks.len(), 1);
    }

    #[test]
    fn deserialize_sessions_mode_from_yaml_fixture() {
        let task_config: TaskConfigTemplate =
            serde_yaml::from_str(include_str!("../../fixtures/task/sessions.yaml")).unwrap();

        assert!(task_config.manage_environment_lifecycle);
        assert!(task_config.manage_game_lifecycle);
        assert_eq!(task_config.sessions.len(), 2);
        assert_eq!(
            task_config.sessions[0].account_name.as_deref(),
            Some("main")
        );
        assert_eq!(task_config.sessions[1].account_name.as_deref(), Some("alt"));
    }

    #[test]
    fn deserialize_sessions_mode_from_toml_fixture() {
        let task_config: TaskConfigTemplate =
            toml::from_str(include_str!("../../fixtures/task/sessions.toml")).unwrap();

        assert!(task_config.manage_environment_lifecycle);
        assert!(task_config.manage_game_lifecycle);
        assert_eq!(task_config.sessions.len(), 2);
        assert_eq!(task_config.sessions[1].tasks.len(), 1);
    }

    #[test]
    fn deserialize_tasks_with_additional_condition_paths() {
        let task_config: TaskConfigTemplate =
            serde_yaml::from_str(include_str!("../../fixtures/task/conditions.yaml")).unwrap();

        assert_eq!(task_config.sessions.len(), 1);
        assert_eq!(task_config.sessions[0].tasks.len(), 5);
    }

    #[test]
    fn reject_missing_mode_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/missing_mode.yaml"),
            "either `tasks` or `sessions` must be provided",
        );
    }

    #[test]
    fn reject_conflicting_modes_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/conflicting_modes.yaml"),
            "`tasks` and `sessions` cannot be used together",
        );
    }

    #[test]
    fn reject_account_name_with_sessions_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/account_name_with_sessions.yaml"),
            "`account_name` is only allowed in `tasks` mode",
        );
    }

    #[test]
    fn reject_empty_tasks_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_tasks.yaml"),
            "invalid length 0",
        );
    }

    #[test]
    fn reject_empty_sessions_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_sessions.yaml"),
            "invalid length 0",
        );
    }

    #[test]
    fn reject_empty_session_tasks_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_session_tasks.yaml"),
            "invalid length 0",
        );
    }

    #[test]
    fn reject_empty_weekdays_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_weekdays.yaml"),
            "invalid length 0",
        );
    }

    #[test]
    fn reject_empty_time_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_time.yaml"),
            "`time_range.from` or `time_range.until` must be set",
        );
    }

    #[test]
    fn reject_empty_datetime_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_datetime.yaml"),
            "`date_range.from` or `date_range.until` must be set",
        );
    }

    #[test]
    fn reject_empty_all_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_all.yaml"),
            "invalid length 0",
        );
    }

    #[test]
    fn reject_empty_any_fixture() {
        expect_task_config_error(
            include_str!("../../fixtures/invalid/empty_any.yaml"),
            "invalid length 0",
        );
    }

    #[cfg(feature = "schema")]
    #[test]
    fn reject_invalid_version_fixture() {
        let err = serde_yaml::from_str::<VersionedTaskConfig>(include_str!(
            "../../fixtures/invalid/version.yaml"
        ))
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported config version `3`, expected `2`"),
            "{err}"
        );
    }
}
