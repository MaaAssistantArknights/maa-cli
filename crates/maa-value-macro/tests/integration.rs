//! Complex integration tests

use std::path::PathBuf;

use maa_value::{Result, object};

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
