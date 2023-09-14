use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{json, Value};

use super::taskchain::TaskChain;

#[derive(Deserialize, Debug)]
pub enum Task {
    StartButton2,
    AutoRecruitTask,
    RecognizeDrops,
    CheckStageValid,
    MedicineConfirm,
    StoneConfirm,
    RecruitRefreshConfirm,
    RecruitConfirm,
    RecruitNowConfirm,
    ReportToPenguinStats,
    ReportToYituliu,
    InfrastDormDoubleConfirmButton,
    StartExplore,
    StageTraderInvestConfirm,
    StageTraderInvestSystemFull,
    ExitThenAbandon,
    MissionCompletedFlag,
    MissionFailedFlag,
    StageTraderEnter,
    StageSafeHouseEnter,
    StageEncounterEnter,
    StageCambatDpsEnter,
    StageEmergencyDps,
    StageDreadfulFoe,
    StartGameTask,
}

#[derive(Deserialize, Debug)]
pub struct ProcessTaskDetails {
    pub task: Task,
    pub action: i32,
    pub exec_times: i32,
    pub max_times: i32,
    pub algorithm: i32,
}

#[derive(Debug)]
pub enum SubTaskStatus {
    SubTaskError,
    SubTaskStart,
    SubTaskCompleted,
    SubTaskStopped,
}

impl From<i32> for SubTaskStatus {
    fn from(value: i32) -> Self {
        match value {
            20000 => SubTaskStatus::SubTaskError,
            20001 => SubTaskStatus::SubTaskStart,
            20002 => SubTaskStatus::SubTaskCompleted,
            20004 => SubTaskStatus::SubTaskStopped,
            _ => panic!("Unknown SubTaskStatus: {}", value),
        }
    }
}

#[derive(Debug)]
pub enum SubTaskDetail {
    ProcessTask {
        status: SubTaskStatus,
        details: ProcessTaskDetails,
    },
}

impl SubTaskDetail {
    pub fn new(msg: i32, details: &str) -> Self {
        let status = SubTaskStatus::from(msg);
        let details: Value = serde_json::from_str(details).unwrap();
        let subtask: String = serde_json::from_value(details["subtask"].clone()).unwrap();
        match subtask.as_str() {
            "ProcessTask" => {
                let details: ProcessTaskDetails =
                    serde_json::from_value(details["details"].clone()).unwrap();
                SubTaskDetail::ProcessTask { status, details }
            }
            _ => panic!("Unknown SubTaskDetail: {}", subtask),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StageDropsStage {
    pub stage_code: String,
    pub stage_id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StageDropsStat {
    pub item_id: String,
    pub item_name: String,
    pub quantity: i32,
    pub add_quantity: i32,
}

#[derive(Deserialize, Debug)]
pub struct StageDropsDetail {
    pub stage: StageDropsStage,
    pub stars: i32,
    pub stats: Vec<StageDropsStat>,
}

#[derive(Deserialize, Debug)]
pub struct RecruitTagsDetectedDetail {
    pub tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct RecruitSpecialTagDetail {
    pub tag: String,
}

#[derive(Deserialize, Debug)]
pub struct RecruitResultOperator {
    pub name: String,
    pub level: i32,
}

#[derive(Deserialize, Debug)]
pub struct RecruitResultItem {
    pub tags: Vec<String>,
    pub level: i32,
    #[serde(rename = "opers")]
    pub operators: Vec<RecruitResultOperator>,
}

#[derive(Deserialize, Debug)]
pub struct RecruitResultDetail {
    pub tags: Vec<String>,
    pub level: i32,
    pub result: Vec<RecruitResultItem>,
}

#[derive(Deserialize, Debug)]
pub struct RecruitTagsRefreshedDetail {
    pub count: i32,
    pub refresh_limit: i32,
}

pub type RecruitTagsSelectedDetail = RecruitTagsDetectedDetail;

#[derive(Deserialize, Debug)]
pub struct EnterFacilityDetail {
    pub facility: String,
    pub index: i32,
}

pub type NotEnoughStaffDetail = EnterFacilityDetail;

#[derive(Deserialize, Debug)]
pub struct ProductOfFacilityDetail {
    pub product: String,
    pub facility: String,
    pub index: i32,
}

#[derive(Deserialize, Debug)]
pub struct StageInfoDetail {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct PenguinIdDetail {
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct DepotItem {
    pub id: String,
    pub have: i32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct DepotArkPlannerObject {
    pub items: Vec<DepotItem>,
    #[serde(rename = "@type")]
    pub object_type: String,
}

#[derive(Deserialize, Debug)]
pub struct DepotArkPlanner {
    pub object: DepotArkPlannerObject,
    pub data: String,
}

#[derive(Deserialize, Debug)]
pub struct DepotLolicon {
    pub object: HashMap<String, i32>,
    pub data: String,
}

#[derive(Deserialize, Debug)]
pub struct DepotDetail {
    pub done: bool,
    pub arkplanner: DepotArkPlanner,
    pub lolicon: DepotLolicon,
}

#[derive(Deserialize, Debug)]
pub struct OperatorBoxAllItem {
    pub id: String,
    pub name: String,
    pub own: bool,
    pub rarity: i32,
}

#[derive(Deserialize, Debug)]
pub struct OperatorBoxOwnItem {
    pub id: String,
    pub name: String,
    pub own: bool,
    pub elite: i32,
    pub level: i32,
    pub potential: i32,
    pub rarity: i32,
}

#[derive(Deserialize, Debug)]
pub struct OperBoxDetail {
    pub done: bool,
    pub all_oper: Vec<OperatorBoxAllItem>,
    pub own_opes: Vec<OperatorBoxOwnItem>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "what", content = "details")]
pub enum SubTaskExtraInfoDetails {
    StageDrops(StageDropsDetail),
    RecruitTagsDetected(RecruitTagsDetectedDetail),
    RecruitSpecialTag(RecruitSpecialTagDetail),
    RecruitResult(RecruitResultDetail),
    RecruitTagsRefreshed(RecruitTagsRefreshedDetail),
    RecruitTagsSelected(RecruitTagsSelectedDetail),
    RecruitSlotCompleted,
    RecruitError,
    EnterFacility(EnterFacilityDetail),
    NotEnoughStaff(NotEnoughStaffDetail),
    ProductOfFacility(ProductOfFacilityDetail),
    StageInfo(StageInfoDetail),
    StageInfoError,
    PenguinId(PenguinIdDetail),
    Depot(DepotDetail),
    OperBox(OperBoxDetail),
    UnsupportedLevel,
}

#[derive(Debug)]
pub struct SubTaskExtraInfoDetail {
    pub taskchain: TaskChain,
    pub class: String,
    pub uuid: String,
    pub details: SubTaskExtraInfoDetails,
}

impl<'de> Deserialize<'de> for SubTaskExtraInfoDetail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let taskchain: TaskChain = serde_json::from_value(value["taskchain"].clone()).unwrap();
        let class: String = serde_json::from_value(value["class"].clone()).unwrap();
        let uuid: String = serde_json::from_value(value["uuid"].clone()).unwrap();
        let what: String = serde_json::from_value(value["what"].clone()).unwrap();
        let details: Value = serde_json::from_value(value["details"].clone()).unwrap();
        let details_json = json!({
            "what":what,
            "details":details,
        });
        let details: SubTaskExtraInfoDetails = serde_json::from_value(details_json).unwrap();
        Ok(SubTaskExtraInfoDetail {
            taskchain,
            class,
            uuid,
            details,
        })
    }
}