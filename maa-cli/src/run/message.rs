use crate::{debug, error, info, normal, trace, warning};

use std::fmt::Write;

use maa_sys::binding::AsstMsgId;
use serde_json::{Map, Value};

trait IterExt: Iterator {
    fn join(&mut self, sep: &str) -> String
    where
        Self: Sized,
        Self::Item: std::fmt::Display,
    {
        match self.next() {
            None => String::new(),
            Some(first_elt) => {
                // estimate lower bound of capacity needed
                let (lower, _) = self.size_hint();
                let mut result = String::with_capacity(sep.len() * lower);
                write!(&mut result, "{}", first_elt).unwrap();
                self.for_each(|elt| {
                    result.push_str(sep);
                    write!(&mut result, "{}", elt).unwrap();
                });
                result
            }
        }
    }
}

impl<B, I, F> IterExt for std::iter::Map<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> B,
{
}

#[repr(i32)]
enum AsstMsg {
    /* Global Info */
    InternalError = 0,
    InitFailed = 1,
    ConnectionInfo = 2,
    AllTasksCompleted = 3,
    AsyncCallInfo = 4,

    /* TaskChain Info */
    TaskChainError = 10000,
    TaskChainStart = 10001,
    TaskChainCompleted = 10002,
    TaskChainExtraInfo = 10003,
    TaskChainStopped = 10004,

    /* SubTask Info */
    SubTaskError = 20000,
    SubTaskStart = 20001,
    SubTaskCompleted = 20002,
    SubTaskExtraInfo = 20003,
    SubTaskStopped = 20004,
}

use AsstMsg::*;

pub unsafe extern "C" fn callback(
    code: maa_sys::binding::AsstMsgId,
    json_raw: *const ::std::os::raw::c_char,
    _: *mut ::std::os::raw::c_void,
) {
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_raw).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
    process_message(code, json);
}

pub fn process_message(code: AsstMsgId, json: Value) {
    if !json.is_object() {
        return;
    }
    let message = json.as_object().unwrap();
    let ret = match code {
        code if code == InternalError as AsstMsgId => Some(()),
        code if code == InitFailed as AsstMsgId => {
            error!("InitializationError");
            Some(())
        }
        code if code == ConnectionInfo as AsstMsgId => process_connection_info(message),
        code if code == AllTasksCompleted as AsstMsgId => {
            normal!("AllTasksCompleted");
            Some(())
        }
        code if code == AsyncCallInfo as AsstMsgId => Some(()),
        code if (code >= TaskChainError as AsstMsgId && code <= TaskChainStopped as AsstMsgId) => {
            process_taskchain(code, message)
        }
        code if code == SubTaskError as AsstMsgId => process_subtask_error(message),
        code if code == SubTaskStart as AsstMsgId => process_subtask_start(message),
        code if code == SubTaskCompleted as AsstMsgId => process_subtask_completed(message),
        code if code == AsstMsg::SubTaskExtraInfo as AsstMsgId => {
            process_subtask_extra_info(message)
        }
        code if code == SubTaskStopped as AsstMsgId => Some(()),
        _ => Some(()),
    };

    // if ret is None, which means the message is not processed well
    // we should print the message to trace the error
    if ret.is_none() {
        debug!(
            "Process Failed",
            format!(
                "code: {}, message: {}",
                code,
                serde_json::to_string_pretty(message).unwrap()
            )
        )
    }
}

fn process_connection_info(message: &Map<String, Value>) -> Option<()> {
    let what = message.get("what")?.as_str()?;

    match what {
        "Connected" => info!("Connected"),
        "UnsupportedResolution" => error!("UnsupportedResolution"),
        "ResolutionError" => error!("ResolutionAcquisitionFailure"),
        "Reconnecting" => error!("TryToReconnect", {
            let times = message.get("times")?.as_i64()?;
            format!("{} times", times + 1)
        }),
        "Reconnected" => normal!("ReconnectSuccess"),
        "Disconnect" => error!("Disconnected"),
        "ScreencapFailed" => error!("ScreencapFailed"),
        "TouchModeNotAvailable" => error!("TouchModeNotAvailable"),
        _ => {
            trace!(
                "UnknownConnectionInfo",
                format!(
                    "what: {}, message: {}",
                    what,
                    serde_json::to_string_pretty(message).unwrap()
                )
            );
        }
    }

    Some(())
}

fn process_taskchain(code: AsstMsgId, message: &Map<String, Value>) -> Option<()> {
    let taskchain = message.get("taskchain")?.as_str()?;

    if taskchain == "CloseDown" {
        return Some(());
    }

    match code {
        code if code == TaskChainError as AsstMsgId => error!("TaskError", taskchain),
        code if code == TaskChainStart as AsstMsgId => normal!("StartTask", taskchain),
        code if code == TaskChainCompleted as AsstMsgId => normal!("CompleteTask", taskchain),
        code if code == TaskChainStopped as AsstMsgId => warning!("TaskChainStopped", taskchain),
        code if code == TaskChainExtraInfo as AsstMsgId => {}
        _ => {
            trace!(
                "UnknownTaskChainInfo",
                format!(
                    "code: {}, message: {}",
                    code,
                    serde_json::to_string_pretty(message).unwrap()
                )
            );
        }
    };

    Some(())
}

fn process_subtask_error(message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    match subtask {
        "StartGameTask" => error!("FailedToOpenClient"),
        "AutoRecruitTask" => error!(
            message
                .get("why")
                .map_or_else(|| "Unknown", |v| v.as_str().unwrap_or("Unknown")),
            "HasReturned"
        ),
        "RecognizeDrops" => error!("DropRecognitionError"),
        "ReportToPenguinStats" => error!(
            message
                .get("why")
                .map_or_else(|| "Unknown", |v| v.as_str().unwrap_or("Unknown")),
            "GiveUpUploadingPenguins"
        ),
        "CheckStageValid" => error!("TheEX"),
        _ => {
            trace!(
                "UnknownSubTaskError",
                format!(
                    "subtask: {}, message: {}",
                    subtask,
                    serde_json::to_string_pretty(message).unwrap()
                )
            )
        }
    };

    Some(())
}
fn process_subtask_start(message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    if subtask == "ProcessTask" {
        let details = message.get("details")?.as_object()?;
        let task = details.get("task")?.as_str()?;

        match task {
            "StartButton2" | "AnnihilationConfirm" => info!(
                "MissionStart",
                format!("{} times", details.get("exec_times")?.as_i64()?)
            ),
            "MedicineConfirm" => info!(
                "MedicineUsed",
                format!("{} times", details.get("exec_times")?.as_i64()?)
            ),
            "StoneConfirm" => info!(
                "StoneUsed",
                format!("{} times", details.get("exec_times")?.as_i64()?)
            ),
            "AbandonAction" => error!("ActingCommandError"),
            "RecruitRefreshConfirm" => info!("LabelsRefreshed"),
            "RecruitConfirm" => info!("RecruitConfirm"),
            "InfrastDormDoubleConfirmButton" => error!("InfrastDormDoubleConfirmed"),
            // RogueLike
            "StartExplore" => info!(
                "BegunToExplore",
                format!("{} times", details.get("exec_times")?.as_i64()?)
            ),
            "StageTraderInvestConfirm" => info!(
                "HasInvested",
                format!("{} times", details.get("exec_times")?.as_i64()?)
            ),
            // TODO: process more instead of just printing
            "ExitThenAbandon" => info!("ExplorationAbandoned"),
            "MissionCompletedFlag" => info!("MissionCompleted"),
            "MissionFailedFlag" => info!("MissionFailed"),
            "StageTraderEnter" => info!("StageTraderEnter"),
            "StageSafeHouseEnter" => info!("StageSafeHouseEnter"),
            "StageCambatDpsEnter" => info!("CambatDpsEnter"),
            "StageEmergencyDps" => info!("EmergencyDpsEnter"),
            "StageDreadfulFoe" | "StageDreadfulFoe-5Enter" => info!("DreadfulFoe"),
            "StageTraderInvestSystemFull" => warning!("TraderInvestSystemFull"),
            "OfflineConfirm" => warning!("GameOffline"),
            "GamePass" => info!("RoguelikeGamePass"),
            "BattleStartAll" => info!("MissionStart"),
            "StageTraderSpecialShoppingAfterRefresh" => {
                info!("RoguelikeSpecialItemBought")
            }
            _ => {} // There are too many tasks to process, so we just ignore them
        }
    }

    Some(())
}
fn process_subtask_completed(_: &Map<String, Value>) -> Option<()> {
    Some(())
}
fn process_subtask_extra_info(message: &Map<String, Value>) -> Option<()> {
    let taskchain = message.get("taskchain")?.as_str()?;
    match taskchain {
        "Depot" => info!("Depot", serde_json::to_string_pretty(message).unwrap()),
        "OperBox" => info!("OperBox", serde_json::to_string_pretty(message).unwrap()),
        _ => {}
    }

    let what = message.get("what")?.as_str()?;
    let details = message.get("details")?;

    match what {
        "StageDrops" => info!("Drops", {
            let statistics = details.get("stats")?.as_array()?;
            let mut all_drops: Vec<String> = Vec::new();
            for item in statistics {
                let name = item.get("itemName")?.as_str()?;
                let total = item.get("quantity")?.as_i64()?;
                let addition = item.get("addQuantity")?.as_i64()?;
                let drop = format!("{}: {} (+{})", name, total, addition);
                all_drops.push(drop);
            }
            if !all_drops.is_empty() {
                all_drops.join(", ")
            } else {
                String::from("none")
            }
        }),
        // Infrast
        "EnterFacility" => info!(
            "EnterFacility",
            format!(
                "{} #{}",
                details.get("facility")?.as_str()?,
                details.get("index")?.as_i64()?
            )
        ),
        "ProductIncorrect" => error!("ProductIncorrect"),
        "ProductUnknown" => error!("ProductUnknown"),
        "ProductChanged" => info!("ProductChanged"),
        "NotEnoughStaff" => error!("NotEnoughStaff"),
        "CustomInfrastRoomOperators" => info!(
            "CustomInfrastRoomOperators",
            details
                .get("names")?
                .as_array()?
                .iter()
                .map(|x| x.as_str().unwrap_or(""))
                .join(", ")
        ),
        // Recruit
        "RecruitTagsDetected" => info!(
            "RecruitResult:\n ",
            details
                .get("tags")?
                .as_array()?
                .iter()
                .map(|tag| tag.as_str().unwrap_or("Unknown"))
                .join("\n  ")
        ),
        "RecruitSpecialTag" => info!("RecruitingTips:", details.get("tag")?.as_str()?),
        "RecruitRobotTag" => info!("RecruitingTips:", details.get("tag")?.as_str()?),
        "RecruitResult" => info!("RecruitResult", {
            let level = details.get("level")?.as_u64()?;
            "★".repeat(level as usize)
        }),
        "RecruitTagsSelected" => info!("RecruitTagsSelected", {
            let tags = details.get("tags")?.as_array()?;
            let tags: Vec<&str> = tags.iter().map(|x| x.as_str().unwrap_or("")).collect();
            tags.join(", ")
        }),
        "RecruitTagsRefreshed" => info!("RecruitTagsRefreshed", {
            let count = details.get("count")?.as_i64()?;
            format!("{} times", count)
        }),
        // RogueLike
        "StageInfo" => info!("StartCombat", details.get("name")?.as_str()?),
        "StageInfoError" => error!("StageInfoError"),
        "BattleFormation" => info!("BattleFormation", details.get("formation")?.as_str()?),
        "BattleFormationSelected" => info!(
            "BattleFormationSelected",
            details.get("selected")?.as_str()?
        ),
        "CopilotAction" => info!("CurrentSteps", {
            format!(
                "{} {}",
                details.get("action")?.as_str()?,
                details.get("target")?.as_str()?,
            )
        }),
        // SSS
        "SSSStage" => info!("CurrentStage", details.get("stage")?.as_str()?),
        "SSSSettlement" => info!("SSSSettlement", details.get("why")?.as_str()?),
        "SSSGamePass" => info!("SSSGamePass"),
        "UnsupportedLevel" => error!("UnsupportedLevel"),
        _ => {
            trace!(
                "UnknownSubTaskExtraInfo",
                format!(
                    "what: {}, message: {}",
                    what,
                    serde_json::to_string_pretty(message).unwrap()
                )
            )
        }
    }

    Some(())
}
