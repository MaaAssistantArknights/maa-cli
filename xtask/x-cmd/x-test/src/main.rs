//! Configure tests, Clippy, and coverage, forwarding trailing command-line arguments to Cargo.

use anyhow::Result;
use flags::Test as TestOptions;

mod test;

fn main() -> Result<()> {
    x_lib::group::finish_prepare();
    test::run_tests(TestOptions::from_env_or_exit())
}

mod flags {
    use std::ffi::OsString;

    use super::CoverageMode;

    xflags::xflags! {
        /// Run tests with optional MaaCore installation and coverage.
        cmd test {
            /// Install MaaCore before testing.
            optional --install-core
            /// Skip tests requiring MaaCore.
            optional --no-core-tests
            /// Skip ignored tests.
            optional --no-ignored-tests
            /// Coverage mode: none (default), test-only, or all.
            optional --coverage mode: CoverageMode
            /// Run Clippy before testing.
            optional --clippy
            /// Disable all features during the Clippy build.
            optional --no-all-features
            /// Add MaaCore to the runtime library path.
            optional --runtime-library-path
            /// Package to test (default: workspace).
            optional -p, --package name: String
            /// Additional Cargo arguments; put flags after --.
            repeated test_args: OsString
        }
    }
}

impl TestOptions {
    fn package_flags(&self) -> Vec<&str> {
        match self.package.as_deref().unwrap_or("workspace") {
            "workspace" => vec!["--workspace", "--exclude", "x", "--exclude", "x-*"],
            package => vec!["--package", package],
        }
    }
}

#[derive(Clone, Copy, Debug)]
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

    #[test]
    fn package_selection() -> Result<()> {
        let defaults = TestOptions::from_vec(Vec::new())?;
        assert_eq!(defaults.package_flags(), [
            "--workspace",
            "--exclude",
            "x",
            "--exclude",
            "x-*"
        ]);
        let package = TestOptions::from_vec(vec!["--package".into(), "maa-dirs".into()])?;
        assert_eq!(package.package_flags(), ["--package", "maa-dirs"]);
        Ok(())
    }
}
