use chrono::{DateTime, Datelike, NaiveDateTime, NaiveTime, TimeZone, Utc, Weekday};
use serde::Deserialize;

use super::client_type::ClientType;
use crate::activity::has_side_story_open;

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
    /// If client is specified, use the weekday in the server time zone and start of the day will
    /// be 04:00:00 instead of 00:00:00 in server time zone, and the end of the day will be
    /// 03:59:59.
    Weekday {
        weekdays: Vec<Weekday>,
        #[serde(default, alias = "client")]
        timezone: TimeOffset,
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
        #[serde(default)]
        timezone: TimeOffset,
    },
    /// The task is active on the specified time range
    ///
    /// If `start` is `None`, the task is active before `end`.
    /// If `end` is `None`, the task is active after `start`.
    Time {
        #[serde(default)]
        start: Option<NaiveTime>,
        #[serde(default)]
        end: Option<NaiveTime>,
        #[serde(default)]
        timezone: TimeOffset,
    },
    /// The task is active on the specified datetime range
    ///
    /// If `start` is `None`, the task is active before `end`.
    /// If `end` is `None`, the task is active after `start`.
    DateTime {
        #[serde(default)]
        start: Option<NaiveDateTime>,
        #[serde(default)]
        end: Option<NaiveDateTime>,
        #[serde(default)]
        timezone: TimeOffset,
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

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(untagged)]
pub enum TimeOffset {
    Client(ClientType),
    TimeZone(i8),
    #[default]
    Local,
}

impl TimeOffset {
    /// Get the current date time in given time zone
    fn naive_now(self) -> NaiveDateTime {
        self.date_time(Utc::now())
    }

    /// Get the naive date time of the given date time in the given time zone
    fn date_time<TZ: TimeZone>(self, datetime: DateTime<TZ>) -> NaiveDateTime {
        use TimeOffset::*;
        match self {
            TimeZone(tz) => datetime.with_timezone(&tz_to_offset(tz)).naive_local(),
            Client(client) => datetime
                .with_timezone(&tz_to_offset(client.server_time_zone()))
                .naive_local(),
            Local => datetime.with_timezone(&chrono::Local).naive_local(),
        }
    }
}

fn tz_to_offset(tz: i8) -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(tz as i32 * 3600).unwrap()
}

impl Condition {
    pub fn is_active(&self) -> bool {
        use Condition::*;
        match *self {
            Always => true,
            Weekday {
                ref weekdays,
                timezone,
            } => weekdays.contains(&timezone.naive_now().weekday()),
            DayMod {
                divisor,
                remainder,
                timezone,
            } => remainder_of_day_mod(timezone, divisor) == remainder,
            Time {
                start,
                end,
                timezone,
            } => {
                let now_time = timezone.naive_now().time();

                match (start, end) {
                    (Some(s), Some(e)) => time_in_range(now_time, s, e),
                    (Some(s), None) => now_time >= s,
                    (None, Some(e)) => now_time < e,
                    (None, None) => true,
                }
            }
            DateTime {
                start,
                end,
                timezone,
            } => {
                let now = timezone.naive_now();
                match (start, end) {
                    (Some(s), Some(e)) => now >= s && now < e,
                    (Some(s), None) => now >= s,
                    (None, Some(e)) => now < e,
                    (None, None) => true,
                }
            }
            OnSideStory { client } => has_side_story_open(client),
            And { ref conditions } => {
                for condition in conditions {
                    if !condition.is_active() {
                        return false;
                    }
                }
                true
            }
            Or { ref conditions } => {
                for condition in conditions {
                    if condition.is_active() {
                        return true;
                    }
                }
                false
            }
            Not { ref condition } => !condition.is_active(),
        }
    }
}

fn time_in_range(time: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start <= end {
        start <= time && time < end
    } else {
        start <= time || time < end
    }
}

pub fn remainder_of_day_mod(tz: TimeOffset, divisor: u32) -> u32 {
    tz.naive_now().num_days_from_ce() as u32 % divisor
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    fn naive_local_datetime(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        Local
            .with_ymd_and_hms(y, m, d, h, mi, s)
            .unwrap()
            .naive_local()
    }

    mod active {
        use chrono::{Duration, FixedOffset, NaiveDate};

        use super::*;

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
            let server_start_of_day = NaiveTime::from_hms_opt(4, 0, 0).unwrap();
            let should_be_prev_day = now_in_cn.time() < server_start_of_day;

            use TimeOffset::*;
            assert!(
                Condition::Weekday {
                    weekdays: vec![weekday],
                    timezone: Local
                }
                .is_active()
            );
            assert!(
                !Condition::Weekday {
                    weekdays: vec![weekday.pred(), weekday.succ()],
                    timezone: Local,
                }
                .is_active()
            );

            assert_eq!(
                Condition::Weekday {
                    weekdays: vec![weekday_in_cn],
                    timezone: Client(ClientType::Official),
                }
                .is_active(),
                !should_be_prev_day
            );

            assert_eq!(
                Condition::Weekday {
                    weekdays: vec![weekday_in_cn.pred(), weekday_in_cn.succ()],
                    timezone: Client(ClientType::Official),
                }
                .is_active(),
                should_be_prev_day
            );
        }

        #[test]
        fn time_offset() {
            fn datetime(tz: i32, y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<FixedOffset> {
                FixedOffset::east_opt(tz * 3600)
                    .unwrap()
                    .with_ymd_and_hms(y, m, d, h, mi, 0)
                    .unwrap()
            }

            fn naive_date(y: i32, m: u32, d: u32) -> NaiveDate {
                NaiveDate::from_ymd_opt(y, m, d).unwrap()
            }

            assert_eq!(
                TimeOffset::Client(ClientType::Official).date_time(datetime(8, 2024, 2, 14, 4, 0)),
                naive_local_datetime(2024, 2, 14, 0, 0, 0),
            );

            assert_eq!(
                TimeOffset::Client(ClientType::YoStarJP).date_time(datetime(8, 2024, 2, 14, 4, 0)),
                naive_local_datetime(2024, 2, 14, 1, 0, 0),
            );

            assert_eq!(
                TimeOffset::TimeZone(8).date_time(datetime(8, 2024, 2, 14, 4, 0)),
                naive_local_datetime(2024, 2, 14, 4, 0, 0),
            );

            assert_eq!(
                TimeOffset::TimeZone(0).date_time(datetime(8, 2024, 2, 14, 4, 0)),
                naive_local_datetime(2024, 2, 13, 20, 0, 0),
            );

            assert_eq!(
                TimeOffset::TimeZone(-7).date_time(datetime(8, 2024, 2, 14, 4, 0)),
                naive_local_datetime(2024, 2, 13, 13, 0, 0),
            );

            // Campat test for old implementation of Weekday
            fn game_date(datetime: DateTime<FixedOffset>, client: ClientType) -> NaiveDate {
                TimeOffset::Client(client).date_time(datetime).date()
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

            // This is a very unusual case that the local time zone is UTC+14 and server time zone
            // is UTC-7 For local time 2024-02-15 00:00:00, the server time is
            // 2024-02-14 03:00:00 Thus the game date is 2024-02-13 even though the
            // local date is 2024-02-15
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

            assert!(
                Condition::DayMod {
                    divisor: 1,
                    remainder: 0,
                    timezone: TimeOffset::Local,
                }
                .is_active()
            );

            assert_eq!(
                Condition::DayMod {
                    divisor: 2,
                    remainder: 0,
                    timezone: TimeOffset::Local
                }
                .is_active(),
                num_days.is_multiple_of(2)
            );

            assert_eq!(
                Condition::DayMod {
                    divisor: 2,
                    remainder: 1,
                    timezone: TimeOffset::Local
                }
                .is_active(),
                num_days % 2 == 1
            );
        }

        fn seconds(s: i64) -> Duration {
            chrono::TimeDelta::try_seconds(s).unwrap()
        }

        #[test]
        fn time() {
            let now = chrono::Local::now();
            let now_time = now.time();

            assert!(
                Condition::Time {
                    start: Some(now_time + seconds(-10)),
                    end: Some(now_time + seconds(10)),
                    timezone: TimeOffset::Local,
                }
                .is_active()
            );
            assert!(
                Condition::Time {
                    start: Some(now_time + seconds(-10)),
                    end: None,
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                Condition::Time {
                    start: None,
                    end: Some(now_time + seconds(10)),
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                Condition::Time {
                    start: None,
                    end: None,
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                !Condition::Time {
                    start: Some(now_time + seconds(10)),
                    end: Some(now_time + seconds(20)),
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                !Condition::Time {
                    start: Some(now_time + seconds(10)),
                    end: None,
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                !Condition::Time {
                    start: None,
                    end: Some(now_time + seconds(-10)),
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
        }

        #[test]
        fn test_time_in_range() {
            fn time_from_hms(h: u32, m: u32, s: u32) -> NaiveTime {
                NaiveTime::from_hms_opt(h, m, s).unwrap()
            }

            let start = time_from_hms(1, 0, 0);
            let end = time_from_hms(2, 59, 59);

            assert!(time_in_range(time_from_hms(1, 0, 0), start, end));
            assert!(time_in_range(time_from_hms(1, 0, 1), start, end));
            assert!(time_in_range(time_from_hms(2, 59, 58), start, end));
            assert!(!time_in_range(time_from_hms(0, 59, 59), start, end));
            assert!(!time_in_range(time_from_hms(2, 59, 59), start, end));

            let start = time_from_hms(23, 0, 0);
            let end = time_from_hms(1, 59, 59);

            assert!(time_in_range(time_from_hms(23, 0, 0), start, end));
            assert!(time_in_range(time_from_hms(23, 0, 1), start, end));
            assert!(time_in_range(time_from_hms(1, 59, 58), start, end));
            assert!(!time_in_range(time_from_hms(22, 59, 59), start, end));
            assert!(!time_in_range(time_from_hms(1, 59, 59), start, end));
        }

        #[test]
        fn datetime() {
            let now = chrono::Local::now();
            let now_datetime = now.naive_local();

            assert!(
                Condition::DateTime {
                    start: Some(now_datetime + seconds(-10)),
                    end: Some(now_datetime + seconds(10)),
                    timezone: TimeOffset::Local,
                }
                .is_active()
            );
            assert!(
                Condition::DateTime {
                    start: Some(now_datetime + seconds(-10)),
                    end: None,
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                Condition::DateTime {
                    start: None,
                    end: Some(now_datetime + seconds(10)),
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                Condition::DateTime {
                    start: None,
                    end: None,
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                !Condition::DateTime {
                    start: Some(now_datetime + seconds(10)),
                    end: Some(now_datetime + seconds(20)),
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                !Condition::DateTime {
                    start: Some(now_datetime + seconds(10)),
                    end: None,
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
            assert!(
                !Condition::DateTime {
                    start: None,
                    end: Some(now_datetime + seconds(-10)),
                    timezone: TimeOffset::Local
                }
                .is_active()
            );
        }

        // It's hart to test OnSideStory, because it depends on real world data
        // #[test]
        // fn on_side_story() {}

        #[test]
        fn boolean() {
            assert!(
                Condition::And {
                    conditions: vec![Condition::Always, Condition::Always]
                }
                .is_active()
            );
            assert!(
                !Condition::And {
                    conditions: vec![Condition::Always, Condition::Not {
                        condition: Box::new(Condition::Always)
                    },]
                }
                .is_active()
            );

            assert!(
                Condition::Or {
                    conditions: vec![Condition::Always, Condition::Not {
                        condition: Box::new(Condition::Always)
                    }]
                }
                .is_active()
            );

            assert!(
                !Condition::Or {
                    conditions: vec![
                        Condition::Not {
                            condition: Box::new(Condition::Always)
                        },
                        Condition::Not {
                            condition: Box::new(Condition::Always)
                        }
                    ]
                }
                .is_active()
            );

            assert!(
                !Condition::Not {
                    condition: Box::new(Condition::Always)
                }
                .is_active()
            );
        }
    }

    mod serde {
        use serde_test::{Token, assert_de_tokens};

        use super::*;

        #[test]
        fn weekday() {
            assert_de_tokens(
                &Condition::Weekday {
                    weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
                    timezone: TimeOffset::Client(ClientType::Official),
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
                    Token::Str("Official"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Condition::Weekday {
                    weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
                    timezone: TimeOffset::Client(ClientType::Official),
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
                    Token::Str("timezone"),
                    Token::Str("Official"),
                    Token::MapEnd,
                ],
            );

            assert_de_tokens(
                &Condition::Weekday {
                    weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
                    timezone: TimeOffset::Local,
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
                timezone: TimeOffset::Local,
            };

            assert_de_tokens(&cond, &[
                Token::Map { len: Some(3) },
                Token::Str("type"),
                Token::Str("DayMod"),
                Token::Str("divisor"),
                Token::U32(7),
                Token::Str("remainder"),
                Token::U32(0),
                Token::MapEnd,
            ]);

            assert_de_tokens(&cond, &[
                Token::Map { len: Some(2) },
                Token::Str("type"),
                Token::Str("DayMod"),
                Token::Str("divisor"),
                Token::U32(7),
                Token::MapEnd,
            ]);
        }

        #[test]
        fn time() {
            assert_de_tokens(
                &Condition::Time {
                    start: Some(NaiveTime::from_hms_opt(1, 0, 0).unwrap()),
                    end: Some(NaiveTime::from_hms_opt(2, 59, 59).unwrap()),
                    timezone: TimeOffset::Local,
                },
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

            assert_de_tokens(
                &Condition::Time {
                    start: Some(NaiveTime::from_hms_opt(1, 0, 0).unwrap()),
                    end: None,
                    timezone: TimeOffset::Client(ClientType::Official),
                },
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("type"),
                    Token::Str("Time"),
                    Token::Str("start"),
                    Token::Str("01:00:00"),
                    Token::Str("timezone"),
                    Token::Str("Official"),
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
                    timezone: TimeOffset::Local,
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

            assert_de_tokens(
                &Condition::DateTime {
                    start: None,
                    end: Some(naive_local_datetime(2021, 8, 21, 4, 0, 0)),
                    timezone: TimeOffset::TimeZone(8),
                },
                &[
                    Token::Map { len: Some(3) },
                    Token::Str("type"),
                    Token::Str("DateTime"),
                    Token::Str("end"),
                    Token::Str("2021-08-21T04:00:00"),
                    Token::Str("timezone"),
                    Token::I8(8),
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
