//! `cargo x test`：通过 TEST_* 配置测试环境、Clippy 和覆盖率，命令行参数传给 Cargo。

use std::{env, ffi::OsString, path::Path};

use anyhow::{Context, Result};
use tempfile::TempDir;
use x_command::{CommandExt, EnvVars, cargo, rustup_up};
use x_group::Group;

const LLVM_COV_ARGS: &[&str] = &["+nightly", "llvm-cov"];

fn main() -> Result<()> {
    x_group::finish_prepare();

    let opts = TestOptions::from_env()?;
    // Build environment variables map
    let mut env_vars = EnvVars::new();
    let test_root = TempDir::new().context("Failed to create isolated test directory")?;

    Group::new("Setup Environment Variables").run(|| {
        let config_dir = x_workspace::root().join("crates/maa-cli/config_examples");
        env_vars.push("MAA_CONFIG_DIR", config_dir.display().to_string());
        env_vars.push(
            "MAA_CACHE_DIR",
            test_root.path().join("cache").display().to_string(),
        );
        env_vars.push(
            "MAA_STATE_DIR",
            test_root.path().join("state").display().to_string(),
        );
        env_vars.push("MAA_EXTRA_SHARE_NAME", "maa-test".to_string());
    });

    let package_flags = opts.package_flags();

    if env::var_os("CI").is_some() {
        Group::new("Update Stable Toolchain")
            .run(|| rustup_up("stable").run().context("Failed to update Rust"))?;
    }

    if opts.coverage.report() && env::var_os("CI").is_some() {
        Group::new("Install Nightly Toolchain").run(|| {
            rustup_up("nightly")
                .args(["--profile=minimal", "-cllvm-tools"])
                .run()
                .context("Failed to install nightly")
        })?;
    }

    if opts.install_core {
        Group::new("Install MaaCore").run(|| {
            let mut cmd = cargo();
            if opts.coverage.coverage_run() {
                cmd.args(LLVM_COV_ARGS);
                cmd.arg("--no-report");
            }
            cmd.args(["run", "--package", "maa-cli", "--"]);
            cmd.args(["install", "beta"]);
            cmd.env_vars(&env_vars);
            cmd.run().context("Failed to install MaaCore")
        })?;
    }

    if opts.no_core_tests {
        Group::new("Skip Core Test").run(|| {
            env_vars.push("SKIP_CORE_TEST", "true".to_owned());
        });
    } else {
        Group::new("Find MaaCore").run(|| -> Result<()> {
            let core_dir = maa_dirs::find_library()
                .ok_or_else(|| anyhow::anyhow!("Failed to find MaaCore"))?;
            env_vars.push("MAA_CORE_DIR", core_dir.display().to_string());
            if opts.runtime_library_path {
                push_runtime_library_path(&mut env_vars, &core_dir)?;
            }
            Ok(())
        })?;
    }

    let mut test_env_vars = env_vars.clone();
    if opts.runtime_library_path && !opts.no_core_tests {
        let core_dir =
            maa_dirs::find_library().ok_or_else(|| anyhow::anyhow!("Failed to find MaaCore"))?;
        push_runtime_library_path(&mut test_env_vars, &core_dir)?;
    }

    if opts.clippy {
        // Build first if we run clippy
        Group::new("Build").run(|| {
            let mut cmd = cargo();
            cmd.args(["build", "--locked"]);
            cmd.args(&package_flags);
            if !opts.no_all_features {
                cmd.arg("--all-features");
            }
            cmd.env_vars(&env_vars);
            cmd.run()
        })?;

        Group::new("Clippy").run(|| {
            let mut cmd = cargo();
            cmd.args(["clippy", "--all-targets"]);
            cmd.args(&package_flags);
            if !opts.no_all_features {
                cmd.arg("--all-features");
            }
            cmd.args(["--", "-D", "warnings"]);
            cmd.env_vars(&env_vars);
            cmd.run()
        })?;
    }

    // Run tests
    Group::new("Tests").run(|| {
        let mut cmd = cargo();
        if opts.coverage.coverage_test() {
            cmd.args(LLVM_COV_ARGS);
            cmd.arg("--no-report");
        }
        cmd.args(["test", "--locked", "--no-fail-fast"]);
        cmd.args(&package_flags);
        cmd.args(&opts.test_args);
        if !opts.no_ignored_tests {
            if !opts.test_args.iter().any(|arg| arg == "--") {
                cmd.arg("--");
            }
            cmd.arg("--include-ignored");
        }
        cmd.env_vars(&test_env_vars);
        cmd.run().context("Failed to run cargo test")
    })?;

    // Collect coverage data
    if opts.coverage.report() {
        Group::new("Coverage").run(|| {
            cargo()
                .args(LLVM_COV_ARGS)
                .args(["report", "--codecov", "--output-path", "codecov.json"])
                .run()
        })?;
    }
    Ok(())
}

fn push_runtime_library_path(env_vars: &mut EnvVars<'_>, core_dir: &Path) -> Result<()> {
    let value = join_runtime_library_path(
        core_dir,
        #[cfg(target_os = "windows")]
        env::var_os("PATH"),
        #[cfg(target_os = "linux")]
        env::var_os("LD_LIBRARY_PATH"),
        #[cfg(target_os = "macos")]
        env::var_os("DYLD_LIBRARY_PATH"),
    )?;

    #[cfg(target_os = "windows")]
    env_vars.push("PATH", value);
    #[cfg(target_os = "linux")]
    env_vars.push("LD_LIBRARY_PATH", value);
    #[cfg(target_os = "macos")]
    env_vars.push("DYLD_LIBRARY_PATH", value);

    Ok(())
}

fn join_runtime_library_path(core_dir: &Path, current: Option<OsString>) -> Result<String> {
    let mut paths = vec![core_dir.to_path_buf()];
    if let Some(current) = current {
        paths.extend(env::split_paths(&current));
    }

    let joined = env::join_paths(paths).context("Failed to join runtime library path")?;
    Ok(joined.to_string_lossy().into_owned())
}

struct TestOptions {
    install_core: bool,
    no_core_tests: bool,
    no_ignored_tests: bool,
    coverage: CoverageMode,
    clippy: bool,
    no_all_features: bool,
    runtime_library_path: bool,
    package: String,
    test_args: Vec<OsString>,
}

impl TestOptions {
    fn from_env() -> Result<Self> {
        Ok(Self {
            install_core: x_env::read_or("TEST_INSTALL_CORE", false)?,
            no_core_tests: x_env::read_or("TEST_NO_CORE_TESTS", false)?,
            no_ignored_tests: x_env::read_or("TEST_NO_IGNORED_TESTS", false)?,
            coverage: x_env::read_or("TEST_COVERAGE", CoverageMode::None)?,
            clippy: x_env::read_or("TEST_CLIPPY", false)?,
            no_all_features: x_env::read_or("TEST_NO_ALL_FEATURES", false)?,
            runtime_library_path: x_env::read_or("TEST_RUNTIME_LIBRARY_PATH", false)?,
            package: x_env::read_or("TEST_PACKAGE", "workspace".to_owned())?,
            test_args: env::args_os().skip(1).collect(),
        })
    }

    fn package_flags(&self) -> Vec<&str> {
        match self.package.as_str() {
            "workspace" => vec!["--workspace", "--exclude", "x", "--exclude", "x-*"],
            _ => vec!["--package", &self.package],
        }
    }
}

#[derive(Clone, Copy)]
enum CoverageMode {
    TestOnly,
    All,
    None,
}

impl std::str::FromStr for CoverageMode {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "test-only" => Ok(Self::TestOnly),
            "all" => Ok(Self::All),
            _ => Err("expected none, test-only or all"),
        }
    }
}

impl CoverageMode {
    fn coverage_run(self) -> bool {
        matches!(self, Self::All)
    }

    fn coverage_test(self) -> bool {
        !matches!(self, Self::None)
    }

    fn report(self) -> bool {
        !matches!(self, Self::None)
    }
}
