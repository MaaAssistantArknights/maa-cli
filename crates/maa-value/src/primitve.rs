use serde::{Deserialize, Serialize};

use super::MAAValue;

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum MAAPrimitive {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
}

impl MAAPrimitive {
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

impl Serialize for MAAPrimitive {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Bool(v) => serializer.serialize_bool(*v),
            Self::Int(v) => serializer.serialize_i32(*v),
            Self::Float(v) => serializer.serialize_f32(*v),
            Self::String(v) => serializer.serialize_str(v),
        }
    }
}

impl PartialEq<MAAPrimitive> for MAAValue {
    fn eq(&self, other: &MAAPrimitive) -> bool {
        match self {
            Self::Primitive(v) => v == other,
            _ => false,
        }
    }
}

impl From<bool> for MAAPrimitive {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i32> for MAAPrimitive {
    fn from(v: i32) -> Self {
        Self::Int(v)
    }
}

impl From<f32> for MAAPrimitive {
    fn from(v: f32) -> Self {
        Self::Float(v)
    }
}

impl From<String> for MAAPrimitive {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for MAAPrimitive {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl From<MAAPrimitive> for MAAValue {
    fn from(v: MAAPrimitive) -> Self {
        Self::Primitive(v)
    }
}

macro_rules! impl_from {
    ($($t:ty),*) => {
        $(
            impl From<$t> for MAAValue {
                fn from(v: $t) -> Self {
                    Self::Primitive(v.into())
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
            MAAPrimitive::Bool(true),
            MAAPrimitive::Int(1),
            MAAPrimitive::Float(1.0),
            MAAPrimitive::String("".to_string()),
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
        assert_eq!(MAAPrimitive::Bool(true).as_bool(), Some(true));
        assert_eq!(MAAPrimitive::Bool(true).as_int(), None);
        assert_eq!(MAAPrimitive::Bool(true).as_float(), None);
        assert_eq!(MAAPrimitive::Bool(true).as_str(), None);

        assert_eq!(MAAPrimitive::Int(1).as_bool(), None);
        assert_eq!(MAAPrimitive::Int(1).as_int(), Some(1));
        assert_eq!(MAAPrimitive::Int(1).as_float(), None);
        assert_eq!(MAAPrimitive::Int(1).as_str(), None);

        assert_eq!(MAAPrimitive::Float(1.0).as_bool(), None);
        assert_eq!(MAAPrimitive::Float(1.0).as_int(), None);
        assert_eq!(MAAPrimitive::Float(1.0).as_float(), Some(1.0));
        assert_eq!(MAAPrimitive::Float(1.0).as_str(), None);

        assert_eq!(MAAPrimitive::String("".to_string()).as_bool(), None);
        assert_eq!(MAAPrimitive::String("".to_string()).as_int(), None);
        assert_eq!(MAAPrimitive::String("".to_string()).as_float(), None);
        assert_eq!(MAAPrimitive::String("".to_string()).as_str(), Some(""));
    }

    #[test]
    fn serialize() {
        use serde_test::{Token, assert_ser_tokens};

        assert_ser_tokens(&MAAPrimitive::Bool(true), &[Token::Bool(true)]);
        assert_ser_tokens(&MAAPrimitive::Bool(false), &[Token::Bool(false)]);
        assert_ser_tokens(&MAAPrimitive::Int(42), &[Token::I32(42)]);
        assert_ser_tokens(&MAAPrimitive::Int(-10), &[Token::I32(-10)]);
        assert_ser_tokens(&MAAPrimitive::Float(1.5), &[Token::F32(1.5)]);
        assert_ser_tokens(&MAAPrimitive::Float(-2.5), &[Token::F32(-2.5)]);
        assert_ser_tokens(&MAAPrimitive::String("hello".to_string()), &[Token::Str(
            "hello",
        )]);
        assert_ser_tokens(&MAAPrimitive::String("".to_string()), &[Token::Str("")]);
    }

    #[test]
    fn from_primitives() {
        // Test From implementations for each primitive type
        let primitive: MAAPrimitive = true.into();
        assert_eq!(primitive, MAAPrimitive::Bool(true));

        let primitive: MAAPrimitive = 42.into();
        assert_eq!(primitive, MAAPrimitive::Int(42));

        let primitive: MAAPrimitive = 1.5f32.into();
        assert_eq!(primitive, MAAPrimitive::Float(1.5));

        let primitive: MAAPrimitive = "hello".to_string().into();
        assert_eq!(primitive, MAAPrimitive::String("hello".to_string()));

        let primitive: MAAPrimitive = "world".into();
        assert_eq!(primitive, MAAPrimitive::String("world".to_string()));
    }

    #[test]
    fn to_maa_value() {
        // Test conversion from primitives to MAAValue (via MAAPrimitive)
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
    fn maa_value_eq_primitive() {
        // Test PartialEq between MAAValue and MAAPrimitive
        assert_eq!(MAAValue::from(true), MAAPrimitive::Bool(true));
        assert_ne!(MAAValue::from(true), MAAPrimitive::Bool(false));
        assert_ne!(MAAValue::from(true), MAAPrimitive::Int(1));

        assert_eq!(MAAValue::from(42), MAAPrimitive::Int(42));
        assert_ne!(MAAValue::from(42), MAAPrimitive::Int(43));
        assert_ne!(MAAValue::from(42), MAAPrimitive::Bool(true));

        assert_eq!(MAAValue::from(1.5f32), MAAPrimitive::Float(1.5));
        assert_ne!(MAAValue::from(1.5f32), MAAPrimitive::Float(2.5));

        assert_eq!(
            MAAValue::from("test"),
            MAAPrimitive::String("test".to_string())
        );
        assert_ne!(
            MAAValue::from("test"),
            MAAPrimitive::String("other".to_string())
        );

        // Test that non-Primitive values don't equal Primitive
        let array = MAAValue::Array(vec![1.into()]);
        assert_ne!(array, MAAPrimitive::Int(1));

        let object = MAAValue::default();
        assert_ne!(object, MAAPrimitive::Bool(true));
    }

    #[test]
    fn edge_cases() {
        // Test edge case values
        assert_eq!(MAAPrimitive::Int(i32::MAX).as_int(), Some(i32::MAX));
        assert_eq!(MAAPrimitive::Int(i32::MIN).as_int(), Some(i32::MIN));
        assert_eq!(
            MAAPrimitive::Float(f32::INFINITY).as_float(),
            Some(f32::INFINITY)
        );
        assert_eq!(
            MAAPrimitive::Float(f32::NEG_INFINITY).as_float(),
            Some(f32::NEG_INFINITY)
        );
        assert!(MAAPrimitive::Float(f32::NAN).as_float().unwrap().is_nan());

        // Test very long strings
        let long_string = "a".repeat(10000);
        let primitive: MAAPrimitive = long_string.as_str().into();
        assert_eq!(primitive.as_str(), Some(long_string.as_str()));
    }
}
