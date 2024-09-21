/// Assistant Extension API type.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AsstExtAPI {
    _unused: [u8; 0],
}
/// Assistant handle type.
pub type AsstHandle = *mut AsstExtAPI;

/// Boolean type for assistant API.
pub type AsstBool = u8;
/// Size type for assistant API.
pub type AsstSize = u64;

/// Id type for assistant API.
pub type AsstId = i32;
/// Message id type for assistant API.
pub type AsstMsgId = AsstId;
/// Task id type for assistant API.
pub type AsstTaskId = AsstId;
/// Async call id type for assistant API.
pub type AsstAsyncCallId = AsstId;

/// Option key type for assistant API.
pub type AsstOptionKey = i32;
/// Static option key type for assistant API.
pub type AsstStaticOptionKey = AsstOptionKey;
/// Instance option key type for assistant API.
pub type AsstInstanceOptionKey = AsstOptionKey;

/// Callback function type for assistant API.
pub type AsstApiCallback = ::std::option::Option<
    unsafe extern "C" fn(
        msg: AsstMsgId,
        details_json: *const ::std::os::raw::c_char,
        custom_arg: *mut ::std::os::raw::c_void,
    ),
>;

link! {
    pub fn AsstSetUserDir(path: *const ::std::os::raw::c_char) -> AsstBool;
    pub fn AsstLoadResource(path: *const ::std::os::raw::c_char) -> AsstBool;
    pub fn AsstSetStaticOption(
        key: AsstStaticOptionKey,
        value: *const ::std::os::raw::c_char,
    ) -> AsstBool;

    pub fn AsstCreate() -> AsstHandle;
    pub fn AsstCreateEx(
        callback: AsstApiCallback,
        custom_arg: *mut ::std::os::raw::c_void,
    ) -> AsstHandle;
    pub fn AsstDestroy(handle: AsstHandle);

    pub fn AsstSetInstanceOption(
        handle: AsstHandle,
        key: AsstInstanceOptionKey,
        value: *const ::std::os::raw::c_char,
    ) -> AsstBool;

    pub fn AsstConnect(
        handle: AsstHandle,
        adb_path: *const ::std::os::raw::c_char,
        address: *const ::std::os::raw::c_char,
        config: *const ::std::os::raw::c_char,
    ) -> AsstBool;

    pub fn AsstAppendTask(
        handle: AsstHandle,
        type_: *const ::std::os::raw::c_char,
        params: *const ::std::os::raw::c_char,
    ) -> AsstTaskId;
    pub fn AsstSetTaskParams(
        andle: AsstHandle,
        id: AsstTaskId,
        params: *const ::std::os::raw::c_char,
    ) -> AsstBool;

    pub fn AsstStart(handle: AsstHandle) -> AsstBool;
    pub fn AsstStop(handle: AsstHandle) -> AsstBool;
    pub fn AsstRunning(handle: AsstHandle) -> AsstBool;
    pub fn AsstConnected(handle: AsstHandle) -> AsstBool;

    pub fn AsstAsyncConnect(
        handle: AsstHandle,
        adb_path: *const ::std::os::raw::c_char,
        address: *const ::std::os::raw::c_char,
        config: *const ::std::os::raw::c_char,
        block: AsstBool,
    ) -> AsstAsyncCallId;
    pub fn AsstAsyncClick(handle: AsstHandle, x: i32, y: i32, block: AsstBool) -> AsstAsyncCallId;
    pub fn AsstAsyncScreencap(handle: AsstHandle, block: AsstBool) -> AsstAsyncCallId;

    pub fn AsstGetImage(
        handle: AsstHandle,
        buff: *mut ::std::os::raw::c_void,
        buff_size: AsstSize,
    ) -> AsstSize;
    pub fn AsstGetUUID(
        handle: AsstHandle,
        buff: *mut ::std::os::raw::c_char,
        buff_size: AsstSize,
    ) -> AsstSize;
    pub fn AsstGetTasksList(
        handle: AsstHandle,
        buff: *mut AsstTaskId,
        buff_size: AsstSize,
    ) -> AsstSize;
    pub fn AsstGetNullSize() -> AsstSize;

    pub fn AsstGetVersion() -> *const ::std::os::raw::c_char;
    pub fn AsstLog(level: *const ::std::os::raw::c_char, message: *const ::std::os::raw::c_char);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "runtime")]
    #[test]
    #[ignore = "Need to set MAA_CORE_DIR"]
    fn test_link() {
        let lib_dir = std::env::var_os("MAA_CORE_DIR")
            .map(std::path::PathBuf::from)
            .expect("Please set MAA_CORE_DIR to the path of the shared library");
        let lib_name = format!(
            "{}MaaCore{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX,
        );
        let lib_path = lib_dir.join(lib_name);

        #[cfg(target_os = "windows")]
        {
            use windows::core::HSTRING;
            use windows::Win32::System::LibraryLoader::SetDllDirectoryW;

            unsafe {
                SetDllDirectoryW(&HSTRING::from(&lib_dir)).expect("Failed to set DLL directory")
            };
        }

        let lib = SharedLibrary::new(lib_path).expect("Failed to load shared library");

        let f = *unsafe {
            lib.handle
                .get::<extern "C" fn() -> *const ::std::os::raw::c_char>(b"AsstGetVersion\0")
        }
        .expect("Failed to get function");

        let ver = f();

        if let Some(v_str) = std::env::var_os("MAA_CORE_VERSION") {
            assert_eq!(
                unsafe { std::ffi::CStr::from_ptr(ver).to_str().unwrap() },
                v_str.to_str().unwrap()
            );
        }
    }
}
