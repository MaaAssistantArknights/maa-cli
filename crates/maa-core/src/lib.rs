#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::{
    ffi::{c_char, c_void},
    sync::RwLock,
};

use maa_ffi_string::ToCString;
use maa_ffi_types::*;
use maa_types::{InstanceOptionKey, StaticOptionKey};

mod callback;
pub use callback::Callback;
use callback::trampoline;

mod error;
use error::{AsstResult, BufferTooSmall};
pub use error::{Error, Result};

/// The user directory of the assistant.
static USER_DIR: RwLock<std::path::PathBuf> = RwLock::new(std::path::PathBuf::new());

/// Get the path of MaaCore's log file.
///
/// For use with `Error::MAAError`.
pub(crate) fn get_log_path() -> std::path::PathBuf {
    // Unwrap: The RwLock only errors if it is poisoned, which should never happen.
    USER_DIR.read().unwrap().join("debug").join("asst.log")
}

/// A safe and convenient wrapper of MaaCore Assistant API.
///
/// The optional callback is heap-allocated (`Box<dyn Callback>`) and its
/// raw pointer is passed to MaaCore. `AsstDestroy` is called in `Drop` before
/// the `Box` is released, guaranteeing the pointee outlives every callback
/// invocation.
pub struct Assistant {
    handle: maa_sys::binding::AsstHandle,
    _callback: Option<Box<dyn Callback>>,
}

impl Drop for Assistant {
    fn drop(&mut self) {
        // Destroy the handle before the callback is dropped to make sure the callback is not used
        // after the Box is freed.
        unsafe { maa_sys::binding::AsstDestroy(self.handle) };
    }
}

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
                // Safety: HSTRING::as_ptr returns a valid, NUL-terminated wide
                // string pointer that lives for the duration of this call.
                let code = unsafe { SetDllDirectoryW(HSTRING::from(dir).as_ptr()) };
                if code == 0 {
                    windows_result::HRESULT::from_thread().ok()?;
                }
            }
        }

        maa_sys::binding::load(path)?;

        Ok(())
    }

    /// Unload the shared library of the MaaCore.
    ///
    /// Must be called after all assistant instances are destroyed.
    pub fn unload() -> Result<()> {
        maa_sys::binding::unload();
        Ok(())
    }

    /// Check if the shared library of the MaaCore is loaded in this thread.
    pub fn loaded() -> bool {
        maa_sys::binding::loaded()
    }
}

#[cfg(not(feature = "runtime"))]
impl Assistant {
    /// Do nothing, as MaaCore is dynamically linked
    pub fn load(_: impl AsRef<std::path::Path>) -> Result<()> {
        Ok(())
    }

    /// Do nothing, as MaaCore is dynamically linked
    pub fn unload() -> Result<()> {
        Ok(())
    }

    /// Always returns true, as MaaCore is dynamically linked.
    pub fn loaded() -> bool {
        true
    }
}

// Static Methods
impl Assistant {
    /// Set the user directory of the assistant.
    ///
    /// Must be called before `set_static_option` and `load_resource`.
    /// If not set, the first `load_resource` directory is used.
    pub fn set_user_dir(path: impl ToCString) -> Result<()> {
        let cstring = path.to_cstring()?;
        unsafe { maa_sys::binding::AsstSetUserDir(cstring.as_ptr()) }.to_maa_result()?;
        let path_buf = std::path::PathBuf::from(cstring.to_string_lossy().as_ref());
        // Unwrap: The RwLock only errors if it is poisoned, which should never happen.
        *USER_DIR.write().unwrap() = path_buf;
        Ok(())
    }

    /// Set a global (static) option. Must be called before `load_resource`.
    pub fn set_static_option(key: StaticOptionKey, value: impl ToCString) -> Result<()> {
        let cstring = value.to_cstring()?;
        unsafe {
            maa_sys::binding::AsstSetStaticOption(key as AsstStaticOptionKey, cstring.as_ptr())
        }
        .to_maa_result()
    }

    /// Load resource from the given directory.
    ///
    /// The given directory should be the parent of the `resource` directory.
    pub fn load_resource(path: impl ToCString) -> Result<()> {
        let cstring = path.to_cstring()?;
        unsafe { maa_sys::binding::AsstLoadResource(cstring.as_ptr()) }.to_maa_result()
    }

    /// Get the version string of MaaCore.
    pub fn get_version() -> Result<String> {
        // Safety: AsstGetVersion returns a pointer into the loaded library's
        // read-only data segment, valid only while the library remains loaded.
        // `to_owned` copies the bytes before any unload can occur.
        let version =
            unsafe { std::ffi::CStr::from_ptr(maa_sys::binding::AsstGetVersion()).to_owned() };
        Ok(String::from_utf8(version.into_bytes())?)
    }

    /// Log a message to the MaaCore log.
    pub fn log(level: impl ToCString, msg: impl ToCString) -> Result<()> {
        let level = level.to_cstring()?;
        let msg = msg.to_cstring()?;
        unsafe { maa_sys::binding::AsstLog(level.as_ptr(), msg.as_ptr()) };
        Ok(())
    }

    /// Set extra ADB connection config (e.g. for MUMU12 or LDPlayer).
    pub fn set_connection_extras(name: impl ToCString, extras: impl ToCString) -> Result<()> {
        let name = name.to_cstring()?;
        let extras = extras.to_cstring()?;
        unsafe { maa_sys::binding::AsstSetConnectionExtras(name.as_ptr(), extras.as_ptr()) }
        Ok(())
    }
}

// Instance Methods
impl Assistant {
    /// Create a new assistant instance without a callback.
    pub fn new() -> Result<Self> {
        let handle = unsafe { maa_sys::binding::AsstCreate() };
        if handle.is_null() {
            return Err(Error::NullHandle);
        }
        Ok(Self {
            handle,
            _callback: None,
        })
    }

    /// Create a new assistant instance with a callback.
    ///
    /// Accepts any `C: Callback + 'static`, including plain functions, closures, or `Arc<C>`
    /// for sharing state with the caller (via the blanket impl on `Arc`).
    pub fn new_with_callback<C: Callback + 'static>(callback: C) -> Result<Self> {
        let boxed = Box::new(callback);
        let raw = &raw const *boxed as *mut c_void;
        let handle = unsafe { maa_sys::binding::AsstCreateEx(Some(trampoline::<C>), raw) };
        if handle.is_null() {
            return Err(Error::NullHandle);
        }
        Ok(Self {
            handle,
            _callback: Some(boxed as Box<dyn Callback>),
        })
    }

    /// Set an instance option.
    pub fn set_instance_option(&self, key: InstanceOptionKey, value: impl ToCString) -> Result<()> {
        unsafe {
            maa_sys::binding::AsstSetInstanceOption(
                self.handle,
                key as AsstInstanceOptionKey,
                value.to_cstring()?.as_ptr(),
            )
        }
        .to_maa_result()
    }

    /// Append a task to the assistant, returning the task ID.
    pub fn append_task(&self, task: impl ToCString, params: impl ToCString) -> Result<AsstTaskId> {
        unsafe {
            maa_sys::binding::AsstAppendTask(
                self.handle,
                task.to_cstring()?.as_ptr(),
                params.to_cstring()?.as_ptr(),
            )
        }
        .to_maa_result()
    }

    /// Set the parameters of an existing task.
    pub fn set_task_params(&self, task_id: AsstTaskId, params: impl ToCString) -> Result<()> {
        let params = params.to_cstring()?;
        unsafe { maa_sys::binding::AsstSetTaskParams(self.handle, task_id, params.as_ptr()) }
            .to_maa_result()
    }

    /// Start the assistant.
    pub fn start(&self) -> Result<()> {
        unsafe { maa_sys::binding::AsstStart(self.handle) }.to_maa_result()
    }

    /// Stop the assistant.
    pub fn stop(&self) -> Result<()> {
        unsafe { maa_sys::binding::AsstStop(self.handle) }.to_maa_result()
    }

    /// Returns `true` if the assistant is currently running.
    pub fn running(&self) -> bool {
        unsafe { maa_sys::binding::AsstRunning(self.handle) != 0 }
    }

    /// Returns `true` if the assistant is connected to a device.
    pub fn connected(&self) -> bool {
        unsafe { maa_sys::binding::AsstConnected(self.handle) != 0 }
    }

    /// Navigate back to the home screen.
    pub fn back_to_home(&self) -> Result<()> {
        unsafe { maa_sys::binding::AsstBackToHome(self.handle) }.to_maa_result()
    }

    /// Connect to a device asynchronously.
    pub fn async_connect(
        &self,
        adb_path: impl ToCString,
        address: impl ToCString,
        config: impl ToCString,
        block: bool,
    ) -> Result<AsstAsyncCallId> {
        let adb_path = adb_path.to_cstring()?;
        let address = address.to_cstring()?;
        let config = config.to_cstring()?;
        unsafe {
            maa_sys::binding::AsstAsyncConnect(
                self.handle,
                adb_path.as_ptr(),
                address.as_ptr(),
                config.as_ptr(),
                block.into(),
            )
        }
        .to_maa_result()
    }

    /// Click the screen at the given position.
    pub fn async_click(&self, x: i32, y: i32, block: bool) -> Result<AsstAsyncCallId> {
        unsafe { maa_sys::binding::AsstAsyncClick(self.handle, x, y, block.into()) }.to_maa_result()
    }

    /// Take a screenshot asynchronously.
    pub fn async_screencap(&self, block: bool) -> Result<AsstAsyncCallId> {
        unsafe { maa_sys::binding::AsstAsyncScreencap(self.handle, block.into()) }.to_maa_result()
    }

    /// Get the most recent screenshot into a caller-provided buffer.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Safety
    ///
    /// `buf` must point to at least `size` bytes of writable memory.
    unsafe fn get_image_raw(&self, buf: *mut u8, size: usize) -> Result<AsstSize, BufferTooSmall> {
        // Safety: caller guarantees buf points to at least `size` writable bytes.
        unsafe { maa_sys::binding::AsstGetImage(self.handle, buf as *mut c_void, size as AsstSize) }
            .to_result()
    }

    /// Get the most recent screenshot into a caller-provided buffer.
    pub fn get_image_with_buf(&self, buf: &mut [u8]) -> Result<AsstSize> {
        Ok(unsafe { self.get_image_raw(buf.as_mut_ptr(), buf.len())? })
    }

    /// Get the most recent screenshot as a PNG-encoded `Vec<u8>`.
    ///
    /// Returns `None` if no screenshot is cached (e.g. not yet connected).
    pub fn get_image(&self) -> Result<Option<Vec<u8>>> {
        // A 720p image with 24-bit color: 1280 × 720 × 3 ≈ 2.7 MB raw.
        // PNG compression keeps it well under 4 MB in practice.
        const INIT_SIZE: usize = 1024 * 1024 * 4;
        // 32 MB covers 4K raw images.
        const MAX_SIZE: usize = 1024 * 1024 * 32;

        let mut buf_size = INIT_SIZE;
        let mut buf = Vec::with_capacity(buf_size);

        loop {
            // Safety: buf has capacity buf_size bytes
            match unsafe { self.get_image_raw(buf.as_mut_ptr(), buf_size) } {
                Ok(0) => return Ok(None),
                Ok(size) => {
                    // Safety: AsstGetImage wrote exactly `size` bytes into buf.
                    unsafe { buf.set_len(size as usize) };
                    return Ok(Some(buf));
                }
                Err(BufferTooSmall) => {
                    if buf_size > MAX_SIZE {
                        return Err(Error::ContentTooLarge(MAX_SIZE));
                    }
                    buf_size *= 2;
                    buf.reserve(buf_size);
                }
            }
        }
    }

    /// Take a fresh screenshot and return it as a PNG-encoded `Vec<u8>`.
    ///
    /// Returns `None` if the device is not connected.
    pub fn get_fresh_image(&self) -> Result<Option<Vec<u8>>> {
        self.async_screencap(true)?;
        self.get_image()
    }

    /// Get the UUID of the connected device.
    ///
    /// Returns `None` if the device is not yet connected.
    ///
    /// The returned string looks like `12345678-1234-1234-1234-1234567890ab`,
    /// but may not be a valid UUID on all platforms.
    pub fn get_uuid(&self) -> Result<Option<String>> {
        const UUID_BUFF_SIZE: usize = 128;
        let mut buf = Vec::with_capacity(UUID_BUFF_SIZE);
        match unsafe {
            maa_sys::binding::AsstGetUUID(
                self.handle,
                buf.as_mut_ptr() as *mut c_char,
                UUID_BUFF_SIZE as AsstSize,
            )
        }
        .to_result()
        {
            Ok(0) => Ok(None),
            Ok(size) => {
                // Safety: AsstGetUUID wrote exactly `size` bytes into buf.
                unsafe { buf.set_len(size as usize) };
                Ok(Some(String::from_utf8(buf)?))
            }
            Err(BufferTooSmall) => Err(Error::ContentTooLarge(UUID_BUFF_SIZE)),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(not(feature = "runtime"))]
    #[test]
    fn get_version() {
        let _version = Assistant::get_version().unwrap();
    }

    #[cfg(not(feature = "runtime"))]
    #[test]
    fn load_core() {
        assert!(Assistant::loaded());
        assert!(Assistant::load(std::path::Path::new("")).is_ok());
        assert!(Assistant::loaded());
        assert!(Assistant::unload().is_ok());
        assert!(Assistant::loaded());
    }
}
