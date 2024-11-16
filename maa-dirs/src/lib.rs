use std::{
    borrow::Cow,
    env::{
        consts::{DLL_PREFIX, DLL_SUFFIX},
        current_exe, var_os,
    },
    ffi::OsStr,
    fs::{create_dir, create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use directories::ProjectDirs;
use dunce::canonicalize;

macro_rules! str_join {
    ($($e:expr),*) => {{
        const LEN: usize = 0 $(+ $e.len())*;
        #[expect(
            unused_assignments,
            reason = "The last assignment will not be used, but in macro, we can't avoid it."
        )]
        const BYTES: [u8; LEN] = {
            let mut dest: [u8; LEN] = [0; LEN];
            let mut offset = 0;
            $(
                let src = $e.as_bytes();
                let len = src.len();
                let mut i = 0;
                while i < len {
                    dest[offset + i] = src[i];
                    i += 1;
                }
                offset += len;
            )*
            dest
        };
        unsafe { std::str::from_utf8_unchecked(&BYTES) }
    }};
}

/// The name of the MaaCore library.
pub const MAA_CORE_NAME: &str = "MaaCore";
/// The name of the MaaCore library with the platform-specific prefix and suffix.
pub const MAA_CORE_LIB: &str = str_join!(DLL_PREFIX, MAA_CORE_NAME, DLL_SUFFIX);

/// A convenient macro to join paths, avoiding intermediate `PathBuf` allocation.
///
/// If the first path is a PathBuf, it will be reused.
/// Otherwise, a new PathBuf will be created from the first path.
/// This is useful to avoid unnecessary allocation when the first path is a PathBuf,
/// and when multiple paths are joined.
///
/// The expression after `;` is optional, which is used to set the extension of the final path.
///
/// Note: Because we reuse the first path, the first path will be consumed.
/// Thus, if you want to reuse the first path, you should pass a Path instead of a PathBuf
#[macro_export]
macro_rules! join {
    ($path:expr, $($paths:expr),+ $(; $ext:expr)?) => {{
        let mut path: ::std::path::PathBuf = $path.into();
        $(
            path.push($paths);
        )+
        $(
            path.set_extension($ext);
        )?
        path
    }}
}

/// Get the directory from environment variables.
///
/// The `maa_env` usually is `MAA_XXX_DIR`, and the `xdg_env` usually is `XDG_XXX_HOME`.
/// If the `maa_env` is set, return the directory `maa_env`.
/// If the `xdg_env` is set, return the directory `xdg_env/maa`.
/// Otherwise, return `None`.
fn dir_from_env(maa_env: impl AsRef<OsStr>, xdg_env: impl AsRef<OsStr>) -> Option<PathBuf> {
    var_os(maa_env)
        .map(PathBuf::from)
        .or_else(|| var_os(xdg_env).map(|xdg| join!(xdg, "maa")))
}

/// Get the data directory.
fn get_data_dir(proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env("MAA_DATA_DIR", "XDG_DATA_HOME")
        .or_else(|| proj.map(|dirs| dirs.data_dir().into()))
        .expect("Failed to get data directory!")
}

/// Get the state directory.
fn get_state_dir(proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env("MAA_STATE_DIR", "XDG_STATE_HOME")
        .or_else(|| proj.map(|dirs| dirs.state_dir().unwrap_or_else(|| dirs.data_dir()).into()))
        .expect("Failed to get state directory!")
}

/// Get the cache directory.
fn get_cache_dir(proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env("MAA_CACHE_DIR", "XDG_CACHE_HOME")
        .or_else(|| proj.map(|dirs| dirs.cache_dir().into()))
        .expect("Failed to get cache directory!")
}

/// Get the config directory.
fn get_config_dir(proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env("MAA_CONFIG_DIR", "XDG_CONFIG_HOME")
        .or_else(|| {
            proj.map(|dirs| {
                if cfg!(target_os = "macos") {
                    dirs.config_dir().join("config")
                } else {
                    dirs.config_dir().into()
                }
            })
        })
        .expect("Failed to get config directory!")
}

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
        let proj = proj.as_ref();
        let data_dir = get_data_dir(proj);
        let state_dir = get_state_dir(proj);
        let cache_dir = get_cache_dir(proj);

        Self {
            copilot: cache_dir.join("copilot"),
            cache: cache_dir,
            config: get_config_dir(proj),
            library: data_dir.join("lib"),
            resource: data_dir.join("resource"),
            hot_update: data_dir.join("MaaResource"),
            data: data_dir,
            log: state_dir.join("debug"),
            state: state_dir,
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
    /// If the library MaaCore is not found in the default library directory,
    /// Try to find it in the directory relative to the executable file.
    /// First, try to find the MaaCore in the same directory as the executable file.
    /// Then, assume the executable file is in the `bin` directory,
    /// try to find the MaaCore in the `lib` directory in the parent directory of the executable
    /// file. If the executable is a symbolic link, will try to find the MaaCore both in the
    /// symbolic link and the link target.
    pub fn find_library<'a>(&'a self, exe_path: &'a Path) -> Option<Cow<'a, Path>> {
        if self.library().join(MAA_CORE_LIB).exists() {
            return Some(self.library().into());
        }

        _find_from(exe_path, |exe_dir| {
            if exe_dir.join(MAA_CORE_LIB).exists() {
                return Some(exe_dir);
            }
            if let Some(dir) = exe_dir.parent() {
                let lib_dir = dir.join("lib");
                let lib_path = lib_dir.join(MAA_CORE_LIB);
                if lib_path.exists() {
                    return Some(lib_dir.into());
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
    /// try to find the resource directory in the `share/maa` directory in the parent directory of
    /// the executable file. If the executable is a symbolic link, will try to find the resource
    /// directory both in the symbolic link and the link target.
    ///
    /// Additionally, if maa is compiled with `MAA_EXTRA_SHARE_NAME` environment variable,
    /// try to find the resource directory in the `share/$MAA_EXTRA_SHARE_NAME` directory.
    /// This is used to support the situation that MaaCore is installed by other package manager.
    pub fn find_resource<'a>(&'a self, exe_path: &'a Path) -> Option<Cow<'a, Path>> {
        if self.resource().exists() {
            return Some(Cow::Borrowed(self.resource()));
        }

        _find_from(exe_path, |exe_dir| {
            let resource_dir = exe_dir.join("resource");
            if resource_dir.exists() {
                return Some(resource_dir.into());
            }
            if let Some(dir) = exe_dir.parent() {
                let share_dir = dir.join("share");
                if let Some(extra_share) = option_env!("MAA_EXTRA_SHARE_NAME") {
                    let resource_dir = join!(&share_dir, extra_share, "resource");
                    if resource_dir.exists() {
                        return Some(resource_dir.into());
                    }
                }
                let resource_dir = join!(share_dir, "maa", "resource");
                if resource_dir.exists() {
                    return Some(resource_dir.into());
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

fn dirs() -> &'static Dirs {
    static DIRS: OnceLock<Dirs> = OnceLock::new();
    DIRS.get_or_init(|| Dirs::new(ProjectDirs::from("com", "loong", "maa")))
}

fn exe() -> Option<&'static Path> {
    static CURRENT_EXE: OnceLock<Option<PathBuf>> = OnceLock::new();
    CURRENT_EXE.get_or_init(|| current_exe().ok()).as_deref()
}

pub fn data() -> &'static Path {
    dirs().data()
}

pub fn library() -> &'static Path {
    dirs().library()
}

pub fn find_library() -> Option<Cow<'static, Path>> {
    dirs().find_library(exe()?)
}

pub fn config() -> &'static Path {
    dirs().config()
}

pub fn abs_config<P: AsRef<Path>, D: AsRef<Path>>(path: P, sub_dir: Option<D>) -> Option<PathBuf> {
    dirs().abs_config(path, sub_dir)
}

pub fn cache() -> &'static Path {
    dirs().cache()
}

pub fn copilot() -> &'static Path {
    dirs().copilot()
}

pub fn resource() -> &'static Path {
    dirs().resource()
}

pub fn find_resource() -> Option<Cow<'static, Path>> {
    dirs().find_resource(exe()?)
}

pub fn hot_update() -> &'static Path {
    dirs().hot_update()
}

pub fn state() -> &'static Path {
    dirs().state()
}

pub fn log() -> &'static Path {
    dirs().log()
}

fn home() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        directories::BaseDirs::new()
            .expect("Failed to get home directory")
            .home_dir()
            .to_path_buf()
    })
}

pub fn expand_tilde(path: &Path) -> Cow<Path> {
    if let Ok(path) = path.strip_prefix("~") {
        home().join(path).into()
    } else {
        path.into()
    }
}

/// Similar to `finder(exe_path.parent()?)`, but try to canonicalize the path first.
fn _find_from<F>(exe_path: &Path, finder: F) -> Option<Cow<Path>>
where
    F: Fn(Cow<Path>) -> Option<Cow<Path>>,
{
    // Try to canonicalize the path first.
    if let Ok(mut canonicalized_exe_path) = canonicalize(exe_path) {
        canonicalized_exe_path.pop();
        if let Some(path) = finder(canonicalized_exe_path.into()) {
            return Some(path);
        };
    }
    finder(exe_path.parent()?.into())
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
            create_dir_all(self)?;
        }
        Ok(self)
    }

    fn ensure_clean(self) -> Result<Self, Self::Error> {
        if self.exists() {
            let mut ret = remove_dir_all(self);
            for i in 1..=3 {
                if let Err(err) = &ret {
                    log::warn!(
                        "Failed to remove dir {} due to {err}, retry {i} times",
                        self.display()
                    );
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    ret = remove_dir_all(self);
                } else {
                    break;
                }
            }
            ret?;
        } else if let Some(parent) = self.parent() {
            parent.ensure()?;
        }
        create_dir(self)?;
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
#[allow(dead_code, reason = "This function is only called at compile time")]
fn ensure_name(name: &str) -> &str {
    assert!(
        !name.contains(std::path::is_separator),
        "The given name should not contain path separator"
    );
    name
}

#[cfg(test)]
mod tests {
    use std::env::{self, temp_dir};

    use super::*;

    #[test]
    fn maa_lib_name() {
        #[cfg(target_os = "macos")]
        assert_eq!(MAA_CORE_LIB, "libMaaCore.dylib");

        #[cfg(target_os = "linux")]
        assert_eq!(MAA_CORE_LIB, "libMaaCore.so");

        #[cfg(target_os = "windows")]
        assert_eq!(MAA_CORE_LIB, "MaaCore.dll");
    }

    mod get_dir {
        use std::fs::{create_dir_all, remove_dir_all};

        use super::*;

        fn project() -> Option<ProjectDirs> {
            ProjectDirs::from("com", "loong", "maa")
        }

        /// A dirs instance created in a clean environment (no environment variables set).
        /// And the static DIRS should also be initialized in this clean environment.
        fn clean_dirs() -> &'static Dirs {
            static TEST_DIRS: OnceLock<Dirs> = OnceLock::new();
            TEST_DIRS.get_or_init(|| {
                env::remove_var("XDG_DATA_HOME");
                env::remove_var("XDG_STATE_HOME");
                env::remove_var("XDG_CACHE_HOME");
                env::remove_var("XDG_CONFIG_HOME");
                env::remove_var("MAA_DATA_DIR");
                env::remove_var("MAA_STATE_DIR");
                env::remove_var("MAA_CACHE_DIR");
                env::remove_var("MAA_CONFIG_DIR");
                home();
                log();
                Dirs::new(project())
            })
        }

        /// Test for path the depends on *_STATE_* environment variables.
        #[test]
        fn state_relative() {
            let dirs = clean_dirs();
            #[cfg(target_os = "macos")]
            {
                assert_eq!(
                    dirs.state(),
                    home().join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    dirs.log(),
                    home().join("Library/Application Support/com.loong.maa/debug")
                );
            }
            #[cfg(target_os = "linux")]
            {
                assert_eq!(dirs.state(), home().join(".local/state/maa"));
                assert_eq!(dirs.log(), home().join(".local/state/maa/debug"));
            }
            assert_eq!(state(), clean_dirs().state());
            assert_eq!(log(), clean_dirs().log());

            env::set_var("XDG_STATE_HOME", "/xdg");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.state(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/xdg/maa/debug"));
            env::remove_var("XDG_STATE_HOME");

            env::set_var("MAA_STATE_DIR", "/maa");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.state(), PathBuf::from("/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/maa/debug"));
            env::remove_var("MAA_STATE_DIR");
        }

        #[test]
        #[ignore = "need installed MaaCore and resource"]
        fn data_relative() {
            let dirs = clean_dirs();

            #[cfg(target_os = "macos")]
            {
                assert_eq!(
                    dirs.data(),
                    home().join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(
                    dirs.library(),
                    home().join("Library/Application Support/com.loong.maa/lib")
                );
                assert_eq!(
                    dirs.resource(),
                    home().join("Library/Application Support/com.loong.maa/resource")
                );
                assert_eq!(
                    dirs.hot_update(),
                    home().join("Library/Application Support/com.loong.maa/MaaResource")
                );
            }

            #[cfg(target_os = "linux")]
            {
                assert_eq!(dirs.data(), home().join(".local/share/maa"));
                assert_eq!(dirs.library(), home().join(".local/share/maa/lib"));
                assert_eq!(dirs.resource(), home().join(".local/share/maa/resource"));
                assert_eq!(
                    dirs.hot_update(),
                    home().join(".local/share/maa/MaaResource")
                );
            }

            assert_eq!(data(), dirs.data());
            assert_eq!(library(), dirs.library());
            assert_eq!(resource(), dirs.resource());
            assert_eq!(hot_update(), dirs.hot_update());

            // Test find_library and find_resource at standard location.
            // So we need MaaCore installed at standard location.
            if env::var_os("SKIP_CORE_TEST").is_none() {
                let exe_path = Path::new("/path/to/maa");
                assert_eq!(
                    clean_dirs().find_library(exe_path).unwrap(),
                    clean_dirs().library()
                );
                assert_eq!(
                    clean_dirs().find_resource(exe_path).unwrap(),
                    clean_dirs().resource()
                );
                assert_eq!(find_library().unwrap(), library());
                assert_eq!(find_resource().unwrap(), resource());
            }

            env::set_var("XDG_DATA_HOME", "/xdg");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.data(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/xdg/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/xdg/maa/resource"));
            env::remove_var("XDG_DATA_HOME");

            env::set_var("MAA_DATA_DIR", "/maa");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.data(), PathBuf::from("/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/maa/resource"));
            env::remove_var("MAA_DATA_DIR");

            env::set_var("XDG_DATA_HOME", "/xdg");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.data(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/xdg/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/xdg/maa/resource"));
            env::remove_var("XDG_DATA_HOME");

            env::set_var("MAA_DATA_DIR", "/maa");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.data(), PathBuf::from("/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/maa/resource"));
            env::remove_var("MAA_DATA_DIR");

            // Test find_library and find_resource at non-standard location (in a test directory)
            // We need to use the provius dirs since the data directory is not at its standard
            // location, to avoid find_library and find_resource returning the standard
            // location.
            let test_root = temp_dir().join("maa-dirs-test-data-relative");
            test_root.ensure_clean().unwrap();
            let test_root = canonicalize(test_root).unwrap();

            fn prepare_files(bin: &Path, lib: &Path, resource: &Path) {
                let bin_dir = bin.parent().expect("Failed to get bin directory");
                let lib_dir = lib.parent().expect("Failed to get lib directory");
                bin_dir.ensure().expect("Failed to create bin directory");
                lib_dir.ensure().expect("Failed to create lib directory");
                std::fs::File::create(bin).expect("Failed to create bin file");
                std::fs::File::create(lib).expect("Failed to create lib file");
                create_dir_all(resource).expect("Failed to create resource directory");
            }

            // Test flat directory structure.
            let bin = join!(&test_root, "maa");
            let lib = join!(&test_root, MAA_CORE_LIB);
            let resource = join!(&test_root, "resource");
            test_root.ensure_clean().unwrap();
            prepare_files(&bin, &lib, &resource);
            assert_eq!(dirs.find_library(&bin).unwrap(), lib.parent().unwrap());
            assert_eq!(dirs.find_resource(&bin).unwrap(), resource);

            // Test unix-like directory structure.
            let bin = join!(&test_root, "bin", "maa");
            let lib = join!(&test_root, "lib", MAA_CORE_LIB);
            let resource = join!(&test_root, "share", "maa", "resource");
            test_root.ensure_clean().unwrap();
            prepare_files(&bin, &lib, &resource);
            assert_eq!(dirs.find_library(&bin).unwrap(), lib.parent().unwrap());
            assert_eq!(dirs.find_resource(&bin).unwrap(), resource);

            // Test unix-like directory structure but resource has a different name.
            if let Some(name) = option_env!("MAA_EXTRA_SHARE_NAME") {
                let bin = join!(&test_root, "bin", "maa");
                let lib = join!(&test_root, "lib", MAA_CORE_LIB);
                let resource = join!(&test_root, "share", name, "resource");
                test_root.ensure_clean().unwrap();
                prepare_files(&bin, &lib, &resource);
                assert_eq!(dirs.find_library(&bin).unwrap(), lib.parent().unwrap());
                assert_eq!(dirs.find_resource(&bin).unwrap(), resource);
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                // Test homebrew-like directory structure.
                let bin = join!(&test_root, "Cellar", "bin", "maa");
                let lib = join!(&test_root, "lib", MAA_CORE_LIB);
                let resource = join!(&test_root, "share", "maa", "resource");
                test_root.ensure_clean().unwrap();
                prepare_files(&bin, &lib, &resource);
                let exe = join!(&test_root, "bin", "maa");
                let exe_dir = exe.parent().expect("Failed to get exe directory");
                exe_dir.ensure().expect("Failed to create exe directory");
                symlink(&bin, &exe).expect("Failed to create symbolic link");
                assert_eq!(dirs.find_library(&exe).unwrap(), lib.parent().unwrap());
                assert_eq!(dirs.find_resource(&exe).unwrap(), resource);

                // Test maa is linked to a different directory.
                let bin = join!(&test_root, "bin", "maa");
                let lib = join!(&test_root, "lib", MAA_CORE_LIB);
                let resource = join!(&test_root, "share", "maa", "resource");
                test_root.ensure_clean().unwrap();
                prepare_files(&bin, &lib, &resource);
                let exe = join!(&test_root, "other", "bin", "maa");
                let exe_dir = exe.parent().expect("Failed to get exe directory");
                exe_dir.ensure().expect("Failed to create exe directory");
                symlink(&bin, &exe).expect("Failed to create symbolic link");
                assert_eq!(dirs.find_library(&exe).unwrap(), lib.parent().unwrap());
                assert_eq!(dirs.find_resource(&exe).unwrap(), resource);
            }

            remove_dir_all(&test_root).unwrap();
        }

        #[test]
        fn config_relative() {
            let dirs = clean_dirs();
            #[cfg(target_os = "macos")]
            {
                assert_eq!(
                    dirs.config(),
                    home().join("Library/Application Support/com.loong.maa/config")
                );
            }
            #[cfg(target_os = "linux")]
            {
                assert_eq!(dirs.config(), home().join(".config/maa"));
            }
            assert_eq!(config(), clean_dirs().config());

            assert_eq!(
                clean_dirs().abs_config::<&str, &str>("foo", None).unwrap(),
                clean_dirs().config().join("foo")
            );
            assert_eq!(
                clean_dirs().abs_config("foo", Some("bar")).unwrap(),
                join!(clean_dirs().config(), "bar", "foo")
            );
            #[cfg(unix)]
            {
                assert_eq!(clean_dirs().abs_config::<&str, &str>("/tmp", None), None);
                assert_eq!(clean_dirs().abs_config("/tmp", Some("bar")), None);
            }

            env::set_var("XDG_CONFIG_HOME", "/xdg");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.config(), PathBuf::from("/xdg/maa"));

            env::set_var("MAA_CONFIG_DIR", "/maa");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.config(), PathBuf::from("/maa"));
        }

        #[test]
        fn cache_relative() {
            let dirs = clean_dirs();
            #[cfg(target_os = "macos")]
            {
                assert_eq!(dirs.cache(), home().join("Library/Caches/com.loong.maa"));
                assert_eq!(
                    dirs.copilot(),
                    home().join("Library/Caches/com.loong.maa/copilot")
                );
            }
            #[cfg(target_os = "linux")]
            {
                assert_eq!(dirs.cache(), home().join(".cache/maa"));
                assert_eq!(dirs.copilot(), home().join(".cache/maa/copilot"));
            }
            assert_eq!(cache(), clean_dirs().cache());
            assert_eq!(copilot(), clean_dirs().copilot());

            env::set_var("XDG_CACHE_HOME", "/xdg");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.cache(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.copilot(), PathBuf::from("/xdg/maa/copilot"));

            env::set_var("MAA_CACHE_DIR", "/maa");
            let dirs = Dirs::new(project());
            assert_eq!(dirs.cache(), PathBuf::from("/maa"));
            assert_eq!(dirs.copilot(), PathBuf::from("/maa/copilot"));
        }
    }

    #[test]
    fn test_expand_tilde() {
        assert_eq!(expand_tilde(Path::new("~")), home());
        assert_eq!(
            expand_tilde(Path::new("~/foo")),
            home().join("foo").as_path()
        );
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

        assert_eq!(global_path([&test_dir1, &test_dir2], "test"), vec![
            test_file.clone()
        ]);
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
