#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::ffi::CString;

use maa_str_ext::ToUtf8String;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Interior null byte")]
    InteriorNull(#[from] std::ffi::NulError),
    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] maa_str_ext::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A trait to convert a reference to a UTF-8 encoded C string passed to MAA.
pub trait ToCString {
    /// Convert the value of `self` to a UTF-8 encoded C string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maa_ffi_string::ToCString;
    ///
    /// let c_str = "Hello, world!".to_cstring().unwrap();
    /// assert_eq!(c_str, std::ffi::CString::new("Hello, world!").unwrap());
    ///
    /// let c_str = std::path::Path::new("/tmp").to_cstring().unwrap();
    /// assert_eq!(c_str, std::ffi::CString::new("/tmp").unwrap());
    ///
    /// let c_str = true.to_cstring().unwrap();
    /// assert_eq!(c_str, std::ffi::CString::new("1").unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// If the value contains an interior null byte, an error is returned.
    /// Or if the value is not valid UTF-8, an error is returned.
    fn to_cstring(self) -> Result<CString>;
}

impl ToCString for CString {
    fn to_cstring(self) -> Result<CString> {
        Ok(self)
    }
}

// Implement ToCString for String separately to avoid unnecessary allocation
impl ToCString for String {
    fn to_cstring(self) -> Result<CString> {
        Ok(CString::new(self)?)
    }
}

impl ToCString for &str {
    fn to_cstring(self) -> Result<CString> {
        Ok(CString::new(self)?)
    }
}

impl ToCString for Vec<u8> {
    fn to_cstring(self) -> Result<CString> {
        Ok(CString::new(self.to_utf8_string()?)?)
    }
}

impl ToCString for &[u8] {
    fn to_cstring(self) -> Result<CString> {
        Ok(CString::new(self.to_utf8_string()?)?)
    }
}

impl ToCString for std::ffi::OsString {
    fn to_cstring(self) -> Result<CString> {
        self.to_utf8_string()?.to_cstring()
    }
}

impl ToCString for &std::ffi::OsStr {
    fn to_cstring(self) -> Result<CString> {
        self.to_utf8_string()?.to_cstring()
    }
}

impl ToCString for std::path::PathBuf {
    fn to_cstring(self) -> Result<CString> {
        self.into_os_string().to_cstring()
    }
}

impl ToCString for &std::path::Path {
    fn to_cstring(self) -> Result<CString> {
        self.as_os_str().to_cstring()
    }
}

impl ToCString for bool {
    fn to_cstring(self) -> Result<CString> {
        if self { "1" } else { "0" }.to_cstring()
    }
}

/// Implement `ToCString` by `to_string` method.
///
/// `impl_to_cstring_by_to_string!(t1, t2, ...)` will implement `ToCString` for `t1`, `t2`, ... by
/// `to_string` method.
macro_rules! impl_to_cstring_by_to_string {
    ($($t:ty),*) => {
        $(
            impl $crate::ToCString for $t {
                fn to_cstring(self) -> Result<std::ffi::CString> {
                    self.to_string().to_cstring()
                }
            }
        )*
    };
}

impl_to_cstring_by_to_string!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        ffi::{CString, OsStr, OsString},
        path::{Path, PathBuf},
    };

    use super::*;

    macro_rules! compare_cstring {
        ($value:expr, $expected:expr) => {
            assert_eq!($value.to_cstring().unwrap().as_c_str(), $expected);
        };
    }

    mod cstring_impl {
        use super::*;

        #[test]
        fn converts_to_itself() {
            let original = CString::new("test").unwrap();
            let result = original.clone().to_cstring().unwrap();
            assert_eq!(result, original);
        }
    }

    mod string_impl {
        use super::*;

        #[test]
        fn converts_string() {
            compare_cstring!(String::from("hello"), c"hello");
        }

        #[test]
        fn interior_null_fails() {
            let result = String::from("hello\0world").to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InteriorNull(_)));
        }
    }

    mod str_impl {
        use super::*;

        #[test]
        fn converts_str() {
            compare_cstring!("test", c"test");
        }

        #[test]
        fn interior_null_fails() {
            let result = "hello\0world".to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InteriorNull(_)));
        }
    }

    mod vec_u8_impl {
        use super::*;

        #[test]
        fn converts_valid_utf8() {
            let vec = b"hello".to_vec();
            compare_cstring!(vec, c"hello");
        }

        #[test]
        fn interior_null_fails() {
            let vec = b"hello\0world".to_vec();
            let result = vec.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InteriorNull(_)));
        }

        #[test]
        fn invalid_utf8_fails() {
            let vec = vec![0xFF, 0xFE, 0xFD];
            let result = vec.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InvalidUtf8(_)));
        }

        #[test]
        fn incomplete_utf8_fails() {
            let vec = vec![0xE0, 0x80]; // Incomplete multi-byte sequence
            let result = vec.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InvalidUtf8(_)));
        }

        #[test]
        fn overlong_encoding_fails() {
            let vec = vec![0xC0, 0x80]; // Overlong encoding
            let result = vec.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InvalidUtf8(_)));
        }
    }

    mod slice_u8_impl {
        use super::*;

        #[test]
        fn converts_valid_utf8() {
            let slice: &[u8] = b"hello";
            compare_cstring!(slice, c"hello");
        }

        #[test]
        fn interior_null_fails() {
            let slice: &[u8] = b"hello\0world";
            let result = slice.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InteriorNull(_)));
        }

        #[test]
        fn invalid_utf8_fails() {
            let slice: &[u8] = &[0xFF, 0xFE, 0xFD];
            let result = slice.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InvalidUtf8(_)));
        }
    }

    mod osstring_impl {
        use super::*;

        #[test]
        fn converts_osstring() {
            compare_cstring!(OsString::from("path"), c"path");
        }

        #[cfg(unix)]
        #[test]
        fn invalid_utf8_fails() {
            use std::os::unix::ffi::OsStringExt;
            let invalid = OsString::from_vec(vec![0xFF, 0xFE, 0xFD]);
            let result = invalid.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InvalidUtf8(_)));
        }
    }

    mod osstr_impl {
        use super::*;

        #[test]
        fn converts_osstr() {
            compare_cstring!(OsStr::new("directory"), c"directory");
        }

        #[cfg(unix)]
        #[test]
        fn invalid_utf8_fails() {
            use std::os::unix::ffi::OsStrExt;
            let invalid = OsStr::from_bytes(&[0xFF, 0xFE, 0xFD]);
            let result = invalid.to_cstring();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::InvalidUtf8(_)));
        }
    }

    mod path_impl {
        use super::*;

        #[test]
        fn converts_pathbuf() {
            compare_cstring!(PathBuf::from("/usr/local"), c"/usr/local");
        }

        #[test]
        fn converts_path() {
            compare_cstring!(Path::new("/etc/config"), c"/etc/config");
        }
    }

    mod bool_impl {
        use super::*;

        #[test]
        fn true_converts_to_1() {
            compare_cstring!(true, c"1");
        }

        #[test]
        fn false_converts_to_0() {
            compare_cstring!(false, c"0");
        }
    }

    mod integer_impl {
        use super::*;

        #[test]
        fn converts_integers() {
            compare_cstring!(42i32, c"42");
            compare_cstring!(-42i32, c"-42");
            compare_cstring!(0, c"0");

            compare_cstring!(i8::MAX, c"127");
            compare_cstring!(i8::MIN, c"-128");
            compare_cstring!(u8::MAX, c"255");
            compare_cstring!(i64::MAX, c"9223372036854775807");
            compare_cstring!(u64::MAX, c"18446744073709551615");
        }
    }

    mod error_handling {
        use super::*;

        #[test]
        fn nul_error_display() {
            let err = String::from("test\0").to_cstring().unwrap_err();
            assert!(err.to_string().contains("Interior null byte"));
        }

        #[test]
        #[cfg(unix)]
        fn invalid_utf8_error_display() {
            use std::os::unix::ffi::OsStringExt;
            let invalid = OsString::from_vec(vec![0xFF]);
            let err = invalid.to_cstring().unwrap_err();
            assert!(err.to_string().contains("Invalid UTF-8"));
        }
    }
}
