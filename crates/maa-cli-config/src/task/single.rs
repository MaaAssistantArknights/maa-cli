use maa_types::TaskType;
use maa_value::{map::StringMap, prelude::*};
use serde::{Deserialize, Serialize};

use super::{Condition, ConditionContext};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TaskTemplate {
    #[serde(rename = "type")]
    task_type: TaskType,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "if")]
    condition: Option<Condition>,
    #[serde(default)]
    params: TaskParamsTemplate,
    #[serde(default)]
    override_strategy: OverrideStrategy,
    #[serde(default)]
    overrides: Vec<TaskOverride>,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TaskOverride {
    #[serde(rename = "if")]
    condition: Condition,
    #[serde(default)]
    params: TaskParamsTemplate,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverrideStrategy {
    #[default]
    First,
    Merge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub task_type: TaskType,
    pub name: Option<String>,
    pub params: TaskParams,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(transparent)]
pub(super) struct TaskParamsTemplate(StringMap<MAAValueTemplate>);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaskParams(StringMap<MAAValue>);

impl TaskTemplate {
    pub fn is_active(&self, context: &ConditionContext) -> bool {
        match &self.condition {
            Some(condition) => condition.is_active(context),
            None => true,
        }
    }

    pub(super) fn resolve_params_template(
        &self,
        context: &ConditionContext,
    ) -> Option<TaskParamsTemplate> {
        if !self.is_active(context) {
            return None;
        }

        let mut params = self.params.clone();

        match self.override_strategy {
            OverrideStrategy::First => {
                if let Some(task_override) = self
                    .overrides
                    .iter()
                    .find(|task_override| task_override.condition.is_active(context))
                {
                    params.merge_from(&task_override.params);
                }
            }
            OverrideStrategy::Merge => {
                for task_override in &self.overrides {
                    if task_override.condition.is_active(context) {
                        params.merge_from(&task_override.params);
                    }
                }
            }
        }

        Some(params)
    }

    pub fn resolve(
        self,
        context: &ConditionContext,
        resolver: &mut impl MAAInputResolver,
    ) -> Result<Option<Task>, maa_value::error::Error> {
        let Some(params) = self.resolve_params_template(context) else {
            return Ok(None);
        };

        Ok(Some(Task {
            task_type: self.task_type,
            name: self.name,
            params: params.resolved_by(resolver)?,
        }))
    }
}

impl TaskParamsTemplate {
    pub(super) fn merge_from(&mut self, overlay: &TaskParamsTemplate) {
        for (key, value) in overlay.0.iter() {
            if let Some(current) = self.0.get_mut(key) {
                current.merge_from(value);
            } else {
                self.0.insert(key.clone(), value.clone());
            }
        }
    }

    pub(super) fn resolved_by(
        self,
        resolver: &mut impl MAAInputResolver,
    ) -> Result<TaskParams, maa_value::error::Error> {
        let params = self
            .0
            .into_iter()
            .map(|(key, value)| Ok((key, value.resolved_by(resolver)?)))
            .collect::<Result<StringMap<MAAValue>, maa_value::error::Error>>()?;

        Ok(TaskParams(params))
    }
}

impl std::ops::Deref for TaskParams {
    type Target = StringMap<MAAValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for TaskParamsTemplate {
    type Target = StringMap<MAAValueTemplate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use maa_value::convert::AsPrimitive;

    use super::*;

    #[test]
    fn task_lowered_params_use_first_override() {
        let task: TaskTemplate = serde_yaml::from_str(
            r#"
type: Fight
params:
  stage: 1-7
override_strategy: first
overrides:
  - if: Always
    params:
      stage: CE-6
  - if: Always
    params:
      stage: AP-5
"#,
        )
        .unwrap();

        let params = task
            .resolve_params_template(&ConditionContext::default())
            .unwrap();

        assert_eq!(
            params.get("stage").and_then(MAAValueTemplate::as_str),
            Some("CE-6")
        );
    }

    #[test]
    fn task_lowered_params_merge_matching_overrides() {
        let task: TaskTemplate = serde_yaml::from_str(
            r#"
type: Fight
params:
  stage: 1-7
  medicine: 0
override_strategy: merge
overrides:
  - if: Always
    params:
      stage: CE-6
  - if: Always
    params:
      medicine: 2
"#,
        )
        .unwrap();

        let params = task
            .resolve_params_template(&ConditionContext::default())
            .unwrap();

        assert_eq!(
            params.get("stage").and_then(MAAValueTemplate::as_str),
            Some("CE-6")
        );
        assert_eq!(
            params.get("medicine").and_then(MAAValueTemplate::as_int),
            Some(2)
        );
    }

    #[test]
    fn inactive_task_has_no_lowered_params() {
        let task: TaskTemplate = serde_yaml::from_str(
            r#"
type: Fight
if:
  weekdays: [Mon]
params:
  stage: 1-7
"#,
        )
        .unwrap();

        assert!(
            task.resolve_params_template(&ConditionContext::with_now(chrono::DateTime::UNIX_EPOCH))
                .is_none()
        );
    }
}
