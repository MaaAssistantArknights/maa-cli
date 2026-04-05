use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

use tempfile::NamedTempFile;

pub fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> io::Result<()> {
    write_with(path, |temp| temp.write_all(content.as_ref()))
}

pub fn write_from(path: impl AsRef<Path>, reader: &mut impl Read) -> io::Result<()> {
    write_with(path, |temp| io::copy(reader, temp).map(|_| ()))
}

pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    let from = from.as_ref();
    write_with(to, |temp| fs::copy(from, temp.path()).map(|_| ()))
}

/// Atomically write to `path` by letting `fill` populate a staging temp file,
/// then fsync + rename. The closure can fail with any error type that
/// absorbs `io::Error`, so callers may return e.g. `serde_json::Error`.
pub fn write_with<P, F, E>(path: P, fill: F) -> Result<(), E>
where
    P: AsRef<Path>,
    F: FnOnce(&mut NamedTempFile) -> Result<(), E>,
    E: From<io::Error>,
{
    let path = path.as_ref();
    let mut temp = NamedTempFile::new_in(parent_dir(path)?)?;
    fill(&mut temp)?;
    temp.as_file_mut().sync_all()?;
    persist(temp.into_temp_path(), path)?;
    Ok(())
}

fn persist(temp_path: tempfile::TempPath, path: &Path) -> io::Result<()> {
    temp_path.persist(path).map_err(|e| e.error)
}

fn parent_dir(path: &Path) -> io::Result<&Path> {
    match path.parent() {
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Path {} has no parent directory", path.display()),
        )),
        Some(p) if p.as_os_str().is_empty() => Ok(Path::new(".")),
        Some(p) => Ok(p),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{fs, io::Cursor};

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn write_replaces_existing_content() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");

        fs::write(&path, "old").unwrap();

        write(&path, "new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn write_from_writes_stream() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        let mut reader = Cursor::new(br#"{"foo":"bar"}"#);

        write_from(&path, &mut reader).unwrap();
        assert_eq!(fs::read(&path).unwrap(), br#"{"foo":"bar"}"#);
    }

    #[test]
    fn copy_replaces_existing_content() {
        let temp_dir = tempdir().unwrap();
        let source = temp_dir.path().join("source.json");
        let target = temp_dir.path().join("target.json");

        fs::write(&source, "new").unwrap();
        fs::write(&target, "old").unwrap();

        copy(&source, &target).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    }

    #[test]
    fn bare_filename_uses_current_directory() {
        assert_eq!(
            parent_dir(Path::new("output.json")).unwrap(),
            Path::new(".")
        );
    }

    #[test]
    fn rejects_root_path() {
        let error = write("/", "new").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn failed_fill_leaves_original_intact() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");

        fs::write(&path, "original").unwrap();

        let result = write_with::<_, _, io::Error>(&path, |_temp| {
            Err(io::Error::other("simulated failure"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }
}
