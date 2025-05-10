#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::{
    borrow::Cow,
    env::consts,
    ffi::{OsStr, OsString},
    fs::{create_dir, create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use directories::ProjectDirs;
use dunce::canonicalize;

pub const MAA_CLI_NAME: &str = "maa";
pub const MAA_CLI_EXE: &str = constcat::concat!(MAA_CLI_NAME, consts::EXE_SUFFIX);

/// The name of the MaaCore library.
pub const MAA_CORE_NAME: &str = "MaaCore";
/// The name of the MaaCore library with the platform-specific prefix and suffix.
pub const MAA_CORE_LIB: &str =
    constcat::concat!(consts::DLL_PREFIX, MAA_CORE_NAME, consts::DLL_SUFFIX);

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
/// Thus, if you want to keep the ownership of the first path, you should pass a reference.
///
/// # Examples
///
/// ```rust
/// use std::path::PathBuf;
///
/// use maa_dirs::join;
///
/// let path = PathBuf::from("foo");
/// // The path will not be consumed.
/// let p1 = join!(&path, "bar", "baz");
/// assert_eq!(p1, PathBuf::from("foo/bar/baz"));
///
/// // The path will be consumed.
/// let p2 = join!(path, "bar", "baz");
/// assert_eq!(p2, PathBuf::from("foo/bar/baz"));
/// ```
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

// Use this trait to make tests more easily.
trait VarOs {
    /// Get the value of the variable `key` as an `OsString`.
    fn var_os(self, key: impl AsRef<OsStr>) -> Option<OsString>;
}

/// A `VarOs` implementation that gets the value from system environment variables.
#[derive(Clone, Copy)]
struct EnvVarOs;

impl VarOs for EnvVarOs {
    fn var_os(self, key: impl AsRef<OsStr>) -> Option<OsString> {
        std::env::var_os(key)
    }
}

/// Get the directory from environment variables.
///
/// The `maa_env` usually is `MAA_XXX_DIR`, and the `xdg_env` usually is `XDG_XXX_HOME`.
/// If the `maa_env` is set, return the directory `maa_env`.
/// If the `xdg_env` is set, return the directory `xdg_env/maa`.
/// Otherwise, return `None`.
fn dir_from_env(v: impl VarOs + Copy, maa_env: &str, xdg_env: &str) -> Option<PathBuf> {
    v.var_os(maa_env)
        .map(PathBuf::from)
        .or_else(|| v.var_os(xdg_env).map(|xdg| join!(xdg, "maa")))
}

/// Get the data directory.
fn get_data_dir(v: impl VarOs + Copy, proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env(v, "MAA_DATA_DIR", "XDG_DATA_HOME")
        .or_else(|| proj.map(|dirs| dirs.data_dir().into()))
        .expect("Failed to get data directory!")
}

/// Get the state directory.
fn get_state_dir(v: impl VarOs + Copy, proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env(v, "MAA_STATE_DIR", "XDG_STATE_HOME")
        .or_else(|| proj.map(|dirs| dirs.state_dir().unwrap_or_else(|| dirs.data_dir()).into()))
        .expect("Failed to get state directory!")
}

/// Get the cache directory.
fn get_cache_dir(v: impl VarOs + Copy, proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env(v, "MAA_CACHE_DIR", "XDG_CACHE_HOME")
        .or_else(|| proj.map(|dirs| dirs.cache_dir().into()))
        .expect("Failed to get cache directory!")
}

/// Get the config directory.
fn get_config_dir(v: impl VarOs + Copy, proj: Option<&ProjectDirs>) -> PathBuf {
    dir_from_env(v, "MAA_CONFIG_DIR", "XDG_CONFIG_HOME")
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
    fn new(proj: Option<&ProjectDirs>) -> Self {
        Self::new_inner(proj, EnvVarOs)
    }

    fn new_inner(proj: Option<&ProjectDirs>, v: impl VarOs + Copy) -> Self {
        let data_dir = get_data_dir(v, proj);
        let state_dir = get_state_dir(v, proj);
        let cache_dir = get_cache_dir(v, proj);

        Self {
            copilot: cache_dir.join("copilot"),
            cache: cache_dir,
            config: get_config_dir(v, proj),
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

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "loong";
const APPLICATION: &str = "maa";

static DIRS: LazyLock<Dirs> =
    LazyLock::new(|| Dirs::new(ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).as_ref()));

fn current_exe() -> Option<&'static Path> {
    static CURRENT_EXE: LazyLock<Option<PathBuf>> = LazyLock::new(|| std::env::current_exe().ok());
    CURRENT_EXE.as_deref()
}

pub fn data() -> &'static Path {
    DIRS.data()
}

pub fn library() -> &'static Path {
    DIRS.library()
}

pub fn find_library() -> Option<Cow<'static, Path>> {
    DIRS.find_library(current_exe()?)
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

pub fn find_resource() -> Option<Cow<'static, Path>> {
    DIRS.find_resource(current_exe()?)
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

fn home() -> &'static Path {
    static HOME: LazyLock<PathBuf> = LazyLock::new(|| {
        directories::BaseDirs::new()
            .expect("Failed to get home directory")
            .home_dir()
            .to_path_buf()
    });

    HOME.as_ref()
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::env;

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

    #[test]
    fn env_var_os() {
        use std::env;

        // Safety: the variable name is safe to avoid conflicts and only modified in this thread
        unsafe {
            const VAR: &str = "DEAWPMONUBYASDCOPBH";

            env::set_var(VAR, "foo");
            assert_eq!(EnvVarOs.var_os(VAR), Some(OsString::from("foo")));

            env::remove_var(VAR);
            assert_eq!(EnvVarOs.var_os(VAR), None);
        }
    }

    mod get_dir {
        use std::{fs::create_dir_all, sync::Once};

        use super::*;

        struct MockVarOs {
            vars: std::collections::BTreeMap<OsString, OsString>,
        }

        impl VarOs for &MockVarOs {
            fn var_os(self, key: impl AsRef<OsStr>) -> Option<OsString> {
                self.vars.get(key.as_ref()).cloned()
            }
        }

        impl MockVarOs {
            fn new() -> Self {
                Self {
                    vars: std::collections::BTreeMap::new(),
                }
            }

            fn with_var(mut self, key: &str, value: &str) -> Self {
                self.vars.insert(OsString::from(key), OsString::from(value));
                self
            }
        }

        impl From<Vec<(&str, &str)>> for MockVarOs {
            fn from(vars: Vec<(&str, &str)>) -> Self {
                let vars = vars
                    .into_iter()
                    .map(|(k, v)| (OsString::from(k), OsString::from(v)))
                    .collect();
                Self { vars }
            }
        }

        /// Clear all related environment variables to avoid pollution from parent process
        fn clear_env() {
            static CLRER: Once = Once::new();
            // env_remove_var is not thread-safe and will be marked as unsafe in rust edition 2024
            CLRER.call_once(|| unsafe {
                env::remove_var("XDG_DATA_HOME");
                env::remove_var("XDG_STATE_HOME");
                env::remove_var("XDG_CACHE_HOME");
                env::remove_var("XDG_CONFIG_HOME");
                env::remove_var("MAA_DATA_DIR");
                env::remove_var("MAA_STATE_DIR");
                env::remove_var("MAA_CACHE_DIR");
                env::remove_var("MAA_CONFIG_DIR");
            });
        }

        #[test]
        fn test_std_dirs() {
            clear_env();

            let home = home();

            #[cfg(target_os = "macos")]
            {
                assert_eq!(
                    data(),
                    home.join("Library/Application Support/com.loong.maa")
                );
                assert_eq!(state(), data());
                assert_eq!(cache(), home.join("Library/Caches/com.loong.maa"));
                assert_eq!(config(), data().join("config"));
            }

            #[cfg(target_os = "linux")]
            {
                assert_eq!(data(), home.join(".local/share/maa"));
                assert_eq!(state(), home.join(".local/state/maa"));
                assert_eq!(cache(), home.join(".cache/maa"));
                assert_eq!(config(), home.join(".config/maa"));
            }

            #[cfg(target_os = "windows")]
            {
                assert_eq!(data(), home.join("AppData\\Roaming\\loong\\maa\\data"));
                assert_eq!(state(), home.join("AppData\\Roaming\\loong\\maa\\data"));
                assert_eq!(cache(), home.join("AppData\\Local\\loong\\maa\\cache"));
                assert_eq!(config(), home.join("AppData\\Roaming\\loong\\maa\\config"));
            }

            assert_eq!(library(), data().join("lib"));
            assert_eq!(resource(), data().join("resource"));
            assert_eq!(hot_update(), data().join("MaaResource"));
            assert_eq!(copilot(), cache().join("copilot"));
            assert_eq!(log(), state().join("debug"));
        }

        #[test]
        #[ignore = "need installed MaaCore and resource"]
        fn find_std_dirs() {
            if env::var_os("SKIP_CORE_TEST").is_some() {
                return;
            }

            clear_env();

            assert_eq!(find_library().unwrap(), library());

            assert_eq!(find_resource().unwrap(), resource());
        }

        static PROJECT: LazyLock<Option<ProjectDirs>> =
            LazyLock::new(|| ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION));

        #[test]
        fn data_dir() {
            // Test with XDG_DATA_HOME set
            let mock = MockVarOs::new().with_var("XDG_DATA_HOME", "/xdg");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.data(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/xdg/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/xdg/maa/resource"));
            assert_eq!(dirs.hot_update(), PathBuf::from("/xdg/maa/MaaResource"));

            // Test with MAA_DATA_DIR set
            let mock = MockVarOs::new().with_var("MAA_DATA_DIR", "/maa");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.data(), PathBuf::from("/maa"));
            assert_eq!(dirs.library(), PathBuf::from("/maa/lib"));
            assert_eq!(dirs.resource(), PathBuf::from("/maa/resource"));
            assert_eq!(dirs.hot_update(), PathBuf::from("/maa/MaaResource"));
        }

        #[test]
        fn find_dirs() {
            use std::fs::File;

            // Make sure library and resource are not found
            let dirs = Dirs::new_inner(
                PROJECT.as_ref(),
                &MockVarOs::new()
                    .with_var("XDG_DATA_HOME", "/xdg")
                    .with_var("XDG_CACHE_HOME", "/xdg")
                    .with_var("XDG_STATE_HOME", "/xdg")
                    .with_var("XDG_CONFIG_HOME", "/xdg"),
            );

            // Test flat directory structure, common in Windows
            // maa in the root
            // lib in the root
            // resource in the root
            {
                let root = tempfile::tempdir().expect("Failed to create temp dir");
                let root = canonicalize(root.path()).unwrap();
                let exe = join!(&root, MAA_CLI_EXE);
                let lib = join!(&root, MAA_CORE_LIB);
                let resource = join!(&root, "resource");

                File::create(&exe).expect("Failed to create exe file");
                File::create(&lib).expect("Failed to create lib file");
                create_dir_all(&resource).expect("Failed to create resource dir");

                assert_eq!(dirs.find_library(&exe).as_deref(), Some(root.as_path()));
                assert_eq!(
                    dirs.find_resource(&exe).as_deref(),
                    Some(resource.as_path())
                );
            }

            // Test unix-like layout
            // maa in the ./bin
            // MaaCore in the ./lib
            // resource in the ./share/maa/resource
            {
                let root = tempfile::tempdir().expect("Failed to create temp dir");
                let root = canonicalize(root.path()).unwrap();
                let bin_dir = join!(&root, "bin");
                let lib_dir = join!(&root, "lib");
                let resource_dir = join!(&root, "share", "maa", "resource");
                bin_dir.ensure().expect("Failed to create bin dir");
                lib_dir.ensure().expect("Failed to create lib dir");
                resource_dir
                    .ensure()
                    .expect("Failed to create resource dir");

                let exe = bin_dir.join(MAA_CLI_EXE);
                let lib = lib_dir.join(MAA_CORE_LIB);

                File::create(&exe).expect("Failed to create exe file");
                File::create(&lib).expect("Failed to create lib file");

                assert_eq!(dirs.find_library(&exe).as_deref(), Some(lib_dir.as_path()));
                assert_eq!(
                    dirs.find_resource(&exe).as_deref(),
                    Some(resource_dir.as_path())
                );
            }

            // Test with maa extra share name
            // maa in the ./bin
            // MaaCore in the ./lib
            // resource in the ./share/{MAA_EXTRA_SHARE_NAME}/resource
            if let Some(extra_share) = option_env!("MAA_EXTRA_SHARE_NAME") {
                let root = tempfile::tempdir().expect("Failed to create temp dir");
                let root = canonicalize(root.path()).unwrap();
                let bin_dir = join!(&root, "bin");
                let lib_dir = join!(&root, "lib");
                let resource_dir = join!(&root, "share", extra_share, "resource");
                bin_dir.ensure().expect("Failed to create bin dir");
                lib_dir.ensure().expect("Failed to create lib dir");
                resource_dir
                    .ensure()
                    .expect("Failed to create resource dir");

                let exe = bin_dir.join(MAA_CLI_EXE);
                let lib = lib_dir.join(MAA_CORE_LIB);

                File::create(&exe).expect("Failed to create exe file");
                File::create(&lib).expect("Failed to create lib file");

                assert_eq!(dirs.find_library(&exe).as_deref(), Some(lib_dir.as_path()));
                assert_eq!(
                    dirs.find_resource(&exe).as_deref(),
                    Some(resource_dir.as_path())
                );
            }

            // Test homebrew-like layout
            // maa in a isolated cellar directory ./cellar/maa-cli/bin
            // MaaCore in a isolated cellar directory ./cellar/maa-core/lib
            // resource in a isolated cellar directory ./cellar/maa-core/share/maa/resource
            #[cfg(unix)]
            {
                let root = tempfile::tempdir().expect("Failed to create temp dir");
                let root = canonicalize(root.path()).unwrap();
                let bin_dir = join!(&root, "bin");
                let lib_dir = join!(&root, "lib");
                let resource_dir = join!(&root, "share", "maa", "resource");

                bin_dir.ensure().expect("Failed to create bin dir");
                lib_dir.ensure().expect("Failed to create lib dir");
                resource_dir
                    .parent()
                    .unwrap()
                    .ensure()
                    .expect("Failed to create resource dir");

                let cellar_dir = join!(&root, "Cellar");
                let maa_cli_cellar = join!(&cellar_dir, "maa-cli");
                let maa_core_cellar = join!(&cellar_dir, "maa-core");
                let maa_cli_bin_dir = join!(&maa_cli_cellar, "bin");
                let maa_core_lib_dir = join!(&maa_core_cellar, "lib");
                let maa_core_resource_dir = join!(&maa_core_cellar, "share", "maa", "resource");

                maa_cli_bin_dir
                    .ensure()
                    .expect("Failed to create maa-cli bin dir");
                maa_core_lib_dir
                    .ensure()
                    .expect("Failed to create maa-core lib dir");
                maa_core_resource_dir
                    .ensure()
                    .expect("Failed to create maa-core resource dir");

                let maa_cli_exe = maa_cli_bin_dir.join(MAA_CLI_EXE);
                let maa_core_lib = maa_core_lib_dir.join(MAA_CORE_LIB);

                File::create(&maa_cli_exe).expect("Failed to create maa-cli exe file");
                File::create(&maa_core_lib).expect("Failed to create maa-core lib file");

                // create symbolic link
                use std::os::unix::fs::symlink;
                let exe = bin_dir.join(MAA_CLI_EXE);
                let lib = lib_dir.join(MAA_CORE_LIB);
                symlink(&maa_cli_exe, &exe).expect("Failed to create symbolic link");
                symlink(&maa_core_lib, &lib).expect("Failed to create symbolic link");
                symlink(&maa_core_resource_dir, &resource_dir)
                    .expect("Failed to create symbolic link");

                assert_eq!(dirs.find_library(&exe).as_deref(), Some(lib_dir.as_path()));
                assert_eq!(
                    dirs.find_resource(&exe).as_deref(),
                    Some(resource_dir.as_path())
                );
            }
        }

        #[test]
        fn state_dir() {
            // Test with XDG_STATE_HOME set
            let mock = MockVarOs::new().with_var("XDG_STATE_HOME", "/xdg");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.state(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/xdg/maa/debug"));

            // Test with MAA_STATE_DIR set
            let mock = MockVarOs::new().with_var("MAA_STATE_DIR", "/maa");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.state(), PathBuf::from("/maa"));
            assert_eq!(dirs.log(), PathBuf::from("/maa/debug"));
        }

        #[test]
        fn cache_dir() {
            // Test with XDG_CACHE_HOME set
            let mock = MockVarOs::new().with_var("XDG_CACHE_HOME", "/xdg");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.cache(), PathBuf::from("/xdg/maa"));
            assert_eq!(dirs.copilot(), PathBuf::from("/xdg/maa/copilot"));

            // Test with MAA_CACHE_DIR set
            let mock = MockVarOs::new().with_var("MAA_CACHE_DIR", "/maa");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.cache(), PathBuf::from("/maa"));
            assert_eq!(dirs.copilot(), PathBuf::from("/maa/copilot"));
        }

        #[test]
        fn config_dir() {
            // Test with XDG_CONFIG_HOME set
            let mock = MockVarOs::new().with_var("XDG_CONFIG_HOME", "/xdg");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.config(), PathBuf::from("/xdg/maa"));

            // Test with MAA_CONFIG_DIR set
            let mock = MockVarOs::new().with_var("MAA_CONFIG_DIR", "/maa");
            let dirs = Dirs::new_inner(PROJECT.as_ref(), &mock);
            assert_eq!(dirs.config(), PathBuf::from("/maa"));
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
        let test_root = tempfile::tempdir().expect("Failed to create temp dir");

        let test_root = test_root.path();
        let test_dir = test_root.join("test");
        assert_eq!(test_root.ensure_clean().unwrap(), test_root);
        assert!(!test_dir.exists());
        assert_eq!(test_dir.ensure().unwrap(), test_dir);
        assert!(test_dir.exists());
    }

    #[test]
    fn global_path_and_find() {
        let test_root = tempfile::tempdir().expect("Failed to create temp dir");
        let test_root = test_root.path();
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
