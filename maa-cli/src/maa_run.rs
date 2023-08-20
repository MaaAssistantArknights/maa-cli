use crate::dirs::Dirs;
use crate::installer::{maa_cli::CLIComponent, maa_core::MAA_CORE_NAME};

use std::env::current_exe;
use std::process::Command;

use anyhow::{bail, Result};

pub const LD_LIB_PATH_VAR: &str = if cfg!(target_os = "macos") {
    "DYLD_FALLBACK_LIBRARY_PATH"
} else if cfg!(target_os = "windows") {
    "PATH"
} else {
    "LD_LIBRARY_PATH"
};

pub fn command(dirs: &Dirs) -> Result<Command> {
    let lib_dir = dirs.library();
    if !lib_dir.join(MAA_CORE_NAME).exists() {
        bail!("MaaCore not found, please run `maa install` to install it")
    }

    let bin_dir = dirs.binary();
    let exe_name = CLIComponent::MaaRun.name();
    let exe_path = bin_dir.join(&exe_name);
    if !exe_path.exists() {
        #[cfg(debug_assertions)]
        if let Ok(path) = current_exe() {
            let exe_path = path.parent().unwrap().join(&exe_name);
            if !exe_path.exists() {
                bail!("maa-run not found, please run `maa self install` to install it")
            } else {
                return Ok(Command::new(exe_path));
            }
        }
        bail!("maa-run not found, please run `maa self install` to install it")
    } else {
        Ok(Command::new(exe_path))
    }
}

pub trait SetLDLibPath {
    fn set_ld_lib_path(&mut self, dirs: &Dirs) -> &mut Self;
}

impl SetLDLibPath for Command {
    fn set_ld_lib_path(&mut self, dirs: &Dirs) -> &mut Self {
        let lib_dir = dirs.library();
        let lib_path = lib_dir.to_str().unwrap();
        self.env(LD_LIB_PATH_VAR, lib_path)
    }
}
