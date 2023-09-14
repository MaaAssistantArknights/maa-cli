use std::fmt::Formatter;
use std::fmt::Display;
use serde::Deserialize;
use serde_json::Value;

use crate::enum_display;

#[derive(Deserialize, Debug)]
pub enum TaskChain {
    StartUp,
    CloseDown,
    Fight,
    Mall,
    Recruit,
    Infrast,
    Award,
    Roguelike,
    Copilot,
    SSSCopilot,
    Depot,
    OperBox,
    ReclamationAlgorithm,
    Custom,
    SingleStep,
    VideoRecognition,
    Debug,
}

enum_display!(
    TaskChain,
    StartUp,
    CloseDown,
    Fight,
    Mall,
    Recruit,
    Infrast,
    Award,
    Roguelike,
    Copilot,
    SSSCopilot,
    Depot,
    OperBox,
    ReclamationAlgorithm,
    Custom,
    SingleStep,
    VideoRecognition,
    Debug
);

pub type TaskChainExtraInfoDetail = Value;

#[derive(Debug)]
pub enum TaskChainStatus {
    TaskChainError,
    TaskChainStart,
    TaskChainCompleted,
    TaskChainExtraInfo,
    TaskChainStopped,
}

enum_display!(
    TaskChainStatus,
    TaskChainError,
    TaskChainStart,
    TaskChainCompleted,
    TaskChainExtraInfo,
    TaskChainStopped
);

impl From<i32> for TaskChainStatus {
    fn from(value: i32) -> Self {
        match value {
            10000 => TaskChainStatus::TaskChainError,
            10001 => TaskChainStatus::TaskChainStart,
            10002 => TaskChainStatus::TaskChainCompleted,
            10004 => TaskChainStatus::TaskChainStopped,
            _ => panic!("Unknown TaskChainStatus: {}", value),
        }
    }
}

#[derive(Debug)]
pub struct TaskChainDetail {
    pub taskchain: TaskChain,
    pub uuid: String,
    pub status: TaskChainStatus,
    pub taskid: i32,
}

impl TaskChainDetail {
    pub fn new(msg: i32, detail: &str) -> Self {
        let status = TaskChainStatus::from(msg);
        let detail: Value = serde_json::from_str(detail).unwrap();
        let taskchain: TaskChain = serde_json::from_value(detail["taskchain"].clone()).unwrap();
        let uuid: String = serde_json::from_value(detail["uuid"].clone()).unwrap();
        let taskid: i32 = serde_json::from_value(detail["taskid"].clone()).unwrap();
        TaskChainDetail {
            taskchain,
            uuid,
            status,
            taskid,
        }
    }
}