//! Prepare binaries and licenses, forwarding trailing command-line arguments to Cargo.

use std::{env, ffi::OsString};

use anyhow::{Context, Result};
use x_lib::args::next_value;

mod build;
mod github;

const HOST_TRIPLET: &str = env!("TARGET");

fn main() -> Result<()> {
    x_lib::group::finish_prepare();
    build::run(BuildOptions::parse(env::args_os().skip(1))?)
}

#[derive(Debug)]
struct BuildOptions {
    profile: String,
    vendored_deps: bool,
    output_dir: OsString,
    rename: Option<OsString>,
    build_args: Vec<OsString>,
}

impl BuildOptions {
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self> {
        let mut args = args.into_iter();
        let mut profile = "dev".to_owned();
        let mut vendored_deps = false;
        let mut output_dir = None;
        let mut rename = None;
        let mut build_args = Vec::new();
        while let Some(arg) = args.next() {
            let (option, inline) = match arg.to_str().and_then(|arg| arg.split_once('=')) {
                Some((option, value)) => (Some(option), Some(OsString::from(value))),
                None => (arg.to_str(), None),
            };
            match option {
                Some("--profile") => {
                    profile = match inline {
                        Some(value) => value,
                        None => next_value(&mut args, "--profile")?,
                    }
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("--profile must be valid UTF-8"))?;
                }
                Some("--vendored-deps") if inline.is_none() => vendored_deps = true,
                Some("--output-dir") => {
                    output_dir = Some(match inline {
                        Some(value) => value,
                        None => next_value(&mut args, "--output-dir")?,
                    });
                }
                Some("--rename") => {
                    rename = Some(match inline {
                        Some(value) => value,
                        None => next_value(&mut args, "--rename")?,
                    });
                }
                Some("--") if inline.is_none() => {
                    build_args.extend(args);
                    break;
                }
                _ => {
                    build_args.push(arg);
                    build_args.extend(args);
                    break;
                }
            }
        }
        Ok(Self {
            profile,
            vendored_deps,
            output_dir: output_dir.context("Missing required --output-dir")?,
            rename,
            build_args,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<BuildOptions> {
        BuildOptions::parse(args.iter().map(OsString::from))
    }

    #[test]
    fn options_and_defaults() -> Result<()> {
        let defaults = parse(&["--output-dir", "out"])?;
        assert_eq!(defaults.profile, "dev");
        assert!(!defaults.vendored_deps);
        assert!(defaults.rename.is_none());
        assert!(defaults.build_args.is_empty());

        let opts = parse(&[
            "--profile=release",
            "--vendored-deps",
            "--output-dir=out",
            "--rename",
            "maa-custom",
        ])?;
        assert_eq!(opts.profile, "release");
        assert!(opts.vendored_deps);
        assert_eq!(opts.output_dir, "out");
        assert_eq!(opts.rename, Some(OsString::from("maa-custom")));
        Ok(())
    }

    #[test]
    fn required_values_are_reported() {
        assert_eq!(
            parse(&[]).unwrap_err().to_string(),
            "Missing required --output-dir"
        );
        for option in ["--profile", "--output-dir", "--rename"] {
            let error = parse(&["--output-dir", "out", option]).unwrap_err();
            assert_eq!(error.to_string(), format!("Missing value for {option}"));
        }
    }

    #[test]
    fn cargo_tail_is_opaque() -> Result<()> {
        for first in ["--features", "positional"] {
            let opts = parse(&["--output-dir", "out", first, "--profile", "release", "--"])?;
            assert_eq!(opts.profile, "dev");
            assert_eq!(
                opts.build_args,
                [first, "--profile", "release", "--"].map(OsString::from)
            );
        }
        let opts = parse(&["--output-dir", "out", "--", "--profile", "release"])?;
        assert_eq!(opts.profile, "dev");
        assert_eq!(
            opts.build_args,
            ["--profile", "release"].map(OsString::from)
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn paths_and_cargo_tail_preserve_non_utf8() -> Result<()> {
        use std::os::unix::ffi::OsStringExt;

        let value = OsString::from_vec(vec![b'o', 0xff]);
        let opts = BuildOptions::parse([
            OsString::from("--output-dir"),
            value.clone(),
            OsString::from("--rename"),
            value.clone(),
            OsString::from("--"),
            value.clone(),
        ])?;
        assert_eq!(opts.output_dir, value);
        assert_eq!(opts.rename.as_ref(), Some(&value));
        assert_eq!(opts.build_args, std::slice::from_ref(&value));
        let error = BuildOptions::parse([OsString::from("--profile"), value]).unwrap_err();
        assert_eq!(error.to_string(), "--profile must be valid UTF-8");
        Ok(())
    }
}
