#[cfg(feature = "__installer")]
mod download;
#[cfg(feature = "__installer")]
mod extract;
#[cfg(feature = "__installer")]
mod version_json;

#[cfg(feature = "cli_installer")]
pub mod maa_cli;
#[cfg(feature = "core_installer")]
pub mod maa_core;

pub mod resource;
