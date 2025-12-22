#[cfg(feature = "cli_installer")]
pub mod maa_cli;
#[cfg(feature = "core_installer")]
pub mod maa_core;

pub mod hot_update;
pub mod resource;
