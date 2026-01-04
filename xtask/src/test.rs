//! Test automation for CI.

use anyhow::{Context, Result, bail};

use crate::{
    TestOptions,
    cmd::{CommandExt, EnvVars, cargo, rustup_up},
    github::Group,
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

    Group::new("Update Stable Toolchain")
        .run(|| rustup_up("stable").run().context("Failed to update Rust"))?;

    if opts.coverage.report() {
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
            let core_dir = maa_dirs::find_library();
            if let Some(core_dir) = core_dir {
                env_vars.push("MAA_CORE_DIR", core_dir.to_str().unwrap().to_owned());
            } else {
                bail!("Failed to find MaaCore")
            }
            Ok(())
        })?;
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
        cmd.env_vars(&env_vars);
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
