#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

#[macro_use]
mod enum_macros;

mod client_type;
mod message_kind;
mod task_type;
mod touch_mode;
mod win32_input_method;
mod win32_screencap_method;

pub use maa_ffi_types as primitive;
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
pub use message_kind::MessageKind;
pub use task_type::{TaskType, UnknownTaskType};
pub use touch_mode::{TouchMode, UnknownTouchModeError};
pub use win32_input_method::{UnknownWin32InputMethodError, Win32InputMethod};
pub use win32_screencap_method::{UnknownWin32ScreencapMethodError, Win32ScreencapMethod};
