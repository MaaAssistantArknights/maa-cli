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
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["release", "channels"])
        .env("CHANNEL", channel)
        .env("MANIFEST", "manifest.json")
        .env("INDEX_DIR", "index")
        .env("GITHUB_OUTPUT", "output")
        .current_dir(root)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "channel selection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(fs::read_to_string(root.join("output"))?)
}

#[test]
fn stable_retry_updates_only_indexes_that_have_not_advanced() -> Result<()> {
    assert_eq!(
        select("stable", "0.7.6", &[
            ("stable", "0.7.5"),
            ("beta", "0.8.0-beta.1"),
            ("nightly", "0.8.0-nightly.100+sha.abc"),
        ])?,
        "channels=stable\n"
    );
    assert_eq!(
        select("stable", "0.8.0", &[
            ("stable", "0.8.0"),
            ("beta", "0.8.0-beta.1"),
            ("nightly", "0.8.0-nightly.100+sha.abc"),
        ])?,
        "channels=stable beta nightly\n"
    );
    Ok(())
}

#[test]
fn prereleases_only_select_their_own_channel() -> Result<()> {
    assert_eq!(select("beta", "0.8.0-beta.2", &[])?, "channels=beta\n");
    assert_eq!(
        select("nightly", "0.8.0-nightly.100+sha.abc", &[])?,
        "channels=nightly\n"
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
        "channels=nightly\n"
    );
    Ok(())
}
