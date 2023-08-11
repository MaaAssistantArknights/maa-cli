use std::path::PathBuf;

pub trait Dirs {
    fn config_dir(&self) -> Option<PathBuf>;
    fn data_dir(&self) -> Option<PathBuf>;
    fn state_dir(&self) -> Option<PathBuf>;
    fn cache_dir(&self) -> Option<PathBuf>;
}

// For unix like systems, use xdg basedir spec
#[cfg(unix)]
mod xdg;
#[cfg(unix)]
pub type ProjectDirs = xdg::XDGProject;
