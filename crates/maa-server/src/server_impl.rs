use crate::session::Session;

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
    // restore and free the mem
    let session_id: SessionID = unsafe {
        let mut raw = [0u8; 16];
        let ptr = session_id_ptr as *mut u8;
        let len = 16;
        raw.copy_from_slice(std::slice::from_raw_parts(ptr, len));
        raw
    };
    if entry(code, json_str, session_id) {
        tracing::info!("Remove Session");
        assert!(Session::remove(session_id));
        let vec = unsafe {
            let ptr = session_id_ptr as *mut u8;
            let len = 16;
            let cap = 16;
            Vec::from_raw_parts(ptr, len, cap)
        };
        drop(vec);
    }
}
