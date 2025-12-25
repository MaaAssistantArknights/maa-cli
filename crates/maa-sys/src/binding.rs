/// Assistant Extension API type.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AsstExtAPI {
    _unused: [u8; 0],
}
/// Assistant handle type.
pub type AsstHandle = *mut AsstExtAPI;

use std::ffi::{c_char, c_void};

use maa_types::primitive::*;

/// Callback function type for assistant API.
pub type AsstApiCallback = ::std::option::Option<
    unsafe extern "C" fn(msg: AsstMsgId, details_json: *const c_char, custom_arg: *mut c_void),
>;

link! {
    pub fn AsstSetUserDir(path: *const c_char) -> AsstBool;
    pub fn AsstLoadResource(path: *const c_char) -> AsstBool;
    pub fn AsstSetStaticOption(key: AsstStaticOptionKey, value: *const c_char) -> AsstBool;

    pub fn AsstCreate() -> AsstHandle;
    pub fn AsstCreateEx(callback: AsstApiCallback, custom_arg: *mut c_void) -> AsstHandle;
    pub fn AsstDestroy(handle: AsstHandle);

    pub fn AsstSetInstanceOption(
        handle: AsstHandle,
        key: AsstInstanceOptionKey,
        value: *const c_char,
    ) -> AsstBool;

    pub fn AsstConnect(
        handle: AsstHandle,
        adb_path: *const c_char,
        address: *const c_char,
        config: *const c_char,
    ) -> AsstBool;

    pub fn AsstAppendTask(
        handle: AsstHandle,
        type_: *const c_char,
        params: *const c_char,
    ) -> AsstTaskId;
    pub fn AsstSetTaskParams(handle: AsstHandle, id: AsstTaskId, params: *const c_char) -> AsstBool;

    pub fn AsstStart(handle: AsstHandle) -> AsstBool;
    pub fn AsstStop(handle: AsstHandle) -> AsstBool;
    pub fn AsstRunning(handle: AsstHandle) -> AsstBool;
    pub fn AsstConnected(handle: AsstHandle) -> AsstBool;
    pub fn AsstBackToHome(handle: AsstHandle) -> AsstBool;

    pub fn AsstAsyncConnect(
        handle: AsstHandle,
        adb_path: *const c_char,
        address: *const c_char,
        config: *const c_char,
        block: AsstBool,
    ) -> AsstAsyncCallId;
    pub fn AsstSetConnectionExtras(name: *const c_char, extras: *const c_char);

    pub fn AsstAsyncClick(handle: AsstHandle, x: i32, y: i32, block: AsstBool) -> AsstAsyncCallId;
    pub fn AsstAsyncScreencap(handle: AsstHandle, block: AsstBool) -> AsstAsyncCallId;

    pub fn AsstGetImage(handle: AsstHandle, buff: *mut c_void, buff_size: AsstSize) -> AsstSize;
    pub fn AsstGetUUID(handle: AsstHandle, buff: *mut c_char, buff_size: AsstSize) -> AsstSize;
    pub fn AsstGetTasksList(
        handle: AsstHandle,
        buff: *mut AsstTaskId,
        buff_size: AsstSize,
    ) -> AsstSize;
    pub fn AsstGetNullSize() -> AsstSize;

    pub fn AsstGetVersion() -> *const c_char;
    pub fn AsstLog(level: *const c_char, message: *const c_char);
}
