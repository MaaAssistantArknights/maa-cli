use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("either `tasks` or `sessions` must be provided")]
    MissingTaskMode,
    #[error("`tasks` and `sessions` cannot be used together")]
    ConflictingTaskModes,
    #[error("`account_name` is only allowed in `tasks` mode")]
    AccountNameWithSessions,
    #[error("`weekdays` cannot be empty")]
    EmptyWeekdays,
    #[error("`time_range.from` or `time_range.until` must be set")]
    EmptyTimeCondition,
    #[error("`date_range.from` or `date_range.until` must be set")]
    EmptyDateTimeCondition,
    #[error("`all` cannot be empty")]
    EmptyAllCondition,
    #[error("`any` cannot be empty")]
    EmptyAnyCondition,
    #[error("unknown condition")]
    UnknownCondition,
    #[error("condition cannot be an empty object")]
    EmptyConditionObject,
}
