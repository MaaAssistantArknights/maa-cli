pub mod summary;
use std::{
    fmt::Write,
    io::Write as _,
    sync::{Arc, atomic::AtomicBool},
};

use log::{debug, error, info, trace, warn};
use maa_core::Callback;
use maa_types::{MessageKind, primitive::AsstTaskId};
use serde_json::{Map, Value};
use summary::{Facility, edit_current_task_detail, end_current_task, start_task};

use crate::state::AGENT;

pub static MAA_CORE_ERRORED: AtomicBool = AtomicBool::new(false);

fn json_pretty(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("<serialization error: {e}>"))
}

/// Render an event as a single line of JSON for `--emit-json` mode.
///
/// The shape is `{"kind": "<Kind>", "message": <payload>}`. The caller
/// supplies the kind as a string so the same helper can emit both MaaCore
/// callbacks (whose `MessageKind` is stringified via its `Debug` repr, e.g.
/// `"TaskChainStart"`, `"Unknown(9999)"`) and synthetic events such as
/// `"TaskRegistered"`. Returns `None` if the event fails to serialize (should
/// not happen in practice — `message` is a `Value` we just built).
pub(crate) fn format_event_line(kind: &str, message: &Value) -> Option<String> {
    let event = serde_json::json!({
        "kind": kind,
        "message": message,
    });
    serde_json::to_string(&event).ok()
}

/// Write one JSONL event to stdout under `--emit-json`, silently dropping the
/// line on write failure (e.g. broken pipe). Locks stdout for the duration.
pub(crate) fn emit_json_event(kind: &str, message: &Value) {
    if let Some(line) = format_event_line(kind, message) {
        let mut out = std::io::stdout().lock();
        // Ignore write errors: if stdout is closed (broken pipe), we silently
        // drop the line rather than propagate an error up through a MaaCore
        // callback thread or the task-registration loop.
        let _ = writeln!(out, "{line}");
    }
}

pub struct MaaCallback {
    auto_reconnect: bool,
    emit_json: bool,
    offline_stop_requested: Arc<AtomicBool>,
}

impl MaaCallback {
    pub fn new(auto_reconnect: bool, emit_json: bool) -> (Self, Arc<AtomicBool>) {
        let offline_stop_requested = Arc::new(AtomicBool::new(false));
        let cb = Self {
            auto_reconnect,
            emit_json,
            offline_stop_requested: Arc::clone(&offline_stop_requested),
        };
        (cb, offline_stop_requested)
    }
}

impl Callback for MaaCallback {
    fn on_message(&self, kind: MessageKind, msg: Option<&str>) {
        let Some(message) = msg else {
            log::warn!("Failed to retrieve message for kind {kind:?}");
            return;
        };
        let Some(message) = serde_json::from_str(message).ok() else {
            log::warn!("Failed to parse message for {kind:?}: {message}");
            return;
        };

        // Emit structured event on stdout for programmatic consumers before
        // running the human-facing processor. `process_message` is always
        // invoked afterwards because it carries side effects a driver still
        // needs — `MAA_CORE_ERRORED` for the exit status, the `summary::`
        // state for `--no-summary`, and the outbound `ReportRequest` HTTP
        // send in `process_report`.
        if self.emit_json {
            emit_json_event(&format!("{kind:?}"), &message);
        }

        self.process_message(kind, message);
    }
}

impl MaaCallback {
    fn process_message(&self, kind: MessageKind, message: Value) {
        let Some(message) = message.as_object() else {
            return;
        };

        use MessageKind::*;

        let ret = match kind {
            InternalError => Some(()),
            InitFailed => {
                error!("InitializationError");
                Some(())
            }
            ConnectionInfo => self.process_connection_info(message),
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
            | TaskChainStopped => self.process_taskchain(kind, message),

            SubTaskError => self.process_subtask_error(message),
            SubTaskStart => self.process_subtask_start(message),
            SubTaskCompleted => self.process_subtask_completed(message),
            SubTaskExtraInfo => self.process_subtask_extra_info(message),
            SubTaskStopped => Some(()),

            ReportRequest => self.process_report(message),

            Unknown(_) => None,
        };

        // if ret is None, which means the message is not processed well
        // we should print the message to trace the error
        if ret.is_none() {
            debug!(
                "FailedToProcessMessage, kind {kind:?}, message: {}",
                json_pretty(message)
            )
        }
    }

    fn process_connection_info(&self, message: &Map<String, Value>) -> Option<()> {
        let what = message.get("what")?.as_str()?;

        match what {
            "UuidGot" => debug!(
                "Got UUID: {}",
                message.get("details")?.get("uuid")?.as_str()?
            ),
            "ConnectFailed" => error!(
                "Failed to connect to android device, {}, Please check your connect configuration: {}",
                message.get("why")?.as_str()?,
                json_pretty(message.get("details")?)
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
            _ => trace!("{}: {}", "Unknown Connection Info", json_pretty(message)),
        }

        Some(())
    }

    fn process_taskchain(&self, kind: MessageKind, message: &Map<String, Value>) -> Option<()> {
        let taskchain = message.get("taskchain")?.as_str()?;

        use MessageKind::*;

        match kind {
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

    fn process_subtask_error(&self, message: &Map<String, Value>) -> Option<()> {
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
            _ => trace!("{}: {}", "UnknownSubTaskError", json_pretty(message)),
        };

        Some(())
    }

    fn process_subtask_start(&self, message: &Map<String, Value>) -> Option<()> {
        let subtask = message.get("subtask")?.as_str()?;

        if subtask == "ProcessTask" {
            let details = message.get("details")?.as_object()?;
            let task = details.get("task")?.as_str()?;

            match task {
                "StartButton2" | "AnnihilationConfirm" => {
                    edit_current_task_detail(|detail| {
                        if let Some(detail) = detail.as_fight_mut()
                            && let Some((series, sanity)) = detail.get_series()
                        {
                            info!("Mission started ({series} times, use {sanity} sanity)");
                            detail.start();
                            return;
                        }
                        info!("Mission started");
                    });
                }
                // Fight
                "StoneConfirm" => {
                    let exec_times = details.get("exec_times")?.as_i64()?;
                    edit_current_task_detail(|detail| {
                        if let Some(detail) = detail.as_fight_mut() {
                            detail.set_stone(exec_times)
                        }
                    });
                    info!("Use {exec_times} stones");
                }
                "AbandonAction" => warn!("{}", "PRTS error"),
                // Recruit
                "RecruitRefreshConfirm" => {
                    edit_current_task_detail(|detail| {
                        if let Some(detail) = detail.as_recruit_mut() {
                            detail.refresh()
                        }
                    });
                    info!("{}", "Refresh Tags")
                }
                "RecruitConfirm" => {
                    edit_current_task_detail(|detail| {
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
                    edit_current_task_detail(|detail| {
                        if let Some(detail) = detail.as_roguelike_mut() {
                            detail.start_exploration()
                        }
                    });
                    info!("Start exploration {exec_times} times")
                }
                "ExitThenAbandon" => {
                    edit_current_task_detail(|detail| {
                        if let Some(detail) = detail.as_roguelike_mut() {
                            detail.set_state(summary::ExplorationState::Abandoned)
                        }
                    });
                    info!("Exploration Abandoned")
                }
                "ExitThenConfirm" => info!("{}", "ExplorationConfirmed"),
                "MissionCompletedFlag" => info!("{}", "MissionCompleted"),
                "MissionFailedFlag" => {
                    // In some cases a failed mission doesn't mean failed exploration;
                    // if the exploration was not failed, its state would be overwritten later
                    if message.get("taskchain")?.as_str()? == "Roguelike" {
                        edit_current_task_detail(|detail| {
                            if let Some(detail) = detail.as_roguelike_mut() {
                                detail.set_state(summary::ExplorationState::Failed)
                            }
                        });
                    }
                    info!("MissionFailed")
                }
                "StageTraderEnter" => info!("{}", "StageTraderEnter"),
                "StageSafeHouseEnter" => info!("{}", "StageSafeHouseEnter"),
                "StageCombatOpsEnter" | "StageCombatDpsEnter" => info!("{}", "StageCombatOpsEnter"),
                "StageEmergencyOps" | "StageEmergencyDps" => info!("{}", "EmergencyOpsEnter"),
                "StageDreadfulFoe" | "StageDreadfulFoe-5Enter" => info!("{}", "DreadfulFoe"),
                "StageTraderInvestSystemFull" => warn!("{}", "TraderInvestSystemFull"),
                "GamePass" => info!("{}", "RoguelikeGamePass"),
                "OfflineConfirm" => {
                    warn!("{}", "GameOffline");
                    if !self.auto_reconnect {
                        warn!("Auto reconnect disabled, stopping");
                        self.offline_stop_requested
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                "BattleStartAll" => info!("{}", "MissionStart"),
                "StageTraderSpecialShoppingAfterRefresh" => {
                    info!("{}", "RoguelikeSpecialItemBought")
                }
                _ => trace!("{}: {}", "UnknownSubTaskStart", json_pretty(message)),
            }
        }

        Some(())
    }

    fn process_subtask_completed(&self, _: &Map<String, Value>) -> Option<()> {
        Some(())
    }

    fn process_subtask_extra_info(&self, message: &Map<String, Value>) -> Option<()> {
        let taskchain = message.get("taskchain")?.as_str()?;

        match taskchain {
            "Depot" => info!("{}: {}", "Depot", json_pretty(message)),
            "OperBox" => info!("{}: {}", "OperBox", json_pretty(message)),
            _ => {}
        }

        let what = message.get("what")?.as_str()?;
        let details = message.get("details")?;

        match what {
            "FightTimes" => {
                let series = details.get("series")?.as_i64()?;
                let sanity_cost = details.get("sanity_cost")?.as_i64()?;
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.set_series(series, sanity_cost);
                    }
                });
            }
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
                        .map(|(item, count)| format!("{item} × {count}"))
                        .join(", ")
                        .unwrap_or_else(|| "none".to_owned())
                );

                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.push_drop(all_drops);
                    }
                });

                let stage = details.get("stage")?.get("stageCode")?.as_str()?;
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.set_stage(stage);
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
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.use_medicine(count, is_expiring);
                    }
                });

                if is_expiring {
                    info!("Use {count} expiring medicine");
                } else {
                    info!("Use {count} medicine");
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
                let facility = details.get("facility")?.as_str()?;
                let index = details.get("index")?.as_i64()?;
                let product = details.get("product")?.as_str()?;

                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_infrast_mut() {
                        detail.set_product(
                            facility.parse().unwrap_or(Facility::Unknown),
                            index,
                            product,
                        );
                    }
                });

                info!("{}: {}", "ProductOfFacility", product)
            }
            "CustomInfrastRoomOperators" => {
                let facility = details.get("facility")?.as_str()?;
                let index = details.get("index")?.as_i64()?;
                let operators = details.get("names")?.as_array()?;
                let candidates = details.get("candidates")?.as_array()?;

                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_infrast_mut() {
                        detail.set_operators(
                            facility.parse().unwrap_or(Facility::Unknown),
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
            "RecruitSpecialTag" | "RecruitRobotTag" => {
                info!("{}: {}", "RecruitingTips", details.get("tag")?.as_str()?)
            }
            "RecruitResult" => {
                let level = details.get("level")?.as_u64()?;
                let tags = details.get("tags")?.as_array()?;

                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_recruit_mut() {
                        detail.push_recruit(
                            level,
                            tags.iter().filter_map(|x| x.as_str().map(|x| x.to_owned())),
                        );
                    }
                });

                info!(
                    "{}: {} {}",
                    "RecruitResult",
                    "★".repeat(level as usize),
                    tags.iter()
                        .filter_map(|x| x.as_str())
                        .join(", ")
                        .unwrap_or_else(|| "none".to_owned())
                )
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
                format!("{count} times")
            }),
            // RogueLike
            "StageInfo" => info!("{} {}", "StartCombat", details.get("name")?.as_str()?),
            "StageInfoError" => error!("{}", "StageInfoError"),
            "RoguelikeInvestment" => {
                let count = details.get("count")?.as_i64()?;
                let total = details.get("total")?.as_i64()?;
                let deposit = details.get("deposit")?.as_i64()?;

                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_roguelike_mut() {
                        detail.invest(count);
                    }
                });

                info!("Deposit {count} / {total} / {deposit} originium ingots")
            }
            "RoguelikeSettlement" => {
                let exp = details.get("exp")?.as_i64()?;
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_roguelike_mut() {
                        detail.set_exp(exp)
                    }
                });
                info!("Gain {exp} exp during this exploration");
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
            _ => trace!("{}: {}", "UnknownSubTaskExtraInfo", json_pretty(message)),
        }

        Some(())
    }

    fn process_report(&self, message: &Map<String, Value>) -> Option<()> {
        let subtask = message.get("subtask")?.as_str()?;
        let url = message.get("url")?.as_str()?;
        let body = message.get("body")?.as_str()?;
        let headers = message.get("headers")?.as_object()?;

        info!("{subtask}: {url}");

        let mut request = AGENT.post(url).content_type("application/json");

        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(key, value_str);
            }
        }

        match request.send(body) {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    info!("Successfully {subtask}");
                } else {
                    warn!("Failed to {subtask}: HTTP {status}");
                }
            }
            Err(e) => warn!("Failed to {subtask}: {e}"),
        }

        Some(())
    }
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
            write!(&mut result, "{first_item}").unwrap();
            self.for_each(|elt| {
                result.push_str(sep);
                write!(&mut result, "{elt}").unwrap();
            });
            result
        })
    }
}

impl<I> IterJoin for I where I: Iterator {}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn iter_join() {
        assert_eq!([1, 2, 3].iter().join(","), Some("1,2,3".to_owned()));
        assert_eq!(Vec::<i32>::new().iter().join(","), None);
    }

    fn offline_confirm_msg() -> &'static str {
        r#"{"taskchain":"Fight","subtask":"ProcessTask","details":{"task":"OfflineConfirm"}}"#
    }

    #[test]
    fn offline_confirm_stops_when_auto_reconnect_disabled() {
        let (cb, offline_stop) = MaaCallback::new(false, false);
        cb.on_message(MessageKind::SubTaskStart, Some(offline_confirm_msg()));
        assert!(offline_stop.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn offline_confirm_does_not_stop_when_auto_reconnect_enabled() {
        let (cb, offline_stop) = MaaCallback::new(true, false);
        cb.on_message(MessageKind::SubTaskStart, Some(offline_confirm_msg()));
        assert!(!offline_stop.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn format_event_line_known_kind() {
        let kind = MessageKind::TaskChainStart;
        let message: Value = serde_json::from_str(r#"{"taskchain":"Fight","taskid":42}"#).unwrap();
        let line = format_event_line(&format!("{kind:?}"), &message).unwrap();
        let parsed: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(
            parsed.get("kind").and_then(Value::as_str),
            Some("TaskChainStart")
        );
        assert_eq!(
            parsed
                .get("message")
                .and_then(|m| m.get("taskchain"))
                .and_then(Value::as_str),
            Some("Fight"),
        );
        assert_eq!(
            parsed
                .get("message")
                .and_then(|m| m.get("taskid"))
                .and_then(Value::as_i64),
            Some(42),
        );
        // The whole event is a single JSON line, no embedded newlines.
        assert!(!line.contains('\n'));
    }

    #[test]
    fn format_event_line_unknown_kind() {
        let kind = MessageKind::Unknown(9999);
        let message: Value = serde_json::from_str("{}").unwrap();
        let line = format_event_line(&format!("{kind:?}"), &message).unwrap();
        let parsed: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(
            parsed.get("kind").and_then(Value::as_str),
            Some("Unknown(9999)"),
        );
    }

    #[test]
    fn format_event_line_task_registered() {
        // Synthetic event emitted by `run_core` for each task registered with
        // MaaCore. Shape mirrors the JSON built in `run/mod.rs`.
        let message = serde_json::json!({
            "id": 42,
            "name": "Daily Fight",
            "task_type": "Fight",
            "params": { "stage": "1-7" },
        });
        let line = format_event_line("TaskRegistered", &message).unwrap();
        let parsed: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(
            parsed.get("kind").and_then(Value::as_str),
            Some("TaskRegistered"),
        );
        // The task id MaaCore assigned at registration; consumers match it
        // against the `taskid` carried by `TaskChainStart`.
        assert_eq!(
            parsed
                .get("message")
                .and_then(|m| m.get("id"))
                .and_then(Value::as_i64),
            Some(42),
        );
        assert_eq!(
            parsed
                .get("message")
                .and_then(|m| m.get("name"))
                .and_then(Value::as_str),
            Some("Daily Fight"),
        );
        assert_eq!(
            parsed
                .get("message")
                .and_then(|m| m.get("task_type"))
                .and_then(Value::as_str),
            Some("Fight"),
        );
        assert_eq!(
            parsed
                .get("message")
                .and_then(|m| m.get("params"))
                .and_then(|p| p.get("stage"))
                .and_then(Value::as_str),
            Some("1-7"),
        );
        assert!(!line.contains('\n'));
    }
}
