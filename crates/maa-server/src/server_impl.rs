use crate::session::SessionExt;

pub mod core;
pub mod task;

unsafe extern "C" fn default_callback(
    code: maa_types::primitive::AsstMsgId,
    json_str: *const std::ffi::c_char,
    session_id_ptr: *mut std::ffi::c_void,
) {
    use crate::{
        callback::entry,
        types::{SessionID, TaskStateType},
    };

    let code: TaskStateType = code.try_into().unwrap();
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_str).to_str().unwrap() };
    let session_id = SessionID::from_ptr(session_id_ptr as *const u8);

    if entry(code, json_str, session_id) {
        // restore and free the mem
        tracing::info!("Remove Session");
        assert!(session_id.remove());
        SessionID::drop_ptr(session_id_ptr as *const u8);
    }
}
