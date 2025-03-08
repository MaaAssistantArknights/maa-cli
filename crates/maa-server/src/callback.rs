use tracing::{debug, error, info, trace, warn};

use crate::{
    session::{Session, State},
    types::{SessionID, TaskId, TaskStateType},
};

type Map = serde_json::Map<String, serde_json::Value>;

#[must_use = "If true, we should destory the session and session_id"]
#[tracing::instrument("C CallBack", skip_all)]
pub fn entry(code: TaskStateType, json_str: &str, session_id: SessionID) -> bool {
    trace!("Session ID: {:?}", session_id);

    Session::log(session_id).log((code, json_str.to_string()));

    let map: Map = serde_json::from_str(json_str).unwrap();

    // if ret is None, which means the message is not processed well
    // we should print the message to trace the error
    if let Some(destory) = process_message(code, map, session_id) {
        if destory {
            return true;
        }
        debug!(
            "FailedToProcessMessage, code: {:?}, message: {}",
            code, json_str
        )
    }
    false
}

fn process_message(code: TaskStateType, message: Map, session_id: SessionID) -> Option<bool> {
    use TaskStateType::*;

    match code {
        InternalError => {}
        InitFailed => {
            error!("InitializationError");
        }
        ConnectionInfo => process_connection_info(message, session_id),
        AllTasksCompleted => {
            let msg = serde_json::to_string_pretty(&message).unwrap();
            Session::info_to_channel(session_id, (code, msg));
            info!("AllTasksCompleted");
        }
        AsyncCallInfo => {}
        Destroyed => {
            info!("Instance destroyed");
            return Some(true);
        }

        TaskChainError | TaskChainStart | TaskChainCompleted | TaskChainExtraInfo
        | TaskChainStopped => process_taskchain(code, message, session_id),

        SubTaskError | SubTaskStart | SubTaskCompleted | SubTaskExtraInfo | SubTaskStopped => {
            subtask::process_subtask(code, message, session_id)
        }

        Unknown => return Some(false),
    }
    None
}

fn process_connection_info(message: Map, session_id: SessionID) {
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
            debug!(
                "Got UUID: {}",
                details.get("uuid").unwrap().as_str().unwrap()
            );
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
            details.get("width").unwrap().as_i64().unwrap(),
            details.get("height").unwrap().as_i64().unwrap()
        ),
        "UnsupportedResolution" => error!("Unsupported Resolution"),
        "ResolutionError" => error!("Resolution Acquisition Failure"),

        // Connection
        "Connected" => info!("Connected"),
        "Disconnect" => warn!("Disconnected"),
        "Reconnecting" => warn!(
            "{} {} {}",
            "Reconnect",
            details.get("times").unwrap().as_i64().unwrap(),
            "times"
        ),
        "Reconnected" => info!("Reconnect Success"),

        // Screen Capture
        "ScreencapFailed" => error!("Screencap Failed"),
        "FastestWayToScreencap" => trace!(
            "{} {} {}",
            "Fastest Way To Screencap",
            details.get("method").unwrap().as_str().unwrap(),
            details.get("cost").unwrap().as_i64().unwrap(),
        ),
        "ScreencapCost" => trace!(
            "{} {} ({} ~ {})",
            "Screencap Cost",
            details.get("avg").unwrap().as_i64().unwrap(),
            details.get("min").unwrap().as_i64().unwrap(),
            details.get("max").unwrap().as_i64().unwrap(),
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
}

fn process_taskchain(code: TaskStateType, message: Map, session_id: SessionID) {
    #[derive(serde::Deserialize)]
    struct TaskChain {
        taskchain: maa_types::TaskType,
        taskid: TaskId,
    }
    let msg = serde_json::to_string_pretty(&message).unwrap();
    let TaskChain { taskchain, taskid } =
        serde_json::from_value(serde_json::Value::Object(message)).unwrap();
    Session::tasks(session_id).update(taskid, (code, msg));

    use TaskStateType::*;
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

        _ => unreachable!(),
    };
}

mod subtask {
    use super::*;

    pub fn process_subtask(code: TaskStateType, message: Map, session_id: SessionID) {
        let msg = serde_json::to_string_pretty(&message).unwrap();
        let taskid = message.get("taskid").unwrap().as_i64().unwrap() as TaskId;
        Session::tasks(session_id).update(taskid, (code, msg));
    }
}
