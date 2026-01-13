//! Maybe insert tests (=>? operator)

use maa_value::object;

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
