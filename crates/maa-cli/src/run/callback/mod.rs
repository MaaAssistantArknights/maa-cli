pub mod summary;
use std::{fmt::Write, sync::atomic::AtomicBool};

use log::{debug, error, info, trace, warn};
use maa_types::primitive::{AsstMsgId, AsstTaskId};
use serde_json::{Map, Value};
use summary::{edit_current_task_detail, end_current_task, start_task};

pub static MAA_CORE_ERRORED: AtomicBool = AtomicBool::new(false);

pub unsafe extern "C" fn default_callback(
    code: AsstMsgId,
    json_raw: *const ::std::os::raw::c_char,
    _: *mut ::std::os::raw::c_void,
) {
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_raw).to_str().unwrap() };
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
    process_message(code, json);
}

#[repr(i32)]
enum AsstMsg {
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

fn process_message(code: AsstMsgId, json: Value) {
    if !json.is_object() {
        return;
    }

    let message = json.as_object().unwrap();

    use AsstMsg::*;

    let ret = match code.into() {
        InternalError => Some(()),
        InitFailed => {
            error!("InitializationError");
            Some(())
        }
        ConnectionInfo => process_connection_info(message),
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
        | TaskChainStopped => process_taskchain(code.into(), message),

        SubTaskError => process_subtask_error(message),
        SubTaskStart => process_subtask_start(message),
        SubTaskCompleted => process_subtask_completed(message),
        SubTaskExtraInfo => process_subtask_extra_info(message),
        SubTaskStopped => Some(()),

        Unknown => None,
    };

    // if ret is None, which means the message is not processed well
    // we should print the message to trace the error
    if ret.is_none() {
        debug!(
            "FailedToProcessMessage, code: {}, message: {}",
            code,
            serde_json::to_string_pretty(message).unwrap()
        )
    }
}

fn process_connection_info(message: &Map<String, Value>) -> Option<()> {
    let what = message.get("what")?.as_str()?;

    match what {
        "UuidGot" => debug!(
            "Got UUID: {}",
            message.get("details")?.get("uuid")?.as_str()?
        ),
        "ConnectFailed" => error!(
            "Failed to connect to android device, {}, Please check your connect configuration: {}",
            message.get("why")?.as_str()?,
            serde_json::to_string_pretty(message.get("details")?).unwrap()
        ),
        // Resolution
        "ResolutionGot" => debug!(
            "Got Resolution: {} × {}",
            message.get("details")?.get("width")?.as_i64()?,
            message.get("details")?.get("height")?.as_i64()?
        ),
        "UnsupportedResolution" => error!("{}", "UnsupportedResolution"),
        "ResolutionError" => error!("{}", "ResolutionAcquisitionFailure"),

        // Connection
        "Connected" => info!("{}", "Connected"),
        "Disconnect" => warn!("{}", "Disconnected"),
        "Reconnecting" => warn!(
            "{} {} {}",
            "Reconnect",
            message.get("details")?.get("times")?.as_i64()?,
            "times"
        ),
        "Reconnected" => info!("{}", "ReconnectSuccess"),

        // Screen Capture
        "ScreencapFailed" => error!("{}", "ScreencapFailed"),
        "FastestWayToScreencap" => info!(
            "{} {} {}",
            "FastestWayToScreencap",
            message.get("details")?.get("method")?.as_str()?,
            message.get("details")?.get("cost")?.as_i64()?,
        ),
        "ScreencapCost" => debug!(
            "{} {} ({} ~ {})",
            "ScreencapCost",
            message.get("details")?.get("avg")?.as_i64()?,
            message.get("details")?.get("min")?.as_i64()?,
            message.get("details")?.get("max")?.as_i64()?,
        ),

        "TouchModeNotAvailable" => error!("{}", "TouchModeNotAvailable"),
        _ => {
            trace!(
                "{}: {}",
                "Unknown Connection Info",
                serde_json::to_string_pretty(message).unwrap()
            );
        }
    }

    Some(())
}

fn process_taskchain(code: AsstMsg, message: &Map<String, Value>) -> Option<()> {
    let taskchain = message.get("taskchain")?.as_str()?;

    use AsstMsg::*;

    match code {
        TaskChainStart => {
            info!("{} {}", taskchain, "Start");
            start_task(message.get("taskid")?.as_i64()? as AsstTaskId);
        }
        TaskChainCompleted => {
            info!("{} {}", taskchain, "Completed");
            end_current_task(summary::Reason::Completed);
        }
        TaskChainStopped => {
            warn!("{} {}", taskchain, "Stopped");
            end_current_task(summary::Reason::Stopped);
        }
        TaskChainError => {
            error!("{} {}", taskchain, "Error");
            end_current_task(summary::Reason::Error);
            MAA_CORE_ERRORED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        TaskChainExtraInfo => {}

        _ => {} // unreachable
    };

    Some(())
}

fn process_subtask_error(message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    match subtask {
        "StartGameTask" => error!("{}", "FailedToStartGame"),
        "AutoRecruitTask" => error!("{} {}", message.get("why")?.as_str()?, "HasReturned"),
        "RecognizeDrops" => error!("{}", "FailedToRecognizeDrops"),
        "ReportToPenguinStats" => error!(
            "{}, {}",
            "FailedToReportToPenguinStats",
            message.get("why")?.as_str()?,
        ),
        "CheckStageValid" => error!("TheEX"),
        _ => trace!(
            "{}: {}",
            "UnknownSubTaskError",
            serde_json::to_string_pretty(message).unwrap()
        ),
    };

    Some(())
}
fn process_subtask_start(message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    if subtask == "ProcessTask" {
        let details = message.get("details")?.as_object()?;
        let task = details.get("task")?.as_str()?;

        match task {
            // Fight
            "StartButton2" | "AnnihilationConfirm" => {
                // Maybe need to update if MAA fight a stage multiple times in one run
                let exec_times = details.get("exec_times")?.as_i64()?;
                edit_current_task_detail(move |detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.set_times(exec_times);
                    }
                });
                info!("{} {} {}", "MissionStart", exec_times, "times");
            }
            "StoneConfirm" => {
                let exec_times = details.get("exec_times")?.as_i64()?;
                edit_current_task_detail(move |detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.set_stone(exec_times)
                    }
                });
                info!("Use {} stones", exec_times);
            }
            "AbandonAction" => warn!("{}", "PRTS error"),
            // Recruit
            "RecruitRefreshConfirm" => {
                edit_current_task_detail(move |detail| {
                    if let Some(detail) = detail.as_recruit_mut() {
                        detail.refresh()
                    }
                });
                info!("{}", "Refresh Tags")
            }
            "RecruitConfirm" => {
                edit_current_task_detail(move |detail| {
                    if let Some(detail) = detail.as_recruit_mut() {
                        detail.recruit()
                    }
                });
                info!("{}", "Recruit")
            }
            // Infrast
            "InfrastDormDoubleConfirmButton" => warn!("{}", "InfrastDormDoubleConfirmed"),
            // RogueLike
            "StartExplore" => {
                let exec_times = details.get("exec_times")?.as_i64()?;
                edit_current_task_detail(move |detail| {
                    if let Some(detail) = detail.as_roguelike_mut() {
                        detail.start_exploration()
                    }
                });
                info!("Start exploration {} times", exec_times)
            }
            "ExitThenAbandon" => {
                edit_current_task_detail(move |detail| {
                    if let Some(detail) = detail.as_roguelike_mut() {
                        detail.set_state(summary::ExplorationState::Abandoned)
                    }
                });
                info!("Exploration Abandoned")
            }
            "ExitThenConfirm" => info!("{}", "ExplorationConfirmed"),
            "MissionCompletedFlag" => info!("{}", "MissionCompleted"),
            "MissionFailedFlag" => {
                // Deposit In some cases a failed mission doesn't mean failed exploration
                // If a exploration was not failed, it's state would be overwritten later
                if message.get("taskchain")?.as_str()? == "Roguelike" {
                    edit_current_task_detail(move |detail| {
                        if let Some(detail) = detail.as_roguelike_mut() {
                            detail.set_state(summary::ExplorationState::Failed)
                        }
                    });
                }
                info!("MissionFailed")
            }
            "StageTraderEnter" => info!("{}", "StageTraderEnter"),
            "StageSafeHouseEnter" => info!("{}", "StageSafeHouseEnter"),
            "StageCambatDpsEnter" => info!("{}", "StageCambatDpsEnter"),
            "StageEmergencyDps" => info!("{}", "EmergencyDpsEnter"),
            "StageDreadfulFoe" | "StageDreadfulFoe-5Enter" => info!("{}", "DreadfulFoe"),
            "StageTraderInvestSystemFull" => warn!("{}", "TraderInvestSystemFull"),
            "GamePass" => info!("{}", "RoguelikeGamePass"),

            "OfflineConfirm" => warn!("{}", "GameOffline"),
            "BattleStartAll" => info!("{}", "MissionStart"),
            "StageTraderSpecialShoppingAfterRefresh" => info!("{}", "RoguelikeSpecialItemBought"),
            _ => trace!(
                "{}: {}",
                "UnknownSubTaskStart",
                serde_json::to_string_pretty(message).unwrap()
            ),
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
        "Depot" => info!(
            "{}: {}",
            "Depot",
            serde_json::to_string_pretty(message).unwrap()
        ),
        "OperBox" => info!(
            "{}: {}",
            "OperBox",
            serde_json::to_string_pretty(message).unwrap()
        ),
        _ => {}
    }

    let what = message.get("what")?.as_str()?;
    let details = message.get("details")?;

    match what {
        "StageDrops" => {
            let drops = details.get("drops")?.as_array()?;
            let mut all_drops = summary::Map::new();
            for drop in drops {
                let drop = drop.as_object()?;
                let item = drop.get("itemName")?.as_str()?;
                let count = drop.get("quantity")?.as_i64()?;
                all_drops.insert(item.to_owned(), count);
            }

            info!(
                "{}: {}",
                "Drops",
                all_drops
                    .iter()
                    .map(|(item, count)| format!("{} × {}", item, count))
                    .join(", ")
                    .unwrap_or_else(|| "none".to_owned())
            );

            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_fight_mut() {
                    detail.push_drop(all_drops);
                }
            });

            let stage = details.get("stage")?.get("stageCode")?.as_str()?.to_owned();
            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_fight_mut() {
                    detail.set_stage(stage.as_str());
                }
            });
        }

        // Sanity and Medicines
        "SanityBeforeStage" => info!(
            "Current sanity: {}/{}",
            details.get("current_sanity")?.as_i64()?,
            details.get("max_sanity")?.as_i64()?
        ),
        "UseMedicine" => {
            let count = details.get("count")?.as_i64()?;
            let is_expiring = details.get("is_expiring")?.as_bool()?;
            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_fight_mut() {
                    detail.use_medicine(count, is_expiring);
                }
            });

            if is_expiring {
                info!("Use {} expiring medicine", count);
            } else {
                info!("Use {} medicine", count);
            }
        }

        // Infrast
        "EnterFacility" => info!(
            "{} {} #{}",
            "EnterFacility",
            details.get("facility")?.as_str()?,
            details.get("index")?.as_i64()?,
        ),
        "ProductIncorrect" => warn!("{}", "ProductIncorrect"),
        "ProductUnknown" => error!("{}", "ProductUnknown"),
        "ProductChanged" => info!("{}", "ProductChanged"),
        "NotEnoughStaff" => error!("{}", "NotEnoughStaff"),
        "ProductOfFacility" => {
            let facility = details.get("facility")?.as_str()?.to_owned();
            let index = details.get("index")?.as_i64()?;
            let product = details.get("product")?.as_str()?.to_owned();

            info!("{}: {}", "ProductOfFacility", product);

            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_infrast_mut() {
                    detail.set_product(facility.parse().unwrap(), index, product.as_str());
                }
            });
        }
        "CustomInfrastRoomOperators" => {
            let facility = details.get("facility")?.as_str()?.to_owned();
            let index = details.get("index")?.as_i64()?;
            let operators = details.get("names")?.as_array()?.to_owned();
            let candidates = details.get("candidates")?.as_array()?.to_owned();

            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_infrast_mut() {
                    detail.set_operators(
                        facility.parse().unwrap(),
                        index,
                        operators
                            .iter()
                            .filter_map(|x| x.as_str().map(|x| x.to_owned()))
                            .collect(),
                        candidates
                            .iter()
                            .filter_map(|x| x.as_str().map(|x| x.to_owned()))
                            .collect(),
                    );
                }
            });

            info!(
                "{}: {}",
                "CustomInfrastRoomOperators",
                details
                    .get("names")?
                    .as_array()?
                    .iter()
                    .filter_map(|x| x.as_str())
                    .join(", ")
                    .unwrap_or_else(|| "none".to_owned())
            )
        }

        // Recruit
        "RecruitTagsDetected" => (), // this info is contained in RecruitResult, so ignore it
        "RecruitSpecialTag" => info!("{}: {}", "RecruitingTips", details.get("tag")?.as_str()?),
        "RecruitRobotTag" => info!("{}: {}", "RecruitingTips", details.get("tag")?.as_str()?),
        "RecruitResult" => {
            let level = details.get("level")?.as_u64()?;
            let tags = details.get("tags")?.as_array()?.to_owned();

            info!(
                "{}: {} {}",
                "RecruitResult",
                "★".repeat(level as usize),
                tags.iter()
                    .filter_map(|x| x.as_str())
                    .join(", ")
                    .unwrap_or_else(|| "none".to_owned())
            );

            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_recruit_mut() {
                    detail.push_recruit(
                        level,
                        tags.iter().filter_map(|x| x.as_str().map(|x| x.to_owned())),
                    );
                }
            });
        }
        "RecruitTagsSelected" => info!("{}: {}", "RecruitTagsSelected", {
            details
                .get("tags")?
                .as_array()?
                .iter()
                .filter_map(|x| x.as_str())
                .join(", ")
                .unwrap_or_else(|| "none".to_owned())
        }),
        "RecruitTagsRefreshed" => info!("{}: {}", "RecruitTagsRefreshed", {
            let count = details.get("count")?.as_i64()?;
            format!("{} times", count)
        }),
        // RogueLike
        "StageInfo" => info!("{} {}", "StartCombat", details.get("name")?.as_str()?),
        "StageInfoError" => error!("{}", "StageInfoError"),
        "RoguelikeInvestment" => {
            let count = details.get("count")?.as_i64()?;
            let total = details.get("total")?.as_i64()?;
            let deposit = details.get("deposit")?.as_i64()?;

            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_roguelike_mut() {
                    detail.invest(count);
                }
            });

            info!("Deposit {count} / {total} / {deposit} originium ingots")
        }
        "RoguelikeSettlement" => {
            let exp = details.get("exp")?.as_i64()?;
            edit_current_task_detail(move |detail| {
                if let Some(detail) = detail.as_roguelike_mut() {
                    detail.set_exp(exp)
                }
            });
            info!("Gain {} exp during this exploration", exp);
        }

        // Copilot
        "BattleFormation" => info!(
            "{} {}",
            "BattleFormation",
            details
                .get("formation")?
                .as_array()?
                .iter()
                .filter_map(|x| x.as_str())
                .join(", ")
                .unwrap_or_else(|| "none".to_owned())
        ),
        "BattleFormationSelected" => info!(
            "{} {}",
            "BattleFormationSelected",
            details.get("selected")?.as_str()?
        ),
        "CopilotAction" => info!(
            "{} {} {}",
            "CurrentSteps",
            details.get("action")?.as_str()?,
            details.get("target")?.as_str()?,
        ),
        // SSS
        "SSSStage" => info!("{} {}", "CurrentStage", details.get("stage")?.as_str()?),
        "SSSSettlement" => info!("{} {}", "SSSSettlement", details.get("why")?.as_str()?),
        "SSSGamePass" => info!("{}", "SSSGamePass"),
        "UnsupportedLevel" => error!("{}", "UnsupportedLevel"),
        _ => {
            trace!(
                "{}: {}",
                "UnknownSubTaskExtraInfo",
                serde_json::to_string_pretty(message).unwrap()
            )
        }
    }

    Some(())
}

trait IterJoin: Iterator {
    fn join(&mut self, sep: &str) -> Option<String>
    where
        Self: Sized,
        Self::Item: std::fmt::Display,
    {
        self.next().map(|first_item| {
            // estimate lower bound of capacity needed
            let (lower, _) = self.size_hint();
            let mut result = String::with_capacity(sep.len() * lower);
            write!(&mut result, "{}", first_item).unwrap();
            self.for_each(|elt| {
                result.push_str(sep);
                write!(&mut result, "{}", elt).unwrap();
            });
            result
        })
    }
}

impl<I> IterJoin for I where I: Iterator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter_join() {
        assert_eq!([1, 2, 3].iter().join(","), Some("1,2,3".to_owned()));
        assert_eq!(Vec::<i32>::new().iter().join(","), None);
    }
}
