#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod error;
mod profile;
mod task;
mod version;

pub use error::ValidationError;
pub use maa_types::{ClientType, TaskType, TouchMode};
pub use profile::{
    AdvancedConfig, BehaviorConfig, ConnectionConfig, ProfileConfig, ResolvedProfileConfig,
    ScreencapMode, VersionedProfileConfig,
};
pub use task::{
    ConditionContext, Session, SessionTemplate, Task, TaskConfig, TaskConfigTemplate, TaskParams,
    TaskTemplate, VersionedTaskConfig,
};
pub use version::Version;

#[cfg(feature = "schema")]
pub fn profile_schema() -> schemars::Schema {
    schemars::schema_for!(profile::VersionedProfileConfig)
}

#[cfg(feature = "schema")]
pub fn task_schema() -> schemars::Schema {
    schemars::schema_for!(task::VersionedTaskConfig)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "schema")]
    use super::*;

    #[cfg(feature = "schema")]
    #[test]
    fn generate_profile_schema() {
        let schema = profile_schema();
        let value = serde_json::to_value(&schema).unwrap();
        assert!(value.is_object());
    }

    #[cfg(feature = "schema")]
    #[test]
    fn generate_task_schema() {
        let schema = task_schema();
        let value = serde_json::to_value(&schema).unwrap();
        assert!(value.is_object());
    }
}
