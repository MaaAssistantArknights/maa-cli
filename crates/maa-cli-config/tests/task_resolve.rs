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
fn on_side_story_boundary_at_from_is_inclusive() {
    // Test that OnSideStory is active at exactly the from boundary [from, until)
    let task_config: TaskConfigTemplate =
        serde_yaml::from_str(include_str!("../fixtures/task/placeholders.yaml")).unwrap();
    let session = &task_config.sessions[0];

    let from = chrono::DateTime::<Utc>::UNIX_EPOCH;
    let until = from + chrono::TimeDelta::hours(1);

    // At exactly from boundary - should be active
    let context = ConditionContext {
        now: from,
        side_story_open_time: Some((from, until)),
    };

    let mut resolver = PanicResolver;
    let params = session.tasks[0]
        .clone()
        .resolve(&context, &mut resolver)
        .unwrap()
        .unwrap()
        .params;

    // Should have the side story override since OnSideStory is active at from boundary
    assert_eq!(
        params.get("note"),
        Some(&MAAValue::Primitive(MAAPrimitive::String(
            "side ${stage}".into()
        )))
    );
}

#[test]
fn on_side_story_boundary_at_until_is_exclusive() {
    // Test that OnSideStory is inactive at exactly the until boundary [from, until)
    let task_config: TaskConfigTemplate =
        serde_yaml::from_str(include_str!("../fixtures/task/placeholders.yaml")).unwrap();
    let session = &task_config.sessions[0];

    let from = chrono::DateTime::<Utc>::UNIX_EPOCH;
    let until = from + chrono::TimeDelta::hours(1);

    // At exactly until boundary - should be inactive
    let context = ConditionContext {
        now: until,
        side_story_open_time: Some((from, until)),
    };

    let mut resolver = PanicResolver;
    let params = session.tasks[0]
        .clone()
        .resolve(&context, &mut resolver)
        .unwrap()
        .unwrap()
        .params;

    // Should NOT have the side story override since OnSideStory is inactive at until boundary
    assert_eq!(
        params.get("note"),
        Some(&MAAValue::Primitive(MAAPrimitive::String(
            "farm ${stage}".into()
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
