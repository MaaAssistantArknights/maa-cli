use super::client_type::ClientType;

use crate::activity::has_side_story_open;

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Weekday};
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
    ///
    /// By default, use the weekday in user local time zone.
    /// If client is specified, use the weekday in the server time zone and start of the day will be
    /// 04:00:00 instead of 00:00:00 in server time zone, and the end of the day will be 03:59:59.
    Weekday {
        weekdays: Vec<Weekday>,
        client: Option<ClientType>,
    },
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
            Condition::Weekday {
                weekdays,
                ref client,
            } => {
                let weekday = if let Some(client) = client {
                    game_date(Local::now(), *client).weekday()
                } else {
                    Local::now().weekday()
                };
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

/// Get the date in the game server time
fn game_date<TZ: TimeZone>(now: DateTime<TZ>, client: ClientType) -> NaiveDate {
    let server_start_of_day = client.server_start_of_day();
    let server_time_zone = client.server_time_zone();
    let now = now.with_timezone(&server_time_zone);
    let date = now.date_naive();
    if now.time() < server_start_of_day {
        date.pred_opt().unwrap()
    } else {
        date
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
        use chrono::{Duration, FixedOffset};

        #[test]
        fn always() {
            assert!(Condition::Always.is_active());
        }

        #[test]
        fn weekday() {
            let now = chrono::Local::now();
            let weekday = now.date_naive().weekday();
            let cn_tz = FixedOffset::east_opt(8 * 3600).unwrap();
            let now_in_cn = now.with_timezone(&cn_tz);
            let weekday_in_cn = now_in_cn.weekday();
            let server_start_of_day = ClientType::Official.server_start_of_day();
            let should_be_prev_day = now_in_cn.time() < server_start_of_day;

            assert!(Condition::Weekday {
                weekdays: vec![weekday],
                client: None,
            }
            .is_active());
            assert!(!Condition::Weekday {
                weekdays: vec![weekday.pred(), weekday.succ()],
                client: None,
            }
            .is_active());

            assert_eq!(
                Condition::Weekday {
                    weekdays: vec![weekday_in_cn],
                    client: Some(ClientType::Official),
                }
                .is_active(),
                !should_be_prev_day
            );

            assert_eq!(
                Condition::Weekday {
                    weekdays: vec![weekday_in_cn.pred(), weekday_in_cn.succ()],
                    client: Some(ClientType::Official),
                }
                .is_active(),
                should_be_prev_day
            );
        }

        #[test]
        fn test_game_date() {
            fn datetime(tz: i32, y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<FixedOffset> {
                FixedOffset::east_opt(tz * 3600)
                    .unwrap()
                    .with_ymd_and_hms(y, m, d, h, mi, 0)
                    .unwrap()
            }

            fn naive_date(y: i32, m: u32, d: u32) -> NaiveDate {
                NaiveDate::from_ymd_opt(y, m, d).unwrap()
            }

            // local and server in the same time zone
            assert_eq!(
                game_date(datetime(8, 2024, 2, 14, 0, 0), ClientType::Official),
                naive_date(2024, 2, 13),
            );
            assert_eq!(
                game_date(datetime(8, 2024, 2, 14, 4, 1), ClientType::Official),
                naive_date(2024, 2, 14),
            );

            // local is late than server
            assert_eq!(
                game_date(datetime(-7, 2024, 2, 13, 10, 0), ClientType::YoStarJP),
                naive_date(2024, 2, 13),
            );
            assert_eq!(
                game_date(datetime(-7, 2024, 2, 13, 19, 0), ClientType::YoStarJP),
                naive_date(2024, 2, 14),
            );

            // local is early than server
            assert_eq!(
                game_date(datetime(8, 2024, 2, 14, 4, 0), ClientType::YoStarEN),
                naive_date(2024, 2, 13),
            );
            assert_eq!(
                game_date(datetime(8, 2024, 2, 14, 20, 0), ClientType::YoStarEN),
                naive_date(2024, 2, 14),
            );

            // This is a very unusual case that the local time zone is UTC+14 and server time zone is UTC-7
            // For local time 2024-02-15 00:00:00, the server time is 2024-02-14 03:00:00
            // Thus the game date is 2024-02-13 even though the local date is 2024-02-15
            assert_eq!(
                game_date(datetime(14, 2024, 2, 15, 0, 0), ClientType::YoStarEN),
                naive_date(2024, 2, 13),
            );
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
            assert_de_tokens(
                &Condition::Weekday {
                    weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
                    client: Some(ClientType::Official),
                },
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
                    Token::Str("client"),
                    Token::Some,
                    Token::Str("Official"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Condition::Weekday {
                    weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
                    client: None,
                },
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
