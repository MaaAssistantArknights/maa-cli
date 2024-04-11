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

const TASK_TYPE_STRS: [&str; 16] = [
    "StartUp",
    "CloseDown",
    "Fight",
    "Recruit",
    "Infrast",
    "Mall",
    "Award",
    "Roguelike",
    "Copilot",
    "SSSCopilot",
    "Depot",
    "OperBox",
    "ReclamationAlgorithm",
    "Custom",
    "SingleStep",
    "VideoRecognition",
];

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug)]
pub struct UnknownTaskType(String);

impl std::fmt::Display for UnknownTaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown task type `{}`", self.0)
    }
}

impl std::error::Error for UnknownTaskType {}

impl std::str::FromStr for TaskType {
    type Err = UnknownTaskType;

    fn from_str(s: &str) -> Result<TaskType, Self::Err> {
        TASK_TYPE_STRS
            .iter()
            .position(|&x| x.eq_ignore_ascii_case(s))
            .map(|i| unsafe { std::mem::transmute(i as u8) })
            .ok_or_else(|| UnknownTaskType(s.to_owned()))
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
                formatter.write_str("a task type")
            }

            fn visit_str<E>(self, value: &str) -> Result<TaskType, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(TaskTypeVisitor)
    }
}

impl TaskType {
    fn to_str(self) -> &'static str {
        unsafe { TASK_TYPE_STRS.get_unchecked(self as usize) }
    }
}

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
    fn error() {
        assert_eq!(
            UnknownTaskType("Unknown".to_owned()).to_string(),
            "unknown task type `Unknown`",
        );
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, Token};

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
        #[should_panic]
        fn deserialize_unknown_variance() {
            assert_de_tokens(
                &StartUp,
                &[
                    Token::Seq { len: Some(1) },
                    Token::Str("Unknown"),
                    Token::SeqEnd,
                ],
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
