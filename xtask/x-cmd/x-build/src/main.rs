//! Prepare binaries and licenses, forwarding trailing command-line arguments to Cargo.

use anyhow::Result;
use flags::Build as BuildOptions;

mod build;
mod github;

const HOST_TRIPLET: &str = env!("TARGET");

fn main() -> Result<()> {
    x_lib::group::finish_prepare();
    build::run(BuildOptions::from_env_or_exit())
}

mod flags {
    use std::ffi::OsString;

    xflags::xflags! {
        /// Prepare maa-cli release artifacts (requires cargo-about).
        cmd build {
            /// Build profile (default: dev).
            optional --profile value: String
            /// Use vendored dependencies.
            optional --vendored-deps
            /// Directory for the binary and licenses.
            required --output-dir path: OsString
            /// Rename the binary.
            optional --rename name: OsString
            /// Additional Cargo arguments; put flags after --.
            repeated build_args: OsString
        }
    }
}
