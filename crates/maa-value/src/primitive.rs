use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    value::{MAAValue, ResolvedMAAValue},
};

pub type Int = i32;
pub type Float = f32;
pub type String = std::string::String;

/// Primitive value type
///
/// Represents the basic data types used in configuration and task parameters.
/// Supports four types: boolean, integer, float, and string.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum MAAPrimitive {
    Bool(bool),
    Int(Int),
    Float(Float),
    String(String),
}

impl From<bool> for MAAPrimitive {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<Int> for MAAPrimitive {
    fn from(v: Int) -> Self {
        Self::Int(v)
    }
}

impl From<Float> for MAAPrimitive {
    fn from(v: Float) -> Self {
        Self::Float(v)
    }
}

impl From<String> for MAAPrimitive {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for MAAPrimitive {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl TryFrom<&OsStr> for MAAPrimitive {
    type Error = Error;

    fn try_from(v: &OsStr) -> Result<Self, Self::Error> {
        use maa_str_ext::ToUtf8String;
        Ok(Self::String(v.to_utf8_string()?))
    }
}

impl TryFrom<OsString> for MAAPrimitive {
    type Error = Error;

    fn try_from(v: OsString) -> Result<Self, Self::Error> {
        use maa_str_ext::ToUtf8String;
        Ok(Self::String(v.to_utf8_string()?))
    }
}

impl TryFrom<&Path> for MAAPrimitive {
    type Error = Error;

    fn try_from(v: &Path) -> Result<Self, Self::Error> {
        use maa_str_ext::ToUtf8String;
        Ok(Self::String(v.to_utf8_string()?))
    }
}

impl TryFrom<PathBuf> for MAAPrimitive {
    type Error = Error;

    fn try_from(v: PathBuf) -> Result<Self, Self::Error> {
        use maa_str_ext::ToUtf8String;
        Ok(Self::String(v.to_utf8_string()?))
    }
}

impl From<MAAPrimitive> for MAAValue {
    fn from(v: MAAPrimitive) -> Self {
        Self::Primitive(v)
    }
}

impl PartialEq<MAAPrimitive> for MAAValue {
    fn eq(&self, other: &MAAPrimitive) -> bool {
        match self {
            Self::Primitive(v) => v == other,
            _ => false,
        }
    }
}
impl From<MAAPrimitive> for ResolvedMAAValue {
    fn from(v: MAAPrimitive) -> Self {
        Self::Primitive(v)
    }
}

impl PartialEq<MAAPrimitive> for ResolvedMAAValue {
    fn eq(&self, other: &MAAPrimitive) -> bool {
        match self {
            Self::Primitive(v) => v == other,
            _ => false,
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::convert::AsPrimitive;

    #[test]
    fn deserialize() {
        use serde_test::{Token, assert_de_tokens};

        let values = vec![
            MAAPrimitive::Bool(true),
            MAAPrimitive::Int(1),
            MAAPrimitive::Float(1.0),
            MAAPrimitive::String("".to_string()),
        ];

        assert_de_tokens(&values, &[
            Token::Seq { len: Some(4) },
            Token::Bool(true),
            Token::I32(1),
            Token::F32(1.0),
            Token::Str(""),
            Token::SeqEnd,
        ]);
    }

    #[test]
    fn as_type() {
        assert_eq!(MAAPrimitive::Bool(true).as_bool(), Some(true));
        assert_eq!(MAAPrimitive::Bool(true).as_int(), None);
        assert_eq!(MAAPrimitive::Bool(true).as_float(), None);
        assert_eq!(MAAPrimitive::Bool(true).as_str(), None);

        assert_eq!(MAAPrimitive::Int(1).as_bool(), None);
        assert_eq!(MAAPrimitive::Int(1).as_int(), Some(1));
        assert_eq!(MAAPrimitive::Int(1).as_float(), None);
        assert_eq!(MAAPrimitive::Int(1).as_str(), None);

        assert_eq!(MAAPrimitive::Float(1.0).as_bool(), None);
        assert_eq!(MAAPrimitive::Float(1.0).as_int(), None);
        assert_eq!(MAAPrimitive::Float(1.0).as_float(), Some(1.0));
        assert_eq!(MAAPrimitive::Float(1.0).as_str(), None);

        assert_eq!(MAAPrimitive::String("".to_string()).as_bool(), None);
        assert_eq!(MAAPrimitive::String("".to_string()).as_int(), None);
        assert_eq!(MAAPrimitive::String("".to_string()).as_float(), None);
        assert_eq!(MAAPrimitive::String("".to_string()).as_str(), Some(""));
    }

    #[test]
    fn serialize() {
        use serde_test::{Token, assert_ser_tokens};

        assert_ser_tokens(&MAAPrimitive::Bool(true), &[Token::Bool(true)]);
        assert_ser_tokens(&MAAPrimitive::Bool(false), &[Token::Bool(false)]);
        assert_ser_tokens(&MAAPrimitive::Int(42), &[Token::I32(42)]);
        assert_ser_tokens(&MAAPrimitive::Int(-10), &[Token::I32(-10)]);
        assert_ser_tokens(&MAAPrimitive::Float(1.5), &[Token::F32(1.5)]);
        assert_ser_tokens(&MAAPrimitive::Float(-2.5), &[Token::F32(-2.5)]);
        assert_ser_tokens(&MAAPrimitive::String("hello".to_string()), &[Token::Str(
            "hello",
        )]);
        assert_ser_tokens(&MAAPrimitive::String("".to_string()), &[Token::Str("")]);
    }

    #[test]
    fn from_primitives() {
        // Test From implementations for each primitive type
        let primitive: MAAPrimitive = true.into();
        assert_eq!(primitive, MAAPrimitive::Bool(true));

        let primitive: MAAPrimitive = 42.into();
        assert_eq!(primitive, MAAPrimitive::Int(42));

        let primitive: MAAPrimitive = 1.5f32.into();
        assert_eq!(primitive, MAAPrimitive::Float(1.5));

        let primitive: MAAPrimitive = "hello".to_string().into();
        assert_eq!(primitive, MAAPrimitive::String("hello".to_string()));

        let primitive: MAAPrimitive = "world".into();
        assert_eq!(primitive, MAAPrimitive::String("world".to_string()));
    }

    #[test]
    fn to_maa_value() {
        // Test conversion from primitives to MAAValue (via MAAPrimitive)
        let value: MAAValue = true.into();
        assert_eq!(value.as_bool(), Some(true));

        let value: MAAValue = 42.into();
        assert_eq!(value.as_int(), Some(42));

        let value: MAAValue = 1.5f32.into();
        assert_eq!(value.as_float(), Some(1.5));

        let value: MAAValue = "test".to_string().into();
        assert_eq!(value.as_str(), Some("test"));

        let value: MAAValue = "str".into();
        assert_eq!(value.as_str(), Some("str"));
    }

    #[test]
    fn maa_value_eq_primitive() {
        // Test PartialEq between MAAValue and MAAPrimitive
        assert_eq!(MAAValue::from(true), MAAPrimitive::Bool(true));
        assert_ne!(MAAValue::from(true), MAAPrimitive::Bool(false));
        assert_ne!(MAAValue::from(true), MAAPrimitive::Int(1));

        assert_eq!(MAAValue::from(42), MAAPrimitive::Int(42));
        assert_ne!(MAAValue::from(42), MAAPrimitive::Int(43));
        assert_ne!(MAAValue::from(42), MAAPrimitive::Bool(true));

        assert_eq!(MAAValue::from(1.5f32), MAAPrimitive::Float(1.5));
        assert_ne!(MAAValue::from(1.5f32), MAAPrimitive::Float(2.5));

        assert_eq!(
            MAAValue::from("test"),
            MAAPrimitive::String("test".to_string())
        );
        assert_ne!(
            MAAValue::from("test"),
            MAAPrimitive::String("other".to_string())
        );

        // Test that non-Primitive values don't equal Primitive
        let array = MAAValue::Array(vec![1.into()]);
        assert_ne!(array, MAAPrimitive::Int(1));

        let object = MAAValue::default();
        assert_ne!(object, MAAPrimitive::Bool(true));
    }

    #[test]
    fn edge_cases() {
        // Test edge case values
        assert_eq!(MAAPrimitive::Int(i32::MAX).as_int(), Some(i32::MAX));
        assert_eq!(MAAPrimitive::Int(i32::MIN).as_int(), Some(i32::MIN));
        assert_eq!(
            MAAPrimitive::Float(f32::INFINITY).as_float(),
            Some(f32::INFINITY)
        );
        assert_eq!(
            MAAPrimitive::Float(f32::NEG_INFINITY).as_float(),
            Some(f32::NEG_INFINITY)
        );
        assert!(MAAPrimitive::Float(f32::NAN).as_float().unwrap().is_nan());

        // Test very long strings
        let long_string = "a".repeat(10000);
        let primitive: MAAPrimitive = long_string.as_str().into();
        assert_eq!(primitive.as_str(), Some(long_string.as_str()));
    }

    #[test]
    fn try_from_os_str() {
        use OsStr;

        // Test valid UTF-8 OsStr
        let os_str = OsStr::new("hello");
        let primitive = MAAPrimitive::try_from(os_str).unwrap();
        assert_eq!(primitive, MAAPrimitive::String("hello".to_string()));

        // Test with path-like strings
        let os_str = OsStr::new("/usr/local/bin");
        let primitive = MAAPrimitive::try_from(os_str).unwrap();
        assert_eq!(
            primitive,
            MAAPrimitive::String("/usr/local/bin".to_string())
        );

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            // Test invalid UTF-8 OsStr
            let invalid = OsStr::from_bytes(&[0xFF, 0xFE, 0xFD]);
            let result = MAAPrimitive::try_from(invalid);
            assert!(result.is_err());
        }
    }

    #[test]
    fn try_from_os_string() {
        use OsString;

        // Test valid UTF-8 OsString
        let os_string = OsString::from("world");
        let primitive = MAAPrimitive::try_from(os_string).unwrap();
        assert_eq!(primitive, MAAPrimitive::String("world".to_string()));

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            // Test invalid UTF-8 OsString
            let invalid = OsString::from_vec(vec![0xFF, 0xFE, 0xFD]);
            let result = MAAPrimitive::try_from(invalid);
            assert!(result.is_err());
        }
    }

    #[test]
    fn try_from_path() {
        // Test valid UTF-8 Path
        let path = Path::new("/etc/config");
        let primitive = MAAPrimitive::try_from(path).unwrap();
        assert_eq!(primitive, MAAPrimitive::String("/etc/config".to_string()));

        // Test relative path
        let path = Path::new("./relative/path");
        let primitive = MAAPrimitive::try_from(path).unwrap();
        assert_eq!(
            primitive,
            MAAPrimitive::String("./relative/path".to_string())
        );
    }

    #[test]
    fn try_from_pathbuf() {
        // Test valid UTF-8 PathBuf
        let pathbuf = PathBuf::from("/usr/local");
        let primitive = MAAPrimitive::try_from(pathbuf).unwrap();
        assert_eq!(primitive, MAAPrimitive::String("/usr/local".to_string()));

        // Test with multiple components
        let mut pathbuf = PathBuf::from("/home");
        pathbuf.push("user");
        pathbuf.push("documents");
        let primitive = MAAPrimitive::try_from(pathbuf.clone()).unwrap();
        assert_eq!(primitive.as_str(), Some(pathbuf.to_str().unwrap()));
    }
}
