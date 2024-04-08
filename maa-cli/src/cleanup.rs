use crate::{
    dirs::{cache, log, state},
    value::userinput::{BoolInput, UserInput},
};

use std::{
    borrow::Cow,
    fs::{read_dir, DirEntry},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{bail, Result};
use log::warn;

pub trait PathProvider {
    /// Path to a directory to be cleaned up
    fn target_dir(&self) -> Cow<Path>;

    /// Determine whether an entry in the directory should be deleted
    ///
    /// Default implementation always returns true, meaning all files and directories will be deleted.
    #[allow(unused_variables)]
    fn should_delete(&self, entry: &DirEntry) -> bool {
        true
    }

    /// Determine whether an entry in the directory should be kept
    ///
    /// Default implementation always returns false, meaning no files and directories will be kept.
    /// This method has higher priority than `should_delete`, meaning if this method returns true,
    /// the entry will not be deleted even if `should_delete` returns true.
    #[allow(unused_variables)]
    fn should_keep(&self, entry: &DirEntry) -> bool {
        false
    }
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum CleanupTarget {
    /// Cache files for maa-cli
    CliCache,
    /// Cache files for MaaCore
    CoreCache,
    /// Debug files
    Debug,
    /// Log files (both for MaaCore and maa-cli)
    Log,
    /// Deprecated, operator avatar cache, will be removed in the future, use core-cache instead
    Avatars,
    /// Deprecated, uncatagorized debug files, will be removed in the future, use debug instead
    Misc,
}

use CleanupTarget::*;

impl PathProvider for CleanupTarget {
    fn target_dir(&self) -> Cow<Path> {
        // Show warning for deprecated targets
        match *self {
            Avatars => warn!("Cleanup target avatars is deprecated, use core-cache instead."),
            Misc => warn!("Cleanup target misc is deprecated, use debug instead."),
            _ => {}
        }
        match *self {
            CliCache => cache().into(),
            CoreCache | Avatars => join!(state(), "cache").into(),
            Debug | Log | Misc => log().into(),
        }
    }

    fn should_delete(&self, entry: &DirEntry) -> bool {
        match self {
            Log => match entry.file_type() {
                Ok(file_type) if file_type.is_file() => entry
                    .file_name()
                    .to_str()
                    .is_some_and(|x| matches!(x, "asst.log" | "asst.bak.log")),
                Ok(file_type) if file_type.is_dir() => {
                    entry.file_name().to_str().is_some_and(|x| {
                        x.starts_with("20") && x.len() == 4 && x.chars().all(|c| c.is_numeric())
                    })
                }
                _ => false,
            },
            Avatars => {
                entry.file_type().is_ok_and(|x| x.is_dir())
                    && entry.file_name().to_str().is_some_and(|x| x == "avatars")
            }
            Misc => {
                entry.file_type().is_ok_and(|x| x.is_dir())
                    && entry
                        .file_name()
                        .to_str()
                        .is_some_and(|x| matches!(x, "drops" | "map" | "other" | "Roguelike"))
            }
            _ => true,
        }
    }

    fn should_keep(&self, entry: &DirEntry) -> bool {
        match self {
            #[cfg(feature = "core_installer")]
            CliCache => {
                use crate::installer::maa_core;

                // Cache the name of the core package to avoid repeated calls
                static CORE_CACHE_NAME: OnceLock<Option<String>> = OnceLock::new();
                let name = CORE_CACHE_NAME.get_or_init(|| {
                    maa_core::version()
                        .and_then(|version| maa_core::name(&version))
                        .ok()
                });

                name.as_deref().is_some_and(|name| {
                    entry.file_type().is_ok_and(|x| x.is_file())
                        && entry.file_name().to_str().is_some_and(|x| x == name)
                })
            }
            _ => false,
        }
    }
}

/// Clean up files and directories
pub fn cleanup<T>(targets: &[T]) -> Result<()>
where
    T: PathProvider,
{
    // If no targets are specified, clean up all
    if targets.is_empty() {
        return cleanup(&[CliCache, CoreCache, Debug]);
    }

    let target_paths: Vec<PathBuf> = targets
        .iter()
        .flat_map(|target| {
            let target_dir = target.target_dir();
            if let Ok(dir) = read_dir(target_dir.as_ref()) {
                dir.filter_map(|entry| {
                    let entry = entry.ok()?;
                    if target.should_delete(&entry) && !target.should_keep(&entry) {
                        Some(entry.path())
                    } else {
                        None
                    }
                })
                .collect()
            } else {
                Vec::new()
            }
        })
        .collect();

    if target_paths.is_empty() {
        println!("No files or directories to clean up.");
        return Ok(());
    }

    for (i, p) in target_paths.iter().enumerate() {
        println!("{}. {}", i + 1, p.display());
    }

    if !BoolInput::new(Some(true), Some("clear files or folders mentioned above")).value()? {
        println!("Canceled.");
        return Ok(());
    }

    let mut has_err = false;

    for path in target_paths {
        print!("Deleting {}", path.display());
        if let Err(e) = del_item(&path) {
            println!(", \x1B[31mfailed\x1B[0m: {}", e);
            has_err = true;
        } else {
            println!(", \x1B[32msuccess\x1B[0m.");
        }
    }

    if has_err {
        bail!("Some errors occurred during cleanup, at least one file or directory failed to be deleted.");
    }

    Ok(())
}

/// Delete a file or directory
fn del_item(path: &Path) -> Result<(), std::io::Error> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dirs::Ensure;

    use std::env::temp_dir;

    mod cleanup_target {
        use super::*;

        #[test]
        fn target_dir() {
            assert_eq!(CliCache.target_dir(), cache());
            assert_eq!(CoreCache.target_dir(), join!(state(), "cache"));
            assert_eq!(Avatars.target_dir(), join!(state(), "cache"));
            assert_eq!(Debug.target_dir(), log());
            assert_eq!(Log.target_dir(), log());
            assert_eq!(Misc.target_dir(), log());
        }

        #[test]
        fn should_delete() {
            let test_root = join!(temp_dir(), "maa-cli-test-should-delete");

            test_root.ensure().unwrap();

            let test_entry = |target: CleanupTarget, is_dir: bool, name: &str| -> bool {
                let path = join!(&test_root, name);
                if is_dir {
                    std::fs::create_dir(&path).unwrap();
                } else {
                    std::fs::File::create(&path).unwrap();
                }
                let entry = test_root.read_dir().unwrap().next().unwrap().unwrap();
                let ret = target.should_delete(&entry);
                del_item(&path).unwrap();
                ret
            };

            // Create a directory with some files and subdirectories
            std::fs::create_dir_all(&test_root).unwrap();

            assert!(test_entry(CliCache, false, "test"));

            assert!(test_entry(Log, false, "asst.log"));
            assert!(test_entry(Log, false, "asst.bak.log"));
            assert!(!test_entry(Log, false, "test"));

            assert!(test_entry(Log, true, "2024"));
            assert!(!test_entry(Log, true, "20A4"));
            assert!(!test_entry(Log, true, "2024-01-01"));

            assert!(test_entry(Avatars, true, "avatars"));
            assert!(!test_entry(Avatars, false, "avatars"));

            assert!(test_entry(Misc, true, "drops"));
            assert!(test_entry(Misc, true, "map"));
            assert!(test_entry(Misc, true, "other"));
            assert!(test_entry(Misc, true, "Roguelike"));
            assert!(!test_entry(Misc, false, "test"));

            std::fs::remove_dir(&test_root).unwrap();
        }

        #[test]
        fn should_keep() {
            let test_root = join!(temp_dir(), "maa-cli-test-should-keep");

            test_root.ensure().unwrap();

            // Create a directory with some files and subdirectories
            std::fs::create_dir_all(&test_root).unwrap();

            let test_entry = |target: CleanupTarget, is_dir: bool, name: &str| -> bool {
                let path = join!(&test_root, name);
                if is_dir {
                    std::fs::create_dir(&path).unwrap();
                } else {
                    std::fs::File::create(&path).unwrap();
                }
                let entry = test_root.read_dir().unwrap().next().unwrap().unwrap();
                let ret = target.should_keep(&entry);
                del_item(&path).unwrap();
                ret
            };

            assert!(!test_entry(CoreCache, false, "test"));

            #[cfg(feature = "core_installer")]
            {
                assert!(!test_entry(CliCache, false, "test"));
                if let Some(version) = std::env::var_os("MAA_CORE_VERSION") {
                    use crate::installer::maa_core;
                    let name =
                        maa_core::name(&version.to_str().unwrap()[1..].parse().unwrap()).unwrap();
                    assert!(test_entry(CliCache, false, &name));
                }
            }

            std::fs::remove_dir(&test_root).unwrap();
        }
    }

    #[test]
    fn test_cleanup() {
        enum TestTarget {
            All,
            // Entries in the blacklist will be deleted
            BlackList(Vec<&'static str>),
            // Entries in the whitelist will not be deleted
            WhiteList(Vec<&'static str>),
        }

        impl PathProvider for TestTarget {
            fn target_dir(&self) -> Cow<Path> {
                join!(temp_dir(), "maa-cli-test-cleanup").into()
            }

            fn should_delete(&self, entry: &DirEntry) -> bool {
                match self {
                    TestTarget::BlackList(list) => entry
                        .file_name()
                        .to_str()
                        .is_some_and(|x| list.contains(&x)),
                    _ => true,
                }
            }

            fn should_keep(&self, entry: &DirEntry) -> bool {
                match self {
                    TestTarget::WhiteList(list) => entry
                        .file_name()
                        .to_str()
                        .is_some_and(|x| list.contains(&x)),
                    _ => false,
                }
            }
        }

        let test_root = join!(temp_dir(), "maa-cli-test-cleanup");

        test_root.ensure().unwrap();

        let create_test_files = || {
            let test_files = ["test1", "test2", "test3"];
            for file in &test_files {
                std::fs::File::create(join!(&test_root, file)).unwrap();
            }
        };

        create_test_files();
        cleanup(&[TestTarget::All]).unwrap();
        assert!(test_root.read_dir().unwrap().next().is_none());

        create_test_files();
        cleanup(&[TestTarget::BlackList(vec!["test1", "test2"])]).unwrap();
        assert_eq!(
            test_root
                .read_dir()
                .unwrap()
                .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            vec!["test3"]
        );
        cleanup(&[TestTarget::All]).unwrap();

        create_test_files();
        cleanup(&[TestTarget::WhiteList(vec!["test1", "test2"])]).unwrap();
        assert_eq!(
            test_root
                .read_dir()
                .unwrap()
                .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            vec!["test1", "test2"]
        );
        cleanup(&[TestTarget::All]).unwrap();

        std::fs::remove_dir(&test_root).unwrap();
    }

    #[test]
    fn test_del_item() {
        let test_root = join!(temp_dir(), "maa-cli-test-del-item");

        test_root.ensure().unwrap();

        let test_file = join!(&test_root, "test");
        std::fs::File::create(&test_file).unwrap();
        assert!(del_item(&test_file).is_ok());
        assert!(!test_file.exists());

        let test_dir = join!(&test_root, "test");
        std::fs::create_dir(&test_dir).unwrap();
        assert!(del_item(&test_dir).is_ok());
        assert!(!test_dir.exists());

        std::fs::remove_dir(&test_root).unwrap();
    }

    #[test]
    #[ignore]
    fn test_cleanup_real_files() {
        // Create some files for testing
        cache().ensure().unwrap();
        state().ensure().unwrap();
        log().ensure().unwrap();

        std::fs::File::create(join!(cache(), "test.tar.gz")).unwrap();

        #[cfg(feature = "core_installer")]
        {
            use crate::installer::maa_core;
            use semver::Version;

            std::fs::File::create(join!(
                cache(),
                maa_core::name(&Version::new(0, 0, 1)).unwrap()
            ))
            .unwrap();

            if let Some(version) = std::env::var_os("MAA_CORE_VERSION") {
                let name =
                    maa_core::name(&version.to_str().unwrap()[1..].parse().unwrap()).unwrap();
                std::fs::File::create(join!(state(), "cache", &name)).unwrap();
            }

            std::fs::File::create(join!(state(), "cache", "avatars")).unwrap();
            std::fs::create_dir_all(join!(log(), "2024")).unwrap();
            std::fs::File::create(join!(log(), "asst.log")).unwrap();
            std::fs::File::create(join!(log(), "asst.bak.log")).unwrap();

            let target: Vec<CleanupTarget> = Vec::new();
            cleanup(&target).unwrap();

            assert!(!join!(cache(), "test.tar.gz").exists());
            assert!(!join!(state(), "cache", "avatars").exists());
            assert!(!join!(log(), "2024").exists());
            assert!(!join!(log(), "asst.log").exists());
            assert!(!join!(log(), "asst.bak.log").exists());
        }
    }
}
