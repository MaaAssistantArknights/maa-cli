use chrono::{FixedOffset, NaiveDateTime, NaiveTime, Utc};
use maa_types::ClientType;
use serde::Deserialize;

use crate::ValidationError;

pub type DateTime = chrono::DateTime<Utc>;

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "i8", into = "i8")]
pub struct UtcOffsetHours(i8);

impl UtcOffsetHours {
    pub const MAX: i8 = 14;
    pub const MIN: i8 = -12;

    pub const fn get(self) -> i8 {
        self.0
    }
}

impl TryFrom<i8> for UtcOffsetHours {
    type Error = String;

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(format!(
                "unsupported UTC offset `{value}`, expected an integer hour between {} and {}",
                Self::MIN,
                Self::MAX,
            ))
        }
    }
}

impl From<UtcOffsetHours> for i8 {
    fn from(value: UtcOffsetHours) -> Self {
        value.get()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeZone {
    #[default]
    Local,
    Client(ClientType),
    FixedOffset(UtcOffsetHours),
}

impl super::ConditionContext {
    pub(super) fn to_naive_datetime_in(&self, timezone: &TimeZone) -> NaiveDateTime {
        timezone.date_time(self)
    }
}

impl TimeZone {
    pub(crate) fn date_time(self, context: &super::ConditionContext) -> NaiveDateTime {
        let now = context.now;
        match self {
            Self::Local => now.with_timezone(&chrono::Local).naive_local(),
            Self::Client(client_type) => now
                .with_timezone(&hour_to_secs(client_type.server_time_zone()))
                .naive_local(),
            Self::FixedOffset(offset) => {
                now.with_timezone(&hour_to_secs(offset.get())).naive_local()
            }
        }
    }
}

const TIMEZONE_NAMES: [&str; ClientType::COUNT + 1] = {
    let mut variants = [""; ClientType::COUNT + 1];
    let (first, rest) = variants.split_first_mut().unwrap();
    *first = "Local";
    rest.copy_from_slice(&ClientType::NAMES);
    variants
};

impl<'de> Deserialize<'de> for TimeZone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawTimeOffset {
            String(String),
            TimeZone(i8),
        }

        use serde::de::Error;

        match RawTimeOffset::deserialize(deserializer)? {
            RawTimeOffset::String(value) if value == "Local" => Ok(Self::Local),
            RawTimeOffset::String(value) => value
                .parse::<ClientType>()
                .map(Self::Client)
                .map_err(|_| D::Error::unknown_variant(&value, &TIMEZONE_NAMES)),
            RawTimeOffset::TimeZone(value) => UtcOffsetHours::try_from(value)
                .map(Self::FixedOffset)
                .map_err(D::Error::custom),
        }
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for TimeZone {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "TimeOffset".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::TimeOffset").into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "oneOf": [
                {
                    "type": "string",
                    "enum": TIMEZONE_NAMES,
                },
                {
                    "type": "integer",
                    "minimum": -12,
                    "maximum": 14
                }
            ]
        })
    }
}

fn hour_to_secs(offset_in_hour: i8) -> FixedOffset {
    FixedOffset::east_opt(i32::from(offset_in_hour) * 3600)
        .expect("time offset within supported hour range")
}

pub trait TimeIn: Ord {
    fn before(&self, until: &Self) -> bool {
        self < until
    }

    fn not_after(&self, until: &Self) -> bool {
        self <= until
    }

    fn not_before(&self, from: &Self) -> bool {
        self >= from
    }

    fn is_in_range(&self, from: &Self, until: &Self) -> bool {
        self.not_before(from) && self.before(until)
    }
}

impl TimeIn for DateTime {}

impl TimeIn for NaiveDateTime {}

impl TimeIn for NaiveTime {
    // Cross-midnight time ranges use [from, until) semantics:
    // - When from <= until: standard [from, until) range
    // - When from > until (wraparound): [from, midnight) U [midnight, until)
    fn is_in_range(&self, from: &Self, until: &Self) -> bool {
        if from <= until {
            *from <= *self && *self < *until
        } else {
            *from <= *self || *self < *until
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TimeRange<T> {
    pub from: Option<T>,
    pub until: Option<T>,
}

trait RangeValue {
    fn empty_error() -> ValidationError;
}

impl RangeValue for NaiveTime {
    fn empty_error() -> ValidationError {
        ValidationError::EmptyTimeCondition
    }
}

impl RangeValue for NaiveDateTime {
    fn empty_error() -> ValidationError {
        ValidationError::EmptyDateTimeCondition
    }
}

impl<'de, T> Deserialize<'de> for TimeRange<T>
where
    T: Deserialize<'de> + RangeValue,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(bound(deserialize = "T: Deserialize<'de>"))]
        #[serde(deny_unknown_fields)]
        struct RawTimeRange<T> {
            #[serde(default)]
            from: Option<T>,
            #[serde(default)]
            until: Option<T>,
        }

        let range = RawTimeRange::<T>::deserialize(deserializer)?;
        if range.from.is_none() && range.until.is_none() {
            return Err(serde::de::Error::custom(T::empty_error()));
        }

        Ok(Self {
            from: range.from,
            until: range.until,
        })
    }
}

impl<T: TimeIn> TimeRange<T> {
    pub fn contains(&self, value: &T) -> bool {
        match (self.from.as_ref(), self.until.as_ref()) {
            (Some(from), Some(until)) => value.is_in_range(from, until),
            (Some(from), None) => value.not_before(from),
            (None, Some(until)) => value.not_after(until),
            (None, None) => true,
        }
    }
}
