mod binding;

use std::ffi::{CString, NulError};

#[derive(Debug)]
pub enum Error {
    MaaError,
    NulError(NulError),
    Utf8Error(std::str::Utf8Error),
}

impl From<NulError> for Error {
    fn from(err: NulError) -> Self {
        Self::NulError(err)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

pub trait ToCString {
    fn to_cstring(self) -> Result<CString, NulError>;
}

impl ToCString for &str {
    fn to_cstring(self) -> Result<CString, NulError> {
        Ok(CString::new(self)?)
    }
}

impl ToCString for String {
    fn to_cstring(self) -> Result<CString, NulError> {
        Ok(CString::new(self.as_str())?)
    }
}

impl ToCString for &std::path::Path {
    fn to_cstring(self) -> Result<CString, NulError> {
        self.to_str().unwrap().to_cstring()
    }
}

impl ToCString for std::path::PathBuf {
    fn to_cstring(self) -> Result<CString, NulError> {
        self.to_str().unwrap().to_cstring()
    }
}

fn handle_error(code: binding::AsstBool) -> Result<(), Error> {
    return match code {
        1 => Ok(()),
        _ => Err(Error::MaaError),
    };
}

#[allow(dead_code)]
pub unsafe extern "C" fn default_callback(
    msg: std::os::raw::c_int,
    detail_json: *const ::std::os::raw::c_char,
    _: *mut ::std::os::raw::c_void,
) {
    println!(
        "msg:{}: {}",
        msg,
        std::ffi::CStr::from_ptr(detail_json)
            .to_str()
            .unwrap()
            .to_string()
    );
}

pub struct Assistant {
    handle: binding::AsstHandle,
}

impl Assistant {
    pub fn new(
        callback: Option<binding::AsstApiCallback>,
        arg: Option<*mut std::os::raw::c_void>,
    ) -> Self {
        return match callback {
            Some(cb) => unsafe {
                let handle = binding::AsstCreateEx(cb, arg.unwrap_or(std::ptr::null_mut()));
                Self { handle }
            },
            None => unsafe {
                let handle = binding::AsstCreate();
                Self { handle }
            },
        };
    }

    /* Static Methods */
    pub fn set_user_dir(path: impl ToCString) -> Result<(), Error> {
        handle_error(unsafe { binding::AsstSetUserDir(path.to_cstring()?.as_ptr()) })
    }

    pub fn load_resource(path: impl ToCString) -> Result<(), Error> {
        handle_error(unsafe { binding::AsstLoadResource(path.to_cstring()?.as_ptr()) })
    }

    pub fn get_version() -> Result<String, Error> {
        return unsafe {
            let c_str = binding::AsstGetVersion();
            let verion = std::ffi::CStr::from_ptr(c_str).to_str()?.to_string();
            Ok(verion)
        };
    }

    /* Instance Methods */
    pub fn set_instance_option(
        &self,
        key: impl Into<binding::AsstOptionKey>,
        value: impl ToCString,
    ) -> Result<(), Error> {
        handle_error(unsafe {
            binding::AsstSetInstanceOption(self.handle, key.into(), value.to_cstring()?.as_ptr())
        })
    }

    pub fn append_task(
        &self,
        task: impl ToCString,
        params: impl ToCString,
    ) -> Result<binding::AsstTaskId, Error> {
        let ret = unsafe {
            binding::AsstAppendTask(
                self.handle,
                task.to_cstring()?.as_ptr(),
                params.to_cstring()?.as_ptr(),
            )
        };
        return Ok(ret);
    }

    #[allow(dead_code)]
    pub fn set_task_params(
        &self,
        task_id: binding::AsstTaskId,
        params: impl ToCString,
    ) -> Result<(), Error> {
        handle_error(unsafe {
            binding::AsstSetTaskParams(self.handle, task_id, params.to_cstring()?.as_ptr())
        })
    }

    pub fn start(&self) -> Result<(), Error> {
        handle_error(unsafe { binding::AsstStart(self.handle) })
    }

    pub fn stop(&self) -> Result<(), Error> {
        handle_error(unsafe { binding::AsstStop(self.handle) })
    }

    pub fn running(&self) -> bool {
        unsafe { binding::AsstRunning(self.handle) != 0 }
    }

    pub fn connect(
        &self,
        adb: impl ToCString,
        addr: impl ToCString,
        config: impl ToCString + std::fmt::Debug,
    ) -> Result<(), Error> {
        handle_error(unsafe {
            binding::AsstConnect(
                self.handle,
                adb.to_cstring()?.as_ptr(),
                addr.to_cstring()?.as_ptr(),
                config.to_cstring()?.as_ptr(),
            )
        })
    }
}

impl Drop for Assistant {
    fn drop(&mut self) {
        unsafe {
            binding::AsstDestroy(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn to_cstring() {
        assert_eq!("/tmp".to_cstring().unwrap(), CString::new("/tmp").unwrap());
        assert_eq!(
            String::from("/tmp").to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );
        assert_eq!(
            Path::new("/tmp").to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );
        assert_eq!(
            PathBuf::from("/tmp").to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );
    }
}
