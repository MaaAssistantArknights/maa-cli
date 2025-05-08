pub mod userinput;

mod primate;
pub use primate::MAAPrimate;

mod input;
pub use std::collections::BTreeMap as Map;
use std::io;

pub use input::MAAInput;
use serde::{Deserialize, Serialize};

/// TODO: Zero-copy deserialization and reduce clone in init
#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum MAAValue {
    /// An array of values
    Array(Vec<MAAValue>),
    /// A value that should be queried from user input
    Input(MAAInput),
    /// A optional value
    ///
    /// A optional value will be initialized only if all the dependencies are satisfied.
    /// If one of the dependencies is not exist or the value is not equal to the expected value,
    /// the optional value will be dropped after initialization.
    ///
    /// Note: Circular dependencies will cause panic.
    Optional {
        /// A map of dependencies
        ///
        /// Keys are the keys of the dependencies in the sam object and values are the expected
        #[serde(alias = "deps")]
        conditions: Map<String, MAAPrimate>,
        /// Input value query from user when all the dependencies are satisfied
        #[serde(alias = "input", flatten)]
        value: BoxedMAAValue,
    },
    /// Object is a map of key-value pair
    Object(Map<String, MAAValue>),
    /// Primate json types: bool, int, float, string
    Primate(MAAPrimate),
}

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Clone)]
#[serde(transparent)]
pub struct BoxedMAAValue(Box<MAAValue>);

impl BoxedMAAValue {
    fn init(self) -> io::Result<MAAValue> {
        self.0.init()
    }
}

impl<T> From<T> for BoxedMAAValue
where
    T: Into<MAAValue>,
{
    fn from(value: T) -> Self {
        Self(Box::new(value.into()))
    }
}

impl Serialize for MAAValue {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use MAAValue::*;

        // shortcut for return custom serde serialization error
        macro_rules! serr {
            ($msg:expr) => {
                Err(serde::ser::Error::custom($msg))
            };
        }

        match self {
            // Serialize the value directly
            Primate(v) => v.serialize(serializer),
            // Serialize as a sequence of values and filter out all the missing values
            Array(v) => v.serialize(serializer),
            // Serialize as a map of key-value pairs and filter all the missing values
            Object(v) => v.serialize(serializer),
            // Input value should be initialized before serializing
            _ => serr!("cannot serialize input value, you should initialize it first"),
        }
    }
}

impl MAAValue {
    /// Create a new empty object
    pub fn new() -> Self {
        Self::Object(Map::new())
    }

    /// Initialize the value
    ///
    /// If the value is an primate value, do nothing.
    /// If the value is an input value, try to get the value from user input and set it to the
    /// value. If the value is an array or an object, initialize all the values in it
    /// recursively. If the value is an optional value, initialize it only if all the
    /// dependencies are satisfied.
    ///
    /// # Errors
    ///
    /// ## InvalidData
    ///
    /// 1. If an optional value is not in an object, the error will be returned.
    /// 2. If a circular dependencies are found, the error will be returned.
    ///
    /// ## Other
    ///
    /// Otherwise, if some value failed to initialize, forward the error.
    pub fn init(self) -> io::Result<Self> {
        use MAAValue::*;
        match self {
            Input(v) => Ok(v.into_primate()?.into()),
            Array(array) => {
                let mut ret = Vec::with_capacity(array.len());
                for value in array {
                    ret.push(value.init()?);
                }
                Ok(Array(ret))
            }
            Object(mut map) => {
                enum Mark {
                    Visiting,
                    Visited,
                }

                // Depth-first search to sort the keys
                fn visit<'key>(
                    sorted_keys: &mut Vec<String>,
                    key: &'key str,
                    map: &'key Map<String, MAAValue>,
                    marks: &mut Map<&'key str, Mark>,
                ) -> io::Result<()> {
                    match marks.get(key) {
                        Some(Mark::Visited) => return Ok(()),
                        Some(Mark::Visiting) => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "circular dependencies",
                            ));
                        }
                        _ => {}
                    }

                    match map.get(key) {
                        // If the key is an optional value, visit all the dependencies first
                        Some(Optional { conditions, .. }) => {
                            marks.insert(key, Mark::Visiting);
                            for cond_key in conditions.keys() {
                                visit(sorted_keys, cond_key, map, marks)?;
                            }
                        }
                        // if the key is not exist, return directly
                        None => return Ok(()),
                        _ => {}
                    }

                    marks.insert(key, Mark::Visited);
                    sorted_keys.push(key.to_string());

                    Ok(())
                }

                let mut sorted_keys: Vec<String> = Vec::with_capacity(map.len());
                let mut marks = std::collections::BTreeMap::<&str, Mark>::new();

                for key in map.keys() {
                    visit(&mut sorted_keys, key, &map, &mut marks)?;
                }

                // Initialize all the values with given order and put them into a new map
                let mut initialized: Map<String, MAAValue> = Map::new();
                for key in sorted_keys {
                    let value = map.remove(&key).unwrap();
                    if let Optional { conditions, value } = value {
                        let mut satisfied = true;
                        // Check if all the dependencies are satisfied
                        for (cond_key, expected) in conditions {
                            // If the dependency is not exist or the value is not equal to the
                            // expected values break the loop and mark
                            // status as unsatisfied
                            if !initialized.get(&cond_key).is_some_and(|v| v == &expected) {
                                satisfied = false;
                                break;
                            }
                        }
                        // if all the dependencies are satisfied, initialize the value
                        if satisfied {
                            initialized.insert(key, value.init()?);
                        }
                    } else {
                        initialized.insert(key, value.init()?);
                    }
                }

                Ok(Object(initialized))
            }
            Optional { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "optional input must be in an object",
            )),
            _ => Ok(self),
        }
    }

    /// Get inner value if the value is an object
    pub fn as_object(&self) -> Option<&Map<String, MAAValue>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable inner value if the value is an object
    pub fn as_object_mut(&mut self) -> Option<&mut Map<String, MAAValue>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Get value of given key
    ///
    /// If the value is an object and the key exists, the value will be returned.
    /// Otherwise, return `None`.
    pub fn get(&self, key: &str) -> Option<&Self> {
        self.as_object().and_then(|map| map.get(key))
    }

    /// Get mutable value of given key
    ///
    /// Same as `get`, but return mutable reference.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        self.as_object_mut().and_then(|map| map.get_mut(key))
    }

    /// Get value of given key or return default value
    ///
    /// If the value is an object and the key exists, get the value and try to convert it to type of
    /// default value. Otherwise, the default value will be returned.
    pub fn get_or<'a, T>(&'a self, key: &str, default: T) -> T
    where
        T: TryFromMAAValue<'a, Value = T>,
    {
        self.get(key).and_then(T::try_from_value).unwrap_or(default)
    }

    /// Insert a key-value pair into the object
    ///
    /// If the value is an object, the key-value pair will be inserted into the object.
    /// If the key is already exist, the value will be replaced,
    /// otherwise the key-value pair will be inserted.
    ///
    /// # Panics
    ///
    /// If the value is not an object, the panic will be raised.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Self>) {
        if let Self::Object(map) = self {
            map.insert(key.into(), value.into());
        } else {
            panic!("value is not an object");
        }
    }

    pub fn maybe_insert(&mut self, key: impl Into<String>, value: Option<impl Into<Self>>) {
        if let Some(value) = value {
            self.insert(key, value);
        }
    }

    /// Get the value if the value is primate
    ///
    /// A primate value can be a bool, int, float or string.
    /// It can not be an array, object or input value.
    fn as_primate(&self) -> Option<&MAAPrimate> {
        match self {
            Self::Primate(v) => Some(v),
            _ => None,
        }
    }

    /// Convert the value to bool if the value is primate bool
    pub fn as_bool(&self) -> Option<bool> {
        self.as_primate().and_then(MAAPrimate::as_bool)
    }

    /// Convert the value to int if the value is primate int
    pub fn as_int(&self) -> Option<i32> {
        self.as_primate().and_then(MAAPrimate::as_int)
    }

    /// Convert the value to float if the value is primate float
    pub fn as_float(&self) -> Option<f32> {
        self.as_primate().and_then(MAAPrimate::as_float)
    }

    /// Convert the value to string if the value is primate string
    pub fn as_str(&self) -> Option<&str> {
        self.as_primate().and_then(MAAPrimate::as_str)
    }

    /// Merge other value into self
    ///
    /// Both self and other should be an object.
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
/// A convenient macro to create a MAAValue::Object
///
/// # Examples
/// ```
/// use maa_cli::value::MAAValue;
///
/// let object = object!(
///     "bool" => true,
///     "int" => 1,
///     "float" => 1.0,
///     "string" => "string",
///     "array" => [1, 2],
///     "object" => object!(
///         "key1" => "value1",
///         "key2" => "value2",
///     )
///     "optional" if "bool" == true => 1,
///     "optional_no_satisfied" if "bool" == false => 1,
///     "optional_no_exist" if "no_exist" == true => 1,
///     "optional_chian" if "optional" == true => 1,
/// );
/// ```
macro_rules! object {
    () => {
        $crate::value::MAAValue::new()
    };
    ($($key:literal $(if $($cond_key:literal == $expected:expr),*)? => $value:expr),* $(,)?) => {{
        let mut object = $crate::value::MAAValue::new();
        $(
            let value = $value;
            $(
                let mut conditions = $crate::value::Map::new();
                $(
                    conditions.insert($cond_key.into(), $expected.into());
                )*
                let value = $crate::value::MAAValue::Optional { conditions, value: value.into() };
            )?
            object.insert($key, value);
        )*
        object
    }};
}

impl Default for MAAValue {
    fn default() -> Self {
        Self::new()
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

impl<T: Into<MAAValue>> From<Vec<T>> for MAAValue {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

/// Try to convert the value to given type
///
/// If the value is not convertible to the type, None will be returned.
pub trait TryFromMAAValue<'a>: Sized {
    type Value;

    fn try_from_value(value: &'a MAAValue) -> Option<Self::Value>;
}

impl TryFromMAAValue<'_> for bool {
    type Value = bool;

    fn try_from_value(value: &MAAValue) -> Option<Self::Value> {
        value.as_bool()
    }
}

impl TryFromMAAValue<'_> for i32 {
    type Value = Self;

    fn try_from_value(value: &MAAValue) -> Option<Self::Value> {
        value.as_int()
    }
}

impl TryFromMAAValue<'_> for f32 {
    type Value = Self;

    fn try_from_value(value: &MAAValue) -> Option<Self::Value> {
        value.as_float()
    }
}

impl<'a> TryFromMAAValue<'a> for &str {
    type Value = &'a str;

    fn try_from_value(value: &'a MAAValue) -> Option<Self::Value> {
        value.as_str()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use userinput::{BoolInput, Input, SelectD};

    use super::*;
    use crate::assert_matches;

    impl MAAValue {
        pub fn merge(&self, other: &Self) -> Self {
            let mut ret = self.clone();
            ret.merge_mut(other);
            ret
        }
    }

    fn sstr(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    #[test]
    fn serde() {
        use serde_test::Token;

        let obj = object!(
            "array" => [1, 2],
            "bool" => true,
            "float" => 1.0,
            "int" => 1,
            "object" => object!("key" => "value"),
            "string" => "string",
            "input_bool" => BoolInput::new(Some(true), None),
            "input_float" => Input::new(Some(1.0), None),
            "input_int" => Input::new(Some(1), None),
            "input_string" => Input::new(sstr("string"), None),
            "select_int" => SelectD::new([1, 2], Some(2), None, false).unwrap(),
            "select_float" => SelectD::new([1.0, 2.0], Some(2), None, false).unwrap(),
            "select_string" => SelectD::<String>::new(["string1", "string2"], Some(2), None, false).unwrap(),
            "optional" if "input_bool" == true => Input::new(Some(1), None),
            "optional_no_satisfied" if "input_bool" == false => Input::new(Some(1), None),
            "optional_object" if "input_bool" == true =>
                object!("key1" => "value1", "key2" => "value2"),
        );

        serde_test::assert_de_tokens(&obj, &[
            Token::Map { len: Some(16) },
            Token::Str("array"),
            Token::Seq { len: Some(2) },
            Token::I32(1),
            Token::I32(2),
            Token::SeqEnd,
            Token::Str("bool"),
            Token::Bool(true),
            Token::Str("float"),
            Token::F32(1.0),
            Token::Str("int"),
            Token::I32(1),
            Token::Str("object"),
            Token::Map { len: Some(1) },
            Token::Str("key"),
            Token::Str("value"),
            Token::MapEnd,
            Token::Str("string"),
            Token::Str("string"),
            Token::Str("input_bool"),
            Token::Map { len: Some(1) },
            Token::Str("default"),
            Token::Bool(true),
            Token::MapEnd,
            Token::Str("input_int"),
            Token::Map { len: Some(1) },
            Token::Str("default"),
            Token::I32(1),
            Token::MapEnd,
            Token::Str("input_float"),
            Token::Map { len: Some(1) },
            Token::Str("default"),
            Token::F32(1.0),
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
            Token::I32(1),
            Token::I32(2),
            Token::SeqEnd,
            Token::Str("default_index"),
            Token::U64(2),
            Token::MapEnd,
            Token::Str("select_float"),
            Token::Map { len: Some(2) },
            Token::Str("alternatives"),
            Token::Seq { len: Some(2) },
            Token::F32(1.0),
            Token::F32(2.0),
            Token::SeqEnd,
            Token::Str("default_index"),
            Token::U64(2),
            Token::MapEnd,
            Token::Str("select_string"),
            Token::Map { len: Some(2) },
            Token::Str("alternatives"),
            Token::Seq { len: Some(2) },
            Token::Str("string1"),
            Token::Str("string2"),
            Token::SeqEnd,
            Token::Str("default_index"),
            Token::U64(2),
            Token::MapEnd,
            Token::Str("optional"),
            Token::Map { len: Some(2) },
            Token::Str("conditions"),
            Token::Map { len: Some(1) },
            Token::Str("input_bool"),
            Token::Bool(true),
            Token::MapEnd,
            Token::Str("default"),
            Token::I32(1),
            Token::MapEnd,
            Token::Str("optional_no_satisfied"),
            Token::Map { len: Some(2) },
            Token::Str("conditions"),
            Token::Map { len: Some(1) },
            Token::Str("input_bool"),
            Token::Bool(false),
            Token::MapEnd,
            Token::Str("default"),
            Token::I32(1),
            Token::MapEnd,
            Token::Str("optional_object"),
            Token::Map { len: Some(3) },
            Token::Str("conditions"),
            Token::Map { len: Some(1) },
            Token::Str("input_bool"),
            Token::Bool(true),
            Token::MapEnd,
            Token::Str("key1"),
            Token::Str("value1"),
            Token::Str("key2"),
            Token::Str("value2"),
            Token::MapEnd,
            Token::MapEnd,
        ]);

        let obj = obj.init().unwrap();

        serde_test::assert_ser_tokens(&obj, &[
            Token::Map { len: Some(15) },
            Token::Str("array"),
            Token::Seq { len: Some(2) },
            Token::I32(1),
            Token::I32(2),
            Token::SeqEnd,
            Token::Str("bool"),
            Token::Bool(true),
            Token::Str("float"),
            Token::F32(1.0),
            Token::Str("input_bool"),
            Token::Bool(true),
            Token::Str("input_float"),
            Token::F32(1.0),
            Token::Str("input_int"),
            Token::I32(1),
            Token::Str("input_string"),
            Token::Str("string"),
            Token::Str("int"),
            Token::I32(1),
            Token::Str("object"),
            Token::Map { len: Some(1) },
            Token::Str("key"),
            Token::Str("value"),
            Token::MapEnd,
            Token::Str("optional"),
            Token::I32(1),
            Token::Str("optional_object"),
            Token::Map { len: Some(2) },
            Token::Str("key1"),
            Token::Str("value1"),
            Token::Str("key2"),
            Token::Str("value2"),
            Token::MapEnd,
            Token::Str("select_float"),
            Token::F32(2.0),
            Token::Str("select_int"),
            Token::I32(2),
            Token::Str("select_string"),
            Token::Str("string2"),
            Token::Str("string"),
            Token::Str("string"),
            Token::MapEnd,
        ]);

        serde_test::assert_ser_tokens_error(
            &object!(
                "input_bool" => BoolInput::new(None, None),
            ),
            &[Token::Map { len: Some(1) }, Token::Str("input_bool")],
            "cannot serialize input value, you should initialize it first",
        );
    }

    #[test]
    fn init() {
        let input = BoolInput::new(Some(true), None);

        let value = object!(
            "input" => input.clone(),
            "array" => [1],
            "primate" => 1,
            "optional" if "input" == true => input.clone(),
            "optional_no_satisfied" if "input" == false => input.clone(),
            "optional_no_exist" if "no_exist" == true => input.clone(),
            "optional_chian" if "optional" == true => input.clone(),
            "optional_nested" if "optional" == true => object!(
                "nested" if "optional" == true => input.clone(),
            ),
        );

        let optional = value.get("optional").unwrap().clone();

        assert_eq!(value.get("input").unwrap(), &MAAValue::from(input.clone()));
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![1.into()])
        );
        assert_eq!(value.get("primate").unwrap(), &MAAValue::from(1));
        assert_matches!(value.get("optional").unwrap(), MAAValue::Optional { .. });
        assert_matches!(
            value.get("optional_no_satisfied").unwrap(),
            MAAValue::Optional { .. }
        );
        assert_matches!(
            value.get("optional_no_exist").unwrap(),
            MAAValue::Optional { .. }
        );
        assert_matches!(
            value.get("optional_chian").unwrap(),
            MAAValue::Optional { .. }
        );
        assert_matches!(
            value.get("optional_nested").unwrap(),
            MAAValue::Optional { .. }
        );

        let value = value.init().unwrap();

        assert_eq!(value.get("input").unwrap(), &MAAValue::from(true));
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![1.into()])
        );
        assert_eq!(value.get("primate").unwrap(), &MAAValue::from(1));
        assert_eq!(value.get("optional").unwrap(), &MAAValue::from(true));
        assert_eq!(value.get("optional_no_satisfied"), None);
        assert_eq!(value.get("optional_no_exist"), None);
        assert_eq!(value.get("optional_chian").unwrap(), &MAAValue::from(true));
        assert_eq!(value.get("optional_nested").unwrap(), &object!());

        assert_eq!(
            optional.init().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let value = object!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional1" == true => input.clone(),
        );
        assert_eq!(value.init().unwrap_err().kind(), io::ErrorKind::InvalidData);

        let value = object!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional3" == true => input.clone(),
            "optional3" if "optional1" == true => input.clone(),
        );
        assert_eq!(value.init().unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn get() {
        let value = MAAValue::from([("int", 1)]);

        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);
        assert_eq!(value.get("float"), None);
        assert_eq!(MAAValue::from(1).get("int"), None);

        assert_eq!(value.get_or("int", 2), 1);
        assert_eq!(value.get_or("int", 2.0), 2.0);
        assert_eq!(value.get_or("float", 2.0), 2.0);

        let mut value = object!("int" => 1);

        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);
        *value.get_mut("int").unwrap() = 2.into();
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 2);
        assert_eq!(value.get_mut("float"), None);
        assert_eq!(MAAValue::from(1).get_mut("int"), None);
    }

    #[test]
    fn insert() {
        let mut value = MAAValue::new();
        assert_eq!(value.get("int"), None);
        value.insert("int", 1);
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);
    }

    #[test]
    #[should_panic(expected = "value is not an object")]
    fn insert_panics() {
        let mut value = MAAValue::from(1);
        value.insert("int", 1);
    }

    #[test]
    fn maybe_insert() {
        let mut value = MAAValue::new();
        assert_eq!(value.get("int"), None);
        value.maybe_insert("int", Some(1));
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);
        value.maybe_insert("float", None::<f32>);
        assert_eq!(value.get("float"), None);
    }

    #[test]
    fn value_from_others() {
        // Array
        assert_eq!(
            MAAValue::from([1, 2]),
            MAAValue::Array(vec![1.into(), 2.into()])
        );
        assert_eq!(
            MAAValue::from(vec![1, 2]),
            MAAValue::Array(vec![1.into(), 2.into()])
        );
    }

    #[test]
    fn try_from_value() {
        // Bool
        assert_eq!(bool::try_from_value(&true.into()), Some(true));
        assert_eq!(i32::try_from_value(&true.into()), None);
        assert_eq!(
            bool::try_from_value(&BoolInput::new(Some(true), None).into()),
            None
        );

        // Int
        assert_eq!(i32::try_from_value(&1.into()), Some(1));
        assert_eq!(f32::try_from_value(&1.into()), None);
        assert_eq!(i32::try_from_value(&Input::new(Some(1), None).into()), None);

        // Float
        assert_eq!(f32::try_from_value(&1.0.into()), Some(1.0));
        assert_eq!(i32::try_from_value(&1.0.into()), None);
        assert_eq!(
            f32::try_from_value(&Input::new(Some(1.0), None).into()),
            None
        );

        // String
        assert_eq!(<&str>::try_from_value(&"string".into()), Some("string"));
        assert_eq!(bool::try_from_value(&"string".into()), None);
    }

    #[test]
    fn merge() {
        let value = object!(
            "bool" => true,
            "int" => 1,
            "float" => 1.0,
            "string" => "string",
            "array" => [1, 2],
            "object" => object!(
                "key1" => "value1",
                "key2" => "value2",
            ),
        );

        let value2 = object!(
            "bool" => false,
            "int" => 2,
            "array" => [3, 4],
            "object" => object!(
                "key2" => "value2_2",
                "key3" => "value3",
            ),
        );

        assert_eq!(
            value.merge(&value2),
            object!(
                "bool" => false,
                "int" => 2,
                "float" => 1.0,
                "string" => "string",
                "array" => [3, 4], // array will be replaced instead of merged
                "object" => object!(
                    "key1" => "value1",
                    "key2" => "value2_2",
                    "key3" => "value3",
                ),
            ),
        );
    }
}
