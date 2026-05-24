use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{Assistant, Error, Result};

const RUNTIME_LOCKED: usize = 1usize << (usize::BITS - 1);
const RUNTIME_LOADED: usize = 1usize << (usize::BITS - 2);
const ASSISTANT_COUNT_MASK: usize = RUNTIME_LOADED - 1;

static RUNTIME_STATE: AtomicUsize = AtomicUsize::new(0);

fn is_locked(state: usize) -> bool {
    state & RUNTIME_LOCKED != 0
}

fn is_loaded(state: usize) -> bool {
    state & RUNTIME_LOADED != 0
}

fn assistant_count(state: usize) -> usize {
    state & ASSISTANT_COUNT_MASK
}

enum StableState {
    Loaded,
    Unloaded,
}

impl StableState {
    fn raw(&self) -> usize {
        match self {
            Self::Loaded => RUNTIME_LOADED,
            Self::Unloaded => 0,
        }
    }
}

struct RuntimeChange {
    committed: bool,
}

impl RuntimeChange {
    fn begin() -> Result<Self> {
        loop {
            let state = RUNTIME_STATE.load(Ordering::Acquire);
            if is_locked(state) {
                return Err(Error::RuntimeChanging);
            }

            if RUNTIME_STATE
                .compare_exchange(
                    state,
                    state | RUNTIME_LOCKED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
            {
                continue;
            }

            let locked_state = RUNTIME_STATE.load(Ordering::Acquire);
            if assistant_count(locked_state) > 0 {
                RUNTIME_STATE.fetch_and(!RUNTIME_LOCKED, Ordering::AcqRel);
                return Err(Error::ActiveAssistants);
            }

            return Ok(Self { committed: false });
        }
    }

    fn commit(mut self, state: StableState) {
        RUNTIME_STATE.store(state.raw(), Ordering::Release);
        self.committed = true;
    }
}

impl Drop for RuntimeChange {
    fn drop(&mut self) {
        if !self.committed {
            RUNTIME_STATE.fetch_and(!RUNTIME_LOCKED, Ordering::AcqRel);
        }
    }
}

impl Assistant {
    /// Load the shared library of the MaaCore
    ///
    /// Must be called before any other method.
    /// Fails if any assistant instances are alive.
    pub fn load(path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let runtime_change = RuntimeChange::begin()?;

        #[cfg(target_os = "windows")]
        if let Some(dir) = path.parent() {
            use windows_strings::HSTRING;
            use windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW;

            if dir != Path::new(".") {
                // Safety: HSTRING::as_ptr returns a valid, NUL-terminated wide
                // string pointer that lives for the duration of this call.
                let code = unsafe { SetDllDirectoryW(HSTRING::from(dir).as_ptr()) };
                if code == 0 {
                    windows_result::HRESULT::from_thread().ok()?;
                }
            }
        }

        maa_sys::binding::load(path)?;
        runtime_change.commit(StableState::Loaded);
        Ok(())
    }

    /// Unload the shared library of the MaaCore.
    ///
    /// Must be called after all assistant instances are destroyed.
    /// Fails if any assistant instances are alive.
    pub fn unload() -> Result<()> {
        let runtime_change = RuntimeChange::begin()?;
        maa_sys::binding::unload();
        runtime_change.commit(StableState::Unloaded);
        Ok(())
    }

    /// Check if the shared library of the MaaCore is currently loaded.
    pub fn loaded() -> bool {
        maa_sys::binding::loaded()
    }
}

pub(crate) struct PendingAssistantRegistration {
    committed: bool,
}

impl PendingAssistantRegistration {
    pub(crate) fn begin() -> Result<Self> {
        loop {
            let state = RUNTIME_STATE.load(Ordering::Acquire);
            if is_locked(state) {
                return Err(Error::RuntimeChanging);
            }

            if !is_loaded(state) {
                return Err(Error::RuntimeNotLoaded);
            }

            let count = assistant_count(state);
            let next_count = count
                .checked_add(1)
                .filter(|count| *count <= ASSISTANT_COUNT_MASK)
                .ok_or(Error::TooManyAssistants)?;
            let next = (state & !ASSISTANT_COUNT_MASK) | next_count;
            if RUNTIME_STATE
                .compare_exchange(state, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(Self { committed: false });
            }
        }
    }

    pub(crate) fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for PendingAssistantRegistration {
    fn drop(&mut self) {
        if !self.committed {
            unregister_assistant();
        }
    }
}

pub(crate) fn unregister_assistant() {
    let previous = RUNTIME_STATE.fetch_sub(1, Ordering::AcqRel);
    debug_assert!(is_loaded(previous));
    debug_assert!(assistant_count(previous) > 0);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    static RUNTIME_STATE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn reset_runtime_state_for_test(state: usize) {
        RUNTIME_STATE.store(state, Ordering::Release);
    }

    #[test]
    fn runtime_change_rejects_active_assistants() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOADED);

        let _registration = PendingAssistantRegistration::begin().unwrap();

        assert!(matches!(
            RuntimeChange::begin(),
            Err(Error::ActiveAssistants)
        ));
    }

    #[test]
    fn assistant_registration_rejects_runtime_changes() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(0);

        let _runtime_change = RuntimeChange::begin().unwrap();

        assert!(matches!(
            PendingAssistantRegistration::begin(),
            Err(Error::RuntimeChanging)
        ));
    }

    #[test]
    fn runtime_change_rejects_existing_runtime_change() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOCKED);

        assert!(matches!(
            RuntimeChange::begin(),
            Err(Error::RuntimeChanging)
        ));
        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOCKED);
    }

    #[test]
    fn load_rejects_existing_runtime_change_before_ffi() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOCKED);

        assert!(matches!(
            Assistant::load("/this/library/does_not_exist.so"),
            Err(Error::RuntimeChanging)
        ));
        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOCKED);
    }

    #[test]
    fn assistant_registration_rejects_unloaded_runtime() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(0);

        assert!(matches!(
            PendingAssistantRegistration::begin(),
            Err(Error::RuntimeNotLoaded)
        ));
    }

    #[test]
    fn assistant_registration_rejects_count_overflow() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOADED | ASSISTANT_COUNT_MASK);

        assert!(matches!(
            PendingAssistantRegistration::begin(),
            Err(Error::TooManyAssistants)
        ));
        assert_eq!(
            RUNTIME_STATE.load(Ordering::Acquire),
            RUNTIME_LOADED | ASSISTANT_COUNT_MASK
        );
    }

    #[test]
    fn runtime_change_restores_state_without_commit() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOADED);

        drop(RuntimeChange::begin().unwrap());

        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOADED);
    }

    #[test]
    fn runtime_change_commits_state() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOADED);

        RuntimeChange::begin()
            .unwrap()
            .commit(StableState::Unloaded);

        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), 0);
    }

    #[test]
    fn runtime_change_commits_loaded_state() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(0);

        RuntimeChange::begin().unwrap().commit(StableState::Loaded);

        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOADED);
    }

    #[test]
    fn runtime_lock_preserves_assistant_count_changes() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOCKED | RUNTIME_LOADED | 1);

        unregister_assistant();
        RUNTIME_STATE.fetch_and(!RUNTIME_LOCKED, Ordering::AcqRel);

        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOADED);
    }

    #[test]
    fn assistant_registration_rolls_back_without_commit() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOADED);

        drop(PendingAssistantRegistration::begin().unwrap());

        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOADED);
    }

    #[test]
    fn assistant_registration_commit_keeps_count() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOADED);

        PendingAssistantRegistration::begin().unwrap().commit();

        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOADED | 1);
        unregister_assistant();
    }

    #[test]
    fn load_restores_state_after_failure() {
        let _state_lock = RUNTIME_STATE_TEST_LOCK.lock().unwrap();
        reset_runtime_state_for_test(RUNTIME_LOADED);

        assert!(Assistant::load("/this/library/does_not_exist.so").is_err());

        assert_eq!(RUNTIME_STATE.load(Ordering::Acquire), RUNTIME_LOADED);
    }
}
