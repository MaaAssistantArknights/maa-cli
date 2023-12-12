use serde::Deserialize;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone, Copy)]
pub enum MAATask {
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

impl AsRef<str> for MAATask {
    fn as_ref(&self) -> &str {
        match self {
            MAATask::StartUp => "StartUp",
            MAATask::CloseDown => "CloseDown",
            MAATask::Fight => "Fight",
            MAATask::Recruit => "Recruit",
            MAATask::Infrast => "Infrast",
            MAATask::Mall => "Mall",
            MAATask::Award => "Award",
            MAATask::Roguelike => "Roguelike",
            MAATask::Copilot => "Copilot",
            MAATask::SSSCopilot => "SSSCopilot",
            MAATask::Depot => "Depot",
            MAATask::OperBox => "OperBox",
            MAATask::ReclamationAlgorithm => "ReclamationAlgorithm",
            MAATask::Custom => "Custom",
            MAATask::SingleStep => "SingleStep",
            MAATask::VideoRecognition => "VideoRecognition",
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TaskOrUnknown {
    MAATask(MAATask),
    Unknown(String),
}

impl From<MAATask> for TaskOrUnknown {
    fn from(task: MAATask) -> Self {
        TaskOrUnknown::MAATask(task)
    }
}

impl AsRef<str> for TaskOrUnknown {
    fn as_ref(&self) -> &str {
        match self {
            TaskOrUnknown::MAATask(task) => task.as_ref(),
            TaskOrUnknown::Unknown(s) => s.as_str(),
        }
    }
}

impl maa_sys::ToCString for &TaskOrUnknown {
    fn to_cstring(self) -> maa_sys::Result<std::ffi::CString> {
        self.as_ref().to_cstring()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_test::{assert_de_tokens, Token};

    #[test]
    fn deserialize() {
        let types: [TaskOrUnknown; 17] = [
            MAATask::StartUp.into(),
            MAATask::CloseDown.into(),
            MAATask::Fight.into(),
            MAATask::Recruit.into(),
            MAATask::Infrast.into(),
            MAATask::Mall.into(),
            MAATask::Award.into(),
            MAATask::Roguelike.into(),
            MAATask::Copilot.into(),
            MAATask::SSSCopilot.into(),
            MAATask::Depot.into(),
            MAATask::OperBox.into(),
            MAATask::ReclamationAlgorithm.into(),
            MAATask::Custom.into(),
            MAATask::SingleStep.into(),
            MAATask::VideoRecognition.into(),
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
        assert_eq!(MAATask::StartUp.as_ref(), "StartUp");
        assert_eq!(MAATask::CloseDown.as_ref(), "CloseDown");
        assert_eq!(MAATask::Fight.as_ref(), "Fight");
        assert_eq!(MAATask::Recruit.as_ref(), "Recruit");
        assert_eq!(MAATask::Infrast.as_ref(), "Infrast");
        assert_eq!(MAATask::Mall.as_ref(), "Mall");
        assert_eq!(MAATask::Award.as_ref(), "Award");
        assert_eq!(MAATask::Roguelike.as_ref(), "Roguelike");
        assert_eq!(MAATask::Copilot.as_ref(), "Copilot");
        assert_eq!(MAATask::SSSCopilot.as_ref(), "SSSCopilot");
        assert_eq!(MAATask::Depot.as_ref(), "Depot");
        assert_eq!(MAATask::OperBox.as_ref(), "OperBox");
        assert_eq!(
            MAATask::ReclamationAlgorithm.as_ref(),
            "ReclamationAlgorithm",
        );
        assert_eq!(MAATask::Custom.as_ref(), "Custom");
        assert_eq!(MAATask::SingleStep.as_ref(), "SingleStep");
        assert_eq!(MAATask::VideoRecognition.as_ref(), "VideoRecognition");
        assert_eq!(TaskOrUnknown::MAATask(MAATask::StartUp).as_ref(), "StartUp");
        assert_eq!(TaskOrUnknown::Unknown("Other".into()).as_ref(), "Other");
    }

    #[test]
    fn to_cstring() {
        use maa_sys::ToCString;
        use std::ffi::CString;

        assert_eq!(
            TaskOrUnknown::MAATask(MAATask::StartUp)
                .to_cstring()
                .unwrap(),
            CString::new("StartUp").unwrap(),
        );

        assert_eq!(
            TaskOrUnknown::Unknown("Other".into()).to_cstring().unwrap(),
            CString::new("Other").unwrap(),
        );
    }
}
