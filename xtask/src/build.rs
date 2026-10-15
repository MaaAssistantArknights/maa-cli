//! Build automation for maa-cli.

use std::{env, fs, path::Path, process::Command};

use anyhow::{Context, Result};

use crate::{
    BuildOptions, HOST_TRIPLET,
    cmd::{CommandExt, cargo, rustup_up},
    github::set_output,
    group::Group,
    workspace_root,
};

/// Build maa-cli binary.
pub fn run(opts: BuildOptions) -> Result<()> {
    // Set GitHub output for use in subsequent steps
    set_output("host_triplet", HOST_TRIPLET).ok();

    if env::var_os("CI").is_some() {
        Group::new("Update Stable Toolchain")
            .run(|| rustup_up("stable").run().context("Failed to update Rust"))?;
    }

    let mut build_args = Vec::new();
    if opts.vendored_deps {
        build_args.extend([
            "--features".to_owned(),
            "git2?/vendored-libgit2,git2?/vendored-openssl".to_owned(),
        ]);
    }
    build_args.extend(opts.build_args);

    Group::new("Build").run(|| {
        let mut cmd = cargo();
        cmd.args(["build", "--package", "maa-cli", "--locked"]);
        cmd.args(["--profile", &opts.profile]);

        cmd.args(&build_args);

        cmd.run().context("Failed to build maa-cli")
    })?;

    let profile_dir = if opts.profile == "dev" {
        "debug"
    } else {
        &opts.profile
    };

    let target_dir = format!("{}/target/{profile_dir}", workspace_root());
    let exe = format!("maa{}", std::env::consts::EXE_SUFFIX);
    let binary = format!("{target_dir}/{}", exe);

    Group::new("Dry Run").run(|| {
        std::process::Command::new(&binary)
            .arg("--version")
            .run()
            .context("Failed to check binary version")
    })?;

    if let Some(output_dir) = opts.output_dir {
        let temp = tempfile::tempdir().context("Failed to create license directory")?;
        let licenses = temp.path().join("licenses.md");
        Group::new("Generate Licenses").run(|| {
            license_command(&build_args, &licenses)?
                .run()
                .context("Failed to generate licenses.md")
        })?;
        Group::new("Prepare Output Directory").run(|| {
            let name = opts.rename.unwrap_or(exe);
            prepare_output_dir(&output_dir, Path::new(&binary), Path::new(&name), &licenses)
        })?;
    }

    Ok(())
}

fn license_command(build_args: &[String], output: &Path) -> Result<Command> {
    let mut cmd = cargo();
    cmd.current_dir(workspace_root()).args([
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

    let mut args = build_args.iter();
    while let Some(arg) = args.next() {
        let arg = arg.as_str();
        let features = match arg {
            "--features" | "-F" => Some(args.next().context("Missing feature list")?.as_str()),
            "--all-features" | "--no-default-features" | "--offline" | "--frozen" => {
                cmd.arg(arg);
                None
            }
            _ => arg.strip_prefix("--features=").or_else(|| {
                arg.strip_prefix("-F")
                    .map(|value| value.trim_start_matches('='))
            }),
        };
        if let Some(features) = features {
            // cargo-about accepts a space-separated list; Cargo also accepts commas.
            let features = features
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|feature| !feature.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            cmd.args(["--features", &features]);
        }
    }
    Ok(cmd)
}

fn prepare_output_dir(output: &Path, binary: &Path, name: &Path, licenses: &Path) -> Result<()> {
    fs::create_dir_all(output)
        .with_context(|| format!("Failed to create output directory: {}", output.display()))?;
    let destination = output.join(name);
    let same_binary = match destination.canonicalize() {
        Ok(path) => {
            path == binary
                .canonicalize()
                .context("Failed to resolve binary path")?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error).context("Failed to resolve output binary path"),
    };
    if !same_binary {
        fs::copy(binary, destination).context("Failed to copy binary to output directory")?;
    }
    fs::copy(licenses, output.join("licenses.md"))
        .context("Failed to copy licenses.md to output directory")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn licenses_use_build_features_and_host_target() -> Result<()> {
        let args = [
            "--features",
            "git2?/vendored-libgit2,git2?/vendored-openssl",
            "--no-default-features",
            "--features=core_installer,git2",
            "-F",
            "maa-core/runtime",
            "-Fmaa-core/static",
            "-F=maa-core/some-feature",
            "--all-features",
            "--offline",
            "--frozen",
            "--jobs",
            "2",
        ]
        .map(String::from);
        let cmd = license_command(&args, Path::new("licenses.md"))?;
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
        let feature_lists: Vec<_> = args
            .windows(2)
            .filter(|pair| pair[0] == "--features")
            .map(|pair| pair[1].as_str())
            .collect();
        assert_eq!(feature_lists, [
            "git2?/vendored-libgit2 git2?/vendored-openssl",
            "core_installer git2",
            "maa-core/runtime",
            "maa-core/static",
            "maa-core/some-feature",
        ]);
        for flag in [
            "--no-default-features",
            "--all-features",
            "--offline",
            "--frozen",
            "--locked",
        ] {
            assert!(args.iter().any(|arg| arg == flag));
        }
        assert!(!args.iter().any(|arg| arg == "--jobs"));
        Ok(())
    }

    #[test]
    fn default_build_keeps_default_features() -> Result<()> {
        let cmd = license_command(&[], Path::new("licenses.md"))?;
        assert!(!cmd.get_args().any(|arg| arg == "--no-default-features"
            || arg == "--all-features"
            || arg == "--features"));
        Ok(())
    }

    #[test]
    fn output_directory_contains_renamed_binary_and_licenses() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let binary = temp.path().join("maa.exe");
        let licenses = temp.path().join("licenses.md");
        let output = temp.path().join("artifacts").join("winget");
        fs::write(&binary, b"binary")?;
        fs::write(&licenses, b"target-specific licenses")?;
        prepare_output_dir(&output, &binary, Path::new("maa-cli.exe"), &licenses)?;
        assert_eq!(fs::read(output.join("maa-cli.exe"))?, b"binary");
        assert_eq!(
            fs::read(output.join("licenses.md"))?,
            b"target-specific licenses"
        );
        assert_eq!(fs::read_dir(&output)?.count(), 2);
        assert_eq!(fs::read(&binary)?, b"binary");
        Ok(())
    }
    #[test]
    fn output_directory_can_be_the_build_directory() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let output = temp.path().join("target").join("debug");
        fs::create_dir_all(&output)?;
        let binary = output.join("maa");
        let licenses = temp.path().join("licenses.md");
        fs::write(&binary, b"existing binary")?;
        fs::write(&licenses, b"target-specific licenses")?;
        prepare_output_dir(&output.join("."), &binary, Path::new("maa"), &licenses)?;
        assert_eq!(fs::read(&binary)?, b"existing binary");
        assert_eq!(
            fs::read(output.join("licenses.md"))?,
            b"target-specific licenses"
        );
        Ok(())
    }
}
