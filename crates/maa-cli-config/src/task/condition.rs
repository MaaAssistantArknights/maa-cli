use std::num::NonZeroU32;

use chrono::{Datelike, NaiveDateTime, NaiveTime, Utc, Weekday};
use nonempty_vec::NonEmptyVec;
use serde::{Deserialize, de::Error as _};
use serde_json::Value;

use super::time::{DateTime, TimeIn, TimeRange, TimeZone};

type DateTimeRange = TimeRange<NaiveDateTime>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionContext {
    pub now: DateTime,
    pub side_story_open_time: Option<(DateTime, DateTime)>,
}

impl ConditionContext {
    pub fn new() -> Self {
        Self {
            now: Utc::now(),
            side_story_open_time: None,
        }
    }

    pub fn with_now(now: DateTime) -> Self {
        Self {
            now,
            side_story_open_time: None,
        }
    }
}

impl Default for ConditionContext {
    fn default() -> Self {
        Self {
            now: DateTime::UNIX_EPOCH,
            side_story_open_time: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition {
    Always,
    OnSideStory,
    Weekday {
        weekdays: NonEmptyVec<Weekday>,
        timezone: TimeZone,
    },
    DayMod {
        divisor: NonZeroU32,
        remainder: u32,
        timezone: TimeZone,
    },
    Time {
        time_range: TimeRange<NaiveTime>,
        timezone: TimeZone,
    },
    DateTime {
        date_range: DateTimeRange,
        timezone: TimeZone,
    },
    All {
        all: NonEmptyVec<Condition>,
    },
    Any {
        any: NonEmptyVec<Condition>,
    },
    Not {
        not: Box<Condition>,
    },
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Condition {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Condition".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::Condition").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let weekdays = generator.subschema_for::<NonEmptyVec<Weekday>>();
        let timezone = generator.subschema_for::<TimeZone>();
        let time_range = generator.subschema_for::<TimeRange<NaiveTime>>();
        let date_range = generator.subschema_for::<DateTimeRange>();
        let all = generator.subschema_for::<NonEmptyVec<Condition>>();
        let any = generator.subschema_for::<NonEmptyVec<Condition>>();
        let not = generator.subschema_for::<Condition>();

        schemars::json_schema!({
            "oneOf": [
                {
                    "type": "string",
                    "enum": SIMPLE_CONDITION_NAMES,
                },
                {
                    "type": "object",
                    "required": ["weekdays"],
                    "properties": {
                        "weekdays": weekdays,
                        "timezone": timezone,
                    },
                    "additionalProperties": false,
                },
                {
                    "type": "object",
                    "required": ["divisor"],
                    "properties": {
                        "divisor": { "type": "integer", "minimum": 1 },
                        "remainder": { "type": "integer", "minimum": 0, "default": 0 },
                        "timezone": timezone,
                    },
                    "additionalProperties": false,
                },
                {
                    "type": "object",
                    "required": ["time_range"],
                    "properties": {
                        "time_range": time_range,
                        "timezone": timezone,
                    },
                    "additionalProperties": false,
                },
                {
                    "type": "object",
                    "required": ["date_range"],
                    "properties": {
                        "date_range": date_range,
                        "timezone": timezone,
                    },
                    "additionalProperties": false,
                },
                {
                    "type": "object",
                    "required": ["all"],
                    "properties": {
                        "all": all,
                    },
                    "additionalProperties": false,
                },
                {
                    "type": "object",
                    "required": ["any"],
                    "properties": {
                        "any": any,
                    },
                    "additionalProperties": false,
                },
                {
                    "type": "object",
                    "required": ["not"],
                    "properties": {
                        "not": not,
                    },
                    "additionalProperties": false,
                }
            ]
        })
    }
}

const SIMPLE_CONDITION_NAMES: &[&str] = &["Always", "OnSideStory"];
const CONDITION_FIELDS: &[&str] = &[
    "weekdays",
    "divisor",
    "remainder",
    "time_range",
    "date_range",
    "all",
    "any",
    "not",
    "timezone",
];
const CONDITION_PRIMARY_FIELDS: &[&str] = &[
    "weekdays",
    "divisor",
    "time_range",
    "date_range",
    "all",
    "any",
    "not",
];

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::String(value) => match value.as_str() {
                "Always" => Ok(Self::Always),
                "OnSideStory" => Ok(Self::OnSideStory),
                _ => Err(D::Error::unknown_variant(&value, SIMPLE_CONDITION_NAMES)),
            },
            Value::Object(object) => {
                if object.is_empty() {
                    return Err(D::Error::invalid_length(0, &"a non-empty condition object"));
                }

                if object
                    .keys()
                    .all(|key| CONDITION_FIELDS.contains(&key.as_str()))
                    && !object
                        .keys()
                        .any(|key| CONDITION_PRIMARY_FIELDS.contains(&key.as_str()))
                {
                    return Err(D::Error::invalid_value(
                        serde::de::Unexpected::Other(
                            "condition object without a primary condition field",
                        ),
                        &"a condition object containing one of `weekdays`, `divisor`, `time_range`, `date_range`, `all`, `any`, or `not`",
                    ));
                }

                if object.contains_key("weekdays") {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct WeekdayCondition {
                        weekdays: NonEmptyVec<Weekday>,
                        #[serde(default)]
                        timezone: TimeZone,
                    }

                    let condition: WeekdayCondition =
                        serde_json::from_value(Value::Object(object)).map_err(D::Error::custom)?;
                    return Ok(Self::Weekday {
                        weekdays: condition.weekdays,
                        timezone: condition.timezone,
                    });
                }

                if object.contains_key("divisor") {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct DayModCondition {
                        divisor: NonZeroU32,
                        #[serde(default)]
                        remainder: u32,
                        #[serde(default)]
                        timezone: TimeZone,
                    }

                    let condition: DayModCondition =
                        serde_json::from_value(Value::Object(object)).map_err(D::Error::custom)?;
                    return Ok(Self::DayMod {
                        divisor: condition.divisor,
                        remainder: condition.remainder,
                        timezone: condition.timezone,
                    });
                }

                if object.contains_key("time_range") {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct TimeCondition {
                        time_range: TimeRange<NaiveTime>,
                        #[serde(default)]
                        timezone: TimeZone,
                    }

                    let condition: TimeCondition =
                        serde_json::from_value(Value::Object(object)).map_err(D::Error::custom)?;
                    return Ok(Self::Time {
                        time_range: condition.time_range,
                        timezone: condition.timezone,
                    });
                }

                if object.contains_key("date_range") {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct DateTimeCondition {
                        date_range: DateTimeRange,
                        #[serde(default)]
                        timezone: TimeZone,
                    }

                    let condition: DateTimeCondition =
                        serde_json::from_value(Value::Object(object)).map_err(D::Error::custom)?;
                    return Ok(Self::DateTime {
                        date_range: condition.date_range,
                        timezone: condition.timezone,
                    });
                }

                if object.contains_key("all") {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct AllCondition {
                        all: NonEmptyVec<Condition>,
                    }

                    let condition: AllCondition =
                        serde_json::from_value(Value::Object(object)).map_err(D::Error::custom)?;
                    return Ok(Self::All { all: condition.all });
                }

                if object.contains_key("any") {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct AnyCondition {
                        any: NonEmptyVec<Condition>,
                    }

                    let condition: AnyCondition =
                        serde_json::from_value(Value::Object(object)).map_err(D::Error::custom)?;
                    return Ok(Self::Any { any: condition.any });
                }

                if object.contains_key("not") {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct NotCondition {
                        not: Box<Condition>,
                    }

                    let condition: NotCondition =
                        serde_json::from_value(Value::Object(object)).map_err(D::Error::custom)?;
                    return Ok(Self::Not { not: condition.not });
                }

                let field = object.keys().next().map(String::as_str).unwrap_or_default();
                Err(D::Error::unknown_field(field, CONDITION_FIELDS))
            }
            Value::Bool(_) => Err(D::Error::invalid_type(
                serde::de::Unexpected::Bool(false),
                &"a string or object condition",
            )),
            Value::Number(_) => Err(D::Error::invalid_type(
                serde::de::Unexpected::Other("number"),
                &"a string or object condition",
            )),
            Value::Array(_) => Err(D::Error::invalid_type(
                serde::de::Unexpected::Seq,
                &"a string or object condition",
            )),
            Value::Null => Err(D::Error::invalid_type(
                serde::de::Unexpected::Unit,
                &"a string or object condition",
            )),
        }
    }
}

impl Condition {
    pub fn is_active(&self, context: &ConditionContext) -> bool {
        match self {
            Self::Always => true,
            Self::OnSideStory => context
                .side_story_open_time
                .is_some_and(|(from, until)| context.now.is_in_range(&from, &until)),
            Self::Weekday { weekdays, timezone } => {
                let now = context.to_naive_datetime_in(timezone);
                weekdays.contains(&now.weekday())
            }
            Self::DayMod {
                divisor,
                remainder,
                timezone,
            } => {
                let now = context.to_naive_datetime_in(timezone);
                now.num_days_from_ce() as u32 % divisor.get() == *remainder
            }
            Self::Time {
                time_range,
                timezone,
            } => {
                let now = context.to_naive_datetime_in(timezone).time();
                time_range.contains(&now)
            }
            Self::DateTime {
                date_range,
                timezone,
            } => {
                let now = context.to_naive_datetime_in(timezone);
                date_range.contains(&now)
            }
            Self::All { all } => all.iter().all(|condition| condition.is_active(context)),
            Self::Any { any } => any.iter().any(|condition| condition.is_active(context)),
            Self::Not { not } => !not.is_active(context),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone as _};

    use super::*;

    fn context(tz: i32, y: i32, m: u32, d: u32, h: u32, min: u32) -> ConditionContext {
        let local_offset = chrono::FixedOffset::east_opt(tz * 3600).unwrap();
        let local_time = local_offset.with_ymd_and_hms(y, m, d, h, min, 0).unwrap();

        ConditionContext {
            now: local_time.with_timezone(&Utc),
            side_story_open_time: None,
        }
    }

    mod deserialize {
        use super::*;

        #[test]
        fn simple_condition() {
            let condition: Condition = serde_yaml::from_str("Always").unwrap();
            assert_eq!(condition, Condition::Always);

            let condition: Condition = serde_yaml::from_str("OnSideStory").unwrap();
            assert_eq!(condition, Condition::OnSideStory);
        }

        #[test]
        fn reject_empty_time_condition() {
            let err = serde_yaml::from_str::<TimeRange<NaiveTime>>("{}").unwrap_err();
            assert!(
                err.to_string()
                    .contains("`time_range.from` or `time_range.until`")
            );
        }

        #[test]
        fn reject_empty_weekdays() {
            let err = serde_yaml::from_str::<NonEmptyVec<Weekday>>("[]").unwrap_err();
            assert!(err.to_string().contains("invalid length 0"));
        }

        #[test]
        fn time_offset() {
            assert_eq!(
                serde_yaml::from_str::<TimeZone>("Local").unwrap(),
                TimeZone::Local
            );
            assert_eq!(
                serde_yaml::from_str::<TimeZone>("Official").unwrap(),
                TimeZone::Client(maa_types::ClientType::Official)
            );
            assert_eq!(
                serde_yaml::from_str::<TimeZone>("8").unwrap(),
                TimeZone::FixedOffset(crate::task::time::UtcOffsetHours::try_from(8).unwrap())
            );
        }

        #[test]
        fn reject_invalid_time_offset() {
            let err = serde_yaml::from_str::<TimeZone>("15").unwrap_err();
            assert!(err.to_string().contains("unsupported UTC offset"));
        }

        #[test]
        fn reject_zero_day_mod_divisor() {
            let err =
                serde_yaml::from_str::<Condition>("divisor: 0\nremainder: 0\ntimezone: Local\n")
                    .unwrap_err();
            assert!(err.to_string().contains("nonzero"));
        }
    }

    mod eval {
        use maa_types::ClientType;

        use super::*;

        #[test]
        fn default_context_uses_stable_defaults() {
            let context = ConditionContext::default();
            assert_eq!(context.now, DateTime::UNIX_EPOCH);
            assert_eq!(context.side_story_open_time, None);
        }

        #[test]
        fn new_context_uses_current_time() {
            let context = ConditionContext::new();
            assert!(context.now <= Utc::now());
        }

        #[test]
        fn always_condition_is_active() {
            assert!(Condition::Always.is_active(&ConditionContext::default()));
        }

        #[test]
        fn on_side_story_uses_open_time_range() {
            let mut context =
                ConditionContext::with_now(Utc.with_ymd_and_hms(2026, 4, 10, 12, 0, 0).unwrap());
            context.side_story_open_time = Some((
                Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 4, 11, 0, 0, 0).unwrap(),
            ));

            assert!(Condition::OnSideStory.is_active(&context));
        }

        #[test]
        fn time_offset_date_time_matches_old_server_day_logic() {
            let context = context(8, 2024, 2, 14, 4, 0);

            assert_eq!(
                TimeZone::Client(ClientType::Official).date_time(&context),
                NaiveDate::from_ymd_opt(2024, 2, 14)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            );
            assert_eq!(
                TimeZone::FixedOffset(crate::task::time::UtcOffsetHours::try_from(0).unwrap())
                    .date_time(&context),
                NaiveDate::from_ymd_opt(2024, 2, 13)
                    .unwrap()
                    .and_hms_opt(20, 0, 0)
                    .unwrap(),
            );
        }

        #[test]
        fn weekday_condition_uses_selected_timezone() {
            let context = context(8, 2024, 2, 14, 4, 1);

            assert!(
                Condition::Weekday {
                    weekdays: NonEmptyVec::new(vec![Weekday::Wed]).unwrap(),
                    timezone: TimeZone::Client(ClientType::Official),
                }
                .is_active(&context)
            );
            assert!(
                !Condition::Weekday {
                    weekdays: NonEmptyVec::new(vec![Weekday::Tue]).unwrap(),
                    timezone: TimeZone::Client(ClientType::Official),
                }
                .is_active(&context)
            );
        }

        #[test]
        fn day_mod_condition_uses_selected_timezone() {
            let context = context(8, 2024, 2, 14, 4, 1);
            let num_days = NaiveDate::from_ymd_opt(2024, 2, 14)
                .unwrap()
                .num_days_from_ce() as u32;

            assert!(
                Condition::DayMod {
                    divisor: NonZeroU32::new(2).unwrap(),
                    remainder: num_days % 2,
                    timezone: TimeZone::Client(ClientType::Official),
                }
                .is_active(&context)
            );
        }

        #[test]
        fn time_condition_supports_wraparound_ranges() {
            let context = context(8, 2024, 2, 14, 0, 30);

            assert!(
                Condition::Time {
                    time_range: TimeRange {
                        from: Some(NaiveTime::from_hms_opt(23, 0, 0).unwrap()),
                        until: Some(NaiveTime::from_hms_opt(2, 0, 0).unwrap()),
                    },
                    timezone: TimeZone::FixedOffset(
                        crate::task::time::UtcOffsetHours::try_from(8).unwrap(),
                    ),
                }
                .is_active(&context)
            );
        }

        #[test]
        fn datetime_condition_supports_ranges() {
            let context = context(8, 2024, 2, 14, 12, 0);

            assert!(
                Condition::DateTime {
                    date_range: DateTimeRange {
                        from: Some(
                            NaiveDate::from_ymd_opt(2024, 2, 14)
                                .unwrap()
                                .and_hms_opt(11, 0, 0)
                                .unwrap(),
                        ),
                        until: Some(
                            NaiveDate::from_ymd_opt(2024, 2, 14)
                                .unwrap()
                                .and_hms_opt(13, 0, 0)
                                .unwrap(),
                        ),
                    },
                    timezone: TimeZone::FixedOffset(
                        crate::task::time::UtcOffsetHours::try_from(8).unwrap(),
                    ),
                }
                .is_active(&context)
            );
        }

        #[test]
        fn all_any_not_conditions_work() {
            let mut context =
                ConditionContext::with_now(Utc.with_ymd_and_hms(2026, 4, 10, 12, 0, 0).unwrap());
            context.side_story_open_time = Some((
                Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 4, 11, 0, 0, 0).unwrap(),
            ));

            let always = Condition::Always;
            let side_story = Condition::OnSideStory;

            assert!(
                Condition::All {
                    all: NonEmptyVec::new(vec![always.clone(), side_story.clone()]).unwrap(),
                }
                .is_active(&context)
            );
            assert!(
                Condition::Any {
                    any: NonEmptyVec::new(vec![
                        Condition::Not {
                            not: Box::new(always.clone()),
                        },
                        side_story,
                    ])
                    .unwrap(),
                }
                .is_active(&context)
            );
        }

        #[test]
        fn time_condition_uses_open_ended_bounds() {
            let context = context(8, 2024, 2, 14, 12, 0);

            assert!(
                Condition::Time {
                    time_range: TimeRange {
                        from: Some(NaiveTime::from_hms_opt(11, 0, 0).unwrap()),
                        until: None,
                    },
                    timezone: TimeZone::FixedOffset(
                        crate::task::time::UtcOffsetHours::try_from(8).unwrap(),
                    ),
                }
                .is_active(&context)
            );
            assert!(
                Condition::Time {
                    time_range: TimeRange {
                        from: None,
                        until: Some(NaiveTime::from_hms_opt(13, 0, 0).unwrap()),
                    },
                    timezone: TimeZone::FixedOffset(
                        crate::task::time::UtcOffsetHours::try_from(8).unwrap(),
                    ),
                }
                .is_active(&context)
            );
        }

        #[test]
        fn datetime_condition_uses_open_ended_bounds() {
            let context = context(8, 2024, 2, 14, 12, 0);

            assert!(
                Condition::DateTime {
                    date_range: DateTimeRange {
                        from: Some(
                            NaiveDate::from_ymd_opt(2024, 2, 14)
                                .unwrap()
                                .and_hms_opt(11, 0, 0)
                                .unwrap(),
                        ),
                        until: None,
                    },
                    timezone: TimeZone::FixedOffset(
                        crate::task::time::UtcOffsetHours::try_from(8).unwrap(),
                    ),
                }
                .is_active(&context)
            );
            assert!(
                Condition::DateTime {
                    date_range: DateTimeRange {
                        from: None,
                        until: Some(
                            NaiveDate::from_ymd_opt(2024, 2, 14)
                                .unwrap()
                                .and_hms_opt(13, 0, 0)
                                .unwrap(),
                        ),
                    },
                    timezone: TimeZone::FixedOffset(
                        crate::task::time::UtcOffsetHours::try_from(8).unwrap(),
                    ),
                }
                .is_active(&context)
            );
        }
    }
}
