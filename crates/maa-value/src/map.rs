use std::borrow::Cow;

use crate::{
    Outcome,
    convert::TryAs,
    value::{MAAValue, ResolvedMAAValue},
};

/// Type alias for the underlying map structure.
///
/// Uses `BTreeMap` to maintain key ordering for consistent serialization.
pub type Map<T, K = String> = std::collections::BTreeMap<K, T>;

/// Trait for map-like operations on values.
///
/// Provides methods for accessing and manipulating map-like structures.
pub trait MapOps: Sized + Clone {
    /// Returns a reference to the inner map if this value is map-like.
    ///
    /// Returns `None` if the value is not map-like.
    fn as_map(&self) -> Option<&Map<Self>>;

    /// Returns a mutable reference to the inner map if this value is map-like.
    ///
    /// Returns `None` if the value is not map-like.
    fn as_mut_map(&mut self) -> Option<&mut Map<Self>>;

    /// Consumes the value and returns the inner map if this value is map-like.
    ///
    /// Returns back if the value is not map-like.
    fn into_map(self) -> Outcome<Map<Self>, Self>;

    /// Returns a reference to the value associated with the given key.
    ///
    /// Returns `None` if the key is not found or if the value is not map-like.
    fn get(&self, key: &str) -> Option<&Self> {
        self.as_map().and_then(|m| m.get(key))
    }

    /// Returns a mutable reference to the value associated with the given key.
    ///
    /// Returns `None` if the key is not found or if the value is not map-like.
    fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        self.as_mut_map().and_then(|m| m.get_mut(key))
    }

    /// Gets a typed value from the map by key.
    ///
    /// This method retrieves a value and attempts to convert it to the specified type
    /// using [`TryAs::try_as`].
    ///
    /// Returns `None` if the key is not found, the value is not map-like,
    /// or the type conversion fails.
    fn get_typed<'a, T>(&'a self, key: &str) -> Option<T>
    where
        Self: TryAs<'a, T>,
    {
        self.get(key).and_then(TryAs::try_as)
    }

    /// Gets a typed value from the map by key, returning a default if not found.
    ///
    /// This is a convenience method that combines [`MapOps::get_typed`] with a default value.
    /// If the key doesn't exist, the value is not map-like, or the type conversion fails,
    /// the provided default is returned.
    fn get_or<'a, T>(&'a self, key: &str, default: T) -> T
    where
        Self: TryAs<'a, T>,
    {
        self.get_typed(key).unwrap_or(default)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the value is not map-like, this method does nothing.
    fn insert(&mut self, key: impl Into<String>, value: Self) {
        self.as_mut_map().map(|m| m.insert(key.into(), value));
    }

    /// Inserts a key-value pair into the map if the value is `Some`.
    ///
    /// If the value is `None` or the value is not map-like, this method does nothing.
    fn maybe_insert(&mut self, key: impl Into<String>, value: Option<Self>) {
        if let Some(value) = value {
            self.insert(key, value);
        }
    }

    /// Merges an owned value into `self`, consuming `other`.
    ///
    /// This method modifies `self` in place, taking ownership of `other`.
    ///
    /// # Behavior
    ///
    /// - **Map-like values**: Recursively merges key-value pairs. If a key exists in both:
    ///   - If both values are map-like, they are recursively merged.
    ///   - Otherwise, the value from `other` replaces the value in `self`.
    ///   - Keys that only exist in `other` are added to `self`.
    /// - **Non-map-like values**: The value in `self` is completely replaced by `other`.
    ///
    /// # Performance
    ///
    /// This is the most efficient merge variant as it consumes `other` and moves values
    /// instead of cloning them.
    ///
    /// See also: [`Self::merge_from`] for borrowed variant, [`Self::join`] for non-mutating
    /// variant.
    fn merge(&mut self, other: Self) {
        if let Some(map) = self.as_mut_map() {
            match other.into_map() {
                Outcome::Value(other) => {
                    for (key, value) in other {
                        if let Some(self_value) = map.get_mut(&key) {
                            self_value.merge(value);
                        } else {
                            map.insert(key, value);
                        }
                    }
                }
                Outcome::Original(other) => *self = other,
            }
        } else {
            *self = other;
        }
    }

    /// Merges a borrowed value into `self`, cloning values from `other` as needed.
    ///
    /// This method borrows `other` and merges it into `self`, modifying `self` in place.
    /// Values from `other` are cloned when inserted into `self`.
    ///
    /// # Behavior
    ///
    /// - **Map-like values**: Recursively merges key-value pairs. If a key exists in both:
    ///   - If both values are map-like, they are recursively merged.
    ///   - Otherwise, the value from `other` replaces the value in `self`.
    ///   - Keys that only exist in `other` are added to `self`.
    /// - **Non-map-like values**: The value in `self` is completely replaced by a clone of `other`.
    ///
    /// # Performance
    ///
    /// This variant clones values from `other`, making it less efficient than [`Self::merge`].
    /// Use this when you need to keep `other` after the merge.
    ///
    /// See also: [`Self::merge`] for owned variant, [`Self::join`] for non-mutating variant.
    fn merge_from(&mut self, other: &Self) {
        match (self.as_mut_map(), other.as_map()) {
            (Some(self_map), Some(other_map)) => {
                for (key, value) in other_map {
                    if let Some(self_value) = self_map.get_mut(key) {
                        self_value.merge_from(value);
                    } else {
                        self_map.insert(key.clone(), value.clone());
                    }
                }
            }
            _ => {
                *self = other.clone();
            }
        }
    }

    /// Creates a new value by merging `other` into a clone of `self`.
    ///
    /// This is a non-mutating variant of merge operations. It clones `self`, merges `other`
    /// into the clone, and returns the result. Neither `self` nor `other` is modified.
    ///
    /// # Behavior
    ///
    /// - **Map-like values**: Recursively merges key-value pairs. If a key exists in both:
    ///   - If both values are map-like, they are recursively merged.
    ///   - Otherwise, the value from `other` replaces the value from `self` in the result.
    ///   - Keys that only exist in either are included in the result.
    /// - **Non-map-like values**: Returns a clone of `other`.
    ///
    /// # Performance
    ///
    /// This method accepts either owned or borrowed values via the `Into<Cow<'a, Self>>` bound:
    ///
    /// - **Owned values** (`value.join(other)`): Uses [`Self::merge`] internally, which moves
    ///   values from `other` without cloning (more efficient).
    /// - **Borrowed values** (`value.join(&other)`): Uses [`Self::merge_from`] internally, which
    ///   clones values from `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use maa_value::prelude::*;
    ///
    /// let base = object!("a" => 1, "b" => 2);
    /// let update = object!("b" => 3, "c" => 4);
    ///
    /// // With owned value (more efficient, consumes update)
    /// let result1 = base.join(update);
    /// assert_eq!(result1, object!("a" => 1, "b" => 3, "c" => 4));
    ///
    /// // With borrowed value (less efficient, keeps update)
    /// let update2 = object!("b" => 5, "d" => 6);
    /// let result2 = base.join(&update2);
    /// assert_eq!(result2, object!("a" => 1, "b" => 5, "d" => 6));
    /// assert_eq!(update2, object!("b" => 5, "d" => 6)); // update2 unchanged
    ///
    /// // base is always unchanged
    /// assert_eq!(base, object!("a" => 1, "b" => 2));
    /// ```
    ///
    /// See also: [`Self::merge`] for consuming variant, [`Self::merge_from`] for borrowed variant.
    fn join<'a, O: Into<Cow<'a, Self>>>(&self, other: O) -> Self
    where
        Self: 'a,
    {
        let mut ret = self.clone();
        let other = other.into();
        match other {
            Cow::Borrowed(other) => ret.merge_from(other),
            Cow::Owned(other) => ret.merge(other),
        }
        ret
    }
}

impl MapOps for MAAValue {
    fn as_map(&self) -> Option<&Map<Self>> {
        match self {
            MAAValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn as_mut_map(&mut self) -> Option<&mut Map<Self>> {
        match self {
            MAAValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn into_map(self) -> Outcome<Map<Self>, Self> {
        match self {
            MAAValue::Object(obj) => Outcome::Value(obj),
            _ => Outcome::Original(self),
        }
    }
}

impl MapOps for ResolvedMAAValue {
    fn as_map(&self) -> Option<&Map<Self>> {
        match self {
            ResolvedMAAValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn as_mut_map(&mut self) -> Option<&mut Map<Self>> {
        match self {
            ResolvedMAAValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn into_map(self) -> Outcome<Map<Self>, Self> {
        match self {
            ResolvedMAAValue::Object(obj) => Outcome::Value(obj),
            _ => Outcome::Original(self),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_value_macro::object;

    use super::*;
    use crate::convert::AsPrimitive;

    #[test]
    fn get() {
        use crate::primitive::{Float, Int};

        let value = object!("int" => 1, "string" => "hello");

        // Test get with existing keys
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);
        assert_eq!(value.get("string").unwrap().as_str().unwrap(), "hello");

        // Test get with non-existent key
        assert_eq!(value.get("missing"), None);

        // Test get on non-map value
        assert_eq!(MAAValue::from(1).get("int"), None);

        // Test get_or - returns value when exists
        assert_eq!(value.get_or("int", 2), 1);

        // Test get_or - returns default when missing or wrong type
        assert_eq!(value.get_or::<Float>("int", 2.0), 2.0);
        assert_eq!(value.get_or::<Int>("missing", 999), 999);

        // Test get_mut
        let mut value = object!("int" => 1);
        *value.get_mut("int").unwrap() = 2.into();
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 2);

        // Test get_mut with non-existent key
        assert_eq!(value.get_mut("missing"), None);

        // Test get_mut on non-map value
        assert_eq!(MAAValue::from(1).get_mut("int"), None);

        // Test with ResolvedMAAValue
        let resolved = object!("key" => 100).resolve().unwrap();
        assert_eq!(resolved.get("key").unwrap().as_int().unwrap(), 100);
        assert_eq!(resolved.get_or::<Int>("key", 999), 100);
    }

    #[test]
    fn insert() {
        // Test insert into map
        let mut value = MAAValue::default();
        assert_eq!(value.get("int"), None);
        value.insert("int", 1.into());
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);

        // Test insert overwrites existing value
        value.insert("int", 2.into());
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 2);

        // Test insert on non-map value does nothing
        let mut non_map = MAAValue::from(42);
        non_map.insert("key", 1.into());
        assert_eq!(non_map.as_int(), Some(42)); // Value unchanged
        assert_eq!(non_map.get("key"), None);
    }

    #[test]
    fn maybe_insert() {
        let mut value = MAAValue::default();
        assert_eq!(value.get("int"), None);

        // Test maybe_insert with Some inserts
        value.maybe_insert("int", Some(1.into()));
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1);

        // Test maybe_insert with None does nothing
        value.maybe_insert("float", None::<MAAValue>);
        assert_eq!(value.get("float"), None);

        // Test maybe_insert doesn't remove when None
        value.maybe_insert("int", None::<MAAValue>);
        assert_eq!(value.get("int").unwrap().as_int().unwrap(), 1); // Still there
    }

    #[test]
    fn get_typed() {
        use crate::primitive::{Float, Int};

        let value = object!(
            "int" => 42,
            "float" => 2.14,
            "string" => "hello"
        );

        // Test successful type conversions
        assert_eq!(value.get_typed::<Int>("int"), Some(42));
        assert_eq!(value.get_typed::<Float>("float"), Some(2.14));
        assert_eq!(value.get_typed::<&str>("string"), Some("hello"));

        // Test type mismatch returns None
        assert_eq!(value.get_typed::<Float>("int"), None);
        assert_eq!(value.get_typed::<Int>("string"), None);

        // Test non-existent key returns None
        assert_eq!(value.get_typed::<Int>("nonexistent"), None);

        // Test on non-map value returns None
        let non_map = MAAValue::from(1);
        assert_eq!(non_map.get_typed::<Int>("int"), None);

        // Test with ResolvedMAAValue
        let resolved = value.resolve().unwrap();
        assert_eq!(resolved.get_typed::<Int>("int"), Some(42));
    }

    #[test]
    fn into_map() {
        use crate::Outcome;

        // Test with map - extracts owned map
        let map_value = object!("key1" => 1, "key2" => 2);
        match map_value.into_map() {
            Outcome::Value(map) => {
                assert_eq!(map.len(), 2);
                assert!(map.contains_key("key1"));
                assert!(map.contains_key("key2"));
            }
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }

        // Test with empty map
        let empty = MAAValue::default();
        match empty.into_map() {
            Outcome::Value(map) => {
                assert_eq!(map.len(), 0);
            }
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }

        // Test with non-map value - returns original
        let non_map = MAAValue::from(42);
        match non_map.clone().into_map() {
            Outcome::Value(_) => panic!("Expected Original, got Value"),
            Outcome::Original(val) => {
                assert_eq!(val, non_map);
            }
        }

        // Test with array - returns original
        let array_val = MAAValue::from([1, 2, 3]);
        match array_val.clone().into_map() {
            Outcome::Value(_) => panic!("Expected Original, got Value"),
            Outcome::Original(val) => {
                assert_eq!(val, array_val);
            }
        }

        // Test with ResolvedMAAValue
        let resolved = object!("key" => "value").resolve().unwrap();
        match resolved.into_map() {
            Outcome::Value(map) => {
                assert_eq!(map.len(), 1);
            }
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }
    }

    #[test]
    fn as_map() {
        // Test with object value
        let obj = object!("key1" => "value1", "key2" => "value2");
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
        let mut obj = object!("key" => "value");
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
