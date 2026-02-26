//! Basic object! macro functionality tests

use maa_value::{prelude::*, primitive::Int};

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
    let empty_array: [Int; 0] = [];
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
