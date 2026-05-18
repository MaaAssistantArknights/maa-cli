#[repr(u64)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
/// Win32 input methods from `AsstWin32InputMethodEnum` in `AsstCaller.h`.
pub enum Win32InputMethod {
    #[default]
    None = 0,
    /// `AsstWin32InputMethod_Seize`
    Seize = 1,
    /// `AsstWin32InputMethod_SendMessage`
    SendMessage = 1 << 1,
    /// `AsstWin32InputMethod_PostMessage`
    PostMessage = 1 << 2,
    /// `AsstWin32InputMethod_LegacyEvent`
    LegacyEvent = 1 << 3,
    /// `AsstWin32InputMethod_PostThreadMessage`
    PostThreadMessage = 1 << 4,
    /// `AsstWin32InputMethod_SendMessageWithCursorPos`
    SendMessageWithCursorPos = 1 << 5,
    /// `AsstWin32InputMethod_PostMessageWithCursorPos`
    PostMessageWithCursorPos = 1 << 6,
    /// `AsstWin32InputMethod_SendMessageWithWindowPos`
    SendMessageWithWindowPos = 1 << 7,
    /// `AsstWin32InputMethod_PostMessageWithWindowPos`
    PostMessageWithWindowPos = 1 << 8,
}

impl Win32InputMethod {
    pub const COUNT: usize = 10;

    pub const NAMES: [&'static str; Self::COUNT] = [
        "None",
        "Seize",
        "SendMessage",
        "PostMessage",
        "LegacyEvent",
        "PostThreadMessage",
        "SendMessageWithCursorPos",
        "PostMessageWithCursorPos",
        "SendMessageWithWindowPos",
        "PostMessageWithWindowPos",
    ];

    pub const VARIANTS: [Self; Self::COUNT] = [
        Self::None,
        Self::Seize,
        Self::SendMessage,
        Self::PostMessage,
        Self::LegacyEvent,
        Self::PostThreadMessage,
        Self::SendMessageWithCursorPos,
        Self::PostMessageWithCursorPos,
        Self::SendMessageWithWindowPos,
        Self::PostMessageWithWindowPos,
    ];

    fn from_str_opt(s: &str) -> Option<Self> {
        Self::VARIANTS
            .iter()
            .find(|v| v.to_str().eq_ignore_ascii_case(s))
            .copied()
    }

    pub const fn to_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Seize => "Seize",
            Self::SendMessage => "SendMessage",
            Self::PostMessage => "PostMessage",
            Self::LegacyEvent => "LegacyEvent",
            Self::PostThreadMessage => "PostThreadMessage",
            Self::SendMessageWithCursorPos => "SendMessageWithCursorPos",
            Self::PostMessageWithCursorPos => "PostMessageWithCursorPos",
            Self::SendMessageWithWindowPos => "SendMessageWithWindowPos",
            Self::PostMessageWithWindowPos => "PostMessageWithWindowPos",
        }
    }
}

impl From<Win32InputMethod> for u64 {
    fn from(value: Win32InputMethod) -> Self {
        value as u64
    }
}

impl_unknown_error!(
    UnknownWin32InputMethodError,
    Win32InputMethod,
    "Win32 input method"
);
impl_from_str!(Win32InputMethod, UnknownWin32InputMethodError);

#[cfg(feature = "serde")]
impl_serde_deserialize!(Win32InputMethod, "a valid Win32 input method");

#[cfg(feature = "serde")]
impl_serde_serialize!(Win32InputMethod);

impl_debug_display!(Win32InputMethod);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!("None".parse(), Ok(Win32InputMethod::None));
        assert_eq!("Seize".parse(), Ok(Win32InputMethod::Seize));
        assert_eq!("sendmessage".parse(), Ok(Win32InputMethod::SendMessage));
        assert_eq!("PostMessage".parse(), Ok(Win32InputMethod::PostMessage));
        assert_eq!("LegacyEvent".parse(), Ok(Win32InputMethod::LegacyEvent));
        assert_eq!(
            "PostThreadMessage".parse(),
            Ok(Win32InputMethod::PostThreadMessage)
        );
        assert_eq!(
            "SendMessageWithCursorPos".parse(),
            Ok(Win32InputMethod::SendMessageWithCursorPos)
        );
        assert_eq!(
            "PostMessageWithCursorPos".parse(),
            Ok(Win32InputMethod::PostMessageWithCursorPos)
        );
        assert_eq!(
            "SendMessageWithWindowPos".parse(),
            Ok(Win32InputMethod::SendMessageWithWindowPos)
        );
        assert_eq!(
            "PostMessageWithWindowPos".parse(),
            Ok(Win32InputMethod::PostMessageWithWindowPos)
        );

        assert_eq!(
            "Unknown".parse::<Win32InputMethod>(),
            Err(UnknownWin32InputMethodError("Unknown".to_owned()))
        );
        assert_eq!(
            UnknownWin32InputMethodError("Unknown".to_owned()).to_string(),
            "unknown Win32 input method `Unknown`, expected one of `None`, `Seize`, `SendMessage`, `PostMessage`, `LegacyEvent`, `PostThreadMessage`, `SendMessageWithCursorPos`, `PostMessageWithCursorPos`, `SendMessageWithWindowPos`, `PostMessageWithWindowPos`",
        );
    }

    #[cfg(feature = "serde")]
    mod serde {
        use serde_test::{Token, assert_de_tokens, assert_de_tokens_error, assert_ser_tokens};

        use super::*;

        #[test]
        fn deserialize() {
            let methods = [
                Win32InputMethod::None,
                Win32InputMethod::Seize,
                Win32InputMethod::SendMessage,
                Win32InputMethod::PostMessage,
                Win32InputMethod::LegacyEvent,
                Win32InputMethod::PostThreadMessage,
                Win32InputMethod::SendMessageWithCursorPos,
                Win32InputMethod::PostMessageWithCursorPos,
                Win32InputMethod::SendMessageWithWindowPos,
                Win32InputMethod::PostMessageWithWindowPos,
            ];

            assert_de_tokens(
                &methods,
                &[
                    Token::Seq { len: Some(10) },
                    Token::Str("None"),
                    Token::Str("Seize"),
                    Token::Str("SendMessage"),
                    Token::Str("PostMessage"),
                    Token::Str("LegacyEvent"),
                    Token::Str("PostThreadMessage"),
                    Token::Str("SendMessageWithCursorPos"),
                    Token::Str("PostMessageWithCursorPos"),
                    Token::Str("SendMessageWithWindowPos"),
                    Token::Str("PostMessageWithWindowPos"),
                    Token::SeqEnd,
                ],
            );
        }

        #[test]
        fn deserialize_error() {
            assert_de_tokens_error::<Win32InputMethod>(
                &[Token::Str("Unknown")],
                "unknown variant `Unknown`, expected one of `None`, `Seize`, `SendMessage`, `PostMessage`, `LegacyEvent`, `PostThreadMessage`, `SendMessageWithCursorPos`, `PostMessageWithCursorPos`, `SendMessageWithWindowPos`, `PostMessageWithWindowPos`",
            );

            assert_de_tokens_error::<Win32InputMethod>(
                &[Token::U64(32)],
                "invalid type: integer `32`, expected a valid Win32 input method",
            );
        }

        #[test]
        fn serialize() {
            assert_ser_tokens(&Win32InputMethod::Seize, &[Token::Str("Seize")]);
            assert_ser_tokens(&Win32InputMethod::SendMessage, &[Token::Str("SendMessage")]);
            assert_ser_tokens(
                &Win32InputMethod::SendMessageWithCursorPos,
                &[Token::Str("SendMessageWithCursorPos")],
            );
        }
    }

    #[test]
    fn to_str() {
        assert_eq!(Win32InputMethod::None.to_str(), "None");
        assert_eq!(Win32InputMethod::Seize.to_str(), "Seize");
        assert_eq!(Win32InputMethod::SendMessage.to_str(), "SendMessage");
        assert_eq!(
            Win32InputMethod::PostMessageWithWindowPos.to_str(),
            "PostMessageWithWindowPos"
        );
    }

    #[test]
    fn to_u64() {
        assert_eq!(u64::from(Win32InputMethod::None), 0);
        assert_eq!(u64::from(Win32InputMethod::Seize), 1);
        assert_eq!(u64::from(Win32InputMethod::SendMessage), 2);
        assert_eq!(u64::from(Win32InputMethod::PostMessage), 4);
        assert_eq!(u64::from(Win32InputMethod::LegacyEvent), 8);
        assert_eq!(u64::from(Win32InputMethod::PostThreadMessage), 16);
        assert_eq!(u64::from(Win32InputMethod::SendMessageWithCursorPos), 32);
        assert_eq!(u64::from(Win32InputMethod::PostMessageWithCursorPos), 64);
        assert_eq!(u64::from(Win32InputMethod::SendMessageWithWindowPos), 128);
        assert_eq!(u64::from(Win32InputMethod::PostMessageWithWindowPos), 256);
    }

    #[test]
    fn fmt() {
        assert_eq!(
            format!("{}", Win32InputMethod::SendMessageWithCursorPos),
            "SendMessageWithCursorPos"
        );
        assert_eq!(
            format!("{:?}", Win32InputMethod::SendMessage),
            "SendMessage"
        );
    }
}
