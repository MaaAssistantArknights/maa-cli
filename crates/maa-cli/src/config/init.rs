use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::{
    object,
    value::{
        userinput::{BoolInput, Input, SelectD, ValueWithDesc},
        MAAValue,
    },
};

fn asst_config_template() -> MAAValue {
    object!(
        "setup_connection" => BoolInput::new(Some(true), Some("setup connection")),
        "connection_config" if "setup_connection" == true => object!(
            "preset" => SelectD::<String>::new(
                ["MuMuPro", "PlayCover", "ADB"],
                Some(3),
                Some("connection preset"),
                false
            ).unwrap(),
            "adb_path" if "preset" == "ADB" => Input::<String>::new(
                Some(String::from("adb")),
                Some("adb path"),
            ),
            "address" => Input::<String>::new(
                Some(String::from("auto")),
                Some("address to connect"),
            ),
            "config" => Input::<String>::new(
                Some(String::from("auto")),
                Some("configuration name to connect (auto for most cases)"),
            ),
        ),
        "setup_instance_options" => BoolInput::new(Some(true), Some("setup instance options")),
        "instance_options" if "setup_instance_options" == true => object!(
            "touch_mode" => SelectD::<String>::new(
                [
                    ValueWithDesc::new(
                        "ADB",
                        Some("most compatible but slow"),
                    ),
                    ValueWithDesc::new(
                        "MiniTouch",
                        Some("faster but may not work on some devices"),
                    ),
                    ValueWithDesc::new(
                        "MaaTouch",
                        Some("rewrite of MiniTouch, fast and compatible with most devices"),
                    ),
                    ValueWithDesc::new(
                        "MacPlayTools",
                        Some("this if and only if you are connecting to PlayCover"),
                    ),
                ],
                Some(3),
                Some("touch mode"),
                false
            ).unwrap(),
            "deployment_with_pause" => BoolInput::new(
                Some(false),
                Some("deploy operator with pause"),
            ),
            "adb_lite_enabled" => BoolInput::new(
                Some(false),
                Some("enable ADB Lite (a lightweight ADB implementation)"),
            ),
            "kill_adb_on_exit" => BoolInput::new(
                Some(false),
                Some("kill ADB server on exit"),
            ),
        ),
        // most of cases don't need to setup resource
        "setup_resource" => BoolInput::new(
            Some(false),
            Some("setup resource configurations (don't setup it for most cases)"),
        ),
        "resource_config" if "setup_resource" == true => object!(
            "global_resource" => SelectD::<String>::new(
                [
                    ValueWithDesc::new(
                        "None",
                        Some("no global resource needed by Official and BiliBili client"),
                    ),
                    ValueWithDesc::new(
                        "YostarJP",
                        Some("resource fror Japanese client"),
                    ),
                    ValueWithDesc::new(
                        "YostarKR",
                        Some("resource for Korean client"),
                    ),
                    ValueWithDesc::new(
                        "YostarEN",
                        Some("resource for English client"),
                    ),
                    ValueWithDesc::new(
                        "Txwy",
                        Some("resource for Traditional Chinese client"),
                    ),
                ],
                Some(1),
                Some("global resource to load"),
                false
            ).unwrap(),
            "platform_diff_resource" => SelectD::<String>::new(
                [
                    ValueWithDesc::new(
                        "None",
                        Some("no platform different resource needed by Android client"),
                    ),
                    ValueWithDesc::new(
                        "iOS",
                        Some("resource for PlayCover which run iOS client on macOS"),
                    ),
                ],
                Some(1),
                Some("platform different resource to load"),
                false
            ).unwrap(),
            "user_resource" => BoolInput::new(
                Some(false),
                Some("load custom resource from user configuration directory"),
            ),
        ),
        // most of cases don't need to setup static options
        "setup_static_options" => BoolInput::new(
            Some(false),
            Some("setup static options (for hardware acceleration)"),
        ),
        "static_options" if "setup_static_options" == true => object!(
            "cpu_ocr" => BoolInput::new(Some(true), Some("use CPU for OCR")),
            "gpu_ocr" if "cpu_ocr" == false => Input::<i32>::new(
                None,
                Some("GPU device ID for OCR (make sure your MAA Core supports GPU OCR)"),
            ),
        ),
    )
}

pub fn init(name: Option<PathBuf>, filetype: Option<super::Filetype>, force: bool) -> Result<()> {
    let name = name.unwrap_or_else(|| PathBuf::from("default"));
    let filetype = filetype.unwrap_or(super::Filetype::Json);
    let profile_dir = join!(crate::dirs::config(), "profiles");
    let dest = join!(&profile_dir, &name; filetype.to_str());

    // check if profiles with same name already exists
    let mut tobe_removed = Vec::new();

    if profile_dir.exists() {
        for ext in super::SUPPORTED_EXTENSION.iter() {
            let path = dest.with_extension(ext);
            if path.exists() {
                if force {
                    if path != dest {
                        tobe_removed.push(path);
                    }
                } else {
                    bail!(
                    "profile `{}` already exists or use another name to create a new profile or use --force to overwrite the existing profile.",
                    name.display()
                );
                }
            }
        }
    } else {
        std::fs::create_dir_all(&profile_dir)?;
    }

    let asst_config = asst_config_template().init()?;
    let mut asst_config_out = object!();
    if let Some(obj) = asst_config.get("connection_config") {
        let mut config = object!(
            "preset" => obj.get("preset").unwrap().to_owned(),
        );

        if let Some(adb_path) = obj.get("adb_path") {
            let adb_path = adb_path.as_str().unwrap();
            match adb_path {
                "adb" => {} // default value
                x => config.insert("adb_path", x),
            };
        }

        match obj.get("address").unwrap().as_str().unwrap() {
            "auto" => {}
            x => config.insert("address", x),
        };
        match obj.get("config").unwrap().as_str().unwrap() {
            "auto" => {}
            x => config.insert("config", x),
        };
        asst_config_out.insert("connection", config);
    }

    // no additional processing needed
    if let Some(obj) = asst_config.get("instance_options") {
        asst_config_out.insert("instance_options", obj.to_owned());
    }

    if let Some(obj) = asst_config.get("resource_config") {
        let mut config = object!(
            "user_resource" => obj.get("user_resource").unwrap().to_owned(),
        );
        match obj.get("global_resource").unwrap().as_str().unwrap() {
            "None" => {}
            x => config.insert("global_resource", x),
        };
        match obj.get("platform_diff_resource").unwrap().as_str().unwrap() {
            "None" => {}
            x => config.insert("platform_diff_resource", x),
        };
        asst_config_out.insert("resource", config);
    }

    if let Some(obj) = asst_config.get("static_options") {
        let mut config = object!(
            "cpu_ocr" => obj.get("cpu_ocr").unwrap().to_owned(),
        );
        if let Some(gpu_ocr) = obj.get("gpu_ocr") {
            config.insert("gpu_ocr", gpu_ocr.to_owned());
        }
        asst_config_out.insert("static_options", config);
    }

    filetype.write(std::fs::File::create(dest)?, &asst_config_out)?;

    // remove same name profiles
    for path in tobe_removed {
        std::fs::remove_file(path)?;
    }

    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use super::{super::Filetype, *};

    #[test]
    #[ignore = "write to user's config directory"]
    fn test_init() {
        let profile_dir = join!(crate::dirs::config(), "profiles");
        let name = PathBuf::from("test");

        init(Some(name.clone()), None, false).expect("failed to init profile");
        assert!(join!(&profile_dir, "test"; "json").exists());

        assert!(init(Some(name.clone()), None, false).is_err());
        init(Some(name.clone()), Some(Filetype::Toml), true).expect("failed to init profile");
        assert!(join!(&profile_dir, "test"; "toml").exists());
    }
}
