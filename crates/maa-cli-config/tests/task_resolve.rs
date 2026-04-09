use std::convert::Infallible;

use chrono::Utc;
use maa_cli_config::{ConditionContext, TaskConfigTemplate};
use maa_question::prelude::*;
use maa_value::{primitive::MAAPrimitive, value::MAAValue};

#[derive(Default)]
struct PanicResolver;

macro_rules! impl_resolve_for_panic_resolver {
    ($($question:ty),* $(,)?) => {
        $(
            impl Resolve<$question> for PanicResolver {
                type Error = Infallible;

                fn resolve(
                    &mut self,
                    _: $question,
                ) -> Result<<$question as Question>::Answer, Self::Error> {
                    panic!("unexpected interactive input during task resolve")
                }
            }
        )*
    };
}

impl_resolve_for_panic_resolver!(
    Confirm,
    Inquiry<i32>,
    Inquiry<f32>,
    Inquiry<String>,
    SelectD<i32>,
    SelectD<f32>,
    SelectD<String>,
);

#[test]
fn resolve_task_params_after_conditions_and_overrides() {
    let task_config: TaskConfigTemplate =
        serde_yaml::from_str(include_str!("../fixtures/task/placeholders.yaml")).unwrap();
    let session = &task_config.sessions[0];

    let now = chrono::DateTime::<Utc>::UNIX_EPOCH + chrono::TimeDelta::minutes(30);
    let context = ConditionContext {
        now,
        side_story_open_time: Some((
            chrono::DateTime::<Utc>::UNIX_EPOCH,
            chrono::DateTime::<Utc>::UNIX_EPOCH + chrono::TimeDelta::hours(1),
        )),
    };

    let mut resolver = PanicResolver;
    let params = session.tasks[0]
        .clone()
        .resolve(&context, &mut resolver)
        .unwrap()
        .unwrap()
        .params;

    assert_eq!(
        params.get("stage"),
        Some(&MAAValue::Primitive(MAAPrimitive::String(
            "${stage}".into()
        )))
    );
    assert_eq!(
        params.get("note"),
        Some(&MAAValue::Primitive(MAAPrimitive::String(
            "side ${stage}".into()
        )))
    );
}

#[test]
fn resolved_params_preserve_variable_placeholders() {
    let task_config: TaskConfigTemplate =
        serde_yaml::from_str(include_str!("../fixtures/task/placeholders.yaml")).unwrap();
    let session = &task_config.sessions[0];

    let now = chrono::DateTime::<Utc>::UNIX_EPOCH + chrono::TimeDelta::minutes(30);
    let context = ConditionContext {
        now,
        side_story_open_time: Some((
            chrono::DateTime::<Utc>::UNIX_EPOCH,
            chrono::DateTime::<Utc>::UNIX_EPOCH + chrono::TimeDelta::hours(1),
        )),
    };

    let mut resolver = PanicResolver;
    let params = session.tasks[0]
        .clone()
        .resolve(&context, &mut resolver)
        .unwrap()
        .unwrap()
        .params;

    assert_eq!(
        params.get("stage"),
        Some(&MAAValue::Primitive(MAAPrimitive::String(
            "${stage}".into()
        )))
    );
    assert_eq!(
        params.get("note"),
        Some(&MAAValue::Primitive(MAAPrimitive::String(
            "side ${stage}".into()
        )))
    );
}

#[test]
fn inactive_task_does_not_resolve_params() {
    let task_config: TaskConfigTemplate =
        serde_yaml::from_str(include_str!("../fixtures/task/placeholders.yaml")).unwrap();
    let session = &task_config.sessions[0];

    let context = ConditionContext {
        now: chrono::DateTime::<Utc>::UNIX_EPOCH,
        side_story_open_time: None,
    };

    assert!(
        session.tasks[1]
            .clone()
            .resolve(&context, &mut PanicResolver)
            .unwrap()
            .is_none()
    );
}
