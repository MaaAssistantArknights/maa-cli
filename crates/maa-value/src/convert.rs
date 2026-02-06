use crate::{
    primitive::{Float, Int, MAAPrimitive},
    value::{MAAValue, ResolvedMAAValue},
};

pub trait TryAs<'a, V> {
    fn try_as(&'a self) -> Option<V>;
}

pub trait AsPrimitive {
    /// Get the value if the value is primitive
    ///
    /// A primitive value can be a bool, int, float or string.
    /// It can not be an array, object or others.
    fn as_primitive(&self) -> Option<&MAAPrimitive>;

    fn as_bool(&self) -> Option<bool> {
        match self.as_primitive()? {
            MAAPrimitive::Bool(v) => Some(*v),
            _ => None,
        }
    }

    fn as_int(&self) -> Option<Int> {
        match self.as_primitive()? {
            MAAPrimitive::Int(v) => Some(*v),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<Float> {
        match self.as_primitive()? {
            MAAPrimitive::Float(v) => Some(*v),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self.as_primitive()? {
            MAAPrimitive::String(v) => Some(v),
            _ => None,
        }
    }
}

impl AsPrimitive for MAAPrimitive {
    fn as_primitive(&self) -> Option<&MAAPrimitive> {
        Some(self)
    }
}

impl AsPrimitive for MAAValue {
    fn as_primitive(&self) -> Option<&MAAPrimitive> {
        match self {
            MAAValue::Primitive(v) => Some(v),
            _ => None,
        }
    }
}

impl AsPrimitive for ResolvedMAAValue {
    fn as_primitive(&self) -> Option<&MAAPrimitive> {
        match self {
            Self::Primitive(v) => Some(v),
            _ => None,
        }
    }
}

impl<T: AsPrimitive> TryAs<'_, bool> for T {
    fn try_as(&self) -> Option<bool> {
        self.as_bool()
    }
}

impl<T: AsPrimitive> TryAs<'_, Int> for T {
    fn try_as(&self) -> Option<Int> {
        self.as_int()
    }
}

impl<T: AsPrimitive> TryAs<'_, Float> for T {
    fn try_as(&self) -> Option<Float> {
        self.as_float()
    }
}

impl<'a, T: AsPrimitive> TryAs<'a, &'a str> for T {
    fn try_as(&'a self) -> Option<&'a str> {
        self.as_str()
    }
}

macro_rules! impl_from_by_from_primitive {
    ($t:path, $($p:ty),*) => {
        $(
            impl From<$p> for $t {
                fn from(v: $p) -> Self {
                    let primitive = $crate::primitive::MAAPrimitive::from(v);
                    Self::from(primitive)
                }
            }
        )*
    };
}

macro_rules! impl_try_from_by_from_primitive {
    ($t:path, $($p:ty),*) => {
        $(
            impl TryFrom<$p> for $t {
                type Error = $crate::error::Error;

                fn try_from(v: $p) -> Result<Self, Self::Error> {
                    let primitive = $crate::primitive::MAAPrimitive::try_from(v)?;
                    Ok(Self::from(primitive))
                }
            }
        )*
    };
}

macro_rules! impl_all_by_from_primitive {
    ($t:path) => {
        impl_from_by_from_primitive!(
            $t,
            bool,
            $crate::primitive::Int,
            $crate::primitive::Float,
            &str,
            String
        );
        impl_try_from_by_from_primitive!(
            $t,
            &std::path::Path,
            std::path::PathBuf,
            &std::ffi::OsStr,
            std::ffi::OsString
        );
    };
}

impl_all_by_from_primitive!(crate::value::MAAValue);
impl_all_by_from_primitive!(crate::value::ResolvedMAAValue);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::userinput::{BoolInput, Input};

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
    fn try_as() {
        use crate::primitive::{Float, Int};

        // Bool
        let bool_val = MAAValue::from(true);
        assert_eq!(TryAs::<bool>::try_as(&bool_val), Some(true));
        assert_eq!(TryAs::<Int>::try_as(&bool_val), None);
        let bool_input_val = MAAValue::from(BoolInput::new(Some(true)));
        assert_eq!(TryAs::<bool>::try_as(&bool_input_val), None);

        // Int
        let int_val = MAAValue::from(1);
        assert_eq!(TryAs::<Int>::try_as(&int_val), Some(1));
        assert_eq!(TryAs::<Float>::try_as(&int_val), None);
        let int_input_val = MAAValue::from(Input::new(Some(1)));
        assert_eq!(TryAs::<Int>::try_as(&int_input_val), None);

        // Float
        let float_val = MAAValue::from(1.0);
        assert_eq!(TryAs::<Float>::try_as(&float_val), Some(1.0));
        assert_eq!(TryAs::<Int>::try_as(&float_val), None);
        let float_input_val = MAAValue::from(Input::new(Some(1.0)));
        assert_eq!(TryAs::<Float>::try_as(&float_input_val), None);

        // String
        let str_val = MAAValue::from("string");
        assert_eq!(TryAs::<&str>::try_as(&str_val), Some("string"));
        assert_eq!(TryAs::<bool>::try_as(&str_val), None);
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
}
