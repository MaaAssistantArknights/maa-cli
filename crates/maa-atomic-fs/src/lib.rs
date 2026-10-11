#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! Utilities for atomically replacing individual files.
//!
//! Writes are staged in a temporary file beside the destination, synchronized,
//! and then atomically moved into place. A failure before the final replacement
//! leaves the destination unchanged.
//!
//! If the destination is a symbolic link, the file it points to is replaced and
//! the link itself is kept, matching [`fs::write`].
//!
//! On Unix, an existing destination keeps its permission bits, while a new
//! destination uses normal file creation permissions (`0o666` filtered by the
//! process umask). Other file metadata is not preserved.
//!
//! These operations do not provide multi-file or directory transactions, file
//! locking, or parent-directory synchronization for power-loss durability.

use std::{
    borrow::Cow,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

use tempfile::NamedTempFile;

/// Atomically replaces `path` with `content`.
pub fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> io::Result<()> {
    write_with(path, |file| file.write_all(content.as_ref()))
}

/// Atomically replaces `path` with all bytes read from `reader`.
pub fn write_from(path: impl AsRef<Path>, reader: &mut impl Read) -> io::Result<u64> {
    write_with(path, |file| io::copy(reader, file))
}

/// Atomically replaces `to` with the content of `from`.
///
/// Permissions follow the same rules as [`write`], not those of `from`.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let mut source = File::open(from)?;
    write_from(to, &mut source)
}

/// Atomically replaces `path` after `fill` populates a staging file.
///
/// `fill` may return any error type that can absorb [`io::Error`]. The
/// destination remains unchanged if filling, synchronizing, or replacing the
/// file fails.
pub fn write_with<P, F, T, E>(path: P, fill: F) -> Result<T, E>
where
    P: AsRef<Path>,
    F: FnOnce(&mut File) -> Result<T, E>,
    E: From<io::Error>,
{
    let path = resolve_symlink(path.as_ref())?;
    let mut temp = create_temp(&path)?;
    let result = fill(temp.as_file_mut())?;
    temp.as_file().sync_all()?;
    temp.into_temp_path()
        .persist(&path)
        .map_err(|error| error.error)?;
    Ok(result)
}

/// Returns the file a symbolic link points to, so that the link is kept.
///
/// A dangling link is returned unchanged and is replaced by a regular file.
fn resolve_symlink(path: &Path) -> io::Result<Cow<'_, Path>> {
    if !path.is_symlink() {
        return Ok(Cow::Borrowed(path));
    }
    match fs::canonicalize(path) {
        Ok(target) => Ok(Cow::Owned(target)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Cow::Borrowed(path)),
        Err(error) => Err(error),
    }
}

fn create_temp(path: &Path) -> io::Result<NamedTempFile> {
    let parent = parent_dir(path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let existing = match fs::metadata(path) {
            Ok(metadata) => Some(metadata.permissions()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        // The creation mode is filtered by the umask, like a regular new file.
        let temp = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o666))
            .tempfile_in(parent)?;
        if let Some(permissions) = existing {
            temp.as_file().set_permissions(permissions)?;
        }
        Ok(temp)
    }

    #[cfg(not(unix))]
    NamedTempFile::new_in(parent)
}

fn parent_dir(path: &Path) -> io::Result<&Path> {
    match path.parent() {
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Path {} has no parent directory", path.display()),
        )),
        Some(parent) if parent.as_os_str().is_empty() => Ok(Path::new(".")),
        Some(parent) => Ok(parent),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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
    fn write_from_replaces_existing_content() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        fs::write(&path, "old").unwrap();
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

    #[cfg(unix)]
    #[test]
    fn write_preserves_existing_permissions() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        write(&path, "new").unwrap();

        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn new_file_uses_regular_creation_permissions() {
        let temp_dir = tempdir().unwrap();
        let regular = temp_dir.path().join("regular.json");
        let atomic = temp_dir.path().join("atomic.json");
        File::create(&regular).unwrap();

        write(&atomic, "new").unwrap();

        let regular_mode = fs::metadata(regular).unwrap().permissions().mode() & 0o777;
        let atomic_mode = fs::metadata(atomic).unwrap().permissions().mode() & 0o777;
        assert_eq!(atomic_mode, regular_mode);
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_destination_permissions() {
        let temp_dir = tempdir().unwrap();
        let source = temp_dir.path().join("source.json");
        let target = temp_dir.path().join("target.json");
        fs::write(&source, "new").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o644)).unwrap();
        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();

        copy(&source, &target).unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(
            fs::metadata(target).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_through_symlink_keeps_link() {
        let temp_dir = tempdir().unwrap();
        let target_dir = temp_dir.path().join("store");
        fs::create_dir(&target_dir).unwrap();
        let target = target_dir.join("config.json");
        let link = temp_dir.path().join("config.json");
        fs::write(&target, "old").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        write(&link, "new").unwrap();

        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    }

    #[test]
    fn failed_fill_leaves_original_intact() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        fs::write(&path, "original").unwrap();

        let result = write_with::<_, _, (), io::Error>(&path, |_file| {
            Err(io::Error::other("simulated failure"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
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
}
