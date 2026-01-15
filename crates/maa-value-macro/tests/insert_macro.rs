//! insert! macro tests

use std::path::PathBuf;

use maa_value::{error::Result, prelude::*};

#[test]
fn insert_basic() {
    let mut obj = object!("existing" => "value");
    insert!(obj, "new_key" => "new_value");

    assert_eq!(obj.get("existing").unwrap().as_str(), Some("value"));
    assert_eq!(obj.get("new_key").unwrap().as_str(), Some("new_value"));
}

#[test]
fn insert_multiple() {
    let mut obj = object!("key1" => 1);
    insert!(obj,
        "key2" => 2,
        "key3" => "three",
        "key4" => true
    );

    assert_eq!(obj.get("key1").unwrap().as_int(), Some(1));
    assert_eq!(obj.get("key2").unwrap().as_int(), Some(2));
    assert_eq!(obj.get("key3").unwrap().as_str(), Some("three"));
    assert_eq!(obj.get("key4").unwrap().as_bool(), Some(true));
}

#[test]
fn insert_with_try() -> Result<()> {
    let mut obj = object!("base" => "value");
    let path = PathBuf::from("/test/path");
    insert!(obj, "path" => path?);

    assert_eq!(obj.get("path").unwrap().as_str(), Some("/test/path"));
    Ok(())
}

#[test]
fn insert_with_try_unwrap() {
    let mut obj = object!("base" => "value");
    let path = PathBuf::from("/test/path");
    insert!(obj, "path" => path??);

    assert_eq!(obj.get("path").unwrap().as_str(), Some("/test/path"));
}

#[test]
fn insert_maybe() {
    let mut obj = object!("base" => "value");
    let some_value: Option<i32> = Some(42);
    let none_value: Option<i32> = None;

    insert!(obj,
        "some" =>? some_value,
        "none" =>? none_value
    );

    assert_eq!(obj.get("some").unwrap().as_int(), Some(42));
    assert!(obj.get("none").is_none());
}

#[test]
fn insert_maybe_with_try() -> Result<()> {
    let mut obj = object!("base" => "value");
    let some_path: Option<PathBuf> = Some(PathBuf::from("/test"));
    let none_path: Option<PathBuf> = None;

    insert!(obj,
        "present" =>? some_path?,
        "absent" =>? none_path?
    );

    assert_eq!(obj.get("present").unwrap().as_str(), Some("/test"));
    assert!(obj.get("absent").is_none());
    Ok(())
}

#[test]
fn insert_conditional() {
    let mut obj = object!(
        "flag" => true,
        "base" => "value"
    );

    insert!(obj,
        "conditional" if "flag" == true => "inserted"
    );

    let initialized = obj.resolve().unwrap();
    assert_eq!(
        initialized.get("conditional").unwrap().as_str(),
        Some("inserted")
    );
}

#[test]
fn insert_conditional_multiple_conditions() {
    let mut obj = object!(
        "flag1" => true,
        "flag2" => "yes",
        "base" => "value"
    );

    insert!(obj,
        "conditional" if "flag1" == true && "flag2" == "yes" => "both satisfied"
    );

    let initialized = obj.resolve().unwrap();
    assert_eq!(
        initialized.get("conditional").unwrap().as_str(),
        Some("both satisfied")
    );
}

#[test]
fn insert_overwrite() {
    let mut obj = object!("key" => "old");
    insert!(obj, "key" => "new");

    assert_eq!(obj.get("key").unwrap().as_str(), Some("new"));
}

#[test]
fn insert_mixed_operations() -> Result<()> {
    let mut obj = object!("base" => "value");
    let path = PathBuf::from("/path");
    let optional: Option<i32> = Some(10);

    insert!(obj,
        "regular" => "test",
        "try" => path?,
        "maybe" =>? optional,
        "try_unwrap" => PathBuf::from("/another")??,
    );

    assert_eq!(obj.get("regular").unwrap().as_str(), Some("test"));
    assert_eq!(obj.get("try").unwrap().as_str(), Some("/path"));
    assert_eq!(obj.get("maybe").unwrap().as_int(), Some(10));
    assert_eq!(obj.get("try_unwrap").unwrap().as_str(), Some("/another"));
    Ok(())
}
