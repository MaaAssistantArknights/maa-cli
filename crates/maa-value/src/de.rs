//! Deserializer implementation for ResolvedMAAValue
//!
//! This allows direct conversion from ResolvedMAAValue to any type implementing Deserialize,
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
//! let resolved = object!("name" => "app", "count" => 42).resolve().unwrap();
//!
//! // Direct conversion - no intermediate JSON!
//! let config = Config::deserialize(resolved).unwrap();
//!
//! assert_eq!(config.name, "app");
//! assert_eq!(config.count, 42);
//! ```

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::{primitive::MAAPrimitive, value::ResolvedMAAValue};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(String);

type Result<T> = std::result::Result<T, Error>;

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl<'de> Deserializer<'de> for ResolvedMAAValue {
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
            ResolvedMAAValue::Primitive(p) => deserialize_primitive(p, visitor),
            ResolvedMAAValue::Array(arr) => visitor.visit_seq(SeqDeserializer {
                iter: arr.into_iter(),
            }),
            ResolvedMAAValue::Object(map) => visitor.visit_map(MapDeserializer {
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
            ResolvedMAAValue::Object(map) => visitor.visit_map(MapDeserializer {
                iter: map.into_iter(),
                value: None,
            }),
            _ => Err(de::Error::custom("expected struct (object)")),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Enums are not directly supported in ResolvedMAAValue
        Err(de::Error::custom("enums are not supported"))
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
    iter: std::vec::IntoIter<ResolvedMAAValue>,
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
    iter: std::collections::btree_map::IntoIter<String, ResolvedMAAValue>,
    value: Option<ResolvedMAAValue>,
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
                seed.deserialize(KeyDeserializer { key }).map(Some)
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
            None => Err(de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct KeyDeserializer {
    key: String,
}

impl<'de> Deserializer<'de> for KeyDeserializer {
    type Error = Error;

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::prelude::*;

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
        )
        .resolve()
        .unwrap();

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
        )
        .resolve()
        .unwrap();

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

        let resolved = object!("items" => [1, 2, 3, 4, 5]).resolve().unwrap();

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

        let resolved = object!("required" => "value").resolve().unwrap();

        let config = Config::deserialize(resolved).unwrap();

        assert_eq!(config.required, "value");
        assert_eq!(config.optional, None);
    }

    #[test]
    fn compare_with_json_approach() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            count: i32,
        }

        let resolved = object!("name" => "app", "count" => 100).resolve().unwrap();

        // Old way: via JSON
        let json = serde_json::to_value(&resolved).unwrap();
        let config1: Config = serde_json::from_value(json).unwrap();

        // New way: direct
        let config2 = Config::deserialize(resolved.clone()).unwrap();

        assert_eq!(config1, config2);
    }
}
