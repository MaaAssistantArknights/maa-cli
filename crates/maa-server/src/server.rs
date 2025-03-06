pub unsafe extern "C" fn default_callback(
    code: maa_types::primitive::AsstMsgId,
    json_raw: *const std::ffi::c_char,
    session_id: *mut std::ffi::c_void,
) {
    use log::Logger;
    let code: maa_server::callback::AsstMsg = code.into();
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_raw).to_str().unwrap() };
    let session_id: SessionIDRef = unsafe {
        std::ffi::CStr::from_ptr(session_id as *mut _ as *mut std::ffi::c_char)
            .to_str()
            .unwrap()
    };

    tracing::trace!("Session ID: {}", session_id);

    if let Some(tx) = log::TX_HANDLERS.read().get(session_id) {
        if tx.log(json_str.to_string()) {
            log::TX_HANDLERS.write().remove(session_id);
        }
    } else {
        Logger::log_to_pool(json_str);
    }

    let map: callback::Map = serde_json::from_str(json_str).unwrap();

    // if ret is None, which means the message is not processed well
    // we should print the message to trace the error
    if callback::process_message(code, map).is_none() {
        tracing::debug!(
            "FailedToProcessMessage, code: {:?}, message: {}",
            code,
            json_str
        )
    }
}

type SessionID = String;
/// an ugly way to make up
type SessionIDRef<'a> = &'a str;

type Uuid = String;
type UuidRef<'a> = &'a str;

use maa_types::primitive::AsstTaskId as TaskId;

mod callback {
    use maa_server::callback::AsstMsg;
    use tracing::{debug, error, info, trace, warn};

    use crate::{SessionID, TaskId};

    pub type Map = serde_json::Map<String, serde_json::Value>;
    pub fn process_message(code: AsstMsg, message: Map) -> Option<()> {
        use AsstMsg::*;

        match code {
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
            | TaskChainStopped => process_taskchain(code, message),

            SubTaskError | SubTaskStart | SubTaskCompleted | SubTaskExtraInfo | SubTaskStopped => {
                subtask::process_subtask(code, message)
            }

            Unknown => None,
        }
    }

    fn process_connection_info(message: Map) -> Option<()> {
        #[derive(serde::Deserialize)]
        struct ConnectionInfo {
            what: String,
            why: Option<String>,
            #[serde(rename = "uuid")]
            _uuid: SessionID,
            details: Map,
        }
        let ConnectionInfo {
            what,
            why,
            _uuid,
            details,
        } = serde_json::from_value(serde_json::Value::Object(message)).unwrap();

        match what.as_str() {
            "UuidGot" => debug!("Got UUID: {}", details.get("uuid")?.as_str()?),
            "ConnectFailed" => error!(
                "Failed to connect to android device, {}, Please check your connect configuration: {}",
                why.unwrap(),serde_json::to_string_pretty(&details).unwrap()
            ),
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
                    "{}: what:{} why:{} detials:{}",
                    "Unknown Connection Info",
                    what, why.as_deref().unwrap_or("No why"),
                    serde_json::to_string_pretty(&details).unwrap()
                ),
        }

        Some(())
    }

    fn process_taskchain(code: AsstMsg, message: Map) -> Option<()> {
        #[derive(serde::Deserialize)]
        struct TaskChain {
            taskchain: maa_types::TaskType,
            taskid: TaskId,
            uuid: SessionID,
        }
        let TaskChain {
            taskchain,
            taskid,
            uuid,
        } = serde_json::from_value(serde_json::Value::Object(message)).unwrap();

        use crate::state::{Reason, StatePool};
        use AsstMsg::*;

        match code {
            TaskChainStart => {
                info!("{} {}", taskchain, "Start");
                StatePool::reason_task(&uuid, taskid, Reason::Start);
            }
            TaskChainCompleted => {
                info!("{} {}", taskchain, "Completed");
                StatePool::reason_task(&uuid, taskid, Reason::Complete);
            }
            TaskChainStopped => {
                warn!("{} {}", taskchain, "Stopped");
                StatePool::reason_task(&uuid, taskid, Reason::Cancel);
            }
            TaskChainError => {
                error!("{} {}", taskchain, "Error");
                StatePool::reason_task(&uuid, taskid, Reason::Error);
            }
            TaskChainExtraInfo => {}

            _ => {} // unreachable
        };

        Some(())
    }

    #[cfg(any())]
    mod subtask {
        use super::*;

        pub fn process_subtask(code: AsstMsg, message: Map) -> Option<()> {
            match code {
                AsstMsg::SubTaskError => process_subtask_error(message),
                AsstMsg::SubTaskStart => process_subtask_start(message),
                AsstMsg::SubTaskCompleted => process_subtask_completed(message),
                AsstMsg::SubTaskExtraInfo => process_subtask_extra_info(message),
                AsstMsg::SubTaskStopped => Some(()),
                _ => unreachable!(),
            }
        }

        fn process_subtask_start(message: Map) -> Option<()> {
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
                    "StageTraderSpecialShoppingAfterRefresh" => {
                        info!("{}", "RoguelikeSpecialItemBought")
                    }
                    _ => trace!(
                        "{}: {}",
                        "UnknownSubTaskStart",
                        serde_json::to_string_pretty(message).unwrap()
                    ),
                }
            }

            Some(())
        }

        fn process_subtask_completed(_: Map) -> Option<()> {
            Some(())
        }

        fn process_subtask_error(message: Map) -> Option<()> {
            let subtask = message.get("subtask")?.as_str()?;

            match subtask {
                "StartGameTask" => error!("Failed To Start Game"),
                "AutoRecruitTask" => error!("{} {}", message.get("why")?.as_str()?, "Has Returned"),
                "RecognizeDrops" => error!("Failed To Recognize Drops"),
                "ReportToPenguinStats" => error!(
                    "{}, {}",
                    "Failed To Report To Penguin Stats",
                    message.get("why")?.as_str()?,
                ),
                "CheckStageValid" => error!("TheEX"),
                _ => debug!(
                    "{}: {}",
                    "Unknown SubTask Error",
                    serde_json::to_string_pretty(&message).unwrap()
                ),
            };

            Some(())
        }

        fn process_subtask_extra_info(message: Map) -> Option<()> {
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
                "RecruitSpecialTag" => {
                    info!("{}: {}", "RecruitingTips", details.get("tag")?.as_str()?)
                }
                "RecruitRobotTag" => {
                    info!("{}: {}", "RecruitingTips", details.get("tag")?.as_str()?)
                }
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
    }

    #[cfg(all())]
    mod subtask {
        use super::*;

        pub fn process_subtask(_code: AsstMsg, message: Map) -> Option<()> {
            #[derive(serde::Deserialize)]
            struct SubTask {
                uuid: String,
                taskid: TaskId,
            }
            let msg = serde_json::to_string_pretty(&message).unwrap();
            let SubTask { uuid, taskid } =
                serde_json::from_value(serde_json::Value::Object(message)).unwrap();
            crate::state::StatePool::update_task(&uuid, taskid, msg);
            Some(())
        }
    }
}

mod state {
    use crate::{SessionID, SessionIDRef, TaskId};
    use parking_lot::RwLock;
    use std::collections::BTreeMap;

    type Pool = RwLock<BTreeMap<SessionID, BTreeMap<TaskId, TaskState>>>;
    static STATE_POOL: Pool = StatePool::new();

    #[derive(Debug, Clone)]
    pub enum TaskState {
        NotStarted,
        Running(Vec<String>),
        Completed(Vec<String>),
        Canceled(Vec<String>),
        Error(Vec<String>),
    }

    impl std::fmt::Display for TaskState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    TaskState::NotStarted => "Not Started".to_owned(),
                    TaskState::Running(items) => format!("Running\n{}", items.join("\n")),
                    TaskState::Completed(items) => format!("Completed\n{}", items.join("\n")),
                    TaskState::Canceled(items) => format!("Canceled\n{}", items.join("\n")),
                    TaskState::Error(items) => format!("Error\n{}", items.join("\n")),
                }
            )
        }
    }

    pub enum Reason {
        Start,
        Complete,
        Cancel,
        Error,
    }

    impl TaskState {
        pub fn reason(&mut self, reason: Reason) {
            match reason {
                Reason::Start => {
                    let TaskState::NotStarted = self else {
                        unreachable!()
                    };
                    *self = TaskState::Running(vec![]);
                }
                reason => {
                    let TaskState::Running(items) = self else {
                        unreachable!()
                    };
                    let vec = std::mem::take(items);
                    match reason {
                        Reason::Start => unreachable!(),
                        Reason::Complete => std::mem::replace(self, TaskState::Completed(vec)),
                        Reason::Cancel => std::mem::replace(self, TaskState::Canceled(vec)),
                        Reason::Error => std::mem::replace(self, TaskState::Error(vec)),
                    };
                }
            }
        }
        pub fn update(&mut self, new: String) {
            let TaskState::Running(items) = self else {
                unreachable!()
            };
            items.push(new);
        }
    }

    pub struct StatePool;

    impl StatePool {
        const fn new() -> Pool {
            RwLock::new(BTreeMap::new())
        }

        pub fn new_task(uuid: SessionID, id: TaskId) {
            STATE_POOL
                .write()
                .entry(uuid)
                .or_default()
                .insert(id, TaskState::NotStarted);
        }

        pub fn reason_task(uuid: SessionIDRef, id: TaskId, reason: Reason) {
            STATE_POOL
                .write()
                .get_mut(uuid)
                .map(|map| map.get_mut(&id))
                .flatten()
                .map(|state| state.reason(reason));
        }

        pub fn update_task(uuid: SessionIDRef, id: TaskId, new: String) {
            STATE_POOL
                .write()
                .get_mut(uuid)
                .map(|map| map.get_mut(&id))
                .flatten()
                .map(|state| state.update(new));
        }

        pub fn get_uuid(uuid: SessionIDRef) -> BTreeMap<TaskId, String> {
            STATE_POOL
                .read()
                .get(uuid)
                .map(|taskstate| taskstate.iter().map(|(&k, v)| (k, v.to_string())).collect())
                .unwrap_or_default()
        }
    }
}

mod log {
    use crate::{SessionID, SessionIDRef};
    use parking_lot::RwLock;
    use std::collections::BTreeMap;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    static LOG_POOL: RwLock<BTreeMap<String, Vec<String>>> = RwLock::new(BTreeMap::new());

    pub fn get_skip_len(uuid: SessionIDRef, len: i32) -> Vec<String> {
        LOG_POOL
            .read()
            .get(uuid)
            .iter()
            .flat_map(|vec| vec.iter())
            .skip(len as usize)
            .cloned()
            .collect()
    }

    /// will be used in callback,
    /// which is out of tokio runtime
    pub static TX_HANDLERS: RwLock<BTreeMap<SessionID, crate::log::Logger<String>>> =
        RwLock::new(BTreeMap::new());

    pub struct Logger<T> {
        tx: UnboundedSender<T>,
        rx: Option<UnboundedReceiver<T>>,
    }

    impl Logger<String> {
        pub fn new() -> Self {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            Self { tx, rx: Some(rx) }
        }
        pub fn take_rx(&mut self) -> Option<UnboundedReceiver<String>> {
            self.rx.take()
        }
        /// if true, the channel is closed, so drop this
        pub fn log(&self, message: String) -> bool {
            Self::log_to_pool(&message);
            // log to global log pool
            self.tx.send(message).is_err()
        }
        pub fn uuid(message: &str) -> SessionID {
            #[derive(serde::Deserialize)]
            struct DeSer {
                uuid: SessionID,
                #[serde(flatten)]
                __extra: BTreeMap<String, serde_json::Value>,
            }
            let value: DeSer = serde_json::from_str(&message).unwrap_or(DeSer {
                uuid: "()".to_owned(),
                __extra: Default::default(),
            });
            value.uuid
        }
        pub fn log_to_pool(message: &str) {
            let uuid = Self::uuid(message);
            LOG_POOL
                .write()
                .entry(uuid)
                .or_default()
                .push(message.to_owned());
        }
    }
}

mod task {
    use maa_server::{
        task::{task_server::TaskServer, task_state::State, *},
        tonic::{self, Request, Response},
    };
    use std::collections::BTreeMap;
    use tokio::sync::RwLock;

    use crate::{log::TX_HANDLERS, SessionID};

    /// build service under package task
    ///
    /// ### Note:
    ///
    /// In order to trace and sync client, an additional header `SESSION_KEY` is needed.
    ///
    /// Client get one by calling [`Task::new_connection`], and destory by calling [`Task::close_connection`]
    ///
    /// ### Usage:
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let addr = "[::1]:10000".parse().unwrap();
    ///
    ///     let svc = task::gen_service();
    ///
    ///     Server::builder().add_service(svc).serve(addr).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`Task::new_connection`]: task_server::Task::new_connection
    /// [`Task::close_connection`]: task_server::Task::close_connection
    pub fn gen_service() -> TaskServer<TaskImpl> {
        TaskServer::new(TaskImpl)
    }

    mod wrapper {
        use tokio::sync::Notify;

        use crate::SessionIDRef;

        /// A wrapper for [`maa_sys::Assistant`]
        ///
        /// The inner can be [Send] but not [Sync],
        /// because every fn related is actually a `ref mut` rather `ref`,
        /// which may cause data race
        ///
        /// By using [Notify], only one request can reach handler at a time
        /// and there should be no data racing
        pub struct Assistant {
            inner: maa_sys::Assistant,
            lock: Notify,
        }

        unsafe impl Sync for Assistant {}

        impl Assistant {
            pub fn new(session_id: SessionIDRef) -> Self {
                let instance = Self {
                    inner: maa_sys::Assistant::new(
                        Some(crate::default_callback),
                        Some(
                            std::ffi::CString::new(session_id).unwrap().into_raw() as *mut _
                                as *mut std::ffi::c_void,
                        ),
                    ),
                    lock: Notify::new(),
                };
                instance.lock.notify_one();
                instance
            }

            pub async fn wait(&self) -> &maa_sys::Assistant {
                self.lock.notified().await;
                self.lock.notify_one();
                self.inner_unchecked()
            }

            pub fn inner_unchecked(&self) -> &maa_sys::Assistant {
                &self.inner
            }
        }

        pub trait SessionExt {
            fn get_session_id(&self) -> tonic::Result<UUIDWrapper>;
        }

        impl<T> SessionExt for tonic::Request<T> {
            fn get_session_id(&self) -> tonic::Result<UUIDWrapper> {
                self.metadata()
                    .get("x-session-key")
                    .ok_or(tonic::Status::not_found("session_id is not found"))?
                    .to_str()
                    .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                    .inspect(|uuid| tracing::trace!("tracking uuid: {uuid}"))
                    .map(UUIDWrapper)
            }
        }

        impl SessionExt for tonic::metadata::MetadataMap {
            fn get_session_id(&self) -> tonic::Result<UUIDWrapper> {
                self.get("x-session-key")
                    .ok_or(tonic::Status::not_found("session_id is not found"))?
                    .to_str()
                    .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                    .map(UUIDWrapper)
            }
        }

        pub struct UUIDWrapper<'a>(SessionIDRef<'a>);

        impl<'a> UUIDWrapper<'a> {
            pub async fn func_with<T>(
                &self,
                f: impl FnOnce(&maa_sys::Assistant) -> T,
            ) -> tonic::Result<T> {
                let read_lock = super::TASK_HANDLERS.read().await;

                let handler = read_lock
                    .get(self.0)
                    .ok_or(tonic::Status::not_found("session_id is not found"))?;

                Ok(f(handler.wait().await))
            }
            pub fn into_inner(self) -> SessionIDRef<'a> {
                self.0
            }
        }
    }

    use wrapper::{Assistant, SessionExt};

    static TASK_HANDLERS: RwLock<BTreeMap<SessionID, Assistant>> =
        RwLock::const_new(BTreeMap::new());

    pub struct TaskImpl;

    type Ret<T> = tonic::Result<Response<T>>;

    #[tonic::async_trait]
    impl task_server::Task for TaskImpl {
        #[tracing::instrument(skip_all)]
        async fn new_connection(&self, req: Request<NewConnectionRequst>) -> Ret<String> {
            let NewConnectionRequst { conncfg, instcfg } = req.into_inner();

            let session_id = uuid::Uuid::now_v7().to_string();

            let asst = Assistant::new(&session_id);
            tracing::debug!("Instance Created");

            if let Some(message) =
                instcfg.and_then(|cfg| cfg.apply_to(asst.inner_unchecked()).err())
            {
                return Err(tonic::Status::internal(message));
            }

            let (adb_path, address, config) = conncfg.unwrap().connect_args();
            asst.inner_unchecked()
                .async_connect(adb_path.as_str(), address.as_str(), config.as_str(), true)
                .unwrap();

            TX_HANDLERS
                .write()
                .insert(session_id.clone(), crate::log::Logger::new());
            TASK_HANDLERS.write().await.insert(session_id.clone(), asst);

            Ok(Response::new(session_id))
        }

        #[tracing::instrument(skip_all)]
        async fn close_connection(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?.into_inner();

            Ok(Response::new(
                TASK_HANDLERS.write().await.remove(session_id).is_some()
                    && TX_HANDLERS.write().remove(session_id).is_some(),
            ))
        }

        #[tracing::instrument(skip_all)]
        async fn append_task(&self, req: Request<NewTaskRequest>) -> Ret<TaskId> {
            let (
                meta,
                _,
                NewTaskRequest {
                    task_type,
                    task_params,
                },
            ) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let task_type: TaskType = task_type.try_into().unwrap();
            let task_type: maa_types::TaskType = task_type.into();

            let ret = session_id
                .func_with(|handler| handler.append_task(task_type, task_params.as_str()))
                .await?;

            match ret {
                Ok(id) => {
                    crate::state::StatePool::new_task(session_id.into_inner().to_owned(), id);
                    Ok(Response::new(id.into()))
                }
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn modify_task(&self, req: Request<ModifyTaskRequest>) -> Ret<bool> {
            let (
                meta,
                _,
                ModifyTaskRequest {
                    task_id,
                    task_params,
                },
            ) = req.into_parts();

            let task_id = task_id
                .ok_or(tonic::Status::invalid_argument("no task_id is given"))?
                .into();

            let session_id = meta.get_session_id()?;

            let ret = session_id
                .func_with(|handler| handler.set_task_params(task_id, task_params.as_str()))
                .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn active_task(&self, req: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let ret = session_id
                .func_with(|handler| {
                    handler.set_task_params(task_id.into(), r#"{ "enable": true }"#)
                })
                .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn deactive_task(&self, req: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let ret = session_id
                .func_with(|handler| {
                    handler.set_task_params(task_id.into(), r#"{ "enable": false }"#)
                })
                .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn start_tasks(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?;

            let ret = session_id.func_with(|handler| handler.start()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn stop_tasks(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?;

            let ret = session_id.func_with(|handler| handler.stop()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        type TaskStateUpdateStream = std::pin::Pin<
            Box<dyn tokio_stream::Stream<Item = tonic::Result<TaskState>> + Send + 'static>,
        >;

        #[tracing::instrument(skip_all)]
        async fn task_state_update(&self, req: Request<()>) -> Ret<Self::TaskStateUpdateStream> {
            let session_id = req.get_session_id()?.into_inner();

            let Some(rx) = TX_HANDLERS
                .write()
                .get_mut(session_id)
                .and_then(|logger| logger.take_rx())
            else {
                return Err(tonic::Status::resource_exhausted("rx has been taken"));
            };

            use tokio_stream::StreamExt as _;
            let streaming = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(|msg| {
                let state = if !maa_sys::binding::loaded() {
                    State::Unloaded
                } else {
                    State::Idle
                };
                let mut st = TaskState::default();
                st.set_state(state);
                st.content = msg;
                Ok(st)
            });

            Ok(Response::new(Box::pin(streaming)))
        }

        #[tracing::instrument(skip_all)]
        async fn fetch_logs(&self, req: Request<i32>) -> Ret<LogArray> {
            let (meta, _, skip) = req.into_parts();

            let session_id = meta.get_session_id()?.into_inner();

            let logs = crate::log::get_skip_len(session_id, skip);

            Ok(Response::new(LogArray {
                items: logs
                    .into_iter()
                    .map(|log| TaskState {
                        content: log,
                        state: 0,
                    })
                    .collect(),
            }))
        }
    }
}

mod core {
    use maa_server::{
        core::{core_server::CoreServer, *},
        tonic::{self, Request, Response},
    };

    /// build service under package core
    ///
    /// ### Usage:
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let addr = "[::1]:10000".parse().unwrap();
    ///
    ///     let svc = core::gen_service();
    ///
    ///     Server::builder().add_service(svc).serve(addr).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn gen_service() -> CoreServer<CoreImpl> {
        CoreServer::new(CoreImpl)
    }

    pub struct CoreImpl;

    type Ret<T> = tonic::Result<Response<T>>;

    #[tonic::async_trait]
    impl core_server::Core for CoreImpl {
        async fn load_core(&self, req: Request<CoreConfig>) -> Ret<bool> {
            // no await, span here won't cause out of bound
            let _span = tracing::trace_span!("Load Core");
            let _entered = _span.enter();

            let CoreConfig {
                static_ops,
                log_ops,
            } = req.into_inner();

            if maa_sys::binding::loaded() {
                tracing::debug!("MaaCore already loaded, skiping Core load");
                // using false here to info the client that core is already loaded
                return Ok(Response::new(false));
            }

            let ret = maa_server::utils::load_core();

            if let Some(core_config::LogOptions { level, name }) = log_ops {
                use maa_dirs::Ensure;
                // Todo: set log level for tracing
                let _ = level;
                maa_sys::Assistant::set_user_dir(
                    maa_dirs::state()
                        .join(name)
                        .as_path()
                        .ensure()
                        .map_err(|e| tonic::Status::from_error(Box::new(e)))?,
                )
                .unwrap();
            }

            if let Some(core_config::StaticOptions { cpu_ocr, gpu_ocr }) = static_ops {
                use maa_sys::{Assistant, StaticOptionKey};
                match (cpu_ocr, gpu_ocr) {
                    (cpu_ocr, Some(gpu_id)) => {
                        if cpu_ocr {
                            tracing::warn!(
                                "Both CPU OCR and GPU OCR are enabled, CPU OCR will be ignored"
                            );
                        }
                        tracing::debug!("Using GPU OCR with GPU ID {}", gpu_id);
                        if Assistant::set_static_option(StaticOptionKey::GpuOCR, gpu_id).is_err() {
                            return Err(tonic::Status::internal(format!(
                                "Failed to enable GPU OCR with GPU ID {}",
                                gpu_id
                            )));
                        }
                    }
                    (true, None) => {
                        tracing::debug!("Using CPU OCR");
                        if Assistant::set_static_option(StaticOptionKey::CpuOCR, true).is_err() {
                            return Err(tonic::Status::internal("Failed to enable CPU OCR"));
                        }
                    }
                    (false, None) => {}
                }
            }

            if maa_server::utils::ResourceConfig::default().load().is_err() {
                return Err(tonic::Status::internal("Failed to load resources"));
            }

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::unknown(e)),
            }
        }

        async fn unload_core(&self, _: Request<()>) -> Ret<bool> {
            // no await, span here won't cause out of bound
            let _span = tracing::trace_span!("Unload Core");
            let _entered = _span.enter();

            maa_sys::binding::unload();

            Ok(Response::new(true))
        }
    }
}

// #[tokio::main(flavor = "current_thread")]
#[tokio::main]
async fn main() {
    use tonic::transport::Server;
    use tracing_subscriber::{filter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

    tracing_subscriber::Registry::default()
        .with(
            fmt::layer()
                .compact()
                .with_ansi(true)
                .with_filter(filter::LevelFilter::DEBUG),
        )
        .init();

    let using_socket = true;

    if using_socket {
        use tokio::net::UnixListener;
        use tokio_stream::wrappers::UnixListenerStream;

        let path = "/tmp/tonic/testing.sock";
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();

        let socket = UnixListener::bind(path).unwrap();
        let stream = UnixListenerStream::new(socket);
        Server::builder()
            .add_service(task::gen_service())
            .add_service(core::gen_service())
            .serve_with_incoming(stream)
            .await
    } else {
        Server::builder()
            .add_service(task::gen_service())
            .add_service(core::gen_service())
            .serve("127.0.0.1:50051".parse().unwrap())
            .await
    }
    .unwrap();
}
