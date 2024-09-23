use clap::ValueEnum;
use serde::Deserialize;

#[repr(u8)]
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, Default, ValueEnum, PartialEq)]
#[clap(rename_all = "verbatim")]
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
        let mut names = ["Official"; Self::COUNT];
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
            variants[i] = unsafe { std::mem::transmute::<u8, ClientType>(i as u8) };
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

    pub const fn resource(self) -> Option<&'static str> {
        match self {
            Txwy => Some("txwy"),
            YoStarEN => Some("YoStarEN"),
            YoStarJP => Some("YoStarJP"),
            YoStarKR => Some("YoStarKR"),
            _ => None,
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn app(self) -> &'static str {
        match self {
            Official | Bilibili | Txwy => "明日方舟",
            YoStarEN => "Arknights",
            YoStarJP => "アークナイツ",
            YoStarKR => "명일방주",
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

impl<'de> Deserialize<'de> for ClientType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ClientTypeVisitor;

        impl<'de> serde::de::Visitor<'de> for ClientTypeVisitor {
            type Value = ClientType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string representing a client type")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                ClientType::from_str_opt(value)
                    .ok_or_else(|| E::unknown_variant(value, &ClientType::NAMES))
            }
        }

        deserializer.deserialize_str(ClientTypeVisitor)
    }
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl std::str::FromStr for ClientType {
    type Err = UnknownClientTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_opt(s).ok_or(UnknownClientTypeError)
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct UnknownClientTypeError;

impl std::fmt::Display for UnknownClientTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Unknown client type")
    }
}

impl std::error::Error for UnknownClientTypeError {}

#[cfg(test)]
mod tests {
    use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

    use super::*;

    impl ClientType {
        const fn to_token(self) -> Token {
            Token::Str(self.to_str())
        }
    }

    #[test]
    fn deserialize() {
        assert_de_tokens(&Official, &[Official.to_token()]);
        assert_de_tokens(&Bilibili, &[Bilibili.to_token()]);
        assert_de_tokens(&Txwy, &[Txwy.to_token()]);
        assert_de_tokens(&YoStarEN, &[YoStarEN.to_token()]);
        assert_de_tokens(&YoStarJP, &[YoStarJP.to_token()]);
        assert_de_tokens(&YoStarKR, &[YoStarKR.to_token()]);

        assert_de_tokens_error::<ClientType>(
            &[Token::Str("UnknownClientType")],
            "unknown variant `UnknownClientType`, expected one of \
            `Official`, `Bilibili`, `txwy`, `YoStarEN`, `YoStarJP`, `YoStarKR`",
        );

        assert_de_tokens_error::<ClientType>(
            &[Token::I8(0)],
            "invalid type: integer `0`, expected a string representing a client type",
        );
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
            Err(UnknownClientTypeError),
        );

        assert_eq!(UnknownClientTypeError.to_string(), "Unknown client type",)
    }

    #[test]
    fn to_resource() {
        assert_eq!(Official.resource(), None);
        assert_eq!(Bilibili.resource(), None);
        assert_eq!(Txwy.resource(), Some("txwy"));
        assert_eq!(YoStarEN.resource(), Some("YoStarEN"));
        assert_eq!(YoStarJP.resource(), Some("YoStarJP"));
        assert_eq!(YoStarKR.resource(), Some("YoStarKR"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn to_app() {
        assert_eq!(Official.app(), "明日方舟");
        assert_eq!(Bilibili.app(), "明日方舟");
        assert_eq!(Txwy.app(), "明日方舟");
        assert_eq!(YoStarEN.app(), "Arknights");
        assert_eq!(YoStarJP.app(), "アークナイツ");
        assert_eq!(YoStarKR.app(), "명일방주");
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

    #[test]
    fn to_str() {
        assert_eq!(Official.to_str(), "Official");
        assert_eq!(Bilibili.to_str(), "Bilibili");
        assert_eq!(Txwy.to_str(), "txwy");
        assert_eq!(YoStarEN.to_str(), "YoStarEN");
        assert_eq!(YoStarJP.to_str(), "YoStarJP");
        assert_eq!(YoStarKR.to_str(), "YoStarKR");

        assert_eq!(Official.to_string(), "Official");
    }
}
