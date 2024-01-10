use std::{io::Write, path::Path};

use crate::{config::task::ClientType, dirs};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use lazy_static::lazy_static;
use log::warn;
use serde::Deserialize;
use serde_json::Value as JsonValue;

lazy_static! {
    static ref STAGE_ACTIVITY: Option<StageActivityJson> = load_stage_activity(
        dirs::hot_update()
            .join("cache")
            .join("gui")
            .join("StageActivity.json")
    )
    .warn_err();
}

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

#[derive(Deserialize)]
pub struct StageActivityJson {
    #[serde(rename = "Official")]
    official: StageActivityContent,
    #[serde(rename = "YoStarEN")]
    yostar_en: StageActivityContent,
    #[serde(rename = "YoStarJP")]
    yostar_jp: StageActivityContent,
    #[serde(rename = "YoStarKR")]
    yostar_kr: StageActivityContent,
    txwy: StageActivityContent,
}

fn load_stage_activity(file_path: impl AsRef<Path>) -> Result<StageActivityJson> {
    let file_path = file_path.as_ref();
    let file = std::fs::File::open(file_path).context("Failed to open StageActivity.json")?;
    serde_json::from_reader(file).context("Failed to parse StageActivity.json")
}

impl StageActivityJson {
    pub fn get_stage_activity(&self, client: ClientType) -> &StageActivityContent {
        use ClientType::*;
        match client {
            Official | Bilibili => &self.official,
            YoStarEN => &self.yostar_en,
            YoStarJP => &self.yostar_jp,
            YoStarKR => &self.yostar_kr,
            Txwy => &self.txwy,
        }
    }

    pub fn display(&self, mut f: impl Write, client: ClientType) -> std::io::Result<()> {
        let item_index = load_item_index(client);
        let stage_activity = self.get_stage_activity(client);
        let mut sidestory_title = false;
        for stage in &stage_activity.side_story_stage {
            if stage.activity.is_active() {
                if !sidestory_title {
                    writeln!(f, "Opening side story stages:")?;
                    sidestory_title = true;
                }
                let drop = item_index
                    .as_ref()
                    .warn_err()
                    .and_then(|item_index| item_index.get(&stage.drop))
                    .and_then(|item| item.get("name"))
                    .and_then(|name| name.as_str())
                    .unwrap_or(&stage.drop);
                writeln!(f, "- {}: {}", stage.value, drop)?;
            }
        }
        if stage_activity.resource_collection.is_active() {
            writeln!(f, "{}", stage_activity.resource_collection.tip)?;
        }
        Ok(())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageActivityContent {
    side_story_stage: Vec<StageInfo>,
    resource_collection: ActivityInfo,
}

impl StageActivityContent {
    pub fn has_side_story_open(&self) -> bool {
        for stage in &self.side_story_stage {
            if stage.activity.is_active() {
                return true;
            }
        }
        false
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StageInfo {
    value: String,
    drop: String,
    activity: ActivityInfo,
}

// Time format: 2023/12/22 16:00:00 and time zone is 8
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ActivityInfo {
    tip: String,
    utc_start_time: String,
    utc_expire_time: String,
    time_zone: i32,
}

impl ActivityInfo {
    pub fn is_active(&self) -> bool {
        let now = chrono::Utc::now();

        parse_time(&self.utc_start_time, self.time_zone).is_some_and(|t| t < now)
            && parse_time(&self.utc_expire_time, self.time_zone).is_some_and(|t| t > now)
    }
}

fn parse_time(time: &str, tz: i32) -> Option<DateTime<FixedOffset>> {
    NaiveDateTime::parse_from_str(time, "%Y/%m/%d %H:%M:%S")
        .ok()
        .and_then(|t| {
            FixedOffset::east_opt(tz * 3600).and_then(|tz| t.and_local_timezone(tz).single())
        })
}

fn load_item_index(client: ClientType) -> Result<JsonValue> {
    let hot_update_resource_dir = dirs::hot_update().join("resource");
    let base_resource_dir = if hot_update_resource_dir.exists() {
        hot_update_resource_dir
    } else if let Some(resource_dir) = dirs::find_resource() {
        resource_dir
    } else {
        bail!("Failed to find resource dir");
    };

    let item_index_path = match client.resource() {
        Some(global_resource) => base_resource_dir.join("global").join(global_resource),
        None => base_resource_dir,
    }
    .join("item_index.json");

    let file = std::fs::File::open(item_index_path).context("Failed to open item_index.json")?;

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
                warn!("{}", err);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_from_activity_info() {
        use chrono::{TimeZone, Utc};

        let time = parse_time("2023/12/22 16:00:00", 8).unwrap();
        assert_eq!(time, Utc.with_ymd_and_hms(2023, 12, 22, 8, 0, 0).unwrap());
    }

    #[test]
    fn parse_stage_activity() {
        let json_str = r#"
{
    "Official": {
        "sideStoryStage": [
            {
                "Display": "SSReopen-FC",
                "Value": "SSReopen-FC",
                "Drop": "代理1~8",
                "MinimumRequired": "v4.28.6",
                "Activity": {
                    "Tip": "SideStory「照我以火」复刻",
                    "StageName": "FC",
                    "UtcStartTime": "2023/12/22 16:00:00",
                    "UtcExpireTime": "2024/01/01 03:59:59",
                    "TimeZone": 8
                }
            },
            {
                "Display": "FC-7",
                "Value": "FC-7",
                "Drop": "31043",
                "MinimumRequired": "v4.28.6",
                "Activity": {
                    "Tip": "SideStory「照我以火」复刻",
                    "StageName": "FC",
                    "UtcStartTime": "2023/12/22 16:00:00",
                    "UtcExpireTime": "2024/01/01 03:59:59",
                    "TimeZone": 8
                }
            }
        ],
        "resourceCollection": {
            "Tip": "2023「感谢庆典」，“资源收集”限时全天开放",
            "UtcStartTime": "2023/11/21 16:00:00",
            "UtcExpireTime": "2023/12/05 03:59:59",
            "TimeZone": 8,
            "IsResourceCollection": true
        }
    },
    "YoStarJP": {
        "sideStoryStage": [],
        "resourceCollection": {
            "Tip": "「資源調達」全ステージ開放",
            "UtcStartTime": "2023/10/01 16:00:00",
            "UtcExpireTime": "2023/10/15 03:59:59",
            "TimeZone": 9,
            "IsResourceCollection": true
        }
    },
    "YoStarKR": {
        "sideStoryStage": [],
        "resourceCollection": {
            "Tip": "[자원 수진] 제한 시간 상시 오픈",
            "UtcStartTime": "2023/10/01 16:00:00",
            "UtcExpireTime": "2023/10/15 03:59:59",
            "TimeZone": 9,
            "IsResourceCollection": true
        }
    },
    "YoStarEN": {
        "sideStoryStage": [],
        "resourceCollection": {
            "Tip": "SUPPLIES & CHIPS EVERYDAY ACCESS",
            "UtcStartTime": "2023/10/01 10:00:00",
            "UtcExpireTime": "2023/10/15 03:59:59",
            "TimeZone": -7,
            "IsResourceCollection": true
        }
    },
    "txwy": {
        "sideStoryStage": [],
        "resourceCollection": {
            "Tip": "「資源收集」限時全天開放",
            "UtcStartTime": "2023/10/03 16:00:00",
            "UtcExpireTime": "2023/10/17 03:59:59",
            "TimeZone": 8,
            "IsResourceCollection": true
        }
    }
}
    "#;

        let stage_activity: StageActivityJson = serde_json::from_str(json_str).unwrap();

        assert_eq!(
            stage_activity.get_stage_activity(ClientType::Official),
            &StageActivityContent {
                side_story_stage: vec![
                    StageInfo {
                        value: "SSReopen-FC".to_string(),
                        drop: "代理1~8".to_string(),
                        activity: ActivityInfo {
                            tip: "SideStory「照我以火」复刻".to_string(),
                            utc_start_time: "2023/12/22 16:00:00".to_string(),
                            utc_expire_time: "2024/01/01 03:59:59".to_string(),
                            time_zone: 8,
                        }
                    },
                    StageInfo {
                        value: "FC-7".to_string(),
                        drop: "31043".to_string(),
                        activity: ActivityInfo {
                            tip: "SideStory「照我以火」复刻".to_string(),
                            utc_start_time: "2023/12/22 16:00:00".to_string(),
                            utc_expire_time: "2024/01/01 03:59:59".to_string(),
                            time_zone: 8,
                        }
                    }
                ],
                resource_collection: ActivityInfo {
                    tip: "2023「感谢庆典」，“资源收集”限时全天开放".to_string(),
                    utc_start_time: "2023/11/21 16:00:00".to_string(),
                    utc_expire_time: "2023/12/05 03:59:59".to_string(),
                    time_zone: 8,
                }
            }
        );

        assert_eq!(
            stage_activity.get_stage_activity(ClientType::Bilibili),
            stage_activity.get_stage_activity(ClientType::Official)
        );

        assert_eq!(
            stage_activity.get_stage_activity(ClientType::YoStarJP),
            &StageActivityContent {
                side_story_stage: vec![],
                resource_collection: ActivityInfo {
                    tip: "「資源調達」全ステージ開放".to_string(),
                    utc_start_time: "2023/10/01 16:00:00".to_string(),
                    utc_expire_time: "2023/10/15 03:59:59".to_string(),
                    time_zone: 9,
                }
            }
        );

        assert_eq!(
            stage_activity.get_stage_activity(ClientType::YoStarKR),
            &StageActivityContent {
                side_story_stage: vec![],
                resource_collection: ActivityInfo {
                    tip: "[자원 수진] 제한 시간 상시 오픈".to_string(),
                    utc_start_time: "2023/10/01 16:00:00".to_string(),
                    utc_expire_time: "2023/10/15 03:59:59".to_string(),
                    time_zone: 9,
                }
            }
        );

        assert_eq!(
            stage_activity.get_stage_activity(ClientType::YoStarEN),
            &StageActivityContent {
                side_story_stage: vec![],
                resource_collection: ActivityInfo {
                    tip: "SUPPLIES & CHIPS EVERYDAY ACCESS".to_string(),
                    utc_start_time: "2023/10/01 10:00:00".to_string(),
                    utc_expire_time: "2023/10/15 03:59:59".to_string(),
                    time_zone: -7,
                }
            }
        );

        assert_eq!(
            stage_activity.get_stage_activity(ClientType::Txwy),
            &StageActivityContent {
                side_story_stage: vec![],
                resource_collection: ActivityInfo {
                    tip: "「資源收集」限時全天開放".to_string(),
                    utc_start_time: "2023/10/03 16:00:00".to_string(),
                    utc_expire_time: "2023/10/17 03:59:59".to_string(),
                    time_zone: 8,
                }
            }
        );
    }

    #[test]
    fn stage_activity_content() {
        assert!(StageActivityContent {
            side_story_stage: vec![StageInfo {
                value: "FC-7".to_string(),
                drop: "31043".to_string(),
                activity: ActivityInfo {
                    tip: "Test".to_string(),
                    utc_start_time: "1970/01/01 00:00:00".to_string(),
                    utc_expire_time: "3000/01/01 00:00:00".to_string(),
                    time_zone: 8,
                },
            }],
            resource_collection: ActivityInfo {
                tip: "Test".to_string(),
                utc_start_time: "1970/01/01 00:00:00".to_string(),
                utc_expire_time: "3000/01/01 00:00:00".to_string(),
                time_zone: 8,
            },
        }
        .has_side_story_open());
        assert!(!StageActivityContent {
            side_story_stage: vec![StageInfo {
                value: "FC-7".to_string(),
                drop: "31043".to_string(),
                activity: ActivityInfo {
                    tip: "Test".to_string(),
                    utc_start_time: "1970/01/01 00:00:00".to_string(),
                    utc_expire_time: "1970/01/01 00:00:00".to_string(),
                    time_zone: 8,
                },
            }],
            resource_collection: ActivityInfo {
                tip: "Test".to_string(),
                utc_start_time: "1970/01/01 00:00:00".to_string(),
                utc_expire_time: "1970/01/01 00:00:00".to_string(),
                time_zone: 8,
            },
        }
        .has_side_story_open());
    }

    #[test]
    fn warn_err() {
        let ok: Result<isize, &str> = Ok(1);
        let err: Result<isize, &str> = Err("test");

        assert_eq!(ok.warn_err(), Some(1));
        assert_eq!(err.warn_err(), None);
    }
}
