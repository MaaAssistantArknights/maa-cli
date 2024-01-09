use crate::consts::MAA_CORE_LIB;

use std::{
    env::{current_exe, var_os},
    fs::{create_dir, create_dir_all, remove_dir_all},
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
    copilot: PathBuf,
    resource: PathBuf,
    hot_update: PathBuf,
    state: PathBuf,
    log: PathBuf,
}

impl Dirs {
    pub fn new(proj: Option<ProjectDirs>) -> Self {
        let data_dir = get_data_dir(&proj);
        let state_dir = get_state_dir(&proj);
        let cache_dir = get_cache_dir(&proj);

        Self {
            data: data_dir.clone(),
            cache: cache_dir.clone(),
            copilot: cache_dir.join("copilot"),
            config: get_config_dir(&proj),
            library: data_dir.join("lib"),
            resource: data_dir.join("resource"),
            hot_update: data_dir.join("MaaResource"),
            state: state_dir.clone(),
            log: state_dir.join("debug"),
        }
    }

    /// Get data directory.
    pub fn data(&self) -> &Path {
        &self.data
    }

    /// Get library directory.
    pub fn library(&self) -> &Path {
        &self.library
    }

    /// Find the library directory.
    ///
    /// By default, the library directory is the `lib` directory in the data directory.
    /// If the library `MaaCore` is not found in the default library directory,
    /// Try to find it in the directory relative to the executable file.
    /// First, try to find the `MaaCore` in the same directory as the executable file.
    /// Then, assume the executable file is in the `bin` directory,
    /// try to find the `MaaCore` in the `lib` directory in the parent directory of the executable file.
    /// If the executable is a symbolic link, will try to find the `MaaCore` both in the symbolic link and the link target.
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

    /// Get config directory.
    pub fn config(&self) -> &Path {
        &self.config
    }

    /// Get absolute path in config directory.
    ///
    /// If the given path is absolute, return `None`.
    /// Otherwise, return the path in the config directory.
    /// The `sub_dir` is the sub directory of the config directory.
    /// If `sub_dir` is `None`, the path is relative to the config directory.
    /// Otherwise, the path is relative to the `sub_dir` directory.
    pub fn abs_config<P: AsRef<Path>, D: AsRef<Path>>(
        &self,
        path: P,
        sub_dir: Option<D>,
    ) -> Option<PathBuf> {
        let path = path.as_ref();
        if path.is_absolute() {
            None
        } else {
            let mut result = self.config.to_path_buf();
            if let Some(sub_dir) = sub_dir {
                result.push(sub_dir);
            }
            result.push(path);
            Some(result)
        }
    }

    /// Get cache directory.
    pub fn cache(&self) -> &Path {
        &self.cache
    }

    /// Get copilot cache directory.
    pub fn copilot(&self) -> &Path {
        &self.copilot
    }

    /// Get resource directory.
    pub fn resource(&self) -> &Path {
        &self.resource
    }

    /// Find the resource directory.
    ///
    /// By default, the resource directory is the `resource` directory in the data directory.
    /// If the resource directory is not found in the default resource directory,
    /// Try to find it in the directory relative to the executable file.
    /// First, try to find the resource directory in the same directory as the executable file.
    /// Then, assume the executable file is in the `bin` directory,
    /// try to find the resource directory in the `share/maa` directory in the parent directory of the executable file.
    /// If the executable is a symbolic link, will try to find the resource directory both in the symbolic link and the link target.
    ///
    /// Additionally, if maa is compiled with `MAA_EXTRA_SHARE_NAME` environment variable,
    /// try to find the resource directory in the `share/$MAA_EXTRA_SHARE_NAME` directory.
    /// This is used to support the situation that MaaCore is installed by other package manager.
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

    /// Get hot update resource directory.
    pub fn hot_update(&self) -> &Path {
        &self.hot_update
    }

    /// Get state directory.
    pub fn state(&self) -> &Path {
        &self.state
    }

    /// Get log directory.
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

pub fn find_library() -> Option<PathBuf> {
    DIRS.find_library(&current_exe().ok()?)
}

pub fn config() -> &'static Path {
    DIRS.config()
}

pub fn abs_config<P: AsRef<Path>, D: AsRef<Path>>(path: P, sub_dir: Option<D>) -> Option<PathBuf> {
    DIRS.abs_config(path, sub_dir)
}

pub fn cache() -> &'static Path {
    DIRS.cache()
}

pub fn copilot() -> &'static Path {
    DIRS.copilot()
}

pub fn resource() -> &'static Path {
    DIRS.resource()
}

pub fn find_resource() -> Option<PathBuf> {
    DIRS.find_resource(&current_exe().ok()?)
}

pub fn hot_update() -> &'static Path {
    DIRS.hot_update()
}

pub fn state() -> &'static Path {
    DIRS.state()
}

pub fn log() -> &'static Path {
    DIRS.log()
}

lazy_static! {
    static ref HOME: PathBuf = directories::BaseDirs::new()
        .expect("Failed to get home directory")
        .home_dir()
        .to_path_buf();
}

pub fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if let Ok(path) = path.strip_prefix("~") {
        HOME.join(path)
    } else {
        path.to_path_buf()
    }
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
            debug!("create-dir", dir = self.to_str().unwrap_or(""));
            create_dir_all(self)?;
        }
        Ok(self)
    }

    fn ensure_clean(self) -> Result<Self, Self::Error> {
        if self.exists() {
            debug!("remove-dir", dir = self.to_string_lossy());
            remove_dir_all(self)?;
            create_dir(self)?;
        } else {
            create_dir_all(self)?;
        }

        Ok(self)
    }
}

/// Similar to `globpath` of vim
pub fn global_path<I, D>(base_dirs: D, path: impl AsRef<Path>) -> Vec<PathBuf>
where
    I: AsRef<Path>,
    D: IntoIterator<Item = I>,
{
    let path = path.as_ref();
    let mut paths = Vec::new();
    for base_dir in base_dirs {
        let full_path = base_dir.as_ref().join(path);
        if full_path.exists() {
            paths.push(full_path);
        }
    }
    paths
}

pub fn global_find<I, D, F>(base_dirs: D, finder: F) -> Vec<PathBuf>
where
    I: AsRef<Path>,
    D: IntoIterator<Item = I>,
    F: Fn(&Path) -> Option<PathBuf>,
{
    let mut paths = Vec::new();
    for base_dir in base_dirs {
        if let Some(path) = finder(base_dir.as_ref()) {
            paths.push(path);
        }
    }
    paths
}

/// Ensure the given str is a name instead of a path.
///
/// # Panics
///
/// Panics if the given str is a string containing path separator.
#[allow(dead_code)]
fn ensure_name(name: &str) -> &str {
    assert!(
        !name.contains(std::path::is_separator),
        "The given name should not contain path separator"
    );
    name
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
            if cfg!(target_os = "macos") {
                assert_eq!(
                    TEST_DIRS.state(),
                    HOME.join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    TEST_DIRS.log(),
                    HOME.join("Library/Application Support/com.loong.maa/debug")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.state(), HOME.join(".local/state/maa"));
                assert_eq!(TEST_DIRS.log(), HOME.join(".local/state/maa/debug"));
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
            if cfg!(target_os = "macos") {
                assert_eq!(
                    TEST_DIRS.data(),
                    HOME.join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    TEST_DIRS.library(),
                    HOME.join("Library/Application Support/com.loong.maa/lib")
                );
                assert_eq!(
                    TEST_DIRS.resource(),
                    HOME.join("Library/Application Support/com.loong.maa/resource")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.data(), HOME.join(".local/share/maa"));
                assert_eq!(TEST_DIRS.library(), HOME.join(".local/share/maa/lib"));
                assert_eq!(TEST_DIRS.resource(), HOME.join(".local/share/maa/resource"));
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

            if let Some(name) = option_env!("MAA_EXTRA_SHARE_NAME") {
                let extra_share_dir = test_root.join("share").join(ensure_name(name));
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
        fn config_relative() {
            env::remove_var("XDG_CONFIG_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            if cfg!(target_os = "macos") {
                assert_eq!(
                    TEST_DIRS.config(),
                    HOME.join("Library/Application Support/com.loong.maa/config")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.config(), HOME.join(".config/maa"));
            }
            assert_eq!(
                TEST_DIRS.abs_config::<&str, &str>("foo", None).unwrap(),
                TEST_DIRS.config().join("foo")
            );
            assert_eq!(
                TEST_DIRS.abs_config("foo", Some("bar")).unwrap(),
                TEST_DIRS.config().join("bar").join("foo")
            );

            #[cfg(unix)]
            {
                assert_eq!(TEST_DIRS.abs_config::<&str, &str>("/tmp", None), None);
                assert_eq!(TEST_DIRS.abs_config("/tmp", Some("bar")), None);
            }

            assert_eq!(config(), TEST_DIRS.config());
            assert_eq!(
                abs_config("foo", Some("bar")).unwrap(),
                config().join("bar").join("foo")
            );

            env::set_var("XDG_CONFIG_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.config(), PathBuf::from("/xdg/maa"));

            env::set_var("MAA_CONFIG_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.config(), PathBuf::from("/maa"));
        }

        #[test]
        fn cache_relative() {
            env::remove_var("XDG_CACHE_HOME");
            let project = ProjectDirs::from("com", "loong", "maa");
            if cfg!(target_os = "macos") {
                assert_eq!(TEST_DIRS.cache(), HOME.join("Library/Caches/com.loong.maa"));
                assert_eq!(
                    TEST_DIRS.copilot(),
                    HOME.join("Library/Caches/com.loong.maa/copilot")
                );
            } else if cfg!(target_os = "linux") {
                assert_eq!(TEST_DIRS.cache(), HOME.join(".cache/maa"));
                assert_eq!(TEST_DIRS.copilot(), HOME.join(".cache/maa/copilot"));
            }
            assert_eq!(cache(), TEST_DIRS.cache());
            assert_eq!(copilot(), TEST_DIRS.copilot());

            env::set_var("XDG_CACHE_HOME", "/xdg");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.cache(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.copilot(), PathBuf::from("/xdg/maa/copilot"));

            env::set_var("MAA_CACHE_DIR", "/maa");
            let dirs = Dirs::new(project.clone());
            assert_eq!(dirs.cache(), PathBuf::from("/maa"));
            assert_eq!(dirs.copilot(), PathBuf::from("/maa/copilot"));
        }
    }

    #[test]
    fn test_expand_tilde() {
        assert_eq!(expand_tilde(Path::new("~")), HOME.as_path());
        assert_eq!(expand_tilde(Path::new("~/foo")), HOME.join("foo").as_path());
        assert_eq!(expand_tilde(Path::new("/foo")), Path::new("/foo"));
    }

    #[test]
    fn ensure() {
        let test_root = temp_dir().join("maa-test-ensure");
        let test_dir = test_root.join("test");
        assert_eq!(test_root.ensure_clean().unwrap(), test_root);
        assert!(!test_dir.exists());
        assert_eq!(test_dir.ensure().unwrap(), test_dir);
        assert!(test_dir.exists());
        remove_dir_all(&test_root).unwrap();
    }

    #[test]
    fn global_path_and_find() {
        let test_root = temp_dir().join("maa-test-global-path");
        let test_dir1 = test_root.join("test1");
        let test_dir2 = test_root.join("test2");
        let test_file = test_dir1.join("test");

        test_dir1.ensure_clean().unwrap();
        test_dir2.ensure_clean().unwrap();

        std::fs::File::create(&test_file).unwrap();

        assert_eq!(
            global_path([&test_dir1, &test_dir2], "test"),
            vec![test_file.clone()]
        );
        assert_eq!(
            global_path([&test_dir1, &test_dir2], "not_exist"),
            Vec::<PathBuf>::new()
        );

        assert_eq!(
            global_find([&test_dir1, &test_dir2], |dir| {
                if dir.join("test").exists() {
                    Some(dir.join("test"))
                } else {
                    None
                }
            }),
            vec![test_file.clone()]
        );

        assert_eq!(
            global_find([&test_dir1, &test_dir2], |dir| {
                if dir.join("not_exist").exists() {
                    Some(dir.join("not_exist"))
                } else {
                    None
                }
            }),
            Vec::<PathBuf>::new()
        );

        remove_dir_all(&test_root).unwrap();
    }

    #[test]
    fn ensure_name_ok() {
        assert_eq!(ensure_name("foo"), "foo");
    }

    #[test]
    #[should_panic]
    fn ensure_name_fail() {
        #[cfg(unix)]
        ensure_name("foo/bar");
        #[cfg(windows)]
        ensure_name("foo\\bar");
    }
}
