pub mod subtask;
pub mod taskchain;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct InitFailedDetail {
    pub what: String,
    pub why: String,
    pub details: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConnectionInfoDetails {
    pub adb: String,
    pub address: String,
    pub config: String,
    pub times: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ConnectionInfoWhat {
    ConnectFailed,
    Connected,
    UuidGot,
    UnsupportedResolution,
    ResolutionError,
    Reconnecting,
    Reconnected,
    Disconnect,
    ScreencapFailed,
    TouchModeNotAvailable,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConnectionInfoDetail {
    pub what: ConnectionInfoWhat,
    pub why: String,
    pub uuid: String,
    pub details: ConnectionInfoDetails,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AllTasksCompletedDetail {
    pub chain: taskchain::TaskChain,
    pub uuid: String,
    pub tasks: Vec<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AsyncCallInfoDetails {
    pub ret: bool,
    pub cost: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AsyncCallInfoDetail {
    pub uuid: String,
    pub what: String,
    pub async_call_id: i32,
    pub details: AsyncCallInfoDetails,
}
