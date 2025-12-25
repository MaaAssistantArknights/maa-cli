/// Available task type for MAA
#[repr(u8)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
            variants[i] = unsafe { Self::from_u8_unchecked(i as u8) };
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

    pub const fn from_u8(v: u8) -> Option<Self> {
        if Self::COUNT > v as usize {
            Some(unsafe { Self::from_u8_unchecked(v) })
        } else {
            None
        }
    }

    const unsafe fn from_u8_unchecked(v: u8) -> Self {
        unsafe { std::mem::transmute(v) }
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
            write!(f, "`{v}`")?;
            for v in iter {
                write!(f, ", `{v}`")?;
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
                if let Ok(v_u8) = u8::try_from(v)
                    && let Some(t) = TaskType::from_u8(v_u8)
                {
                    Ok(t)
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

#[cfg(feature = "ffi")]
impl maa_ffi_string::ToCString for TaskType {
    fn to_cstring(self) -> maa_ffi_string::Result<std::ffi::CString> {
        self.to_str().to_cstring()
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!("StartUp".parse(), Ok(TaskType::StartUp));
        assert_eq!("CloseDown".parse(), Ok(TaskType::CloseDown));
        assert_eq!("Fight".parse(), Ok(TaskType::Fight));
        assert_eq!("Recruit".parse(), Ok(TaskType::Recruit));
        assert_eq!("Infrast".parse(), Ok(TaskType::Infrast));
        assert_eq!("Mall".parse(), Ok(TaskType::Mall));
        assert_eq!("Award".parse(), Ok(TaskType::Award));
        assert_eq!("Roguelike".parse(), Ok(TaskType::Roguelike));
        assert_eq!("Copilot".parse(), Ok(TaskType::Copilot));
        assert_eq!("SSSCopilot".parse(), Ok(TaskType::SSSCopilot));
        assert_eq!("Depot".parse(), Ok(TaskType::Depot));
        assert_eq!("OperBox".parse(), Ok(TaskType::OperBox));
        assert_eq!("Reclamation".parse(), Ok(TaskType::Reclamation));
        assert_eq!("Custom".parse(), Ok(TaskType::Custom));
        assert_eq!("SingleStep".parse(), Ok(TaskType::SingleStep));
        assert_eq!("VideoRecognition".parse(), Ok(TaskType::VideoRecognition));
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
        use serde_test::{Token, assert_de_tokens, assert_de_tokens_error, assert_ser_tokens};

        use super::*;

        #[test]
        fn deserialize() {
            let types: [TaskType; 2] = [TaskType::StartUp, TaskType::CloseDown];

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

        #[test]
        fn serialize() {
            assert_ser_tokens(&TaskType::StartUp, &[Token::U64(0)]);
            assert_ser_tokens(&TaskType::CloseDown, &[Token::U64(1)]);
            assert_ser_tokens(&TaskType::Fight, &[Token::U64(2)]);
            assert_ser_tokens(&TaskType::Recruit, &[Token::U64(3)]);
            assert_ser_tokens(&TaskType::Infrast, &[Token::U64(4)]);
            assert_ser_tokens(&TaskType::Mall, &[Token::U64(5)]);
            assert_ser_tokens(&TaskType::Award, &[Token::U64(6)]);
            assert_ser_tokens(&TaskType::Roguelike, &[Token::U64(7)]);
            assert_ser_tokens(&TaskType::Copilot, &[Token::U64(8)]);
            assert_ser_tokens(&TaskType::SSSCopilot, &[Token::U64(9)]);
            assert_ser_tokens(&TaskType::Depot, &[Token::U64(10)]);
            assert_ser_tokens(&TaskType::OperBox, &[Token::U64(11)]);
            assert_ser_tokens(&TaskType::Reclamation, &[Token::U64(12)]);
            assert_ser_tokens(&TaskType::Custom, &[Token::U64(13)]);
            assert_ser_tokens(&TaskType::SingleStep, &[Token::U64(14)]);
            assert_ser_tokens(&TaskType::VideoRecognition, &[Token::U64(15)]);
        }
    }

    #[test]
    fn to_str() {
        assert_eq!(TaskType::StartUp.to_str(), "StartUp");
        assert_eq!(TaskType::CloseDown.to_str(), "CloseDown");
        assert_eq!(TaskType::Fight.to_str(), "Fight");
        assert_eq!(TaskType::Recruit.to_str(), "Recruit");
        assert_eq!(TaskType::Infrast.to_str(), "Infrast");
        assert_eq!(TaskType::Mall.to_str(), "Mall");
        assert_eq!(TaskType::Award.to_str(), "Award");
        assert_eq!(TaskType::Roguelike.to_str(), "Roguelike");
        assert_eq!(TaskType::Copilot.to_str(), "Copilot");
        assert_eq!(TaskType::SSSCopilot.to_str(), "SSSCopilot");
        assert_eq!(TaskType::Depot.to_str(), "Depot");
        assert_eq!(TaskType::OperBox.to_str(), "OperBox");
        assert_eq!(TaskType::Reclamation.to_str(), "Reclamation",);
        assert_eq!(TaskType::Custom.to_str(), "Custom");
        assert_eq!(TaskType::SingleStep.to_str(), "SingleStep");
        assert_eq!(TaskType::VideoRecognition.to_str(), "VideoRecognition");
    }

    #[cfg(feature = "ffi")]
    #[test]
    fn to_cstring() {
        use maa_ffi_string::ToCString;
        assert_eq!(TaskType::StartUp.to_cstring().unwrap(), c"StartUp");
        assert_eq!(TaskType::CloseDown.to_cstring().unwrap(), c"CloseDown");
    }

    #[test]
    fn fmt() {
        assert_eq!(format!("{}", TaskType::StartUp), "StartUp");
        assert_eq!(format!("{:?}", TaskType::StartUp), "StartUp");
    }
}
