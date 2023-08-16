use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::Dirs;

#[derive(Debug)]
pub struct XDGProject {
    pub name: String,
}

fn env_path(var: impl AsRef<OsStr>) -> Option<PathBuf> {
    env::var_os(var).and_then(|path| match path {
        path if path.is_empty() => None,
        path => Some(PathBuf::from(path)),
    })
}

fn home_dir(path: impl AsRef<Path>) -> Option<PathBuf> {
    return env_path("HOME").map(|home| home.join(path));
}

fn env_path_or_default<V, N>(var: V, default: Option<PathBuf>, name: N) -> Option<PathBuf>
where
    V: AsRef<OsStr>,
    N: AsRef<Path>,
{
    return match env_path(var) {
        Some(path) => Some(path.join(name)),
        None => default.map(|path| path.join(name)),
    };
}

impl XDGProject {
    pub fn new(name: String) -> XDGProject {
        return XDGProject { name };
    }
}

impl From<&str> for XDGProject {
    fn from(name: &str) -> Self {
        return XDGProject::new(String::from(name));
    }
}

impl Dirs for XDGProject {
    fn config_dir(&self) -> Option<PathBuf> {
        return env_path_or_default("XDG_CONFIG_HOME", home_dir(".config"), &self.name);
    }

    fn data_dir(&self) -> Option<PathBuf> {
        return env_path_or_default("XDG_DATA_HOME", home_dir(".local/share"), &self.name);
    }

    fn state_dir(&self) -> Option<PathBuf> {
        return env_path_or_default("XDG_STATE_HOME", home_dir(".local/state"), &self.name);
    }

    fn cache_dir(&self) -> Option<PathBuf> {
        return env_path_or_default("XDG_CACHE_HOME", home_dir(".cache"), &self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_with_env<F>(var: &str, value: Option<&str>, f: F)
    where
        F: FnOnce(),
    {
        match value {
            Some(value) => env::set_var(var, value),
            None => env::remove_var(var),
        }
        match env::var_os(var) {
            Some(val) => {
                if val.to_str() != value {
                    panic!("env var {} is set to {:?} instead of {:?}", var, val, value)
                }
            }
            None => {
                if value.is_some() && !value.unwrap().is_empty() {
                    panic!("env var {:?} is not set to {:?}", var, value)
                }
            }
        }

        f();
    }

    // NOTE: we must use different var names for each test,
    // otherwise the tests will interfere with each other

    #[test]
    fn test_env_path() {
        let var = "ENV_PATH_NONE";
        test_with_env(var, None, || assert_eq!(env_path(var), None));

        let var = "ENV_PATH_EMPTY";
        test_with_env("ENV_PATH_EMPTY", Some(""), || {
            assert_eq!(env_path(var), None)
        });

        let var = "ENV_PATH_NORMAL";
        test_with_env("ENV_PATH_NORMAL", Some("/foo"), || {
            assert_eq!(env_path(var), Some(PathBuf::from("/foo")))
        });
    }

    #[test]
    fn test_env_path_or_default() {
        let var = "ENV_PATH_OR_DEFAULT_NONE";
        test_with_env(var, None, || {
            assert_eq!(env_path_or_default(var, None, "test"), None)
        });

        let var = "ENV_PATH_OR_DEFAULT_NO_ENV";
        test_with_env(var, None, || {
            assert_eq!(
                env_path_or_default(var, Some(PathBuf::from("/tmp")), "test"),
                Some(PathBuf::from("/tmp/test"))
            )
        });
        let var = "ENV_PATH_OR_DEFAULT_NO_DEFAULT";
        test_with_env(var, Some("/foo"), || {
            assert_eq!(
                env_path_or_default(var, None, "test"),
                Some(PathBuf::from("/foo/test"))
            )
        });
        let var = "ENV_PATH_OR_DEFAULT_NORMAL";
        test_with_env(var, Some("/foo"), || {
            assert_eq!(
                env_path_or_default(var, Some(PathBuf::from("/tmp")), "test"),
                Some(PathBuf::from("/foo/test"))
            )
        });
    }

    #[test]
    fn test_xdg_dirs() {
        let xdg = XDGProject::from("test");

        let home = env::var_os("HOME").unwrap();
        let home_path = PathBuf::from(&home);

        test_with_env("XDG_CONFIG_HOME", None, || {
            assert_eq!(xdg.config_dir(), Some(home_path.join(".config/test")))
        });
        test_with_env("XDG_DATA_HOME", None, || {
            assert_eq!(xdg.data_dir(), Some(home_path.join(".local/share/test")))
        });
        test_with_env("XDG_STATE_HOME", None, || {
            assert_eq!(xdg.state_dir(), Some(home_path.join(".local/state/test")))
        });
        test_with_env("XDG_CACHE_HOME", None, || {
            assert_eq!(xdg.cache_dir(), Some(home_path.join(".cache/test")))
        });

        test_with_env("XDG_CONFIG_HOME", Some("/xdg/config"), || {
            assert_eq!(xdg.config_dir(), Some(PathBuf::from("/xdg/config/test")))
        });
        test_with_env("XDG_DATA_HOME", Some("/xdg/data"), || {
            assert_eq!(xdg.data_dir(), Some(PathBuf::from("/xdg/data/test")))
        });
        test_with_env("XDG_STATE_HOME", Some("/xdg/state"), || {
            assert_eq!(xdg.state_dir(), Some(PathBuf::from("/xdg/state/test")))
        });
        test_with_env("XDG_CACHE_HOME", Some("/xdg/cache"), || {
            assert_eq!(xdg.cache_dir(), Some(PathBuf::from("/xdg/cache/test")))
        });
    }
}
