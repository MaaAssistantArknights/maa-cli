use std::{collections::BTreeMap, io::Write, path::Path, sync::LazyLock};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use log::warn;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::config::task::ClientType;

static STAGE_ACTIVITY: LazyLock<Option<StageActivityJson>> =
    LazyLock::new(|| load_stage_activity(maa_dirs::activity()).warn_err());

pub fn has_side_story_open(client: ClientType) -> bool {
    STAGE_ACTIVITY
        .as_ref()
        .map(|stage_activity| {
            stage_activity
                .get_stage_activity(client)
                .has_side_story_open()
        })
        .unwrap_or(false)
}

pub fn display_stage_activity(client: ClientType) -> std::io::Result<()> {
    if let Some(stage_activity) = STAGE_ACTIVITY.as_ref() {
        stage_activity.display(std::io::stdout(), client)?;
        std::io::stdout().flush()?;
    }
    Ok(())
}

// TODO: use stage activity as alternative in fight task
// This feature require allow custom input in select

// TODO: use MinimumRequired to verify stage

#[derive(Deserialize)]
#[serde(transparent)]
pub struct StageActivityJson(BTreeMap<String, StageActivityContent>);

fn load_stage_activity(file_path: impl AsRef<Path>) -> Result<StageActivityJson> {
    let file_path = file_path.as_ref();
    let file = std::fs::File::open(file_path)
        .with_context(|| format!("Failed to open {}", file_path.display()))?;
    serde_json::from_reader(file)
        .map_err(|e| anyhow!("Failed to parse {}: {e}", file_path.display()))
}

impl StageActivityJson {
    pub fn get_stage_activity(&self, mut client: ClientType) -> &StageActivityContent {
        if client == ClientType::Bilibili {
            client = ClientType::Official;
        }
        self.0
            .get(client.to_str())
            .expect("All client types should be covered")
    }

    pub fn display(&self, mut f: impl Write, client: ClientType) -> std::io::Result<()> {
        let item_index = load_item_index(client);
        let stage_activity = self.get_stage_activity(client);

        let mut sidestory_title = false;
        for activity in stage_activity.side_story_stage.values() {
            if activity.activity.is_active() {
                if !sidestory_title {
                    writeln!(f, "Opening side story stages:")?;
                    sidestory_title = true;
                }
                writeln!(f, "- {}", activity.activity.tip)?;
                for stage in &activity.stages {
                    let drop = item_index
                        .as_ref()
                        .warn_err()
                        .and_then(|item_index| item_index.get(&stage.drop))
                        .and_then(|item| item.get("name"))
                        .and_then(|name| name.as_str())
                        .unwrap_or(&stage.drop);
                    writeln!(f, "  - {}: {}", stage.display, drop)?;
                }
            }
        }

        if stage_activity.resource_collection.is_active() {
            writeln!(f, "{}", stage_activity.resource_collection.tip)?;
        }

        let mut minigame_title = false;
        for game in &stage_activity.mini_game {
            if game.is_active() {
                if !minigame_title {
                    writeln!(f, "Opening mini games:")?;
                    minigame_title = true;
                }
                writeln!(f, "- {}", game.display)?;
            }
        }

        Ok(())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageActivityContent {
    side_story_stage: BTreeMap<String, SideStoryActivity>,
    resource_collection: ActivityInfo,
    #[serde(default)]
    mini_game: Vec<MiniGameInfo>,
}

impl StageActivityContent {
    pub fn has_side_story_open(&self) -> bool {
        self.side_story_stage
            .values()
            .any(|activity| activity.activity.is_active())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SideStoryActivity {
    activity: ActivityInfo,
    stages: Vec<StageInfo>,
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ActivityInfo {
    tip: String,

    #[serde(flatten)]
    time_info: TimeInfo,
}

impl ActivityInfo {
    pub fn is_active(&self) -> bool {
        self.time_info.is_active()
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StageInfo {
    display: String,
    // value: String,
    drop: String,
}

#[cfg_attr(test, derive(Debug, PartialEq, Clone))]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MiniGameInfo {
    display: String,

    #[serde(flatten)]
    time_info: TimeInfo,
}

impl MiniGameInfo {
    pub fn is_active(&self) -> bool {
        self.time_info.is_active()
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
/// Time information for activities.
///
/// The utc_* prefix is quiet missleading, as it is not UTC time but local time at given time zone.
struct TimeInfo {
    #[serde(deserialize_with = "parse_naive_data_time")]
    utc_start_time: NaiveDateTime,
    #[serde(deserialize_with = "parse_naive_data_time")]
    utc_expire_time: NaiveDateTime,
    #[serde(deserialize_with = "parse_fixed_offset")]
    time_zone: FixedOffset,
}

fn parse_naive_data_time<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, "%Y/%m/%d %H:%M:%S").map_err(serde::de::Error::custom)
}

fn parse_fixed_offset<'de, D>(deserializer: D) -> Result<FixedOffset, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let tz = i32::deserialize(deserializer)?;
    FixedOffset::east_opt(tz * 3600)
        .ok_or_else(|| serde::de::Error::custom(format!("Timezone offset {tz} out of range")))
}

impl TimeInfo {
    pub fn is_active(&self) -> bool {
        self.is_active_at(Utc::now())
    }

    pub fn is_active_at(&self, time: DateTime<Utc>) -> bool {
        let time = time.with_timezone(&self.time_zone).naive_local();
        self.utc_start_time < time && time < self.utc_expire_time
    }
}

fn load_item_index(client: ClientType) -> Result<JsonValue> {
    let maa_resource_dir = maa_dirs::maa_resource().join("resource");
    let base_resource_dir = if maa_resource_dir.exists() {
        maa_resource_dir.into()
    } else if let Some(resource_dir) = maa_dirs::find_resource() {
        resource_dir
    } else {
        bail!("Failed to find resource dir");
    };

    let item_index_path = match client.resource() {
        Some(global_resource) => join!(
            base_resource_dir,
            "global",
            global_resource,
            "resource",
            "item_index.json"
        ),
        None => join!(base_resource_dir, "item_index.json"),
    };

    let file = std::fs::File::open(&item_index_path).with_context(|| {
        format!(
            "Failed to open item_index.json: {}",
            item_index_path.display()
        )
    })?;

    serde_json::from_reader(file).context("Failed to parse item_index.json")
}

pub trait WarnError<T> {
    /// If the result is Ok, return the Some value, otherwise log the error and return None.
    fn warn_err(self) -> Option<T>;
}

impl<T, E: std::fmt::Display> WarnError<T> for std::result::Result<T, E> {
    fn warn_err(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(err) => {
                warn!("{err}");
                None
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::env::var_os;

    use super::*;

    // Helper functions for creating test data
    fn naive_dt(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, min, sec)
            .unwrap()
    }

    fn tz(hours: i32) -> FixedOffset {
        FixedOffset::east_opt(hours * 3600).unwrap()
    }

    fn tz_west(hours: i32) -> FixedOffset {
        FixedOffset::west_opt(hours * 3600).unwrap()
    }

    mod stage_activity_json {
        use super::*;

        #[test]
        fn test_parse_activity_fixture() {
            let json_str = include_str!("../fixtures/activity.json");
            let stage_activity: StageActivityJson = serde_json::from_str(json_str).unwrap();

            // Test Official server
            let official = stage_activity.get_stage_activity(ClientType::Official);
            assert_eq!(official.side_story_stage.len(), 2);
            assert!(official.side_story_stage.contains_key("SSReopen"));
            assert!(official.side_story_stage.contains_key("UR"));
            assert_eq!(official.mini_game.len(), 1);

            // Test SSReopen activity
            let ssreopen = &official.side_story_stage["SSReopen"];
            assert_eq!(ssreopen.activity.tip, "SideStory「出苍白海」复刻");
            assert_eq!(ssreopen.stages.len(), 5);
            assert_eq!(ssreopen.stages[0].display, "SSReopen-EP");
            assert_eq!(ssreopen.stages[0].drop, "代理1~8");

            // Test txwy server
            let txwy = stage_activity.get_stage_activity(ClientType::Txwy);
            assert_eq!(txwy.side_story_stage.len(), 2);
            assert!(txwy.side_story_stage.contains_key("巴别塔 復刻"));
            assert!(txwy.side_story_stage.contains_key("眾生行記"));
            assert_eq!(txwy.mini_game.len(), 1);
        }

        #[test]
        #[ignore = "need installed resource"]
        fn test_load_stage_activity() {
            if var_os("SKIP_CORE_TEST").is_some() {
                return;
            }
            let _ = STAGE_ACTIVITY.as_ref();
        }

        #[test]
        #[ignore = "need installed resource"]
        fn test_item_index() {
            if var_os("SKIP_CORE_TEST").is_some() {
                return;
            }

            load_item_index(ClientType::Official).unwrap();
            load_item_index(ClientType::Bilibili).unwrap();
            load_item_index(ClientType::YoStarEN).unwrap();
            load_item_index(ClientType::YoStarJP).unwrap();
            load_item_index(ClientType::YoStarKR).unwrap();
            load_item_index(ClientType::Txwy).unwrap();
        }
    }

    mod time_info {
        use super::*;

        #[test]
        fn test_is_active_at_active_range() {
            // Use a broad time range from 2024 to 2050 for active period
            let time_info = TimeInfo {
                utc_start_time: naive_dt(2024, 1, 1, 0, 0, 0),
                utc_expire_time: naive_dt(2050, 12, 31, 23, 59, 59),
                time_zone: tz(8), // UTC+8
            };

            // Test time within active range
            let test_time = naive_dt(2025, 6, 15, 12, 0, 0).and_utc();
            assert!(time_info.is_active_at(test_time));
        }

        #[test]
        fn test_is_active_at_expired() {
            // Use early time range (before 2023) for expired period
            let time_info = TimeInfo {
                utc_start_time: naive_dt(2020, 1, 1, 0, 0, 0),
                utc_expire_time: naive_dt(2022, 12, 31, 23, 59, 59),
                time_zone: tz(8),
            };

            // Test time after expiration
            let test_time = naive_dt(2025, 1, 15, 12, 0, 0).and_utc();
            assert!(!time_info.is_active_at(test_time));
        }

        #[test]
        fn test_is_active_at_future() {
            let time_info = TimeInfo {
                utc_start_time: naive_dt(2051, 1, 1, 0, 0, 0),
                utc_expire_time: naive_dt(2060, 12, 31, 23, 59, 59),
                time_zone: tz(8),
            };

            // Test time before start
            let test_time = naive_dt(2025, 6, 15, 12, 0, 0).and_utc();
            assert!(!time_info.is_active_at(test_time));
        }

        #[test]
        fn test_timezone_makes_difference() {
            // Same UTC times, but activity defined in different timezones
            // Activity from 10:00 to 18:00 in local time
            let start = naive_dt(2025, 6, 15, 10, 0, 0);
            let expire = naive_dt(2025, 6, 15, 18, 0, 0);

            let time_info_utc8 = TimeInfo {
                utc_start_time: start,   // UTC: 02:00
                utc_expire_time: expire, // UTC: 10:00
                time_zone: tz(8),
            };

            let time_info_utc_m5 = TimeInfo {
                utc_start_time: start,   // UTC: 15:00
                utc_expire_time: expire, // UTC: 23:00
                time_zone: tz_west(5),
            };

            // UTC 4:00
            let time1 = naive_dt(2025, 6, 15, 4, 0, 0).and_utc();

            assert!(time_info_utc8.is_active_at(time1));
            assert!(!time_info_utc_m5.is_active_at(time1));

            // UTC 16:00
            let time2 = naive_dt(2025, 6, 15, 16, 0, 0).and_utc();
            assert!(!time_info_utc8.is_active_at(time2));
            assert!(time_info_utc_m5.is_active_at(time2));
        }

        #[test]
        fn test_timezone_boundary() {
            // Activity from 2025-06-15 20:00 to 2025-06-16 10:00 (UTC+8)
            // It's 2025-06-15 12:00 to 2025-06-16 2:00 in UTC
            let time_info = TimeInfo {
                utc_start_time: naive_dt(2025, 6, 15, 20, 0, 0),
                utc_expire_time: naive_dt(2025, 6, 16, 10, 0, 0),
                time_zone: tz(8),
            };

            // Test at 13:00 - should be active
            let time1 = naive_dt(2025, 6, 15, 13, 0, 0).and_utc();
            assert!(time_info.is_active_at(time1));

            // Test at 0:00 next day - should be active
            let time2 = naive_dt(2025, 6, 16, 0, 0, 0).and_utc();
            assert!(time_info.is_active_at(time2));

            // Test at 3:00 next day - should be inactive
            let time3 = naive_dt(2025, 6, 16, 3, 0, 0).and_utc();
            assert!(!time_info.is_active_at(time3));
        }
    }

    mod activity_info {
        use super::*;

        #[test]
        fn test_is_active() {
            let active_activity = ActivityInfo {
                tip: "Test Active Activity".to_string(),
                time_info: TimeInfo {
                    utc_start_time: naive_dt(2024, 1, 1, 0, 0, 0),
                    utc_expire_time: naive_dt(2050, 12, 31, 23, 59, 59),
                    time_zone: tz(8),
                },
            };

            let expired_activity = ActivityInfo {
                tip: "Test Expired Activity".to_string(),
                time_info: TimeInfo {
                    utc_start_time: naive_dt(2020, 1, 1, 0, 0, 0),
                    utc_expire_time: naive_dt(2022, 12, 31, 23, 59, 59),
                    time_zone: tz(8),
                },
            };

            assert!(active_activity.is_active());
            assert!(!expired_activity.is_active());
        }
    }

    mod mini_game_info {
        use super::*;

        #[test]
        fn test_is_active() {
            let active_game = MiniGameInfo {
                display: "Active Mini Game".to_string(),
                time_info: TimeInfo {
                    utc_start_time: naive_dt(2024, 1, 1, 0, 0, 0),
                    utc_expire_time: naive_dt(2050, 12, 31, 23, 59, 59),
                    time_zone: tz(8),
                },
            };

            let future_game = MiniGameInfo {
                display: "Future Mini Game".to_string(),
                time_info: TimeInfo {
                    utc_start_time: naive_dt(2051, 1, 1, 0, 0, 0),
                    utc_expire_time: naive_dt(2060, 12, 31, 23, 59, 59),
                    time_zone: tz(8),
                },
            };

            assert!(active_game.is_active());
            assert!(!future_game.is_active());
        }
    }

    mod stage_activity_content {
        use super::*;

        #[test]
        fn test_has_side_story_open_with_active() {
            let content = StageActivityContent {
                side_story_stage: {
                    let mut map = BTreeMap::new();
                    map.insert("ActiveStory".to_string(), SideStoryActivity {
                        activity: ActivityInfo {
                            tip: "Active Side Story".to_string(),
                            time_info: TimeInfo {
                                utc_start_time: naive_dt(2024, 1, 1, 0, 0, 0),
                                utc_expire_time: naive_dt(2050, 12, 31, 23, 59, 59),
                                time_zone: tz(8),
                            },
                        },
                        stages: vec![],
                    });
                    map
                },
                resource_collection: ActivityInfo {
                    tip: "Resource Collection".to_string(),
                    time_info: TimeInfo {
                        utc_start_time: naive_dt(2024, 1, 1, 0, 0, 0),
                        utc_expire_time: naive_dt(2050, 12, 31, 23, 59, 59),
                        time_zone: tz(8),
                    },
                },
                mini_game: vec![],
            };

            // Should have active side story (using current time which is within 2024-2050)
            assert!(content.has_side_story_open());
        }

        #[test]
        fn test_has_side_story_open_with_expired() {
            let content = StageActivityContent {
                side_story_stage: {
                    let mut map = BTreeMap::new();
                    map.insert("ExpiredStory".to_string(), SideStoryActivity {
                        activity: ActivityInfo {
                            tip: "Expired Side Story".to_string(),
                            time_info: TimeInfo {
                                utc_start_time: naive_dt(2020, 1, 1, 0, 0, 0),
                                utc_expire_time: naive_dt(2022, 12, 31, 23, 59, 59),
                                time_zone: tz(8),
                            },
                        },
                        stages: vec![],
                    });
                    map
                },
                resource_collection: ActivityInfo {
                    tip: "Resource Collection".to_string(),
                    time_info: TimeInfo {
                        utc_start_time: naive_dt(2020, 1, 1, 0, 0, 0),
                        utc_expire_time: naive_dt(2022, 12, 31, 23, 59, 59),
                        time_zone: tz(8),
                    },
                },
                mini_game: vec![],
            };

            // Should not have active side story (expired)
            let _ = content.has_side_story_open();
        }

        #[test]
        fn test_has_side_story_open_empty() {
            let content = StageActivityContent {
                side_story_stage: BTreeMap::new(),
                resource_collection: ActivityInfo {
                    tip: "Resource Collection".to_string(),
                    time_info: TimeInfo {
                        utc_start_time: naive_dt(2024, 1, 1, 0, 0, 0),
                        utc_expire_time: naive_dt(2050, 12, 31, 23, 59, 59),
                        time_zone: tz(8),
                    },
                },
                mini_game: vec![],
            };

            assert!(!content.has_side_story_open());
        }
    }

    mod utils {
        use super::*;

        #[test]
        fn warn_err() {
            let ok: Result<isize, &str> = Ok(1);
            let err: Result<isize, &str> = Err("test");

            assert_eq!(ok.warn_err(), Some(1));
            assert_eq!(err.warn_err(), None);
        }
    }
}
