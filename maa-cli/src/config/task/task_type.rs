use serde::Deserialize;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone, Copy)]
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

impl AsRef<str> for TaskType {
    fn as_ref(&self) -> &str {
        match self {
            TaskType::StartUp => "StartUp",
            TaskType::CloseDown => "CloseDown",
            TaskType::Fight => "Fight",
            TaskType::Recruit => "Recruit",
            TaskType::Infrast => "Infrast",
            TaskType::Mall => "Mall",
            TaskType::Award => "Award",
            TaskType::Roguelike => "Roguelike",
            TaskType::Copilot => "Copilot",
            TaskType::SSSCopilot => "SSSCopilot",
            TaskType::Depot => "Depot",
            TaskType::OperBox => "OperBox",
            TaskType::ReclamationAlgorithm => "ReclamationAlgorithm",
            TaskType::Custom => "Custom",
            TaskType::SingleStep => "SingleStep",
            TaskType::VideoRecognition => "VideoRecognition",
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TaskOrUnknown {
    Task(TaskType),
    Unknown(String),
}

impl From<TaskType> for TaskOrUnknown {
    fn from(task_type: TaskType) -> Self {
        TaskOrUnknown::Task(task_type)
    }
}

impl AsRef<str> for TaskOrUnknown {
    fn as_ref(&self) -> &str {
        match self {
            TaskOrUnknown::Task(task) => task.as_ref(),
            TaskOrUnknown::Unknown(s) => s.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_test::{assert_de_tokens, Token};

    #[test]
    fn deserialize() {
        let types: [TaskOrUnknown; 17] = [
            TaskType::StartUp.into(),
            TaskType::CloseDown.into(),
            TaskType::Fight.into(),
            TaskType::Recruit.into(),
            TaskType::Infrast.into(),
            TaskType::Mall.into(),
            TaskType::Award.into(),
            TaskType::Roguelike.into(),
            TaskType::Copilot.into(),
            TaskType::SSSCopilot.into(),
            TaskType::Depot.into(),
            TaskType::OperBox.into(),
            TaskType::ReclamationAlgorithm.into(),
            TaskType::Custom.into(),
            TaskType::SingleStep.into(),
            TaskType::VideoRecognition.into(),
            TaskOrUnknown::Unknown("Other".to_string()),
        ];

        assert_de_tokens(
            &types,
            &[
                Token::Seq { len: Some(17) },
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
                Token::Str("Other"),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn as_str() {
        assert_eq!(TaskType::StartUp.as_ref(), "StartUp",);
        assert_eq!(TaskOrUnknown::Task(TaskType::StartUp).as_ref(), "StartUp");
        assert_eq!(TaskOrUnknown::Unknown("Other".into()).as_ref(), "Other");
    }
}
