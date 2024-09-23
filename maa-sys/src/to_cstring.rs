use std::{
    ffi::CString,
    path::{Path, PathBuf},
};

use crate::{Error, Result};

/// A trait to convert a value to a UTF-8 encoded C string passed to MAA.
pub trait ToCString {
    /// Convert the value of `self` to a UTF-8 encoded C string.
    ///
    /// # Examples
    /// ```
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

impl ToCString for &Path {
    fn to_cstring(self) -> Result<CString> {
        self.to_str().ok_or(Error::Utf8Error(None))?.to_cstring()
    }
}

/// Implement `ToCString` by `as_ref` method.
///
/// `impl_to_cstring_by_as_ref!(ref_t, t1, t2, ...)` will implement `ToCString` for `t1`, `t2`, ...
/// by `as_ref::<ref_t>` method.
#[macro_export]
macro_rules! impl_to_cstring_by_as_ref {
    ($ref_t:ty, $($t:ty),*) => {
        $(
            impl $crate::ToCString for $t {
                fn to_cstring(self) -> Result<std::ffi::CString> {
                    let r: &$ref_t = self.as_ref();
                    r.to_cstring()
                }
            }
        )*
    };
}

impl_to_cstring_by_as_ref!(str, String, &String);

impl_to_cstring_by_as_ref!(Path, PathBuf, &PathBuf);

impl ToCString for bool {
    fn to_cstring(self) -> Result<CString> {
        if self { "1" } else { "0" }.to_cstring()
    }
}

/// Implement `ToCString` by `to_string` method.
///
/// `impl_to_cstring_by_to_string!(t1, t2, ...)` will implement `ToCString` for `t1`, `t2`, ... by
/// `to_string` method.
#[macro_export]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_cstring() {
        assert_eq!("foo".to_cstring().unwrap(), CString::new("foo").unwrap());
        assert_eq!(
            String::from("foo").to_cstring().unwrap(),
            CString::new("foo").unwrap()
        );
        assert_eq!(
            (&String::from("foo")).to_cstring().unwrap(),
            CString::new("foo").unwrap()
        );

        assert_eq!(
            Path::new("/tmp").to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );
        assert_eq!(
            PathBuf::from("/tmp").to_cstring().unwrap(),
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
    }
}
