use crate::{
    Outcome,
    value::{MAAValue, ResolvedMAAValue},
};

pub trait ArrayOps: Sized {
    fn as_slice(&self) -> Option<&[Self]>;

    fn as_mut_vec(&mut self) -> Option<&mut Vec<Self>>;

    fn into_vec(self) -> Outcome<Vec<Self>, Self>;
}

impl ArrayOps for MAAValue {
    fn as_slice(&self) -> Option<&[Self]> {
        match self {
            MAAValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn as_mut_vec(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            MAAValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn into_vec(self) -> Outcome<Vec<Self>, Self> {
        match self {
            MAAValue::Array(arr) => Outcome::Value(arr),
            _ => Outcome::Original(self),
        }
    }
}

impl ArrayOps for ResolvedMAAValue {
    fn as_slice(&self) -> Option<&[Self]> {
        match self {
            ResolvedMAAValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn as_mut_vec(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            ResolvedMAAValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn into_vec(self) -> Outcome<Vec<Self>, Self> {
        match self {
            ResolvedMAAValue::Array(arr) => Outcome::Value(arr),
            _ => Outcome::Original(self),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::convert::AsPrimitive;

    #[test]
    fn into_vec() {
        use crate::Outcome;
        use maa_value_macro::object;

        // Test with array - extracts owned Vec
        let array_value = MAAValue::from([1, 2, 3]);
        match array_value.into_vec() {
            Outcome::Value(vec) => {
                assert_eq!(vec.len(), 3);
                assert_eq!(vec[0].as_int(), Some(1));
                assert_eq!(vec[2].as_int(), Some(3));
            }
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }

        // Test with empty array
        let empty_array: [i32; 0] = [];
        let empty_value = MAAValue::from(empty_array);
        match empty_value.into_vec() {
            Outcome::Value(vec) => {
                assert_eq!(vec.len(), 0);
            }
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }

        // Test with non-array value - returns original
        let non_array = MAAValue::from(42);
        match non_array.clone().into_vec() {
            Outcome::Value(_) => panic!("Expected Original, got Value"),
            Outcome::Original(val) => {
                assert_eq!(val, non_array);
            }
        }

        // Test with object - returns original
        let obj_val = object!("key" => "value");
        match obj_val.clone().into_vec() {
            Outcome::Value(_) => panic!("Expected Original, got Value"),
            Outcome::Original(val) => {
                assert_eq!(val, obj_val);
            }
        }

        // Test with ResolvedMAAValue
        let resolved_array = MAAValue::from([1, 2, 3]).resolve().unwrap();
        match resolved_array.into_vec() {
            Outcome::Value(vec) => {
                assert_eq!(vec.len(), 3);
            }
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }
    }

    #[test]
    fn as_slice() {
        use maa_value_macro::object;

        // Test with array - returns correct slice
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
        assert_eq!(object!("key" => "value").as_slice(), None);

        // Test with ResolvedMAAValue
        let resolved = MAAValue::from([1, 2, 3]).resolve().unwrap();
        assert!(resolved.as_slice().is_some());
    }

    #[test]
    fn as_mut_vec() {
        use maa_value_macro::object;

        // Test with array - returns mutable reference
        let mut array_value = MAAValue::from([1, 2, 3]);
        let vec = array_value.as_mut_vec().unwrap();
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0].as_int(), Some(1));

        // Test modifications persist through the reference
        vec[0] = 10.into();
        vec.push(4.into());
        drop(vec); // Drop the reference

        // Verify modifications persisted
        let slice = array_value.as_slice().unwrap();
        assert_eq!(slice.len(), 4);
        assert_eq!(slice[0].as_int(), Some(10));
        assert_eq!(slice[3].as_int(), Some(4));

        // Test with empty array
        let empty_array: [i32; 0] = [];
        let mut empty_value = MAAValue::from(empty_array);
        let vec = empty_value.as_mut_vec().unwrap();
        assert_eq!(vec.len(), 0);
        vec.push(1.into());
        drop(vec);
        assert_eq!(empty_value.as_slice().unwrap().len(), 1);

        // Test with non-array values (should return None)
        assert_eq!(MAAValue::from(1).as_mut_vec(), None);
        assert_eq!(MAAValue::from(true).as_mut_vec(), None);
        assert_eq!(MAAValue::from("string").as_mut_vec(), None);
        assert_eq!(MAAValue::default().as_mut_vec(), None);
        assert_eq!(object!("key" => "value").as_mut_vec(), None);

        // Test with ResolvedMAAValue
        let mut resolved = MAAValue::from([1, 2, 3]).resolve().unwrap();
        assert!(resolved.as_mut_vec().is_some());
    }
}
