mod error;
pub use error::{Error, Result};

mod asst_type;
pub use asst_type::{InstanceOptionKey, StaticOptionKey, TouchMode};

mod to_cstring;
pub use to_cstring::ToCString;

mod task_type;
pub use task_type::TaskType;

#[macro_use]
mod link;

/// Raw binding
pub mod binding;

use std::ffi::CStr;

use binding::{AsstAsyncCallId, AsstSize, AsstTaskId};

fn handle_asst(code: binding::AsstBool) -> Result<()> {
    if code == 1 {
        Ok(())
    } else {
        Err(Error::MAAError)
    }
}

/// A wrapper of the assistant instance.
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

    /*------------------------- Static Methods -------------------------*/

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
        handle_asst(unsafe { binding::AsstSetUserDir(path.to_cstring()?.as_ptr()) })
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
    pub fn set_static_option(
        key: binding::AsstStaticOptionKey,
        value: impl ToCString,
    ) -> Result<()> {
        handle_asst(unsafe { binding::AsstSetStaticOption(key, value.to_cstring()?.as_ptr()) })
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
        handle_asst(unsafe { binding::AsstLoadResource(path.to_cstring()?.as_ptr()) })
    }

    /// Get the null size of the assistant.
    pub fn get_null_size() -> AsstSize {
        unsafe { binding::AsstGetNullSize() }
    }

    /// Get the version of the assistant.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the version is not a valid UTF-8 string.
    pub fn get_version<'a>() -> Result<&'a str> {
        unsafe {
            let c_str = binding::AsstGetVersion();
            let version = CStr::from_ptr(c_str).to_str()?;
            Ok(version)
        }
    }

    /// Log a message to the assistant log.
    pub fn log(level: impl ToCString, msg: impl ToCString) -> Result<()> {
        unsafe { binding::AsstLog(level.to_cstring()?.as_ptr(), msg.to_cstring()?.as_ptr()) };
        Ok(())
    }

    /*------------------------ Instance Methods ------------------------*/
    //// Set the instance option of the assistant.
    pub fn set_instance_option(
        &self,
        key: binding::AsstInstanceOptionKey,
        value: impl ToCString,
    ) -> Result<()> {
        handle_asst(unsafe {
            binding::AsstSetInstanceOption(self.handle, key, value.to_cstring()?.as_ptr())
        })
    }

    /// Connect to device with the given adb path, address and config.
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
        handle_asst(unsafe {
            binding::AsstSetTaskParams(self.handle, task_id, params.to_cstring()?.as_ptr())
        })
    }

    /// Start the assistant.
    pub fn start(&self) -> Result<()> {
        handle_asst(unsafe { binding::AsstStart(self.handle) })
    }
    /// Stop the assistant.
    pub fn stop(&self) -> Result<()> {
        handle_asst(unsafe { binding::AsstStop(self.handle) })
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

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "runtime"))]
    #[test]
    fn get_version() {
        let version = super::Assistant::get_version().unwrap();

        if let Some(v_str) = std::env::var_os("MAA_CORE_VERSION") {
            assert_eq!(version, &v_str.to_str().unwrap()[1..]);
        }
    }
}
