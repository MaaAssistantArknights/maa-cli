// Type definitions for assistant
//
// More details see:
// https://github.com/MaaAssistantArknights/MaaAssistantArknights/blob/dev/src/MaaCore/Common/AsstTypes.h

use crate::{Assistant, Result, ToCString};

/// Available static option key
#[repr(i32)]
#[derive(Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum StaticOptionKey {
    /// set to true to enable CPU OCR
    CpuOCR = 1,
    /// set to CPU ID to enable GPU OCR
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

/// Available instance option key
#[repr(i32)]
#[derive(Clone, Copy)]
pub enum InstanceOptionKey {
    /// set touch mode of instance
    TouchMode = 2,
    /// set to true to pause deployment
    DeploymentWithPause = 3,
    /// set to true to enable AdbLite
    AdbLiteEnabled = 4,
    /// set to true to kill ADB on exit
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

/// Available touch mode
#[repr(u8)]
#[derive(Default, Clone, Copy, PartialEq, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum TouchMode {
    #[default]
    ADB,
    MiniTouch,
    MaaTouch,
    MacPlayTools,
}

impl TouchMode {
    const COUNT: usize = 4;
    const VARIANTS: [TouchMode; Self::COUNT] = {
        let mut i = 0;
        let mut variants = [TouchMode::ADB; Self::COUNT];
        while i < Self::COUNT {
            variants[i] = unsafe { std::mem::transmute::<u8, Self>(i as u8) };
            i += 1;
        }
        variants
    };

    /// Convert TouchMode to a static string slice
    pub const fn to_str(self) -> &'static str {
        match self {
            TouchMode::ADB => "adb",
            TouchMode::MiniTouch => "minitouch",
            TouchMode::MaaTouch => "maatouch",
            TouchMode::MacPlayTools => "MacPlayTools",
        }
    }

    const NAMES: [&'static str; Self::COUNT] = {
        let mut i = 0;
        let mut names = [""; Self::COUNT];
        while i < Self::COUNT {
            names[i] = Self::VARIANTS[i].to_str();
            i += 1;
        }
        names
    };

    fn from_str_opt(s: &str) -> Option<TouchMode> {
        Self::VARIANTS
            .iter()
            .find(|v| v.to_str().eq_ignore_ascii_case(s))
            .copied()
    }
}

// DEPRECATED: use `to_str` instead, will be removed in the future.
impl AsRef<str> for TouchMode {
    fn as_ref(&self) -> &str {
        self.to_str()
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug)]
pub struct UnknownTouchModeError(String);

impl std::fmt::Display for UnknownTouchModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown touch mode `{}`, expected one of ", self.0)?;
        let mut iter = TouchMode::NAMES.iter();
        if let Some(name) = iter.next() {
            write!(f, "`{}`", name)?;
            for v in iter {
                write!(f, ", `{}`", v)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for UnknownTouchModeError {}

impl std::str::FromStr for TouchMode {
    type Err = UnknownTouchModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_opt(s).ok_or_else(|| UnknownTouchModeError(s.to_owned()))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TouchMode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<TouchMode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TouchModeVisitor;

        impl<'de> serde::de::Visitor<'de> for TouchModeVisitor {
            type Value = TouchMode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid touch mode")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<TouchMode, E>
            where
                E: serde::de::Error,
            {
                TouchMode::from_str_opt(value)
                    .ok_or_else(|| E::unknown_variant(value, &TouchMode::NAMES))
            }
        }

        deserializer.deserialize_str(TouchModeVisitor)
    }
}

impl std::fmt::Display for TouchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl ToCString for TouchMode {
    fn to_cstring(self) -> Result<std::ffi::CString> {
        self.to_str().to_cstring()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use TouchMode::*;

    // #[cfg(not(feature = "runtime"))]
    // #[test]
    // fn apply_to() {
    //     // Apply static options
    //     // We can't apply_to GPU OCR option because it requires a GPU which is not available in CI.
    //     StaticOptionKey::CpuOCR.apply_to(true).unwrap();
    //     // StaticOptionKey::GpuOCR.apply_to(1).unwrap();
    //
    //     use std::{env, path::Path};
    //     if let Some(Some(path)) =
    //         env::var_os("MAA_RESOURCE_DIR").map(|s| Path::new(&s).parent().map(|p| p.to_owned()))
    //     {
    //         Assistant::load_resource(path).unwrap();
    //
    //         // Apply instance options
    //         let asst = Assistant::new(None, None);
    //         InstanceOptionKey::TouchMode
    //             .apply_to(&asst, TouchMode::MaaTouch)
    //             .unwrap();
    //         InstanceOptionKey::DeploymentWithPause
    //             .apply_to(&asst, false)
    //             .unwrap();
    //         InstanceOptionKey::AdbLiteEnabled
    //             .apply_to(&asst, false)
    //             .unwrap();
    //         InstanceOptionKey::KillAdbOnExit
    //             .apply_to(&asst, false)
    //             .unwrap();
    //     }
    // }

    #[test]
    fn to_str() {
        assert_eq!(ADB.to_str(), "adb");
        assert_eq!(MiniTouch.to_str(), "minitouch");
        assert_eq!(MaaTouch.to_str(), "maatouch");
        assert_eq!(MacPlayTools.to_str(), "MacPlayTools");
    }

    #[test]
    fn to_string() {
        assert_eq!(ADB.to_string(), "adb");
        assert_eq!(MiniTouch.to_string(), "minitouch");
        assert_eq!(MaaTouch.to_string(), "maatouch");
        assert_eq!(MacPlayTools.to_string(), "MacPlayTools");
    }

    #[test]
    fn as_ref() {
        assert_eq!(ADB.as_ref(), "adb");
        assert_eq!(MiniTouch.as_ref(), "minitouch");
        assert_eq!(MaaTouch.as_ref(), "maatouch");
        assert_eq!(MacPlayTools.as_ref(), "MacPlayTools");
    }

    #[test]
    fn to_cstring() {
        use std::ffi::CString;

        fn csting(s: &str) -> CString {
            CString::new(s).unwrap()
        }

        assert_eq!(ADB.to_cstring().unwrap(), csting("adb"));
        assert_eq!(MiniTouch.to_cstring().unwrap(), csting("minitouch"));
        assert_eq!(MaaTouch.to_cstring().unwrap(), csting("maatouch"));
        assert_eq!(MacPlayTools.to_cstring().unwrap(), csting("MacPlayTools"));
    }

    #[test]
    fn parse() {
        assert_eq!("adb".parse(), Ok(ADB));
        assert_eq!("Adb".parse(), Ok(ADB));
        assert_eq!("ADB".parse(), Ok(ADB));
        assert_eq!("minitouch".parse(), Ok(MiniTouch));
        assert_eq!("MiniTouch".parse(), Ok(MiniTouch));
        assert_eq!("maatouch".parse(), Ok(MaaTouch));
        assert_eq!("MaaTouch".parse(), Ok(MaaTouch));
        assert_eq!("MAATouch".parse(), Ok(MaaTouch));
        assert_eq!("macplaytools".parse(), Ok(MacPlayTools));
        assert_eq!("MacPlayTools".parse(), Ok(MacPlayTools));

        assert_eq!(
            "Unknown".parse::<TouchMode>(),
            Err(UnknownTouchModeError("Unknown".to_owned()))
        );
        assert_eq!(
            UnknownTouchModeError("Unknown".to_owned()).to_string(),
            "unknown touch mode `Unknown`, expected one of `adb`, `minitouch`, `maatouch`, `MacPlayTools`",
        );
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

        #[test]
        fn deserialize() {
            assert_de_tokens(&ADB, &[Token::Str("adb")]);
            assert_de_tokens(&MiniTouch, &[Token::Str("minitouch")]);
            assert_de_tokens(&MaaTouch, &[Token::Str("maatouch")]);
            assert_de_tokens(&MacPlayTools, &[Token::Str("MacPlayTools")]);
        }

        #[test]
        fn deserialize_unknown() {
            assert_de_tokens_error::<TouchMode>(
                &[Token::Str("Unknown")],
                "unknown variant `Unknown`, expected one of \
                `adb`, `minitouch`, `maatouch`, `MacPlayTools`",
            );
        }
    }
}
