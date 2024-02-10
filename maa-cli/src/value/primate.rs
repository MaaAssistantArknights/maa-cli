use super::MAAValue;

use serde::{Deserialize, Serialize};

#[cfg_attr(test, derive(Debug))]
#[derive(Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum MAAPrimate {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
}

impl MAAPrimate {
    pub(super) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }
    pub(super) fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn as_float(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub(super) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }
}

impl Serialize for MAAPrimate {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Bool(v) => serializer.serialize_bool(*v),
            Self::Int(v) => serializer.serialize_i32(*v),
            Self::Float(v) => serializer.serialize_f32(*v),
            Self::String(v) => serializer.serialize_str(v),
        }
    }
}

impl PartialEq<MAAPrimate> for MAAValue {
    fn eq(&self, other: &MAAPrimate) -> bool {
        match self {
            Self::Primate(v) => v == other,
            _ => false,
        }
    }
}

impl From<bool> for MAAPrimate {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i32> for MAAPrimate {
    fn from(v: i32) -> Self {
        Self::Int(v)
    }
}

impl From<f32> for MAAPrimate {
    fn from(v: f32) -> Self {
        Self::Float(v)
    }
}

impl From<String> for MAAPrimate {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for MAAPrimate {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl From<MAAPrimate> for MAAValue {
    fn from(v: MAAPrimate) -> Self {
        Self::Primate(v)
    }
}

macro_rules! impl_from {
    ($($t:ty),*) => {
        $(
            impl From<$t> for MAAValue {
                fn from(v: $t) -> Self {
                    Self::Primate(v.into())
                }
            }
        )*
    };
}

impl_from!(bool, i32, f32, String, &str);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        use serde_test::{assert_de_tokens, Token};

        let values = vec![
            MAAPrimate::Bool(true),
            MAAPrimate::Int(1),
            MAAPrimate::Float(1.0),
            MAAPrimate::String("".to_string()),
        ];

        assert_de_tokens(
            &values,
            &[
                Token::Seq { len: Some(4) },
                Token::Bool(true),
                Token::I32(1),
                Token::F32(1.0),
                Token::Str(""),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn as_type() {
        assert_eq!(MAAPrimate::Bool(true).as_bool(), Some(true));
        assert_eq!(MAAPrimate::Bool(true).as_int(), None);
        assert_eq!(MAAPrimate::Bool(true).as_float(), None);
        assert_eq!(MAAPrimate::Bool(true).as_str(), None);

        assert_eq!(MAAPrimate::Int(1).as_bool(), None);
        assert_eq!(MAAPrimate::Int(1).as_int(), Some(1));
        assert_eq!(MAAPrimate::Int(1).as_float(), None);
        assert_eq!(MAAPrimate::Int(1).as_str(), None);

        assert_eq!(MAAPrimate::Float(1.0).as_bool(), None);
        assert_eq!(MAAPrimate::Float(1.0).as_int(), None);
        assert_eq!(MAAPrimate::Float(1.0).as_float(), Some(1.0));
        assert_eq!(MAAPrimate::Float(1.0).as_str(), None);

        assert_eq!(MAAPrimate::String("".to_string()).as_bool(), None);
        assert_eq!(MAAPrimate::String("".to_string()).as_int(), None);
        assert_eq!(MAAPrimate::String("".to_string()).as_float(), None);
        assert_eq!(MAAPrimate::String("".to_string()).as_str(), Some(""));
    }
}
