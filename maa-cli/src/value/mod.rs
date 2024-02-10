pub mod userinput;

mod primate;
pub use primate::MAAPrimate;

mod input;
pub use input::MAAInput;

use std::io;

use serde::{Deserialize, Serialize};

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum MAAValue {
    /// An array of values
    Array(Vec<MAAValue>),
    /// A value that should be queried from user input
    Input(MAAInput),
    /// A optional value only if all the dependencies are satisfied. Must in an object.
    ///
    /// If keys in dependencies are not exist or the values are not equal to the expected values,
    /// the value will be dropped during initialization.
    OptionalInput {
        /// A map of dependencies
        ///
        /// Keys are the keys of the dependencies in the sam object and values are the expected
        deps: Map<MAAPrimate>,
        /// Input value query from user when all the dependencies are satisfied
        #[serde(flatten)]
        input: MAAInput,
    },
    /// Object is a map of key-value pair
    Object(Map<MAAValue>),
    /// Primate json types: bool, int, float, string
    Primate(MAAPrimate),
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
    /// If the value is an input value, try to get the value from user input and set it to the value.
    /// If the value is an array or an object, initialize all the values in it recursively.
    ///
    /// For optional input value, initialize it only if all the dependencies are satisfied.
    ///
    /// Note: circular dependencies will be set to missing.
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
                let mut sorted_map = map.iter().collect::<Vec<_>>();
                sorted_map.sort_by(|(k1, v1), (k2, v2)| match (v1, v2) {
                    (OptionalInput { deps: deps1, .. }, OptionalInput { deps: deps2, .. }) => {
                        match (deps1.contains_key(*k2), deps2.contains_key(*k1)) {
                            (false, false) => k1.cmp(k2),
                            (true, false) => std::cmp::Ordering::Greater,
                            (false, true) => std::cmp::Ordering::Less,
                            (true, true) => panic!("circular dependencies"),
                        }
                    }
                    (OptionalInput { .. }, _) => std::cmp::Ordering::Greater,
                    (_, OptionalInput { .. }) => std::cmp::Ordering::Less,
                    _ => k1.cmp(k2),
                });

                // Clone sorted keys to release the borrow of map
                let sorted_keys = sorted_map
                    .iter()
                    .map(|(k, _)| (*k).clone())
                    .collect::<Vec<_>>();

                // Initialize all the values with given order and put them into a new map
                let mut initialized: Map<MAAValue> = Map::new();
                for key in sorted_keys {
                    let value = map.remove(&key).unwrap();
                    if let OptionalInput { deps, input } = value {
                        let mut satisfied = true;
                        // Check if all the dependencies are satisfied
                        for (dep_key, expected) in deps {
                            // If the dependency is not exist or the value is not equal to the expected values
                            // break the loop and mark status as unsatisfied
                            if !initialized.get(&dep_key).is_some_and(|v| v == &expected) {
                                satisfied = false;
                                break;
                            }
                        }
                        // if all the dependencies are satisfied, initialize the value
                        if satisfied {
                            initialized.insert(key, input.into_primate()?.into());
                        }
                    } else {
                        initialized.insert(key, value.init()?);
                    }
                }

                Ok(Object(initialized))
            }
            OptionalInput { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "optional input must be in an object",
            )),
            _ => Ok(self),
        }
    }

    /// Get inner value if the value is an object
    pub fn as_object(&self) -> Option<&Map<MAAValue>> {
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

    /// Get the value if the value is primate
    fn as_primate(&self) -> Option<&MAAPrimate> {
        match self {
            Self::Primate(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.as_primate().and_then(MAAPrimate::as_bool)
    }

    pub fn as_int(&self) -> Option<i32> {
        self.as_primate().and_then(MAAPrimate::as_int)
    }

    pub fn as_float(&self) -> Option<f32> {
        self.as_primate().and_then(MAAPrimate::as_float)
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_primate().and_then(MAAPrimate::as_str)
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

// TODO: shortcur for OptionalInput
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

/// Try to convert the value to given type
///
/// If the value is not convertible to the type, None will be returned.
pub trait TryFromMAAValue<'a>: Sized {
    type Value;

    fn try_from_value(value: &'a MAAValue) -> Option<Self::Value>;
}

impl<'a> TryFromMAAValue<'a> for bool {
    type Value = bool;

    fn try_from_value(value: &MAAValue) -> Option<Self::Value> {
        value.as_bool()
    }
}

impl<'a> TryFromMAAValue<'a> for i32 {
    type Value = Self;

    fn try_from_value(value: &MAAValue) -> Option<Self::Value> {
        value.as_int()
    }
}

impl<'a> TryFromMAAValue<'a> for f32 {
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

pub type Map<T> = std::collections::BTreeMap<String, T>;

#[cfg(test)]
mod tests {
    use super::*;

    use userinput::{BoolInput, Input, SelectD};

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
            "object" => [("key", "value")],
            "string" => "string",
            "input_bool" => BoolInput::new(Some(true), None),
            "input_float" => Input::new(Some(1.0), None),
            "input_int" => Input::new(Some(1), None),
            "input_string" => Input::new(sstr("string"), None),
            "select_int" => SelectD::new([1, 2], Some(2), None, false).unwrap(),
            "select_float" => SelectD::new([1.0, 2.0], Some(2), None, false).unwrap(),
            "select_string" => SelectD::<String>::new(["string1", "string2"], Some(2), None, false).unwrap(),
            "optional" => MAAValue::OptionalInput {
                deps: Map::from([("input_bool".to_string(), true.into())]),
                input: Input::new(Some(1), None).into(),
            },
            "optional_no_satisfied" => MAAValue::OptionalInput {
                deps: Map::from([("input_bool".to_string(), false.into())]),
                input: Input::new(Some(1), None).into(),
            },
        );

        serde_test::assert_de_tokens(
            &obj,
            &[
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
                Token::Str("deps"),
                Token::Map { len: Some(1) },
                Token::Str("input_bool"),
                Token::Bool(true),
                Token::MapEnd,
                Token::Str("default"),
                Token::I32(1),
                Token::MapEnd,
                Token::Str("optional_no_satisfied"),
                Token::Map { len: Some(2) },
                Token::Str("deps"),
                Token::Map { len: Some(1) },
                Token::Str("input_bool"),
                Token::Bool(false),
                Token::MapEnd,
                Token::Str("default"),
                Token::I32(1),
                Token::MapEnd,
                Token::MapEnd,
            ],
        );

        let obj = obj.init().unwrap();

        serde_test::assert_ser_tokens(
            &obj,
            &[
                Token::Map { len: Some(14) },
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
                Token::Str("select_float"),
                Token::F32(2.0),
                Token::Str("select_int"),
                Token::I32(2),
                Token::Str("select_string"),
                Token::Str("string2"),
                Token::Str("string"),
                Token::Str("string"),
                Token::MapEnd,
            ],
        );

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
        use MAAValue::OptionalInput;

        let input = BoolInput::new(Some(true), None);
        let optional = OptionalInput {
            deps: Map::from([("input".to_string(), true.into())]),
            input: input.clone().into(),
        };
        let optional_no_satisfied = OptionalInput {
            deps: Map::from([("input".to_string(), false.into())]),
            input: input.clone().into(),
        };
        let optional_no_exist = OptionalInput {
            deps: Map::from([("no_exist".to_string(), true.into())]),
            input: input.clone().into(),
        };
        let optional_chianed = OptionalInput {
            deps: Map::from([("optional".to_string(), true.into())]),
            input: input.clone().into(),
        };

        let value = object!(
            "input" => input.clone(),
            "array" => [1],
            "primate" => 1,
            "optional" => optional.clone(),
            "optional_no_satisfied" => optional_no_satisfied.clone(),
            "optional_no_exist" => optional_no_exist.clone(),
            "optional_chian" => optional_chianed.clone(),
        );

        assert_eq!(value.get("input").unwrap(), &MAAValue::from(input.clone()));
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![1.into()])
        );
        assert_eq!(value.get("primate").unwrap(), &MAAValue::from(1));
        assert_eq!(value.get("optional").unwrap(), &optional);
        assert_eq!(
            value.get("optional_no_satisfied").unwrap(),
            &optional_no_satisfied
        );
        assert_eq!(value.get("optional_no_exist").unwrap(), &optional_no_exist);
        assert_eq!(value.get("optional_chian").unwrap(), &optional_chianed);

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

        assert_eq!(
            optional.init().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        )
    }

    #[test]
    #[should_panic(expected = "circular dependencies")]
    fn init_circular_dependencies() {
        let input1 = BoolInput::new(Some(true), None);
        let value = object!(
            "optional1" => MAAValue::OptionalInput {
                deps: Map::from([("optional2".to_string(), true.into())]),
                input: input1.clone().into(),
            },
            "optional2" => MAAValue::OptionalInput {
                deps: Map::from([("optional1".to_string(), true.into())]),
                input: input1.clone().into(),
            },
        );

        value.init().unwrap();
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
    fn try_from() {
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
