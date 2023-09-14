use crate::{error, info, normal, trace, warning};

use std::fmt::Write;

use maa_sys::binding::AsstMsgId;
use maa_types::message::{AsstMessage, detail::{ConnectionInfoDetail, ConnectionInfoWhat, taskchain::{TaskChainDetail, TaskChain, TaskChainStatus}, subtask::{SubTaskDetail, SubTaskStatus, Task, ProcessTaskDetails}}};
use maa_types::message::detail::subtask::{SubTaskExtraInfoDetail, SubTaskExtraInfoDetails};

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
            error!("InitializationError");
        }
        AsstMessage::ConnectionInfo(info) => process_connection_info(info),
        AsstMessage::AllTasksCompleted(info) => {
            normal!("AllTasksCompleted");
        }
        AsstMessage::AsyncCallInfo(info) => {}
        AsstMessage::TaskChainInfo(info) => process_taskchain(info),
        AsstMessage::TaskChainExtraInfo(info) => {}
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
        AsstMessage::SubTaskExtraInfo(info) => {
            process_subtask_extra_info(info)
        }
    };
}

fn process_connection_info(info:ConnectionInfoDetail) {
    match info.what {
        ConnectionInfoWhat::Connected => info!("Connected"),
        ConnectionInfoWhat::UnsupportedResolution => error!("UnsupportedResolution"),
        ConnectionInfoWhat::ResolutionError => error!("ResolutionAcquisitionFailure"),
        ConnectionInfoWhat::Reconnecting => error!("TryToReconnect", {
            // TODO this is not yet implemented in maa_types
            // let times = message.get("times")?.as_i64()?;
            // format!("{} times", times + 1)
            format!("")
        }),
        ConnectionInfoWhat::Reconnected => normal!("ReconnectSuccess"),
        ConnectionInfoWhat::Disconnect => error!("Disconnected"),
        ConnectionInfoWhat::ScreencapFailed => error!("ScreencapFailed"),
        ConnectionInfoWhat::TouchModeNotAvailable => error!("TouchModeNotAvailable"),
        _ => {
            trace!(
                "UnknownConnectionInfo",
                format!("")
            );
        }
    }
}

fn process_taskchain(info: TaskChainDetail) {

    if matches!(info.taskchain,TaskChain::CloseDown) {
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

fn process_subtask_error(detail:ProcessTaskDetails) {

    match detail.task {
        Task::StartGameTask => error!("FailedToOpenClient"),
        Task::AutoRecruitTask => error!("FailedToRecruit"),
        Task::RecognizeDrops => error!("DropRecognitionError"),
        Task::ReportToPenguinStats => error!("GiveUpReporting"),
        Task::CheckStageValid => error!("TheEX"),
        _ => {
            trace!(
                "UnknownSubTaskError",
                format!("{:?}",detail)
            )
        }
    };

}
fn process_subtask_start(details:ProcessTaskDetails) {

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
            // TODO "AbandonAction" => error!("ActingCommandError"),
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
            // TODO Task::StageDreadfulFoe | Task::StageDreadfulFoe-5Enter => info!("DreadfulFoe"),
            Task::StageTraderInvestSystemFull => warning!("TraderInvestSystemFull"),
            /* TODO Task::OfflineConfirm => warning!("GameOffline"),
            Task::GamePass => info!("RoguelikeGamePass"),
            Task::BattleStartAll => info!("MissionStart"),
            Task::StageTraderSpecialShoppingAfterRefresh => {
                info!("RoguelikeSpecialItemBought")
            } */
            _ => {} // There are too many tasks to process, so we just ignore them
    }

}
fn process_subtask_completed(_:ProcessTaskDetails) {
}

fn process_subtask_extra_info(details:SubTaskExtraInfoDetail) {

    let details = details.details;

    match details{
        SubTaskExtraInfoDetails::StageDrops(drops) => info!("Drops", {
            let statistics = drops.stats;
            let mut all_drops: Vec<String> = Vec::new();
            for item in statistics {
                let drop = format!("{}: {} (+{})", item.item_name, item.quantity, item.add_quantity);
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
            format!(
                "{} #{}",
                detail.facility,
                detail.index
            )
        ),
        // TODO "ProductIncorrect" => error!("ProductIncorrect"),
        // TODO "ProductUnknown" => error!("ProductUnknown"),
        // TODO "ProductChanged" => info!("ProductChanged"),
        SubTaskExtraInfoDetails::NotEnoughStaff(detail) => error!("NotEnoughStaff"),
        // TODO "CustomInfrastRoomOperators" => info!(
        //    "CustomInfrastRoomOperators",
        //    details
        //        .get("names")?
        //        .as_array()?
        //        .iter()
        //        .map(|x| x.as_str().unwrap_or(""))
        //        .join(", ")
        // ),
        // Recruit
        SubTaskExtraInfoDetails::RecruitTagsDetected(tags) => info!(
            "RecruitResult:\n ",
            tags.tags.join("\n ")
        ),
        SubTaskExtraInfoDetails::RecruitSpecialTag(tag) => info!("RecruitingTips:", tag.tag),
        // TODO "RecruitRobotTag" => info!("RecruitingTips:", details.get("tag")?.as_str()?),
        SubTaskExtraInfoDetails::RecruitResult(result) => info!("RecruitResult", {"★".repeat(result.level as usize)}),
        SubTaskExtraInfoDetails::RecruitTagsSelected(tags) => info!("RecruitTagsSelected", {tags.tags.join(", ")}),
        SubTaskExtraInfoDetails::RecruitTagsRefreshed(tags) => info!("RecruitTagsRefreshed", {format!("{} times", tags.count)}),
        // RogueLike
        SubTaskExtraInfoDetails::StageInfo(info) => info!("StartCombat", info.name),
        SubTaskExtraInfoDetails::StageInfoError => error!("StageInfoError"),
        // TODO "BattleFormation" => info!("BattleFormation", details.get("formation")?.as_str()?),
        // TODO "BattleFormationSelected" => info!(
        //    "BattleFormationSelected",
        //    details.get("selected")?.as_str()?
        // ),
        /* TODO "CopilotAction" => info!("CurrentSteps", {
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
        } */
        _ => {}
    }
}
