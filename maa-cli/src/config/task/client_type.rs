use serde::Deserialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Copy, Deserialize)]
pub enum ClientType {
    Official,
    Bilibili,
    #[serde(alias = "txwy", alias = "TXWY")]
    Txwy,
    YoStarEN,
    YoStarJP,
    YoStarKR,
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
            "txwy" => Ok(ClientType::Txwy),
            "YoStarEN" => Ok(ClientType::YoStarEN),
            "YoStarJP" => Ok(ClientType::YoStarJP),
            "YoStarKR" => Ok(ClientType::YoStarKR),
            _ => Err(Error::UnknownClientType),
        }
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
    use super::*;

    #[test]
    fn parse_client() {
        assert_eq!(ClientType::Official, "Official".parse().unwrap());
        assert_eq!(ClientType::Official, "".parse().unwrap());
        assert_eq!(ClientType::Bilibili, "Bilibili".parse().unwrap());
        assert_eq!(ClientType::Txwy, "txwy".parse().unwrap());
        assert_eq!(ClientType::YoStarEN, "YoStarEN".parse().unwrap());
        assert_eq!(ClientType::YoStarJP, "YoStarJP".parse().unwrap());
        assert_eq!(ClientType::YoStarKR, "YoStarKR".parse().unwrap());
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
}
