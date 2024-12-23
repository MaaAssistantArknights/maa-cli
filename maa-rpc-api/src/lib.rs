use jsonrpsee::{core::SubscriptionResult, proc_macros::rpc, types::error::ErrorCode};
use maa_types::{primitive::AsstTaskId, TaskType};
use serde_json::Value;

#[cfg_attr(all(feature = "server", not(feature = "client")), rpc(server))]
#[cfg_attr(all(feature = "client", not(feature = "server")), rpc(client))]
#[cfg_attr(all(feature = "client", feature = "server"), rpc(client, server))]
pub trait Rpc {
    #[method(name = "load_core")]
    /// Load (lib)MaaCore
    ///
    /// Currently, the path to the core can only by set at the server side for security reasons.
    /// In the future, this method may be extended to allow clients to specify the path.
    /// To make sure the library is trusted, the server may need to be signed by a trusted
    /// key, and a public key should be specified at the server side.
    async fn load_core(&self) -> Result<(), ErrorCode>;

    #[method(name = "unload_core")]
    /// Unload (lib)MaaCore.
    async fn unload_core(&self) -> Result<(), ErrorCode>;

    #[method(name = "set_log_dir")]
    async fn set_log_dir(&self, log_dir: String) -> Result<(), ErrorCode>;

    #[method(name = "append_task")]
    /// Append a task to task list.
    async fn append_task(
        &self,
        task_type: TaskType,
        task_params: Value,
        process_params: Vec<&str>,
    ) -> Result<AsstTaskId, ErrorCode>;

    #[method(name = "set_task_params")]
    /// Set task parameters for task with given `task_id`.
    async fn set_task_params(
        &self,
        task_id: AsstTaskId,
        task_params: Value,
        process_params: Vec<&str>,
    ) -> Result<(), ErrorCode>;

    #[method(name = "start_tasks")]
    /// Start task with given `id`
    async fn start_task(&self, id: AsstTaskId) -> Result<(), ErrorCode>;

    #[method(name = "stop_tasks")]
    async fn stop_tasks(&self) -> Result<(), ErrorCode>;

    #[method(name = "asst_state")]
    /// Check if any task is running.
    async fn asst_state(&self) -> Result<bool, ErrorCode>;

    #[method(name = "log")]
    /// Write a log message to server log.
    fn log(&self, level: log::Level, message: &str) -> Result<(), ErrorCode>;

    #[subscription(name = "subscribe_log", item = LogMessage)]
    /// Get log messages from server.
    ///
    /// If `raw` is `true`, return raw log messages. Otherwise, return log processed by server.
    async fn subscribe_log(&self, raw: bool) -> SubscriptionResult;
}

pub enum State {
    /// MaaCore is not loaded.
    Unloaded,
    /// MaaCore is loaded but not initialized.
    Uninitialized,
    /// MaaCore is initialized but not connected to the device.
    Unconnected,
    /// MaaCore is available to run tasks.
    Idle,
    /// MaaCore is actively running tasks.
    Running,
}

#[derive(Debug)]
#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(feature = "server", derive(serde::Deserialize))]
/// A type representing a log message.
struct LogMessage {
    timestamp: chrono::DateTime<chrono::Utc>,
    level: log::Level,
    message: String,
}
