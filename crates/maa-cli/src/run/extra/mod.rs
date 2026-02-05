#[cfg(target_os = "windows")]
pub mod mumu;

#[cfg(target_os = "windows")]
pub use mumu::mumu_extra;

#[cfg(target_os = "windows")]
pub mod ldplayer;

#[cfg(target_os = "windows")]
pub use ldplayer::ld_extra;
