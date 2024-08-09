/// Available task type for MAA
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    ReclamationAlgorithm,
    Custom,
    SingleStep,
    VideoRecognition,
}

impl TaskType {
    const COUNT: usize = 16;

    const VARIANTS: [Self; Self::COUNT] = {
        let mut i = 0;
        let mut variants = [Self::StartUp; Self::COUNT];
        while i < Self::COUNT {
            variants[i] = unsafe { std::mem::transmute(i as u8) };
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
            Self::ReclamationAlgorithm => "ReclamationAlgorithm",
            Self::Custom => "Custom",
            Self::SingleStep => "SingleStep",
            Self::VideoRecognition => "VideoRecognition",
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

        impl<'de> serde::de::Visitor<'de> for TaskTypeVisitor {
            type Value = TaskType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid task type")
            }

            fn visit_str<E>(self, value: &str) -> Result<TaskType, E>
            where
                E: serde::de::Error,
            {
                TaskType::from_str_opt(value)
                    .ok_or_else(|| E::unknown_variant(&value, &TaskType::NAMES))
            }
        }

        deserializer.deserialize_str(TaskTypeVisitor)
    }
}

// DEPRECATED: use `to_str` instead, will be removed in the future.
impl AsRef<str> for TaskType {
    fn as_ref(&self) -> &str {
        self.to_str()
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl crate::ToCString for TaskType {
    fn to_cstring(self) -> crate::Result<std::ffi::CString> {
        self.to_str().to_cstring()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use TaskType::*;

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
        assert_eq!("ReclamationAlgorithm".parse(), Ok(ReclamationAlgorithm));
        assert_eq!("Custom".parse(), Ok(Custom));
        assert_eq!("SingleStep".parse(), Ok(SingleStep));
        assert_eq!("VideoRecognition".parse(), Ok(VideoRecognition));
        assert_eq!(
            "Unknown".parse::<TaskType>(),
            Err(UnknownTaskType("Unknown".to_owned()))
        );
    }

    #[test]
    fn unknown_task_type_error() {
        assert_eq!(
            UnknownTaskType("Unknown".to_owned()).to_string(),
            "unknown task type `Unknown`, expected one of `StartUp`, `CloseDown`, `Fight`, \
            `Recruit`, `Infrast`, `Mall`, `Award`, `Roguelike`, `Copilot`, `SSSCopilot`, \
            `Depot`, `OperBox`, `ReclamationAlgorithm`, `Custom`, `SingleStep`, `VideoRecognition`",
        );
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

        #[test]
        fn deserialize() {
            let types: [TaskType; 2] = [StartUp, CloseDown];

            assert_de_tokens(
                &types,
                &[
                    Token::Seq { len: Some(2) },
                    Token::Str("StartUp"),
                    Token::Str("CloseDown"),
                    Token::SeqEnd,
                ],
            );
        }

        #[test]
        fn deserialize_unknown_variance() {
            assert_de_tokens_error::<TaskType>(
                &[Token::Str("Unknown")],
                "unknown variant `Unknown`, expected one of `StartUp`, `CloseDown`, `Fight`, \
                `Recruit`, `Infrast`, `Mall`, `Award`, `Roguelike`, `Copilot`, `SSSCopilot`, \
                `Depot`, `OperBox`, `ReclamationAlgorithm`, `Custom`, `SingleStep`, `VideoRecognition`",
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
        assert_eq!(
            TaskType::ReclamationAlgorithm.to_str(),
            "ReclamationAlgorithm",
        );
        assert_eq!(Custom.to_str(), "Custom");
        assert_eq!(SingleStep.to_str(), "SingleStep");
        assert_eq!(VideoRecognition.to_str(), "VideoRecognition");
    }

    #[test]
    fn to_string() {
        assert_eq!(StartUp.to_string(), "StartUp");
    }

    #[test]
    fn to_cstring() {
        use crate::ToCString;

        use std::ffi::CString;

        assert_eq!(
            StartUp.to_cstring().unwrap(),
            CString::new("StartUp").unwrap(),
        );
    }
}
