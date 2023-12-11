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

impl StaticOptionKey {
    /// Apply the static option to MaaCore
    ///
    /// # Example
    //
    /// ```no_run
    /// use maa_sys::StaticOptionKey;
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

impl InstanceOptionKey {
    /// Apply the instance option to the given assistant.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use maa_sys::{Assistant, InstanceOptionKey, TouchMode};
    ///
    /// let asst = Assistant::new(None, None);
    /// InstanceOptionKey::TouchMode.apply_to(&asst, TouchMode::ADB);
    /// ```
    pub fn apply_to(self, asst: &Assistant, value: impl ToCString) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "runtime"))]
    #[test]
    fn apply() {
        // Apply static options
        // We can't apply GPU OCR option because it requires a GPU which is not available in CI.
        StaticOptionKey::CpuOCR.apply(true).unwrap();
        // StaticOptionKey::GpuOCR.apply(1).unwrap();

        use std::{env, path::Path};
        if let Some(path) = env::var_os("MAA_RESOURCE_DIR") {
            Assistant::load_resource(Path::new(&path)).unwrap();

            // Apply instance options
            let asst = Assistant::new(None, None);
            InstanceOptionKey::TouchMode
                .apply_to(&asst, TouchMode::MaaTouch)
                .unwrap();
            InstanceOptionKey::DeploymentWithPause
                .apply_to(&asst, false)
                .unwrap();
            InstanceOptionKey::AdbLiteEnabled
                .apply_to(&asst, false)
                .unwrap();
            InstanceOptionKey::KillAdbOnExit
                .apply_to(&asst, false)
                .unwrap();
        }
    }

    mod to_cstring {
        use super::*;
        use std::ffi::CString;

        #[test]
        fn touch_mode() {
            assert_eq!(
                TouchMode::ADB.to_cstring().unwrap(),
                CString::new("adb").unwrap()
            );

            assert_eq!(
                TouchMode::MiniTouch.to_cstring().unwrap(),
                CString::new("minitouch").unwrap()
            );

            assert_eq!(
                TouchMode::MaaTouch.to_cstring().unwrap(),
                CString::new("maatouch").unwrap()
            );

            assert_eq!(
                TouchMode::MacPlayTools.to_cstring().unwrap(),
                CString::new("MacPlayTools").unwrap()
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn touch_mode() {
            assert_eq!(TouchMode::ADB.to_string(), "adb");
            assert_eq!(TouchMode::MiniTouch.to_string(), "minitouch");
            assert_eq!(TouchMode::MaaTouch.to_string(), "maatouch");
            assert_eq!(TouchMode::MacPlayTools.to_string(), "MacPlayTools");
        }
    }
}
