//! Error propagation tests (? operator early return on conversion failure)

use std::path::PathBuf;

use maa_value::{error::Result, prelude::*};

#[cfg(unix)]
pub fn invalid_utf8_path() -> PathBuf {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    PathBuf::from(OsString::from_vec(vec![0xFF, 0xFE, 0xFD]))
}

#[test]
#[cfg(unix)]
fn object_try_conversion_failure_early_return() {
    fn create_object() -> Result<MAAValue> {
        let invalid_path = invalid_utf8_path();

        Ok(object!(
            "before" => "this should not be present",
            "invalid" => invalid_path?,
            "after" => "this should not be present either"
        ))
    }

    let result = create_object();
    assert!(
        result.is_err(),
        "Expected error from invalid UTF-8 conversion"
    );
}

#[test]
#[cfg(unix)]
fn object_try_conversion_failure_multiple() {
    fn create_object() -> Result<MAAValue> {
        let valid_path = PathBuf::from("/valid/path");
        let invalid_path = invalid_utf8_path();

        Ok(object!(
            "valid" => valid_path?,
            "invalid" => invalid_path?,
            "after" => "should not reach here"
        ))
    }

    let result = create_object();
    assert!(
        result.is_err(),
        "Expected error from second invalid conversion"
    );
}

#[test]
#[cfg(unix)]
fn object_maybe_try_conversion_failure_early_return() {
    fn create_object() -> Result<MAAValue> {
        let some_invalid: Option<PathBuf> = Some(invalid_utf8_path());

        Ok(object!(
            "before" => "should not be present",
            "invalid" =>? some_invalid?,
            "after" => "should not be present"
        ))
    }

    let result = create_object();
    assert!(
        result.is_err(),
        "Expected error from =>? with invalid UTF-8"
    );
}

#[test]
#[cfg(unix)]
fn object_conditional_try_conversion_failure() {
    fn create_object() -> Result<MAAValue> {
        let invalid_path = invalid_utf8_path();

        Ok(object!(
            "flag" => true,
            "before" => "should not be present",
            "conditional" if "flag" == true => invalid_path?,
            "after" => "should not be present"
        ))
    }

    let result = create_object();
    assert!(
        result.is_err(),
        "Expected error from conditional with invalid conversion"
    );
}

#[test]
#[cfg(unix)]
fn object_conditional_maybe_try_conversion_failure() {
    fn create_object() -> Result<MAAValue> {
        let some_invalid: Option<PathBuf> = Some(invalid_utf8_path());

        Ok(object!(
            "flag" => true,
            "before" => "should not be present",
            "conditional" if "flag" == true =>? some_invalid?,
            "after" => "should not be present"
        ))
    }

    let result = create_object();
    assert!(
        result.is_err(),
        "Expected error from conditional =>? with invalid conversion"
    );
}

#[test]
#[cfg(unix)]
fn insert_try_conversion_failure_early_return() {
    fn modify_object() -> Result<MAAValue> {
        let mut obj = object!("existing" => "value");
        let invalid_path = invalid_utf8_path();

        insert!(obj,
            "before" => "should not be inserted",
            "invalid" => invalid_path?,
            "after" => "should not be inserted"
        );

        Ok(obj)
    }

    let result = modify_object();
    assert!(
        result.is_err(),
        "Expected error from insert! with invalid conversion"
    );
}

#[test]
#[cfg(unix)]
fn insert_maybe_try_conversion_failure() {
    fn modify_object() -> Result<MAAValue> {
        let mut obj = object!("existing" => "value");
        let some_invalid: Option<PathBuf> = Some(invalid_utf8_path());

        insert!(obj,
            "before" => "should not be inserted",
            "invalid" =>? some_invalid?,
            "after" => "should not be inserted"
        );

        Ok(obj)
    }

    let result = modify_object();
    assert!(
        result.is_err(),
        "Expected error from insert! =>? with invalid conversion"
    );
}

#[test]
#[cfg(unix)]
fn insert_conditional_try_conversion_failure() {
    fn modify_object() -> Result<MAAValue> {
        let mut obj = object!(
            "flag" => true,
            "existing" => "value"
        );
        let invalid_path = invalid_utf8_path();

        insert!(obj,
            "before" => "should not be inserted",
            "conditional" if "flag" == true => invalid_path?,
            "after" => "should not be inserted"
        );

        Ok(obj)
    }

    let result = modify_object();
    assert!(
        result.is_err(),
        "Expected error from insert! conditional with invalid conversion"
    );
}

#[test]
#[cfg(unix)]
fn mixed_success_and_failure() {
    fn create_object() -> Result<MAAValue> {
        let valid1 = PathBuf::from("/valid/one");
        let valid2 = PathBuf::from("/valid/two");
        let invalid = invalid_utf8_path();

        Ok(object!(
            "first" => valid1?,
            "second" => valid2?,
            "third" => invalid?,
            "fourth" => "should never reach"
        ))
    }

    let result = create_object();
    assert!(
        result.is_err(),
        "Expected error when third conversion fails"
    );
}

#[test]
#[cfg(unix)]
fn error_propagation_through_nested_objects() {
    fn create_object() -> Result<MAAValue> {
        let invalid = invalid_utf8_path();

        Ok(object!(
            "outer" => object!(
                "inner" => object!(
                    "invalid" => invalid?
                )
            )
        ))
    }

    let result = create_object();
    assert!(
        result.is_err(),
        "Expected error to propagate through nested objects"
    );
}

#[test]
#[cfg(unix)]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value")]
fn object_conditional_maybe_try_unwrap_conversion_failure() {
    let some_invalid: Option<PathBuf> = Some(invalid_utf8_path());

    let _obj = object!(
        "flag" => true,
        "conditional" if "flag" == true =>? some_invalid??
    );
}

#[test]
fn object_conditional_maybe_try_unwrap_success() {
    let some_valid: Option<PathBuf> = Some(PathBuf::from("/valid/path"));

    let obj = object!(
        "flag" => true,
        "conditional" if "flag" == true =>? some_valid??
    );

    let initialized = obj.resolve().unwrap();
    assert_eq!(
        initialized.get("conditional").unwrap().as_str(),
        Some("/valid/path")
    );
}

#[test]
fn object_conditional_maybe_try_unwrap_none() {
    let none_value: Option<PathBuf> = None;

    let obj = object!(
        "flag" => true,
        "conditional" if "flag" == true =>? none_value??
    );

    let initialized = obj.resolve().unwrap();
    assert!(initialized.get("conditional").is_none());
}

#[test]
#[cfg(unix)]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value")]
fn insert_conditional_maybe_try_unwrap_conversion_failure() {
    let mut obj = object!(
        "flag" => true,
        "base" => "value"
    );
    let some_invalid: Option<PathBuf> = Some(invalid_utf8_path());

    insert!(obj,
        "conditional" if "flag" == true =>? some_invalid??
    );
}

#[test]
fn insert_conditional_maybe_try_unwrap_success() {
    let mut obj = object!(
        "flag" => true,
        "base" => "value"
    );
    let some_valid: Option<PathBuf> = Some(PathBuf::from("/test"));

    insert!(obj,
        "conditional" if "flag" == true =>? some_valid??
    );

    let initialized = obj.resolve().unwrap();
    assert_eq!(
        initialized.get("conditional").unwrap().as_str(),
        Some("/test")
    );
}
