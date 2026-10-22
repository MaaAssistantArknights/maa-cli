use std::{fs, process::Command};

use anyhow::Result;

fn select(channel: &str, version: &str, indexes: &[(&str, &str)]) -> Result<String> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("manifest.json"),
        format!(r#"{{"version":"{version}","details":null}}"#),
    )?;
    fs::create_dir(root.join("index"))?;
    for (channel, version) in indexes {
        fs::write(
            root.join(format!("index/{channel}.json")),
            format!(r#"{{"version":"{version}"}}"#),
        )?;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_x-channel"))
        .env("CHANNEL", channel)
        .env("MANIFEST", "manifest.json")
        .env("INDEX_DIR", "index")
        .env("GITHUB_ACTIONS", "true")
        .current_dir(root)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "channel selection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"::endgroup::\n");
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn stable_retry_updates_only_indexes_that_have_not_advanced() -> Result<()> {
    assert_eq!(
        select("stable", "0.7.6", &[
            ("stable", "0.7.5"),
            ("beta", "0.8.0-beta.1"),
            ("nightly", "0.8.0-nightly.100+sha.abc"),
        ])?,
        "stable\n"
    );
    assert_eq!(
        select("stable", "0.8.0", &[
            ("stable", "0.8.0"),
            ("beta", "0.8.0-beta.1"),
            ("nightly", "0.8.0-nightly.100+sha.abc"),
        ])?,
        "stable beta nightly\n"
    );
    Ok(())
}

#[test]
fn prereleases_only_select_their_own_channel() -> Result<()> {
    assert_eq!(select("beta", "0.8.0-beta.2", &[])?, "beta\n");
    assert_eq!(
        select("nightly", "0.8.0-nightly.100+sha.abc", &[])?,
        "nightly\n"
    );
    Ok(())
}

#[test]
fn nightly_always_updates_its_index() -> Result<()> {
    assert_eq!(
        select("nightly", "0.8.0-nightly.99+sha.def", &[(
            "nightly",
            "0.8.0-nightly.100+sha.abc"
        )])?,
        "nightly\n"
    );
    Ok(())
}

#[test]
fn nightly_needs_no_manifest_or_index() -> Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_x-channel"))
        .env("CHANNEL", "nightly")
        .env_remove("MANIFEST")
        .env_remove("INDEX_DIR")
        .env_remove("GITHUB_OUTPUT")
        .env("GITHUB_ACTIONS", "true")
        .output()?;
    assert!(output.status.success());
    assert_eq!(output.stdout, b"nightly\n");
    assert_eq!(output.stderr, b"::endgroup::\n");
    Ok(())
}

#[test]
fn beta_skips_newer_versions_and_ignores_build_metadata() -> Result<()> {
    assert_eq!(
        select("beta", "0.8.0-beta.1", &[("beta", "0.8.0-beta.2")])?,
        "\n"
    );
    assert_eq!(
        select("beta", "0.8.0-beta.2+sha.abc", &[(
            "beta",
            "0.8.0-beta.2+sha.def"
        )])?,
        "beta\n"
    );
    Ok(())
}
