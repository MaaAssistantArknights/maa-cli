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

impl_to_cstring_by_to_string!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        ffi::{OsStr, OsString},
        path::{Path, PathBuf},
    };

    use super::*;

    #[test]
    fn to_cstring() {
        macro_rules! compare_cstring {
            ($value:expr, $expected:expr) => {
                assert_eq!($value.to_cstring().unwrap().as_c_str(), $expected);
            };
        }

        compare_cstring!(CString::new("foo").unwrap(), c"foo");

        compare_cstring!("foo", c"foo");
        compare_cstring!(String::from("foo"), c"foo");

        compare_cstring!(OsStr::new("/tmp"), c"/tmp");
        compare_cstring!(OsString::from("/tmp"), c"/tmp");

        compare_cstring!(Path::new("/tmp"), c"/tmp");
        compare_cstring!(PathBuf::from("/tmp"), c"/tmp");

        compare_cstring!(true, c"1");
        compare_cstring!(false, c"0");

        compare_cstring!(1, c"1");
        compare_cstring!(1i8, c"1");

        compare_cstring!(maa_types::TouchMode::MaaTouch, c"maatouch");
        compare_cstring!(maa_types::TaskType::StartUp, c"StartUp");
    }
}
