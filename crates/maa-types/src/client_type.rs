#[repr(u8)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientType {
    #[default]
    Official,
    Bilibili,
    Txwy,
    YoStarEN,
    YoStarJP,
    YoStarKR,
}

use ClientType::*;

impl ClientType {
    impl_enum_utils!(ClientType, 6, Official);

    pub const fn to_str(self) -> &'static str {
        match self {
            Official => "Official",
            Bilibili => "Bilibili",
            Txwy => "txwy",
            YoStarEN => "YoStarEN",
            YoStarJP => "YoStarJP",
            YoStarKR => "YoStarKR",
        }
    }

    fn from_str_opt(s: &str) -> Option<Self> {
        // Default to Official if empty
        if s.is_empty() {
            return Some(Official);
        }

        Self::VARIANTS
            .iter()
            .find(|v| v.to_str().eq_ignore_ascii_case(s))
            .copied()
    }
}

impl ClientType {
    pub const fn to_package(self) -> &'static str {
        match self {
            Official => "com.hypergryph.arknights",
            Bilibili => "com.hypergryph.arknights.bilibili",
            YoStarEN => "com.YoStarEN.Arknights",
            YoStarJP => "com.YoStarJP.Arknights",
            YoStarKR => "com.YoStarKR.Arknights",
            Txwy => "tw.txwy.and.arknights",
        }
    }

    pub const fn to_resource(self) -> Option<&'static str> {
        match self {
            Official | Bilibili => None,
            c => Some(c.to_str()),
        }
    }

    pub const fn server_time_zone(self) -> i8 {
        match self {
            Official | Bilibili | Txwy => 4,
            YoStarEN => -11,
            YoStarJP | YoStarKR => 5,
        }
    }

    /// The server type of sample used in the report.
    pub const fn server_report(self) -> Option<&'static str> {
        match self {
            Official | Bilibili => Some("CN"),
            YoStarEN => Some("US"),
            YoStarJP => Some("JP"),
            YoStarKR => Some("KR"),
            _ => None,
        }
    }
}

impl_unknown_error!(UnknownClientTypeError, ClientType, "client type");
impl_from_str!(ClientType, UnknownClientTypeError);

#[cfg(feature = "serde")]
impl_serde_deserialize!(ClientType, "a valid client type");

#[cfg(feature = "serde")]
impl_serde_serialize!(ClientType);

#[cfg(feature = "clap")]
impl clap::ValueEnum for ClientType {
    fn value_variants<'a>() -> &'a [Self] {
        &Self::VARIANTS
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.to_str()))
    }
}

impl_debug_display!(ClientType);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn from_u8() {
        assert_eq!(ClientType::from_u8(0), Some(Official));
        assert_eq!(ClientType::from_u8(1), Some(Bilibili));
        assert_eq!(ClientType::from_u8(2), Some(Txwy));
        assert_eq!(ClientType::from_u8(3), Some(YoStarEN));
        assert_eq!(ClientType::from_u8(4), Some(YoStarJP));
        assert_eq!(ClientType::from_u8(5), Some(YoStarKR));
        assert_eq!(ClientType::from_u8(6), None);
        assert_eq!(ClientType::from_u8(255), None);
    }

    #[test]
    fn parse() {
        assert_eq!("".parse(), Ok(Official));
        assert_eq!("Official".parse(), Ok(Official));
        assert_eq!("Bilibili".parse(), Ok(Bilibili));
        assert_eq!("txwy".parse(), Ok(Txwy));
        assert_eq!("TXWY".parse(), Ok(Txwy));
        assert_eq!("YoStarEN".parse(), Ok(YoStarEN));
        assert_eq!("YoStarJP".parse(), Ok(YoStarJP));
        assert_eq!("YoStarKR".parse(), Ok(YoStarKR));
        assert_eq!(
            "UnknownClientType".parse::<ClientType>(),
            Err(UnknownClientTypeError("UnknownClientType".to_owned())),
        );

        assert_eq!(
            UnknownClientTypeError("Unknown".to_owned()).to_string(),
            "unknown client type `Unknown`, expected one of `Official`, `Bilibili`, `txwy`, `YoStarEN`, `YoStarJP`, `YoStarKR`",
        )
    }

    #[cfg(feature = "serde")]
    mod serde {
        use serde_test::{Token, assert_de_tokens, assert_de_tokens_error, assert_ser_tokens};

        use super::*;

        #[test]
        fn deserialize() {
            assert_de_tokens(&ClientType::Official, &[Token::Str("Official")]);
            assert_de_tokens(&ClientType::Bilibili, &[Token::Str("Bilibili")]);
            assert_de_tokens(&ClientType::Txwy, &[Token::Str("txwy")]);
            assert_de_tokens(&ClientType::YoStarEN, &[Token::Str("YoStarEN")]);
            assert_de_tokens(&ClientType::YoStarJP, &[Token::Str("YoStarJP")]);
            assert_de_tokens(&ClientType::YoStarKR, &[Token::Str("YoStarKR")]);

            assert_de_tokens_error::<ClientType>(
                &[Token::Str("UnknownClientType")],
                "unknown variant `UnknownClientType`, expected one of \
                `Official`, `Bilibili`, `txwy`, `YoStarEN`, `YoStarJP`, `YoStarKR`",
            );

            assert_de_tokens_error::<ClientType>(
                &[Token::I8(0)],
                "invalid type: integer `0`, expected a valid client type",
            );
        }

        #[test]
        fn serialize() {
            assert_ser_tokens(&ClientType::Official, &[Token::Str("Official")]);
            assert_ser_tokens(&ClientType::Bilibili, &[Token::Str("Bilibili")]);
            assert_ser_tokens(&ClientType::Txwy, &[Token::Str("txwy")]);
            assert_ser_tokens(&ClientType::YoStarEN, &[Token::Str("YoStarEN")]);
            assert_ser_tokens(&ClientType::YoStarJP, &[Token::Str("YoStarJP")]);
            assert_ser_tokens(&ClientType::YoStarKR, &[Token::Str("YoStarKR")]);
        }
    }

    #[test]
    fn to_str() {
        assert_eq!(ClientType::Official.to_str(), "Official");
        assert_eq!(ClientType::Bilibili.to_str(), "Bilibili");
        assert_eq!(ClientType::Txwy.to_str(), "txwy");
        assert_eq!(ClientType::YoStarEN.to_str(), "YoStarEN");
        assert_eq!(ClientType::YoStarJP.to_str(), "YoStarJP");
        assert_eq!(ClientType::YoStarKR.to_str(), "YoStarKR");

        assert_eq!(ClientType::Official.to_string(), "Official");
    }

    #[test]
    fn fmt() {
        assert_eq!(format!("{}", ClientType::Official), "Official");
        assert_eq!(format!("{:?}", ClientType::Official), "Official");
    }

    #[test]
    fn to_resource() {
        assert_eq!(Official.to_resource(), None);
        assert_eq!(Bilibili.to_resource(), None);
        assert_eq!(Txwy.to_resource(), Some("txwy"));
        assert_eq!(YoStarEN.to_resource(), Some("YoStarEN"));
        assert_eq!(YoStarJP.to_resource(), Some("YoStarJP"));
        assert_eq!(YoStarKR.to_resource(), Some("YoStarKR"));
    }

    #[test]
    fn to_package() {
        assert_eq!(Official.to_package(), "com.hypergryph.arknights");
        assert_eq!(Bilibili.to_package(), "com.hypergryph.arknights.bilibili");
        assert_eq!(Txwy.to_package(), "tw.txwy.and.arknights");
        assert_eq!(YoStarEN.to_package(), "com.YoStarEN.Arknights");
        assert_eq!(YoStarJP.to_package(), "com.YoStarJP.Arknights");
        assert_eq!(YoStarKR.to_package(), "com.YoStarKR.Arknights");
    }

    #[test]
    fn to_server_time_zone() {
        assert_eq!(Official.server_time_zone(), 4);
        assert_eq!(Bilibili.server_time_zone(), 4);
        assert_eq!(Txwy.server_time_zone(), 4);
        assert_eq!(YoStarEN.server_time_zone(), -11);
        assert_eq!(YoStarJP.server_time_zone(), 5);
        assert_eq!(YoStarKR.server_time_zone(), 5);
    }

    #[test]
    fn to_server_report() {
        assert_eq!(Official.server_report(), Some("CN"));
        assert_eq!(Bilibili.server_report(), Some("CN"));
        assert_eq!(Txwy.server_report(), None);
        assert_eq!(YoStarEN.server_report(), Some("US"));
        assert_eq!(YoStarJP.server_report(), Some("JP"));
        assert_eq!(YoStarKR.server_report(), Some("KR"));
    }
}
