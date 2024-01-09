#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone)]
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
    Unknown(String),
}

impl TaskType {
    pub fn to_str(&self) -> &str {
        use TaskType::*;
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
            Unknown(s) => s.as_str(),
        }
    }

    pub fn to_fl_string(&self) -> String {
        use TaskType::*;
        match self {
            StartUp => fl!("task-type-startup"),
            CloseDown => fl!("task-type-closedown"),
            Fight => fl!("task-type-fight"),
            Recruit => fl!("task-type-recruit"),
            Infrast => fl!("task-type-infrast"),
            Mall => fl!("task-type-mall"),
            Award => fl!("task-type-award"),
            Roguelike => fl!("task-type-roguelike"),
            Copilot => fl!("task-type-copilot"),
            SSSCopilot => fl!("task-type-ssscopilot"),
            Depot => fl!("task-type-depot"),
            OperBox => fl!("task-type-operbox"),
            ReclamationAlgorithm => fl!("task-type-reclamationalgorithm"),
            Custom => fl!("task-type-custom"),
            SingleStep => fl!("task-type-singlestep"),
            VideoRecognition => fl!("task-type-videorecognition"),
            Unknown(s) => s.clone(),
        }
    }
}

impl std::str::FromStr for TaskType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use TaskType::*;
        Ok(match s {
            "StartUp" => StartUp,
            "CloseDown" => CloseDown,
            "Fight" => Fight,
            "Recruit" => Recruit,
            "Infrast" => Infrast,
            "Mall" => Mall,
            "Award" => Award,
            "Roguelike" => Roguelike,
            "Copilot" => Copilot,
            "SSSCopilot" => SSSCopilot,
            "Depot" => Depot,
            "OperBox" => OperBox,
            "ReclamationAlgorithm" => ReclamationAlgorithm,
            "Custom" => Custom,
            "SingleStep" => SingleStep,
            "VideoRecognition" => VideoRecognition,
            _ => {
                warn!("unknown-task-type", task_type = s);
                Unknown(s.to_string())
            }
        })
    }
}

impl<'de> serde::Deserialize<'de> for TaskType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<TaskType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TaskTypeVisitor;

        impl<'de> serde::de::Visitor<'de> for TaskTypeVisitor {
            type Value = TaskType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string representing a task type")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<TaskType, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(TaskTypeVisitor)
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl maa_sys::ToCString for &TaskType {
    fn to_cstring(self) -> maa_sys::Result<std::ffi::CString> {
        self.to_str().to_cstring()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TaskType::*;

    use serde_test::{assert_de_tokens, Token};

    #[test]
    fn deserialize() {
        let types: [TaskType; 17] = [
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
            TaskType::ReclamationAlgorithm.to_str(),
            "ReclamationAlgorithm",
        );
        assert_eq!(Custom.to_str(), "Custom");
        assert_eq!(SingleStep.to_str(), "SingleStep");
        assert_eq!(VideoRecognition.to_str(), "VideoRecognition");
        assert_eq!(Unknown("Other".into()).to_str(), "Other");
    }

    #[test]
    fn to_cstring() {
        use maa_sys::ToCString;
        use std::ffi::CString;

        assert_eq!(
            StartUp.to_cstring().unwrap(),
            CString::new("StartUp").unwrap(),
        );

        assert_eq!(
            Unknown("Other".into()).to_cstring().unwrap(),
            CString::new("Other").unwrap(),
        );
    }
}
