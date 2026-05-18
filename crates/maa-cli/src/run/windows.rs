use anyhow::{Result, bail};
use log::{debug, warn};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM},
    UI::WindowsAndMessaging::{EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible},
};
use windows_sys::core::BOOL;

pub(super) struct WindowMatch {
    pub hwnd: isize,
}

pub(super) fn find_window_by_title(expected_title: &str) -> Result<WindowMatch> {
    let mut state = EnumState::new(expected_title);

    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            &mut state as *mut EnumState<'_> as LPARAM,
        );
    }

    if state.exact_matches.is_empty() {
        let related_titles = if state.related_titles.is_empty() {
            String::new()
        } else {
            format!(
                " Related visible windows: {}.",
                state.related_titles.join(", ")
            )
        };

        bail!(
            "No visible window found with exact title `{expected_title}`. \
Start the PC client first or override `connection.window_title`.{related_titles}"
        );
    }

    let matched = &state.exact_matches;
    if matched.len() > 1 {
        warn!(
            "Found {} visible windows titled `{}`; using the first one (HWND=0x{:X})",
            matched.len(),
            expected_title,
            matched[0].hwnd as usize,
        );
    } else {
        debug!(
            "Found visible window `{}` (HWND=0x{:X})",
            expected_title, matched[0].hwnd as usize,
        );
    }

    Ok(WindowMatch {
        hwnd: matched[0].hwnd,
    })
}

struct EnumState<'a> {
    expected_title: &'a str,
    exact_matches: Vec<WindowMatch>,
    related_titles: Vec<String>,
}

impl<'a> EnumState<'a> {
    fn new(expected_title: &'a str) -> Self {
        Self {
            expected_title,
            exact_matches: Vec::new(),
            related_titles: Vec::new(),
        }
    }
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = unsafe { &mut *(lparam as *mut EnumState<'_>) };

    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }

    let Some(title) = window_title(hwnd) else {
        return 1;
    };

    if title == state.expected_title {
        state.exact_matches.push(WindowMatch {
            hwnd: hwnd as isize,
        });
    } else if is_related_title(&title) && state.related_titles.len() < 8 {
        state.related_titles.push(format!("`{title}`"));
    }

    1
}

fn window_title(hwnd: HWND) -> Option<String> {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return None;
    }

    let mut buffer = vec![0u16; len as usize + 1];
    let written = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if written <= 0 {
        return None;
    }

    let title = String::from_utf16_lossy(&buffer[..written as usize]);
    (!title.trim().is_empty()).then_some(title)
}

fn is_related_title(title: &str) -> bool {
    title.contains("Arknights")
        || title.contains("明日")
        || title.contains("方舟")
        || title.contains("bilibili游戏")
}
