use std::{fs, path::Path, process::Command};

use anyhow::{Result, ensure};

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(args)
        .current_dir(dir)
        .output()?;
    ensure!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn commit(dir: &Path, version: &str) -> Result<()> {
    fs::create_dir_all(dir.join("crates/maa-cli"))?;
    fs::write(
        dir.join("crates/maa-cli/Cargo.toml"),
        format!("[package]\nname = \"maa-cli\"\nversion = \"{version}\"\n"),
    )?;
    git(dir, &["add", "."])?;
    git(dir, &[
        "commit",
        "--allow-empty",
        "-m",
        &format!("version {version}"),
    ])?;
    Ok(())
}

/// Create a bare origin and a clone whose `main` holds `version`.
fn setup(version: &str) -> Result<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    let work = temp.path().join("work");
    git(temp.path(), &["init", "--bare", "-b", "main", "origin.git"])?;
    git(temp.path(), &["clone", "origin.git", "work"])?;
    git(&work, &["checkout", "-b", "main"])?;
    commit(&work, version)?;
    git(&work, &["push", "origin", "main"])?;
    Ok(temp)
}

fn tag(work: &Path, channel: &str) -> Result<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_x-tag"))
        .args([channel, "--yes"])
        .current_dir(work)
        .output()?)
}

fn remote_tag(temp: &Path, tag: &str) -> Result<String> {
    git(&temp.join("origin.git"), &["rev-parse", tag])
}

#[test]
fn stable_tags_remote_main_with_cargo_version() -> Result<()> {
    let temp = setup("0.7.6")?;
    let work = temp.path().join("work");
    let main = git(&work, &["rev-parse", "HEAD"])?;
    // Unpushed local commits are not tagged.
    commit(&work, "0.7.6")?;

    let output = tag(&work, "stable")?;
    ensure!(output.status.success(), "{output:?}");
    assert_eq!(remote_tag(temp.path(), "v0.7.6")?, main);

    // An existing tag is never moved to a newer main commit.
    git(&work, &["push", "origin", "main"])?;
    assert!(!tag(&work, "stable")?.status.success());
    assert_eq!(remote_tag(temp.path(), "v0.7.6")?, main);
    Ok(())
}

#[test]
fn beta_increments_number_on_the_dev_branch() -> Result<()> {
    let temp = setup("0.7.6")?;
    let work = temp.path().join("work");
    assert!(!tag(&work, "beta")?.status.success());

    git(&work, &["checkout", "-b", "dev-0.8"])?;
    commit(&work, "0.8.0")?;
    git(&work, &["push", "origin", "dev-0.8"])?;
    let first = git(&work, &["rev-parse", "HEAD"])?;
    let output = tag(&work, "beta")?;
    ensure!(output.status.success(), "{output:?}");
    assert_eq!(remote_tag(temp.path(), "v0.8.0-beta.1")?, first);

    commit(&work, "0.8.0")?;
    git(&work, &["push", "origin", "dev-0.8"])?;
    let second = git(&work, &["rev-parse", "HEAD"])?;
    let output = tag(&work, "beta")?;
    ensure!(output.status.success(), "{output:?}");
    assert_eq!(remote_tag(temp.path(), "v0.8.0-beta.2")?, second);

    git(&work, &["push", "origin", "main:dev-0.9"])?;
    assert!(!tag(&work, "beta")?.status.success());
    Ok(())
}
