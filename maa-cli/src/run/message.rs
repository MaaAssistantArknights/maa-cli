use crate::{error, info, normal, trace, warning};

use maa_sys::binding::AsstMsgId;
use maa_types::MaybeDeserialized;
use maa_types::detail::subtask::{SubTaskExtraInfoDetail, SubTaskExtraInfoDetails};
use maa_types::{
    detail::{
        subtask::{ProcessTaskDetails, SubTaskDetail, SubTaskStatus, Task},
        taskchain::{TaskChain, TaskChainDetail, TaskChainStatus},
        ConnectionInfoDetail, ConnectionInfoWhat,
    },
    AsstMessage,
};

pub unsafe extern "C" fn callback(
    code: AsstMsgId,
    json_raw: *const std::os::raw::c_char,
    _: *mut std::os::raw::c_void,
) {
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_raw).to_str().unwrap() };
    process_message(code, json_str);
}

pub fn process_message(code: AsstMsgId, json: &str) {
    let message = AsstMessage::get(code, &json);
    if let Err(e) = message {
        error!("MessageParseError", e);
        return;
    }
    let message = message.unwrap();
    match message {
        AsstMessage::InternalError => {}
        AsstMessage::InitFailed(e) => {
            error!("InitializationError",e.details);
        }
        AsstMessage::ConnectionInfo(info) => process_connection_info(info),
        AsstMessage::AllTasksCompleted(_) => {
            normal!("AllTasksCompleted");
        }
        AsstMessage::AsyncCallInfo(_) => {}
        AsstMessage::TaskChainInfo(info) => process_taskchain(info),
        AsstMessage::TaskChainExtraInfo(_) => {}
        AsstMessage::SubTaskInfo(info) => {
            // keep the if let here, because we may add more subtask types in the future
            #[allow(irrefutable_let_patterns)]
            if let SubTaskDetail::ProcessTask { status, details } = info {
                match status {
                    SubTaskStatus::SubTaskError => {
                        process_subtask_error(details);
                    }
                    SubTaskStatus::SubTaskStart => {
                        process_subtask_start(details);
                    }
                    SubTaskStatus::SubTaskCompleted => {
                        process_subtask_completed(details);
                    }
                    SubTaskStatus::SubTaskStopped => {}
                }
            }
        }
        AsstMessage::SubTaskExtraInfo(info) => process_subtask_extra_info(info),
    };
}

fn process_connection_info(info: ConnectionInfoDetail) {
    match info.what {
        ConnectionInfoWhat::Connected => info!("Connected"),
        ConnectionInfoWhat::UnsupportedResolution => error!("UnsupportedResolution"),
        ConnectionInfoWhat::ResolutionError => error!("ResolutionAcquisitionFailure"),
        ConnectionInfoWhat::Reconnecting => error!("TryToReconnect", {
            format!("{} times", info.details.times.unwrap() + 1)
        }),
        ConnectionInfoWhat::Reconnected => normal!("ReconnectSuccess"),
        ConnectionInfoWhat::Disconnect => error!("Disconnected"),
        ConnectionInfoWhat::ScreencapFailed => error!("ScreencapFailed"),
        ConnectionInfoWhat::TouchModeNotAvailable => error!("TouchModeNotAvailable"),
        _ => {
            trace!(
                "UnknownConnectionInfo",
                format!("{}", serde_json::to_string_pretty(&info).unwrap())
            );
        }
    }
}

fn process_taskchain(info: TaskChainDetail) {
    if matches!(info.taskchain, TaskChain::CloseDown) {
        return;
    }

    match info.status {
        TaskChainStatus::TaskChainError => error!("TaskError", info.taskchain),
        TaskChainStatus::TaskChainStart => normal!("StartTask", info.taskchain),
        TaskChainStatus::TaskChainCompleted => normal!("CompleteTask", info.taskchain),
        TaskChainStatus::TaskChainStopped => warning!("TaskChainStopped", info.taskchain),
        TaskChainStatus::TaskChainExtraInfo => {}
    };
}

fn process_subtask_error(detail: ProcessTaskDetails) {
    match detail.task {
        Task::StartGameTask => error!("FailedToOpenClient"),
        Task::AutoRecruitTask => error!("FailedToRecruit"),
        Task::RecognizeDrops => error!("DropRecognitionError"),
        Task::ReportToPenguinStats => error!("GiveUpReporting"),
        Task::CheckStageValid => error!("TheEX"),
        _ => {
            trace!(
                "UnknownSubTaskError",
                format!("{}", serde_json::to_string_pretty(&detail).unwrap())
            )
        }
    };
}
fn process_subtask_start(details: ProcessTaskDetails) {
    match details.task {
            Task::StartButton2 /* TODO | Task::AnnihilationConfirm */ => info!(
                "MissionStart",
                format!("{} times", details.exec_times)
            ),
            Task::MedicineConfirm => info!(
                "MedicineUsed",
                format!("{} times", details.exec_times)
            ),
            Task::StoneConfirm => info!(
                "StoneUsed",
                format!("{} times", details.exec_times)
            ),
            Task::AbandonAction => error!("ActingCommandError"),
            Task::RecruitRefreshConfirm => info!("LabelsRefreshed"),
            Task::RecruitConfirm => info!("RecruitConfirm"),
            Task::InfrastDormDoubleConfirmButton => error!("InfrastDormDoubleConfirmed"),
            // RogueLike
            Task::StartExplore => info!(
                "BegunToExplore",
                format!("{} times", details.exec_times)
            ),
            Task::StageTraderInvestConfirm => info!(
                "HasInvested",
                format!("{} times", details.exec_times)
            ),
            // TODO: process more instead of just printing
            Task::ExitThenAbandon => info!("ExplorationAbandoned"),
            Task::MissionCompletedFlag => info!("MissionCompleted"),
            Task::MissionFailedFlag => info!("MissionFailed"),
            Task::StageTraderEnter => info!("StageTraderEnter"),
            Task::StageSafeHouseEnter => info!("StageSafeHouseEnter"),
            Task::StageCambatDpsEnter => info!("CambatDpsEnter"),
            Task::StageEmergencyDps => info!("EmergencyDpsEnter"),
            Task::StageDreadfulFoe | Task::StageDreadfulFoe5Enter => info!("DreadfulFoe"),
            Task::StageTraderInvestSystemFull => warning!("TraderInvestSystemFull"),
            Task::OfflineConfirm => warning!("GameOffline"),
            Task::GamePass => info!("RoguelikeGamePass"),
            Task::BattleStartAll => info!("MissionStart"),
            Task::StageTraderSpecialShoppingAfterRefresh => {
                info!("RoguelikeSpecialItemBought")
            }
            _ => {} // There are too many tasks to process, so we just ignore them
    }
}
fn process_subtask_completed(_: ProcessTaskDetails) {}

fn process_subtask_extra_info(details: SubTaskExtraInfoDetail) {
    let details = details.details;

    if let MaybeDeserialized::Raw(raw) = details {
        trace!("RawSubTaskExtraInfo", raw);
        return;
    }

    let details = details.unwrap();

    match details {
        SubTaskExtraInfoDetails::StageDrops(drops) => info!("Drops", {
            let statistics = drops.stats;
            let mut all_drops: Vec<String> = Vec::new();
            for item in statistics {
                let drop = format!(
                    "{}: {} (+{})",
                    item.item_name, item.quantity, item.add_quantity
                );
                all_drops.push(drop);
            }
            if !all_drops.is_empty() {
                all_drops.join(", ")
            } else {
                String::from("none")
            }
        }),
        // Infrast
        SubTaskExtraInfoDetails::EnterFacility(detail) => info!(
            "EnterFacility",
            format!("{} #{}", detail.facility, detail.index)
        ),
        SubTaskExtraInfoDetails::ProductIncorrect => error!("ProductIncorrect"),
        SubTaskExtraInfoDetails::ProductUnknown => error!("ProductUnknown"),
        SubTaskExtraInfoDetails::ProductChanged => info!("ProductChanged"),
        SubTaskExtraInfoDetails::NotEnoughStaff(_detail) => error!("NotEnoughStaff"),
        SubTaskExtraInfoDetails::CustomInfrastRoomOperators(names) => {
            info!("CustomInfrastRoomOperators", names.names.join(", "))
        }
        // Recruit
        SubTaskExtraInfoDetails::RecruitTagsDetected(tags) => {
            info!("RecruitResult:\n ", tags.tags.join("\n "))
        }
        SubTaskExtraInfoDetails::RecruitSpecialTag(tag) => info!("RecruitingTips:", tag.tag),
        SubTaskExtraInfoDetails::RecruitRobotTag(tag) => info!("RecruitingTips:", tag.tag),
        SubTaskExtraInfoDetails::RecruitResult(result) => {
            info!("RecruitResult", { "★".repeat(result.level as usize) })
        }
        SubTaskExtraInfoDetails::RecruitTagsSelected(tags) => {
            info!("RecruitTagsSelected", { tags.tags.join(", ") })
        }
        SubTaskExtraInfoDetails::RecruitTagsRefreshed(tags) => {
            info!("RecruitTagsRefreshed", { format!("{} times", tags.count) })
        }
        // RogueLike
        SubTaskExtraInfoDetails::StageInfo(info) => info!("StartCombat", info.name),
        SubTaskExtraInfoDetails::StageInfoError => error!("StageInfoError"),
        SubTaskExtraInfoDetails::BattleFormation(formation) => info!("BattleFormation", formation.formation),
        SubTaskExtraInfoDetails::BattleFormationSelected(selected) => info!(
            "BattleFormationSelected",
            selected.selected
        ),
        SubTaskExtraInfoDetails::CopilotAction(detail) => info!("CurrentSteps", {
            format!(
                "{} {}",
                detail.action,
                detail.target,
            )
        }),
        // TODO SSS
        // "SSSStage" => info!("CurrentStage", details.get("stage")?.as_str()?),
        // "SSSSettlement" => info!("SSSSettlement", details.get("why")?.as_str()?),
        // "SSSGamePass" => info!("SSSGamePass"),
        // "UnsupportedLevel" => error!("UnsupportedLevel"),
        _ => {
            trace!(
                "UnknownSubTaskExtraInfo",
                serde_json::to_string_pretty(&details).unwrap()
            )
        },
    }
}
