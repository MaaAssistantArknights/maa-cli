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

    pub fn find_library(&self, exe_path: &Path) -> Option<PathBuf> {
        if self.library.join(MAA_CORE_LIB).exists() {
            return Some(self.library.clone());
        }

        _find_from(exe_path, |exe_dir| {
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

    pub fn find_resource(&self, exe_path: &Path) -> Option<PathBuf> {
        if self.resource.exists() {
            return Some(self.resource.clone());
        }

        _find_from(exe_path, |exe_dir| {
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
    DIRS.find_library(&current_exe().ok()?)
}

pub fn find_resource() -> Option<PathBuf> {
    DIRS.find_resource(&current_exe().ok()?)
}

/// Similar to `finder(exe_path.parent()?)`, but try to canonicalize the path first.
fn _find_from<F>(exe_path: &Path, finder: F) -> Option<PathBuf>
where
    F: Fn(&Path) -> Option<PathBuf>,
{
    // Try to canonicalize the path first.
    if let Ok(canonicalized_exe_path) = canonicalize(exe_path) {
        if let Some(path) = finder(canonicalized_exe_path.parent()?) {
            return Some(path);
        };
    }
    finder(exe_path.parent()?)
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
    use std::env::{self, temp_dir};

    mod get_dir {
        use super::*;
        use std::fs::{create_dir_all, remove_dir_all, File};

        lazy_static! {
            static ref TEST_DIRS: Dirs = Dirs::new(ProjectDirs::from("com", "loong", "maa"));
        }

        #[test]
        fn state_relative() {
            env::remove_var("XDG_STATE_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            if cfg!(target_os = "macos") {
                assert_eq!(
                    TEST_DIRS.state(),
                    home_dir.join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    TEST_DIRS.log(),
                    home_dir.join("Library/Application Support/com.loong.maa/debug")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.state(), home_dir.join(".local/state/maa"));
                assert_eq!(TEST_DIRS.log(), home_dir.join(".local/state/maa/debug"));
            }
            assert_eq!(state(), TEST_DIRS.state());
            assert_eq!(log(), TEST_DIRS.log());

            env::set_var("XDG_STATE_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.state(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/xdg/maa/debug"));
            env::remove_var("XDG_STATE_HOME");

            env::set_var("MAA_STATE_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.state(), PathBuf::from("/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/maa/debug"));
            env::remove_var("MAA_STATE_DIR");
        }

        #[test]
        fn data_relative() {
            env::remove_var("XDG_DATA_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            if cfg!(target_os = "macos") {
                assert_eq!(
                    TEST_DIRS.data(),
                    home_dir.join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    TEST_DIRS.library(),
                    home_dir.join("Library/Application Support/com.loong.maa/lib")
                );
                assert_eq!(
                    TEST_DIRS.resource(),
                    home_dir.join("Library/Application Support/com.loong.maa/resource")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.data(), home_dir.join(".local/share/maa"));
                assert_eq!(TEST_DIRS.library(), home_dir.join(".local/share/maa/lib"));
                assert_eq!(
                    TEST_DIRS.resource(),
                    home_dir.join(".local/share/maa/resource")
                );
            }
            assert_eq!(data(), TEST_DIRS.data());
            assert_eq!(library(), TEST_DIRS.library());
            assert_eq!(resource(), TEST_DIRS.resource());
            // The value of `MAA_COER_VERSION` is set in CI,
            // where the MaaCore is installed at standard location.
            if env::var_os("MAA_CORE_INSTALLED").is_some() {
                // This is not used in this test, but needed.
                let extra_dir = Path::new("/usr/local/share/maa");
                assert_eq!(
                    TEST_DIRS.find_library(extra_dir).unwrap(),
                    TEST_DIRS.library()
                );
                assert_eq!(
                    TEST_DIRS.find_resource(extra_dir).unwrap(),
                    TEST_DIRS.resource()
                );
                assert_eq!(find_library().unwrap(), library());
                assert_eq!(find_resource().unwrap(), resource());
            }

            env::set_var("XDG_DATA_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.data(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/xdg/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/xdg/maa/resource"));
            env::remove_var("XDG_DATA_HOME");

            env::set_var("MAA_DATA_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.data(), PathBuf::from("/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/maa/resource"));
            env::remove_var("MAA_DATA_DIR");

            // In this test case we use the Dirs instance created by former test case.
            // Because the /maa directory not exists, and which shadow the installation
            // of MaaCore, so we can test the situation that MaaCore is installed at
            // non-standard location.
            let test_root = temp_dir().join("maa-test-data");
            let test_root = canonicalize(test_root.ensure().unwrap()).unwrap();

            // Test the situation that maa -> path, core -> path, resource -> path/resource
            test_root.ensure_clean().unwrap();
            let bin_dir = test_root.clone();
            let library_dir = test_root.clone();
            let resource_dir = test_root.join("resource");
            bin_dir.ensure_clean().unwrap();
            library_dir.ensure_clean().unwrap();
            resource_dir.ensure_clean().unwrap();
            let bin_exe = bin_dir.join("maa");
            File::create(&bin_exe).unwrap();
            File::create(library_dir.join(MAA_CORE_LIB)).unwrap();
            assert_eq!(dirs.find_library(&bin_exe).unwrap(), library_dir);
            assert_eq!(dirs.find_resource(&bin_exe).unwrap(), resource_dir);

            // Test the situation maa -> path/bin, core -> path/lib, resource -> path/share/maa
            test_root.ensure_clean().unwrap();
            let bin_dir = test_root.join("bin");
            let library_dir = test_root.join("lib");
            let share_dir = test_root.join("share").join("maa");
            let resource_dir = share_dir.join("resource");
            bin_dir.ensure_clean().unwrap();
            library_dir.ensure_clean().unwrap();
            resource_dir.ensure_clean().unwrap();
            let bin_exe = bin_dir.join("maa");
            File::create(bin_dir.join("maa")).unwrap();
            File::create(library_dir.join(MAA_CORE_LIB)).unwrap();
            assert_eq!(dirs.find_library(&bin_exe).unwrap(), library_dir);
            assert_eq!(dirs.find_resource(&bin_exe).unwrap(), resource_dir);

            if let Some(extra_share) = option_env!("MAA_EXTRA_SHARE_NAME") {
                let extra_share_dir = test_root.join("share").join(extra_share);
                let extra_resource_dir = extra_share_dir.join("resource");
                create_dir_all(&extra_resource_dir).unwrap();
                assert_eq!(dirs.find_resource(&bin_exe).unwrap(), extra_resource_dir);
                remove_dir_all(&extra_share_dir).unwrap();
            }

            // Test the situation that maa linked
            #[cfg(target_os = "macos")]
            {
                use std::os::unix::fs::symlink;

                test_root.ensure_clean().unwrap();

                // Test the situation that maa -> path/cellar/bin, core -> path/cellar/lib,
                // resource -> path/share/maa, and maa is linked to path/bin.
                let cellar = test_root.join("Cellar");
                let bin_dir = cellar.join("bin");
                let library_dir = cellar.join("lib");
                let share_dir = test_root.join("share").join("maa");
                let resource_dir = share_dir.join("resource");
                let linked_dir = test_root.join("bin");
                bin_dir.ensure_clean().unwrap();
                library_dir.ensure_clean().unwrap();
                resource_dir.ensure_clean().unwrap();
                linked_dir.ensure_clean().unwrap();
                let bin_exe = bin_dir.join("maa");
                let linked_exe = linked_dir.join("maa");
                File::create(&bin_exe).unwrap();
                File::create(library_dir.join(MAA_CORE_LIB)).unwrap();
                symlink(&bin_exe, &linked_exe).unwrap();
                assert_eq!(dirs.find_library(&linked_exe).unwrap(), library_dir);
                assert_eq!(dirs.find_resource(&linked_exe).unwrap(), resource_dir);
                // Test the situation that maa -> path/cellar/bin, core -> path/lib, resource -> path/share/maa,
                // and maa is linked to path/bin.

                // remove old dirs
                remove_dir_all(&library_dir).unwrap();
                remove_dir_all(&share_dir).unwrap();

                let library_dir = test_root.join("lib");
                let share_dir = test_root.join("share").join("maa");
                let resource_dir = share_dir.join("resource");
                std::fs::create_dir_all(&library_dir).unwrap();
                std::fs::create_dir_all(&resource_dir).unwrap();
                File::create(library_dir.join(MAA_CORE_LIB)).unwrap();
                assert_eq!(dirs.find_library(&linked_exe).unwrap(), library_dir);
                assert_eq!(dirs.find_resource(&linked_exe).unwrap(), resource_dir);
            }

            remove_dir_all(&test_root).unwrap();
        }

        #[test]
        fn config_dir() {
            env::remove_var("XDG_CONFIG_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            let home_dir = PathBuf::from(env::var_os("HOME").unwrap());
            if cfg!(target_os = "macos") {
                assert_eq!(
                    TEST_DIRS.config(),
                    home_dir.join("Library/Application Support/com.loong.maa/config")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.config(), home_dir.join(".config/maa"));
            }
            assert_eq!(config(), TEST_DIRS.config());

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
            if cfg!(target_os = "macos") {
                assert_eq!(
                    TEST_DIRS.cache(),
                    home_dir.join("Library/Caches/com.loong.maa")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.cache(), home_dir.join(".cache/maa"));
            }
            assert_eq!(cache(), TEST_DIRS.cache());

            env::set_var("XDG_CACHE_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.cache(), PathBuf::from("/xdg/maa"));

            env::set_var("MAA_CACHE_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.cache(), PathBuf::from("/maa"));
        }
    }

    #[test]
    fn ensure() {
        let test_root = temp_dir().join("maa-test-ensure");
        let test_dir = test_root.join("test");
        assert_eq!(test_root.ensure_clean().unwrap(), test_root);
        assert!(!test_dir.exists());
        assert_eq!(test_dir.ensure().unwrap(), test_dir);
        assert!(test_dir.exists());
    }
}
