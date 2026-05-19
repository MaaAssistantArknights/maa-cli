#[repr(u64)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
/// Win32 screencap methods from `AsstWin32ScreencapMethodEnum` in `AsstCaller.h`.
pub enum Win32ScreencapMethod {
    #[default]
    None = 0,
    /// `AsstWin32ScreencapMethod_GDI`
    Gdi = 1,
    /// `AsstWin32ScreencapMethod_FramePool`
    FramePool = 1 << 1,
    /// `AsstWin32ScreencapMethod_DXGI_DesktopDup`
    DxgiDesktopDup = 1 << 2,
    /// `AsstWin32ScreencapMethod_DXGI_DesktopDup_Window`
    DxgiDesktopDupWindow = 1 << 3,
    /// `AsstWin32ScreencapMethod_PrintWindow`
    PrintWindow = 1 << 4,
    /// `AsstWin32ScreencapMethod_ScreenDC`
    ScreenDc = 1 << 5,
}

impl Win32ScreencapMethod {
    pub const COUNT: usize = 7;
    pub const NAMES: [&'static str; Self::COUNT] = [
        "None",
        "GDI",
        "FramePool",
        "DXGIDesktopDup",
        "DXGIDesktopDupWindow",
        "PrintWindow",
        "ScreenDC",
    ];
    pub const VARIANTS: [Self; Self::COUNT] = [
        Self::None,
        Self::Gdi,
        Self::FramePool,
        Self::DxgiDesktopDup,
        Self::DxgiDesktopDupWindow,
        Self::PrintWindow,
        Self::ScreenDc,
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
            Self::Gdi => "GDI",
            Self::FramePool => "FramePool",
            Self::DxgiDesktopDup => "DXGIDesktopDup",
            Self::DxgiDesktopDupWindow => "DXGIDesktopDupWindow",
            Self::PrintWindow => "PrintWindow",
            Self::ScreenDc => "ScreenDC",
        }
    }
}

impl From<Win32ScreencapMethod> for u64 {
    fn from(value: Win32ScreencapMethod) -> Self {
        value as u64
    }
}

impl_unknown_error!(
    UnknownWin32ScreencapMethodError,
    Win32ScreencapMethod,
    "Win32 screencap method"
);
impl_from_str!(Win32ScreencapMethod, UnknownWin32ScreencapMethodError);

#[cfg(feature = "serde")]
impl_serde_deserialize!(Win32ScreencapMethod, "a valid Win32 screencap method");

#[cfg(feature = "serde")]
impl_serde_serialize!(Win32ScreencapMethod);

impl_debug_display!(Win32ScreencapMethod);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!("None".parse(), Ok(Win32ScreencapMethod::None));
        assert_eq!("gdi".parse(), Ok(Win32ScreencapMethod::Gdi));
        assert_eq!("FramePool".parse(), Ok(Win32ScreencapMethod::FramePool));
        assert_eq!(
            "DXGIDesktopDup".parse(),
            Ok(Win32ScreencapMethod::DxgiDesktopDup)
        );
        assert_eq!(
            "DXGIDesktopDupWindow".parse(),
            Ok(Win32ScreencapMethod::DxgiDesktopDupWindow)
        );
        assert_eq!("PrintWindow".parse(), Ok(Win32ScreencapMethod::PrintWindow));
        assert_eq!("ScreenDC".parse(), Ok(Win32ScreencapMethod::ScreenDc));

        assert_eq!(
            "Unknown".parse::<Win32ScreencapMethod>(),
            Err(UnknownWin32ScreencapMethodError("Unknown".to_owned()))
        );
        assert_eq!(
            UnknownWin32ScreencapMethodError("Unknown".to_owned()).to_string(),
            "unknown Win32 screencap method `Unknown`, expected one of `None`, `GDI`, `FramePool`, `DXGIDesktopDup`, `DXGIDesktopDupWindow`, `PrintWindow`, `ScreenDC`",
        );
    }

    #[cfg(feature = "serde")]
    mod serde {
        use serde_test::{Token, assert_de_tokens, assert_de_tokens_error, assert_ser_tokens};

        use super::*;

        #[test]
        fn deserialize() {
            let methods = [
                Win32ScreencapMethod::None,
                Win32ScreencapMethod::Gdi,
                Win32ScreencapMethod::FramePool,
                Win32ScreencapMethod::DxgiDesktopDup,
                Win32ScreencapMethod::DxgiDesktopDupWindow,
                Win32ScreencapMethod::PrintWindow,
                Win32ScreencapMethod::ScreenDc,
            ];

            assert_de_tokens(&methods, &[
                Token::Seq { len: Some(7) },
                Token::Str("None"),
                Token::Str("GDI"),
                Token::Str("FramePool"),
                Token::Str("DXGIDesktopDup"),
                Token::Str("DXGIDesktopDupWindow"),
                Token::Str("PrintWindow"),
                Token::Str("ScreenDC"),
                Token::SeqEnd,
            ]);
        }

        #[test]
        fn deserialize_error() {
            assert_de_tokens_error::<Win32ScreencapMethod>(
                &[Token::Str("Unknown")],
                "unknown variant `Unknown`, expected one of `None`, `GDI`, `FramePool`, `DXGIDesktopDup`, `DXGIDesktopDupWindow`, `PrintWindow`, `ScreenDC`",
            );

            assert_de_tokens_error::<Win32ScreencapMethod>(
                &[Token::U64(2)],
                "invalid type: integer `2`, expected a valid Win32 screencap method",
            );
        }

        #[test]
        fn serialize() {
            assert_ser_tokens(&Win32ScreencapMethod::None, &[Token::Str("None")]);
            assert_ser_tokens(&Win32ScreencapMethod::Gdi, &[Token::Str("GDI")]);
            assert_ser_tokens(&Win32ScreencapMethod::FramePool, &[Token::Str("FramePool")]);
            assert_ser_tokens(&Win32ScreencapMethod::DxgiDesktopDupWindow, &[Token::Str(
                "DXGIDesktopDupWindow",
            )]);
        }
    }

    #[test]
    fn to_str() {
        assert_eq!(Win32ScreencapMethod::None.to_str(), "None");
        assert_eq!(Win32ScreencapMethod::Gdi.to_str(), "GDI");
        assert_eq!(Win32ScreencapMethod::FramePool.to_str(), "FramePool");
        assert_eq!(
            Win32ScreencapMethod::DxgiDesktopDup.to_str(),
            "DXGIDesktopDup"
        );
        assert_eq!(
            Win32ScreencapMethod::DxgiDesktopDupWindow.to_str(),
            "DXGIDesktopDupWindow"
        );
        assert_eq!(Win32ScreencapMethod::PrintWindow.to_str(), "PrintWindow");
        assert_eq!(Win32ScreencapMethod::ScreenDc.to_str(), "ScreenDC");
    }

    #[test]
    fn to_u64() {
        assert_eq!(u64::from(Win32ScreencapMethod::None), 0);
        assert_eq!(u64::from(Win32ScreencapMethod::Gdi), 1);
        assert_eq!(u64::from(Win32ScreencapMethod::FramePool), 2);
        assert_eq!(u64::from(Win32ScreencapMethod::DxgiDesktopDup), 4);
        assert_eq!(u64::from(Win32ScreencapMethod::DxgiDesktopDupWindow), 8);
        assert_eq!(u64::from(Win32ScreencapMethod::PrintWindow), 16);
        assert_eq!(u64::from(Win32ScreencapMethod::ScreenDc), 32);
    }

    #[test]
    fn fmt() {
        assert_eq!(format!("{}", Win32ScreencapMethod::FramePool), "FramePool");
        assert_eq!(format!("{:?}", Win32ScreencapMethod::ScreenDc), "ScreenDC");
    }
}
