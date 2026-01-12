#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// Allow the proc-macro to reference types via `maa_value::` path even inside this crate
extern crate self as maa_value;

pub mod userinput;

mod primitive;
pub use primitive::MAAPrimitive;

mod input;
use std::borrow::Cow;
pub use std::collections::BTreeMap as Map;

mod error;
pub use error::{Error, Result};
pub use input::MAAInput;
pub use maa_value_macro::{insert, object};
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
        conditions: Map<String, MAAPrimitive>,
        /// Input value query from user when all the dependencies are satisfied
        #[serde(alias = "input", flatten)]
        value: BoxedMAAValue,
    },
    /// Object is a map of key-value pair
    Object(Map<String, MAAValue>),
    /// Primitive json types: bool, int, float, string
    Primitive(MAAPrimitive),
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
            Primitive(v) => v.serialize(serializer),
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
    /// Initializes the value by resolving all inputs and conditional fields.
    ///
    /// This method recursively processes the value structure and performs the following
    /// operations based on the variant:
    ///
    /// - **Primitive**: Returns the value unchanged.
    /// - **Input**: Resolves the input by querying for user input and converts it to a primitive
    ///   value.
    /// - **Array**: Recursively initializes each element in the array.
    /// - **Object**: Initializes all values in the object, handling optional fields based on their
    ///   conditions. Uses topological sorting (depth-first search) to process fields in dependency
    ///   order, ensuring that conditional dependencies are evaluated before the fields that depend
    ///   on them.
    /// - **Optional**: Must be contained within an object. The optional field is only initialized
    ///   if all its condition dependencies are satisfied (i.e., the required fields exist in the
    ///   object and match their expected values).
    ///
    /// # Errors
    ///
    /// Returns an error in the following cases:
    ///
    /// - [`Error::OptionalNotInObject`]: An `Optional` variant is encountered outside of an object
    ///   context.
    /// - [`Error::CircularDependency`]: Circular dependencies are detected among optional fields in
    ///   an object (e.g., field A depends on B, and B depends on A).
    /// - Other errors: Any errors encountered during initialization of nested values are propagated
    ///   upward.
    pub fn init(self) -> Result<Self> {
        use MAAValue::*;
        match self {
            Input(v) => Ok(v.into_primitive()?.into()),
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

    /// Get inner map if the value is an object.
    ///
    /// Returns a reference to the underlying `Map<String, MAAValue>` if this value is an
    /// `Object` variant. Returns `None` for all other variants.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::{MAAValue, object};
    ///
    /// let obj = object!("key" => "value");
    /// let map = obj.as_map().unwrap();
    /// assert_eq!(map.len(), 1);
    ///
    /// let not_obj = MAAValue::from(42);
    /// assert!(not_obj.as_map().is_none());
    /// ```
    pub fn as_map(&self) -> Option<&Map<String, MAAValue>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable inner map if the value is an object.
    ///
    /// Returns a mutable reference to the underlying `Map<String, MAAValue>` if this value
    /// is an `Object` variant. Returns `None` for all other variants.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::{MAAValue, object};
    ///
    /// let mut obj = object!("key" => "value");
    /// let map = obj.as_mut_map().unwrap();
    /// map.insert("new_key".to_string(), "new_value".into());
    /// assert_eq!(obj.get("new_key").unwrap().as_str(), Some("new_value"));
    ///
    /// let mut not_obj = MAAValue::from(42);
    /// assert!(not_obj.as_mut_map().is_none());
    /// ```
    pub fn as_mut_map(&mut self) -> Option<&mut Map<String, MAAValue>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Get inner slice if the value is an array.
    ///
    /// Returns a reference to the array elements as a slice if this value is an `Array`
    /// variant. Returns `None` for all other variants.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let array = MAAValue::from([1, 2, 3]);
    /// let slice = array.as_slice().unwrap();
    /// assert_eq!(slice.len(), 3);
    /// assert_eq!(slice[0].as_int(), Some(1));
    ///
    /// let not_array = MAAValue::from(42);
    /// assert!(not_array.as_slice().is_none());
    /// ```
    pub fn as_slice(&self) -> Option<&[MAAValue]> {
        match self {
            Self::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable inner vector if the value is an array.
    ///
    /// Returns a mutable reference to the underlying `Vec<MAAValue>` if this value is an
    /// `Array` variant. Returns `None` for all other variants. This allows modifying the
    /// array in place, including pushing, popping, and changing elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let mut array = MAAValue::from([1, 2, 3]);
    /// let vec = array.as_mut_vec().unwrap();
    /// vec.push(4.into());
    /// assert_eq!(array.as_slice().unwrap().len(), 4);
    ///
    /// let mut not_array = MAAValue::from(42);
    /// assert!(not_array.as_mut_vec().is_none());
    /// ```
    pub fn as_mut_vec(&mut self) -> Option<&mut Vec<MAAValue>> {
        match self {
            Self::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Get value of given key
    ///
    /// If the value is an object and the key exists, the value will be returned.
    /// Otherwise, return `None`.
    pub fn get(&self, key: &str) -> Option<&Self> {
        self.as_map().and_then(|map| map.get(key))
    }

    /// Get mutable value of given key
    ///
    /// Same as `get`, but return mutable reference.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        self.as_mut_map().and_then(|map| map.get_mut(key))
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

    /// Inserts a key-value pair into the object.
    ///
    /// If the key already exists, the value will be replaced.
    ///
    /// This method requires the value to implement `Into<MAAValue>`.
    /// For types that only implement `TryInto<MAAValue>` (like `PathBuf` or `OsString`),
    /// use [`Self::try_insert`] instead.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not an object variant.
    ///
    /// # Example
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let mut obj = MAAValue::default();
    /// obj.insert("key", "value".into());
    /// obj.insert("count", 42.into());
    ///
    /// assert_eq!(obj.get("key").unwrap().as_str().unwrap(), "value");
    /// assert_eq!(obj.get("count").unwrap().as_int().unwrap(), 42);
    /// ```
    pub fn insert(&mut self, key: impl Into<String>, value: Self) {
        if let Self::Object(map) = self {
            map.insert(key.into(), value);
        } else {
            panic!("value is not an object");
        }
    }

    /// Inserts a key-value pair into the object if the value is `Some`.
    ///
    /// If `value` is `None`, this method does nothing.
    /// If the key already exists and `value` is `Some`, the value will be replaced.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not an object variant and `value` is `Some`.
    ///
    /// # Example
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let mut obj = MAAValue::default();
    /// obj.maybe_insert("present", Some("value".into()));
    /// obj.maybe_insert("absent", None::<MAAValue>);
    ///
    /// assert_eq!(obj.get("present").unwrap().as_str().unwrap(), "value");
    /// assert!(obj.get("absent").is_none());
    /// ```
    ///
    /// # See also
    ///
    /// - [`Self::insert`] for unconditional insertion.
    pub fn maybe_insert(&mut self, key: impl Into<String>, value: Option<Self>) {
        if let Some(value) = value {
            self.insert(key, value);
        }
    }

    /// Get the value if the value is primative
    ///
    /// A primative value can be a bool, int, float or string.
    /// It can not be an array, object or input value.
    fn as_primitive(&self) -> Option<&MAAPrimitive> {
        match self {
            Self::Primitive(v) => Some(v),
            _ => None,
        }
    }

    /// Extract boolean value if this is a Primitive bool.
    ///
    /// Returns `Some(bool)` if this value is a `Primitive(Bool)` variant.
    /// Returns `None` for all other value types, including input values.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let bool_val = MAAValue::from(true);
    /// assert_eq!(bool_val.as_bool(), Some(true));
    ///
    /// let int_val = MAAValue::from(42);
    /// assert_eq!(int_val.as_bool(), None);
    ///
    /// let string_val = MAAValue::from("true");
    /// assert_eq!(string_val.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        self.as_primitive().and_then(MAAPrimitive::as_bool)
    }

    /// Extract integer value if this is a Primitive int.
    ///
    /// Returns `Some(i32)` if this value is a `Primitive(Int)` variant.
    /// Returns `None` for all other value types, including input values.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let int_val = MAAValue::from(42);
    /// assert_eq!(int_val.as_int(), Some(42));
    ///
    /// let negative_val = MAAValue::from(-10);
    /// assert_eq!(negative_val.as_int(), Some(-10));
    ///
    /// let float_val = MAAValue::from(3.14);
    /// assert_eq!(float_val.as_int(), None);
    ///
    /// let string_val = MAAValue::from("42");
    /// assert_eq!(string_val.as_int(), None);
    /// ```
    pub fn as_int(&self) -> Option<i32> {
        self.as_primitive().and_then(MAAPrimitive::as_int)
    }

    /// Extract float value if this is a Primitive float.
    ///
    /// Returns `Some(f32)` if this value is a `Primitive(Float)` variant.
    /// Returns `None` for all other value types, including input values.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let float_val = MAAValue::from(3.14);
    /// assert_eq!(float_val.as_float(), Some(3.14));
    ///
    /// let negative_val = MAAValue::from(-2.5);
    /// assert_eq!(negative_val.as_float(), Some(-2.5));
    ///
    /// let int_val = MAAValue::from(42);
    /// assert_eq!(int_val.as_float(), None);
    ///
    /// let string_val = MAAValue::from("3.14");
    /// assert_eq!(string_val.as_float(), None);
    /// ```
    pub fn as_float(&self) -> Option<f32> {
        self.as_primitive().and_then(MAAPrimitive::as_float)
    }

    /// Extract string reference if this is a Primitive string.
    ///
    /// Returns `Some(&str)` if this value is a `Primitive(String)` variant.
    /// Returns `None` for all other value types, including input values.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::MAAValue;
    ///
    /// let string_val = MAAValue::from("hello");
    /// assert_eq!(string_val.as_str(), Some("hello"));
    ///
    /// let owned_string = MAAValue::from(String::from("world"));
    /// assert_eq!(owned_string.as_str(), Some("world"));
    ///
    /// let int_val = MAAValue::from(42);
    /// assert_eq!(int_val.as_str(), None);
    ///
    /// let bool_val = MAAValue::from(true);
    /// assert_eq!(bool_val.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        self.as_primitive().and_then(MAAPrimitive::as_str)
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
    /// See also: [`Self::merge_from`] for borrowing variant, [`join`](Self::join) for
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
    /// This variant clones values from `other`, making it less efficient than [`Self::merge`].
    /// Use this when you need to keep `other` after the merge.
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
    /// See also: [`Self::merge`] for owned variant, [`Self::join`] for non-mutating variant.
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
    ///   - If both values are objects, they are recursively merged.
    ///   - Otherwise, the value from `other` replaces the value from `self` in the result.
    /// - **Non-objects**: Returns a copy/clone of `other`.
    ///
    /// # Generic Parameter
    ///
    /// Accepts either `MAAValue` or `&MAAValue` for convenience:
    ///
    /// - Passing an owned value uses [`Self::merge`] internally (more efficient).
    /// - Passing a reference uses [`Self::merge_from`] internally.
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
    /// See also: [`merge`](Self::merge), [`merge_from`](Self::merge_from) for mutating variants.
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

// impl<T: Into<MAAValue>> From<Vec<T>> for MAAValue {
//     fn from(value: Vec<T>) -> Self {
//         Self::Array(value.into_iter().map(Into::into).collect())
//     }
// }

impl<T: TryInto<MAAValue>> TryFrom<Vec<T>> for MAAValue {
    type Error = T::Error;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        Ok(Self::Array(
            value
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        ))
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
            "primitive" => 1,
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
        assert_eq!(value.get("primitive").unwrap(), &MAAValue::from(1));
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
        assert_eq!(value.get("primitive").unwrap(), &MAAValue::from(1));
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
        value.insert("int", 1.into());
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);
    }

    #[test]
    #[should_panic(expected = "value is not an object")]
    fn insert_panics() {
        let mut value = MAAValue::from(1);
        value.insert("int", 1.into());
    }

    #[test]
    fn maybe_insert() {
        let mut value = MAAValue::default();
        assert_eq!(value.get("int"), None);
        value.maybe_insert("int", Some(1.into()));
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);
        value.maybe_insert("float", None::<MAAValue>);
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
            MAAValue::try_from(vec![1, 2]).unwrap(),
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

    mod as_methods {
        use super::*;

        #[test]
        fn as_bool() {
            // Test with bool value
            let true_value = MAAValue::from(true);
            assert_eq!(true_value.as_bool(), Some(true));

            let false_value = MAAValue::from(false);
            assert_eq!(false_value.as_bool(), Some(false));

            // Test with non-bool values (should return None)
            assert_eq!(MAAValue::from(1).as_bool(), None);
            assert_eq!(MAAValue::from(1.0).as_bool(), None);
            assert_eq!(MAAValue::from("string").as_bool(), None);
            assert_eq!(MAAValue::from([1, 2]).as_bool(), None);
            assert_eq!(MAAValue::default().as_bool(), None);

            // Test with input values (should return None)
            assert_eq!(MAAValue::from(BoolInput::new(Some(true))).as_bool(), None);
        }

        #[test]
        fn as_int() {
            // Test with int value
            let int_value = MAAValue::from(42);
            assert_eq!(int_value.as_int(), Some(42));

            let negative_value = MAAValue::from(-10);
            assert_eq!(negative_value.as_int(), Some(-10));

            let zero_value = MAAValue::from(0);
            assert_eq!(zero_value.as_int(), Some(0));

            // Test with non-int values (should return None)
            assert_eq!(MAAValue::from(true).as_int(), None);
            assert_eq!(MAAValue::from(1.0).as_int(), None);
            assert_eq!(MAAValue::from("42").as_int(), None);
            assert_eq!(MAAValue::from([1, 2]).as_int(), None);
            assert_eq!(MAAValue::default().as_int(), None);

            // Test with input values (should return None)
            assert_eq!(MAAValue::from(Input::new(Some(42))).as_int(), None);
        }

        #[test]
        fn as_float() {
            // Test with float value
            let float_value = MAAValue::from(2.14);
            assert_eq!(float_value.as_float(), Some(2.14));

            let negative_value = MAAValue::from(-2.5);
            assert_eq!(negative_value.as_float(), Some(-2.5));

            let zero_value = MAAValue::from(0.0);
            assert_eq!(zero_value.as_float(), Some(0.0));

            // Test with non-float values (should return None)
            assert_eq!(MAAValue::from(true).as_float(), None);
            assert_eq!(MAAValue::from(42).as_float(), None);
            assert_eq!(MAAValue::from("3.14").as_float(), None);
            assert_eq!(MAAValue::from([1.0, 2.0]).as_float(), None);
            assert_eq!(MAAValue::default().as_float(), None);

            // Test with input values (should return None)
            assert_eq!(MAAValue::from(Input::new(Some(2.14))).as_float(), None);
        }

        #[test]
        fn as_str() {
            // Test with string value
            let string_value = MAAValue::from("hello");
            assert_eq!(string_value.as_str(), Some("hello"));

            let empty_string = MAAValue::from("");
            assert_eq!(empty_string.as_str(), Some(""));

            let owned_string = MAAValue::from(String::from("world"));
            assert_eq!(owned_string.as_str(), Some("world"));

            // Test with non-string values (should return None)
            assert_eq!(MAAValue::from(true).as_str(), None);
            assert_eq!(MAAValue::from(42).as_str(), None);
            assert_eq!(MAAValue::from(2.14).as_str(), None);
            assert_eq!(MAAValue::from([1, 2]).as_str(), None);
            assert_eq!(MAAValue::default().as_str(), None);

            // Test with input values (should return None)
            assert_eq!(
                MAAValue::from(Input::new(Some(String::from("hello")))).as_str(),
                None
            );
        }

        #[test]
        fn as_map() {
            // Test with object value
            let obj = MAAValue::from([("key1", "value1"), ("key2", "value2")]);
            let map = obj.as_map().unwrap();
            assert_eq!(map.len(), 2);
            assert!(map.contains_key("key1"));
            assert!(map.contains_key("key2"));

            // Test with empty object
            let empty_obj = MAAValue::default();
            let empty_map = empty_obj.as_map().unwrap();
            assert_eq!(empty_map.len(), 0);

            // Test with non-object values (should return None)
            assert_eq!(MAAValue::from(true).as_map(), None);
            assert_eq!(MAAValue::from(42).as_map(), None);
            assert_eq!(MAAValue::from(2.14).as_map(), None);
            assert_eq!(MAAValue::from("string").as_map(), None);
            assert_eq!(MAAValue::from([1, 2]).as_map(), None);
        }

        #[test]
        fn as_mut_map() {
            // Test with object value - read access
            let mut obj = MAAValue::from([("key", "value")]);
            let map = obj.as_mut_map().unwrap();
            assert_eq!(map.len(), 1);
            assert!(map.contains_key("key"));

            // Test with object value - modify existing entry
            let map = obj.as_mut_map().unwrap();
            map.insert("key".to_string(), "new_value".into());
            assert_eq!(obj.get("key").unwrap().as_str(), Some("new_value"));

            // Test with object value - insert new entry
            let map = obj.as_mut_map().unwrap();
            map.insert("key2".to_string(), 42.into());
            assert_eq!(obj.get("key2").unwrap().as_int(), Some(42));

            // Test with object value - remove entry
            let map = obj.as_mut_map().unwrap();
            map.remove("key");
            assert!(obj.get("key").is_none());

            // Test with empty object
            let mut empty_obj = MAAValue::default();
            let map = empty_obj.as_mut_map().unwrap();
            map.insert("new_key".to_string(), "value".into());
            assert_eq!(empty_obj.get("new_key").unwrap().as_str(), Some("value"));

            // Test with non-object values (should return None)
            assert_eq!(MAAValue::from(true).as_mut_map(), None);
            assert_eq!(MAAValue::from(42).as_mut_map(), None);
            assert_eq!(MAAValue::from("string").as_mut_map(), None);
            assert_eq!(MAAValue::from([1, 2]).as_mut_map(), None);
        }

        #[test]
        fn as_slice() {
            // Test with array
            let array_value = MAAValue::from([1, 2, 3]);
            let slice = array_value.as_slice().unwrap();
            assert_eq!(slice.len(), 3);
            assert_eq!(slice[0].as_int(), Some(1));
            assert_eq!(slice[1].as_int(), Some(2));
            assert_eq!(slice[2].as_int(), Some(3));

            // Test with empty array
            let empty_array: [i32; 0] = [];
            let empty_value = MAAValue::from(empty_array);
            let empty_slice = empty_value.as_slice().unwrap();
            assert_eq!(empty_slice.len(), 0);

            // Test with non-array values (should return None)
            assert_eq!(MAAValue::from(1).as_slice(), None);
            assert_eq!(MAAValue::from(true).as_slice(), None);
            assert_eq!(MAAValue::from("string").as_slice(), None);
            assert_eq!(MAAValue::default().as_slice(), None);
        }

        #[test]
        fn as_mut_vec() {
            // Test with array - read access
            let mut array_value = MAAValue::from([1, 2, 3]);
            let vec = array_value.as_mut_vec().unwrap();
            assert_eq!(vec.len(), 3);
            assert_eq!(vec[0].as_int(), Some(1));

            // Test with array - modify existing elements
            let vec = array_value.as_mut_vec().unwrap();
            vec[0] = 10.into();
            vec[1] = 20.into();
            let slice = array_value.as_slice().unwrap();
            assert_eq!(slice[0].as_int(), Some(10));
            assert_eq!(slice[1].as_int(), Some(20));
            assert_eq!(slice[2].as_int(), Some(3));

            // Test with array - push new element
            let vec = array_value.as_mut_vec().unwrap();
            vec.push(4.into());
            let slice = array_value.as_slice().unwrap();
            assert_eq!(slice.len(), 4);
            assert_eq!(slice[3].as_int(), Some(4));

            // Test with array - pop element
            let vec = array_value.as_mut_vec().unwrap();
            let popped = vec.pop();
            assert_eq!(popped.unwrap().as_int(), Some(4));
            let slice = array_value.as_slice().unwrap();
            assert_eq!(slice.len(), 3);

            // Test with empty array
            let empty_array: [i32; 0] = [];
            let mut empty_value = MAAValue::from(empty_array);
            let vec = empty_value.as_mut_vec().unwrap();
            assert_eq!(vec.len(), 0);
            vec.push(1.into());
            assert_eq!(empty_value.as_slice().unwrap().len(), 1);

            // Test with non-array values (should return None)
            assert_eq!(MAAValue::from(1).as_mut_vec(), None);
            assert_eq!(MAAValue::from(true).as_mut_vec(), None);
            assert_eq!(MAAValue::from("string").as_mut_vec(), None);
            assert_eq!(MAAValue::default().as_mut_vec(), None);
        }

        #[test]
        fn as_primitive() {
            // Test with Primitive bool
            let bool_value = MAAValue::from(true);
            let primitive = bool_value.as_primitive().unwrap();
            assert_eq!(primitive.as_bool(), Some(true));

            // Test with Primitive int
            let int_value = MAAValue::from(42);
            let primitive = int_value.as_primitive().unwrap();
            assert_eq!(primitive.as_int(), Some(42));

            // Test with Primitive float
            let float_value = MAAValue::from(2.14);
            let primitive = float_value.as_primitive().unwrap();
            assert_eq!(primitive.as_float(), Some(2.14));

            // Test with Primitive string
            let string_value = MAAValue::from("hello");
            let primitive = string_value.as_primitive().unwrap();
            assert_eq!(primitive.as_str(), Some("hello"));

            // Test with non-Primitive values (should return None)
            assert_eq!(MAAValue::from([1, 2]).as_primitive(), None);
            assert_eq!(MAAValue::default().as_primitive(), None);
            assert_eq!(
                MAAValue::from(BoolInput::new(Some(true))).as_primitive(),
                None
            );
        }
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
