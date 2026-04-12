#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::ops::{Deref, DerefMut};

#[cfg(feature = "schema")]
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// A vector guaranteed to contain at least one element.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmptyVec<T>(Vec<T>);

impl<T> NonEmptyVec<T> {
    /// Creates a new `NonEmptyVec` from `vec`.
    ///
    /// Returns `None` if `vec` is empty.
    pub fn new(vec: Vec<T>) -> Option<Self> {
        if vec.is_empty() {
            None
        } else {
            Some(Self(vec))
        }
    }

    /// Collects `iter` into a `NonEmptyVec`.
    ///
    /// Returns `None` if the iterator yields no items.
    pub fn collect<I: IntoIterator<Item = T>>(iter: I) -> Option<Self> {
        Self::new(iter.into_iter().collect())
    }

    /// Returns the number of elements in the vector.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `false`.
    ///
    /// This exists only for API compatibility with slice-like code paths.
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Returns a shared reference to the first element.
    pub fn first(&self) -> &T {
        // SAFETY: the vector is non-empty, so the first element always exists
        unsafe { self.0.get_unchecked(0) }
    }

    /// Returns a shared reference to the last element.
    pub fn last(&self) -> &T {
        // SAFETY: the vector is non-empty, so the last element always exists
        unsafe { self.0.get_unchecked(self.0.len() - 1) }
    }

    /// Consumes `self` and returns the underlying `Vec`.
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<T> Deref for NonEmptyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<T> DerefMut for NonEmptyVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}

#[cfg(feature = "serde")]
impl<T: Serialize> Serialize for NonEmptyVec<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: Deserialize<'de>> Deserialize<'de> for NonEmptyVec<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<T>::deserialize(deserializer)?;
        Self::new(vec).ok_or_else(|| D::Error::invalid_length(0, &"a non-empty array"))
    }
}

#[cfg(feature = "schema")]
impl<T: JsonSchema> JsonSchema for NonEmptyVec<T> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        format!("NonEmptyArray_of_{}", T::schema_name()).into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("nonempty_vec::NonEmptyVec<{}>", T::schema_id()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "array",
            "minItems": 1,
            "items": generator.subschema_for::<T>(),
        })
    }
}

#[macro_export]
macro_rules! nevev {
    [$item:expr $(, $rest:expr)* $(,)?] => {
        NonEmptyVec(vec![$item, $($rest),*])
    };
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::NonEmptyVec;

    #[test]
    fn new_rejects_empty_vec() {
        assert_eq!(NonEmptyVec::<i32>::new(vec![]), None);
    }

    #[test]
    fn new_accepts_non_empty_vec() {
        let vec = NonEmptyVec::new(vec![1, 2, 3]).unwrap();
        assert_eq!(&*vec, &[1, 2, 3]);
        assert_eq!(vec.first(), &1);
        assert_eq!(vec.last(), &3);
    }

    #[test]
    fn from_iter_rejects_empty_iterator() {
        assert_eq!(NonEmptyVec::<i32>::collect(std::iter::empty()), None);
    }

    #[test]
    fn collect_accepts_non_empty_iterator() {
        let vec = NonEmptyVec::collect([1, 2, 3]).unwrap();
        assert_eq!(&*vec, &[1, 2, 3]);
    }

    #[test]
    fn len_and_is_empty_reflect_invariant() {
        let vec = nevev![1, 2, 3];
        assert_eq!(vec.len(), 3);
        assert!(!vec.is_empty());
    }

    #[test]
    fn into_vec_returns_inner_vec() {
        let vec = nevev![1, 2, 3];
        assert_eq!(vec.into_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn first_and_last_always_return() {
        let vec = nevev![42];
        assert_eq!(vec.first(), &42);
        assert_eq!(vec.last(), &42);
    }

    #[test]
    fn deref_gives_correct_slice() {
        let vec = nevev![1, 2, 3];
        let slice: &[i32] = &vec;
        assert_eq!(slice, &[1, 2, 3]);
    }

    #[test]
    fn deref_mut_can_modify_inner_vec() {
        let mut vec = nevev![1, 2, 3];
        vec.iter_mut().for_each(|i| *i *= 2);
        assert_eq!(&*vec, &[2, 4, 6]);
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;

        #[test]
        fn serde_roundtrip_preserves_values() {
            let vec = nevev![1, 2, 3];
            let json = serde_json::to_string(&vec).unwrap();
            assert_eq!(json, "[1,2,3]");
            let restored: NonEmptyVec<i32> = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, vec);
        }

        #[cfg(feature = "serde")]
        #[test]
        fn serde_rejects_empty_array() {
            let error = serde_json::from_str::<NonEmptyVec<i32>>("[]").unwrap_err();
            assert!(error.to_string().contains("a non-empty array"));
        }

        #[cfg(feature = "schema")]
        #[test]
        fn schema_marks_array_as_non_empty() {
            let schema = schemars::schema_for!(NonEmptyVec<i32>);
            let schema = serde_json::to_value(&schema).unwrap();

            assert_eq!(schema["type"], "array");
            assert_eq!(schema["minItems"], 1);
            assert_eq!(schema["items"]["type"], "integer");
        }
    }

    mod nevev_macro {
        use super::NonEmptyVec;

        #[test]
        fn nevev_single_element() {
            let vec: NonEmptyVec<i32> = nevev![1];
            assert_eq!(&*vec, &[1]);
        }

        #[test]
        fn nevev_multiple_elements() {
            let vec = nevev![1, 2, 3];
            assert_eq!(&*vec, &[1, 2, 3]);
        }

        #[test]
        fn nevev_trailing_comma() {
            let vec = nevev![1, 2, 3,];
            assert_eq!(&*vec, &[1, 2, 3]);
        }
    }
}
