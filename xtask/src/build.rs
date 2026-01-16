//! Build automation for maa-cli.

use std::fs::File;
use std::env;

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

    Group::new("Build").run(|| {
        let mut cmd = cargo();
        cmd.args(["build", "--package", "maa-cli", "--locked"]);
        cmd.args(["--profile", &opts.profile]);

        if opts.vendored_deps {
            cmd.args([
                "--features",
                "git2?/vendored-libgit2,git2?/vendored-openssl",
            ]);
        }

        // Add any additional arguments
        cmd.args(&opts.build_args);

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
        Group::new("Create Tar Package").run(|| {
            let tar_name = format!("{HOST_TRIPLET}.tar");

            let file = File::create(&tar_name)
                .with_context(|| format!("Failed to create tar file: {tar_name}"))?;

            let mut tar = tar::Builder::new(file);
            let archive_name = opts.rename.unwrap_or(exe);
            tar.append_path_with_name(&binary, archive_name)
                .with_context(|| format!("Failed to add {binary} to tar archive"))?;

            tar.finish().context("Failed to finalize tar archive")?;

            Ok(())
        })?;
    }

    Ok(())
}
