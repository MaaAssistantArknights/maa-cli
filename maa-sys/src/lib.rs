use std::ffi::CStr;

use maa_types::primitive::*;
pub use maa_types::{InstanceOptionKey, StaticOptionKey, TaskType, TouchMode};

mod to_cstring;
pub use to_cstring::ToCString;

#[macro_use]
mod link;

/// Raw binding of MaaCore API
pub mod binding;

#[cfg_attr(test, derive(PartialEq, Eq))]
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
        unsafe { binding::AsstSetUserDir(path.to_cstring()?.as_ptr()) }.to_result()
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
        .to_result()
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
        unsafe { binding::AsstLoadResource(path.to_cstring()?.as_ptr()) }.to_result()
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
        .to_result()
    }

    /// Append a task to the assistant, return the task id.
    pub fn append_task(&self, task: impl ToCString, params: impl ToCString) -> Result<AsstTaskId> {
        unsafe {
            binding::AsstAppendTask(
                self.handle,
                task.to_cstring()?.as_ptr(),
                params.to_cstring()?.as_ptr(),
            )
        }
        .to_result()
    }

    /// Set the parameters of the given task.
    pub fn set_task_params(&self, task_id: AsstTaskId, params: impl ToCString) -> Result<()> {
        unsafe { binding::AsstSetTaskParams(self.handle, task_id, params.to_cstring()?.as_ptr()) }
            .to_result()
    }

    /// Start the assistant.
    pub fn start(&self) -> Result<()> {
        unsafe { binding::AsstStart(self.handle) }.to_result()
    }

    /// Stop the assistant.
    pub fn stop(&self) -> Result<()> {
        unsafe { binding::AsstStop(self.handle) }.to_result()
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
        unsafe {
            binding::AsstAsyncConnect(
                self.handle,
                adb_path.to_cstring()?.as_ptr(),
                address.to_cstring()?.as_ptr(),
                config.to_cstring()?.as_ptr(),
                block.into(),
            )
        }
        .to_result()
    }

    /// Set the connection extras of the assistant
    pub fn set_connection_extras(
        &self,
        name: impl ToCString,
        extras: impl ToCString,
    ) -> Result<()> {
        handle_asst(unsafe {
            binding::AsstSetConnectionExtras(
                self.handle,
                name.to_cstring()?.as_ptr(),
                extras.to_cstring()?.as_ptr(),
            )
        })
    }

    /// Click the screen at the given position
    pub fn async_click(&self, x: i32, y: i32, block: bool) -> Result<AsstAsyncCallId> {
        unsafe { binding::AsstAsyncClick(self.handle, x, y, block.into()) }.to_result()
    }

    /// Take a screenshot
    pub fn async_screncap(&self, block: bool) -> Result<AsstAsyncCallId> {
        unsafe { binding::AsstAsyncScreencap(self.handle, block.into()) }.to_result()
    }

    /// Take a screenshot and save it to the given buffer
    pub fn get_image(&self, buff: &mut [u8], buff_size: AsstSize) -> Result<AsstSize> {
        unsafe {
            binding::AsstGetImage(
                self.handle,
                buff.as_mut_ptr() as *mut std::os::raw::c_void,
                buff_size,
            )
        }
        .to_result()
    }

    /// Get the UUID of the device
    pub fn get_uuid(&self, buff: &mut [u8], buff_size: AsstSize) -> Result<AsstSize> {
        unsafe {
            binding::AsstGetUUID(
                self.handle,
                buff.as_mut_ptr() as *mut std::os::raw::c_char,
                buff_size,
            )
        }
        .to_result()
    }
}

trait AsstResult {
    /// The return type of the function
    type Return;

    fn to_result(self) -> Result<Self::Return>;
}

impl AsstResult for maa_types::primitive::AsstBool {
    type Return = ();

    fn to_result(self) -> Result<()> {
        if self == 1 {
            Ok(())
        } else {
            Err(Error::MAAError)
        }
    }
}

/// The null size is used to indicate a failure in the API which returns a AsstSize.
///
/// Ideally we should use a binding::GetNullSize() function to get the null size,
/// but it's okay to use a constant here since the null size is defined as u64::MAX in MaaCore.
pub const NULL_SIZE: AsstSize = AsstSize::MAX;

impl AsstResult for maa_types::primitive::AsstSize {
    type Return = Self;

    fn to_result(self) -> Result<Self> {
        if self == NULL_SIZE {
            Err(Error::MAAError)
        } else {
            Ok(self)
        }
    }
}

/// The invalid id is used to indicate a failure in the API which returns a AsstId.
///
/// The invalid id is defined as 0 in MaaCore, so we can use a constant here.
pub const INVALID_ID: AsstId = 0;

impl AsstResult for maa_types::primitive::AsstId {
    type Return = Self;

    fn to_result(self) -> Result<Self> {
        if self == INVALID_ID {
            Err(Error::MAAError)
        } else {
            Ok(self)
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
    fn asst_bool() {
        assert_eq!(0u8.to_result(), Err(super::Error::MAAError));
        assert_eq!(1u8.to_result(), Ok(()));
    }

    #[test]
    fn asst_size() {
        assert_eq!(NULL_SIZE.to_result(), Err(super::Error::MAAError));
        assert_eq!(1u64.to_result(), Ok(1u64));
        #[cfg(not(feature = "runtime"))]
        assert_eq!(unsafe { binding::AsstGetNullSize() }, NULL_SIZE);
    }

    #[test]
    fn asst_id() {
        assert_eq!(INVALID_ID.to_result(), Err(super::Error::MAAError));
        assert_eq!(1u64.to_result(), Ok(1u64));
    }
}
