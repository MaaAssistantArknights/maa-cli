use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
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

impl MAATask {
    fn to_str(self) -> &'static str {
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

#[derive(Deserialize, Debug, Clone, PartialEq)]
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
            TaskOrUnknown::MAATask(task) => task.to_str(),
            TaskOrUnknown::Unknown(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for TaskOrUnknown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
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
    fn to_str() {
        assert_eq!(MAATask::StartUp.to_str(), "StartUp");
        assert_eq!(MAATask::CloseDown.to_str(), "CloseDown");
        assert_eq!(MAATask::Fight.to_str(), "Fight");
        assert_eq!(MAATask::Recruit.to_str(), "Recruit");
        assert_eq!(MAATask::Infrast.to_str(), "Infrast");
        assert_eq!(MAATask::Mall.to_str(), "Mall");
        assert_eq!(MAATask::Award.to_str(), "Award");
        assert_eq!(MAATask::Roguelike.to_str(), "Roguelike");
        assert_eq!(MAATask::Copilot.to_str(), "Copilot");
        assert_eq!(MAATask::SSSCopilot.to_str(), "SSSCopilot");
        assert_eq!(MAATask::Depot.to_str(), "Depot");
        assert_eq!(MAATask::OperBox.to_str(), "OperBox");
        assert_eq!(
            MAATask::ReclamationAlgorithm.to_str(),
            "ReclamationAlgorithm",
        );
        assert_eq!(MAATask::Custom.to_str(), "Custom");
        assert_eq!(MAATask::SingleStep.to_str(), "SingleStep");
        assert_eq!(MAATask::VideoRecognition.to_str(), "VideoRecognition");
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
