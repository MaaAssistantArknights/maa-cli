use maa_types::primitive::{AsstMsgId, AsstTaskId as TaskId};

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum AsstMsg {
    /* Global Info */
    InternalError = 0,
    InitFailed = 1,
    ConnectionInfo = 2,
    AllTasksCompleted = 3,
    AsyncCallInfo = 4,
    Destroyed = 5,

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

    /* Unknown */
    Unknown = -1,
}

impl From<AsstMsgId> for AsstMsg {
    fn from(msg: AsstMsgId) -> Self {
        match msg {
            0 => AsstMsg::InternalError,
            1 => AsstMsg::InitFailed,
            2 => AsstMsg::ConnectionInfo,
            3 => AsstMsg::AllTasksCompleted,
            4 => AsstMsg::AsyncCallInfo,
            5 => AsstMsg::Destroyed,

            10000 => AsstMsg::TaskChainError,
            10001 => AsstMsg::TaskChainStart,
            10002 => AsstMsg::TaskChainCompleted,
            10003 => AsstMsg::TaskChainExtraInfo,
            10004 => AsstMsg::TaskChainStopped,

            20000 => AsstMsg::SubTaskError,
            20001 => AsstMsg::SubTaskStart,
            20002 => AsstMsg::SubTaskCompleted,
            20003 => AsstMsg::SubTaskExtraInfo,
            20004 => AsstMsg::SubTaskStopped,

            _ => AsstMsg::Unknown,
        }
    }
}

use tracing::{debug, error, info, trace, warn};

use crate::{session::{Session, State}, types::SessionID};

type Map = serde_json::Map<String, serde_json::Value>;

#[tracing::instrument("C CallBack", skip_all)]
pub fn main(code: AsstMsg, json_str: &str, session_id: SessionID) {
    trace!("Session ID: {:?}", session_id);

    Session::log(session_id).log(json_str.to_string());

    let map: Map = serde_json::from_str(json_str).unwrap();

    // if ret is None, which means the message is not processed well
    // we should print the message to trace the error
    if process_message(code, map, session_id).is_none() {
        debug!(
            "FailedToProcessMessage, code: {:?}, message: {}",
            code, json_str
        )
    }
}

fn process_message(code: AsstMsg, message: Map, session_id: SessionID) -> Option<()> {
    use AsstMsg::*;

    match code {
        InternalError => Some(()),
        InitFailed => {
            error!("InitializationError");
            Some(())
        }
        ConnectionInfo => process_connection_info(message, session_id),
        AllTasksCompleted => {
            info!("AllTasksCompleted");
            Some(())
        }
        AsyncCallInfo => Some(()),
        Destroyed => {
            debug!("Instance destroyed");
            Some(())
        }

        TaskChainError | TaskChainStart | TaskChainCompleted | TaskChainExtraInfo
        | TaskChainStopped => process_taskchain(code, message, session_id),

        SubTaskError | SubTaskStart | SubTaskCompleted | SubTaskExtraInfo | SubTaskStopped => {
            subtask::process_subtask(code, message, session_id)
        }

        Unknown => None,
    }
}

fn process_connection_info(message: Map, session_id: SessionID) -> Option<()> {
    #[derive(serde::Deserialize)]
    struct ConnectionInfo {
        what: String,
        why: Option<String>,
        details: Map,
    }
    let ConnectionInfo { what, why, details } =
        serde_json::from_value(serde_json::Value::Object(message)).unwrap();

    match what.as_str() {
        "UuidGot" => {
            debug!("Got UUID: {}", details.get("uuid")?.as_str()?);
            Session::test_connection_result(session_id, None);
        }
        "ConnectFailed" => {
            let err = format!("Failed to connect to android device, {}, Please check your connect configuration: {}",
                why.unwrap(),serde_json::to_string_pretty(&details).unwrap());
            error!(err);
            Session::test_connection_result(session_id, Some(err));
        }
        // Resolution
        "ResolutionGot" => trace!(
            "Got Resolution: {} X {}",
            details.get("width")?.as_i64()?,
            details.get("height")?.as_i64()?
        ),
        "UnsupportedResolution" => error!("Unsupported Resolution"),
        "ResolutionError" => error!("Resolution Acquisition Failure"),

        // Connection
        "Connected" => info!("Connected"),
        "Disconnect" => warn!("Disconnected"),
        "Reconnecting" => warn!(
            "{} {} {}",
            "Reconnect",
            details.get("times")?.as_i64()?,
            "times"
        ),
        "Reconnected" => info!("Reconnect Success"),

        // Screen Capture
        "ScreencapFailed" => error!("Screencap Failed"),
        "FastestWayToScreencap" => trace!(
            "{} {} {}",
            "Fastest Way To Screencap",
            details.get("method")?.as_str()?,
            details.get("cost")?.as_i64()?,
        ),
        "ScreencapCost" => trace!(
            "{} {} ({} ~ {})",
            "Screencap Cost",
            details.get("avg")?.as_i64()?,
            details.get("min")?.as_i64()?,
            details.get("max")?.as_i64()?,
        ),

        "TouchModeNotAvailable" => error!("Touch Mode Not Available"),
        _ => debug!(
            "{}: what:{} why:{} details:{}",
            "Unknown Connection Info",
            what,
            why.as_deref().unwrap_or("No why"),
            serde_json::to_string_pretty(&details).unwrap()
        ),
    }

    Some(())
}

fn process_taskchain(code: AsstMsg, message: Map, session_id: SessionID) -> Option<()> {
    #[derive(serde::Deserialize)]
    struct TaskChain {
        taskchain: maa_types::TaskType,
        taskid: TaskId,
    }
    let TaskChain { taskchain, taskid } =
        serde_json::from_value(serde_json::Value::Object(message)).unwrap();

    use AsstMsg::*;

    match code {
        TaskChainStart => {
            info!("{} {}", taskchain, "Start");
            Session::tasks(session_id).state(taskid, State::Running);
        }
        TaskChainCompleted => {
            info!("{} {}", taskchain, "Completed");
            Session::tasks(session_id).state(taskid, State::Completed);
        }
        TaskChainStopped => {
            warn!("{} {}", taskchain, "Stopped");
            Session::tasks(session_id).state(taskid, State::Canceled);
        }
        TaskChainError => {
            error!("{} {}", taskchain, "Error");
            Session::tasks(session_id).state(taskid, State::Error);
        }
        TaskChainExtraInfo => {}

        _ => {} // unreachable
    };

    Some(())
}

mod subtask {
    use super::*;

    pub fn process_subtask(_code: AsstMsg, message: Map, session_id: SessionID) -> Option<()> {
        let msg = serde_json::to_string_pretty(&message).unwrap();
        let taskid = message.get("taskid")?.as_i64()? as TaskId;
        Session::tasks(session_id).update(taskid, msg);
        Some(())
    }
}
