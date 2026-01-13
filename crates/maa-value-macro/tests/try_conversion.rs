//! Try conversion tests (? and ?? operators for fallible conversions)

use std::path::PathBuf;

use maa_value::{Result, object};

#[test]
fn try_insert_success() -> Result<()> {
    let path = PathBuf::from("/valid/utf8/path");

    let obj = object!("path" => path?);

    assert_eq!(obj.get("path").unwrap().as_str(), Some("/valid/utf8/path"));
    Ok(())
}

#[test]
fn try_insert_multiple() -> Result<()> {
    let path1 = PathBuf::from("/path/one");
    let path2 = PathBuf::from("/path/two");

    let obj = object!(
        "path1" => path1?,
        "regular" => "value",
        "path2" => path2?
    );

    assert_eq!(obj.get("path1").unwrap().as_str(), Some("/path/one"));
    assert_eq!(obj.get("regular").unwrap().as_str(), Some("value"));
    assert_eq!(obj.get("path2").unwrap().as_str(), Some("/path/two"));
    Ok(())
}

#[test]
fn try_insert_unwrap() {
    let path = PathBuf::from("/valid/utf8/path");

    // This should work without requiring Result return type
    let obj = object!("path" => path??);

    assert_eq!(obj.get("path").unwrap().as_str(), Some("/valid/utf8/path"));
}

#[test]
fn try_insert_unwrap_multiple() {
    let path1 = PathBuf::from("/path/one");
    let path2 = PathBuf::from("/path/two");

    let obj = object!(
        "path1" => path1??,
        "regular" => "value",
        "path2" => path2??
    );

    assert_eq!(obj.get("path1").unwrap().as_str(), Some("/path/one"));
    assert_eq!(obj.get("regular").unwrap().as_str(), Some("value"));
    assert_eq!(obj.get("path2").unwrap().as_str(), Some("/path/two"));
}

#[test]
fn maybe_with_try() -> Result<()> {
    let some_path: Option<PathBuf> = Some(PathBuf::from("/test/path"));
    let none_path: Option<PathBuf> = None;

    let obj = object!(
        "present" =>? some_path?,
        "absent" =>? none_path?
    );

    assert_eq!(obj.get("present").unwrap().as_str(), Some("/test/path"));
    assert!(obj.get("absent").is_none());
    Ok(())
}

#[test]
fn maybe_with_try_unwrap() {
    let some_path: Option<PathBuf> = Some(PathBuf::from("/test/path"));
    let none_path: Option<PathBuf> = None;

    let obj = object!(
        "present" =>? some_path??,
        "absent" =>? none_path??
    );

    assert_eq!(obj.get("present").unwrap().as_str(), Some("/test/path"));
    assert!(obj.get("absent").is_none());
}
