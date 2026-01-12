use std::path::PathBuf;

use maa_value::{MAAValue, Result, object};

#[test]
fn empty_object() {
    let obj = object!();
    assert_eq!(obj, MAAValue::default());
    assert!(obj.as_map().unwrap().is_empty());
}

#[test]
fn basic_insert() {
    let obj = object!(
        "bool" => true,
        "int" => 1,
        "float" => 1.0,
        "string" => "value"
    );

    assert_eq!(obj.get("bool").unwrap().as_bool(), Some(true));
    assert_eq!(obj.get("int").unwrap().as_int(), Some(1));
    assert_eq!(obj.get("float").unwrap().as_float(), Some(1.0));
    assert_eq!(obj.get("string").unwrap().as_str(), Some("value"));
}

#[test]
fn nested_objects() {
    let obj = object!(
        "outer" => object!(
            "inner" => object!(
                "deep" => "value"
            )
        )
    );

    let inner = obj.get("outer").unwrap().get("inner").unwrap();
    assert_eq!(inner.get("deep").unwrap().as_str(), Some("value"));
}

#[test]
fn arrays() {
    let empty_array: [i32; 0] = [];
    let obj = object!(
        "empty" => empty_array,
        "numbers" => [1, 2, 3],
        "strings" => ["a", "b", "c"]
    );

    assert_eq!(obj.get("empty").unwrap(), &MAAValue::Array(vec![]));

    assert_eq!(
        obj.get("numbers").unwrap().as_slice().unwrap(),
        &[1, 2, 3].map(MAAValue::from)
    );

    assert_eq!(
        obj.get("strings").unwrap().as_slice().unwrap(),
        &["a", "b", "c"].map(MAAValue::from)
    );
}

#[test]
fn maybe_insert_some() {
    let some_value: Option<i32> = Some(42);
    let obj = object!(
        "present" =>? some_value
    );

    assert_eq!(obj.get("present").unwrap().as_int(), Some(42));
}

#[test]
fn maybe_insert_none() {
    let none_value: Option<i32> = None;
    let obj = object!(
        "absent" =>? none_value
    );

    assert!(obj.get("absent").is_none());
}

#[test]
fn maybe_insert_mixed() {
    let some_value: Option<i32> = Some(1);
    let none_value: Option<i32> = None;

    let obj = object!(
        "required" => "always here",
        "optional_present" =>? some_value,
        "optional_absent" =>? none_value,
        "another_required" => 99
    );

    assert_eq!(obj.get("required").unwrap().as_str(), Some("always here"));
    assert_eq!(obj.get("optional_present").unwrap().as_int(), Some(1));
    assert!(obj.get("optional_absent").is_none());
    assert_eq!(obj.get("another_required").unwrap().as_int(), Some(99));
}

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
        "conditional" if "flag1" == true, "flag2" == "yes" => "both satisfied"
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
        "conditional" if "flag1" == true, "flag2" == "yes" => "excluded"
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
fn mixed_insert_types() -> Result<()> {
    let some_value: Option<i32> = Some(10);
    let none_value: Option<String> = None;
    let path = PathBuf::from("/path");

    let obj = object!(
        "regular" => "normal",
        "optional_present" =>? some_value,
        "optional_absent" =>? none_value,
        "fallible" => path?,
        "flag" => true,
        "conditional" if "flag" == true => "depends on flag"
    );

    let initialized = obj.init()?;
    assert_eq!(initialized.get("regular").unwrap().as_str(), Some("normal"));
    assert_eq!(
        initialized.get("optional_present").unwrap().as_int(),
        Some(10)
    );
    assert!(initialized.get("optional_absent").is_none());
    assert_eq!(initialized.get("fallible").unwrap().as_str(), Some("/path"));
    assert_eq!(
        initialized.get("conditional").unwrap().as_str(),
        Some("depends on flag")
    );
    Ok(())
}

#[test]
fn trailing_comma() {
    let obj = object!(
        "key1" => "value1",
        "key2" => "value2",
    );

    assert_eq!(obj.get("key1").unwrap().as_str(), Some("value1"));
    assert_eq!(obj.get("key2").unwrap().as_str(), Some("value2"));
}

#[test]
fn no_trailing_comma() {
    let obj = object!(
        "key1" => "value1",
        "key2" => "value2"
    );

    assert_eq!(obj.get("key1").unwrap().as_str(), Some("value1"));
    assert_eq!(obj.get("key2").unwrap().as_str(), Some("value2"));
}

#[test]
fn complex_nested_structure() -> Result<()> {
    let some_val: Option<i32> = Some(100);
    let path = PathBuf::from("/config");

    let obj = object!(
        "metadata" => object!(
            "version" => "1.0",
            "author" => "test"
        ),
        "config" => object!(
            "enabled" => true,
            "path" => path?,
            "optional_setting" =>? some_val,
            "nested" => object!(
                "deep" => "value"
            )
        ),
        "features" => ["feature1", "feature2"],
        "debug" => false,
        "advanced" if "debug" == true => object!(
            "verbose" => true
        )
    );

    let initialized = obj.init()?;

    // Check metadata
    let metadata = initialized.get("metadata").unwrap();
    assert_eq!(metadata.get("version").unwrap().as_str(), Some("1.0"));
    assert_eq!(metadata.get("author").unwrap().as_str(), Some("test"));

    // Check config
    let config = initialized.get("config").unwrap();
    assert_eq!(config.get("enabled").unwrap().as_bool(), Some(true));
    assert_eq!(config.get("path").unwrap().as_str(), Some("/config"));
    assert_eq!(config.get("optional_setting").unwrap().as_int(), Some(100));
    assert_eq!(
        config.get("nested").unwrap().get("deep").unwrap().as_str(),
        Some("value")
    );

    // Check features array
    assert!(initialized.get("features").is_some());

    // Check conditional not included
    assert!(initialized.get("advanced").is_none());

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
fn absolute_crate_path() {
    // This test verifies that the macro uses absolute paths (::maa_value)
    // so it works even when there are local variables with conflicting names

    #[allow(dead_code)]
    struct MAAValue; // Shadow the type name

    let obj = object!(
        "key" => "value"
    );

    // Should still work despite the shadowing
    assert_eq!(obj.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn single_key_value() {
    let obj = object!("key" => "value");
    assert_eq!(obj.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn overwrite_key() {
    let obj = object!(
        "key" => "first",
        "key" => "second"
    );

    // The second value should overwrite the first
    assert_eq!(obj.get("key").unwrap().as_str(), Some("second"));
}

#[test]
fn variable_expressions() {
    let x = 42;
    let s = String::from("hello");
    let arr = vec![1, 2, 3];

    let obj = object!(
        "x" => x,
        "s" => s.clone(),
        "computed" => x * 2,
        "arr" => MAAValue::try_from(arr).unwrap()
    );

    assert_eq!(obj.get("x").unwrap().as_int(), Some(42));
    assert_eq!(obj.get("s").unwrap().as_str(), Some("hello"));
    assert_eq!(obj.get("computed").unwrap().as_int(), Some(84));
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

#[test]
fn empty_string_key() {
    let obj = object!(
        "" => "empty key"
    );

    assert_eq!(obj.get("").unwrap().as_str(), Some("empty key"));
}

#[test]
fn special_characters_in_keys() {
    let obj = object!(
        "key-with-dash" => 1,
        "key.with.dot" => 2,
        "key_with_underscore" => 3,
        "key with spaces" => 4,
        "key:with:colon" => 5
    );

    assert_eq!(obj.get("key-with-dash").unwrap().as_int(), Some(1));
    assert_eq!(obj.get("key.with.dot").unwrap().as_int(), Some(2));
    assert_eq!(obj.get("key_with_underscore").unwrap().as_int(), Some(3));
    assert_eq!(obj.get("key with spaces").unwrap().as_int(), Some(4));
    assert_eq!(obj.get("key:with:colon").unwrap().as_int(), Some(5));
}

#[test]
fn maybe_insert_in_nested() {
    let opt: Option<i32> = Some(5);

    let obj = object!(
        "outer" => object!(
            "inner" =>? opt
        )
    );

    let outer = obj.get("outer").unwrap();
    assert_eq!(outer.get("inner").unwrap().as_int(), Some(5));
}

#[test]
fn all_insert_kinds_together() -> Result<()> {
    let regular = "regular";
    let maybe_val: Option<i32> = Some(1);
    let try_val = PathBuf::from("/path");

    let obj = object!(
        "a_regular" => regular,
        "b_maybe" =>? maybe_val,
        "c_try" => try_val?,
        "d_flag" => true,
        "e_conditional" if "d_flag" == true => "conditional"
    );

    let initialized = obj.init()?;
    assert_eq!(
        initialized.get("a_regular").unwrap().as_str(),
        Some("regular")
    );
    assert_eq!(initialized.get("b_maybe").unwrap().as_int(), Some(1));
    assert_eq!(initialized.get("c_try").unwrap().as_str(), Some("/path"));
    assert_eq!(
        initialized.get("e_conditional").unwrap().as_str(),
        Some("conditional")
    );
    Ok(())
}
