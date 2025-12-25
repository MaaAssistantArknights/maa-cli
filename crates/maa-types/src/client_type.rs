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
    pub const COUNT: usize = 6;
    pub const NAMES: [&'static str; Self::COUNT] = {
        let mut i = 0;
        let mut names = [""; Self::COUNT];
        while i < Self::COUNT {
            names[i] = Self::VARIANTS[i].to_str();
            i += 1;
        }
        names
    };
    pub const VARIANTS: [ClientType; Self::COUNT] = {
        let mut i = 0;
        let mut variants = [Official; Self::COUNT];
        while i < Self::COUNT {
            variants[i] = unsafe { Self::from_u8_unchecked(i as u8) };
            i += 1;
        }
        variants
    };

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

        Self::NAMES
            .iter()
            .position(|&name| name.eq_ignore_ascii_case(s))
            .map(|i| Self::VARIANTS[i])
    }

    pub const fn from_u8(value: u8) -> Option<Self> {
        if Self::COUNT > value as usize {
            Some(unsafe { Self::from_u8_unchecked(value) })
        } else {
            None
        }
    }

    const unsafe fn from_u8_unchecked(value: u8) -> Self {
        unsafe { std::mem::transmute(value) }
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

impl std::str::FromStr for ClientType {
    type Err = UnknownClientTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_opt(s).ok_or_else(|| UnknownClientTypeError(s.to_owned()))
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug)]
pub struct UnknownClientTypeError(String);

impl std::fmt::Display for UnknownClientTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown client type `{}`, expected one of ", self.0)?;
        let mut iter = ClientType::NAMES.iter();
        if let Some(name) = iter.next() {
            write!(f, "`{name}`")?;
            for v in iter {
                write!(f, ", `{v}`")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for UnknownClientTypeError {}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ClientType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ClientTypeVisitor;

        impl serde::de::Visitor<'_> for ClientTypeVisitor {
            type Value = ClientType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid client type")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                ClientType::from_str_opt(value)
                    .ok_or_else(|| E::unknown_variant(value, &ClientType::NAMES))
            }
        }

        deserializer.deserialize_str(ClientTypeVisitor)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ClientType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_str())
    }
}

#[cfg(feature = "clap")]
impl clap::ValueEnum for ClientType {
    fn value_variants<'a>() -> &'a [Self] {
        &Self::VARIANTS
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.to_str()))
    }
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl std::fmt::Debug for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

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
            assert_ser_tokens(&ClientType::Official, &[Token::U64(0)]);
            assert_ser_tokens(&ClientType::Bilibili, &[Token::U64(1)]);
            assert_ser_tokens(&ClientType::Txwy, &[Token::U64(2)]);
            assert_ser_tokens(&ClientType::YoStarEN, &[Token::U64(3)]);
            assert_ser_tokens(&ClientType::YoStarJP, &[Token::U64(4)]);
            assert_ser_tokens(&ClientType::YoStarKR, &[Token::U64(5)]);
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
