//! Read `CHANNEL` and print eligible release index channels to stdout.

use std::{cmp::Ordering, env, fs, io::ErrorKind, path::Path};

use anyhow::{Context, Result, bail};
use semver::Version;
use serde_json::Value;

fn main() -> Result<()> {
    x_group::finish_prepare();
    let channel: String = x_env::read("CHANNEL")?;
    let candidates: &[&str] = match channel.as_str() {
        "stable" => &["stable", "beta", "nightly"],
        "beta" => &["beta"],
        // The rolling Nightly release is always replaced, so its index must follow it.
        "nightly" => {
            println!("nightly");
            return Ok(());
        }
        _ => bail!("Unknown release channel: {channel}"),
    };
    let manifest = env::var_os("MANIFEST").context("Missing MANIFEST")?;
    let index_dir = env::var_os("INDEX_DIR").context("Missing INDEX_DIR")?;
    let new = read_version(Path::new(&manifest))?
        .with_context(|| format!("Manifest {} does not exist", manifest.display()))?;
    let index_dir = Path::new(&index_dir);
    let mut selected = Vec::new();
    for &channel in candidates {
        if let Some(old) = read_version(&index_dir.join(format!("{channel}.json")))?
            && new.cmp_precedence(&old) == Ordering::Less
        {
            continue;
        }
        selected.push(channel);
    }
    println!("{}", selected.join(" "));
    Ok(())
}

fn read_version(path: &Path) -> Result<Option<Version>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("Read {}", path.display())),
    };
    let json: Value =
        serde_json::from_slice(&bytes).with_context(|| format!("Parse {}", path.display()))?;
    let version = json["version"]
        .as_str()
        .with_context(|| format!("{}: missing version string", path.display()))?;
    let version = Version::parse(version)
        .with_context(|| format!("{}: invalid version {version}", path.display()))?;
    Ok(Some(version))
}
