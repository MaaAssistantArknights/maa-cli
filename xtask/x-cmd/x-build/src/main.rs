//! Prepare binaries and licenses, forwarding trailing command-line arguments to Cargo.

use std::{env, ffi::OsString, fs, path::Path, process::Command};

use anyhow::{Context, Result};
use github::set_output;
use x_lib::{
    args::next_value,
    command::{CommandExt, cargo, rustup_up},
    group::Group,
};

mod github;

const HOST_TRIPLET: &str = env!("TARGET");

fn main() -> Result<()> {
    x_lib::group::finish_prepare();
    let BuildOptions {
        profile,
        vendored_deps,
        output_dir,
        rename,
        build_args,
    } = BuildOptions::parse(env::args_os().skip(1))?;
    // Set GitHub output for use in subsequent steps
    set_output("host_triplet", HOST_TRIPLET).ok();

    if env::var_os("CI").is_some() {
        Group::new("Update Stable Toolchain")
            .run(|| rustup_up("stable").run().context("Failed to update Rust"))?;
    }

    let mut cargo_args = Vec::new();
    if vendored_deps {
        cargo_args.extend([
            OsString::from("--features"),
            OsString::from("git2?/vendored-libgit2,git2?/vendored-openssl"),
        ]);
    }
    cargo_args.extend(build_args);

    Group::new("Build").run(|| {
        let mut cmd = cargo();
        cmd.args(["build", "--package", "maa-cli", "--locked"]);
        cmd.args(["--profile", &profile]);

        cmd.args(&cargo_args);

        cmd.run().context("Failed to build maa-cli")
    })?;

    let profile_dir = if profile == "dev" { "debug" } else { &profile };

    let target_dir = x_lib::workspace::root().join("target").join(profile_dir);
    let exe = format!("maa{}", std::env::consts::EXE_SUFFIX);
    let binary = target_dir.join(&exe);

    Group::new("Dry Run").run(|| {
        std::process::Command::new(&binary)
            .arg("--version")
            .run()
            .context("Failed to check binary version")
    })?;

    let output_dir =
        std::path::absolute(output_dir).context("Failed to resolve output directory")?;
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;
    fs::copy(
        &binary,
        output_dir.join(rename.as_deref().unwrap_or_else(|| exe.as_ref())),
    )
    .context("Failed to copy binary to output directory")?;
    let licenses = output_dir.join("licenses.md");
    Group::new("Generate Licenses").run(|| {
        license_command(&cargo_args, &licenses)
            .run()
            .context("Failed to generate licenses.md")
    })?;

    Ok(())
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

fn license_command(build_args: &[OsString], output: &Path) -> Command {
    let mut cmd = cargo();
    cmd.current_dir(x_lib::workspace::root()).args([
        "about",
        "generate",
        "about.hbs",
        "--manifest-path",
        "crates/maa-cli/Cargo.toml",
        "--config",
        "about.toml",
        "--target",
        HOST_TRIPLET,
        "--locked",
    ]);
    cmd.arg("--output-file").arg(output);

    cmd.args(build_args);
    cmd
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

    #[test]
    fn licenses_forward_build_arguments_for_host_target() {
        let build_args = [
            "--features",
            "git2?/vendored-libgit2,git2?/vendored-openssl",
            "--no-default-features",
            "--features",
            "core_installer,git2",
        ]
        .map(OsString::from);
        let cmd = license_command(&build_args, Path::new("licenses.md"));
        let args: Vec<_> = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--target", HOST_TRIPLET])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--manifest-path", "crates/maa-cli/Cargo.toml"])
        );
        assert!(
            cmd.get_args().collect::<Vec<_>>().ends_with(
                &build_args
                    .iter()
                    .map(OsString::as_os_str)
                    .collect::<Vec<_>>()
            )
        );
    }
}
