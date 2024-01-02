use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
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
use MAATask::*;

impl MAATask {
    fn to_str(self) -> &'static str {
        match self {
            StartUp => "StartUp",
            CloseDown => "CloseDown",
            Fight => "Fight",
            Recruit => "Recruit",
            Infrast => "Infrast",
            Mall => "Mall",
            Award => "Award",
            Roguelike => "Roguelike",
            Copilot => "Copilot",
            SSSCopilot => "SSSCopilot",
            Depot => "Depot",
            OperBox => "OperBox",
            ReclamationAlgorithm => "ReclamationAlgorithm",
            Custom => "Custom",
            SingleStep => "SingleStep",
            VideoRecognition => "VideoRecognition",
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum TaskOrUnknown {
    Task(MAATask),
    Unknown(String),
}
use TaskOrUnknown::*;

impl From<MAATask> for TaskOrUnknown {
    fn from(task: MAATask) -> Self {
        Task(task)
    }
}

impl AsRef<str> for TaskOrUnknown {
    fn as_ref(&self) -> &str {
        match self {
            Task(task) => task.to_str(),
            Unknown(s) => s.as_str(),
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

    impl PartialEq<MAATask> for TaskOrUnknown {
        fn eq(&self, other: &MAATask) -> bool {
            match self {
                Task(task) => task == other,
                Unknown(_) => false,
            }
        }
    }

    #[test]
    fn deserialize() {
        let types: [TaskOrUnknown; 17] = [
            StartUp.into(),
            CloseDown.into(),
            Fight.into(),
            Recruit.into(),
            Infrast.into(),
            Mall.into(),
            Award.into(),
            Roguelike.into(),
            Copilot.into(),
            SSSCopilot.into(),
            Depot.into(),
            OperBox.into(),
            ReclamationAlgorithm.into(),
            Custom.into(),
            SingleStep.into(),
            VideoRecognition.into(),
            Unknown("Other".to_string()),
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
            MAATask::ReclamationAlgorithm.to_str(),
            "ReclamationAlgorithm",
        );
        assert_eq!(Custom.to_str(), "Custom");
        assert_eq!(SingleStep.to_str(), "SingleStep");
        assert_eq!(VideoRecognition.to_str(), "VideoRecognition");
        assert_eq!(Task(StartUp).as_ref(), "StartUp");
        assert_eq!(Unknown("Other".into()).as_ref(), "Other");
    }

    #[test]
    fn to_cstring() {
        use maa_sys::ToCString;
        use std::ffi::CString;

        assert_eq!(
            Task(StartUp).to_cstring().unwrap(),
            CString::new("StartUp").unwrap(),
        );

        assert_eq!(
            Unknown("Other".into()).to_cstring().unwrap(),
            CString::new("Other").unwrap(),
        );
    }
}
