#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

#[macro_use]
mod enum_macros;

mod client_type;
mod task_type;
mod touch_mode;

pub mod primitive {
    /// Boolean type for assistant API.
    pub type AsstBool = u8;
    /// Size type for assistant API.
    pub type AsstSize = u64;

    /// Id type for assistant API.
    pub type AsstId = i32;
    /// Message id type for assistant API.
    pub type AsstMsgId = AsstId;
    /// Task id type for assistant API.
    pub type AsstTaskId = AsstId;
    /// Async call id type for assistant API.
    pub type AsstAsyncCallId = AsstId;

    /// Option key type for assistant API.
    pub type AsstOptionKey = i32;
    /// Static option key type for assistant API.
    pub type AsstStaticOptionKey = AsstOptionKey;
    /// Instance option key type for assistant API.
    pub type AsstInstanceOptionKey = AsstOptionKey;
}

/// Available static option key
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum StaticOptionKey {
    /// set to true to enable CPU OCR
    CpuOCR = 1,
    /// set to CPU ID to enable GPU OCR
    GpuOCR = 2,
}

/// Available instance option key
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum InstanceOptionKey {
    /// set touch mode of instance
    TouchMode = 2,
    /// set to true to pause deployment
    DeploymentWithPause = 3,
    /// set to true to enable AdbLite
    AdbLiteEnabled = 4,
    /// set to true to kill Adb on exit
    KillAdbOnExit = 5,
}

pub use client_type::{ClientType, UnknownClientTypeError};
pub use task_type::{TaskType, UnknownTaskType};
pub use touch_mode::{TouchMode, UnknownTouchModeError};
