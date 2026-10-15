//! Build automation for maa-cli.

use std::{env, fs::File, path::Path, process::Command};

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

    if opts.tar {
        let temp = tempfile::tempdir().context("Failed to create license directory")?;
        let licenses = temp.path().join("licenses.md");
        Group::new("Generate Licenses").run(|| {
            license_command(&build_args, &licenses)?
                .run()
                .context("Failed to generate licenses.md")
        })?;
        Group::new("Create Tar Package").run(|| {
            let tar_name = format!("{HOST_TRIPLET}.tar");
            let archive_name = opts.rename.unwrap_or(exe);
            create_tar(
                Path::new(&tar_name),
                Path::new(&binary),
                Path::new(&archive_name),
                &licenses,
            )
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

fn create_tar(output: &Path, binary: &Path, name: &Path, licenses: &Path) -> Result<()> {
    let file = File::create(output)
        .with_context(|| format!("Failed to create tar file: {}", output.display()))?;
    let mut tar = tar::Builder::new(file);
    tar.append_path_with_name(binary, name)
        .context("Failed to add binary to tar archive")?;
    tar.append_path_with_name(licenses, "licenses.md")
        .context("Failed to add licenses.md to tar archive")?;
    tar.finish().context("Failed to finalize tar archive")
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read};

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
    fn tar_contains_renamed_binary_and_licenses() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let binary = temp.path().join("maa.exe");
        let licenses = temp.path().join("licenses.md");
        let output = temp.path().join("build.tar");
        fs::write(&binary, b"binary")?;
        fs::write(&licenses, b"target-specific licenses")?;
        create_tar(&output, &binary, Path::new("maa-cli.exe"), &licenses)?;
        let mut archive = tar::Archive::new(File::open(output)?);
        let mut entries = Vec::new();
        for entry in archive.entries()? {
            let mut entry = entry?;
            let name = entry.path()?.into_owned();
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            entries.push((name, content));
        }
        assert_eq!(entries, vec![
            ("maa-cli.exe".into(), "binary".into()),
            ("licenses.md".into(), "target-specific licenses".into()),
        ]);
        Ok(())
    }
}
