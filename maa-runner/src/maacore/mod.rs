mod binding;

mod message;
pub use message::default_callback;

use anyhow::{anyhow, Context, Result};
use std::ffi::{CStr, CString, NulError};

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

fn handle_error(code: binding::AsstBool) -> Result<()> {
    return match code {
        1 => Ok(()),
        _ => Err(anyhow!("MaaCore Error: {}", code)),
    };
}

pub struct Assistant {
    handle: binding::AsstHandle,
}

impl Assistant {
    pub fn new(callback: binding::AsstApiCallback, arg: Option<*mut std::os::raw::c_void>) -> Self {
        return match callback {
            Some(cb) => unsafe {
                let handle = binding::AsstCreateEx(Some(cb), arg.unwrap_or(std::ptr::null_mut()));
                Self { handle }
            },
            None => unsafe {
                let handle = binding::AsstCreate();
                Self { handle }
            },
        };
    }

    /* Static Methods */
    pub fn set_user_dir(path: impl ToCString) -> Result<()> {
        handle_error(unsafe { binding::AsstSetUserDir(path.to_cstring()?.as_ptr()) })
            .context("set_user_dir failed")
    }

    pub fn load_resource(path: impl ToCString) -> Result<()> {
        handle_error(unsafe { binding::AsstLoadResource(path.to_cstring()?.as_ptr()) })
            .context("load_resource failed")
    }

    pub fn get_version<'a>() -> Result<&'a str> {
        return unsafe {
            let c_str = binding::AsstGetVersion();
            let verion = CStr::from_ptr(c_str).to_str()?;
            Ok(verion)
        };
    }

    /* Instance Methods */
    pub fn set_instance_option(
        &self,
        key: impl Into<binding::AsstOptionKey>,
        value: impl ToCString,
    ) -> Result<()> {
        handle_error(unsafe {
            binding::AsstSetInstanceOption(self.handle, key.into(), value.to_cstring()?.as_ptr())
        })
        .context("set_instance_option failed")
    }

    pub fn append_task(
        &self,
        task: impl ToCString,
        params: impl ToCString,
    ) -> Result<binding::AsstTaskId> {
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
    ) -> Result<()> {
        handle_error(unsafe {
            binding::AsstSetTaskParams(self.handle, task_id, params.to_cstring()?.as_ptr())
        })
        .context("set_task_params failed")
    }

    pub fn start(&self) -> Result<()> {
        handle_error(unsafe { binding::AsstStart(self.handle) }).context("start failed")
    }

    pub fn stop(&self) -> Result<()> {
        handle_error(unsafe { binding::AsstStop(self.handle) }).context("stop failed")
    }

    pub fn running(&self) -> bool {
        unsafe { binding::AsstRunning(self.handle) != 0 }
    }

    pub fn connect(
        &self,
        adb: impl ToCString,
        addr: impl ToCString,
        config: impl ToCString + std::fmt::Debug,
    ) -> Result<()> {
        handle_error(unsafe {
            binding::AsstConnect(
                self.handle,
                adb.to_cstring()?.as_ptr(),
                addr.to_cstring()?.as_ptr(),
                config.to_cstring()?.as_ptr(),
            )
        })
        .context("connect failed")
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
