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

/// Available touch mode
#[repr(u8)]
#[derive(Default, Clone, Copy, PartialEq)]
pub enum TouchMode {
    #[default]
    Adb,
    MiniTouch,
    MaaTouch,
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
            variants[i] = unsafe { std::mem::transmute::<u8, Self>(i as u8) };
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
            write!(f, "`{}`", name)?;
            for v in iter {
                write!(f, ", `{}`", v)?;
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

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v < TouchMode::COUNT as u64 {
                    Ok(unsafe { std::mem::transmute::<u8, TouchMode>(v as u8) })
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &self))
                }
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
        serializer.serialize_u64(*self as u64)
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

/// Available task type for MAA
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    StartUp,
    CloseDown,
    Fight,
    Recruit,
    Infrast,
    Mall,
    Award,
    Roguelike,
    Copilot,
    SSSCopilot,
    Depot,
    OperBox,
    Reclamation,
    Custom,
    SingleStep,
    VideoRecognition,
}

impl TaskType {
    pub const COUNT: usize = 16;
    pub const NAMES: [&'static str; Self::COUNT] = {
        let mut i = 0;
        let mut names = [""; Self::COUNT];
        while i < Self::COUNT {
            names[i] = Self::VARIANTS[i].to_str();
            i += 1;
        }
        names
    };
    pub const VARIANTS: [Self; Self::COUNT] = {
        let mut i = 0;
        let mut variants = [Self::StartUp; Self::COUNT];
        while i < Self::COUNT {
            variants[i] = unsafe { std::mem::transmute::<u8, Self>(i as u8) };
            i += 1;
        }
        variants
    };

    pub const fn to_str(self) -> &'static str {
        match self {
            Self::StartUp => "StartUp",
            Self::CloseDown => "CloseDown",
            Self::Fight => "Fight",
            Self::Recruit => "Recruit",
            Self::Infrast => "Infrast",
            Self::Mall => "Mall",
            Self::Award => "Award",
            Self::Roguelike => "Roguelike",
            Self::Copilot => "Copilot",
            Self::SSSCopilot => "SSSCopilot",
            Self::Depot => "Depot",
            Self::OperBox => "OperBox",
            Self::Reclamation => "Reclamation",
            Self::Custom => "Custom",
            Self::SingleStep => "SingleStep",
            Self::VideoRecognition => "VideoRecognition",
        }
    }

    fn from_str_opt(s: &str) -> Option<Self> {
        Self::VARIANTS
            .iter()
            .find(|v| v.to_str().eq_ignore_ascii_case(s))
            .copied()
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug)]
pub struct UnknownTaskType(String);

impl std::fmt::Display for UnknownTaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown task type `{}`, expected one of ", self.0)?;
        let mut iter = TaskType::NAMES.iter();
        if let Some(v) = iter.next() {
            write!(f, "`{}`", v)?;
            for v in iter {
                write!(f, ", `{}`", v)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for UnknownTaskType {}

impl std::str::FromStr for TaskType {
    type Err = UnknownTaskType;

    fn from_str(s: &str) -> Result<TaskType, Self::Err> {
        Self::from_str_opt(s).ok_or_else(|| UnknownTaskType(s.to_owned()))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TaskType {
    fn deserialize<D>(deserializer: D) -> Result<TaskType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TaskTypeVisitor;

        impl serde::de::Visitor<'_> for TaskTypeVisitor {
            type Value = TaskType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid task type")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v < TaskType::COUNT as u64 {
                    Ok(unsafe { std::mem::transmute::<u8, TaskType>(v as u8) })
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &self))
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<TaskType, E>
            where
                E: serde::de::Error,
            {
                TaskType::from_str_opt(value)
                    .ok_or_else(|| E::unknown_variant(value, &TaskType::NAMES))
            }
        }

        deserializer.deserialize_str(TaskTypeVisitor)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TaskType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(*self as u64)
    }
}

impl std::fmt::Debug for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod touch_mode {
        use TouchMode::*;

        use super::*;

        #[test]
        fn parse() {
            assert_eq!("adb".parse(), Ok(Adb));
            assert_eq!("Adb".parse(), Ok(Adb));
            assert_eq!("ADB".parse(), Ok(Adb));
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
            use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

            use super::*;

            #[test]
            fn deserialize() {
                let modes = [Adb, MiniTouch, MaaTouch, MacPlayTools];

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
        }

        #[test]
        fn to_str() {
            assert_eq!(Adb.to_str(), "adb");
            assert_eq!(MiniTouch.to_str(), "minitouch");
            assert_eq!(MaaTouch.to_str(), "maatouch");
            assert_eq!(MacPlayTools.to_str(), "MacPlayTools");
        }

        #[test]
        fn fmt() {
            assert_eq!(format!("{}", Adb), "adb");
            assert_eq!(format!("{:?}", MiniTouch), "minitouch");
        }
    }

    mod task_type {
        use TaskType::*;

        use super::*;

        #[test]
        fn parse() {
            assert_eq!("StartUp".parse(), Ok(StartUp));
            assert_eq!("CloseDown".parse(), Ok(CloseDown));
            assert_eq!("Fight".parse(), Ok(Fight));
            assert_eq!("Recruit".parse(), Ok(Recruit));
            assert_eq!("Infrast".parse(), Ok(Infrast));
            assert_eq!("Mall".parse(), Ok(Mall));
            assert_eq!("Award".parse(), Ok(Award));
            assert_eq!("Roguelike".parse(), Ok(Roguelike));
            assert_eq!("Copilot".parse(), Ok(Copilot));
            assert_eq!("SSSCopilot".parse(), Ok(SSSCopilot));
            assert_eq!("Depot".parse(), Ok(Depot));
            assert_eq!("OperBox".parse(), Ok(OperBox));
            assert_eq!("Reclamation".parse(), Ok(Reclamation));
            assert_eq!("Custom".parse(), Ok(Custom));
            assert_eq!("SingleStep".parse(), Ok(SingleStep));
            assert_eq!("VideoRecognition".parse(), Ok(VideoRecognition));
            assert_eq!(
                "Unknown".parse::<TaskType>(),
                Err(UnknownTaskType("Unknown".to_owned()))
            );
            assert_eq!(
                UnknownTaskType("Unknown".to_owned()).to_string(),
                "unknown task type `Unknown`, expected one of `StartUp`, `CloseDown`, `Fight`, \
                `Recruit`, `Infrast`, `Mall`, `Award`, `Roguelike`, `Copilot`, `SSSCopilot`, \
                `Depot`, `OperBox`, `Reclamation`, `Custom`, `SingleStep`, `VideoRecognition`",
            );
        }

        #[cfg(feature = "serde")]
        mod serde {
            use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

            use super::*;

            #[test]
            fn deserialize() {
                let types: [TaskType; 2] = [StartUp, CloseDown];

                assert_de_tokens(&types, &[
                    Token::Seq { len: Some(2) },
                    Token::Str("StartUp"),
                    Token::Str("CloseDown"),
                    Token::SeqEnd,
                ]);

                assert_de_tokens(&types, &[
                    Token::Seq { len: Some(2) },
                    Token::U64(0),
                    Token::U64(1),
                    Token::SeqEnd,
                ]);
            }

            #[test]
            fn deserialize_error() {
                assert_de_tokens_error::<TaskType>(
                    &[Token::Str("Unknown")],
                    "unknown variant `Unknown`, expected one of `StartUp`, `CloseDown`, `Fight`, \
                    `Recruit`, `Infrast`, `Mall`, `Award`, `Roguelike`, `Copilot`, `SSSCopilot`, \
                    `Depot`, `OperBox`, `Reclamation`, `Custom`, `SingleStep`, `VideoRecognition`",
                );

                assert_de_tokens_error::<TaskType>(
                    &[Token::U64(16)],
                    "invalid value: integer `16`, expected a valid task type",
                );
            }
        }

        #[test]
        fn to_str() {
            assert_eq!(StartUp.to_str(), "StartUp");
            assert_eq!(CloseDown.to_str(), "CloseDown");
            assert_eq!(Fight.to_str(), "Fight");
            assert_eq!(Recruit.to_str(), "Recruit");
            assert_eq!(Infrast.to_str(), "Infrast");
            assert_eq!(Mall.to_str(), "Mall");
            assert_eq!(Award.to_str(), "Award");
            assert_eq!(Roguelike.to_str(), "Roguelike");
            assert_eq!(Copilot.to_str(), "Copilot");
            assert_eq!(SSSCopilot.to_str(), "SSSCopilot");
            assert_eq!(Depot.to_str(), "Depot");
            assert_eq!(OperBox.to_str(), "OperBox");
            assert_eq!(Reclamation.to_str(), "Reclamation",);
            assert_eq!(Custom.to_str(), "Custom");
            assert_eq!(SingleStep.to_str(), "SingleStep");
            assert_eq!(VideoRecognition.to_str(), "VideoRecognition");
        }

        #[test]
        fn fmt() {
            assert_eq!(format!("{}", StartUp), "StartUp");
            assert_eq!(format!("{:?}", StartUp), "StartUp");
        }
    }
}
