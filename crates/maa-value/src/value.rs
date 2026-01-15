use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    input::MAAInput,
    map::Map,
    primitive::MAAPrimitive,
};

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
    /// Note: Circular dependencies will cause error.
    Optional {
        /// A map of dependencies
        ///
        /// Keys are the keys of the dependencies in the sam object and values are the expected
        #[serde(alias = "deps")]
        conditions: Map<MAAPrimitive>,
        /// Input value query from user when all the dependencies are satisfied
        #[serde(alias = "input", flatten)]
        value: BoxedMAAValue,
    },
    /// Object is a map of key-value pair
    Object(Map<MAAValue>),
    /// Primitive json types: bool, int, float, string
    Primitive(MAAPrimitive),
}

impl Default for MAAValue {
    fn default() -> Self {
        Self::Object(Map::default())
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(transparent)]
pub struct BoxedMAAValue(Box<MAAValue>);

impl BoxedMAAValue {
    fn resolve(self) -> Result<ResolvedMAAValue> {
        self.0.resolve()
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

/// A resolved MAAValue containing only concrete values.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ResolvedMAAValue {
    /// An array of resolved values
    Array(Vec<Self>),
    /// An object containing resolved key-value pairs
    Object(Map<Self>),
    /// A primitive JSON value: bool, int, float, or string
    Primitive(MAAPrimitive),
}

impl Default for ResolvedMAAValue {
    fn default() -> Self {
        Self::Object(Map::new())
    }
}

impl MAAValue {
    /// Resolves the value by evaluating all user inputs and conditional fields.
    ///
    /// This method transforms a [`MAAValue`] (which may contain unresolved
    /// [`Input`](MAAValue::Input) and [`Optional`](MAAValue::Optional) variants) into a
    /// [`ResolvedMAAValue`] (which contains only concrete values). The resolution process
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
    /// # Returns
    ///
    /// Returns a [`ResolvedMAAValue`] containing only concrete values (no `Input` or `Optional`
    /// variants).
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
    /// // Resolving a simple object with primitives
    /// let value = object!("key" => "value", "count" => 42);
    /// let resolved = value.resolve().unwrap();
    /// assert_eq!(resolved.get("key").unwrap().as_str(), Some("value"));
    /// assert_eq!(resolved.get("count").unwrap().as_int(), Some(42));
    /// ```
    pub fn resolve(self) -> Result<ResolvedMAAValue> {
        use MAAValue::*;
        match self {
            Input(v) => Ok(ResolvedMAAValue::Primitive(v.into_primitive()?)),
            Array(array) => {
                let mut ret = Vec::with_capacity(array.len());
                for value in array {
                    ret.push(value.resolve()?);
                }
                Ok(ResolvedMAAValue::Array(ret))
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
                    map: &'key Map<MAAValue>,
                    marks: &mut Map<Mark, &'key str>,
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
                let mut marks = std::collections::BTreeMap::<&str, Mark>::new();

                for key in map.keys() {
                    visit(&mut sorted_keys, key, &map, &mut marks)?;
                }

                // Initialize all the values with given order and put them into a new map
                let mut initialized: Map<ResolvedMAAValue> = Map::new();
                for key in sorted_keys {
                    let value = map.remove(&key).unwrap();
                    if let Optional { conditions, value } = value {
                        let mut satisfied = true;
                        // Check if all the dependencies are satisfied
                        for (cond_key, expected) in conditions {
                            // If the dependency is not exist or the value is not equal to the
                            // expected values break the loop and mark
                            // status as unsatisfied
                            if !initialized.get(&cond_key).is_some_and(
                                |v| matches!(v, ResolvedMAAValue::Primitive(p) if p == &expected),
                            ) {
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

                Ok(ResolvedMAAValue::Object(initialized))
            }
            Optional { .. } => Err(Error::OptionalNotInObject),
            Primitive(p) => Ok(ResolvedMAAValue::Primitive(p)),
        }
    }
}

impl<const N: usize, T: Into<MAAValue>> From<[T; N]> for MAAValue {
    fn from(value: [T; N]) -> Self {
        Self::Array(value.into_iter().map(|v| v.into()).collect::<Vec<_>>())
    }
}

impl<const N: usize, T: Into<ResolvedMAAValue>> From<[T; N]> for ResolvedMAAValue {
    fn from(value: [T; N]) -> Self {
        Self::Array(value.into_iter().map(|v| v.into()).collect::<Vec<_>>())
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

impl<T: TryInto<ResolvedMAAValue>> TryFrom<Vec<T>> for ResolvedMAAValue {
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

impl From<ResolvedMAAValue> for Cow<'_, ResolvedMAAValue> {
    fn from(value: ResolvedMAAValue) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a ResolvedMAAValue> for Cow<'a, ResolvedMAAValue> {
    fn from(value: &'a ResolvedMAAValue) -> Self {
        Cow::Borrowed(value)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::num::NonZero;

    use crate::{
        error::Error,
        map::MapOps,
        userinput::{BoolInput, Input, SelectD},
    };
    use maa_value_macro::object;

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
    }

    #[test]
    fn resolve_optionals() {
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
            optional_uninitialized.resolve().unwrap_err(),
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

        let value = value.resolve().unwrap();

        assert_eq!(value.get("input").unwrap(), &ResolvedMAAValue::from(true));
        assert_eq!(
            value.get("array").unwrap(),
            &ResolvedMAAValue::Array(vec![1.into()])
        );
        assert_eq!(value.get("primitive").unwrap(), &ResolvedMAAValue::from(1));
        assert_eq!(
            value.get("optional").unwrap(),
            &ResolvedMAAValue::from(true)
        );
        assert_eq!(value.get("optional_no_satisfied"), None);
        assert_eq!(value.get("optional_no_exist"), None);
        assert_eq!(
            value.get("optional_chian").unwrap(),
            &ResolvedMAAValue::from(true)
        );
        assert_eq!(
            value.get("optional_nested").unwrap(),
            &object!().resolve().unwrap()
        );

        let value = object!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional1" == true => input.clone(),
        );
        assert!(matches!(
            value.resolve().unwrap_err(),
            Error::CircularDependency,
        ));

        let value = object!(
            "optional1" if "optional2" == true => input.clone(),
            "optional2" if "optional3" == true => input.clone(),
            "optional3" if "optional1" == true => input.clone(),
        );
        assert!(matches!(
            value.resolve().unwrap_err(),
            Error::CircularDependency,
        ));
    }
}
