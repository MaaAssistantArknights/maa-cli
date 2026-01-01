use serde::{Deserialize, Serialize};

use super::MAAValue;

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Debug, Clone, PartialEq)]
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        use serde_test::{Token, assert_de_tokens};

        let values = vec![
            MAAPrimate::Bool(true),
            MAAPrimate::Int(1),
            MAAPrimate::Float(1.0),
            MAAPrimate::String("".to_string()),
        ];

        assert_de_tokens(&values, &[
            Token::Seq { len: Some(4) },
            Token::Bool(true),
            Token::I32(1),
            Token::F32(1.0),
            Token::Str(""),
            Token::SeqEnd,
        ]);
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

    #[test]
    fn serialize() {
        use serde_test::{Token, assert_ser_tokens};

        assert_ser_tokens(&MAAPrimate::Bool(true), &[Token::Bool(true)]);
        assert_ser_tokens(&MAAPrimate::Bool(false), &[Token::Bool(false)]);
        assert_ser_tokens(&MAAPrimate::Int(42), &[Token::I32(42)]);
        assert_ser_tokens(&MAAPrimate::Int(-10), &[Token::I32(-10)]);
        assert_ser_tokens(&MAAPrimate::Float(1.5), &[Token::F32(1.5)]);
        assert_ser_tokens(&MAAPrimate::Float(-2.5), &[Token::F32(-2.5)]);
        assert_ser_tokens(&MAAPrimate::String("hello".to_string()), &[Token::Str(
            "hello",
        )]);
        assert_ser_tokens(&MAAPrimate::String("".to_string()), &[Token::Str("")]);
    }

    #[test]
    fn from_primitives() {
        // Test From implementations for each primitive type
        let primate: MAAPrimate = true.into();
        assert_eq!(primate, MAAPrimate::Bool(true));

        let primate: MAAPrimate = 42.into();
        assert_eq!(primate, MAAPrimate::Int(42));

        let primate: MAAPrimate = 1.5f32.into();
        assert_eq!(primate, MAAPrimate::Float(1.5));

        let primate: MAAPrimate = "hello".to_string().into();
        assert_eq!(primate, MAAPrimate::String("hello".to_string()));

        let primate: MAAPrimate = "world".into();
        assert_eq!(primate, MAAPrimate::String("world".to_string()));
    }

    #[test]
    fn to_maa_value() {
        // Test conversion from primitives to MAAValue (via MAAPrimate)
        let value: MAAValue = true.into();
        assert_eq!(value.as_bool(), Some(true));

        let value: MAAValue = 42.into();
        assert_eq!(value.as_int(), Some(42));

        let value: MAAValue = 1.5f32.into();
        assert_eq!(value.as_float(), Some(1.5));

        let value: MAAValue = "test".to_string().into();
        assert_eq!(value.as_str(), Some("test"));

        let value: MAAValue = "str".into();
        assert_eq!(value.as_str(), Some("str"));
    }

    #[test]
    fn maa_value_eq_primate() {
        // Test PartialEq between MAAValue and MAAPrimate
        assert_eq!(MAAValue::from(true), MAAPrimate::Bool(true));
        assert_ne!(MAAValue::from(true), MAAPrimate::Bool(false));
        assert_ne!(MAAValue::from(true), MAAPrimate::Int(1));

        assert_eq!(MAAValue::from(42), MAAPrimate::Int(42));
        assert_ne!(MAAValue::from(42), MAAPrimate::Int(43));
        assert_ne!(MAAValue::from(42), MAAPrimate::Bool(true));

        assert_eq!(MAAValue::from(1.5f32), MAAPrimate::Float(1.5));
        assert_ne!(MAAValue::from(1.5f32), MAAPrimate::Float(2.5));

        assert_eq!(
            MAAValue::from("test"),
            MAAPrimate::String("test".to_string())
        );
        assert_ne!(
            MAAValue::from("test"),
            MAAPrimate::String("other".to_string())
        );

        // Test that non-primate values don't equal primate
        let array = MAAValue::Array(vec![1.into()]);
        assert_ne!(array, MAAPrimate::Int(1));

        let object = MAAValue::default();
        assert_ne!(object, MAAPrimate::Bool(true));
    }

    #[test]
    fn edge_cases() {
        // Test edge case values
        assert_eq!(MAAPrimate::Int(i32::MAX).as_int(), Some(i32::MAX));
        assert_eq!(MAAPrimate::Int(i32::MIN).as_int(), Some(i32::MIN));
        assert_eq!(
            MAAPrimate::Float(f32::INFINITY).as_float(),
            Some(f32::INFINITY)
        );
        assert_eq!(
            MAAPrimate::Float(f32::NEG_INFINITY).as_float(),
            Some(f32::NEG_INFINITY)
        );
        assert!(MAAPrimate::Float(f32::NAN).as_float().unwrap().is_nan());

        // Test very long strings
        let long_string = "a".repeat(10000);
        let primate: MAAPrimate = long_string.as_str().into();
        assert_eq!(primate.as_str(), Some(long_string.as_str()));
    }
}
