use crate::consts::MAA_CORE_LIB;

use std::{
    env::{current_exe, var_os},
    fs::{create_dir, remove_dir_all},
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use dunce::canonicalize;
use lazy_static::lazy_static;
use paste::paste;

macro_rules! matct_loc {
    (state, $dirs:ident) => {
        $dirs
            .state_dir()
            .unwrap_or_else(|| $dirs.data_dir())
            .to_path_buf()
    };
    (config, $dirs:ident) => {
        if cfg!(target_os = "macos") {
            $dirs.config_dir().join("config")
        } else {
            $dirs.config_dir().to_path_buf()
        }
    };
    ($loc:ident, $dirs:ident) => {
        paste! {
            $dirs.[<$loc _dir>]().to_path_buf()
        }
    };
}

macro_rules! get_dir {
    ($loc:ident) => {
        paste! {
            fn [<get_ $loc _dir>](proj: &Option<ProjectDirs>) -> PathBuf {
                if let Some(dir) = var_os(stringify!([<MAA_ $loc:upper _DIR>])) {
                    PathBuf::from(dir)
                } else if let Some(dir) = var_os(stringify!([<XDG_ $loc:upper _HOME>])) {
                    PathBuf::from(dir).join("maa")
                } else if let Some(dirs) = proj {
                    matct_loc!($loc, dirs)
                } else {
                    panic!("Failed to get {} directory!", stringify!($loc))
                }
            }
        }
    };
}

get_dir!(state);
get_dir!(data);
get_dir!(config);
get_dir!(cache);

pub struct Dirs {
    data: PathBuf,
    library: PathBuf,
    config: PathBuf,
    cache: PathBuf,
    resource: PathBuf,
    state: PathBuf,
    log: PathBuf,
}

impl Dirs {
    pub fn new(proj: Option<ProjectDirs>) -> Self {
        let data_dir = get_data_dir(&proj);
        let state_dir = get_state_dir(&proj);

        Self {
            data: data_dir.clone(),
            cache: get_cache_dir(&proj),
            config: get_config_dir(&proj),
            library: data_dir.join("lib"),
            resource: data_dir.join("resource"),
            state: state_dir.clone(),
            log: state_dir.join("debug"),
        }
    }

    pub fn data(&self) -> &Path {
        &self.data
    }

    pub fn library(&self) -> &Path {
        &self.library
    }

    pub fn config(&self) -> &Path {
        &self.config
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn resource(&self) -> &Path {
        &self.resource
    }

    pub fn state(&self) -> &Path {
        &self.state
    }

    pub fn log(&self) -> &Path {
        &self.log
    }
}

lazy_static! {
    pub static ref DIRS: Dirs = Dirs::new(ProjectDirs::from("com", "loong", "maa"));
}

pub fn data() -> &'static Path {
    DIRS.data()
}

pub fn library() -> &'static Path {
    DIRS.library()
}

pub fn config() -> &'static Path {
    DIRS.config()
}

pub fn cache() -> &'static Path {
    DIRS.cache()
}

pub fn resource() -> &'static Path {
    DIRS.resource()
}

pub fn state() -> &'static Path {
    DIRS.state()
}

pub fn log() -> &'static Path {
    DIRS.log()
}

pub fn find_library() -> Option<PathBuf> {
    let lib_dir = library();
    if lib_dir.join(MAA_CORE_LIB).exists() {
        return Some(lib_dir.to_path_buf());
    }

    current_exe_dir_find(|exe_dir| {
        if exe_dir.join(MAA_CORE_LIB).exists() {
            return Some(exe_dir.to_path_buf());
        }
        if let Some(dir) = exe_dir.parent() {
            let lib_dir = dir.join("lib");
            let lib_path = lib_dir.join(MAA_CORE_LIB);
            if lib_path.exists() {
                return Some(lib_dir);
            }
        }

        None
    })
}

pub fn find_resource() -> Option<PathBuf> {
    let resource_dir = resource();
    if resource_dir.exists() {
        return Some(resource_dir.to_path_buf());
    }

    current_exe_dir_find(|exe_dir| {
        let resource_dir = exe_dir.join("resource");
        if resource_dir.exists() {
            return Some(resource_dir);
        }
        if let Some(dir) = exe_dir.parent() {
            let share_dir = dir.join("share");
            if let Some(extra_share) = option_env!("MAA_EXTRA_SHARE_NAME") {
                let resource_dir = share_dir.join(extra_share).join("resource");
                if resource_dir.exists() {
                    return Some(resource_dir);
                }
            }
            let resource_dir = share_dir.join("maa").join("resource");
            if resource_dir.exists() {
                return Some(resource_dir);
            }
        }
        None
    })
}

/// Find path starting from current executable directory
pub fn current_exe_dir_find<F>(finder: F) -> Option<PathBuf>
where
    F: Fn(&Path) -> Option<PathBuf>,
{
    let exe_path = current_exe().ok()?;
    let exe_dir = exe_path.parent().unwrap();
    let canonicalized = canonicalize(exe_dir).ok()?;
    if let Some(path) = finder(&canonicalized) {
        return Some(path);
    };
    if canonicalized != exe_dir {
        finder(exe_dir)
    } else {
        None
    }
}

pub trait Ensure: Sized {
    type Error;

    /// Ensure the path exists, create it if not.
    ///
    /// Return the path itself if it exists or created successfully.
    /// Otherwise, return an error.
    fn ensure(self) -> Result<Self, Self::Error>;

    /// Ensure the dir is empty, create it if not.
    ///
    /// Return the path itself if it exists or created successfully.
    /// If the dir is not empty, remove all files in it.
    fn ensure_clean(self) -> Result<Self, Self::Error>;
}

impl Ensure for &Path {
    type Error = std::io::Error;

    fn ensure(self) -> Result<Self, Self::Error> {
        if !self.exists() {
            if let Some(parent) = self.parent() {
                parent.ensure()?;
            }
            create_dir(self)?;
        }
        Ok(self)
    }

    fn ensure_clean(self) -> Result<Self, Self::Error> {
        if self.exists() {
            remove_dir_all(self)?;
        } else if let Some(parent) = self.parent() {
            parent.ensure()?;
        }
        create_dir(self)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_dir {
        use super::*;
        use std::env;

        #[test]
        fn state_relative() {
            env::remove_var("XDG_STATE_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            let dirs = Dirs::new(project.clone());
            if cfg!(target_os = "macos") {
                assert_eq!(
                    dirs.state(),
                    home_dir.join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    dirs.log(),
                    home_dir.join("Library/Application Support/com.loong.maa/debug")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(dirs.state(), home_dir.join(".local/state/maa"));
                assert_eq!(dirs.log(), home_dir.join(".local/state/maa/debug"));
            }
            assert_eq!(state(), dirs.state());
            assert_eq!(log(), dirs.log());

            env::set_var("XDG_STATE_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.state(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/xdg/maa/debug"));

            env::set_var("MAA_STATE_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.state(), PathBuf::from("/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/maa/debug"));
        }

        #[test]
        fn data_relative() {
            env::remove_var("XDG_DATA_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            let dirs = Dirs::new(project.clone());
            if cfg!(target_os = "macos") {
                assert_eq!(
                    dirs.data(),
                    home_dir.join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    dirs.library(),
                    home_dir.join("Library/Application Support/com.loong.maa/lib")
                );
                assert_eq!(
                    dirs.resource(),
                    home_dir.join("Library/Application Support/com.loong.maa/resource")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(dirs.data(), home_dir.join(".local/share/maa"));
                assert_eq!(dirs.library(), home_dir.join(".local/share/maa/lib"));
                assert_eq!(dirs.resource(), home_dir.join(".local/share/maa/resource"));
            }
            assert_eq!(data(), dirs.data());
            assert_eq!(library(), dirs.library());

            env::set_var("XDG_DATA_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.data(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/xdg/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/xdg/maa/resource"));

            env::set_var("MAA_DATA_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.data(), PathBuf::from("/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/maa/resource"));
        }

        #[test]
        fn config_dir() {
            env::remove_var("XDG_CONFIG_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            let dirs = Dirs::new(project.clone());
            if cfg!(target_os = "macos") {
                assert_eq!(
                    dirs.config(),
                    home_dir.join("Library/Application Support/com.loong.maa/config")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(dirs.config(), home_dir.join(".config/maa"));
            }

            env::set_var("XDG_CONFIG_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.config(), PathBuf::from("/xdg/maa"));

            env::set_var("MAA_CONFIG_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.config(), PathBuf::from("/maa"));
        }

        #[test]
        fn cache_dir() {
            env::remove_var("XDG_CACHE_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            let dirs = Dirs::new(project.clone());
            if cfg!(target_os = "macos") {
                assert_eq!(dirs.cache(), home_dir.join("Library/Caches/com.loong.maa"));
            } else if cfg!(target_os = "linux") {
                assert_eq!(dirs.cache(), home_dir.join(".cache/maa"));
            }
            assert_eq!(cache(), dirs.cache());

            env::set_var("XDG_CACHE_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.cache(), PathBuf::from("/xdg/maa"));

            env::set_var("MAA_CACHE_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.cache(), PathBuf::from("/maa"));
        }
    }
}
