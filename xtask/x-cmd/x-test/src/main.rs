//! Configure tests, Clippy, and coverage, forwarding trailing command-line arguments to Cargo.

use std::{env, ffi::OsString, path::Path};

use anyhow::{Context, Result};
use tempfile::TempDir;
use x_lib::{
    args::next_value,
    command::{CommandExt, EnvVars, cargo, rustup_up},
    group::Group,
};

const LLVM_COV_ARGS: &[&str] = &["+nightly", "llvm-cov"];

fn main() -> Result<()> {
    x_lib::group::finish_prepare();

    let opts = TestOptions::parse(env::args_os().skip(1))?;
    // Build environment variables map
    let mut env_vars = EnvVars::new();
    let test_root = TempDir::new().context("Failed to create isolated test directory")?;

    Group::new("Setup Environment Variables").run(|| {
        let config_dir = x_lib::workspace::root().join("crates/maa-cli/config_examples");
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

#[derive(Debug)]
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
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self> {
        let mut opts = Self {
            install_core: false,
            no_core_tests: false,
            no_ignored_tests: false,
            coverage: CoverageMode::None,
            clippy: false,
            no_all_features: false,
            runtime_library_path: false,
            package: "workspace".to_owned(),
            test_args: Vec::new(),
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let (option, inline) = match arg.to_str().and_then(|arg| arg.split_once('=')) {
                Some((option, value)) => (Some(option), Some(OsString::from(value))),
                None => (arg.to_str(), None),
            };
            match option {
                Some("--install-core") if inline.is_none() => opts.install_core = true,
                Some("--no-core-tests") if inline.is_none() => opts.no_core_tests = true,
                Some("--no-ignored-tests") if inline.is_none() => opts.no_ignored_tests = true,
                Some("--clippy") if inline.is_none() => opts.clippy = true,
                Some("--no-all-features") if inline.is_none() => opts.no_all_features = true,
                Some("--runtime-library-path") if inline.is_none() => {
                    opts.runtime_library_path = true
                }
                Some("--coverage") => {
                    let value = match inline {
                        Some(value) => value,
                        None => next_value(&mut args, "--coverage")?,
                    };
                    opts.coverage = value
                        .to_str()
                        .context("--coverage must be valid UTF-8")?
                        .parse()
                        .map_err(|error| anyhow::anyhow!("Invalid --coverage: {error}"))?;
                }
                Some("-p" | "--package") => {
                    opts.package = match inline {
                        Some(value) => value,
                        None => next_value(&mut args, "--package")?,
                    }
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("--package must be valid UTF-8"))?;
                }
                Some("--") if inline.is_none() => {
                    opts.test_args.extend(args);
                    break;
                }
                _ => {
                    opts.test_args.push(arg);
                    opts.test_args.extend(args);
                    break;
                }
            }
        }
        Ok(opts)
    }

    fn package_flags(&self) -> Vec<&str> {
        match self.package.as_str() {
            "workspace" => vec!["--workspace", "--exclude", "x", "--exclude", "x-*"],
            _ => vec!["--package", &self.package],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<TestOptions> {
        TestOptions::parse(args.iter().map(OsString::from))
    }

    #[test]
    fn defaults_select_workspace_without_coverage() -> Result<()> {
        let opts = parse(&[])?;
        assert!(!opts.install_core && !opts.no_core_tests && !opts.no_ignored_tests);
        assert!(!opts.clippy && !opts.no_all_features && !opts.runtime_library_path);
        assert_eq!(opts.coverage, CoverageMode::None);
        assert_eq!(opts.package_flags(), [
            "--workspace",
            "--exclude",
            "x",
            "--exclude",
            "x-*"
        ]);
        assert!(opts.test_args.is_empty());
        Ok(())
    }

    #[test]
    fn wrapper_flags_and_values() -> Result<()> {
        let opts = parse(&[
            "--install-core",
            "--no-core-tests",
            "--no-ignored-tests",
            "--clippy",
            "--no-all-features",
            "--runtime-library-path",
            "--coverage=test-only",
            "-p",
            "maa-dirs",
        ])?;
        assert!(opts.install_core && opts.no_core_tests && opts.no_ignored_tests);
        assert!(opts.clippy && opts.no_all_features && opts.runtime_library_path);
        assert_eq!(opts.coverage, CoverageMode::TestOnly);
        assert_eq!(opts.package_flags(), ["--package", "maa-dirs"]);
        let opts = parse(&["--coverage", "all", "--package=maa-types"])?;
        assert_eq!(opts.coverage, CoverageMode::All);
        assert_eq!(opts.package, "maa-types");
        Ok(())
    }

    #[test]
    fn missing_or_invalid_values_are_reported() {
        for option in ["--coverage", "--package", "-p"] {
            let error = parse(&[option]).unwrap_err();
            let option = if option == "-p" { "--package" } else { option };
            assert_eq!(error.to_string(), format!("Missing value for {option}"));
        }
        assert_eq!(
            parse(&["--coverage=invalid"]).unwrap_err().to_string(),
            "Invalid --coverage: expected none, test-only or all"
        );
    }

    #[test]
    fn cargo_tail_and_separator_stop_wrapper_parsing() -> Result<()> {
        for first in ["--features", "filter"] {
            let opts = parse(&[first, "--clippy", "--", "--ignored"])?;
            assert!(!opts.clippy);
            assert_eq!(
                opts.test_args,
                [first, "--clippy", "--", "--ignored"].map(OsString::from)
            );
        }
        let opts = parse(&["--no-core-tests", "--", "--", "--ignored"])?;
        assert!(opts.no_core_tests);
        assert_eq!(opts.test_args, ["--", "--ignored"].map(OsString::from));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_tail_is_preserved_and_text_values_are_rejected() -> Result<()> {
        use std::os::unix::ffi::OsStringExt;

        let value = OsString::from_vec(vec![0xff]);
        let opts = TestOptions::parse([value.clone(), OsString::from("--clippy")])?;
        assert!(!opts.clippy);
        assert_eq!(opts.test_args, [value.clone(), OsString::from("--clippy")]);
        for option in ["--coverage", "--package"] {
            let error = TestOptions::parse([OsString::from(option), value.clone()]).unwrap_err();
            assert_eq!(error.to_string(), format!("{option} must be valid UTF-8"));
        }
        Ok(())
    }
}
