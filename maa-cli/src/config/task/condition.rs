use chrono::{Datelike, Local, NaiveDateTime, NaiveTime, Weekday};
use serde::Deserialize;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[derive(Default)]
pub enum Condition {
    #[default]
    Always,
    Weekday {
        weekdays: Vec<Weekday>,
    },
    Time {
        #[serde(default, deserialize_with = "deserialize_from_str")]
        start: Option<NaiveTime>,
        #[serde(default, deserialize_with = "deserialize_from_str")]
        end: Option<NaiveTime>,
    },
    DateTime {
        #[serde(default, deserialize_with = "deserialize_from_str")]
        start: Option<NaiveDateTime>,
        #[serde(default, deserialize_with = "deserialize_from_str")]
        end: Option<NaiveDateTime>,
    },
    Combined {
        conditions: Vec<Condition>,
    },
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
                let now = Local::now();
                let weekday = now.date_naive().weekday();
                weekdays.contains(&weekday)
            }
            Condition::Time { start, end } => {
                let now = Local::now();
                let now_time = now.time();
                match (start, end) {
                    (Some(s), Some(e)) => now_time >= *s && now_time <= *e,
                    (Some(s), None) => now_time >= *s,
                    (None, Some(e)) => now_time <= *e,
                    (None, None) => true,
                }
            }
            Condition::DateTime { start, end } => {
                let now = Local::now().naive_local();
                match (start, end) {
                    (Some(s), Some(e)) => now >= *s && now <= *e,
                    (Some(s), None) => now >= *s,
                    (None, Some(e)) => now <= *e,
                    (None, None) => true,
                }
            }
            Condition::Combined { conditions } => {
                for condition in conditions {
                    if !condition.is_active() {
                        return false;
                    }
                }
                true
            }
        }
    }
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
        use chrono::Duration;

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
        fn time() {
            let now = chrono::Local::now();
            let now_time = now.time();

            assert!(Condition::Time {
                start: Some(now_time + Duration::minutes(-10)),
                end: Some(now_time + Duration::minutes(10)),
            }
            .is_active());
            assert!(!Condition::Time {
                start: Some(now_time + Duration::minutes(10)),
                end: Some(now_time + Duration::minutes(20)),
            }
            .is_active());
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
            assert!(!Condition::DateTime {
                start: Some(now_datetime + Duration::minutes(10)),
                end: Some(now_datetime + Duration::minutes(20)),
            }
            .is_active());
        }

        #[test]
        fn combined() {
            let now = chrono::Local::now();
            let now_time = now.time();
            let weekday = now.date_naive().weekday();

            assert!(Condition::Combined {
                conditions: vec![
                    Condition::Time {
                        start: Some(now_time + Duration::minutes(-10)),
                        end: Some(now_time + Duration::minutes(10)),
                    },
                    Condition::Weekday {
                        weekdays: vec![weekday]
                    },
                ]
            }
            .is_active());
            assert!(!Condition::Combined {
                conditions: vec![
                    Condition::Time {
                        start: Some(now_time + Duration::minutes(10)),
                        end: Some(now_time + Duration::minutes(20)),
                    },
                    Condition::Weekday {
                        weekdays: vec![weekday]
                    },
                ]
            }
            .is_active());
        }
    }

    mod serde {
        use super::*;
        use serde_test::{assert_de_tokens, Token};

        #[test]
        fn weakday() {
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
        fn datatime() {
            let cond_str = r#"{
                "type": "DateTime",
                "start": "2021-08-01T16:00:00",
                "end": "2021-08-21T04:00:00"
            }"#;
            let cond: Condition = serde_json::from_str(cond_str).unwrap();
            assert_eq!(
                cond,
                Condition::DateTime {
                    start: Some(naive_local_datetime(2021, 8, 1, 16, 0, 0)),
                    end: Some(naive_local_datetime(2021, 8, 21, 4, 0, 0)),
                }
            );
        }
    }
}
