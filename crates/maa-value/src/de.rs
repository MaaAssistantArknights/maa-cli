//! Deserializer implementation for MAAValue
//!
//! This allows direct conversion from MAAValue to any type implementing Deserialize,
//! without going through an intermediate format like serde_json::Value.
//!
//! # Example
//!
//! ```
//! use maa_value::prelude::*;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Debug, PartialEq)]
//! struct Config {
//!     name: String,
//!     count: i32,
//! }
//!
//! let object = object!("name" => "app", "count" => 42);
//!
//! // Direct conversion - no intermediate JSON!
//! let config = Config::deserialize(object).unwrap();
//!
//! assert_eq!(config.name, "app");
//! assert_eq!(config.count, 42);
//! ```

use serde::de::{
    self, Deserializer, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};

use crate::{primitive::MAAPrimitive, value::MAAValue};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("{0}")]
    Custom(String),
    #[error("expected {expected}, got {actual}")]
    TypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },
    #[error("expected enum (string or single-key object), got {actual}")]
    ExpectedEnum { actual: &'static str },
    #[error("value is missing")]
    MissingMapValue,
    #[error("expected unit variant")]
    ExpectedUnitVariant,
    #[error("expected newtype variant payload")]
    ExpectedNewtypeVariantPayload,
    #[error("expected tuple variant payload")]
    ExpectedTupleVariantPayload,
    #[error("expected struct variant payload")]
    ExpectedStructVariantPayload,
}

type Result<T> = std::result::Result<T, Error>;

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl Error {
    fn type_mismatch(expected: &'static str, actual: &'static str) -> Self {
        Self::TypeMismatch { expected, actual }
    }
}

impl MAAValue {
    fn type_name(&self) -> &'static str {
        match self {
            MAAValue::Primitive(primitive) => primitive.type_name(),
            MAAValue::Array(_) => "array",
            MAAValue::Object(_) => "object",
        }
    }
}

impl MAAPrimitive {
    fn type_name(&self) -> &'static str {
        match self {
            MAAPrimitive::Bool(_) => "boolean",
            MAAPrimitive::Int(_) => "integer",
            MAAPrimitive::Float(_) => "float",
            MAAPrimitive::String(_) => "string",
        }
    }
}

impl<'de> Deserializer<'de> for MAAValue {
    type Error = Error;

    // Forward all other deserialize_* methods to deserialize_any
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            MAAValue::Primitive(p) => deserialize_primitive(p, visitor),
            MAAValue::Array(arr) => visitor.visit_seq(SeqDeserializer {
                iter: arr.into_iter(),
            }),
            MAAValue::Object(map) => visitor.visit_map(MapDeserializer {
                iter: map.into_iter(),
                value: None,
            }),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            MAAValue::Object(map) => visitor.visit_map(MapDeserializer {
                iter: map.into_iter(),
                value: None,
            }),
            value => Err(Error::type_mismatch("struct (object)", value.type_name())),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            MAAValue::Primitive(MAAPrimitive::String(variant)) => {
                visitor.visit_enum(EnumDeserializer {
                    variant,
                    value: None,
                })
            }
            MAAValue::Object(mut map) if map.len() == 1 => {
                let (variant, value) = map.shift_remove_index(0).unwrap();
                visitor.visit_enum(EnumDeserializer {
                    variant,
                    value: Some(value),
                })
            }
            value => Err(Error::ExpectedEnum {
                actual: value.type_name(),
            }),
        }
    }
}

fn deserialize_primitive<'de, V>(p: MAAPrimitive, visitor: V) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    match p {
        MAAPrimitive::Bool(b) => visitor.visit_bool(b),
        MAAPrimitive::Int(i) => visitor.visit_i32(i),
        MAAPrimitive::Float(f) => visitor.visit_f32(f),
        MAAPrimitive::String(s) => visitor.visit_string(s),
    }
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<MAAValue>,
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct MapDeserializer {
    iter: indexmap::map::IntoIter<String, MAAValue>,
    value: Option<MAAValue>,
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(Error::MissingMapValue),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct EnumDeserializer {
    variant: String,
    value: Option<MAAValue>,
}

struct VariantDeserializer {
    value: Option<MAAValue>,
}

impl<'de> EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let Self { variant, value } = self;
        let variant = seed.deserialize(variant.into_deserializer())?;
        Ok((variant, VariantDeserializer { value }))
    }
}

impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.value {
            None => Ok(()),
            Some(_) => Err(Error::ExpectedUnitVariant),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value.ok_or(Error::ExpectedNewtypeVariantPayload)?)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(MAAValue::Array(arr)) => visitor.visit_seq(SeqDeserializer {
                iter: arr.into_iter(),
            }),
            _ => Err(Error::ExpectedTupleVariantPayload),
        }
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(MAAValue::Object(map)) => visitor.visit_map(MapDeserializer {
                iter: map.into_iter(),
                value: None,
            }),
            _ => Err(Error::ExpectedStructVariantPayload),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::Error;
    use crate::prelude::*;

    #[test]
    fn deserialize_numeric() {
        assert_eq!(u8::deserialize(MAAValue::from(42)).unwrap(), 42u8);
        assert_eq!(u16::deserialize(MAAValue::from(42)).unwrap(), 42u16);
        assert_eq!(u32::deserialize(MAAValue::from(42)).unwrap(), 42u32);
        assert_eq!(u64::deserialize(MAAValue::from(42)).unwrap(), 42u64);
        assert_eq!(usize::deserialize(MAAValue::from(42)).unwrap(), 42usize);

        assert_eq!(i8::deserialize(MAAValue::from(42)).unwrap(), 42i8);
        assert_eq!(i16::deserialize(MAAValue::from(42)).unwrap(), 42i16);
        assert_eq!(i32::deserialize(MAAValue::from(42)).unwrap(), 42i32);
        assert_eq!(i64::deserialize(MAAValue::from(42)).unwrap(), 42i64);
        assert_eq!(isize::deserialize(MAAValue::from(42)).unwrap(), 42isize);

        assert_eq!(f32::deserialize(MAAValue::from(42)).unwrap(), 42.0f32);
        assert_eq!(f64::deserialize(MAAValue::from(42)).unwrap(), 42.0f64);

        assert_eq!(f32::deserialize(MAAValue::from(42.0)).unwrap(), 42.0f32);
        assert_eq!(f64::deserialize(MAAValue::from(42.0)).unwrap(), 42.0f64);

        assert!(u8::deserialize(MAAValue::from(300)).is_err());
        assert!(i8::deserialize(MAAValue::from(200)).is_err());
        assert!(u8::deserialize(MAAValue::from(-1)).is_err());
        assert!(u8::deserialize(MAAValue::from(-1.0)).is_err());
    }

    #[test]
    fn deserialize_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            count: i32,
            enabled: bool,
        }

        let resolved = object!(
            "name" => "my-app",
            "count" => 42,
            "enabled" => true
        );

        // Direct conversion!
        let config = Config::deserialize(resolved).unwrap();

        assert_eq!(config.name, "my-app");
        assert_eq!(config.count, 42);
        assert!(config.enabled);
    }

    #[test]
    fn deserialize_nested_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Inner {
            value: String,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct Outer {
            name: String,
            inner: Inner,
        }

        let resolved = object!(
            "name" => "outer",
            "inner" => object!("value" => "inner_value")
        );

        let config = Outer::deserialize(resolved).unwrap();

        assert_eq!(config.name, "outer");
        assert_eq!(config.inner.value, "inner_value");
    }

    #[test]
    fn deserialize_array() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            items: Vec<i32>,
        }

        let resolved = object!("items" => [1, 2, 3, 4, 5]);

        let config = Config::deserialize(resolved).unwrap();

        assert_eq!(config.items, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn deserialize_optional_fields() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            required: String,
            optional: Option<i32>,
        }

        let value = object!("required" => "value");
        let config = Config::deserialize(value).unwrap();
        assert_eq!(config.required, "value");
        assert_eq!(config.optional, None);

        let value = object!("required" => "value", "optional" => 42);
        let config = Config::deserialize(value).unwrap();
        assert_eq!(config.required, "value");
        assert_eq!(config.optional, Some(42));
    }

    #[test]
    fn deserialize_externally_tagged_enum_variants() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum Example {
            Unit,
            Newtype(i32),
            Tuple(i32, String),
            Struct { value: i32 },
        }

        assert_eq!(
            Example::deserialize(MAAValue::from("Unit")).unwrap(),
            Example::Unit
        );
        assert_eq!(
            Example::deserialize(object!("Newtype" => 42)).unwrap(),
            Example::Newtype(42)
        );
        assert_eq!(
            Example::deserialize(object!(
                "Tuple" => MAAValue::Array(vec![1.into(), "two".into()])
            ))
            .unwrap(),
            Example::Tuple(1, "two".to_string())
        );
        assert_eq!(
            Example::deserialize(object!("Struct" => object!("value" => 7))).unwrap(),
            Example::Struct { value: 7 }
        );
    }

    #[test]
    fn deserialize_struct_with_enum_field() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum Mode {
            Fast,
            Custom { retries: i32 },
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            mode: Mode,
        }

        assert_eq!(
            Config::deserialize(object!("mode" => "Fast")).unwrap(),
            Config { mode: Mode::Fast }
        );
        assert_eq!(
            Config::deserialize(object!(
                "mode" => object!("Custom" => object!("retries" => 3))
            ))
            .unwrap(),
            Config {
                mode: Mode::Custom { retries: 3 }
            }
        );
    }

    #[test]
    fn deserialize_struct_reports_type_mismatch() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            value: i32,
        }

        assert_eq!(
            Config::deserialize(MAAValue::from(42)).unwrap_err(),
            Error::TypeMismatch {
                expected: "struct (object)",
                actual: "integer",
            }
        );
    }

    #[test]
    fn deserialize_internally_tagged_enum() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(tag = "type")]
        enum Command {
            Move { x: i32, y: i32 },
            Stop,
        }

        assert_eq!(
            Command::deserialize(object!("type" => "Move", "x" => 1, "y" => 2)).unwrap(),
            Command::Move { x: 1, y: 2 }
        );
        assert_eq!(
            Command::deserialize(object!("type" => "Stop")).unwrap(),
            Command::Stop
        );
    }

    #[test]
    fn deserialize_adjacently_tagged_enum() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(tag = "t", content = "c")]
        enum Payload {
            Num(i32),
            Text(String),
        }

        assert_eq!(
            Payload::deserialize(object!("t" => "Num", "c" => 42)).unwrap(),
            Payload::Num(42)
        );
        assert_eq!(
            Payload::deserialize(object!("t" => "Text", "c" => "hello")).unwrap(),
            Payload::Text("hello".to_string())
        );
    }

    #[test]
    fn deserialize_untagged_enum() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(untagged)]
        enum Value {
            Int(i32),
            Text(String),
        }

        assert_eq!(
            Value::deserialize(MAAValue::from(42)).unwrap(),
            Value::Int(42)
        );
        assert_eq!(
            Value::deserialize(MAAValue::from("hello")).unwrap(),
            Value::Text("hello".to_string())
        );
    }

    #[test]
    fn deserialize_enum_reports_semantic_error() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum Example {
            Unit,
        }

        assert_eq!(
            Example::deserialize(MAAValue::from(42)).unwrap_err(),
            Error::ExpectedEnum { actual: "integer" }
        );
    }
}
