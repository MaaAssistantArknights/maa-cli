use crate::input::{BoolInput, Input, SelectD, UserInput};

use std::io;

use serde::{Deserialize, Serialize};

type Result<T, E = io::Error> = std::result::Result<T, E>;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum MAAValue {
    Array(Vec<MAAValue>),
    InputBool(BoolInput),
    InputInt(Input<i64>),
    InputFloat(Input<f64>),
    InputString(Input<String>),
    SelectInt(SelectD<i64>),
    SelectFloat(SelectD<f64>),
    SelectString(SelectD<String>),
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

impl From<Input<i64>> for MAAValue {
    fn from(value: Input<i64>) -> Self {
        Self::InputInt(value)
    }
}

impl From<SelectD<i64>> for MAAValue {
    fn from(value: SelectD<i64>) -> Self {
        Self::SelectInt(value)
    }
}

impl From<f64> for MAAValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<Input<f64>> for MAAValue {
    fn from(value: Input<f64>) -> Self {
        Self::InputFloat(value)
    }
}

impl From<SelectD<f64>> for MAAValue {
    fn from(value: SelectD<f64>) -> Self {
        Self::SelectFloat(value)
    }
}

impl From<String> for MAAValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for MAAValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<Input<String>> for MAAValue {
    fn from(value: Input<String>) -> Self {
        Self::InputString(value)
    }
}

impl From<SelectD<String>> for MAAValue {
    fn from(value: SelectD<String>) -> Self {
        Self::SelectString(value)
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
    pub fn init(&mut self) -> io::Result<()> {
        use MAAValue::*;
        match self {
            InputBool(v) => *self = Self::Bool(v.value()?),
            InputString(v) => *self = Self::String(v.value()?),
            InputInt(v) => *self = Self::Int(v.value()?),
            InputFloat(v) => *self = Self::Float(v.value()?),
            SelectInt(v) => *self = Self::Int(v.value()?),
            SelectFloat(v) => *self = Self::Float(v.value()?),
            SelectString(v) => *self = Self::String(v.value()?),
            Object(map) => {
                for value in map.values_mut() {
                    value.init()?;
                }
            }
            Array(array) => {
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

    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Self::InputBool(v) => Ok(v.value()?),
            Self::Bool(v) => Ok(*v),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "value is not a bool",
            )),
        }
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(_) | Self::InputInt(_))
    }

    pub fn as_int(&self) -> Result<i64> {
        match self {
            Self::InputInt(v) => Ok(v.value()?),
            Self::SelectInt(v) => Ok(v.value()?),
            Self::Int(v) => Ok(*v),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "value is not an int",
            )),
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_) | Self::InputFloat(_))
    }

    pub fn as_float(&self) -> Result<f64> {
        match self {
            Self::InputFloat(v) => Ok(v.value()?),
            Self::SelectFloat(v) => Ok(v.value()?),
            Self::Float(v) => Ok(*v),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "value is not a float",
            )),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_) | Self::InputString(_))
    }

    pub fn as_string(&self) -> io::Result<String> {
        match self {
            Self::InputString(v) => Ok(v.value()?),
            Self::SelectString(v) => Ok(v.value()?),
            Self::String(v) => Ok(v.clone()),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "value is not a string",
            )),
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
    type Error = io::Error;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_bool()
    }
}

impl TryFrom<&MAAValue> for i64 {
    type Error = io::Error;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_int()
    }
}

impl TryFrom<&MAAValue> for f64 {
    type Error = io::Error;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_float()
    }
}

impl TryFrom<&MAAValue> for String {
    type Error = io::Error;

    fn try_from(value: &MAAValue) -> Result<Self, Self::Error> {
        value.as_string()
    }
}

pub type Map<T> = std::collections::BTreeMap<String, T>;

#[cfg(test)]
mod tests {
    use super::*;

    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, assert_ser_tokens, assert_tokens, Token};

        #[test]
        fn input() {
            let value = object!(
                "input_bool" => BoolInput::new(Some(true), None::<&str>),
                "input_int" => Input::<i64>::new(Some(1), None::<&str>),
                "input_float" => Input::<f64>::new(Some(1.0), None::<&str>),
                "input_string" => Input::<String>::new(Some("string".to_string()), None::<&str>),
                "select_int" => SelectD::<i64>::new([1, 2], Some(2), None::<&str>, false),
                "select_float" => SelectD::<f64>::new([1.0, 2.0], Some(2), None::<&str>, false),
                "select_string" => SelectD::<String>::new(
                    ["string1", "string2"],
                    Some(2),
                    None::<&str>,
                    false
                ),
            );

            assert_de_tokens(
                &value,
                &[
                    Token::Map { len: Some(7) },
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
                    Token::Str("select_int"),
                    Token::Map { len: Some(2) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(2) },
                    Token::I64(1),
                    Token::I64(2),
                    Token::SeqEnd,
                    Token::Str("default_index"),
                    Token::I64(2),
                    Token::MapEnd,
                    Token::Str("select_float"),
                    Token::Map { len: Some(2) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(2) },
                    Token::F64(1.0),
                    Token::F64(2.0),
                    Token::SeqEnd,
                    Token::Str("default_index"),
                    Token::I64(2),
                    Token::MapEnd,
                    Token::Str("select_string"),
                    Token::Map { len: Some(2) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(2) },
                    Token::Str("string1"),
                    Token::Str("string2"),
                    Token::SeqEnd,
                    Token::Str("default_index"),
                    Token::I64(2),
                    Token::MapEnd,
                    Token::MapEnd,
                ],
            );

            assert_ser_tokens(
                &value,
                &[
                    Token::Map { len: Some(7) },
                    Token::Str("input_bool"),
                    Token::Bool(true),
                    Token::Str("input_int"),
                    Token::I64(1),
                    Token::Str("input_float"),
                    Token::F64(1.0),
                    Token::Str("input_string"),
                    Token::Str("string"),
                    Token::Str("select_int"),
                    Token::I64(2),
                    Token::Str("select_float"),
                    Token::F64(2.0),
                    Token::Str("select_string"),
                    Token::Str("string2"),
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
        let input_bool = BoolInput::new(Some(true), None::<&str>);
        let input_int = Input::<i64>::new(Some(1), None::<&str>);
        let input_float = Input::<f64>::new(Some(1.0), None::<&str>);
        let input_string = Input::<String>::new(Some("string".to_string()), None::<&str>);
        let select_int = SelectD::<i64>::new([1, 2], Some(2), None::<&str>, false);
        let select_float = SelectD::<f64>::new([1.0, 2.0], Some(2), None::<&str>, false);
        let select_string =
            SelectD::<String>::new(["string1", "string2"], Some(2), None::<&str>, false);

        let mut value = object!(
            "null" => MAAValue::Null,
            "input_bool" => input_bool.clone(),
            "input_int" => input_int.clone(),
            "input_float" => input_float.clone(),
            "input_string" => input_string.clone(),
            "select_int" => select_int.clone(),
            "select_float" => select_float.clone(),
            "select_string" => select_string.clone(),
            "array" => [input_int.clone()],
            "object" => [("int", input_int.clone())],
        );

        assert_eq!(value.get("null").unwrap(), &MAAValue::Null);
        assert_eq!(
            value.get("input_bool").unwrap(),
            &MAAValue::InputBool(input_bool.clone())
        );
        assert_eq!(
            value.get("input_int").unwrap(),
            &MAAValue::InputInt(input_int.clone())
        );
        assert_eq!(
            value.get("input_float").unwrap(),
            &MAAValue::InputFloat(input_float.clone())
        );
        assert_eq!(
            value.get("input_string").unwrap(),
            &MAAValue::InputString(input_string.clone())
        );
        assert_eq!(
            value.get("select_int").unwrap(),
            &MAAValue::SelectInt(select_int.clone())
        );
        assert_eq!(
            value.get("select_float").unwrap(),
            &MAAValue::SelectFloat(select_float.clone())
        );
        assert_eq!(
            value.get("select_string").unwrap(),
            &MAAValue::SelectString(select_string.clone())
        );
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![MAAValue::InputInt(input_int.clone())])
        );
        assert_eq!(
            value.get("object").unwrap(),
            &MAAValue::Object(Map::from([(
                "int".to_string(),
                MAAValue::InputInt(input_int)
            )]))
        );

        value.init().unwrap();

        assert_eq!(value.get("null").unwrap(), &MAAValue::Null);
        assert_eq!(value.get("input_bool").unwrap(), &MAAValue::Bool(true));
        assert_eq!(value.get("input_int").unwrap(), &MAAValue::Int(1));
        assert_eq!(value.get("input_float").unwrap(), &MAAValue::Float(1.0));
        assert_eq!(
            value.get("input_string").unwrap(),
            &MAAValue::String("string".to_string())
        );
        assert_eq!(value.get("select_int").unwrap(), &MAAValue::Int(2));
        assert_eq!(value.get("select_float").unwrap(), &MAAValue::Float(2.0));
        assert_eq!(
            value.get("select_string").unwrap(),
            &MAAValue::String("string2".to_string())
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
    fn try_from() {
        // Bool
        let bool_value = MAAValue::from(true);
        assert!(bool::try_from(&bool_value).unwrap());
        assert_eq!(
            i64::try_from(&bool_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let bool_input_value =
            MAAValue::from(BoolInput::new(Some(true), None::<&str>).value().unwrap());
        assert!(bool::try_from(&bool_input_value).unwrap());
        assert_eq!(
            i64::try_from(&bool_input_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        // Int
        let int_value = MAAValue::from(1);
        assert_eq!(i64::try_from(&int_value).unwrap(), 1);
        assert_eq!(
            f64::try_from(&int_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let int_input_value =
            MAAValue::from(Input::<i64>::new(Some(1), None::<&str>).value().unwrap());
        assert_eq!(i64::try_from(&int_input_value).unwrap(), 1);
        assert_eq!(
            f64::try_from(&int_input_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let int_select_value = MAAValue::from(
            SelectD::new([1, 2], Some(2), None::<&str>, false)
                .value()
                .unwrap(),
        );
        assert_eq!(i64::try_from(&int_select_value).unwrap(), 2);
        assert_eq!(
            f64::try_from(&int_select_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        // Float
        let float_value = MAAValue::from(1.0);
        assert_eq!(f64::try_from(&float_value).unwrap(), 1.0);
        assert!(matches!(
            String::try_from(&float_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        ));
        let float_input_value =
            MAAValue::from(Input::<f64>::new(Some(1.0), None::<&str>).value().unwrap());
        assert_eq!(f64::try_from(&float_input_value).unwrap(), 1.0);
        assert_eq!(
            String::try_from(&float_input_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let flaot_select_value = MAAValue::from(
            SelectD::new([1.0, 2.0], Some(2), None::<&str>, false)
                .value()
                .unwrap(),
        );
        assert_eq!(f64::try_from(&flaot_select_value).unwrap(), 2.0);
        assert_eq!(
            String::try_from(&flaot_select_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        // String
        let string_value = MAAValue::from("string");
        assert_eq!(String::try_from(&string_value).unwrap(), "string");
        assert_eq!(
            bool::try_from(&string_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let string_input_value = MAAValue::from(Input::<String>::new(Some("string"), None::<&str>));

        assert_eq!(String::try_from(&string_input_value).unwrap(), "string");
        assert_eq!(
            bool::try_from(&string_input_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let string_select_value = MAAValue::from(
            SelectD::<String>::new(["string1", "string2"], Some(2), None::<&str>, false)
                .value()
                .unwrap(),
        );
        assert_eq!(String::try_from(&string_select_value).unwrap(), "string2");
        assert_eq!(
            bool::try_from(&string_select_value).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
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
