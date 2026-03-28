//! Test automation for CI.

use std::{env, ffi::OsString, path::Path};

use anyhow::{Context, Result};

use crate::{
    TestOptions,
    cmd::{CommandExt, EnvVars, cargo, rustup_up},
    group::Group,
    workspace_root,
};

const LLVM_COV_ARGS: &[&str] = &["+nightly", "llvm-cov"];

/// Run tests with optional core installation and coverage.
pub fn run_tests(opts: TestOptions) -> Result<()> {
    // Build environment variables map
    let mut env_vars = EnvVars::new();

    Group::new("Setup Environment Variables").run(|| {
        let config_dir = format!("{}/crates/maa-cli/config_examples", workspace_root());
        env_vars.push("MAA_CONFIG_DIR", config_dir);
        env_vars.push("MAA_EXTRA_SHARE_NAME", "maa-test".to_string());

        Ok(())
    })?;

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
            Ok(())
        })?;
    } else {
        Group::new("Find MaaCore").run(|| {
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

    if !opts.no_clippy {
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
            cmd.args(["--", "--include-ignored"]);
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
