//! Test automation for CI.

use anyhow::{Context, Result};

use crate::{
    TestOptions,
    cmd::{CommandExt, EnvVars},
    github::Group,
    workspace_root,
};

fn cargo() -> std::process::Command {
    std::process::Command::new("cargo")
}

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

    Group::new("Update Stable Toolchain").run(|| {
        std::process::Command::new("rustup")
            .args(["update", "stable"])
            .run()
            .context("Failed to update Rust")
    })?;

    if opts.coverage.report() {
        Group::new("Install Nightly Toolchain").run(|| {
            std::process::Command::new("rustup")
                .args(["install", "nightly", "--profile=minimal", "-cllvm-tools"])
                .arg("--no-self-update")
                .run()
                .context("Failed to install nightly")
        })?;
    }

    if opts.with_core {
        Group::new("Install MaaCore").run(|| {
            let core_dir = maa_dirs::library().to_str().unwrap();
            env_vars.push("MAA_CORE_DIR", core_dir.to_owned());

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
    } else {
        Group::new("Skip Core Test").run(|| {
            env_vars.push("SKIP_CORE_TEST", "true".to_owned());
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
        cmd.args(["test", "--locked"]);
        cmd.args(&opts.test_args);
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
