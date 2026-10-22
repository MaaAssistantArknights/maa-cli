use std::path::PathBuf;

use anyhow::Result;
use log::warn;

use crate::config::cli::{CLI_CONFIG, resource::ResourceSource};

mod git;
mod mirrorchyan;

pub fn update(is_auto: bool) -> Result<()> {
    let config = CLI_CONFIG.resource_config();
    if is_auto && !config.auto_update() {
        return Ok(());
    }
    match config.source() {
        ResourceSource::Git(git) => self::git::update(git, config.warn_on_update_failure()),
        ResourceSource::MirrorChyan(mirror) => mirror_result(
            mirrorchyan::update(mirror, is_auto),
            config.warn_on_update_failure(),
            mirrorchyan::usable_snapshot(),
        ),
    }
}

/// The resource layer owns source selection and the order of overlays.
pub fn resource_dirs() -> Vec<PathBuf> {
    let resource_dir = match CLI_CONFIG.resource_config().source() {
        ResourceSource::Git(_) => maa_dirs::maa_resource().join("resource"),
        ResourceSource::MirrorChyan(_) => mirrorchyan::resource_dir().join("resource"),
    };
    vec![resource_dir, maa_dirs::hot_update_resource().to_path_buf()]
}

fn mirror_result(result: Result<()>, warn_only: bool, usable: bool) -> Result<()> {
    if warn_only && usable {
        if let Err(error) = result {
            warn!("Failed to update MirrorChyan resources: {error}");
        }
        Ok(())
    } else {
        result
    }
}

#[cfg(test)]
mod mirror_tests {
    use super::*;

    #[test]
    fn warning_mode_requires_a_usable_mirror_snapshot() {
        assert!(mirror_result(Err(anyhow::anyhow!("download failed")), true, false).is_err());
        assert!(mirror_result(Err(anyhow::anyhow!("download failed")), true, true).is_ok());
        assert!(mirror_result(Err(anyhow::anyhow!("download failed")), false, true).is_err());
        assert!(mirror_result(Ok(()), true, false).is_ok());
    }
}
