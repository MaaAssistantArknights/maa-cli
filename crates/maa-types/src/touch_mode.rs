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
    pub const COUNT: usize = 4;
    pub const NAMES: [&'static str; Self::COUNT] = {
        let mut i = 0;
        let mut names = [""; Self::COUNT];
        while i < Self::COUNT {
            names[i] = Self::VARIANTS[i].to_str();
            i += 1;
        }
        names
    };
    pub const VARIANTS: [TouchMode; Self::COUNT] = {
        let mut i = 0;
        let mut variants = [TouchMode::Adb; Self::COUNT];
        while i < Self::COUNT {
            variants[i] = unsafe { Self::from_u8_unchecked(i as u8) };
            i += 1;
        }
        variants
    };

    /// Convert TouchMode to a static string slice
    pub const fn to_str(self) -> &'static str {
        match self {
            TouchMode::Adb => "adb",
            TouchMode::MiniTouch => "minitouch",
            TouchMode::MaaTouch => "maatouch",
            TouchMode::MacPlayTools => "MacPlayTools",
        }
    }

    fn from_str_opt(s: &str) -> Option<TouchMode> {
        Self::VARIANTS
            .iter()
            .find(|v| v.to_str().eq_ignore_ascii_case(s))
            .copied()
    }

    pub const fn from_u8(value: u8) -> Option<TouchMode> {
        if Self::COUNT > value as usize {
            Some(unsafe { Self::from_u8_unchecked(value) })
        } else {
            None
        }
    }

    const unsafe fn from_u8_unchecked(value: u8) -> TouchMode {
        unsafe { std::mem::transmute(value) }
    }
}

impl std::str::FromStr for TouchMode {
    type Err = UnknownTouchModeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_str_opt(s).ok_or_else(|| UnknownTouchModeError(s.to_owned()))
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
            write!(f, "`{name}`")?;
            for v in iter {
                write!(f, ", `{v}`")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for UnknownTouchModeError {}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TouchMode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<TouchMode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TouchModeVisitor;

        impl serde::de::Visitor<'_> for TouchModeVisitor {
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

#[cfg(feature = "serde")]
impl serde::Serialize for TouchMode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_str())
    }
}

#[cfg(feature = "ffi")]
impl maa_ffi_string::ToCString for TouchMode {
    fn to_cstring(self) -> maa_ffi_string::Result<std::ffi::CString> {
        self.to_str().to_cstring()
    }
}

impl std::fmt::Debug for TouchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl std::fmt::Display for TouchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

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

            // Test deserializing from u64
            assert_de_tokens(&modes, &[
                Token::Seq { len: Some(4) },
                Token::U64(0),
                Token::U64(1),
                Token::U64(2),
                Token::U64(3),
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
                "invalid value: integer `4`, expected a valid touch mode",
            );
        }

        #[test]
        fn serialize() {
            assert_ser_tokens(&TouchMode::Adb, &[Token::U64(0)]);
            assert_ser_tokens(&TouchMode::MiniTouch, &[Token::U64(1)]);
            assert_ser_tokens(&TouchMode::MaaTouch, &[Token::U64(2)]);
            assert_ser_tokens(&TouchMode::MacPlayTools, &[Token::U64(3)]);
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
