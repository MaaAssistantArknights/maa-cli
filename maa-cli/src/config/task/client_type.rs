use chrono::NaiveTime;
use clap::ValueEnum;

use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Copy, Default, ValueEnum)]
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

impl<'de> Deserialize<'de> for ClientType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl ClientType {
    pub fn resource(self) -> Option<&'static str> {
        match self {
            ClientType::Txwy => Some("txwy"),
            ClientType::YoStarEN => Some("YoStarEN"),
            ClientType::YoStarJP => Some("YoStarJP"),
            ClientType::YoStarKR => Some("YoStarKR"),
            _ => None,
        }
    }

    pub fn reset_time(&self) -> NaiveTime {
        let server_reset_hour = 4;
        NaiveTime::from_hms_opt(server_reset_hour, 0, 0).unwrap()
    }

    pub fn timezone(self) -> i32 {
        match self {
            ClientType::Official | ClientType::Bilibili | ClientType::Txwy => 8,
            ClientType::YoStarEN => -7,
            ClientType::YoStarJP | ClientType::YoStarKR => 9,
        }
    }

    #[cfg(target_os = "macos")]
    pub fn app(self) -> &'static str {
        match self {
            ClientType::Official | ClientType::Bilibili | ClientType::Txwy => "明日方舟",
            ClientType::YoStarEN => "Arknights",
            ClientType::YoStarJP => "アークナイツ",
            ClientType::YoStarKR => "명일방주",
        }
    }
}

impl AsRef<str> for ClientType {
    fn as_ref(&self) -> &str {
        match self {
            ClientType::Official => "Official",
            ClientType::Bilibili => "Bilibili",
            ClientType::Txwy => "txwy",
            ClientType::YoStarEN => "YoStarEN",
            ClientType::YoStarJP => "YoStarJP",
            ClientType::YoStarKR => "YoStarKR",
        }
    }
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl std::str::FromStr for ClientType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Official" | "" => Ok(ClientType::Official),
            "Bilibili" => Ok(ClientType::Bilibili),
            "Txwy" | "TXWY" | "txwy" => Ok(ClientType::Txwy),
            "YoStarEN" => Ok(ClientType::YoStarEN),
            "YoStarJP" => Ok(ClientType::YoStarJP),
            "YoStarKR" => Ok(ClientType::YoStarKR),
            _ => Err(Error::UnknownClientType),
        }
    }
}

impl TryFrom<&str> for ClientType {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[derive(Debug)]
pub enum Error {
    UnknownClientType,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::UnknownClientType => f.write_str("Unknown client type"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use crate::assert_matches;

    use super::*;

    use serde_test::{assert_de_tokens, Token};

    impl ClientType {
        fn to_token(self) -> Token {
            match self {
                ClientType::Official => Token::Str("Official"),
                ClientType::Bilibili => Token::Str("Bilibili"),
                ClientType::Txwy => Token::Str("txwy"),
                ClientType::YoStarEN => Token::Str("YoStarEN"),
                ClientType::YoStarJP => Token::Str("YoStarJP"),
                ClientType::YoStarKR => Token::Str("YoStarKR"),
            }
        }
    }

    #[test]
    fn deserialize() {
        assert_de_tokens(&ClientType::Official, &[ClientType::Official.to_token()]);
        assert_de_tokens(&ClientType::Bilibili, &[ClientType::Bilibili.to_token()]);
        assert_de_tokens(&ClientType::Txwy, &[ClientType::Txwy.to_token()]);
        assert_de_tokens(&ClientType::YoStarEN, &[ClientType::YoStarEN.to_token()]);
        assert_de_tokens(&ClientType::YoStarJP, &[ClientType::YoStarJP.to_token()]);
        assert_de_tokens(&ClientType::YoStarKR, &[ClientType::YoStarKR.to_token()]);
    }

    #[test]
    fn parse() {
        assert_matches!("".parse::<ClientType>().unwrap(), ClientType::Official);
        assert_matches!(
            "Official".parse::<ClientType>().unwrap(),
            ClientType::Official
        );
        assert_matches!(
            "Bilibili".parse::<ClientType>().unwrap(),
            ClientType::Bilibili
        );
        assert_matches!("txwy".parse::<ClientType>().unwrap(), ClientType::Txwy);
        assert_matches!("TXWY".parse::<ClientType>().unwrap(), ClientType::Txwy);
        assert_matches!(
            "YoStarEN".parse::<ClientType>().unwrap(),
            ClientType::YoStarEN
        );
        assert_matches!(
            "YoStarJP".parse::<ClientType>().unwrap(),
            ClientType::YoStarJP
        );
        assert_matches!(
            "YoStarKR".parse::<ClientType>().unwrap(),
            ClientType::YoStarKR
        );

        assert_matches!(
            "UnknownClientType".parse::<ClientType>().unwrap_err(),
            Error::UnknownClientType,
        );
    }

    #[test]
    fn client_to_resource() {
        assert_eq!(ClientType::Official.resource(), None);
        assert_eq!(ClientType::Bilibili.resource(), None);
        assert_eq!(ClientType::Txwy.resource(), Some("txwy"));
        assert_eq!(ClientType::YoStarEN.resource(), Some("YoStarEN"));
        assert_eq!(ClientType::YoStarJP.resource(), Some("YoStarJP"));
        assert_eq!(ClientType::YoStarKR.resource(), Some("YoStarKR"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn client_to_app() {
        assert_eq!(ClientType::Official.app(), "明日方舟");
        assert_eq!(ClientType::Bilibili.app(), "明日方舟");
        assert_eq!(ClientType::Txwy.app(), "明日方舟");
        assert_eq!(ClientType::YoStarEN.app(), "Arknights");
        assert_eq!(ClientType::YoStarJP.app(), "アークナイツ");
        assert_eq!(ClientType::YoStarKR.app(), "명일방주");
    }

    #[test]
    fn client_to_string() {
        assert_eq!(ClientType::Official.to_string(), "Official");
        assert_eq!(ClientType::Bilibili.to_string(), "Bilibili");
        assert_eq!(ClientType::Txwy.to_string(), "txwy");
        assert_eq!(ClientType::YoStarEN.to_string(), "YoStarEN");
        assert_eq!(ClientType::YoStarJP.to_string(), "YoStarJP");
        assert_eq!(ClientType::YoStarKR.to_string(), "YoStarKR");
    }
}
