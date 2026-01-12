#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

#[derive(Debug)]
pub struct Error(Option<std::str::Utf8Error>);

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(e) => write!(f, "{e}"),
            None => write!(f, "Invalid UTF-8"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A trait to convert various string-like types to UTF-8 String.
///
/// This trait provides a unified interface for converting from:
///
/// - OsStr/OsString
/// - Path/PathBuf
/// - Vec<u8>/&[u8]
/// - CString
/// - String/&str (infallible)
pub trait ToUtf8String {
    /// Convert the value of `self` to a UTF-8 encoded String.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maa_str_ext::ToUtf8String;
    ///
    /// // From Path
    /// let string = std::path::Path::new("/tmp").to_utf8_string().unwrap();
    /// assert_eq!(string, "/tmp");
    ///
    /// // From OsStr
    /// let string = std::ffi::OsStr::new("hello").to_utf8_string().unwrap();
    /// assert_eq!(string, "hello");
    ///
    /// // From Vec<u8>
    /// let string = b"world".to_vec().to_utf8_string().unwrap();
    /// assert_eq!(string, "world");
    ///
    /// // From &str (infallible)
    /// let string = "test".to_utf8_string().unwrap();
    /// assert_eq!(string, "test");
    /// ```
    ///
    /// # Errors
    ///
    /// If the value is not valid UTF-8, an error is returned.
    fn to_utf8_string(self) -> Result<String>;
}

// Infallible conversions from String and &str
impl ToUtf8String for String {
    fn to_utf8_string(self) -> Result<String> {
        Ok(self)
    }
}

impl ToUtf8String for &str {
    fn to_utf8_string(self) -> Result<String> {
        Ok(self.to_owned())
    }
}

/// Conversions from Vec and slice of bytes
///
/// The input is assumed to be valid UTF-8, but if it is not, an error is returned.
mod bytes {
    use super::*;

    impl ToUtf8String for Vec<u8> {
        fn to_utf8_string(self) -> Result<String> {
            String::from_utf8(self).map_err(|e| Error(Some(e.utf8_error())))
        }
    }

    impl ToUtf8String for &[u8] {
        fn to_utf8_string(self) -> Result<String> {
            std::str::from_utf8(self)
                .map(|s| s.to_owned())
                .map_err(|e| Error(Some(e)))
        }
    }
}

/// Conversions from OsString and &OsStr on Unix platforms
///
/// Here we use the `into_vec` and `as_bytes` methods to get the inner bytes, and then
/// convert as bytes.
#[cfg(unix)]
mod os_str_unix {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    use super::*;

    impl ToUtf8String for std::ffi::OsString {
        fn to_utf8_string(self) -> Result<String> {
            self.into_vec().to_utf8_string()
        }
    }

    impl ToUtf8String for &std::ffi::OsStr {
        fn to_utf8_string(self) -> Result<String> {
            self.as_bytes().to_utf8_string()
        }
    }
}

/// Conversions from OsString and &OsStr on non-Unix platforms
///
/// On non-Unix platforms, the `as_bytes` / `into_vec` method is not available.
/// Therefore, we directly use the `to_str` method, which provides less detailed error information.
#[cfg(not(unix))]
mod os_str_non_unix {
    use super::*;

    impl ToUtf8String for std::ffi::OsString {
        fn to_utf8_string(self) -> Result<String> {
            Ok(self.to_str().ok_or(Error(None))?.to_string())
        }
    }

    #[cfg(not(unix))]
    impl ToUtf8String for &std::ffi::OsStr {
        fn to_utf8_string(self) -> Result<String> {
            Ok(self.to_str().ok_or(Error(None))?.to_string())
        }
    }
}

impl ToUtf8String for std::path::PathBuf {
    fn to_utf8_string(self) -> Result<String> {
        self.into_os_string().to_utf8_string()
    }
}

impl ToUtf8String for &std::path::Path {
    fn to_utf8_string(self) -> Result<String> {
        self.as_os_str().to_utf8_string()
    }
}

// Conversion from CString
impl ToUtf8String for std::ffi::CString {
    fn to_utf8_string(self) -> Result<String> {
        self.into_string().map_err(|e| Error(Some(e.utf8_error())))
    }
}

impl ToUtf8String for &std::ffi::CStr {
    fn to_utf8_string(self) -> Result<String> {
        self.to_str()
            .map(|s| s.to_string())
            .map_err(|e| Error(Some(e)))
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        ffi::{OsStr, OsString},
        path::{Path, PathBuf},
    };

    use super::*;

    mod osstring_impl {
        use super::*;

        #[test]
        fn converts_osstring() {
            let os_string = OsString::from("path");
            assert_eq!(os_string.to_utf8_string().unwrap(), "path");
        }

        #[cfg(unix)]
        #[test]
        fn invalid_utf8_fails() {
            use std::os::unix::ffi::OsStringExt;
            let invalid = OsString::from_vec(vec![0xFF, 0xFE, 0xFD]);
            let result = invalid.to_utf8_string();
            assert!(result.is_err());
        }
    }

    mod osstr_impl {
        use super::*;

        #[test]
        fn converts_osstr() {
            let os_str = OsStr::new("directory");
            assert_eq!(os_str.to_utf8_string().unwrap(), "directory");
        }

        #[cfg(unix)]
        #[test]
        fn invalid_utf8_fails() {
            use std::os::unix::ffi::OsStrExt;
            let invalid = OsStr::from_bytes(&[0xFF, 0xFE, 0xFD]);
            let result = invalid.to_utf8_string();
            assert!(result.is_err());
        }
    }

    mod path_impl {
        use super::*;

        #[test]
        fn converts_pathbuf() {
            let path = PathBuf::from("/usr/local");
            assert_eq!(path.to_utf8_string().unwrap(), "/usr/local");
        }

        #[test]
        fn converts_path() {
            let path = Path::new("/etc/config");
            assert_eq!(path.to_utf8_string().unwrap(), "/etc/config");
        }
    }

    mod string_impl {
        use super::*;

        #[test]
        fn converts_string() {
            let s = String::from("hello");
            assert_eq!(s.to_utf8_string().unwrap(), "hello");
        }

        #[test]
        fn converts_str() {
            let s = "world";
            assert_eq!(s.to_utf8_string().unwrap(), "world");
        }
    }

    mod vec_u8_impl {
        use super::*;

        #[test]
        fn converts_valid_utf8() {
            let vec = b"hello".to_vec();
            assert_eq!(vec.to_utf8_string().unwrap(), "hello");
        }

        #[test]
        fn invalid_utf8_fails() {
            let vec = vec![0xFF, 0xFE, 0xFD];
            let result = vec.to_utf8_string();
            assert!(result.is_err());
        }
    }

    mod slice_u8_impl {
        use super::*;

        #[test]
        fn converts_valid_utf8() {
            let slice: &[u8] = b"hello";
            assert_eq!(slice.to_utf8_string().unwrap(), "hello");
        }

        #[test]
        fn invalid_utf8_fails() {
            let slice: &[u8] = &[0xFF, 0xFE, 0xFD];
            let result = slice.to_utf8_string();
            assert!(result.is_err());
        }
    }

    mod cstring_impl {
        use std::ffi::CString;

        use super::*;

        #[test]
        fn converts_cstring() {
            let c_string = CString::new("test").unwrap();
            assert_eq!(c_string.to_utf8_string().unwrap(), "test");
        }

        #[test]
        fn converts_cstr() {
            let c_str = c"hello";
            assert_eq!(c_str.to_utf8_string().unwrap(), "hello");
        }

        #[test]
        fn invalid_utf8_fails() {
            let invalid = c"\xFF\xFE\xFD";
            let result = invalid.to_utf8_string();
            assert!(result.is_err());
        }
    }

    mod error_handling {
        use super::*;

        #[test]
        #[cfg(unix)]
        fn invalid_utf8_error_display_unix() {
            use std::os::unix::ffi::OsStringExt;
            let invalid = OsString::from_vec(vec![0xFF]);
            let err = invalid.to_utf8_string().unwrap_err();
            assert_eq!(err.to_string(), err.0.unwrap().to_string());
        }

        #[test]
        fn invalid_utf8_error_display_non_info() {
            let err = Error(None);
            assert!(err.to_string().contains("Invalid UTF-8"));
        }
    }
}
