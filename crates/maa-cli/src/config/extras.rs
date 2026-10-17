use std::{path::PathBuf, process::Command};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum ExtraConnectionConfig {
    MaaFw(ExtraConnectionConfigMaaFw),
}

impl ExtraConnectionConfig {
    pub fn name(&self) -> &str {
        match self {
            Self::MaaFw(..) => "MaaFw",
        }
    }

    pub fn get_for_gamescope() -> Result<Self> {
        let pw_out = Command::new("pw-cli")
            .args(["ls", "Node"])
            .output()
            .context("Failed to run 'pw-cli' to get gamescope node id")?;

        if !pw_out.status.success() {
            bail!("'pw-cli ls Node' exited with {}", pw_out.status)
        };

        let Some(pw_node_id) = find_gamescope_id(&String::from_utf8_lossy(&pw_out.stdout)) else {
            bail!("Can't find gamescope pipewire node id")
        };

        let eis_socket_path = "/run/user/1000/gamescope-0-ei".into();

        Ok(Self::MaaFw(ExtraConnectionConfigMaaFw {
            library_name: MaaFwControlUnitLibraryName::Linux,
            screencap_method: MaaFwLinuxScreencapMethod::PipeWire as u32,
            input_method: MaaFwLinuxInputMethod::Libei as u32,
            pw_node_id: Some(pw_node_id),
            eis_socket_path: Some(eis_socket_path),
            ..Default::default()
        }))
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Serialize, Clone, Default)]
pub struct ExtraConnectionConfigMaaFw {
    // Used by MaaCore's MaaFwController::connect_with_extras
    library_name: MaaFwControlUnitLibraryName,
    #[serde(skip_serializing_if = "Option::is_none")]
    screen_resolution: Option<ScreenResolution>,

    // Used by MaaFramework's MaaLinuxControllerCreate invoked by MaaCore
    // See https://maafw.com/docs/2.2-IntegratedInterfaceOverview#maalinuxcontrollercreate
    screencap_method: u32,
    input_method: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    wlr_socket_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pw_socket_fd: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pw_node_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uinput_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uinput_screen_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uinput_screen_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eis_socket_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_win32_vk_code: Option<u32>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Default)]
pub enum MaaFwControlUnitLibraryName {
    #[default]
    Adb,
    Linux,
    #[allow(unused)]
    AndroidNative,
}

impl MaaFwControlUnitLibraryName {
    const fn to_str(&self) -> &'static str {
        match self {
            Self::Adb => "MaaAdbControlUnit",
            Self::Linux => "MaaLinuxControlUnit",
            Self::AndroidNative => "MaaAndroidNativeControlUnit",
        }
    }
}

impl serde::Serialize for MaaFwControlUnitLibraryName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_str())
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Serialize)]
pub struct ScreenResolution {
    width: u32,
    height: u32,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Default)]
pub enum MaaFwLinuxScreencapMethod {
    #[default]
    Wlr = 1,
    PipeWire = 4,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Default)]
pub enum MaaFwLinuxInputMethod {
    #[default]
    Wlr = 1,
    #[allow(unused)]
    UInput = 2,
    Libei = 4,
}

fn find_gamescope_id(input: &str) -> Option<u32> {
    let mut current_id: Option<u32> = None;

    for line in input.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("id ") {
            if let Some((id_str, _)) = rest.split_once(',') {
                current_id = id_str.trim().parse::<u32>().ok();
            }
        } else if let Some(rest) = line.strip_prefix("node.name = ") {
            let name = rest.trim().trim_matches('"');
            if name == "gamescope" {
                return current_id;
            }
        }
    }

    None
}
