use std::{
    borrow::Cow,
    fs::{DirEntry, read_dir},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use anyhow::{Result, bail};

use crate::{
    dirs::{cache, log, state},
    value::userinput::{BoolInput, UserInput},
};

pub trait PathProvider {
    /// Path to a directory to be cleaned up
    fn target_dir(&self) -> Cow<'_, Path>;

    /// Determine whether an entry in the directory should be deleted
    ///
    /// Default implementation always returns true, meaning all files and directories will be
    /// deleted. This method and `should_keep` determine whether an entry should be deleted.
    /// If this method returns true and `should_keep` returns false, the entry will be deleted.
    /// Otherwise, the entry will not be deleted.
    #[expect(
        unused_variables,
        reason = "This is default implementation, the variable may used by other implementations"
    )]
    fn should_delete(&self, entry: &DirEntry) -> bool {
        true
    }

    /// Determine whether an entry in the directory should be kept
    ///
    /// Default implementation always returns false, meaning no files and directories will be kept.
    /// This method and `should_delete` determine whether an entry should be deleted.
    /// If `should_delete` returns true and this method returns false, the entry will be deleted.
    /// Otherwise, the entry will not be deleted.
    #[expect(
        unused_variables,
        reason = "This is default implementation, the variable may used by other implementations"
    )]
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
    /// Debug files (including log and other debug files)
    Debug,
    /// Log files (both for MaaCore and maa-cli)
    Log,
}

use CleanupTarget::*;

impl PathProvider for CleanupTarget {
    fn target_dir(&self) -> Cow<'_, Path> {
        match *self {
            CliCache => cache().into(),
            CoreCache => join!(state(), "cache").into(),
            Debug | Log => log().into(),
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
            _ => true,
        }
    }

    fn should_keep(&self, entry: &DirEntry) -> bool {
        match self {
            #[cfg(feature = "core_installer")]
            CliCache => {
                use crate::installer::maa_core::this_asset_name;

                // Cache the name of the core package to avoid repeated calls
                static CORE_CACHE_NAME: LazyLock<Option<String>> =
                    LazyLock::new(|| crate::state::CORE_VERSION.as_ref().map(this_asset_name));

                CORE_CACHE_NAME.as_deref().is_some_and(|name| {
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
            println!(", \x1B[31mfailed\x1B[0m: {e}");
            has_err = true;
        } else {
            println!(", \x1B[32msuccess\x1B[0m.");
        }
    }

    if has_err {
        bail!(
            "Some errors occurred during cleanup, at least one file or directory failed to be deleted."
        );
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{
        collections::BTreeSet,
        env::{temp_dir, var_os},
    };

    use super::*;
    use crate::dirs::Ensure;

    mod cleanup_target {
        use super::*;

        #[test]
        fn target_dir() {
            assert_eq!(CliCache.target_dir(), cache());
            assert_eq!(CoreCache.target_dir(), join!(state(), "cache"));
            assert_eq!(Debug.target_dir(), log());
            assert_eq!(Log.target_dir(), log());
        }

        fn create_target_entry(dir: &Path, name: &str) -> Result<DirEntry> {
            let path = dir.join(name);
            if name.ends_with('/') {
                std::fs::create_dir(&path)?;
            } else {
                std::fs::File::create(&path)?;
            };

            for entry in dir.read_dir()? {
                let entry = entry?;
                if entry.path() == path {
                    return Ok(entry);
                }
            }

            bail!("Entry not found");
        }

        #[test]
        fn should_delete() {
            let test_root = join!(temp_dir(), "maa-cli-test-should-delete");

            test_root.ensure().unwrap();

            macro_rules! assert_should_delete {
                ($target:expr, $name:expr, $expected:expr) => {
                    let entry = create_target_entry(&test_root, $name).unwrap();
                    assert_eq!($target.should_delete(&entry), $expected);
                    del_item(&entry.path()).unwrap();
                };
            }

            // Create a directory with some files and subdirectories
            std::fs::create_dir_all(&test_root).unwrap();

            assert_should_delete!(Log, "asst.log", true);
            assert_should_delete!(Log, "asst.bak.log", true);

            assert_should_delete!(Log, "2024", false);
            assert_should_delete!(Log, "2024/", true);
            assert_should_delete!(Log, "20A4", false);
            assert_should_delete!(Log, "2024-01-01", false);

            assert_should_delete!(CliCache, "avatars/", true);
            assert_should_delete!(CliCache, "drops/", true);

            assert_should_delete!(CliCache, "copilot", true);
            assert_should_delete!(CliCache, "copilot/", true);
            assert_should_delete!(
                CliCache,
                "MAA-v5.6.0-beta.2-macos-runtime-universal.zip",
                true
            );

            std::fs::remove_dir(&test_root).unwrap();
        }

        #[test]
        #[ignore = "Need installed MaaCore"]
        fn should_keep() {
            let test_root = join!(temp_dir(), "maa-cli-test-should-keep");

            test_root.ensure().unwrap();

            // Create a directory with some files and subdirectories
            std::fs::create_dir_all(&test_root).unwrap();

            macro_rules! assert_should_keep {
                ($target:expr, $name:expr, $expected:expr) => {
                    let entry = create_target_entry(&test_root, $name).unwrap();
                    assert_eq!($target.should_keep(&entry), $expected);
                    del_item(&entry.path()).unwrap();
                };
            }

            assert_should_keep!(CoreCache, "avatars/", false);

            assert_should_keep!(CliCache, "test", false);
            assert_should_keep!(CliCache, "copilot/", false);

            #[cfg(feature = "core_installer")]
            if var_os("SKIP_CORE_TEST").is_none() {
                let version = var_os("MAA_CORE_VERSION")
                    .expect("MAA_CORE_VERSION environment variable not set");
                let version = version.to_str().unwrap()[1..].parse().unwrap();
                let name = crate::installer::maa_core::this_asset_name(&version);
                assert_should_keep!(CliCache, &name, true);
            }

            std::fs::remove_dir(&test_root).unwrap();
        }
    }

    #[test]
    fn test_cleanup() {
        struct All;

        impl PathProvider for All {
            fn target_dir(&self) -> Cow<'_, Path> {
                join!(temp_dir(), "maa-cli-test-cleanup").into()
            }
        }

        struct BlackList(Vec<&'static str>);

        impl PathProvider for BlackList {
            fn target_dir(&self) -> Cow<'_, Path> {
                join!(temp_dir(), "maa-cli-test-cleanup").into()
            }

            fn should_delete(&self, entry: &DirEntry) -> bool {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|x| self.0.contains(&x))
            }
        }

        struct WhiteList(Vec<&'static str>);

        impl PathProvider for WhiteList {
            fn target_dir(&self) -> Cow<'_, Path> {
                join!(temp_dir(), "maa-cli-test-cleanup").into()
            }

            fn should_keep(&self, entry: &DirEntry) -> bool {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|x| self.0.contains(&x))
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
        cleanup(&[All]).unwrap();
        assert!(test_root.read_dir().unwrap().next().is_none());

        create_test_files();
        cleanup(&[BlackList(vec!["test1", "test2"])]).unwrap();
        assert_eq!(
            test_root
                .read_dir()
                .unwrap()
                .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            vec!["test3"]
        );
        cleanup(&[All]).unwrap();

        create_test_files();
        cleanup(&[WhiteList(vec!["test1", "test2"])]).unwrap();
        assert_eq!(
            test_root
                .read_dir()
                .unwrap()
                .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
                .collect::<BTreeSet<_>>(),
            ["test1", "test2"]
                .iter()
                .map(|x| x.to_string())
                .collect::<BTreeSet<_>>()
        );
        cleanup(&[All]).unwrap();

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
    #[ignore = "Need installed MaaCore and write to user directories"]
    fn test_cleanup_real_files() {
        // Create some files for testing
        cache().ensure().unwrap();
        state().ensure().unwrap();
        log().ensure().unwrap();

        std::fs::File::create(join!(cache(), "test.tar.gz")).unwrap();

        #[cfg(feature = "core_installer")]
        {
            use semver::Version;

            use crate::installer::maa_core::this_asset_name;

            std::fs::File::create(cache().join(this_asset_name(&Version::new(5, 16, 1)))).unwrap();

            if var_os("SKIP_CORE_TEST").is_none() {
                let version = var_os("MAA_CORE_VERSION")
                    .expect("MAA_CORE_VERSION environment variable not set");
                let version = version.to_str().unwrap()[1..].parse().unwrap();
                let name = this_asset_name(&version);
                std::fs::File::create(join!(cache(), &name)).unwrap();
            }
        }

        let core_cache = join!(state(), "cache");
        core_cache.ensure().unwrap();

        std::fs::File::create(join!(&core_cache, "avatars")).unwrap();
        std::fs::create_dir_all(join!(log(), "2024")).unwrap();
        std::fs::File::create(join!(log(), "asst.log")).unwrap();
        std::fs::File::create(join!(log(), "asst.bak.log")).unwrap();

        let target: Vec<CleanupTarget> = Vec::new();
        cleanup(&target).unwrap();

        assert!(!join!(cache(), "test.tar.gz").exists());
        assert!(!join!(core_cache, "avatars").exists());
        assert!(!join!(log(), "2024").exists());
        assert!(!join!(log(), "asst.log").exists());
        assert!(!join!(log(), "asst.bak.log").exists());

        cleanup(&target).unwrap(); // Cleanup again to test no files left
    }
}
