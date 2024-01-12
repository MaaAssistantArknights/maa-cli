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
                use TaskType::*;
                match value {
                    "StartUp" | "startup" | "Startup" => Ok(StartUp),
                    "CloseDown" | "closedown" | "Closedown" => Ok(CloseDown),
                    "Fight" | "fight" => Ok(Fight),
                    "Recruit" | "recruit" => Ok(Recruit),
                    "Infrast" | "infrast" => Ok(Infrast),
                    "Mall" | "mall" => Ok(Mall),
                    "Award" | "award" => Ok(Award),
                    "Roguelike" | "roguelike" => Ok(Roguelike),
                    "Copilot" | "copilot" => Ok(Copilot),
                    "SSSCopilot" | "ssscopilot" => Ok(SSSCopilot),
                    "Depot" | "depot" => Ok(Depot),
                    "OperBox" | "operbox" => Ok(OperBox),
                    "ReclamationAlgorithm" | "reclamationalgorithm" => Ok(ReclamationAlgorithm),
                    "Custom" | "custom" => Ok(Custom),
                    "SingleStep" | "singlestep" => Ok(SingleStep),
                    "VideoRecognition" | "videorecognition" => Ok(VideoRecognition),
                    _ => Err(E::unknown_variant(value, &TASK_TYPE_STRS)),
                }
            }
        }

        deserializer.deserialize_str(TaskTypeVisitor)
    }
}

impl TaskType {
    fn to_str(self) -> &'static str {
        TASK_TYPE_STRS[self as usize]
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

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, Token};

        #[test]
        fn deserialize() {
            let types: [TaskType; 16] = [
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
            ];

            assert_de_tokens(
                &types,
                &[
                    Token::Seq { len: Some(16) },
                    Token::Str("StartUp"),
                    Token::Str("CloseDown"),
                    Token::Str("Fight"),
                    Token::Str("Recruit"),
                    Token::Str("Infrast"),
                    Token::Str("Mall"),
                    Token::Str("Award"),
                    Token::Str("Roguelike"),
                    Token::Str("Copilot"),
                    Token::Str("SSSCopilot"),
                    Token::Str("Depot"),
                    Token::Str("OperBox"),
                    Token::Str("ReclamationAlgorithm"),
                    Token::Str("Custom"),
                    Token::Str("SingleStep"),
                    Token::Str("VideoRecognition"),
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
