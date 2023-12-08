// Type definitions for assistant
//
// More details see:
// https://github.com/MaaAssistantArknights/MaaAssistantArknights/blob/dev/src/MaaCore/Common/AsstTypes.h

use crate::{impl_to_cstring_by_as_ref, Assistant, Result, ToCString};

#[repr(i32)]
#[derive(Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum StaticOptionKey {
    CpuOCR = 1,
    GpuOCR = 2,
}

impl AsRef<str> for StaticOptionKey {
    fn as_ref(&self) -> &str {
        match self {
            StaticOptionKey::CpuOCR => "CPUOCR",
            StaticOptionKey::GpuOCR => "GPUOCR",
        }
    }
}

impl std::fmt::Display for StaticOptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl StaticOptionKey {
    /// Apply the static option to MaaCore
    ///
    /// # Example
    //
    /// ```no_run
    /// use maa_core::StaticOptionKey;
    ///
    /// StaticOptionKey::CpuOCR.apply(true);
    /// ```
    pub fn apply(self, value: impl ToCString) -> Result<()> {
        Assistant::set_static_option(self as i32, value)
    }
}

#[repr(i32)]
#[derive(Clone, Copy)]
pub enum InstanceOptionKey {
    TouchMode = 2,
    DeploymentWithPause = 3,
    AdbLiteEnabled = 4,
    KillAdbOnExit = 5,
}

impl AsRef<str> for InstanceOptionKey {
    fn as_ref(&self) -> &str {
        match self {
            InstanceOptionKey::TouchMode => "TouchMode",
            InstanceOptionKey::DeploymentWithPause => "DeploymentWithPause",
            InstanceOptionKey::AdbLiteEnabled => "AdbLiteEnabled",
            InstanceOptionKey::KillAdbOnExit => "KillAdbOnExit",
        }
    }
}

impl std::fmt::Display for InstanceOptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl InstanceOptionKey {
    /// Apply the instance option to the given assistant.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use maa_core::{Assistant, InstanceOptionKey, TouchMode};
    ///
    /// let asst = Assistant::new(None, None);
    /// InstanceOptionKey::TouchMode.apply(&asst, TouchMode::ADB);
    /// ```
    pub fn apply(self, asst: &Assistant, value: impl ToCString) -> Result<()> {
        asst.set_instance_option(self as i32, value)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Default, Clone, Copy, PartialEq, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum TouchMode {
    #[default]
    ADB,
    MiniTouch,
    #[cfg_attr(feature = "serde", serde(alias = "MAATouch"))]
    MaaTouch,
    MacPlayTools,
}

impl AsRef<str> for TouchMode {
    fn as_ref(&self) -> &str {
        match self {
            TouchMode::ADB => "adb",
            TouchMode::MiniTouch => "minitouch",
            TouchMode::MaaTouch => "maatouch",
            TouchMode::MacPlayTools => "MacPlayTools",
        }
    }
}

impl std::fmt::Display for TouchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl_to_cstring_by_as_ref!(str, TouchMode);
