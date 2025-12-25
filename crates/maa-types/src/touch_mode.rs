#[repr(u8)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default, Clone, Copy, PartialEq)]
/// Method to emulate touch input
pub enum TouchMode {
    #[default]
    /// Usable on all emulators, containers, and real devices, but touch emulation is not perfect
    Adb,
    /// Have better touch emulation than Adb, but may not work on some platforms
    MiniTouch,
    /// A port of MiniTouch, with better touch emulation and works for most of platforms,
    /// recommended for most users
    MaaTouch,
    /// A special touch mode that not works with Android but works with iOS app running on Mac with
    /// PlayCover. If you are connected to PlayCover, you must use this mode.
    ///
    /// If you use preset `PlayCover`, you can ignore this option as it's set automatically.
    MacPlayTools,
}

impl TouchMode {
    impl_enum_utils!(TouchMode, 4, TouchMode::Adb);

    impl_from_str_opt!();

    /// Convert TouchMode to a static string slice
    pub const fn to_str(self) -> &'static str {
        match self {
            TouchMode::Adb => "adb",
            TouchMode::MiniTouch => "minitouch",
            TouchMode::MaaTouch => "maatouch",
            TouchMode::MacPlayTools => "MacPlayTools",
        }
    }
}

impl_unknown_error!(UnknownTouchModeError, TouchMode, "touch mode");
impl_from_str!(TouchMode, UnknownTouchModeError);

#[cfg(feature = "serde")]
impl_serde_deserialize!(TouchMode, "a valid touch mode");

#[cfg(feature = "serde")]
impl_serde_serialize!(TouchMode);

#[cfg(feature = "ffi")]
impl maa_ffi_string::ToCString for TouchMode {
    fn to_cstring(self) -> maa_ffi_string::Result<std::ffi::CString> {
        self.to_str().to_cstring()
    }
}

impl_debug_display!(TouchMode);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!("adb".parse(), Ok(TouchMode::Adb));
        assert_eq!("Adb".parse(), Ok(TouchMode::Adb));
        assert_eq!("ADB".parse(), Ok(TouchMode::Adb));
        assert_eq!("minitouch".parse(), Ok(TouchMode::MiniTouch));
        assert_eq!("MiniTouch".parse(), Ok(TouchMode::MiniTouch));
        assert_eq!("maatouch".parse(), Ok(TouchMode::MaaTouch));
        assert_eq!("MaaTouch".parse(), Ok(TouchMode::MaaTouch));
        assert_eq!("MAATouch".parse(), Ok(TouchMode::MaaTouch));
        assert_eq!("macplaytools".parse(), Ok(TouchMode::MacPlayTools));
        assert_eq!("MacPlayTools".parse(), Ok(TouchMode::MacPlayTools));

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
        use serde_test::{Token, assert_de_tokens, assert_de_tokens_error, assert_ser_tokens};

        use super::*;

        #[test]
        fn deserialize() {
            let modes = [
                TouchMode::Adb,
                TouchMode::MiniTouch,
                TouchMode::MaaTouch,
                TouchMode::MacPlayTools,
            ];

            // Test deserializing from string
            assert_de_tokens(&modes, &[
                Token::Seq { len: Some(4) },
                Token::Str("adb"),
                Token::Str("minitouch"),
                Token::Str("maatouch"),
                Token::Str("MacPlayTools"),
                Token::SeqEnd,
            ]);
        }

        #[test]
        fn deserialize_error() {
            assert_de_tokens_error::<TouchMode>(
                &[Token::Str("Unknown")],
                "unknown variant `Unknown`, expected one of \
                `adb`, `minitouch`, `maatouch`, `MacPlayTools`",
            );

            assert_de_tokens_error::<TouchMode>(
                &[Token::U64(4)],
                "invalid type: integer `4`, expected a valid touch mode",
            );
        }

        #[test]
        fn serialize() {
            assert_ser_tokens(&TouchMode::Adb, &[Token::Str("adb")]);
            assert_ser_tokens(&TouchMode::MiniTouch, &[Token::Str("minitouch")]);
            assert_ser_tokens(&TouchMode::MaaTouch, &[Token::Str("maatouch")]);
            assert_ser_tokens(&TouchMode::MacPlayTools, &[Token::Str("MacPlayTools")]);
        }
    }

    #[test]
    fn to_str() {
        assert_eq!(TouchMode::Adb.to_str(), "adb");
        assert_eq!(TouchMode::MiniTouch.to_str(), "minitouch");
        assert_eq!(TouchMode::MaaTouch.to_str(), "maatouch");
        assert_eq!(TouchMode::MacPlayTools.to_str(), "MacPlayTools");
    }

    #[cfg(feature = "ffi")]
    #[test]
    fn to_cstring() {
        use maa_ffi_string::ToCString;

        assert_eq!(TouchMode::Adb.to_cstring().unwrap().as_c_str(), c"adb");
        assert_eq!(
            TouchMode::MiniTouch.to_cstring().unwrap().as_c_str(),
            c"minitouch"
        );
    }

    #[test]
    fn fmt() {
        assert_eq!(format!("{}", TouchMode::Adb), "adb");
        assert_eq!(format!("{:?}", TouchMode::MiniTouch), "minitouch");
    }
}
