pub mod input;

use std::fmt::Display;

use input::UserInput;

use serde::{Deserialize, Serialize};

#[cfg_attr(test, derive(PartialEq))]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Value {
    InputString(UserInput<String>),
    InputBool(UserInput<bool>),
    InputInt(UserInput<i64>),
    InputFloat(UserInput<f64>),
    Object(Map<Value>),
    Array(Vec<Value>),
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    Null,
}

impl Default for Value {
    fn default() -> Self {
        Self::Object(Default::default())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl<const N: usize> From<[(String, Value); N]> for Value {
    fn from(value: [(String, Value); N]) -> Self {
        Self::Object(Map::from(value))
    }
}

impl<const N: usize> From<[(&str, Value); N]> for Value {
    fn from(value: [(&str, Value); N]) -> Self {
        Self::Object(Map::from(value.map(|(k, v)| (k.to_string(), v))))
    }
}

impl<const N: usize> From<[Value; N]> for Value {
    fn from(value: [Value; N]) -> Self {
        Self::Array(value.into())
    }
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Self> {
        if let Self::Object(map) = self {
            if let Some(value) = map.get(key) {
                return Some(value);
            }
        }
        None
    }

    /// Get value with key or return default value
    ///
    /// This will try to convert the value to the type of the default value.
    /// If the key does not exist, the default value will be returned.
    pub fn get_or<'a, T>(&'a self, key: &str, default: T) -> std::result::Result<T, T::Error>
    where
        T: TryFrom<&'a Self>,
    {
        if let Self::Object(map) = self {
            if let Some(value) = map.get(key) {
                return value.try_into();
            }
        }
        Ok(default)
    }

    pub fn set(&mut self, key: &str, value: impl Into<Self>) {
        if let Self::Object(map) = self {
            map.insert(key.to_string(), value.into());
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_) | Self::InputBool(_))
    }

    pub fn as_bool(&self) -> TryFromResult<bool> {
        match self {
            Self::InputBool(v) => Ok(v.get()?),
            Self::Bool(v) => Ok(*v),
            _ => Err(TryFromError::TypeMismatch),
        }
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(_) | Self::InputInt(_))
    }

    pub fn as_int(&self) -> TryFromResult<i64> {
        match self {
            Self::InputInt(v) => Ok(v.get()?),
            Self::Int(v) => Ok(*v),
            _ => Err(TryFromError::TypeMismatch),
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_) | Self::InputFloat(_))
    }

    pub fn as_float(&self) -> TryFromResult<f64> {
        match self {
            Self::InputFloat(v) => Ok(v.get()?),
            Self::Float(v) => Ok(*v),
            _ => Err(TryFromError::TypeMismatch),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_) | Self::InputString(_))
    }

    pub fn as_string(&self) -> TryFromResult<String> {
        match self {
            Self::InputString(v) => Ok(v.get()?),
            Self::String(v) => Ok(v.clone()),
            _ => Err(TryFromError::TypeMismatch),
        }
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    pub fn as_array(&self) -> TryFromResult<&Vec<Self>> {
        match self {
            Self::Array(v) => Ok(v),
            _ => Err(TryFromError::TypeMismatch),
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub fn as_object(&self) -> TryFromResult<&Map<Self>> {
        match self {
            Self::Object(v) => Ok(v),
            _ => Err(TryFromError::TypeMismatch),
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut ret = self.clone();
        ret.merge_mut(other);
        ret
    }

    pub fn merge_mut(&mut self, other: &Self) {
        match (self, other) {
            (Self::Object(self_map), Self::Object(other_map)) => {
                for (key, value) in other_map {
                    if let Some(self_value) = self_map.get_mut(key) {
                        self_value.merge_mut(value);
                    } else {
                        self_map.insert(key.clone(), value.clone());
                    }
                }
            }
            (s, o) => *s = o.clone(),
        }
    }
}

impl TryFrom<&Value> for bool {
    type Error = TryFromError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.as_bool()
    }
}

impl TryFrom<&Value> for i64 {
    type Error = TryFromError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.as_int()
    }
}

impl TryFrom<&Value> for f64 {
    type Error = TryFromError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.as_float()
    }
}

impl TryFrom<&Value> for String {
    type Error = TryFromError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value.as_string()
    }
}

type TryFromResult<T> = Result<T, TryFromError>;

#[derive(Debug)]
pub enum TryFromError {
    TypeMismatch,
    IOError(std::io::Error),
}

impl From<std::io::Error> for TryFromError {
    fn from(error: std::io::Error) -> Self {
        Self::IOError(error)
    }
}

impl Display for TryFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryFromError::TypeMismatch => write!(f, "type mismatch"),
            TryFromError::IOError(error) => write!(f, "io error: {}", error),
        }
    }
}

impl std::error::Error for TryFromError {}

pub type Map<T> = std::collections::BTreeMap<String, T>;

#[cfg(test)]
mod tests {
    use super::*;

    mod serde {
        use super::*;

        use serde_test::{assert_tokens, Token};

        #[test]
        fn value() {
            let mut map = Map::new();
            map.insert("bool".to_string(), Value::Bool(true));
            map.insert("int".to_string(), Value::Int(1.into()));
            map.insert("float".to_string(), Value::Float(1.0));
            map.insert("string".to_string(), Value::String("string".to_string()));
            map.insert(
                "array".to_string(),
                Value::Array(vec![Value::Int(1.into()), Value::Int(2.into())]),
            );

            let mut sub_map = Map::new();
            sub_map.insert("key".to_string(), Value::String("value".to_string()));
            map.insert("object".to_string(), Value::Object(sub_map));

            let value = Value::Object(map);

            assert_tokens(
                &value,
                &[
                    Token::Map { len: Some(6) },
                    Token::Str("array"),
                    Token::Seq { len: Some(2) },
                    Token::I64(1),
                    Token::I64(2),
                    Token::SeqEnd,
                    Token::Str("bool"),
                    Token::Bool(true),
                    Token::Str("float"),
                    Token::F64(1.0),
                    Token::Str("int"),
                    Token::I64(1),
                    Token::Str("object"),
                    Token::Map { len: Some(1) },
                    Token::Str("key"),
                    Token::Str("value"),
                    Token::MapEnd,
                    Token::Str("string"),
                    Token::Str("string"),
                    Token::MapEnd,
                ],
            )
        }

        #[test]
        fn array() {
            let value = Value::Array(vec![
                Value::Bool(true),
                Value::Int(1.into()),
                Value::Float(1.0),
                Value::String("string".to_string()),
                Value::Array(vec![Value::Int(1.into()), Value::Int(2.into())]),
                Value::Object({
                    let mut map = Map::new();
                    map.insert("key".to_string(), Value::String("value".to_string()));
                    map
                }),
            ]);
            assert_tokens(
                &value,
                &[
                    Token::Seq { len: Some(6) },
                    Token::Bool(true),
                    Token::I64(1),
                    Token::F64(1.0),
                    Token::Str("string"),
                    Token::Seq { len: Some(2) },
                    Token::I64(1),
                    Token::I64(2),
                    Token::SeqEnd,
                    Token::Map { len: Some(1) },
                    Token::Str("key"),
                    Token::Str("value"),
                    Token::MapEnd,
                    Token::SeqEnd,
                ],
            )
        }

        #[test]
        fn bool() {
            let boolean = Value::Bool(true);
            assert_tokens(&boolean, &[Token::Bool(true)]);
        }

        #[test]
        fn int() {
            let integer = Value::Int(1);
            assert_tokens(&integer, &[Token::I64(1)]);
        }

        #[test]
        fn float() {
            let float = Value::Float(1.0);
            assert_tokens(&float, &[Token::F64(1.0)]);
        }

        #[test]
        fn string() {
            let string = Value::String("string".to_string());
            assert_tokens(&string, &[Token::Str("string")]);
        }

        #[test]
        fn null() {
            let null = Value::Null;
            assert_tokens(&null, &[Token::Unit]);
        }
    }

    #[test]
    fn is_sth() {
        assert!(Value::Null.is_null());
        assert!(Value::from(true).is_bool());
        assert!(Value::from(1).is_int());
        assert!(Value::from(1.0).is_float());
        assert!(Value::from(String::from("string")).is_string());
        assert!(Value::from("string").is_string());
        assert!(Value::from([Value::from(1), Value::from(2)]).is_array());
        assert!(Value::from([(String::from("key"), Value::from("value"))]).is_object());
    }

    #[test]
    fn try_from() {
        let bool_value = Value::from(true);
        assert_eq!(bool::try_from(&bool_value).unwrap(), true);
        assert!(matches!(
            i64::try_from(&bool_value),
            Err(TryFromError::TypeMismatch)
        ));
        let int_value = Value::from(1);
        assert_eq!(i64::try_from(&int_value).unwrap(), 1);
        assert!(matches!(
            bool::try_from(&int_value),
            Err(TryFromError::TypeMismatch)
        ));
        let float_value = Value::from(1.0);
        assert_eq!(f64::try_from(&float_value).unwrap(), 1.0);
        assert!(matches!(
            bool::try_from(&float_value),
            Err(TryFromError::TypeMismatch)
        ));
        let string_value = Value::from("string");
        assert_eq!(String::try_from(&string_value).unwrap(), "string");
        assert!(matches!(
            bool::try_from(&string_value),
            Err(TryFromError::TypeMismatch)
        ));
    }

    #[test]
    fn merge() {
        let mut map_base = Map::new();
        map_base.insert("bool".to_string(), Value::Bool(true));
        map_base.insert("int".to_string(), Value::Int(1.into()));
        map_base.insert("float".to_string(), Value::Float(1.0));
        map_base.insert("string".to_string(), Value::String("string".to_string()));
        map_base.insert(
            "array".to_string(),
            Value::Array(vec![Value::Int(1.into()), Value::Int(2.into())]),
        );

        let mut sub_map = Map::new();
        sub_map.insert("key1".to_string(), Value::String("value1".to_string()));
        sub_map.insert("key2".to_string(), Value::String("value2".to_string()));

        map_base.insert("object".to_string(), Value::Object(sub_map));

        let value = Value::Object(map_base);

        let mut map_other = Map::new();
        map_other.insert("bool".to_string(), Value::Bool(false));
        map_other.insert("int".to_string(), Value::Int(2.into()));
        map_other.insert(
            "array".to_string(),
            Value::Array(vec![Value::Int(3.into()), Value::Int(4.into())]),
        );

        let mut sub_map2 = Map::new();
        sub_map2.insert("key2".to_string(), Value::String("value2_2".to_string()));
        sub_map2.insert("key3".to_string(), Value::String("value3".to_string()));

        map_other.insert("object".to_string(), Value::Object(sub_map2));

        let value2 = Value::Object(map_other);

        let value_merged = value.merge(&value2);

        let mut map_expected = Map::new();
        map_expected.insert("bool".to_string(), Value::Bool(false));
        map_expected.insert("int".to_string(), Value::Int(2.into()));
        map_expected.insert("float".to_string(), Value::Float(1.0));
        map_expected.insert("string".to_string(), Value::String("string".to_string()));
        map_expected.insert(
            "array".to_string(),
            Value::Array(vec![Value::Int(3.into()), Value::Int(4.into())]),
        );

        let mut sub_map_expected = Map::new();
        sub_map_expected.insert("key1".to_string(), Value::String("value1".to_string()));
        sub_map_expected.insert("key2".to_string(), Value::String("value2_2".to_string()));
        sub_map_expected.insert("key3".to_string(), Value::String("value3".to_string()));
        map_expected.insert("object".to_string(), Value::Object(sub_map_expected));

        let value_expected = Value::Object(map_expected);

        assert_eq!(value_merged, value_expected);
    }
}
