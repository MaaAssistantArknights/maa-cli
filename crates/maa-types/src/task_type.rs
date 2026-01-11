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
    ParadoxCopilot,
    Depot,
    OperBox,
    Reclamation,
    Custom,
    SingleStep,
    VideoRecognition,
}

impl TaskType {
    impl_enum_utils!(TaskType, 17, Self::StartUp);

    impl_from_str_opt!();

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
            Self::ParadoxCopilot => "ParadoxCopilot",
            Self::Depot => "Depot",
            Self::OperBox => "OperBox",
            Self::Reclamation => "Reclamation",
            Self::Custom => "Custom",
            Self::SingleStep => "SingleStep",
            Self::VideoRecognition => "VideoRecognition",
        }
    }
}

impl_unknown_error!(UnknownTaskType, TaskType, "task type");
impl_from_str!(TaskType, UnknownTaskType);

#[cfg(feature = "serde")]
impl_serde_deserialize!(TaskType, "a valid task type");

#[cfg(feature = "serde")]
impl_serde_serialize!(TaskType);

#[cfg(feature = "ffi")]
impl maa_ffi_string::ToCString for TaskType {
    fn to_cstring(self) -> maa_ffi_string::Result<std::ffi::CString> {
        self.to_str().to_cstring()
    }
}

impl_debug_display!(TaskType);

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
        assert_eq!("ParadoxCopilot".parse(), Ok(TaskType::ParadoxCopilot));
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
            `ParadoxCopilot`, `Depot`, `OperBox`, `Reclamation`, `Custom`, `SingleStep`, `VideoRecognition`",
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
        }

        #[test]
        fn deserialize_error() {
            assert_de_tokens_error::<TaskType>(
                &[Token::Str("Unknown")],
                "unknown variant `Unknown`, expected one of `StartUp`, `CloseDown`, `Fight`, \
                `Recruit`, `Infrast`, `Mall`, `Award`, `Roguelike`, `Copilot`, `SSSCopilot`, \
                `ParadoxCopilot`, `Depot`, `OperBox`, `Reclamation`, `Custom`, `SingleStep`, `VideoRecognition`",
            );

            assert_de_tokens_error::<TaskType>(
                &[Token::U64(17)],
                "invalid type: integer `17`, expected a valid task type",
            );
        }

        #[test]
        fn serialize() {
            assert_ser_tokens(&TaskType::StartUp, &[Token::Str("StartUp")]);
            assert_ser_tokens(&TaskType::CloseDown, &[Token::Str("CloseDown")]);
            assert_ser_tokens(&TaskType::Fight, &[Token::Str("Fight")]);
            assert_ser_tokens(&TaskType::Recruit, &[Token::Str("Recruit")]);
            assert_ser_tokens(&TaskType::Infrast, &[Token::Str("Infrast")]);
            assert_ser_tokens(&TaskType::Mall, &[Token::Str("Mall")]);
            assert_ser_tokens(&TaskType::Award, &[Token::Str("Award")]);
            assert_ser_tokens(&TaskType::Roguelike, &[Token::Str("Roguelike")]);
            assert_ser_tokens(&TaskType::Copilot, &[Token::Str("Copilot")]);
            assert_ser_tokens(&TaskType::SSSCopilot, &[Token::Str("SSSCopilot")]);
            assert_ser_tokens(&TaskType::ParadoxCopilot, &[Token::Str("ParadoxCopilot")]);
            assert_ser_tokens(&TaskType::Depot, &[Token::Str("Depot")]);
            assert_ser_tokens(&TaskType::OperBox, &[Token::Str("OperBox")]);
            assert_ser_tokens(&TaskType::Reclamation, &[Token::Str("Reclamation")]);
            assert_ser_tokens(&TaskType::Custom, &[Token::Str("Custom")]);
            assert_ser_tokens(&TaskType::SingleStep, &[Token::Str("SingleStep")]);
            assert_ser_tokens(&TaskType::VideoRecognition, &[Token::Str(
                "VideoRecognition",
            )]);
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
        assert_eq!(TaskType::ParadoxCopilot.to_str(), "ParadoxCopilot");
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
