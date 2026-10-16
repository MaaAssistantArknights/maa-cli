//! Configure tests, Clippy, and coverage, forwarding trailing command-line arguments to Cargo.

use std::{env, ffi::OsString};

use anyhow::{Context, Result};
use x_lib::args::next_value;

mod test;

fn main() -> Result<()> {
    x_lib::group::finish_prepare();
    test::run_tests(TestOptions::parse(env::args_os().skip(1))?)
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
