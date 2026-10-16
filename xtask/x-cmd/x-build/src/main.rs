//! `cargo x build`：通过 BUILD_* 准备二进制和许可证，命令行参数传给 Cargo。

use std::{env, ffi::OsString, fs, path::Path, process::Command};

use anyhow::{Context, Result};
use github::set_output;
use x_command::{CommandExt, cargo, rustup_up};
use x_group::Group;

mod github;

const HOST_TRIPLET: &str = env!("TARGET");

fn main() -> Result<()> {
    x_group::finish_prepare();
    let profile = x_env::read_or("BUILD_PROFILE", "dev".to_owned())?;
    let vendored_deps = x_env::read_or("BUILD_VENDORED_DEPS", false)?;
    let output_dir = env::var_os("BUILD_OUTPUT_DIR").context("Missing BUILD_OUTPUT_DIR")?;
    let rename = env::var_os("BUILD_RENAME");
    // Set GitHub output for use in subsequent steps
    set_output("host_triplet", HOST_TRIPLET).ok();

    if env::var_os("CI").is_some() {
        Group::new("Update Stable Toolchain")
            .run(|| rustup_up("stable").run().context("Failed to update Rust"))?;
    }

    let mut build_args = Vec::new();
    if vendored_deps {
        build_args.extend([
            OsString::from("--features"),
            OsString::from("git2?/vendored-libgit2,git2?/vendored-openssl"),
        ]);
    }
    build_args.extend(env::args_os().skip(1));

    Group::new("Build").run(|| {
        let mut cmd = cargo();
        cmd.args(["build", "--package", "maa-cli", "--locked"]);
        cmd.args(["--profile", &profile]);

        cmd.args(&build_args);

        cmd.run().context("Failed to build maa-cli")
    })?;

    let profile_dir = if profile == "dev" { "debug" } else { &profile };

    let target_dir = x_workspace::root().join("target").join(profile_dir);
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
        license_command(&build_args, &licenses)
            .run()
            .context("Failed to generate licenses.md")
    })?;

    Ok(())
}

fn license_command(build_args: &[OsString], output: &Path) -> Command {
    let mut cmd = cargo();
    cmd.current_dir(x_workspace::root()).args([
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
