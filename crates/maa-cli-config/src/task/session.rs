use maa_value::prelude::*;
use nonempty_vec::NonEmptyVec;
use serde::Deserialize;

use super::{ConditionContext, Task, TaskTemplate};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SessionTemplate {
    #[serde(default)]
    pub account_name: Option<String>,
    pub tasks: NonEmptyVec<TaskTemplate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub account_name: Option<String>,
    pub tasks: NonEmptyVec<Task>,
}

impl SessionTemplate {
    pub fn resolve(
        self,
        context: &ConditionContext,
        resolver: &mut impl MAAInputResolver,
    ) -> Result<Option<Session>, maa_value::error::Error> {
        let Self {
            account_name,
            tasks,
        } = self;

        let tasks = tasks
            .into_vec()
            .into_iter()
            .map(|task| task.resolve(context, resolver))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let Some(tasks) = NonEmptyVec::new(tasks) else {
            return Ok(None);
        };

        Ok(Some(Session {
            account_name,
            tasks,
        }))
    }
}
