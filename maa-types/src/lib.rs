use core::panic;

use serde::Deserialize;
use thiserror::Error;

pub mod detail;

use detail::subtask::*;
use detail::taskchain::*;
use detail::*;

#[macro_export(local_inner_macros)]
macro_rules! enum_display {
    ($enum_name:ident,$($variant:ident),*) => {
        impl Display for $enum_name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        $enum_name::$variant => std::write!(f, std::stringify!($variant)),
                    )*
                }
            }
        }
    };
}

#[derive(Debug)]
pub enum MaybeDeserialized<T: for<'a> Deserialize<'a>> {
    Raw(String),
    Deserialized(T),
}

impl<T: for<'a> Deserialize<'a>> MaybeDeserialized<T> {
    pub fn get(json:&str) -> Self {
        match serde_json::from_str::<T>(json) {
            Ok(v) => MaybeDeserialized::Deserialized(v),
            Err(_) => MaybeDeserialized::Raw(json.to_string()),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            MaybeDeserialized::Raw(_json) => panic!("Cannot unwrap raw"),
            MaybeDeserialized::Deserialized(v) => v,
        }
    }
}

#[derive(Debug)]
pub enum AsstMessage {
    InternalError,
    InitFailed(InitFailedDetail),
    ConnectionInfo(ConnectionInfoDetail),
    AllTasksCompleted(AllTasksCompletedDetail),
    AsyncCallInfo(AsyncCallInfoDetail),
    TaskChainInfo(TaskChainDetail),
    TaskChainExtraInfo(TaskChainExtraInfoDetail),
    SubTaskInfo(SubTaskDetail),
    SubTaskExtraInfo(SubTaskExtraInfoDetail),
}

#[derive(Error, Debug)]
pub enum MessageParseError {
    #[error("Unknown message code: {0}")]
    UnknownMessageCodeError(i32),

    #[error("Failed to parse message details: {0}")]
    JsonParseError(#[from] serde_json::Error),
}

impl AsstMessage {
    pub fn get(msg: i32, details: &str) -> Result<Self, MessageParseError> {
        match msg {
            0 => Ok(AsstMessage::InternalError),
            1 => {
                let detail: InitFailedDetail = serde_json::from_str(details)?;
                Ok(AsstMessage::InitFailed(detail))
            }
            2 => {
                let detail: ConnectionInfoDetail = serde_json::from_str(details)?;
                Ok(AsstMessage::ConnectionInfo(detail))
            }
            3 => {
                let detail: AllTasksCompletedDetail = serde_json::from_str(details)?;
                Ok(AsstMessage::AllTasksCompleted(detail))
            }
            4 => {
                let detail: AsyncCallInfoDetail = serde_json::from_str(details)?;
                Ok(AsstMessage::AsyncCallInfo(detail))
            }
            10000 | 10001 | 10002 | 10004 => {
                let detail = TaskChainDetail::new(msg, details);
                Ok(AsstMessage::TaskChainInfo(detail))
            }
            10003 => {
                let detail: TaskChainExtraInfoDetail = serde_json::from_str(details)?;
                Ok(AsstMessage::TaskChainExtraInfo(detail))
            }
            20000 | 20001 | 20002 | 20004 => {
                let detail = SubTaskDetail::new(msg, details);
                Ok(AsstMessage::SubTaskInfo(detail))
            }
            20003 => {
                let detail: SubTaskExtraInfoDetail = serde_json::from_str(details)?;
                Ok(AsstMessage::SubTaskExtraInfo(detail))
            }
            _ => Err(MessageParseError::UnknownMessageCodeError(msg)),
        }
    }
}
