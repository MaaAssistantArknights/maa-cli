//! Smoke tests that run the `maa` binary end to end.
//!
//! Each test points every maa directory at its own temp directory, so they never touch the
//! user's real directories and can run in parallel with each other and with unit tests.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn dir(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    /// Run `maa` with the given arguments in batch mode
    ///
    /// The environment is inherited rather than cleared, so `LLVM_PROFILE_FILE` reaches the child
    /// and coverage of the binary is still collected.
    fn maa(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_maa"))
            .args(args)
            .arg("--batch")
            .env("MAA_CONFIG_DIR", self.dir("config"))
            .env("MAA_DATA_DIR", self.dir("data"))
            .env("MAA_STATE_DIR", self.dir("state"))
            .env("MAA_CACHE_DIR", self.dir("cache"))
            .current_dir(self.root.path())
            .output()
            .unwrap()
    }

    /// Run `maa`, assert it succeeds and return its stdout
    fn run(&self, args: &[&str]) -> String {
        let output = self.maa(args);
        assert!(
            output.status.success(),
            "`maa {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

#[test]
fn dir() {
    let sandbox = Sandbox::new();

    for (dir, expected) in [
        ("config", sandbox.dir("config")),
        ("data", sandbox.dir("data")),
        ("cache", sandbox.dir("cache")),
        ("log", sandbox.dir("state").join("debug")),
    ] {
        assert_eq!(
            sandbox.run(&["dir", dir]).trim_end(),
            expected.to_str().unwrap()
        );
    }
}

#[test]
fn cleanup() {
    let sandbox = Sandbox::new();
    let cli_cache = sandbox.dir("cache").join("foo.zip");
    let core_cache = sandbox.dir("state").join("cache").join("foo");
    let debug = sandbox.dir("state").join("debug").join("asst.log");
    for path in [&cli_cache, &core_cache, &debug] {
        touch(path);
    }

    sandbox.run(&["cleanup"]);
    for path in [&cli_cache, &core_cache, &debug] {
        assert!(!path.exists(), "{} was not deleted", path.display());
    }

    assert!(
        sandbox
            .run(&["cleanup"])
            .contains("No files or directories to clean up.")
    );
}

#[test]
fn init_and_convert() {
    let sandbox = Sandbox::new();

    sandbox.run(&["init"]);
    let profile = sandbox.dir("config").join("profiles").join("default.json");
    assert!(profile.exists());

    // An existing profile is not overwritten without --force
    assert!(!sandbox.maa(&["init"]).status.success());
    sandbox.run(&["init", "--force"]);

    let toml = sandbox.dir("default.toml");
    sandbox.run(&["convert", profile.to_str().unwrap(), toml.to_str().unwrap()]);
    let converted: toml::Table = fs::read_to_string(&toml).unwrap().parse().unwrap();
    let original: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&profile).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(converted).unwrap(), original);
}
