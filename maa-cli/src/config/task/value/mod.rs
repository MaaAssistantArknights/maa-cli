pub mod input;

use std::fmt::Display;

use input::{BoolInput, Input, InputOrSelect, Select};

use serde::{Deserialize, Serialize};

#[cfg_attr(test, derive(PartialEq))]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MAAValue {
    Array(Vec<MAAValue>),
    InputString(InputOrSelect<String>),
    InputBool(BoolInput),
    InputInt(InputOrSelect<i64>),
    InputFloat(InputOrSelect<f64>),
    Object(Map<MAAValue>),
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    Null,
}

impl Default for MAAValue {
    fn default() -> Self {
        Self::new()
    }
}

impl From<bool> for MAAValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<BoolInput> for MAAValue {
    fn from(value: BoolInput) -> Self {
        Self::InputBool(value)
    }
}

impl From<i64> for MAAValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<InputOrSelect<i64>> for MAAValue {
    fn from(value: InputOrSelect<i64>) -> Self {
        Self::InputInt(value)
    }
}

impl From<Input<i64>> for MAAValue {
    fn from(value: Input<i64>) -> Self {
        Self::InputInt(InputOrSelect::Input(value))
    }
}

impl From<Select<i64>> for MAAValue {
    fn from(value: Select<i64>) -> Self {
        Self::InputInt(InputOrSelect::Select(value))
    }
}

impl From<f64> for MAAValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<InputOrSelect<f64>> for MAAValue {
    fn from(value: InputOrSelect<f64>) -> Self {
        Self::InputFloat(value)
    }
}

impl From<Input<f64>> for MAAValue {
    fn from(value: Input<f64>) -> Self {
        Self::InputFloat(InputOrSelect::Input(value))
    }
}

impl From<Select<f64>> for MAAValue {
    fn from(value: Select<f64>) -> Self {
        Self::InputFloat(InputOrSelect::Select(value))
    }
}

impl From<String> for MAAValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<InputOrSelect<String>> for MAAValue {
    fn from(value: InputOrSelect<String>) -> Self {
        Self::InputString(value)
    }
}

impl From<Input<String>> for MAAValue {
    fn from(value: Input<String>) -> Self {
        Self::InputString(InputOrSelect::Input(value))
    }
}

impl From<Select<String>> for MAAValue {
    fn from(value: Select<String>) -> Self {
        Self::InputString(InputOrSelect::Select(value))
    }
}

impl From<&str> for MAAValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl<const N: usize, S: Into<String>, V: Into<MAAValue>> From<[(S, V); N]> for MAAValue {
    fn from(value: [(S, V); N]) -> Self {
        Self::Object(Map::from(value.map(|(k, v)| (k.into(), v.into()))))
    }
}

impl<const N: usize, T: Into<MAAValue>> From<[T; N]> for MAAValue {
    fn from(value: [T; N]) -> Self {
        Self::Array(Vec::from(value.map(|v| v.into())))
    }
}

impl MAAValue {
    /// Create a new empty object
    pub fn new() -> Self {
        Self::Object(Map::new())
    }

    /// Initialize the value
    ///
    /// If the value is an input value, try to get the value from user input and set it to the value.
    /// If the value is an array or an object, initialize all the values in it recursively.
    pub fn init(&mut self) -> Result<(), TryFromError> {
        match self {
            Self::InputString(v) => *self = Self::String(v.get()?),
            Self::InputBool(v) => *self = Self::Bool(v.get()?),
            Self::InputInt(v) => *self = Self::Int(v.get()?),
            Self::InputFloat(v) => *self = Self::Float(v.get()?),
            Self::Object(map) => {
                for value in map.values_mut() {
                    value.init()?;
                }
            }
            Self::Array(array) => {
                for value in array {
                    value.init()?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Get value of given key
    ///
    /// If the value is an object and the key exists, the value will be returned.
    /// If the key is not exist, `None` will be returned.
    /// Otherwise, the panic will be raised.
    pub fn get(&self, key: &str) -> Option<&Self> {
        if let Self::Object(map) = self {
            map.get(key)
        } else {
            panic!("value is not an object");
        }
    }

    /// Get value of given key or return default value
    ///
    /// Get value of key by calling `get`. If the key is not exist, the default value will be returned.
    /// Otherwise the value will be converted to the type of the default value.
    ///
    /// # Errors
    ///
    /// If the value is not convertible to the type of the default value, the error will be returned.
    pub fn get_or<'a, T>(&'a self, key: &str, default: T) -> Result<T, T::Error>
    where
        T: TryFrom<&'a Self>,
    {
        match self.get(key) {
            Some(value) => value.try_into(),
            None => Ok(default),
        }
    }

    /// Insert a key-value pair into the object
    ///
    /// If the value is an object, the key-value pair will be inserted into the object.
    /// Otherwise, the panic will be raised.
    /// If the key is already exist, the value will be replaced,
    /// otherwise the key-value pair will be inserted.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Self>) {
        if let Self::Object(map) = self {
            map.insert(key.into(), value.into());
        } else {
            panic!("value is not an object");
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

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
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

#[macro_export]
macro_rules! object {
    () => {
        MAAValue::new()
    };
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut value = MAAValue::new();
        $(value.insert($key, $value);)*
        value
    }};
}

impl TryFrom<&MAAValue> for bool {
    type Error = TryFromError;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_bool()
    }
}

impl TryFrom<&MAAValue> for i64 {
    type Error = TryFromError;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_int()
    }
}

impl TryFrom<&MAAValue> for f64 {
    type Error = TryFromError;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_float()
    }
}

impl TryFrom<&MAAValue> for String {
    type Error = TryFromError;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_string()
    }
}

type TryFromResult<T> = Result<T, TryFromError>;

#[derive(Debug)]
pub enum TryFromError {
    TypeMismatch,
    InputError(input::Error),
}

impl From<input::Error> for TryFromError {
    fn from(error: input::Error) -> Self {
        Self::InputError(error)
    }
}

impl Display for TryFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryFromError::TypeMismatch => write!(f, "type mismatch"),
            TryFromError::InputError(error) => write!(f, "{}", error),
        }
    }
}

impl std::error::Error for TryFromError {}

pub type Map<T> = std::collections::BTreeMap<String, T>;

#[cfg(test)]
mod tests {
    use super::*;

    use super::input::Input;

    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, assert_tokens, Token};

        #[test]
        fn input() {
            let value = object!(
                "input_bool" => BoolInput {
                    default: Some(true),
                    description: None,
                },
                "input_int" => Input {
                    default: Some(1),
                    description: None,
                },
                "input_float" => Input {
                    default: Some(1.0),
                    description: None,
                },
                "input_string" => Input {
                    default: Some("string".to_string()),
                    description: None,
                },
                "input_no_default" => Input::<String> {
                    default: None,
                    description: None,
                },
                "select_int" => Select {
                    alternatives: vec![1, 2],
                    description: None,
                },
                "select_float" => Select {
                    alternatives: vec![1.0, 2.0],
                    description: None,
                },
                "select_string" => Select {
                    alternatives: vec!["string1".to_string(), "string2".to_string()],
                    description: None,
                },
            );

            assert_de_tokens(
                &value,
                &[
                    Token::Map { len: Some(8) },
                    Token::Str("input_bool"),
                    Token::Map { len: Some(1) },
                    Token::Str("default"),
                    Token::Bool(true),
                    Token::MapEnd,
                    Token::Str("input_int"),
                    Token::Map { len: Some(1) },
                    Token::Str("default"),
                    Token::I64(1),
                    Token::MapEnd,
                    Token::Str("input_float"),
                    Token::Map { len: Some(1) },
                    Token::Str("default"),
                    Token::F64(1.0),
                    Token::MapEnd,
                    Token::Str("input_string"),
                    Token::Map { len: Some(1) },
                    Token::Str("default"),
                    Token::Str("string"),
                    Token::MapEnd,
                    Token::Str("input_no_default"),
                    Token::Map { len: Some(0) },
                    Token::MapEnd,
                    Token::Str("select_int"),
                    Token::Map { len: Some(1) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(2) },
                    Token::I64(1),
                    Token::I64(2),
                    Token::SeqEnd,
                    Token::MapEnd,
                    Token::Str("select_float"),
                    Token::Map { len: Some(1) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(2) },
                    Token::F64(1.0),
                    Token::F64(2.0),
                    Token::SeqEnd,
                    Token::MapEnd,
                    Token::Str("select_string"),
                    Token::Map { len: Some(1) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(2) },
                    Token::Str("string1"),
                    Token::Str("string2"),
                    Token::SeqEnd,
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            )
        }

        #[test]
        fn value() {
            let value = object!(
                "array" => [1, 2],
                "bool" => true,
                "float" => 1.0,
                "int" => 1,
                "null" => MAAValue::Null,
                "object" => [("key", "value")],
                "string" => "string",
            );

            assert_tokens(
                &value,
                &[
                    Token::Map { len: Some(7) },
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
                    Token::Str("null"),
                    Token::Unit,
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
            let value = MAAValue::Array(vec![
                MAAValue::Bool(true),
                MAAValue::Int(1_i64),
                MAAValue::Float(1.0),
                MAAValue::String("string".to_string()),
                MAAValue::from([1, 2]),
                MAAValue::from([("key", "value")]),
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
            let boolean = MAAValue::Bool(true);
            assert_tokens(&boolean, &[Token::Bool(true)]);
        }

        #[test]
        fn int() {
            let integer = MAAValue::Int(1);
            assert_tokens(&integer, &[Token::I64(1)]);
        }

        #[test]
        fn float() {
            let float = MAAValue::Float(1.0);
            assert_tokens(&float, &[Token::F64(1.0)]);
        }

        #[test]
        fn string() {
            let string = MAAValue::String("string".to_string());
            assert_tokens(&string, &[Token::Str("string")]);
        }

        #[test]
        fn null() {
            let null = MAAValue::Null;
            assert_tokens(&null, &[Token::Unit]);
        }
    }

    #[test]
    fn init() {
        let input_bool = BoolInput {
            default: Some(true),
            description: None,
        };
        let input_int = InputOrSelect::Input(Input {
            default: Some(1),
            description: None,
        });
        let input_float = InputOrSelect::Input(Input {
            default: Some(1.0),
            description: None,
        });
        let input_string = InputOrSelect::Input(Input {
            default: Some("string".to_string()),
            description: None,
        });

        let mut value = object!(
            "null" => MAAValue::Null,
            "bool" => input_bool.clone(),
            "int" => input_int.clone(),
            "float" => input_float.clone(),
            "string" => input_string.clone(),
            "array" => [input_int.clone()],
            "object" => [("int", input_int.clone())],
        );

        assert_eq!(value.get("null").unwrap(), &MAAValue::Null);
        assert_eq!(value.get("bool").unwrap(), &MAAValue::InputBool(input_bool));
        assert_eq!(
            value.get("int").unwrap(),
            &MAAValue::InputInt(input_int.clone())
        );
        assert_eq!(
            value.get("float").unwrap(),
            &MAAValue::InputFloat(input_float)
        );
        assert_eq!(
            value.get("string").unwrap(),
            &MAAValue::InputString(input_string)
        );
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![MAAValue::InputInt(input_int.clone())])
        );
        assert_eq!(
            value.get("object").unwrap(),
            &MAAValue::Object(Map::from([(
                "int".to_string(),
                MAAValue::InputInt(input_int.clone())
            )]))
        );

        value.init().unwrap();

        assert_eq!(value.get("null").unwrap(), &MAAValue::Null);
        assert_eq!(value.get("bool").unwrap(), &MAAValue::Bool(true));
        assert_eq!(value.get("int").unwrap(), &MAAValue::Int(1));
        assert_eq!(value.get("float").unwrap(), &MAAValue::Float(1.0));
        assert_eq!(
            value.get("string").unwrap(),
            &MAAValue::String("string".to_string())
        );
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![MAAValue::Int(1)])
        );
        assert_eq!(
            value.get("object").unwrap(),
            &MAAValue::Object(Map::from([("int".to_string(), MAAValue::Int(1))]))
        );
    }

    #[test]
    fn get() {
        let value = MAAValue::from([("int", 1)]);

        assert_eq!(value.get("int").unwrap(), &MAAValue::Int(1.into()));
        assert!(value.get("float").is_none());

        assert_eq!(value.get_or("int", 2).unwrap(), 1);
        assert_eq!(value.get_or("float", 2.0).unwrap(), 2.0);
    }

    #[test]
    #[should_panic]
    fn get_panic() {
        let value = MAAValue::Null;
        value.get("int");
    }

    #[test]
    fn insert() {
        let mut value = MAAValue::Object(Map::new());
        assert_eq!(value.get_or("int", 2).unwrap(), 2);
        value.insert("int", 1);
        assert_eq!(value.get_or("int", 2).unwrap(), 1);
    }

    #[test]
    #[should_panic]
    fn insert_panic() {
        let mut value = MAAValue::Null;
        value.insert("int", 1);
    }

    #[test]
    fn is_sth() {
        assert!(MAAValue::Null.is_null());
        assert!(MAAValue::from(true).is_bool());
        assert!(MAAValue::from(1).is_int());
        assert!(MAAValue::from(1.0).is_float());
        assert!(MAAValue::from(String::from("string")).is_string());
        assert!(MAAValue::from("string").is_string());
        assert!(MAAValue::from([1, 2]).is_array());
        assert!(MAAValue::from([("key", "value")]).is_object());
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn try_from() {
        // Bool
        let bool_value = MAAValue::from(true);
        assert_eq!(bool::try_from(&bool_value).unwrap(), true);
        assert!(matches!(
            i64::try_from(&bool_value),
            Err(TryFromError::TypeMismatch)
        ));
        let bool_input_value = MAAValue::InputBool(BoolInput {
            default: Some(true),
            description: None,
        });
        assert_eq!(bool::try_from(&bool_input_value).unwrap(), true);
        assert!(matches!(
            i64::try_from(&bool_input_value),
            Err(TryFromError::TypeMismatch)
        ));

        // Int
        let int_value = MAAValue::from(1);
        assert_eq!(i64::try_from(&int_value).unwrap(), 1);
        assert!(matches!(
            f64::try_from(&int_value),
            Err(TryFromError::TypeMismatch)
        ));
        let int_input_value = MAAValue::InputInt(InputOrSelect::Input(Input {
            default: Some(1),
            description: None,
        }));
        assert_eq!(i64::try_from(&int_input_value).unwrap(), 1);
        assert!(matches!(
            f64::try_from(&int_input_value),
            Err(TryFromError::TypeMismatch)
        ));

        // Float
        let float_value = MAAValue::from(1.0);
        assert_eq!(f64::try_from(&float_value).unwrap(), 1.0);
        assert!(matches!(
            String::try_from(&float_value),
            Err(TryFromError::TypeMismatch)
        ));
        let float_input_value = MAAValue::InputFloat(InputOrSelect::Input(Input {
            default: Some(1.0),
            description: None,
        }));
        assert_eq!(f64::try_from(&float_input_value).unwrap(), 1.0);
        assert!(matches!(
            String::try_from(&float_input_value),
            Err(TryFromError::TypeMismatch)
        ));

        // String
        let string_value = MAAValue::from("string");
        assert_eq!(String::try_from(&string_value).unwrap(), "string");
        assert!(matches!(
            bool::try_from(&string_value),
            Err(TryFromError::TypeMismatch)
        ));
        let string_input_value = MAAValue::InputString(InputOrSelect::Input(Input {
            default: Some("string".to_string()),
            description: None,
        }));
        assert_eq!(String::try_from(&string_input_value).unwrap(), "string");
        assert!(matches!(
            bool::try_from(&string_input_value),
            Err(TryFromError::TypeMismatch)
        ));
    }

    #[test]
    fn merge() {
        let value = object!(
            "bool" => true,
            "int" => 1,
            "float" => 1.0,
            "string" => "string",
            "array" => [1, 2],
            "object" => [("key1", "value1"), ("key2", "value2")],
        );

        let value2 = object!(
            "bool" => false,
            "int" => 2,
            "array" => [3, 4],
            "object" => [("key2", "value2_2"), ("key3", "value3")],
        );

        assert_eq!(
            value.merge(&value2),
            object!(
                "bool" => false,
                "int" => 2,
                "float" => 1.0,
                "string" => "string",
                "array" => [3, 4], // array will be replaced instead of merged
                "object" => [("key1", "value1"), ("key2", "value2_2"), ("key3", "value3")],
            ),
        );
    }
}
