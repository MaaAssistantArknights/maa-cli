mod download;

#[cfg(feature = "extract_helper")]
mod extract;

#[cfg(any(feature = "cli_installer", feature = "core_installer"))]
mod version_json;

#[cfg(feature = "cli_installer")]
pub mod maa_cli;
#[cfg(feature = "core_installer")]
pub mod maa_core;
