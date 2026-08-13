#[cfg(any(windows, test))]
use std::{
    num::NonZeroIsize,
    path::{Path, PathBuf},
};

#[cfg(any(windows, test))]
use anyhow::Context;
use anyhow::{Result, bail};

use crate::config::asst::WindowSelector;

#[cfg(any(windows, test))]
const WIN32_CONTROL_UNIT: &str = "MaaWin32ControlUnit.dll";

#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WindowCandidate {
    pub handle: isize,
    pub title: String,
    pub process_id: u32,
    pub executable: Option<PathBuf>,
    pub visible: bool,
}

#[cfg(any(windows, test))]
pub(super) fn select_window(
    selector: &WindowSelector<'_>,
    candidates: &[WindowCandidate],
) -> Result<NonZeroIsize> {
    let expected_executable = selector
        .executable
        .map(canonical_path)
        .transpose()
        .context("Failed to canonicalize configured window_executable")?;
    let matches: Vec<_> = candidates
        .iter()
        .filter(|candidate| candidate.visible && candidate.title == selector.title)
        .filter(|candidate| {
            selector
                .process_id
                .is_none_or(|process_id| candidate.process_id == process_id)
        })
        .filter(|candidate| match &expected_executable {
            Some(expected) => candidate
                .executable
                .as_deref()
                .and_then(|path| canonical_path(path).ok())
                .as_ref()
                .is_some_and(|path| paths_equal(path, expected)),
            None => true,
        })
        .collect();

    match matches.as_slice() {
        [] => bail!(
            "No visible top-level window exactly matched title {:?}",
            selector.title
        ),
        [candidate] => {
            NonZeroIsize::new(candidate.handle).context("Matched window has a null handle")
        }
        _ => bail!(
            "Multiple visible top-level windows exactly matched title {:?}; set window_process_id or window_executable",
            selector.title
        ),
    }
}

#[cfg(any(windows, test))]
fn canonical_path(path: &Path) -> std::io::Result<PathBuf> {
    dunce::canonicalize(path)
}

#[cfg(any(windows, test))]
fn paths_equal(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(any(windows, test))]
pub(super) fn validate_win32_control_unit_at(library_dir: Option<&Path>) -> Result<()> {
    let directory = library_dir.context("MaaCore library directory could not be located")?;
    let control_unit = directory.join(WIN32_CONTROL_UNIT);
    if !control_unit.is_file() {
        bail!(
            "Win32 connection requires {} beside MaaCore.dll (not found at {})",
            WIN32_CONTROL_UNIT,
            control_unit.display()
        );
    }
    Ok(())
}

#[cfg(windows)]
mod platform {
    use std::ptr;

    use anyhow::{Context, Result};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HWND, LPARAM},
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
            IsWindowVisible,
        },
    };

    use super::WindowCandidate;

    unsafe extern "system" fn collect_window(hwnd: HWND, state: LPARAM) -> i32 {
        let candidates = unsafe { &mut *(state as *mut Vec<WindowCandidate>) };
        let title_length = unsafe { GetWindowTextLengthW(hwnd) };
        if title_length <= 0 {
            return 1;
        }
        let mut title = vec![0u16; title_length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
        if copied <= 0 {
            return 1;
        }
        let mut process_id = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
        candidates.push(WindowCandidate {
            handle: hwnd as isize,
            title: String::from_utf16_lossy(&title[..copied as usize]),
            process_id,
            executable: process_executable(process_id),
            visible: unsafe { IsWindowVisible(hwnd) } != 0,
        });
        1
    }

    fn process_executable(process_id: u32) -> Option<std::path::PathBuf> {
        if process_id == 0 {
            return None;
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return None;
        }
        let mut path = vec![0u16; 32_768];
        let mut length = path.len() as u32;
        let queried =
            unsafe { QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) };
        unsafe { CloseHandle(process) };
        (queried != 0)
            .then(|| std::path::PathBuf::from(String::from_utf16_lossy(&path[..length as usize])))
    }

    pub(super) fn enumerate() -> Result<Vec<WindowCandidate>> {
        let mut candidates = Vec::new();
        let result = unsafe {
            EnumWindows(
                Some(collect_window),
                ptr::from_mut(&mut candidates) as LPARAM,
            )
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error())
                .context("Failed to enumerate top-level windows");
        }
        Ok(candidates)
    }
}

#[cfg(windows)]
pub(super) fn resolve_window(selector: &WindowSelector<'_>) -> Result<maa_core::WindowHandle> {
    let candidates = platform::enumerate()?;
    let handle = select_window(selector, &candidates)?;
    maa_core::WindowHandle::new(handle.get() as *mut std::ffi::c_void)
        .context("Matched window has a null handle")
}

#[cfg(not(windows))]
pub(super) fn resolve_window(_selector: &WindowSelector<'_>) -> Result<()> {
    bail!("Win32 connection is only supported on Windows")
}
