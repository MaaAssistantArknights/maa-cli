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
    #[error("Buffer too small")]
    BufferTooSmall,
    #[error("The content returned by MaaCore is too large (length > {0})")]
    ContentTooLarge(usize),
    #[error("Interior null byte")]
    Nul(#[from] std::ffi::NulError),
    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("Invalid UTF-8")]
    InvalidUtf8NoInfo,
    #[cfg(all(feature = "runtime", target_os = "windows"))]
    #[error("OS error")]
    OS(#[from] windows_result::Error),
    #[cfg(feature = "runtime")]
    #[error("Failed to load the shared library")]
    LoadError(#[from] libloading::Error),
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

// Load and unload Assistant
#[cfg(feature = "runtime")]
impl Assistant {
    /// Load the shared library of the MaaCore
    ///
    /// Must be called first before any other method.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<()> {
        let path = path.as_ref();

        #[cfg(target_os = "windows")]
        if let Some(dir) = path.parent() {
            use windows_strings::HSTRING;
            use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

            if dir != std::path::Path::new(".") {
                let code = unsafe { SetDllDirectoryW(HSTRING::from(dir.as_ref()).as_ptr()) };
                if code == 0 {
                    return Err(windows_result::Error::from_win32().into());
                }
            }
        }

        binding::load(path)?;

        Ok(())
    }

    /// Unload the shared library of the MaaCore.
    ///
    /// The shared library is used to load the assistant.
    ///
    /// Must be called after all assistant instances are destroyed.
    pub fn unload() -> Result<()> {
        binding::unload();

        Ok(())
    }

    /// Check if the shared library of the MaaCore is loaded in this thread.
    pub fn loaded() -> bool {
        binding::loaded()
    }
}

#[cfg(not(feature = "runtime"))]
impl Assistant {
    /// Do nothing, as MaaCore is linked dynamically at compile time
    pub fn load() -> Result<()> {
        Ok(())
    }

    /// Do nothing, as MaaCore is linked dynamically at compile time
    pub fn unload() -> Result<()> {
        Ok(())
    }

    /// Always returns true, as MaaCore is linked dynamically at compile time
    pub fn loaded() -> bool {
        true
    }
}

// Static Methods
impl Assistant {
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

    /// Get the version of the assistant.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the version is not a valid UTF-8 string.
    pub fn get_version() -> Result<String> {
        unsafe {
            let c_str = binding::AsstGetVersion();
            let version = std::ffi::CStr::from_ptr(c_str).to_str()?;
            Ok(String::from(version))
        }
    }

    /// Log a message to the assistant log.
    pub fn log(level: impl ToCString, msg: impl ToCString) -> Result<()> {
        unsafe { binding::AsstLog(level.to_cstring()?.as_ptr(), msg.to_cstring()?.as_ptr()) };
        Ok(())
    }
}

// Instance Methods
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

    /// Click the screen at the given position
    pub fn async_click(&self, x: i32, y: i32, block: bool) -> Result<AsstAsyncCallId> {
        unsafe { binding::AsstAsyncClick(self.handle, x, y, block.into()) }.to_result()
    }

    /// Take a screenshot
    pub fn async_screencap(&self, block: bool) -> Result<AsstAsyncCallId> {
        unsafe { binding::AsstAsyncScreencap(self.handle, block.into()) }.to_result()
    }

    /// Get the image of the most recent screenshot with a buffer
    ///
    /// Returns the size of the image data in bytes
    ///
    /// # Safety
    ///
    /// The buffer pointer should be larger or equal to the given `size`.
    pub unsafe fn get_image_with_buf(&self, buf: *mut u8, size: usize) -> Result<AsstSize> {
        unsafe {
            binding::AsstGetImage(
                self.handle,
                buf as *mut std::os::raw::c_void,
                size as AsstSize,
            )
        }
        .to_result()
    }

    /// Get the image of the most recent screenshot
    ///
    /// The returned value is a Vec<u8> containing the image data, encoded as PNG.
    pub fn get_image(&self) -> Result<Vec<u8>> {
        // A 720p image with 24bit color depth, the raw size is 1280 * 720 * 3 (2.7 MB)
        // Compressed image data should be smaller than the raw size.
        // 4MB should be enough in most cases
        const INIT_SIZE: usize = 1024 * 1024 * 4;
        // 32MB should be enough for 4K raw images
        const MAX_SIZE: usize = 1024 * 1024 * 32;

        let mut buf_size = INIT_SIZE;
        let mut buf = Vec::with_capacity(buf_size);

        loop {
            match unsafe { self.get_image_with_buf(buf.as_mut_ptr(), buf_size) } {
                Ok(size) => {
                    // Safety: the buffer is initialized by FFI, the size is the actual size
                    unsafe { buf.set_len(size as usize) };
                    return Ok(buf);
                }
                Err(_) => {
                    if buf_size > MAX_SIZE {
                        return Err(Error::ContentTooLarge(MAX_SIZE));
                    }
                    // Double the buffer size if it's not enough
                    buf_size *= 2;
                    buf.reserve(buf_size);
                }
            }
        }
    }

    /// Take a screenshot and get the image
    pub fn get_fresh_image(&self) -> Result<Vec<u8>> {
        self.async_screencap(true)?;
        self.get_image()
    }

    /// Get the UUID of the device as a string
    ///
    /// The return value is a string containing the device's UUID,
    /// which looks like `12345678-1234-1234-1234-1234567890ab`.
    /// However, it may not be a valid UUID, especially on macOS.
    /// Don't rely on it for anything important.
    pub fn get_uuid(&self) -> Result<String> {
        // A standard UUID representation is a 36 character hexadecimal string.
        // Even in some cases, the UUID may not be a valid UUID, but 128 bytes is enough to hold it.
        const UUID_BUFF_SIZE: usize = 128;
        let mut buff = Vec::with_capacity(UUID_BUFF_SIZE);
        match unsafe {
            binding::AsstGetUUID(
                self.handle,
                buff.as_mut_ptr() as *mut std::os::raw::c_char,
                UUID_BUFF_SIZE as AsstSize,
            )
        }
        .to_result()
        {
            Ok(size) => {
                // Safety: the buffer is initialized by FFI, the len is the actual length
                unsafe { buff.set_len(size as usize) };
                String::from_utf8(buff).map_err(|e| Error::InvalidUtf8(e.utf8_error()))
            }
            Err(_) => Err(Error::ContentTooLarge(UUID_BUFF_SIZE)),
        }
    }
}

/// Trait to convert the return value of asst FFI to a Result
trait AsstResult {
    /// The return type of the function
    type Return;

    /// Convert the return value to a Result
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
            Err(Error::BufferTooSmall)
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
    use std::ffi::OsStr;

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
        assert!(matches!(0u8.to_result(), Err(super::Error::MAAError)));
        assert!(matches!(1u8.to_result(), Ok(())));
    }

    #[test]
    fn asst_size() {
        assert!(matches!(
            NULL_SIZE.to_result(),
            Err(super::Error::BufferTooSmall)
        ));
        assert!(matches!(1u64.to_result(), Ok(1u64)));
        #[cfg(not(feature = "runtime"))]
        assert_eq!(unsafe { binding::AsstGetNullSize() }, NULL_SIZE);
    }

    #[test]
    fn asst_id() {
        assert!(matches!(
            INVALID_ID.to_result(),
            Err(super::Error::MAAError)
        ));
        assert!(matches!(1u64.to_result(), Ok(1u64)));
    }
}
