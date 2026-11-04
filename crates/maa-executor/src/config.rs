use std::path::PathBuf;

use maa_types::{TaskType, TouchMode};
use maa_value::prelude::MAAValue;

/// Build-time configuration for the process-wide MaaCore runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutorConfig {
    pub lib_path: PathBuf,
    pub state_dir: PathBuf,
    pub resources: Vec<PathBuf>,
    pub static_options: StaticOptions,
}

/// MaaCore options that must be applied before loading resources.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StaticOptions {
    pub cpu_ocr: Option<bool>,
    pub gpu_ocr: Option<u32>,
}

/// Options applied when creating an Assistant instance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InstanceOptions {
    pub touch_mode: Option<TouchMode>,
    pub deployment_with_pause: Option<bool>,
    pub adb_lite_enabled: Option<bool>,
    pub kill_adb_on_exit: Option<bool>,
}

/// Static connection inputs, optionally completed by an execution environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionConfig {
    pub adb_path: PathBuf,
    pub address: Option<String>,
    pub config: String,
}

/// Policies that affect one execution without changing its sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionOptions {
    pub auto_reconnect: bool,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
        }
    }
}

/// An ordered group of MaaCore tasks executed on the shared Assistant instance.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub name: Option<String>,
    pub tasks: Vec<MaaTask>,
}

/// A resolved MaaCore task.
#[derive(Clone, Debug, PartialEq)]
pub struct MaaTask {
    pub name: Option<String>,
    pub task_type: TaskType,
    pub params: MAAValue,
}

impl MaaTask {
    /// Returns the configured display name or the MaaCore task type.
    pub fn name_or_default(&self) -> &str {
        self.name
            .as_deref()
            .unwrap_or_else(|| self.task_type.to_str())
    }
}
