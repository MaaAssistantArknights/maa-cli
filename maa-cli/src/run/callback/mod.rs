pub mod summary;
use summary::{edit_current_task_detail, end_current_task, start_task};

use std::fmt::Write;

use maa_sys::binding::{AsstMsgId, AsstTaskId};
use serde_json::{Map, Value};

pub unsafe extern "C" fn default_callback(
    code: maa_sys::binding::AsstMsgId,
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
            error!("init-failed");
            Some(())
        }
        ConnectionInfo => process_connection_info(message),
        AllTasksCompleted => {
            info!("all-tasks-completed");
            Some(())
        }
        AsyncCallInfo => Some(()),

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
            "failed-process-message",
            code = code,
            message = serde_json::to_string_pretty(message).unwrap(),
        )
    }
}

fn process_connection_info(message: &Map<String, Value>) -> Option<()> {
    let what = message.get("what")?.as_str()?;

    match what {
        // Resulution
        "ResulutionGot" => info!(
            "got-resolution",
            width = message.get("details")?.get("width")?.as_i64()?,
            height = message.get("details")?.get("height")?.as_i64()?
        ),
        "ResolutionError" => error!("failed-get-resolution"),
        "UnsupportedResolution" => {
            let details = message.get("details")?.as_object()?;
            let width = details.get("width")?.as_i64()?;
            let height = details.get("height")?.as_i64()?;

            match message.get("why")?.as_str()? {
                "low-screen-resolution" => {
                    error!("low-screen-resolution", width = width, height = height)
                }
                "not-16-9" => error!("not-16-9", width = width, height = height),
                s => error!(
                    "unsupported-resolution",
                    why = s,
                    width = width,
                    height = height
                ),
            };
        }

        // Connection
        "Connected" => info!(
            "connected",
            address = message.get("details")?.get("address")?.as_str()?
        ),
        "Disconnect" => warn!("disconnected"),
        "Reconnecting" => warn!(
            "reconnecting",
            times = message.get("details")?.get("times")?.as_i64()?,
        ),
        "Reconnected" => info!("reconnected"),

        // Screen Capture
        "ScreencapFailed" => error!("failed-screencap"),
        "FastestWayToScreencap" => info!(
            "fastest-way-screencap",
            method = message.get("details")?.get("method")?.as_str()?,
            cost = message.get("details")?.get("cost")?.as_i64()?,
        ),
        "ScreencapCost" => trace!(
            "screencap-cost",
            min = message.get("details")?.get("min")?.as_i64()?,
            max = message.get("details")?.get("max")?.as_i64()?,
            avg = message.get("details")?.get("avg")?.as_i64()?,
        ),

        "TouchModeNotAvailable" => error!("touch-mode-not-available"),
        _ => {
            trace!(
                "unknown-connection-info",
                message = serde_json::to_string_pretty(message).unwrap()
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
            info!("taskchain-start", name = taskchain);
            start_task(message.get("taskid")?.as_i64()? as AsstTaskId);
        }
        TaskChainCompleted => {
            info!("taskchain-completed", name = taskchain);
            end_current_task(summary::Reason::Completed);
        }
        TaskChainStopped => {
            warn!("taskchain-stopped", name = taskchain);
            end_current_task(summary::Reason::Stopped);
        }
        TaskChainError => {
            error!("taskchain-error", name = taskchain);
            end_current_task(summary::Reason::Error);
        }
        TaskChainExtraInfo => {}

        _ => {} // unreachable
    };

    Some(())
}

fn process_subtask_error(message: &Map<String, Value>) -> Option<()> {
    let subtask = message.get("subtask")?.as_str()?;

    match subtask {
        "StartGameTask" => error!("failed-start-game"),
        "AutoRecruitTask" => warn!("failed-auto-recruit", why = message.get("why")?.as_str()?),
        "RecognizeDrops" => warn!("failed-recognize-drops"),
        "ReportToPenguinStats" => warn!(
            "failed-report-penguinstats",
            why = message.get("why")?.as_str()?,
        ),
        "ReportToYituliu" => warn!("failed-report-yituliu", why = message.get("why")?.as_str()?),
        "CheckStageValid" => warn!(
            "invalid-stage-for-recognition",
            why = message.get("why")?.as_str()?
        ),
        _ => trace!(
            "unknown-subtask-error",
            message = serde_json::to_string_pretty(message).unwrap()
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
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.set_times(exec_times);
                    }
                });
                info!("mission-start-times", times = exec_times);
            }
            "MedicineConfirm" => {
                let exec_times = details.get("exec_times")?.as_i64()?;
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.set_medicine(exec_times)
                    }
                });
                info!("medicine-used", times = exec_times);
            }
            "StoneConfirm" => {
                let exec_times = details.get("exec_times")?.as_i64()?;
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_fight_mut() {
                        detail.set_stone(exec_times)
                    }
                });
                info!("stone-used", times = exec_times);
            }
            "AbandonAction" => warn!("prts-error"),
            // Recruit
            "RecruitRefreshConfirm" => {
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_recruit_mut() {
                        detail.refresh()
                    }
                });
                info!("recruit-refresh");
            }
            "RecruitConfirm" => {
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_recruit_mut() {
                        detail.recruit()
                    }
                });
                info!("recruit-confirm");
            }
            // Infrast
            "InfrastDormDoubleConfirmButton" => warn!("infrast-dorm-double-confirm"),
            // RogueLike
            "StartExplore" => {
                let exec_times = details.get("exec_times")?.as_i64()?;
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_roguelike_mut() {
                        detail.set_times(exec_times)
                    }
                });
                info!("roguelike-start", times = exec_times);
            }
            "ExitThenAbandon" => info!("roguelike-abandon"),
            "MissionCompletedFlag" => info!("mission-complete"),
            "MissionFailedFlag" => info!("mission-failed"),
            "StageTraderEnter" => info!("trader-enter"),
            "StageSafeHouseEnter" => info!("safe-house-enter"),
            "StageCambatDpsEnter" => info!("normal-dps-enter"),
            "StageEmergencyDps" => info!("emergency-dps-enter"),
            "StageDreadfulFoe" | "StageDreadfulFoe-5Enter" => info!("dreadful-foe-enter"),
            "StageTraderInvestConfirm" => {
                let exec_times = details.get("exec_times")?.as_i64()?;
                edit_current_task_detail(|detail| {
                    if let Some(detail) = detail.as_roguelike_mut() {
                        detail.set_invest(exec_times)
                    }
                });
                info!("invest", times = exec_times);
            }
            "StageTraderInvestSystemFull" => warn!("invest-full"),

            "StageTraderSpecialShoppingAfterRefresh" => info!("special-item-bought"),
            "GamePass" => info!("roguelike-complete"),

            "OfflineConfirm" => warn!("game-offline"),
            "BattleStartAll" => info!("mission-start"),
            _ => trace!(
                "unknown-subtask-start",
                message = serde_json::to_string_pretty(message).unwrap()
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

    let details = message.get("details")?.as_object()?;

    match taskchain {
        "Depot" => info!(
            "depot-recognition",
            result = serde_json::to_string_pretty(details).unwrap(),
        ),
        "OperBox" => info!(
            "operator-recognition",
            result = serde_json::to_string_pretty(details).unwrap(),
        ),
        _ => {}
    }

    let what = message.get("what")?.as_str()?;

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
                "drops",
                drops = all_drops
                    .iter()
                    .map(|(item, count)| format!("{} × {}", item, count))
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
        "SanityBeforeStage" => {
            info!(
                "sanity-before-stage",
                sanity = details.get("current_sanity")?.as_i64()?,
                max = details.get("max_sanity")?.as_i64()?,
            );
        }

        // Infrast
        "EnterFacility" => debug!(
            "facility-enter",
            facility = details.get("facility")?.as_facility()?,
            index = details.get("index")?.as_i64()?
        ),
        "ProductOfFacility" => {
            let facility = details.get("facility")?.as_str()?.parse().unwrap();
            let index = details.get("index")?.as_i64()?;
            let product = details.get("product")?.as_str()?.parse().unwrap();

            edit_current_task_detail(|detail| {
                if let Some(detail) = detail.as_infrast_mut() {
                    detail.set_product(facility, index, product);
                }
            });
            debug!(
                "product-of-facility",
                facility = facility,
                index = index,
                product = product
            );
        }
        "ProductIncorrect" => {
            let facility = details.get("facility")?.as_str()?;
            let index = details.get("index")?.as_i64()?;
            let product = details.get("product")?.as_str()?;

            warn!(
                "product-incorrect",
                facility = facility,
                index = index,
                product = product,
            );
        }
        "ProductChanged" => {
            let facility = details.get("facility")?.as_facility()?;
            let index = details.get("index")?.as_i64()?;
            let product = details.get("product")?.as_product()?;

            warn!(
                "product-changed",
                facility = facility,
                index = index,
                product = product,
            );

            //  Set product to the new product
            edit_current_task_detail(|detail| {
                if let Some(detail) = detail.as_infrast_mut() {
                    detail.set_product(facility, index, product);
                }
            });
        }
        "NotEnoughStaff" => error!(
            "not-enough-staff",
            facility = details.get("facility")?.as_facility()?,
            index = details.get("index")?.as_i64()?
        ),
        "CustomInfrastRoomOperators" => {
            let facility = details.get("facility")?.as_facility()?;
            let index = details.get("index")?.as_i64()?;
            let operators = details
                .get("names")?
                .as_array()?
                .into_iter()
                .filter_map(|x| x.as_str().map(|x| x.to_owned()))
                .collect::<Vec<_>>();
            let candidates = details
                .get("candidates")?
                .as_array()?
                .into_iter()
                .filter_map(|x| x.as_str().map(|x| x.to_owned()))
                .collect::<Vec<_>>();

            match (operators.is_empty(), candidates.is_empty()) {
                (true, true) => return Some(()),
                (true, false) => info!(
                    "custom-infrast-candidates",
                    facility = facility,
                    index = index,
                    candidates = candidates.iter().join(", ").unwrap()
                ),
                (false, true) => info!(
                    "custom-infrast-operators",
                    facility = facility,
                    index = index,
                    operators = operators.iter().join(", ").unwrap()
                ),
                (false, false) => info!(
                    "custom-infrast-both",
                    facility = facility,
                    index = index,
                    operators = operators.iter().join(", ").unwrap(),
                    candidates = candidates.iter().join(", ").unwrap()
                ),
            }

            edit_current_task_detail(|detail| {
                if let Some(detail) = detail.as_infrast_mut() {
                    detail.set_operators(facility, index, operators, candidates);
                }
            });
        }

        // Recruit
        "RecruitTagsDetected" => (), // this info is contained in RecruitResult, so ignore it
        "RecruitSpecialTag" => info!("recruit-special-tag", tag = details.get("tag")?.as_str()?),
        "RecruitRobotTag" => info!("recruit-robot-tag", tag = details.get("tag")?.as_str()?),
        "RecruitResult" => {
            let level = details.get("level")?.as_u64()?;
            let tags = details
                .get("tags")?
                .as_array()?
                .into_iter()
                .filter_map(|x| x.as_str().map(|x| x.to_owned()))
                .collect::<Vec<_>>();

            info!(
                "recruit-tags",
                star = level,
                tags = tags.iter().join(", ").unwrap_or_else(|| "none".to_owned())
            );

            edit_current_task_detail(|detail| {
                if let Some(detail) = detail.as_recruit_mut() {
                    detail.push_recruit(level, tags);
                }
            });
        }
        "RecruitTagsSelected" => info!(
            "recruit-tags-selected",
            tags = details
                .get("tags")?
                .as_array()?
                .iter()
                .filter_map(|x| x.as_str())
                .join(", ")
                .unwrap_or_else(|| "none".to_owned())
        ),
        "RecruitTagsRefreshed" => {} // see RecruitRefreshConfirm
        "RecruitNoPermit" => warn!("recruit-no-permit"),
        // RogueLike
        "RoguelikeSettlement" => {
            let difficulty = details.get("difficulty")?.as_str()?;
            let pass = details.get("pass")?.as_bool()?;
            let explore = details.get("explore")?.as_i64()?;
            let steps = details.get("steps")?.as_i64()?;
            let combat = details.get("combat")?.as_i64()?;
            let emergency = details.get("emergency")?.as_i64()?;
            let boss = details.get("boss")?.as_i64()?;
            let recruit = details.get("recruit")?.as_i64()?;
            let object = details.get("object")?.as_i64()?;
            let score = details.get("score")?.as_i64()?;
            let exp = details.get("exp")?.as_i64()?;
            let skill = details.get("skill")?.as_i64()?;

            // TODO: add to summary

            let pass = if pass {
                fl!("roguelike-pass")
            } else {
                fl!("roguelike-fail")
            };

            info!(
                "roguelike-settlement",
                difficulty = difficulty,
                pass = pass,
                explore = explore,
                steps = steps,
                combat = combat,
                emergency = emergency,
                boss = boss,
                recruit = recruit,
                object = object,
                score = score,
                exp = exp,
                skill = skill,
            );
        }
        "StageInfo" => info!(
            "roguelike-stage-enter",
            name = details.get("name")?.as_str()?
        ),
        "StageInfoError" => error!("roguelike-stage-info-error"),
        "RoguelikeEvent" => info!("roguelike-event", name = details.get("name")?.as_str()?),
        // Copilot
        "BattleFormation" => info!(
            "battle-formation",
            formation = serde_json::to_string_pretty(details.get("formation")?).unwrap()
        ),
        "BattleFormationSelected" => info!(
            "battle-formation-selected",
            selected = details.get("selected")?.as_str()?
        ),
        // TODO: localize action, there is enum in MaaCore
        "CopilotAction" => info!(
            "current-copilot-action",
            action = details.get("action")?.as_action()?,
            target = details.get("target")?.as_str().unwrap_or(""),
            doc = details.get("doc")?.as_str().unwrap_or(""),
        ),
        // SSS
        "SSSStage" => info!("sss-stage-enter", name = details.get("stage")?.as_str()?),
        "SSSSettlement" => info!("sss-settlement", why = details.get("why")?.as_str()?),
        "SSSGamePass" => info!("sss-game-pass"),
        "UnsupportedLevel" => error!("unsupported-level"),
        _ => {
            trace!(
                "unknown-subtask-extra-info",
                message = serde_json::to_string_pretty(message).unwrap()
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

trait AsFacility {
    fn as_facility(&self) -> Option<summary::Facility>;
}

impl AsFacility for Value {
    fn as_facility(&self) -> Option<summary::Facility> {
        self.as_str().and_then(|s| s.parse().ok())
    }
}

trait AsProduct {
    fn as_product(&self) -> Option<summary::Product>;
}

impl AsProduct for Value {
    fn as_product(&self) -> Option<summary::Product> {
        self.as_str().and_then(|s| s.parse().ok())
    }
}

trait AsAction {
    fn as_action(&self) -> Option<ActionType>;
}

impl AsAction for Value {
    fn as_action(&self) -> Option<ActionType> {
        self.as_str().and_then(|s| s.parse().ok())
    }
}

enum ActionType {
    Deploy,
    UseSkill,
    Retreat,
    SwitchSpeed,
    BulletTime,
    SkillUsage,
    Output,
    SkillDaemon,

    /* 引航者试炼 */
    MoveCamera,
    /* 保全派驻 */
    DrawCard,
    CheckIfStartOver,

    Unknown(String),
}

impl ActionType {
    fn to_fl_string(&self) -> String {
        use ActionType::*;
        match self {
            Deploy => fl!("Deploy"),
            UseSkill => fl!("UseSkill"),
            Retreat => fl!("Retreat"),
            SwitchSpeed => fl!("SwitchSpeed"),
            BulletTime => fl!("BulletTime"),
            SkillUsage => fl!("SkillUsage"),
            Output => fl!("Output"),
            SkillDaemon => fl!("SkillDaemon"),
            MoveCamera => fl!("MoveCamera"),
            DrawCard => fl!("DrawCard"),
            CheckIfStartOver => fl!("CheckIfStartOver"),
            Unknown(s) => s.clone(),
        }
    }
}

impl std::str::FromStr for ActionType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Deploy" | "DEPLOY" | "deploy" | "部署" => Ok(ActionType::Deploy),
            "Skill" | "SKILL" | "skill" | "技能" => Ok(ActionType::UseSkill),
            "Retreat" | "RETREAT" | "retreat" | "撤退" => Ok(ActionType::Retreat),
            "SpeedUp" | "SPEEDUP" | "Speedup" | "speedup" | "二倍速" | "SwitchSpeed" => {
                Ok(ActionType::SwitchSpeed)
            }
            "BulletTime" | "BULLETTIME" | "Bullettime" | "bullettime" | "子弹时间" => {
                Ok(ActionType::BulletTime)
            }
            "SkillUsage" | "SKILLUSAGE" | "Skillusage" | "skillusage" | "技能用法" => {
                Ok(ActionType::SkillUsage)
            }
            "Output" | "OUTPUT" | "output" | "输出" | "打印" => Ok(ActionType::Output),
            "SkillDaemon" | "skilldaemon" | "SKILLDAEMON" | "Skilldaemon" | "DoNothing"
            | "摆完挂机" | "开摆" => Ok(ActionType::SkillDaemon),
            "MoveCamera" | "movecamera" | "MOVECAMERA" | "Movecamera" | "移动镜头" => {
                Ok(ActionType::MoveCamera)
            }
            "DrawCard" | "drawcard" | "DRAWCARD" | "Drawcard" | "抽卡" | "抽牌" | "调配"
            | "调配干员" => Ok(ActionType::DrawCard),
            "CheckIfStartOver" | "Checkifstartover" | "CHECKIFSTARTOVER" | "checkifstartover"
            | "检查重开" => Ok(ActionType::CheckIfStartOver),
            s => Ok(ActionType::Unknown(s.to_string())),
        }
    }
}

impl<'source> From<ActionType> for fluent_bundle::FluentValue<'source> {
    fn from(value: ActionType) -> Self {
        Self::String(value.to_fl_string().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter_join() {
        assert_eq!([1, 2, 3].iter().join(","), Some("1,2,3".to_owned()));
        assert_eq!(Vec::<i32>::new().iter().join(","), None);
    }
}
