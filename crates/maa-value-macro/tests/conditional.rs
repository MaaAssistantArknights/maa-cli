//! Conditional insert tests (if "key" == value syntax)

use std::path::PathBuf;

use maa_value::{MAAValue, Result, object};

#[test]
fn single_condition() {
    let obj = object!(
        "flag" => true,
        "conditional" if "flag" == true => "included"
    );

    // Before init, conditional should be Optional variant
    assert!(matches!(
        obj.get("conditional"),
        Some(MAAValue::Optional { .. })
    ));

    let initialized = obj.init().unwrap();
    assert_eq!(
        initialized.get("conditional").unwrap().as_str(),
        Some("included")
    );
}

#[test]
fn condition_not_satisfied() {
    let obj = object!(
        "flag" => false,
        "conditional" if "flag" == true => "excluded"
    );

    let initialized = obj.init().unwrap();
    assert!(initialized.get("conditional").is_none());
}

#[test]
fn condition_key_not_exist() {
    let obj = object!(
        "conditional" if "nonexistent" == true => "excluded"
    );

    let initialized = obj.init().unwrap();
    assert!(initialized.get("conditional").is_none());
}

#[test]
fn multiple_conditions() {
    let obj = object!(
        "flag1" => true,
        "flag2" => "yes",
        "conditional" if "flag1" == true && "flag2" == "yes" => "both satisfied"
    );

    let initialized = obj.init().unwrap();
    assert_eq!(
        initialized.get("conditional").unwrap().as_str(),
        Some("both satisfied")
    );
}

#[test]
fn multiple_conditions_one_fails() {
    let obj = object!(
        "flag1" => true,
        "flag2" => "no",
        "conditional" if "flag1" == true && "flag2" == "yes" => "excluded"
    );

    let initialized = obj.init().unwrap();
    assert!(initialized.get("conditional").is_none());
}

#[test]
fn chained_conditions() {
    let obj = object!(
        "base" => true,
        "level1" if "base" == true => 1,
        "level2" if "level1" == 1 => 2,
        "level3" if "level2" == 2 => 3
    );

    let initialized = obj.init().unwrap();
    assert_eq!(initialized.get("base").unwrap().as_bool(), Some(true));
    assert_eq!(initialized.get("level1").unwrap().as_int(), Some(1));
    assert_eq!(initialized.get("level2").unwrap().as_int(), Some(2));
    assert_eq!(initialized.get("level3").unwrap().as_int(), Some(3));
}

#[test]
fn chained_conditions_break() {
    let obj = object!(
        "base" => false,
        "level1" if "base" == true => 1,
        "level2" if "level1" == 1 => 2,
        "level3" if "level2" == 2 => 3
    );

    let initialized = obj.init().unwrap();
    assert_eq!(initialized.get("base").unwrap().as_bool(), Some(false));
    assert!(initialized.get("level1").is_none());
    assert!(initialized.get("level2").is_none());
    assert!(initialized.get("level3").is_none());
}

#[test]
fn conditional_with_nested_object() {
    let obj = object!(
        "enable_nested" => true,
        "nested_obj" if "enable_nested" == true => object!(
            "key1" => "value1",
            "key2" => "value2"
        )
    );

    let initialized = obj.init().unwrap();
    let nested = initialized.get("nested_obj").unwrap();
    assert_eq!(nested.get("key1").unwrap().as_str(), Some("value1"));
    assert_eq!(nested.get("key2").unwrap().as_str(), Some("value2"));
}

#[test]
fn conditional_maybe_insert() {
    let some_value: Option<i32> = Some(42);
    let none_value: Option<i32> = None;

    let obj = object!(
        "flag" => true,
        "cond_some" if "flag" == true =>? some_value,
        "cond_none" if "flag" == true =>? none_value
    );

    let initialized = obj.init().unwrap();
    assert_eq!(initialized.get("cond_some").unwrap().as_int(), Some(42));
    assert!(initialized.get("cond_none").is_none());
}

#[test]
fn conditional_try_insert() -> Result<()> {
    let path = PathBuf::from("/test/path");

    let obj = object!(
        "flag" => true,
        "cond_path" if "flag" == true => path?
    );

    let initialized = obj.init()?;
    assert_eq!(
        initialized.get("cond_path").unwrap().as_str(),
        Some("/test/path")
    );
    Ok(())
}

#[test]
fn conditional_try_insert_multiple() -> Result<()> {
    let path1 = PathBuf::from("/path/one");
    let path2 = PathBuf::from("/path/two");

    let obj = object!(
        "flag1" => true,
        "flag2" => false,
        "cond_path1" if "flag1" == true => path1?,
        "cond_path2" if "flag2" == true => path2?
    );

    let initialized = obj.init()?;
    // cond_path1 should be present (condition satisfied)
    assert_eq!(
        initialized.get("cond_path1").unwrap().as_str(),
        Some("/path/one")
    );
    // cond_path2 should be absent (condition not satisfied)
    assert!(initialized.get("cond_path2").is_none());
    Ok(())
}

#[test]
fn condition_with_different_types() {
    let obj = object!(
        "string_key" => "expected",
        "int_key" => 42,
        "bool_key" => true,
        "cond_string" if "string_key" == "expected" => "string matched",
        "cond_int" if "int_key" == 42 => "int matched",
        "cond_bool" if "bool_key" == true => "bool matched"
    );

    let initialized = obj.init().unwrap();
    assert_eq!(
        initialized.get("cond_string").unwrap().as_str(),
        Some("string matched")
    );
    assert_eq!(
        initialized.get("cond_int").unwrap().as_str(),
        Some("int matched")
    );
    assert_eq!(
        initialized.get("cond_bool").unwrap().as_str(),
        Some("bool matched")
    );
}

#[test]
fn conditional_order_independence() {
    // Conditions should work regardless of declaration order
    let obj = object!(
        "depends_on_flag" if "flag" == true => "yes",
        "flag" => true
    );

    let initialized = obj.init().unwrap();
    assert_eq!(
        initialized.get("depends_on_flag").unwrap().as_str(),
        Some("yes")
    );
}
