use std::sync::Arc;

use maa_ffi_types::AsstMsgId;
use maa_types::MessageKind;

/// Trait for receiving MaaCore callback messages.
///
/// MaaCore invokes [`on_message`] from its **own internal thread**, concurrently
/// with any method calls you make on [`Assistant`] from your thread. Therefore:
///
/// - Both `Send` and `Sync` are required.
/// - Any mutable state inside the implementation must use interior mutability (`Mutex`, `RwLock`,
///   `Atomic*`, etc.).
///
/// # Provided implementations
///
/// Two blanket implementations are provided so you rarely need to implement
/// this trait manually:
///
/// - **Closures / function pointers** — any `Fn(MessageKind, Option<&str>) + Send + Sync`.
/// - **`Arc<C>`** where `C: Callback` — lets you share a callback with the caller while also
///   passing it to [`Assistant::new_with_callback`].
///
/// # Example
///
/// ```
/// use std::sync::{Arc, Mutex};
///
/// use maa_core::Callback;
/// use maa_types::MessageKind;
///
/// // Plain closure — no manual impl needed
/// let cb = |kind: MessageKind, msg: Option<&str>| {
///     println!("{kind:?}: {}", msg.unwrap_or("<no detail>"));
/// };
///
/// // Struct with interior mutability
/// struct Log(Mutex<Vec<String>>);
/// impl Callback for Log {
///     fn on_message(&self, kind: MessageKind, msg: Option<&str>) {
///         if let Some(m) = msg {
///             self.0.lock().unwrap().push(format!("{kind:?}: {m}"));
///         }
///     }
/// }
/// ```
///
/// [`on_message`]: Callback::on_message
/// [`Assistant`]: crate::Assistant
/// [`Assistant::new_with_callback`]: crate::Assistant::new_with_callback
pub trait Callback: Send + Sync {
    /// Called by MaaCore when a message is received.
    ///
    /// # Parameters
    ///
    /// - `kind` — the category of the message (see [`MessageKind`]).
    /// - `msg` — a JSON string with details about the message, or `None` if MaaCore passed a null
    ///   pointer, or the detail string is not valid UTF-8.
    ///
    /// See the [MaaCore callback schema] for details. Always handle `None` gracefully.
    ///
    /// # Panics
    ///
    /// If this method panics, MaaCore's internal thread will **abort** the
    /// process to prevent unwinding across the FFI boundary. Do not panic.
    ///
    /// [MaaCore callback schema]: https://docs.maa.plus/en-us/protocol/callback-schema.html
    fn on_message(&self, kind: MessageKind, msg: Option<&str>);
}

impl<F> Callback for F
where
    F: Fn(MessageKind, Option<&str>) + Send + Sync,
{
    fn on_message(&self, kind: MessageKind, msg: Option<&str>) {
        self(kind, msg)
    }
}

impl<C: Callback> Callback for Arc<C> {
    fn on_message(&self, kind: MessageKind, msg: Option<&str>) {
        (**self).on_message(kind, msg)
    }
}

/// The `extern "C"` trampoline monomorphized for each concrete `C: Callback`.
///
/// # Safety
///
/// The `userdata` pointer must be the address of a `C` (inside a `Box`), and the pointer
/// must remain valid during the MaaCore instance's lifetime.
pub(crate) unsafe extern "C" fn trampoline<C: Callback>(
    msg_kind: AsstMsgId,
    msg: *const std::ffi::c_char,
    userdata: *mut std::os::raw::c_void,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Safety: see function-level safety contract above.
        let cb = unsafe { &*(userdata as *const C) };

        let msg_str = if msg.is_null() {
            None
        } else {
            unsafe { std::ffi::CStr::from_ptr(msg) }.to_str().ok()
        };

        cb.on_message(msg_kind.into(), msg_str);
    }));
    if result.is_err() {
        std::process::abort();
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::{Arc, Mutex};

    use maa_types::MessageKind;

    use super::*;

    // Concrete Callback impl shared by all tests.
    struct Recorder(Mutex<Vec<(MessageKind, Option<String>)>>);

    impl Callback for Recorder {
        fn on_message(&self, kind: MessageKind, msg: Option<&str>) {
            self.0.lock().unwrap().push((kind, msg.map(String::from)));
        }
    }

    impl Recorder {
        fn new() -> Self {
            Self(Mutex::new(Vec::new()))
        }

        fn entries(&self) -> Vec<(MessageKind, Option<String>)> {
            self.0.lock().unwrap().clone()
        }
    }

    #[test]
    fn fn_blanket_impl() {
        type Log = Arc<Mutex<Vec<(MessageKind, Option<String>)>>>;

        let log = Log::default();
        let log2 = log.clone();
        let cb = move |kind: MessageKind, msg: Option<&str>| {
            log2.lock().unwrap().push((kind, msg.map(String::from)));
        };
        cb.on_message(MessageKind::InternalError, Some("a"));
        cb.on_message(MessageKind::AllTasksCompleted, None);
        let entries = log.lock().unwrap();
        assert_eq!(entries[0], (MessageKind::InternalError, Some("a".into())));
        assert_eq!(entries[1], (MessageKind::AllTasksCompleted, None));
    }

    #[test]
    fn arc_delegates() {
        let rec = Arc::new(Recorder::new());
        let cb: Arc<dyn Callback> = rec.clone();
        cb.on_message(MessageKind::ConnectionInfo, None);
        cb.on_message(MessageKind::InitFailed, None);
        let entries = rec.entries();
        assert_eq!(entries[0].0, MessageKind::ConnectionInfo);
        assert_eq!(entries[1].0, MessageKind::InitFailed);
    }

    #[test]
    fn trampoline_valid_msg() {
        use std::ffi::CString;
        let rec = Recorder::new();
        let msg = CString::new("payload").unwrap();
        unsafe { trampoline::<Recorder>(1, msg.as_ptr(), &rec as *const Recorder as *mut _) };
        let entries = rec.entries();
        assert_eq!(entries[0].0, MessageKind::InitFailed);
        assert_eq!(entries[0].1.as_deref(), Some("payload"));
    }

    #[test]
    fn trampoline_null_msg() {
        let rec = Recorder::new();
        unsafe { trampoline::<Recorder>(0, std::ptr::null(), &rec as *const Recorder as *mut _) };
        assert_eq!(rec.entries()[0].1, None);
    }

    #[test]
    fn trampoline_invalid_utf8_yields_none() {
        let rec = Recorder::new();
        // b"\xff" is not valid UTF-8; CStr needs a trailing null.
        let cstr = c"\xff";
        unsafe { trampoline::<Recorder>(0, cstr.as_ptr(), &rec as *const Recorder as *mut _) };
        assert_eq!(rec.entries()[0].1, None);
    }
}
