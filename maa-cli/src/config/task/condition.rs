use super::client_type::ClientType;

use crate::activity::has_side_story_open;

use chrono::{Datelike, Local, NaiveDateTime, NaiveTime, Weekday};
use serde::Deserialize;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize)]
#[serde(tag = "type")]
#[derive(Default)]
pub enum Condition {
    /// The task is always active
    #[default]
    Always,
    /// The task is active on the specified weekdays
    Weekday { weekdays: Vec<Weekday> },
    /// Day modula
    ///
    /// The task is active on `num_days % divisor == remainder`.
    /// The `num_days` is the number of days since the Common Era, (i.e. 0001-01-01 is 1).
    /// If `remainder` is not specified, it is 0.
    DayMod {
        divisor: u32,
        #[serde(default)]
        remainder: u32,
    },
    /// The task is active on the specified time range
    ///
    /// If `start` is `None`, the task is active before `end`.
    /// If `end` is `None`, the task is active after `start`.
    Time {
        #[serde(default, deserialize_with = "deserialize_from_str")]
        start: Option<NaiveTime>,
        #[serde(default, deserialize_with = "deserialize_from_str")]
        end: Option<NaiveTime>,
    },
    /// The task is active on the specified datetime range
    ///
    /// If `start` is `None`, the task is active before `end`.
    /// If `end` is `None`, the task is active after `start`.
    DateTime {
        #[serde(default, deserialize_with = "deserialize_from_str")]
        start: Option<NaiveDateTime>,
        #[serde(default, deserialize_with = "deserialize_from_str")]
        end: Option<NaiveDateTime>,
    },
    OnSideStory {
        #[serde(default)]
        client: ClientType,
    },
    /// The task is active if all the sub-conditions are met
    #[serde(alias = "Combined")]
    And { conditions: Vec<Condition> },
    /// The task is active if any of the sub-conditions is met
    Or { conditions: Vec<Condition> },
    /// The task is active if the inner condition is not met
    Not { condition: Box<Condition> },
}

fn deserialize_from_str<'de, S, D>(deserializer: D) -> Result<Option<S>, D::Error>
where
    S: std::str::FromStr,
    S::Err: std::fmt::Display,
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => match s.parse::<S>() {
            Ok(t) => Ok(Some(t)),
            Err(e) => Err(serde::de::Error::custom(format!("Invalid format: {}", e))),
        },
        None => Ok(None),
    }
}

impl Condition {
    pub fn is_active(&self) -> bool {
        match self {
            Condition::Always => true,
            Condition::Weekday { weekdays } => {
                let weekday = Local::now().weekday();
                weekdays.contains(&weekday)
            }
            Condition::DayMod { divisor, remainder } => {
                remainder_of_day_mod(*divisor) == *remainder
            }
            Condition::Time { start, end } => {
                let now_time = Local::now().time();
                match (start, end) {
                    (Some(s), Some(e)) => time_in_range(&now_time, s, e),
                    (Some(s), None) => now_time >= *s,
                    (None, Some(e)) => now_time < *e,
                    (None, None) => true,
                }
            }
            Condition::DateTime { start, end } => {
                let now = Local::now().naive_local();
                match (start, end) {
                    (Some(s), Some(e)) => now >= *s && now < *e,
                    (Some(s), None) => now >= *s,
                    (None, Some(e)) => now < *e,
                    (None, None) => true,
                }
            }
            Condition::OnSideStory { client } => has_side_story_open(*client),
            Condition::And { conditions } => {
                for condition in conditions {
                    if !condition.is_active() {
                        return false;
                    }
                }
                true
            }
            Condition::Or { conditions } => {
                for condition in conditions {
                    if condition.is_active() {
                        return true;
                    }
                }
                false
            }
            Condition::Not { condition } => !condition.is_active(),
        }
    }
}

fn time_in_range(time: &NaiveTime, start: &NaiveTime, end: &NaiveTime) -> bool {
    if start <= end {
        start <= time && time < end
    } else {
        start <= time || time < end
    }
}

pub fn remainder_of_day_mod(divisor: u32) -> u32 {
    let day = Local::now().date_naive().num_days_from_ce() as u32;
    day % divisor
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn naive_local_datetime(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        Local
            .with_ymd_and_hms(y, m, d, h, mi, s)
            .unwrap()
            .naive_local()
    }

    mod active {
        use super::*;
        use chrono::{Duration, NaiveDate};

        #[test]
        fn always() {
            assert!(Condition::Always.is_active());
        }

        #[test]
        fn weekday() {
            let now = chrono::Local::now();
            let weekday = now.date_naive().weekday();

            assert!(Condition::Weekday {
                weekdays: vec![weekday]
            }
            .is_active());
            assert!(!Condition::Weekday {
                weekdays: vec![weekday.pred(), weekday.succ()]
            }
            .is_active());
        }

        #[test]
        fn day_mod() {
            // We don't care about the correctness of the num_days_from_ce() method
            // We need to make sure it never changes during the update of chrono
            assert_eq!(
                NaiveDate::from_ymd_opt(1, 1, 1).unwrap().num_days_from_ce(),
                1
            );
            assert_eq!(
                NaiveDate::from_ymd_opt(1, 1, 2).unwrap().num_days_from_ce(),
                2
            );
            assert_eq!(
                NaiveDate::from_ymd_opt(2024, 1, 27)
                    .unwrap()
                    .num_days_from_ce(),
                738912
            );
            assert_eq!(
                NaiveDate::from_ymd_opt(2024, 2, 27)
                    .unwrap()
                    .num_days_from_ce(),
                738943
            );
            assert_eq!(
                NaiveDate::from_ymd_opt(2025, 1, 27)
                    .unwrap()
                    .num_days_from_ce(),
                739278
            );

            let num_days = Local::now().num_days_from_ce() as u32;

            assert!(Condition::DayMod {
                divisor: 1,
                remainder: 0,
            }
            .is_active());

            assert_eq!(
                Condition::DayMod {
                    divisor: 2,
                    remainder: 0,
                }
                .is_active(),
                num_days % 2 == 0
            );

            assert_eq!(
                Condition::DayMod {
                    divisor: 2,
                    remainder: 1,
                }
                .is_active(),
                num_days % 2 == 1
            );
        }

        #[test]
        fn time() {
            let now = chrono::Local::now();
            let now_time = now.time();

            assert!(Condition::Time {
                start: Some(now_time + Duration::seconds(-10)),
                end: Some(now_time + Duration::seconds(10)),
            }
            .is_active());
            assert!(Condition::Time {
                start: Some(now_time + Duration::seconds(-10)),
                end: None,
            }
            .is_active());
            assert!(Condition::Time {
                start: None,
                end: Some(now_time + Duration::seconds(10)),
            }
            .is_active());
            assert!(Condition::Time {
                start: None,
                end: None,
            }
            .is_active());
            assert!(!Condition::Time {
                start: Some(now_time + Duration::seconds(10)),
                end: Some(now_time + Duration::seconds(20)),
            }
            .is_active());
            assert!(!Condition::Time {
                start: Some(now_time + Duration::seconds(10)),
                end: None,
            }
            .is_active());
            assert!(!Condition::Time {
                start: None,
                end: Some(now_time + Duration::seconds(-10)),
            }
            .is_active());
        }

        #[test]
        fn test_time_in_range() {
            fn time_from_hms(h: u32, m: u32, s: u32) -> NaiveTime {
                NaiveTime::from_hms_opt(h, m, s).unwrap()
            }

            let start = time_from_hms(1, 0, 0);
            let end = time_from_hms(2, 59, 59);

            assert!(time_in_range(&time_from_hms(1, 0, 0), &start, &end));
            assert!(time_in_range(&time_from_hms(1, 0, 1), &start, &end));
            assert!(time_in_range(&time_from_hms(2, 59, 58), &start, &end));
            assert!(!time_in_range(&time_from_hms(0, 59, 59), &start, &end));
            assert!(!time_in_range(&time_from_hms(2, 59, 59), &start, &end));

            let start = time_from_hms(23, 0, 0);
            let end = time_from_hms(1, 59, 59);

            assert!(time_in_range(&time_from_hms(23, 0, 0), &start, &end));
            assert!(time_in_range(&time_from_hms(23, 0, 1), &start, &end));
            assert!(time_in_range(&time_from_hms(1, 59, 58), &start, &end));
            assert!(!time_in_range(&time_from_hms(22, 59, 59), &start, &end));
            assert!(!time_in_range(&time_from_hms(1, 59, 59), &start, &end));
        }

        #[test]
        fn datetime() {
            let now = chrono::Local::now();
            let now_datetime = now.naive_local();

            assert!(Condition::DateTime {
                start: Some(now_datetime + Duration::minutes(-10)),
                end: Some(now_datetime + Duration::minutes(10)),
            }
            .is_active());
            assert!(Condition::DateTime {
                start: Some(now_datetime + Duration::minutes(-10)),
                end: None,
            }
            .is_active());
            assert!(Condition::DateTime {
                start: None,
                end: Some(now_datetime + Duration::minutes(10)),
            }
            .is_active());
            assert!(Condition::DateTime {
                start: None,
                end: None,
            }
            .is_active());
            assert!(!Condition::DateTime {
                start: Some(now_datetime + Duration::minutes(10)),
                end: Some(now_datetime + Duration::minutes(20)),
            }
            .is_active());
            assert!(!Condition::DateTime {
                start: Some(now_datetime + Duration::minutes(10)),
                end: None,
            }
            .is_active());
            assert!(!Condition::DateTime {
                start: None,
                end: Some(now_datetime + Duration::minutes(-10)),
            }
            .is_active());
        }

        // It's hart to test OnSideStory, because it depends on real world data
        // #[test]
        // fn on_side_story() {}

        #[test]
        fn boolean() {
            assert!(Condition::And {
                conditions: vec![Condition::Always, Condition::Always]
            }
            .is_active());
            assert!(!Condition::And {
                conditions: vec![
                    Condition::Always,
                    Condition::Not {
                        condition: Box::new(Condition::Always)
                    },
                ]
            }
            .is_active());

            assert!(Condition::Or {
                conditions: vec![
                    Condition::Always,
                    Condition::Not {
                        condition: Box::new(Condition::Always)
                    }
                ]
            }
            .is_active());

            assert!(!Condition::Or {
                conditions: vec![
                    Condition::Not {
                        condition: Box::new(Condition::Always)
                    },
                    Condition::Not {
                        condition: Box::new(Condition::Always)
                    }
                ]
            }
            .is_active());

            assert!(!Condition::Not {
                condition: Box::new(Condition::Always)
            }
            .is_active());
        }
    }

    mod serde {
        use super::*;
        use serde_test::{assert_de_tokens, Token};

        #[test]
        fn weekday() {
            let cond = Condition::Weekday {
                weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
            };

            assert_de_tokens(
                &cond,
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("Weekday"),
                    Token::Str("weekdays"),
                    Token::Seq { len: Some(3) },
                    Token::Str("Mon"),
                    Token::Str("Wed"),
                    Token::Str("Fri"),
                    Token::SeqEnd,
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &cond,
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("Weekday"),
                    Token::Str("weekdays"),
                    Token::Seq { len: Some(3) },
                    Token::Str("Monday"),
                    Token::Str("Wednesday"),
                    Token::Str("Friday"),
                    Token::SeqEnd,
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn day_mod() {
            let cond = Condition::DayMod {
                divisor: 7,
                remainder: 0,
            };

            assert_de_tokens(
                &cond,
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("type"),
                    Token::Str("DayMod"),
                    Token::Str("divisor"),
                    Token::U32(7),
                    Token::Str("remainder"),
                    Token::U32(0),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &cond,
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("DayMod"),
                    Token::Str("divisor"),
                    Token::U32(7),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn time() {
            let cond = Condition::Time {
                start: Some(NaiveTime::from_hms_opt(1, 0, 0).unwrap()),
                end: Some(NaiveTime::from_hms_opt(2, 59, 59).unwrap()),
            };

            assert_de_tokens(
                &cond,
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("type"),
                    Token::Str("Time"),
                    Token::Str("start"),
                    Token::Str("01:00:00"),
                    Token::Str("end"),
                    Token::Str("02:59:59"),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn datetime() {
            assert_de_tokens(
                &Condition::DateTime {
                    start: Some(naive_local_datetime(2021, 8, 1, 16, 0, 0)),
                    end: Some(naive_local_datetime(2021, 8, 21, 4, 0, 0)),
                },
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("type"),
                    Token::Str("DateTime"),
                    Token::Str("start"),
                    Token::Str("2021-08-01T16:00:00"),
                    Token::Str("end"),
                    Token::Str("2021-08-21T04:00:00"),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn on_side_story() {
            assert_de_tokens(
                &Condition::OnSideStory {
                    client: ClientType::Official,
                },
                &[
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("OnSideStory"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Condition::OnSideStory {
                    client: ClientType::Txwy,
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("OnSideStory"),
                    Token::Str("client"),
                    Token::Str("txwy"),
                    Token::MapEnd,
                ],
            );
        }

        #[test]
        fn boolean() {
            assert_de_tokens(
                &Condition::And {
                    conditions: vec![Condition::Always, Condition::Always],
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("Combined"),
                    Token::Str("conditions"),
                    Token::Seq { len: Some(2) },
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("Always"),
                    Token::MapEnd,
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("Always"),
                    Token::MapEnd,
                    Token::SeqEnd,
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Condition::And {
                    conditions: vec![Condition::Always, Condition::Always],
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("And"),
                    Token::Str("conditions"),
                    Token::Seq { len: Some(2) },
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("Always"),
                    Token::MapEnd,
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("Always"),
                    Token::MapEnd,
                    Token::SeqEnd,
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Condition::Or {
                    conditions: vec![Condition::Always, Condition::Always],
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("Or"),
                    Token::Str("conditions"),
                    Token::Seq { len: Some(2) },
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("Always"),
                    Token::MapEnd,
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("Always"),
                    Token::MapEnd,
                    Token::SeqEnd,
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Condition::Not {
                    condition: Box::new(Condition::Always),
                },
                &[
                    Token::Map { len: Some(2) },
                    Token::Str("type"),
                    Token::Str("Not"),
                    Token::Str("condition"),
                    Token::Map { len: Some(1) },
                    Token::Str("type"),
                    Token::Str("Always"),
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            );
        }
    }
}
