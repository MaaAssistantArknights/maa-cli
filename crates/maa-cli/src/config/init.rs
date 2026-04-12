use std::path::Path;

use anyhow::{Result, bail};
use maa_question::prelude::*;
use maa_value::prelude::*;

fn asst_config_template() -> MAAValueTemplate {
    template!(
        "setup_connection" => Confirm::new(true).with_description("setup connection"),
        "connection_config" if "setup_connection" == true => template!(
            "preset" => SelectD::<String>::from_iter(
                [
                    ValueWithDesc::new(
                        "MuMuPro",
                        None,
                    ),
                    ValueWithDesc::new(
                        "PlayCover",
                        Some("macOS"),
                    ),
                    ValueWithDesc::new(
                        "Waydroid",
                        Some("Linux"),
                    ),
                    ValueWithDesc::new(
                        "ADB",
                        None,
                    ),
                ],
                std::num::NonZero::new(4).unwrap(),
            ).unwrap()
            .with_description("connection preset"),
            "adb_path" if "preset" == "ADB" => Inquiry::<String>::new(
                String::from("adb"),
            ).with_description("adb path"),
            "address" => Inquiry::<String>::new(
                String::from("auto"),
            ).with_description("address to connect"),
            "config" => Inquiry::<String>::new(
                String::from("auto"),
            ).with_description("configuration name to connect (auto for most cases)"),
        ),
        "setup_instance_options" => Confirm::new(true).with_description("setup instance options"),
        "instance_options" if "setup_instance_options" == true => template!(
            "touch_mode" => SelectD::<String>::from_iter(
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
                    ValueWithDesc::new(
                        "MaaFwAdb",
                        Some("use MaaFramework ADB controller for emulator extras support"),
                    ),
                ],
                std::num::NonZero::new(3).unwrap(),
            ).unwrap()
            .with_description("touch mode"),
            "deployment_with_pause" => Confirm::new(
                false,
            ).with_description("deploy operator with pause"),
            "adb_lite_enabled" => Confirm::new(
                false,
            ).with_description("enable ADB Lite (a lightweight ADB implementation)"),
            "kill_adb_on_exit" => Confirm::new(
                false,
            ).with_description("kill ADB server on exit"),
        ),
        // most of cases don't need to setup resource
        "setup_resource" => Confirm::new(
            false,
        ).with_description("setup resource configurations (don't setup it for most cases)"),
        "resource_config" if "setup_resource" == true => template!(
            "global_resource" => SelectD::<String>::from_iter(
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
                        "txwy",
                        Some("resource for Traditional Chinese client"),
                    ),
                ],
                std::num::NonZero::new(1).unwrap(),
            ).unwrap().with_description("global resource to load"),
            "platform_diff_resource" => SelectD::<String>::from_iter(
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
                std::num::NonZero::new(1).unwrap(),
            ).unwrap().with_description("platform different resource to load"),
            "user_resource" => Confirm::new(false)
                .with_description("load custom resource from user configuration directory"),
        ),
        // most of cases don't need to setup static options
        "setup_static_options" => Confirm::new(false)
            .with_description("setup static options (for hardware acceleration)"),
        "static_options" if "setup_static_options" == true => template!(
            "cpu_ocr" => Confirm::new(true).with_description("use CPU for OCR"),
            "gpu_ocr" if "cpu_ocr" == false => Inquiry::<i32>::new(0)
                .with_description("GPU device ID for OCR (make sure your MAA Core supports GPU OCR)"),
        ),
    )
}

pub fn init(name: Option<&Path>, filetype: Option<super::Filetype>, force: bool) -> Result<()> {
    let name = name.unwrap_or_else(|| Path::new("default"));
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

    // TODO: better logic to handle the template
    let asst_config = crate::resolver::with_global_resolver(|resolver| {
        asst_config_template().resolved_by(resolver)
    })?;
    let mut asst_config_out = MAAValue::default();
    if let Some(obj) = asst_config.get("connection_config") {
        let mut config = MAAValue::default();
        insert!(config, "preset" => obj.get("preset").unwrap().to_owned());

        if let Some(adb_path) = obj.get("adb_path") {
            let adb_path = adb_path.as_str().unwrap();
            match adb_path {
                "adb" => {} // default value
                x => insert!(config, "adb_path" => x),
            };
        }

        match obj.get("address").unwrap().as_str().unwrap() {
            "auto" => {}
            x => insert!(config, "address" => x),
        };
        match obj.get("config").unwrap().as_str().unwrap() {
            "auto" => {}
            x => insert!(config, "config" => x),
        };
        insert!(asst_config_out, "connection" => config);
    }

    // no additional processing needed
    if let Some(obj) = asst_config.get("instance_options") {
        asst_config_out.insert("instance_options", obj.to_owned());
    }

    if let Some(obj) = asst_config.get("resource_config") {
        let mut config = MAAValue::default();
        insert!(config, "user_resource" => obj.get("user_resource").unwrap().to_owned());
        match obj.get("global_resource").unwrap().as_str().unwrap() {
            "None" => {}
            x => insert!(config, "global_resource" => x),
        };
        match obj.get("platform_diff_resource").unwrap().as_str().unwrap() {
            "None" => {}
            x => insert!(config, "platform_diff_resource" => x),
        };
        asst_config_out.insert("resource", config);
    }

    if let Some(obj) = asst_config.get("static_options") {
        let mut config = MAAValue::default();
        insert!(config, "cpu_ocr" => obj.get("cpu_ocr").unwrap().to_owned());
        if let Some(gpu_ocr) = obj.get("gpu_ocr") {
            config.insert("gpu_ocr", gpu_ocr.to_owned());
        }
        asst_config_out.insert("static_options", config);
    }

    filetype.write(&dest, &asst_config_out)?;

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
        let profile_dir = maa_dirs::config().join("profiles");
        let name = Path::new("__test__");

        // First time init
        init(Some(name), None, false).expect("failed to init profile");
        assert!(profile_dir.join("__test__.json").exists());

        // Second time init, same name
        assert!(init(Some(name), None, false).is_err());

        // Third time init, same name, force write
        init(Some(name), Some(Filetype::Toml), true).expect("failed to init profile");
        assert!(profile_dir.join("__test__.toml").exists());

        // cleanup
        let _ = std::fs::remove_file(profile_dir.join("__test__.toml"));
    }
}
