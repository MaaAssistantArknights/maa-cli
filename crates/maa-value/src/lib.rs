#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod userinput;

mod primate;
pub use primate::MAAPrimate;

mod input;
use std::borrow::Cow;
pub use std::collections::BTreeMap as Map;

mod error;
pub use error::{Error, Result};
pub use input::MAAInput;
use serde::{Deserialize, Serialize};

// TODO: Zero-copy deserialization and reduce clone in init

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, Debug, PartialEq)]
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

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(transparent)]
pub struct BoxedMAAValue(Box<MAAValue>);

impl BoxedMAAValue {
    fn init(self) -> Result<MAAValue> {
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

impl Default for MAAValue {
    fn default() -> Self {
        Self::Object(Map::default())
    }
}

impl MAAValue {
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
    pub fn init(self) -> Result<Self> {
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
                ) -> Result<()> {
                    match marks.get(key) {
                        Some(Mark::Visited) => return Ok(()),
                        Some(Mark::Visiting) => {
                            return Err(crate::Error::CircularDependency);
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
            Optional { .. } => Err(Error::OptionalNotInObject),
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

    /// Get value of given key with given type
    ///
    /// If the value is an object and the key exists, get the value and try to convert it given
    /// type. Otherwise, return `None`.
    pub fn get_typed<'a, T>(&'a self, key: &str) -> Option<T>
    where
        T: TryFromMAAValue<'a, Value = T>,
    {
        self.get(key).and_then(T::try_from_value)
    }

    /// Get value of given key or return default value
    ///
    /// If the value is an object and the key exists, get the value and try to convert it to type of
    /// default value. Otherwise, the default value will be returned.
    pub fn get_or<'a, T>(&'a self, key: &str, default: T) -> T
    where
        T: TryFromMAAValue<'a, Value = T>,
    {
        self.get_typed(key).unwrap_or(default)
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

    /// Merge another owned value into self, taking ownership of `other`.
    ///
    /// This method consumes `other` and merges it into `self`, modifying `self` in place.
    ///
    /// # Behavior
    ///
    /// - **Objects**: Recursively merges key-value pairs. If a key exists in both objects:
    ///   - If both values are objects, they are recursively merged
    ///   - Otherwise, the value from `other` replaces the value in `self`
    /// - **Non-objects**: The value in `self` is completely replaced by `other`
    ///
    /// # Performance
    ///
    /// This is the most efficient merge variant as it can move values from `other`
    /// instead of cloning them. Use this when you don't need `other` after the merge.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::object;
    ///
    /// let mut base = object!("a" => 1, "b" => 2);
    /// let update = object!("b" => 3, "c" => 4);
    ///
    /// base.merge(update);
    /// assert_eq!(base, object!("a" => 1, "b" => 3, "c" => 4));
    /// ```
    ///
    /// See also: [`merge_from`](Self::merge_from) for borrowing variant, [`join`](Self::join) for
    /// non-mutating variant
    pub fn merge(&mut self, other: Self) {
        match (self, other) {
            (Self::Object(self_map), Self::Object(other_map)) => {
                for (key, value) in other_map {
                    if let Some(self_value) = self_map.get_mut(&key) {
                        self_value.merge(value);
                    } else {
                        self_map.insert(key, value);
                    }
                }
            }
            (s, o) => *s = o,
        }
    }

    /// Merge a borrowed value into self, cloning values from `other` as needed.
    ///
    /// This method borrows `other` and merges it into `self`, modifying `self` in place.
    /// Values from `other` are cloned when inserted into `self`.
    ///
    /// # Behavior
    ///
    /// - **Objects**: Recursively merges key-value pairs. If a key exists in both objects:
    ///   - If both values are objects, they are recursively merged
    ///   - Otherwise, the value from `other` replaces the value in `self`
    /// - **Non-objects**: The value in `self` is completely replaced by a clone of `other`
    ///
    /// # Performance
    ///
    /// This variant clones values from `other`, making it less efficient than
    /// [`merge`](Self::merge). Use this when you need to keep `other` after the merge.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::object;
    ///
    /// let mut base = object!("a" => 1, "b" => 2);
    /// let update = object!("b" => 3, "c" => 4);
    ///
    /// base.merge_from(&update);
    /// assert_eq!(base, object!("a" => 1, "b" => 3, "c" => 4));
    /// // update is still usable
    /// assert_eq!(update, object!("b" => 3, "c" => 4));
    /// ```
    ///
    /// See also: [`merge`](Self::merge) for owned variant, [`join`](Self::join) for non-mutating
    /// variant
    pub fn merge_from(&mut self, other: &Self) {
        match (self, other) {
            (Self::Object(self_map), Self::Object(other_map)) => {
                for (key, value) in other_map {
                    if let Some(self_value) = self_map.get_mut(key) {
                        self_value.merge_from(value);
                    } else {
                        self_map.insert(key.clone(), value.clone());
                    }
                }
            }
            (s, o) => *s = o.clone(),
        }
    }

    /// Create a new value by merging `other` into a clone of `self`.
    ///
    /// This method clones `self` and merges `other` into the clone, returning the result.
    /// Neither `self` nor `other` is modified.
    ///
    /// # Behavior
    ///
    /// - **Objects**: Recursively merges key-value pairs. If a key exists in both objects:
    ///   - If both values are objects, they are recursively merged
    ///   - Otherwise, the value from `other` replaces the value from `self` in the result
    /// - **Non-objects**: Returns a copy/clone of `other`
    ///
    /// # Generic Parameter
    ///
    /// Accepts either `MAAValue` or `&MAAValue` for convenience:
    /// - Passing an owned value uses [`merge`](Self::merge) internally (more efficient)
    /// - Passing a reference uses [`merge_from`](Self::merge_from) internally
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::object;
    ///
    /// let base = object!("a" => 1, "b" => 2);
    /// let update = object!("b" => 3, "c" => 4);
    ///
    /// // Can use owned or borrowed
    /// let result1 = base.join(update.clone());
    /// let result2 = base.join(&update);
    ///
    /// assert_eq!(result1, object!("a" => 1, "b" => 3, "c" => 4));
    /// assert_eq!(result2, result1);
    /// // base and update are unchanged
    /// ```
    ///
    /// See also: [`merge`](Self::merge), [`merge_from`](Self::merge_from) for mutating variants
    pub fn join<'a, O: Into<Cow<'a, Self>>>(&self, other: O) -> Self {
        let mut ret = self.clone();
        let other = other.into();
        match other {
            Cow::Borrowed(other) => ret.merge_from(other),
            Cow::Owned(other) => ret.merge(other),
        }
        ret
    }
}

impl<'a> From<MAAValue> for Cow<'a, MAAValue> {
    fn from(value: MAAValue) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a MAAValue> for Cow<'a, MAAValue> {
    fn from(value: &'a MAAValue) -> Self {
        Cow::Borrowed(value)
    }
}

#[macro_export]
/// A convenient macro to create a MAAValue::Object
///
/// # Examples
/// ```
/// use maa_value::{MAAValue, object};
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
///     ),
///     "optional" if "bool" == true => 1,
///     "optional_no_satisfied" if "bool" == false => 1,
///     "optional_no_exist" if "no_exist" == true => 1,
///     "optional_chian" if "optional" == true => 1,
/// );
/// ```
macro_rules! object {
    () => {
        $crate::MAAValue::default()
    };
    ($($key:literal $(if $($cond_key:literal == $expected:expr),*)? => $value:expr),* $(,)?) => {{
        let mut object = $crate::MAAValue::default();
        $(
            let value = $value;
            $(
                let mut conditions = $crate::Map::new();
                $(
                    conditions.insert($cond_key.into(), $expected.into());
                )*
                let value = $crate::MAAValue::Optional { conditions, value: value.into() };
            )?
            object.insert($key, value);
        )*
        object
    }};
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
    use std::num::NonZero;

    use userinput::{BoolInput, Input, SelectD};

    use super::*;

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
            "input_bool" => BoolInput::new(Some(true)),
            "input_float" => Input::new(Some(1.0)),
            "input_int" => Input::new(Some(1)),
            "input_string" => Input::new(sstr("string")),
            "select_int" => SelectD::from_iter([1, 2], NonZero::new(2)).unwrap(),
            "select_float" => SelectD::from_iter([1.0, 2.0], NonZero::new(2)).unwrap(),
            "select_string" => SelectD::<String>::from_iter(["string1", "string2"], NonZero::new(2)).unwrap(),
            "optional" if "input_bool" == true => Input::new(Some(1)),
            "optional_no_satisfied" if "input_bool" == false => Input::new(Some(1)),
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
                "input_bool" => BoolInput::new(None),
            ),
            &[Token::Map { len: Some(1) }, Token::Str("input_bool")],
            "cannot serialize input value, you should initialize it first",
        );
    }

    #[test]
    fn init() {
        let input = BoolInput::new(Some(true));

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

        let optional_uninitialized = value.get("optional").unwrap().clone();
        assert!(matches!(
            optional_uninitialized.init().unwrap_err(),
            Error::OptionalNotInObject,
        ));

        assert_eq!(value.get("input").unwrap(), &MAAValue::from(input.clone()));
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![1.into()])
        );
        assert_eq!(value.get("primate").unwrap(), &MAAValue::from(1));
        assert!(matches!(
            value.get("optional").unwrap(),
            MAAValue::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_no_satisfied").unwrap(),
            MAAValue::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_no_exist").unwrap(),
            MAAValue::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_chian").unwrap(),
            MAAValue::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_nested").unwrap(),
            MAAValue::Optional { .. }
        ));

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

        let value = object!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional1" == true => input.clone(),
        );
        assert!(matches!(
            value.init().unwrap_err(),
            Error::CircularDependency,
        ));

        let value = object!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional3" == true => input.clone(),
            "optional3" if "optional1" == true => input.clone(),
        );
        assert!(matches!(
            value.init().unwrap_err(),
            Error::CircularDependency,
        ));
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
        let mut value = MAAValue::default();
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
        let mut value = MAAValue::default();
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
            bool::try_from_value(&BoolInput::new(Some(true)).into()),
            None
        );

        // Int
        assert_eq!(i32::try_from_value(&1.into()), Some(1));
        assert_eq!(f32::try_from_value(&1.into()), None);
        assert_eq!(i32::try_from_value(&Input::new(Some(1)).into()), None);

        // Float
        assert_eq!(f32::try_from_value(&1.0.into()), Some(1.0));
        assert_eq!(i32::try_from_value(&1.0.into()), None);
        assert_eq!(f32::try_from_value(&Input::new(Some(1.0)).into()), None);

        // String
        assert_eq!(<&str>::try_from_value(&"string".into()), Some("string"));
        assert_eq!(bool::try_from_value(&"string".into()), None);
    }

    mod merge {
        use super::*;

        #[test]
        fn merge_owned_objects() {
            let mut base = object!(
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

            let update = object!(
                "bool" => false,
                "int" => 2,
                "array" => [3, 4],
                "object" => object!(
                    "key2" => "value2_2",
                    "key3" => "value3",
                ),
            );

            base.merge(update);

            assert_eq!(
                base,
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

        #[test]
        fn merge_owned_primitives() {
            let mut base = MAAValue::from(1);
            base.merge(MAAValue::from(2));
            assert_eq!(base, MAAValue::from(2));

            let mut base = MAAValue::from("hello");
            base.merge(MAAValue::from("world"));
            assert_eq!(base, MAAValue::from("world"));
        }

        #[test]
        fn merge_owned_deep_nesting() {
            let mut base = object!(
                "level1" => object!(
                    "level2" => object!(
                        "key" => "original",
                    ),
                ),
            );

            let update = object!(
                "level1" => object!(
                    "level2" => object!(
                        "key" => "updated",
                        "new_key" => "added",
                    ),
                ),
            );

            base.merge(update);

            assert_eq!(
                base,
                object!(
                    "level1" => object!(
                        "level2" => object!(
                            "key" => "updated",
                            "new_key" => "added",
                        ),
                    ),
                ),
            );
        }

        #[test]
        fn merge_from_objects() {
            let mut base = object!(
                "a" => 1,
                "b" => 2,
            );

            let update = object!(
                "b" => 3,
                "c" => 4,
            );

            base.merge_from(&update);

            assert_eq!(base, object!("a" => 1, "b" => 3, "c" => 4));
            // Ensure update is unchanged
            assert_eq!(update, object!("b" => 3, "c" => 4));
        }

        #[test]
        fn merge_from_primitives() {
            let mut base = MAAValue::from(1);
            let update = MAAValue::from(2);

            base.merge_from(&update);

            assert_eq!(base, MAAValue::from(2));
            assert_eq!(update, MAAValue::from(2)); // update unchanged
        }

        #[test]
        fn merge_from_mixed_types() {
            let mut base = MAAValue::from(1);
            let update = object!("key" => "value");

            base.merge_from(&update);

            // Base should be completely replaced
            assert_eq!(base, object!("key" => "value"));
            assert_eq!(update, object!("key" => "value")); // update unchanged
        }

        #[test]
        fn join_with_owned() {
            let base = object!("a" => 1, "b" => 2);
            let update = object!("b" => 3, "c" => 4);

            let result = base.join(update);

            assert_eq!(result, object!("a" => 1, "b" => 3, "c" => 4));
            // Base should be unchanged
            assert_eq!(base, object!("a" => 1, "b" => 2));
        }

        #[test]
        fn join_with_borrowed() {
            let base = object!("a" => 1, "b" => 2);
            let update = object!("b" => 3, "c" => 4);

            let result = base.join(&update);

            assert_eq!(result, object!("a" => 1, "b" => 3, "c" => 4));
            // Both should be unchanged
            assert_eq!(base, object!("a" => 1, "b" => 2));
            assert_eq!(update, object!("b" => 3, "c" => 4));
        }

        #[test]
        fn join_complex_nested() {
            let base = object!(
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

            let update = object!(
                "bool" => false,
                "int" => 2,
                "array" => [3, 4],
                "object" => object!(
                    "key2" => "value2_2",
                    "key3" => "value3",
                ),
            );

            assert_eq!(
                base.join(&update),
                object!(
                    "bool" => false,
                    "int" => 2,
                    "float" => 1.0,
                    "string" => "string",
                    "array" => [3, 4],
                    "object" => object!(
                        "key1" => "value1",
                        "key2" => "value2_2",
                        "key3" => "value3",
                    ),
                ),
            );
        }

        #[test]
        fn join_empty_objects() {
            let base = object!();
            let update = object!("a" => 1);

            assert_eq!(base.join(&update), object!("a" => 1));

            let base = object!("a" => 1);
            let update = object!();

            assert_eq!(base.join(&update), object!("a" => 1));
        }

        #[test]
        fn merge_overwrites_different_types() {
            let mut base = object!(
                "key" => "string_value",
            );

            let update = object!(
                "key" => 123,
            );

            base.merge_from(&update);

            assert_eq!(base, object!("key" => 123));
        }

        #[test]
        fn merge_preserves_unmentioned_keys() {
            let mut base = object!(
                "keep1" => "value1",
                "keep2" => "value2",
                "override" => "old",
            );

            let update = object!(
                "override" => "new",
                "add" => "added",
            );

            base.merge_from(&update);

            assert_eq!(
                base,
                object!(
                    "keep1" => "value1",
                    "keep2" => "value2",
                    "override" => "new",
                    "add" => "added",
                ),
            );
        }

        #[test]
        fn merge_three_levels_deep() {
            let mut base = object!(
                "a" => object!(
                    "b" => object!(
                        "c" => 1,
                        "d" => 2,
                    ),
                ),
            );

            let update = object!(
                "a" => object!(
                    "b" => object!(
                        "c" => 999,
                        "e" => 3,
                    ),
                ),
            );

            base.merge_from(&update);

            assert_eq!(
                base,
                object!(
                    "a" => object!(
                        "b" => object!(
                            "c" => 999,
                            "d" => 2,
                            "e" => 3,
                        ),
                    ),
                ),
            );
        }

        #[test]
        fn merge_array_replacement() {
            let mut base = object!(
                "arrays" => object!(
                    "arr1" => [1, 2, 3],
                    "arr2" => ["a", "b"],
                ),
            );

            let update = object!(
                "arrays" => object!(
                    "arr1" => [4, 5],
                ),
            );

            base.merge_from(&update);

            // Arrays should be replaced, not merged
            assert_eq!(
                base,
                object!(
                    "arrays" => object!(
                        "arr1" => [4, 5],
                        "arr2" => ["a", "b"],
                    ),
                ),
            );
        }
    }
}
