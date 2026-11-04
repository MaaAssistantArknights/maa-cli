use std::path::PathBuf;

use super::window::{WindowCandidate, select_window, validate_win32_control_unit_at};
use crate::config::asst::WindowSelector;

fn candidate(
    handle: isize,
    title: &str,
    process_id: u32,
    executable: Option<&str>,
) -> WindowCandidate {
    WindowCandidate {
        handle,
        title: title.to_owned(),
        process_id,
        executable: executable.map(PathBuf::from),
        visible: true,
    }
}

#[test]
fn selects_only_an_exact_visible_title() {
    let selector = WindowSelector {
        title: "Arknights",
        process_id: None,
        executable: None,
    };
    let mut hidden = candidate(1, "Arknights", 100, None);
    hidden.visible = false;
    let candidates = vec![
        hidden,
        candidate(2, "Arknights ", 101, None),
        candidate(3, "Arknights", 102, None),
    ];

    assert_eq!(select_window(&selector, &candidates).unwrap().get(), 3);
}

#[test]
fn optional_pid_disambiguates_matching_titles() {
    let selector = WindowSelector {
        title: "Arknights",
        process_id: Some(102),
        executable: None,
    };
    let candidates = vec![
        candidate(1, "Arknights", 101, None),
        candidate(2, "Arknights", 102, None),
    ];

    assert_eq!(select_window(&selector, &candidates).unwrap().get(), 2);
}

#[test]
fn optional_canonical_executable_disambiguates_matching_titles() {
    let root = tempfile::tempdir().unwrap();
    let expected_dir = root.path().join("expected");
    let other_dir = root.path().join("other");
    std::fs::create_dir_all(&expected_dir).unwrap();
    std::fs::create_dir_all(&other_dir).unwrap();
    let expected_executable = expected_dir.join("Arknights.exe");
    let other_executable = other_dir.join("Arknights.exe");
    std::fs::write(&expected_executable, []).unwrap();
    std::fs::write(&other_executable, []).unwrap();
    let selector = WindowSelector {
        title: "Arknights",
        process_id: None,
        executable: Some(&expected_executable),
    };
    let candidates = vec![
        candidate(1, "Arknights", 101, other_executable.to_str()),
        candidate(2, "Arknights", 102, expected_executable.to_str()),
    ];

    assert_eq!(select_window(&selector, &candidates).unwrap().get(), 2);
}

#[test]
fn rejects_ambiguous_visible_windows() {
    let selector = WindowSelector {
        title: "Arknights",
        process_id: None,
        executable: None,
    };
    let candidates = vec![
        candidate(1, "Arknights", 101, None),
        candidate(2, "Arknights", 102, None),
    ];

    assert!(
        select_window(&selector, &candidates)
            .unwrap_err()
            .to_string()
            .contains("Multiple visible top-level windows")
    );
}

#[test]
fn rejects_missing_window() {
    let selector = WindowSelector {
        title: "Arknights",
        process_id: None,
        executable: None,
    };

    assert!(
        select_window(&selector, &[])
            .unwrap_err()
            .to_string()
            .contains("No visible top-level window")
    );
}

#[test]
fn win32_control_unit_capability_requires_the_dll() {
    let dir = tempfile::tempdir().unwrap();
    let error = validate_win32_control_unit_at(Some(dir.path())).unwrap_err();
    assert!(error.to_string().contains("MaaWin32ControlUnit.dll"));

    std::fs::write(dir.path().join("MaaWin32ControlUnit.dll"), []).unwrap();
    validate_win32_control_unit_at(Some(dir.path())).unwrap();
}

#[cfg(not(windows))]
#[test]
fn win32_connection_reports_a_clear_platform_error() {
    let selector = WindowSelector {
        title: "Arknights",
        process_id: None,
        executable: None,
    };

    assert_eq!(
        super::window::resolve_window(&selector)
            .unwrap_err()
            .to_string(),
        "Win32 connection is only supported on Windows"
    );
}
