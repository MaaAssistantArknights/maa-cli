use std::ffi::CStr;

use maa_types::primitive::*;
pub use maa_types::{InstanceOptionKey, StaticOptionKey, TaskType, TouchMode};

mod to_cstring;
pub use to_cstring::ToCString;

#[macro_use]
mod link;

/// Raw binding of MaaCore API
pub mod binding;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("MaaCore returned an error, check its log for details")]
    MAAError,
    #[error("Buffer Too Small")]
    BufferTooSmall,
    #[error("Interior null byte")]
    Nul(#[from] std::ffi::NulError),
    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("Invalid UTF-8")]
    InvalidUtf8NoInfo,
    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub fn custom(msg: impl Into<String>) -> Self {
        Error::Custom(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A safe and convenient wrapper of MaaCore Assistant API.
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
    /// Create a new assistant instance with the given callback and argument.
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

    /* ------------------------- Static Methods ------------------------- */

    /// Set the user directory of the assistant.
    ///
    /// The user directory is used to store the log file and some cache files.
    ///
    /// Must by called before `set_static_option` and `load_resource`.
    /// If user directory is not set, the first load resource directory will be used.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the path is not a valid UTF-8 string,
    /// or raise an error if set the user directory failed.
    pub fn set_user_dir(path: impl ToCString) -> Result<()> {
        unsafe { binding::AsstSetUserDir(path.to_cstring()?.as_ptr()) }.to_err()
    }

    /// Set the static option of the assistant.
    ///
    /// The static option is used to set the global configuration of the assistant.
    /// Available options are defined in `StaticOptionKey`.
    ///
    /// This function must be called before `load_resource`.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the value is not a valid UTF-8 string,
    /// or raise an error if set the static option failed.
    pub fn set_static_option(key: StaticOptionKey, value: impl ToCString) -> Result<()> {
        unsafe {
            binding::AsstSetStaticOption(key as AsstStaticOptionKey, value.to_cstring()?.as_ptr())
        }
        .to_err()
    }

    /// Load resource from the given directory.
    ///
    /// The given directory should be the parent directory of the `resource` directory.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the path is not a valid UTF-8 string,
    /// or raise an error if load resource failed.
    pub fn load_resource(path: impl ToCString) -> Result<()> {
        unsafe { binding::AsstLoadResource(path.to_cstring()?.as_ptr()) }.to_err()
    }

    /// Get the null size of the assistant.
    pub fn get_null_size() -> maa_types::primitive::AsstSize {
        unsafe { binding::AsstGetNullSize() }
    }

    /// Get the version of the assistant.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the version is not a valid UTF-8 string.
    pub fn get_version() -> Result<String> {
        unsafe {
            let c_str = binding::AsstGetVersion();
            let version = CStr::from_ptr(c_str).to_str()?;
            Ok(String::from(version))
        }
    }

    /// Log a message to the assistant log.
    pub fn log(level: impl ToCString, msg: impl ToCString) -> Result<()> {
        unsafe { binding::AsstLog(level.to_cstring()?.as_ptr(), msg.to_cstring()?.as_ptr()) };
        Ok(())
    }

    /* ------------------------ Instance Methods ------------------------ */
    //// Set the instance option of the assistant.
    pub fn set_instance_option(&self, key: InstanceOptionKey, value: impl ToCString) -> Result<()> {
        unsafe {
            binding::AsstSetInstanceOption(
                self.handle,
                key as AsstInstanceOptionKey,
                value.to_cstring()?.as_ptr(),
            )
        }
        .to_err()
    }

    /// Append a task to the assistant, return the task id.
    pub fn append_task(&self, task: impl ToCString, params: impl ToCString) -> Result<AsstTaskId> {
        let task_id = unsafe {
            binding::AsstAppendTask(
                self.handle,
                task.to_cstring()?.as_ptr(),
                params.to_cstring()?.as_ptr(),
            )
        };
        if task_id == 0 {
            Err(Error::MAAError)
        } else {
            Ok(task_id)
        }
    }

    /// Set the parameters of the given task.
    pub fn set_task_params(&self, task_id: AsstTaskId, params: impl ToCString) -> Result<()> {
        unsafe { binding::AsstSetTaskParams(self.handle, task_id, params.to_cstring()?.as_ptr()) }
            .to_err()
    }

    /// Start the assistant.
    pub fn start(&self) -> Result<()> {
        unsafe { binding::AsstStart(self.handle) }.to_err()
    }

    /// Stop the assistant.
    pub fn stop(&self) -> Result<()> {
        unsafe { binding::AsstStop(self.handle) }.to_err()
    }

    /// Check if the assistant is running.
    pub fn running(&self) -> bool {
        unsafe { binding::AsstRunning(self.handle) != 0 }
    }

    /// Check if the assistant is connected.
    pub fn connected(&self) -> bool {
        unsafe { binding::AsstConnected(self.handle) != 0 }
    }

    /// Connect to device with the given adb path, address and config asynchronously
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

    /// Click the screen at the given position
    pub fn async_click(&self, x: i32, y: i32, block: bool) -> Result<AsstAsyncCallId> {
        Ok(unsafe { binding::AsstAsyncClick(self.handle, x, y, block.into()) })
    }

    /// Take a screenshot
    pub fn async_screncap(&self, block: bool) -> Result<AsstAsyncCallId> {
        Ok(unsafe { binding::AsstAsyncScreencap(self.handle, block.into()) })
    }

    /// Take a screenshot and save it to the given buffer
    pub fn get_image(&self, buff: &mut [u8], buff_size: AsstSize) -> Result<AsstSize> {
        Ok(unsafe {
            binding::AsstGetImage(
                self.handle,
                buff.as_mut_ptr() as *mut std::os::raw::c_void,
                buff_size,
            )
        })
    }

    /// Get the UUID of the device
    pub fn get_uuid(&self, buff: &mut [u8], buff_size: AsstSize) -> Result<AsstSize> {
        Ok(unsafe {
            binding::AsstGetUUID(
                self.handle,
                buff.as_mut_ptr() as *mut std::os::raw::c_char,
                buff_size,
            )
        })
    }
}

trait AsstBoolExt {
    fn to_err(self) -> Result<()>;
}

impl AsstBoolExt for maa_types::primitive::AsstBool {
    fn to_err(self) -> Result<()> {
        if self == 1 {
            Ok(())
        } else {
            Err(Error::MAAError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "runtime"))]
    #[test]
    fn get_version() {
        let version = Assistant::get_version().unwrap();

        if let Some(v_str) = std::env::var_os("MAA_CORE_VERSION") {
            assert_eq!(version, v_str.to_str().unwrap());
        }
    }

    #[test]
    fn asst_bool_ext() {
        assert!(matches!(0.to_err(), Err(super::Error::MAAError)));
        assert!(matches!(1.to_err(), Ok(())));
    }
}
