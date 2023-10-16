pub mod binding;

mod link;

use std::ffi::{CStr, CString, NulError};
use std::path::{Path, PathBuf};
use std::str::Utf8Error;

use binding::{AsstAsyncCallId, AsstSize, AsstTaskId};

#[derive(Debug, Clone)]
pub enum Error {
    MAAError,
    BufferTooSmall,
    NulError(NulError),
    Utf8Error(Utf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MAAError => write!(f, "MAAError"),
            Error::BufferTooSmall => write!(f, "BufferTooSmall"),
            Error::NulError(err) => write!(f, "{}", err),
            Error::Utf8Error(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<NulError> for Error {
    fn from(err: NulError) -> Self {
        Error::NulError(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Error::Utf8Error(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait ToCString {
    fn to_cstring(self) -> Result<CString>;
}

impl ToCString for &str {
    fn to_cstring(self) -> Result<CString> {
        Ok(CString::new(self)?)
    }
}

impl ToCString for String {
    fn to_cstring(self) -> Result<CString> {
        Ok(CString::new(self)?)
    }
}

impl ToCString for &Path {
    fn to_cstring(self) -> Result<CString> {
        self.to_str().unwrap().to_cstring()
    }
}

impl ToCString for PathBuf {
    fn to_cstring(self) -> Result<CString> {
        self.to_str().unwrap().to_cstring()
    }
}

impl ToCString for &PathBuf {
    fn to_cstring(self) -> Result<CString> {
        self.to_str().unwrap().to_cstring()
    }
}

// Used in set_instance_option
impl ToCString for bool {
    fn to_cstring(self) -> Result<CString> {
        if self { "1" } else { "0" }.to_cstring()
    }
}

fn handle_asst(code: binding::AsstBool) -> Result<()> {
    if code == 1 {
        Ok(())
    } else {
        Err(Error::MAAError)
    }
}

pub struct Assistant {
    handle: binding::AsstHandle,
}

impl Drop for Assistant {
    fn drop(&mut self) {
        unsafe {
            binding::AsstDestroy(self.handle);
        }
    }
}

impl Assistant {
    pub fn new(callback: binding::AsstApiCallback, arg: Option<*mut std::os::raw::c_void>) -> Self {
        match callback {
            Some(cb) => unsafe {
                let handle = binding::AsstCreateEx(Some(cb), arg.unwrap_or(std::ptr::null_mut()));
                Self { handle }
            },
            None => unsafe {
                let handle = binding::AsstCreate();
                Self { handle }
            },
        }
    }

    /* Static Methods */
    pub fn set_user_dir(path: impl ToCString) -> Result<()> {
        handle_asst(unsafe { binding::AsstSetUserDir(path.to_cstring()?.as_ptr()) })
    }
    pub fn load_resource(path: impl ToCString) -> Result<()> {
        handle_asst(unsafe { binding::AsstLoadResource(path.to_cstring()?.as_ptr()) })
    }
    pub fn set_static_option(
        key: binding::AsstStaticOptionKey,
        value: impl ToCString,
    ) -> Result<()> {
        handle_asst(unsafe { binding::AsstSetStaticOption(key, value.to_cstring()?.as_ptr()) })
    }

    /* Instance Methods */
    pub fn set_instance_option(
        &self,
        key: binding::AsstInstanceOptionKey,
        value: impl ToCString,
    ) -> Result<()> {
        handle_asst(unsafe {
            binding::AsstSetInstanceOption(self.handle, key, value.to_cstring()?.as_ptr())
        })
    }

    #[deprecated(note = "use async_connect instead")]
    pub fn connect(
        &self,
        adb_path: impl ToCString,
        address: impl ToCString,
        config: impl ToCString,
    ) -> Result<()> {
        handle_asst(unsafe {
            #[allow(deprecated)]
            binding::AsstConnect(
                self.handle,
                adb_path.to_cstring()?.as_ptr(),
                address.to_cstring()?.as_ptr(),
                config.to_cstring()?.as_ptr(),
            )
        })
    }

    pub fn append_task(&self, task: impl ToCString, params: impl ToCString) -> Result<AsstTaskId> {
        Ok(unsafe {
            binding::AsstAppendTask(
                self.handle,
                task.to_cstring()?.as_ptr(),
                params.to_cstring()?.as_ptr(),
            )
        })
    }
    pub fn set_task_params(&self, task_id: AsstTaskId, params: impl ToCString) -> Result<()> {
        handle_asst(unsafe {
            binding::AsstSetTaskParams(self.handle, task_id, params.to_cstring()?.as_ptr())
        })
    }

    pub fn start(&self) -> Result<()> {
        handle_asst(unsafe { binding::AsstStart(self.handle) })
    }
    pub fn stop(&self) -> Result<()> {
        handle_asst(unsafe { binding::AsstStop(self.handle) })
    }
    pub fn running(&self) -> bool {
        unsafe { binding::AsstRunning(self.handle) != 0 }
    }
    pub fn connected(&self) -> bool {
        unsafe { binding::AsstConnected(self.handle) != 0 }
    }

    pub fn async_connect(
        &self,
        adb_path: impl ToCString,
        address: impl ToCString,
        config: impl ToCString,
        block: bool,
    ) -> Result<AsstAsyncCallId> {
        Ok(unsafe {
            binding::AsstAsyncConnect(
                self.handle,
                adb_path.to_cstring()?.as_ptr(),
                address.to_cstring()?.as_ptr(),
                config.to_cstring()?.as_ptr(),
                block.into(),
            )
        })
    }
    pub fn async_click(&self, x: i32, y: i32, block: bool) -> Result<AsstAsyncCallId> {
        Ok(unsafe { binding::AsstAsyncClick(self.handle, x, y, block.into()) })
    }
    pub fn async_screncap(&self, block: bool) -> Result<AsstAsyncCallId> {
        Ok(unsafe { binding::AsstAsyncScreencap(self.handle, block.into()) })
    }

    pub fn get_image(&self, buff: &mut [u8], buff_size: AsstSize) -> Result<AsstSize> {
        Ok(unsafe {
            binding::AsstGetImage(
                self.handle,
                buff.as_mut_ptr() as *mut std::os::raw::c_void,
                buff_size,
            )
        })
    }
    pub fn get_uuid(&self, buff: &mut [u8], buff_size: AsstSize) -> Result<AsstSize> {
        Ok(unsafe {
            binding::AsstGetUUID(
                self.handle,
                buff.as_mut_ptr() as *mut std::os::raw::c_char,
                buff_size,
            )
        })
    }
    pub fn get_null_size() -> AsstSize {
        unsafe { binding::AsstGetNullSize() }
    }

    pub fn get_version<'a>() -> Result<&'a str> {
        unsafe {
            let c_str = binding::AsstGetVersion();
            let verion = CStr::from_ptr(c_str).to_str()?;
            Ok(verion)
        }
    }
    pub fn log(level: impl ToCString, msg: impl ToCString) -> Result<()> {
        unsafe { binding::AsstLog(level.to_cstring()?.as_ptr(), msg.to_cstring()?.as_ptr()) };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn to_cstring() {
        assert_eq!("/tmp".to_cstring().unwrap(), CString::new("/tmp").unwrap());
        assert_eq!(
            Path::new("/tmp").to_cstring().unwrap(),
            CString::new("/tmp").unwrap()
        );
    }

    #[cfg(not(feature = "runtime"))]
    #[test]
    fn get_version() {
        assert!(Assistant::get_version().is_ok());
    }
}
