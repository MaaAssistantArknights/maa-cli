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
            output_dir.join(opts.rename.as_deref().unwrap_or(&exe)),
        )
        .context("Failed to copy binary to output directory")?;
        let licenses = output_dir.join("licenses.md");
        Group::new("Generate Licenses").run(|| {
            license_command(&build_args, &licenses)
                .run()
                .context("Failed to generate licenses.md")
        })?;
    }

    Ok(())
}

fn license_command(build_args: &[String], output: &Path) -> Command {
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

    cmd.args(build_args);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn licenses_forward_build_arguments_for_host_target() {
        let build_args = [
            "--features",
            "git2?/vendored-libgit2,git2?/vendored-openssl",
            "--no-default-features",
            "--features",
            "core_installer,git2",
        ]
        .map(String::from);
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
        assert!(args.ends_with(&build_args));
    }
}
