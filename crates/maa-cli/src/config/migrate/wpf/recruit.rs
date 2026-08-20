use anyhow::Result;
use log::warn;
use maa_types::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{MigrationSummary, report_unknown_fields};

const HANDLED: &[&str] = &[
    "MaxTimes",
    "ExtraTagMode",
    "RefreshLevel3",
    "ForceRefresh",
    "Level6Choose",
    "Level6Time",
    "Level5Choose",
    "Level5Time",
    "Level4Choose",
    "Level4Time",
    "Level3Choose",
    "Level3Time",
    "PreferTagEnabled",
    "Level3PreferTags",
    "PreserveTagEnabled",
    "PreserveTagList",
];

#[derive(Debug, Serialize)]
struct CliRecruitTask {
    #[serde(rename = "type")]
    task_type: TaskType,
    #[serde(skip_serializing_if = "str::is_empty")]
    name: String,
    params: CliRecruitParams,
}

#[derive(Debug, Serialize)]
struct CliRecruitParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    times: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extra_tags_mode: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    select: Vec<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    confirm: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recruitment_time: Option<maa_value::map::StringMap<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preserve_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable: Option<bool>,
}

/// WPF GUI `RecruitTask` (`$type = "RecruitTask"`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct WpfRecruitTask {
    name: String,
    is_enable: bool,
    max_times: i32,
    extra_tag_mode: i32,
    refresh_level3: bool,
    force_refresh: bool,
    level3_choose: bool,
    #[serde(default)]
    level3_time: Option<i32>,
    level4_choose: bool,
    #[serde(default)]
    level4_time: Option<i32>,
    level5_choose: bool,
    #[serde(default)]
    level5_time: Option<i32>,
    level6_choose: bool,
    #[serde(default)]
    level6_time: Option<i32>,
    prefer_tag_enabled: bool,
    #[serde(default)]
    level3_prefer_tags: Vec<String>,
    preserve_tag_enabled: bool,
    #[serde(default)]
    preserve_tag_list: Vec<String>,
    #[serde(flatten)]
    unknown: Map<String, Value>,
}

impl WpfRecruitTask {
    pub(super) fn report_to(&self, summary: &mut MigrationSummary) {
        if !self.is_enable {
            summary.disable_task("RecruitTask", Some(self.name.clone()));
        }
        // GUI ForceRefresh is not maa-cli/MaaCore `expedite`; maa-cli does not support it.
        if self.force_refresh {
            summary.skip_field("RecruitTask", Some(self.name.clone()), "ForceRefresh");
            warn!(
                "RecruitTask ForceRefresh is not supported by maa-cli; this setting has no effect"
            );
        }
        if self.level6_choose {
            warn!(
                "RecruitTask Level6Choose is enabled; recruitment will auto-confirm 6★ operators (dangerous)"
            );
        }
        report_unknown_fields(
            summary,
            "RecruitTask",
            Some(self.name.clone()),
            &self.unknown,
            HANDLED,
        );
    }
}

impl TryFrom<&WpfRecruitTask> for Value {
    type Error = anyhow::Error;

    fn try_from(task: &WpfRecruitTask) -> Result<Self> {
        Ok(serde_json::to_value(CliRecruitTask::try_from(task)?)?)
    }
}

impl TryFrom<&WpfRecruitTask> for CliRecruitTask {
    type Error = anyhow::Error;

    fn try_from(task: &WpfRecruitTask) -> Result<Self> {
        let levels = [
            (task.level6_choose, task.level6_time, 6),
            (task.level5_choose, task.level5_time, 5),
            (task.level4_choose, task.level4_time, 4),
            (task.level3_choose, task.level3_time, 3),
        ];

        let mut select = Vec::new();
        let mut confirm = Vec::new();
        let mut recruitment_time = maa_value::map::StringMap::new();
        for (choose, minutes, level) in levels {
            if choose {
                select.push(level);
                confirm.push(level);
            }
            if let Some(minutes) = minutes {
                recruitment_time.insert(level.to_string(), minutes);
            }
        }

        let first_tags = (task.prefer_tag_enabled && !task.level3_prefer_tags.is_empty())
            .then(|| task.level3_prefer_tags.clone());
        let preserve_tags = (task.preserve_tag_enabled && !task.preserve_tag_list.is_empty())
            .then(|| task.preserve_tag_list.clone());

        Ok(CliRecruitTask {
            task_type: TaskType::Recruit,
            name: task.name.clone(),
            params: CliRecruitParams {
                times: Some(task.max_times),
                extra_tags_mode: Some(task.extra_tag_mode),
                refresh: Some(task.refresh_level3),
                select,
                confirm,
                recruitment_time: (!recruitment_time.is_empty()).then_some(recruitment_time),
                first_tags,
                preserve_tags,
                enable: (!task.is_enable).then_some(false),
            },
        })
    }
}
