use std::ffi::CString;

use crate::Result;

/// A trait to convert a reference to a UTF-8 encoded C string passed to MAA.
pub trait ToCString {
    /// Convert the value of `self` to a UTF-8 encoded C string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maa_sys::ToCString;
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

impl ToCString for &str {
    fn to_cstring(self) -> Result<CString> {
        Ok(CString::new(self)?)
    }
}

#[cfg(unix)]
impl ToCString for &std::ffi::OsStr {
    fn to_cstring(self) -> Result<CString> {
        use std::os::unix::ffi::OsStrExt;
        std::str::from_utf8(self.as_bytes())?.to_cstring()
    }
}

#[cfg(not(unix))]
impl ToCString for &std::ffi::OsStr {
    fn to_cstring(self) -> Result<CString> {
        // OsStr on non-Unix platforms can not use `as_bytes` method. So, we use the `to_str`
        // method directly, which lacks the detailed error information.
        self.to_str()
            .ok_or(crate::Error::InvalidUtf8NoInfo)?
            .to_cstring()
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

impl_to_cstring_by_to_string!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl ToCString for maa_types::TouchMode {
    fn to_cstring(self) -> Result<CString> {
        self.to_str().to_cstring()
    }
}

impl ToCString for maa_types::TaskType {
    fn to_cstring(self) -> Result<std::ffi::CString> {
        self.to_str().to_cstring()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::{OsStr, OsString},
        path::{Path, PathBuf},
    };

    use super::*;

    #[test]
    fn to_cstring() {
        assert_eq!("foo".to_cstring().unwrap(), CString::new("foo").unwrap());
        assert_eq!(
            (&String::from("foo")).to_cstring().unwrap(),
            CString::new("foo").unwrap()
        );

        assert_eq!(
            OsStr::new("/tmp").to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );
        assert_eq!(
            (&OsString::from("/tmp")).to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );

        assert_eq!(
            Path::new("/tmp").to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );
        assert_eq!(
            (&PathBuf::from("/tmp")).to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );

        assert_eq!(true.to_cstring().unwrap(), CString::new("1").unwrap());
        assert_eq!(false.to_cstring().unwrap(), CString::new("0").unwrap());

        assert_eq!(1.to_cstring().unwrap(), CString::new("1").unwrap());
        assert_eq!(1i8.to_cstring().unwrap(), CString::new("1").unwrap());

        assert_eq!(
            maa_types::TouchMode::MaaTouch.to_cstring().unwrap(),
            CString::new("maatouch").unwrap()
        );

        assert_eq!(
            maa_types::TaskType::StartUp.to_cstring().unwrap(),
            CString::new("StartUp").unwrap()
        );
    }
}
