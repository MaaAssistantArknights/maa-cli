use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    input::MAAInput,
    map::{Map, StringMap},
    primitive::MAAPrimitive,
};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum MAAValueTemplate {
    /// An array of values
    Array(Vec<MAAValueTemplate>),
    /// A value that should be queried from user input
    Input(MAAInput),
    /// An optional value
    ///
    /// An optional value will be initialized only if all the dependencies are satisfied.
    /// If one of the dependencies is not exist or the value is not equal to the expected value,
    /// the optional value will be dropped after initialization.
    ///
    /// Note: Circular dependencies will cause error.
    Optional {
        /// A map of dependencies
        ///
        /// Keys are the keys of the dependencies in the same object and values are the expected
        #[serde(alias = "deps")]
        conditions: StringMap<MAAPrimitive>,
        /// Input value query from user when all the dependencies are satisfied
        #[serde(alias = "input", flatten)]
        value: BoxedMAAValueTemplate,
    },
    /// Object is a map of key-value pair
    Object(StringMap<MAAValueTemplate>),
    /// Primitive json types: bool, int, float, string
    Primitive(MAAPrimitive),
}

impl Default for MAAValueTemplate {
    fn default() -> Self {
        Self::Object(StringMap::default())
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(transparent)]
pub struct BoxedMAAValueTemplate(Box<MAAValueTemplate>);

impl BoxedMAAValueTemplate {
    fn resolve(self) -> Result<MAAValue> {
        self.0.resolve()
    }
}

impl<T> From<T> for BoxedMAAValueTemplate
where
    T: Into<MAAValueTemplate>,
{
    fn from(value: T) -> Self {
        Self(Box::new(value.into()))
    }
}

/// A concrete value containing only resolved data.
///
/// This type represents the output of [`MAAValueTemplate::resolve()`], containing only
/// concrete data without any [`Input`](MAAValueTemplate::Input) or
/// [`Optional`](MAAValueTemplate::Optional) variants. It implements both [`Serialize`] and
/// [`serde::de::Deserializer`] to support serialization and deserialization to any types
/// implementing [`Deserialize`].
///
/// # Creating `MAAValue` Directly
///
/// Use the [`object!`](maa_value_macro::object) macro for simple concrete objects:
///
/// ```
/// use maa_value::prelude::*;
///
/// let value = object!("name" => "app", "count" => 42);
/// assert_eq!(value.get("name").unwrap().as_str(), Some("app"));
/// ```
///
/// For templates with user inputs or conditionals, use [`template!`](maa_value_macro::template)
/// and call [`MAAValueTemplate::resolve()`] to obtain a `MAAValue`.
///
/// # Converting to Typed Structs
///
/// `MAAValue` implements the `Deserializer` trait, allowing direct conversion to typed structs
/// without an intermediate format:
///
/// ```
/// use maa_value::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Config {
///     name: String,
///     count: i32,
/// }
///
/// let value = object!("name" => "app", "count" => 42);
///
/// let config = Config::deserialize(value).unwrap();
///
/// assert_eq!(config.name, "app");
/// assert_eq!(config.count, 42);
/// ```
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MAAValue {
    /// An array of resolved values
    Array(Vec<Self>),
    /// An object containing resolved key-value pairs
    Object(StringMap<Self>),
    /// A primitive JSON value: bool, int, float, or string
    Primitive(MAAPrimitive),
}

impl Default for MAAValue {
    fn default() -> Self {
        Self::Object(StringMap::new())
    }
}

impl MAAValueTemplate {
    /// Resolves the value by evaluating all user inputs and conditional fields.
    ///
    /// This method transforms a [`MAAValueTemplate`] (which may contain unresolved
    /// [`Input`](Self::Input) and [`Optional`](Self::Optional) variants) into a
    /// [`MAAValue`] (which contains only concrete values). The resolution process
    /// recursively processes the value structure:
    ///
    /// - **Primitive**: Returns the value unchanged as a resolved primitive.
    /// - **Input**: Queries the user for input and converts the response to a primitive value.
    /// - **Array**: Recursively resolves each element in the array.
    /// - **Object**: Resolves all values in the object while handling optional fields based on
    ///   their conditions. Uses topological sorting (depth-first search) to process fields in
    ///   dependency order, ensuring that conditional dependencies are evaluated before the fields
    ///   that depend on them. Optional fields that don't satisfy their conditions are omitted from
    ///   the result.
    /// - **Optional**: Must be contained within an object. The optional field is only resolved and
    ///   included if all its condition dependencies are satisfied (i.e., the required fields exist
    ///   in the object and match their expected values). Otherwise, it is silently omitted.
    ///
    /// # Nested Optional Behavior
    ///
    /// When an optional field contains an object with its own optional fields, the nested
    /// resolution follows these rules:
    ///
    /// - If the outer optional's conditions are satisfied, the field is included and its value is
    ///   resolved recursively.
    /// - Nested optional fields within are evaluated independently based on their own conditions.
    /// - If all nested optionals are omitted, the result is an empty object (`{}`), not `None`.
    ///
    /// **Example:**
    /// ```
    /// use maa_value::prelude::*;
    ///
    /// let value = template!(
    ///     "enabled" => true,
    ///     "config" if "enabled" == true => template!(
    ///         "enabled" => false,
    ///         "nested_field" if "enabled" == true => "will_be_omitted",
    ///     ),
    /// );
    ///
    /// let resolved = value.resolve().unwrap();
    /// // "config" is included because "enabled" == true
    /// // But it's an empty object because the nested condition isn't satisfied
    /// assert_eq!(resolved.get("config").unwrap(), &object!("enabled" => false));
    /// ```
    ///
    /// # Performance
    ///
    /// For objects with optional fields, this method performs a topological sort using depth-first
    /// search to determine the resolution order. The complexity is O(V + E) where V is the number
    /// of fields and E is the number of dependencies. For typical configuration sizes (dozens to
    /// hundreds of fields), this overhead is negligible. The method is designed for configuration
    /// initialization, not high-frequency runtime operations.
    ///
    /// # Returns
    ///
    /// Returns a [`MAAValue`] containing only concrete values (no `Input` or `Optional` variants).
    ///
    /// # Errors
    ///
    /// Returns an error in the following cases:
    ///
    /// - [`Error::OptionalNotInObject`]: An `Optional` variant is encountered outside of an object
    ///   context.
    /// - [`Error::CircularDependency`]: Circular dependencies are detected among optional fields in
    ///   an object (e.g., field A depends on B, and B depends on A).
    /// - Other errors: Any errors encountered during resolution of nested values are propagated
    ///   upward (e.g., user input errors, type conversion errors).
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::prelude::*;
    ///
    /// // Resolving a simple template with primitives
    /// let value = template!("key" => "value", "count" => 42);
    /// let resolved = value.resolve().unwrap();
    /// assert_eq!(resolved.get("key").unwrap().as_str(), Some("value"));
    /// assert_eq!(resolved.get("count").unwrap().as_int(), Some(42));
    /// ```
    pub fn resolve(self) -> Result<MAAValue> {
        use MAAValueTemplate::*;
        match self {
            Input(v) => Ok(MAAValue::Primitive(v.into_primitive()?)),
            Array(array) => {
                let mut ret = Vec::with_capacity(array.len());
                for value in array {
                    ret.push(value.resolve()?);
                }
                Ok(MAAValue::Array(ret))
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
                    map: &'key StringMap<MAAValueTemplate>,
                    marks: &mut Map<&'key str, Mark>,
                ) -> Result<()> {
                    match marks.get(key) {
                        Some(Mark::Visited) => return Ok(()),
                        Some(Mark::Visiting) => {
                            return Err(Error::CircularDependency);
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
                let mut marks = Map::<&str, Mark>::new();

                for key in map.keys() {
                    visit(&mut sorted_keys, key, &map, &mut marks)?;
                }

                // Initialize all the values with given order and put them into a new map
                let mut initialized: StringMap<MAAValue> = StringMap::new();
                for key in sorted_keys {
                    let value = map.swap_remove(&key).unwrap();
                    if let Optional { conditions, value } = value {
                        let mut satisfied = true;
                        // Check if all the dependencies are satisfied
                        for (cond_key, expected) in conditions {
                            if let Some(v) = initialized.get(&cond_key) {
                                if v != &expected {
                                    // Value does not match expected, mark as unsatisfied
                                    satisfied = false;
                                    break;
                                } // else satisfied
                            } else {
                                // Dependency key is missing completely.
                                // TODO: Return an Error instead of silently failing.
                                log::warn!(
                                    "Optional field '{key}' depends on a missing key '{cond_key}'. \
                                         This condition is considered unmet and the field will be ignored. \
                                         Please consider fixing this, this would be a hard failure in the future."
                                );
                                satisfied = false;
                                break;
                            }
                        }
                        // if all the dependencies are satisfied, initialize the value
                        if satisfied {
                            initialized.insert(key, value.resolve()?);
                        }
                    } else {
                        initialized.insert(key, value.resolve()?);
                    }
                }

                Ok(MAAValue::Object(initialized))
            }
            Optional { .. } => Err(Error::OptionalNotInObject),
            Primitive(p) => Ok(MAAValue::Primitive(p)),
        }
    }
}

impl From<MAAValue> for MAAValueTemplate {
    fn from(value: MAAValue) -> Self {
        match value {
            MAAValue::Primitive(p) => Self::Primitive(p),
            MAAValue::Array(a) => Self::Array(a.into_iter().map(Into::into).collect()),
            MAAValue::Object(m) => {
                Self::Object(m.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

impl<const N: usize, T: Into<MAAValueTemplate>> From<[T; N]> for MAAValueTemplate {
    fn from(value: [T; N]) -> Self {
        Self::Array(value.into_iter().map(|v| v.into()).collect::<Vec<_>>())
    }
}

impl<const N: usize, T: Into<MAAValue>> From<[T; N]> for MAAValue {
    fn from(value: [T; N]) -> Self {
        Self::Array(value.into_iter().map(|v| v.into()).collect::<Vec<_>>())
    }
}

impl<T: TryInto<MAAValueTemplate>> TryFrom<Vec<T>> for MAAValueTemplate {
    type Error = T::Error;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        Ok(Self::Array(
            value
                .into_iter()
                .map(|v| v.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl<T: TryInto<MAAValue>> TryFrom<Vec<T>> for MAAValue {
    type Error = T::Error;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        Ok(Self::Array(
            value
                .into_iter()
                .map(|v| v.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl From<MAAValueTemplate> for Cow<'_, MAAValueTemplate> {
    fn from(value: MAAValueTemplate) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a MAAValueTemplate> for Cow<'a, MAAValueTemplate> {
    fn from(value: &'a MAAValueTemplate) -> Self {
        Cow::Borrowed(value)
    }
}

impl From<MAAValue> for Cow<'_, MAAValue> {
    fn from(value: MAAValue) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a MAAValue> for Cow<'a, MAAValue> {
    fn from(value: &'a MAAValue) -> Self {
        Cow::Borrowed(value)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::num::NonZero;

    use super::*;
    use crate::prelude::*;

    #[test]
    fn serde() {
        use serde_test::Token;

        let obj = template!(
            "array" => [1, 2],
            "bool" => true,
            "float" => 1.0,
            "int" => 1,
            "object" => template!("key" => "value"),
            "string" => "string",
            "input_bool" => BoolInput::new(Some(true)),
            "input_float" => Input::new(Some(1.0)),
            "input_int" => Input::new(Some(1)),
            "input_string" => Input::new(Some("string".to_string())),
            "select_int" => SelectD::from_iter([1, 2], NonZero::new(2)).unwrap(),
            "select_float" => SelectD::from_iter([1.0, 2.0], NonZero::new(2)).unwrap(),
            "select_string" => SelectD::<String>::from_iter(["string1", "string2"], NonZero::new(2)).unwrap(),
            "optional" if "input_bool" == true => Input::new(Some(1)),
            "optional_no_satisfied" if "input_bool" == false => Input::new(Some(1)),
            "optional_object" if "input_bool" == true =>
                template!("key1" => "value1", "key2" => "value2"),
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

        let obj = obj.resolve().unwrap();

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
            Token::Bool(true),
            Token::Str("input_float"),
            Token::F32(1.0),
            Token::Str("input_int"),
            Token::I32(1),
            Token::Str("input_string"),
            Token::Str("string"),
            Token::Str("select_int"),
            Token::I32(2),
            Token::Str("select_float"),
            Token::F32(2.0),
            Token::Str("select_string"),
            Token::Str("string2"),
            Token::Str("optional"),
            Token::I32(1),
            Token::Str("optional_object"),
            Token::Map { len: Some(2) },
            Token::Str("key1"),
            Token::Str("value1"),
            Token::Str("key2"),
            Token::Str("value2"),
            Token::MapEnd,
            Token::MapEnd,
        ]);
    }

    #[test]
    fn resolve_optionals() {
        let input = BoolInput::new(Some(true));

        let value = template!(
            "input" => input.clone(),
            "array" => [1],
            "primitive" => 1,
            "optional" if "input" == true => input.clone(),
            "optional_no_satisfied" if "input" == false => input.clone(),
            "optional_no_exist" if "no_exist" == true => input.clone(),
            "optional_chain" if "optional" == true => input.clone(),
            "optional_nested" if "optional" == true => template!(
                "nested" if "optional" == true => input.clone(),
            ),
        );

        let optional_uninitialized = value.get("optional").unwrap().clone();
        assert!(matches!(
            optional_uninitialized.resolve().unwrap_err(),
            Error::OptionalNotInObject,
        ));

        assert_eq!(
            value.get("input").unwrap(),
            &MAAValueTemplate::from(input.clone())
        );
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValueTemplate::Array(vec![1.into()])
        );
        assert_eq!(value.get("primitive").unwrap(), &MAAValueTemplate::from(1));
        assert!(matches!(
            value.get("optional").unwrap(),
            MAAValueTemplate::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_no_satisfied").unwrap(),
            MAAValueTemplate::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_no_exist").unwrap(),
            MAAValueTemplate::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_chain").unwrap(),
            MAAValueTemplate::Optional { .. }
        ));
        assert!(matches!(
            value.get("optional_nested").unwrap(),
            MAAValueTemplate::Optional { .. }
        ));

        let value = value.resolve().unwrap();

        assert_eq!(value.get("input").unwrap(), &MAAValue::from(true));
        assert_eq!(
            value.get("array").unwrap(),
            &MAAValue::Array(vec![1.into()])
        );
        assert_eq!(value.get("primitive").unwrap(), &MAAValue::from(1));
        assert_eq!(value.get("optional").unwrap(), &MAAValue::from(true));
        assert_eq!(value.get("optional_no_satisfied"), None);
        assert_eq!(value.get("optional_no_exist"), None);
        assert_eq!(value.get("optional_chain").unwrap(), &MAAValue::from(true));
        assert_eq!(value.get("optional_nested").unwrap(), &MAAValue::default());

        let value = template!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional1" == true => input.clone(),
        );
        assert!(matches!(
            value.resolve().unwrap_err(),
            Error::CircularDependency,
        ));

        let value = template!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional3" == true => input.clone(),
            "optional3" if "optional1" == true => input.clone(),
        );
        assert!(matches!(
            value.resolve().unwrap_err(),
            Error::CircularDependency,
        ));
    }

    #[test]
    fn value_creation() {
        let bool_val = MAAValue::from(true);
        assert_eq!(bool_val.as_bool(), Some(true));

        let int_val = MAAValue::from(42);
        assert_eq!(int_val.as_int(), Some(42));

        let float_val = MAAValue::from(2.14);
        assert_eq!(float_val.as_float(), Some(2.14));

        let str_val = MAAValue::from("hello");
        assert_eq!(str_val.as_str(), Some("hello"));

        let array_val = MAAValue::from([1, 2, 3]);
        match array_val {
            MAAValue::Array(vec) => {
                assert_eq!(vec.len(), 3);
                assert_eq!(vec[0].as_int(), Some(1));
            }
            _ => panic!("Expected Array variant"),
        }

        let vec_val = MAAValue::try_from(vec![1, 2, 3]).unwrap();
        match vec_val {
            MAAValue::Array(vec) => {
                assert_eq!(vec.len(), 3);
            }
            _ => panic!("Expected Array variant"),
        }

        // Test Default
        let default_val = MAAValue::default();
        assert!(matches!(default_val, MAAValue::Object(_)));
        assert_eq!(default_val.as_map().unwrap().len(), 0);
    }

    #[test]
    fn value_equality() {
        // Test primitive equality
        assert_eq!(MAAValue::from(42), MAAValue::from(42));
        assert_ne!(MAAValue::from(42), MAAValue::from(43));

        assert_eq!(MAAValue::from("hello"), MAAValue::from("hello"));
        assert_ne!(MAAValue::from("hello"), MAAValue::from("world"));

        // Test array equality
        assert_eq!(MAAValue::from([1, 2, 3]), MAAValue::from([1, 2, 3]));
        assert_ne!(MAAValue::from([1, 2, 3]), MAAValue::from([1, 2, 4]));

        // Test empty arrays
        let empty1: [i32; 0] = [];
        let empty2: [i32; 0] = [];
        assert_eq!(MAAValue::from(empty1), MAAValue::from(empty2));

        // Test resolved object equality
        let obj1 = object!("key" => 1);
        let obj2 = object!("key" => 1);
        let obj3 = object!("key" => 2);

        assert_eq!(obj1, obj2);
        assert_ne!(obj1, obj3);

        // Test cross-type inequality
        assert_ne!(MAAValue::from(1), MAAValue::from("1"));
        assert_ne!(MAAValue::from(1), MAAValue::from([1]));
    }

    #[test]
    fn value_nested_structures() {
        // Test nested arrays
        let nested_array = MAAValueTemplate::from([
            MAAValueTemplate::from([1, 2]),
            MAAValueTemplate::from([3, 4]),
        ])
        .resolve()
        .unwrap();

        let outer = nested_array.as_slice().unwrap();
        assert_eq!(outer.len(), 2);
        assert_eq!(outer[0].as_slice().unwrap().len(), 2);
        assert_eq!(outer[0].as_slice().unwrap()[0].as_int(), Some(1));

        // Test nested objects
        let nested_obj = object!(
            "outer" => object!(
                "inner" => object!(
                    "value" => 42
                )
            )
        );

        let outer = nested_obj.get("outer").unwrap();
        let inner = outer.get("inner").unwrap();
        assert_eq!(inner.get("value").unwrap().as_int(), Some(42));

        // Test mixed nesting (array in object in array)
        let mixed = MAAValueTemplate::from([template!("key" => [1, 2])])
            .resolve()
            .unwrap();

        let arr = mixed.as_slice().unwrap();
        let obj = &arr[0];
        let inner_arr = obj.get("key").unwrap().as_slice().unwrap();
        assert_eq!(inner_arr.len(), 2);
        assert_eq!(inner_arr[0].as_int(), Some(1));
    }

    #[test]
    fn value_cloning() {
        // Test that cloning works correctly
        let original = MAAValue::from([1, 2, 3]);
        let cloned = original.clone();

        assert_eq!(original, cloned);

        // Test cloning with nested structures
        let nested = object!(
            "array" => [1, 2],
            "obj" => object!("key" => "value")
        );

        let cloned_nested = nested.clone();
        assert_eq!(nested, cloned_nested);

        // Verify they're independent
        assert_eq!(
            nested.get("array").unwrap().as_slice().unwrap().len(),
            cloned_nested
                .get("array")
                .unwrap()
                .as_slice()
                .unwrap()
                .len()
        );
    }

    #[test]
    fn resolve_primitives() {
        // Test resolving primitive values directly
        assert_eq!(
            MAAValueTemplate::from(42).resolve().unwrap(),
            MAAValue::from(42)
        );

        assert_eq!(
            MAAValueTemplate::from(true).resolve().unwrap(),
            MAAValue::from(true)
        );

        assert_eq!(
            MAAValueTemplate::from("hello").resolve().unwrap(),
            MAAValue::from("hello")
        );

        assert_eq!(
            MAAValueTemplate::from(2.14).resolve().unwrap(),
            MAAValue::from(2.14)
        );
    }

    #[test]
    fn resolve_arrays() {
        // Test resolving simple array
        let array = MAAValueTemplate::from([1, 2, 3]);
        let resolved = array.resolve().unwrap();
        assert_eq!(resolved, MAAValue::from([1, 2, 3]));

        // Test resolving empty array
        let empty: [i32; 0] = [];
        let empty_resolved = MAAValueTemplate::from(empty).resolve().unwrap();
        assert_eq!(empty_resolved, MAAValue::from(empty));

        // Test resolving array with inputs
        let with_inputs = MAAValueTemplate::from([
            MAAValueTemplate::from(1),
            MAAValueTemplate::from(Input::new(Some(2))),
            MAAValueTemplate::from(3),
        ]);
        let resolved = with_inputs.resolve().unwrap();
        let slice = resolved.as_slice().unwrap();
        assert_eq!(slice.len(), 3);
        assert_eq!(slice[0].as_int(), Some(1));
        assert_eq!(slice[1].as_int(), Some(2));
        assert_eq!(slice[2].as_int(), Some(3));
    }

    #[test]
    fn resolve_objects() {
        // Test resolving simple object
        let resolved = object!("key1" => 1, "key2" => "value");

        assert_eq!(resolved.get("key1").unwrap().as_int(), Some(1));
        assert_eq!(resolved.get("key2").unwrap().as_str(), Some("value"));

        // Test resolving empty object
        let resolved_empty = object!();
        assert_eq!(resolved_empty.as_map().unwrap().len(), 0);

        // Test resolving object with inputs
        let with_inputs = template!(
            "direct" => 1,
            "input" => Input::new(Some(2))
        );
        let resolved = with_inputs.resolve().unwrap();
        assert_eq!(resolved.get("direct").unwrap().as_int(), Some(1));
        assert_eq!(resolved.get("input").unwrap().as_int(), Some(2));
    }

    mod serde_tests {
        use serde_json::{self, json};

        use super::*;

        mod deserialize {
            use super::*;

            #[test]
            fn empty_map_is_object() {
                // Test that an empty map is deserialized as an object instead of an input
                let json = json!({});
                let value: MAAValueTemplate = serde_json::from_value(json).unwrap();
                assert!(matches!(value, MAAValueTemplate::Object(_)));
            }

            #[test]
            fn input_variant() {
                // Test Input variant with default value
                let json = json!({"default": 42});
                let value: MAAValueTemplate = serde_json::from_value(json).unwrap();
                assert!(matches!(value, MAAValueTemplate::Input(_)));

                // Test Input variant with description
                let json = json!({
                    "default": "hello",
                    "description": "Enter a greeting"
                });
                let value: MAAValueTemplate = serde_json::from_value(json).unwrap();
                assert!(matches!(value, MAAValueTemplate::Input(_)));
            }

            #[test]
            fn optional_variant() {
                // Test Optional variant with "conditions" key
                let json = json!({
                    "conditions": {"enabled": true},
                    "default": 42
                });
                let value: MAAValueTemplate = serde_json::from_value(json).unwrap();
                assert!(matches!(value, MAAValueTemplate::Optional { .. }));

                // Test Optional variant with "deps" alias
                let json = json!({
                    "deps": {"flag": false},
                    "default": "value"
                });
                let value: MAAValueTemplate = serde_json::from_value(json).unwrap();
                assert!(matches!(value, MAAValueTemplate::Optional { .. }));

                // Test Optional with flatten (object value)
                let json = json!({
                    "conditions": {"mode": "advanced"},
                    "key1": "value1",
                    "key2": "value2"
                });
                let value: MAAValueTemplate = serde_json::from_value(json).unwrap();
                assert!(matches!(value, MAAValueTemplate::Optional { .. }));
            }

            #[test]
            fn resolved_value_preserves_object_order() {
                let value: MAAValue =
                    serde_json::from_str(r#"{"z":1,"a":{"k2":2,"k1":1},"m":3}"#).unwrap();

                let keys = value
                    .as_map()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                assert_eq!(keys, ["z", "a", "m"]);

                let nested_keys = value
                    .get("a")
                    .unwrap()
                    .as_map()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                assert_eq!(nested_keys, ["k2", "k1"]);
            }
        }

        mod serialize {
            use super::*;

            #[test]
            fn resolved_value() {
                // Test that MAAValue serializes correctly
                let value = object!(
                    "primitive" => 42,
                    "array" => [1, 2, 3],
                    "nested" => object!("key" => "value")
                );

                let json = serde_json::to_value(&value).unwrap();

                assert_eq!(json["primitive"], 42);
                assert_eq!(json["array"], json!([1, 2, 3]));
                assert_eq!(json["nested"]["key"], "value");
            }

            #[test]
            fn resolved_value_preserves_object_order() {
                let value = object!(
                    "z" => 1,
                    "a" => object!(
                        "k2" => 2,
                        "k1" => 1,
                    ),
                    "m" => 3,
                );

                assert_eq!(
                    serde_json::to_string(&value).unwrap(),
                    r#"{"z":1,"a":{"k2":2,"k1":1},"m":3}"#
                );
            }
        }

        mod integration {
            use super::*;

            #[test]
            fn resolve_inputs_then_serialize() {
                // Test that Input variants resolve to their default values
                let value = template!(
                    "direct" => 42,
                    "from_input" => Input::new(Some(100)),
                    "array_with_input" => [
                        MAAValueTemplate::from(1),
                        MAAValueTemplate::from(Input::new(Some(2)))
                    ]
                );

                let resolved = value.resolve().unwrap();
                let json = serde_json::to_value(&resolved).unwrap();

                // Inputs should be resolved to their default values
                assert_eq!(json["direct"], 42);
                assert_eq!(json["from_input"], 100);
                assert_eq!(json["array_with_input"][0], 1);
                assert_eq!(json["array_with_input"][1], 2);
            }

            #[test]
            fn resolve_optionals_then_serialize() {
                // Test that Optional variants are evaluated based on conditions
                let value = template!(
                    "flag" => true,
                    "conditional" if "flag" == true => 42,
                    "not_included" if "flag" == false => 99
                );

                let resolved = value.resolve().unwrap();
                let json = serde_json::to_value(&resolved).unwrap();

                // Satisfied optional should be included
                assert_eq!(json["conditional"], 42);

                // Unsatisfied optional should not be included
                assert!(json.get("not_included").is_none());
            }
        }
    }
}
