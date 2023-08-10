use super::binding::AsstMsgId;
use serde_json::{from_str, Map, Value};

#[repr(i32)]
enum AsstMsg {
    /* Global Info */
    InternalError = 0,     // 内部错误
    InitFailed = 1,        // 初始化失败
    ConnectionInfo = 2,    // 连接相关信息
    AllTasksCompleted = 3, // 全部任务完成
    #[allow(dead_code)]
    AsyncCallInfo = 4, // 外部异步调用信息

    /* TaskChain Info */
    TaskChainError = 10000,     // 任务链执行/识别错误
    TaskChainStart = 10001,     // 任务链开始
    TaskChainCompleted = 10002, // 任务链完成
    TaskChainExtraInfo = 10003, // 任务链额外信息
    TaskChainStopped = 10004,   // 任务链手动停止

    /* SubTask Info */
    SubTaskError = 20000,     // 原子任务执行/识别错误
    SubTaskStart = 20001,     // 原子任务开始
    SubTaskCompleted = 20002, // 原子任务完成
    SubTaskExtraInfo = 20003, // 原子任务额外信息
    SubTaskStopped = 20004,   // 原子任务手动停止
}

pub unsafe extern "C" fn default_callback(
    msg: std::os::raw::c_int,
    json_raw: *const ::std::os::raw::c_char,
    _: *mut ::std::os::raw::c_void,
) {
    let json_str = std::ffi::CStr::from_ptr(json_raw).to_str().unwrap();
    let json: Value = from_str(json_str).unwrap();
    processs_message(msg as AsstMsgId, json);
}

fn processs_message(code: AsstMsgId, json: Value) -> Option<()> {
    if !json.is_object() {
        return None;
    }
    let message = json.as_object().unwrap();
    match code {
        code if code == AsstMsg::InternalError as AsstMsgId => process_internal_error(message),
        code if code == AsstMsg::InitFailed as AsstMsgId => process_init_failed(message),
        code if code == AsstMsg::ConnectionInfo as AsstMsgId => process_connection_info(message),
        code if code == AsstMsg::AllTasksCompleted as AsstMsgId => {
            process_all_tasks_completed(message)
        }
        code if (code >= AsstMsg::TaskChainError as AsstMsgId
            && code <= AsstMsg::SubTaskStopped as AsstMsgId) =>
        {
            process_taskchain(code, message)
        }
        code if code == AsstMsg::SubTaskError as AsstMsgId => process_subtask_error(message),
        code if code == AsstMsg::SubTaskStart as AsstMsgId => process_subtask_start(message),
        code if code == AsstMsg::SubTaskCompleted as AsstMsgId => {
            process_subtask_completed(message)
        }
        code if code == AsstMsg::SubTaskExtraInfo as AsstMsgId => {
            process_subtask_extra_info(message)
        }
        _ => None,
    }
}

fn process_internal_error(_: &Map<String, Value>) -> Option<()> {
    return Some(());
}

fn process_init_failed(message: &Map<String, Value>) -> Option<()> {
    let what = message.get("what")?.as_str()?;
    let why = message.get("why")?.as_str()?;

    println!("InitFailed: {} {}", what, why);

    return Some(());
}

fn process_connection_info(message: &Map<String, Value>) -> Option<()> {
    let what = message.get("what")?.as_str()?;

    match what {
        "Connected" => {}
        "Reconnecting" => {
            let times = message.get("times")?.as_i64()?;
            println!("Reconnect {} times", times);
        }
        "Reconnected" => {
            println!("Reconnect successfully");
        }
        _ => {
            eprintln!("Unknown Error: {}", what);
            eprintln!("Why: {}", message.get("why")?.as_str()?);
            eprintln!(
                "Details: {}",
                serde_json::to_string_pretty(message.get("details")?).ok()?
            );
        }
    }

    return Some(());
}

fn process_all_tasks_completed(_: &Map<String, Value>) -> Option<()> {
    println!("AllTasksCompleted");
    return Some(());
}

fn process_taskchain(code: AsstMsgId, message: &Map<String, Value>) -> Option<()> {
    let taskchain = message.get("taskchain")?.as_str()?;

    match code {
        code if code == AsstMsg::TaskChainError as AsstMsgId => {
            eprintln!("TaskChainError: {}", taskchain);
        }
        code if code == AsstMsg::TaskChainStart as AsstMsgId => {
            println!("TaskChainStart: {}", taskchain);
        }
        code if code == AsstMsg::TaskChainCompleted as AsstMsgId => {
            println!("TaskChainCompleted: {}", taskchain);
        }
        code if code == AsstMsg::TaskChainExtraInfo as AsstMsgId => {
            println!("TaskChainStopped: {}", taskchain);
        }
        code if code == AsstMsg::TaskChainStopped as AsstMsgId => {}
        _ => {
            return None;
        }
    };

    return Some(());
}

fn process_subtask_error(message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    match subtask {
        "StartGameTask" => {
            eprintln!("Failed to start game");
        }
        "AutoRecruitTask" => {
            let why = message.get("why")?.as_str().unwrap_or("Unknown");
            eprintln!("Failed to recruit: {}", why);
        }
        "RecognizeDrops" => {
            eprintln!("Failed to recognize drops");
        }
        "ReportToPenguinStats" => {
            let why = message.get("why")?.as_str().unwrap_or("Unknown");
            eprintln!("Failed to report to penguin-stats: {}", why);
        }
        _ => {}
    };

    return Some(());
}
fn process_subtask_start(message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    match subtask {
        "ProcessTask" => {
            let name = message.get("details")?.get("task")?.as_str()?;
            let times = message.get("details")?.get("exec_times")?.as_i64()?;

            match name {
                "StartButton2" => {
                    println!("MissionStart {} times", times);
                }
                "AnnihilationConfirm" => {
                    println!("MissionStart {} times", times);
                }
                "MedicineConfirm" => {
                    println!("MedicineUsed {} times", times);
                }
                "StoneConfirm" => {
                    println!("StoneUsed {} times", times);
                }
                "AbandonAction" => {
                    eprintln!("ActingCommandError");
                }
                "RecruitRefreshConfirm" => {
                    println!("LabelsRefreshed");
                }
                "RecruitConfirm" => {
                    println!("RecruitConfirm");
                }
                "InfrastDormDoubleConfirmButton" => {
                    println!("InfrastDormDoubleConfirmed");
                }
                _ => {}
            }
        }
        _ => {}
    }
    return Some(());
}
fn process_subtask_completed(_: &Map<String, Value>) -> Option<()> {
    return Some(());
}
fn process_subtask_extra_info(message: &Map<String, Value>) -> Option<()> {
    let taskchain = message.get("taskchain")?.as_str()?;
    let what = message.get("what")?.as_str()?;
    let details = message.get("details")?;

    match taskchain {
        _ => {}
    }

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
                println!("Drops: {}", all_drops.join(", "));
            } else {
                println!("Drops: None");
            }
        }
        // Infrast
        "EnterFacility" => {
            let facility = details.get("facility")?.as_str()?;
            let index = details.get("index")?.as_i64()?;
            println!("Enterv facility: {}{}", facility, index);
        }
        "ProductIncorrect" => eprintln!("Product incorrect"),
        "NotEnoughStuff" => eprintln!("Not enough stuff"),
        // Recruit
        "RecruitTagsDetected" => {
            let tags = details.get("tags")?.as_array()?;
            let tags: Vec<&str> = tags.iter().map(|x| x.as_str().unwrap_or("")).collect();
            println!("Recruit Tags: {}", tags.join(", "));
        }
        "RecruitSpecialTag" => {
            let tags = details.get("tag")?.as_array()?;
            let tags: Vec<&str> = tags.iter().map(|x| x.as_str().unwrap_or("")).collect();
            println!("Recruit Special Tag: {}", tags.join(", "));
        }
        "RecruitTagsRefreshed" => {
            let count = details.get("count")?.as_i64()?;
            println!("Recruit refresh count: {}", count);
        }
        "RecruitTagsSelect" => {
            let tags = details.get("tags")?.as_array()?;
            let tags: Vec<&str> = tags.iter().map(|x| x.as_str().unwrap_or("")).collect();
            println!("Selected tags: {}", tags.join(", "));
        }
        // misc
        // TODO: process more instead of just printing
        "Depot" => {
            println!("Depot: {}", serde_json::to_string_pretty(details).ok()?);
        }
        "OperBox" => {
            println!("OperBox: {}", serde_json::to_string_pretty(details).ok()?);
        }
        _ => {}
    }

    return Some(());
}
