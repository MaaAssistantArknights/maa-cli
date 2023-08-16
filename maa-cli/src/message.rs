use crate::log::Logger;
use maa_sys::binding::AsstMsgId;

use serde_json::{Map, Value};

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

macro_rules! create_callback {
    ($verbose:literal) => {{
        unsafe extern "C" fn callback(
            code: maa_sys::binding::AsstMsgId,
            json_raw: *const ::std::os::raw::c_char,
            _: *mut ::std::os::raw::c_void,
        ) {
            let logger = Logger::from($verbose);
            let json_str = unsafe { std::ffi::CStr::from_ptr(json_raw).to_str().unwrap() };
            let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
            crate::message::process_message(&logger, code, json);
        }
        callback
    }};
}

pub(crate) use create_callback;

pub fn process_message(logger: &Logger, code: AsstMsgId, json: Value) {
    if !json.is_object() {
        return;
    }
    let message = json.as_object().unwrap();
    let ret = match code {
        code if code == AsstMsg::InternalError as AsstMsgId => Some(()),
        code if code == AsstMsg::InitFailed as AsstMsgId => {
            logger.error("Error", || "init failed");
            Some(())
        }
        code if code == AsstMsg::ConnectionInfo as AsstMsgId => {
            process_connection_info(logger, message)
        }
        code if code == AsstMsg::AllTasksCompleted as AsstMsgId => {
            logger.info("AllTasksCompleted", || "");
            Some(())
        }
        code if code == AsstMsg::AsyncCallInfo as AsstMsgId => Some(()),
        code if (code >= AsstMsg::TaskChainError as AsstMsgId
            && code <= AsstMsg::TaskChainStopped as AsstMsgId) =>
        {
            process_taskchain(logger, code, message)
        }
        code if code == AsstMsg::SubTaskError as AsstMsgId => {
            process_subtask_error(logger, message)
        }
        code if code == AsstMsg::SubTaskStart as AsstMsgId => {
            process_subtask_start(logger, message)
        }
        code if code == AsstMsg::SubTaskCompleted as AsstMsgId => {
            process_subtask_completed(logger, message)
        }
        code if code == AsstMsg::SubTaskExtraInfo as AsstMsgId => {
            process_subtask_extra_info(logger, message)
        }
        code if code == AsstMsg::SubTaskStopped as AsstMsgId => Some(()),
        _ => Some(()),
    };

    // if ret is None, some unwarp failed
    if ret.is_none() {
        logger.debug("Process Failed", || {
            serde_json::to_string_pretty(message).unwrap()
        });
    }
}

fn process_connection_info(logger: &Logger, message: &Map<String, Value>) -> Option<()> {
    let what = message.get("what")?.as_str()?;

    match what {
        "Connected" => {
            logger.info("Connected", || "");
            logger.debug("Details", || {
                serde_json::to_string_pretty(message.get("details").unwrap()).unwrap()
            });
        }
        "UnsupportedResolution" => logger.error("UnsupportedResolution", || ""),
        "ResolutionError" => {
            logger.error("Error", || "ResolutionAcquisitionFailure");
        }
        "Reconnecting" => {
            logger.warning("Reconnecting", || {
                let times = message.get("times").unwrap().as_i64().unwrap();
                format!("Reconnect {} times", times + 1)
            });
        }
        "Reconnected" => {
            logger.info("Reconnected", || "");
        }
        "Disconnect" => {
            logger.error("Disconnected", || "");
        }
        "ScreencapFailed" => {
            logger.error("ReconnectFailed", || "");
        }
        "TouchModeNotAvailable" => {
            logger.error("TouchModeNotAvailable", || "");
        }
        _ => {
            logger.debug(what, || {
                serde_json::to_string_pretty(message.get("details").unwrap()).unwrap()
            });
        }
    }

    Some(())
}

fn process_taskchain(logger: &Logger, code: AsstMsgId, message: &Map<String, Value>) -> Option<()> {
    let taskchain = message.get("taskchain")?.as_str()?;

    if taskchain == "CloseDown" {
        return Some(());
    }

    match code {
        code if code == AsstMsg::TaskChainError as AsstMsgId => {
            logger.error("TaskChainError", || taskchain);
        }
        code if code == AsstMsg::TaskChainStart as AsstMsgId => {
            logger.info("TaskChainStart", || taskchain);
        }
        code if code == AsstMsg::TaskChainCompleted as AsstMsgId => {
            logger.info("TaskChainCompleted", || taskchain);
        }
        code if code == AsstMsg::TaskChainStopped as AsstMsgId => {
            logger.warning("TaskChainStopped", || taskchain);
        }
        code if code == AsstMsg::TaskChainExtraInfo as AsstMsgId => {}
        _ => {}
    };

    Some(())
}

fn process_subtask_error(logger: &Logger, message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    match subtask {
        "StartGameTask" => {
            logger.error("Failed to start game", || "");
        }
        "AutoRecruitTask" => {
            logger.error("Failed to auto recruit,", || {
                let why = message
                    .get("why")
                    .map_or_else(|| "Unknown", |v| v.as_str().unwrap_or("Unknown"));
                why
            });
        }
        "RecognizeDrops" => {
            logger.error("Failed to recognize drops", || "");
        }
        "ReportToPenguinStats" => {
            logger.error("Failed to report to penguin-stats,", || {
                let why = message
                    .get("why")
                    .map_or_else(|| "Unknown", |v| v.as_str().unwrap_or("Unknown"));
                why
            });
        }
        "CheckStageValid" => {
            logger.error("Invalid stage", || {
                let why = message
                    .get("why")
                    .map_or_else(|| "Unknown", |v| v.as_str().unwrap_or("Unknown"));
                why
            });
        }
        _ => {}
    };

    Some(())
}
fn process_subtask_start(logger: &Logger, message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    if subtask == "ProcessTask" {
        let details = message.get("details")?.as_object()?;
        let task = details.get("task")?.as_str()?;

        match task {
            "StartButton2" => logger.info("MissionStart", || {
                let times = details.get("exec_times").unwrap().as_i64().unwrap();
                format!("{} times", times)
            }),
            "AnnihilationConfirm" => logger.info("Annihilation", || {
                let times = details.get("exec_times").unwrap().as_i64().unwrap();
                format!("{} times", times)
            }),
            "MedicineConfirm" => logger.info("Medicine Used", || {
                let times = details.get("exec_times").unwrap().as_i64().unwrap();
                format!("{} times", times)
            }),
            "StoneConfirm" => logger.info("Stone Used", || {
                let times = details.get("exec_times").unwrap().as_i64().unwrap();
                format!("{} times", times)
            }),
            "AbandonAction" => {
                logger.error("ActingCommandError", || "");
            }
            "RecruitRefreshConfirm" => {
                logger.info("RecruitRefreshConfirm", || "");
            }
            "RecruitConfirm" => {
                logger.info("RecruitConfirm", || "");
            }
            "InfrastDormDoubleConfirmButton" => {
                logger.info("InfrastDormDoubleConfirmButton", || "");
            }
            _ => {}
        }
    }

    Some(())
}
fn process_subtask_completed(_: &Logger, _: &Map<String, Value>) -> Option<()> {
    Some(())
}
fn process_subtask_extra_info(logger: &Logger, message: &Map<String, Value>) -> Option<()> {
    let what = message.get("what")?.as_str()?;
    let details = message.get("details")?;

    match what {
        "StageDrops" => {
            let statistics = details.get("stats")?.as_array()?;
            let mut all_drops: Vec<String> = Vec::new();
            for item in statistics {
                let name = item.get("itemName")?.as_str()?;
                let total = item.get("quantity")?.as_i64()?;
                let addition = item.get("addition")?.as_i64()?;

                let mut drop = format!("{}: {}", name, total);
                if addition > 0 {
                    drop.push_str(&format!(" (+{})", addition));
                }
                all_drops.push(drop);
            }
            if !all_drops.is_empty() {
                logger.info("Drops:", || all_drops.join(", "));
            } else {
                logger.info("Drops:", || "None");
            }
        }
        // Infrast
        "EnterFacility" => {
            logger.info("EnterFacility", || {
                let facility = details.get("facility").unwrap().as_str().unwrap();
                let index = details.get("index").unwrap().as_i64().unwrap();
                format!("{}{}", facility, index)
            });
        }
        "ProductIncorrect" => logger.error("ProductIncorrect", || ""),
        "NotEnoughStuff" => logger.error("NotEnoughStuff", || ""),
        // Recruit
        "RecruitTagsDetected" => {
            logger.info("RecruitTagsDetected:", || {
                let tags = details.get("tags").unwrap().as_array().unwrap();
                let tags: Vec<&str> = tags.iter().map(|x| x.as_str().unwrap_or("")).collect();
                tags.join(", ")
            });
        }
        "RecruitSpecialTag" => {
            logger.info("RecruitSpecialTag:", || {
                let tags = details.get("tag").unwrap().as_array().unwrap();
                let tags: Vec<&str> = tags.iter().map(|x| x.as_str().unwrap_or("")).collect();
                tags.join(", ")
            });
        }
        "RecruitTagsRefreshed" => {
            logger.info("RecruitTagsRefreshed", || "");
        }
        "RecruitTagsSelect" => {
            let tags = details.get("tags")?.as_array()?;
            let tags: Vec<&str> = tags.iter().map(|x| x.as_str().unwrap_or("")).collect();
            println!("Selected tags: {}", tags.join(", "));
        }
        "RecruitRobotTag" => {
            logger.info("RecruitRobotTag", || "");
        }
        // misc
        // TODO: process more instead of just printing
        "Depot" => {
            logger.info("Depot", || serde_json::to_string_pretty(details).unwrap());
        }
        "OperBox" => {
            logger.info("OperBox", || serde_json::to_string_pretty(details).unwrap());
        }
        _ => {}
    }

    Some(())
}
