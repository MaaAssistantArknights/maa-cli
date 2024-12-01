/// Assistant Extension API type.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AsstExtAPI {
    _unused: [u8; 0],
}
/// Assistant handle type.
pub type AsstHandle = *mut AsstExtAPI;

use maa_types::primitive::*;

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

    pub fn AsstSetConnectionExtras(
        handle: AsstHandle,
        name: *const ::std::os::raw::c_char,
        extras: *const ::std::os::raw::c_char,
    ) -> AsstBool;

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
