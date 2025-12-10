//! Archive extractors

use std::{
    fs::File,
    path::{Path, PathBuf},
};

use indicatif::ProgressBar;

use crate::error::{Error, ErrorKind, Result, WithDesc};

fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path).with_desc("Failed to create directory")?;
    }
    Ok(())
}

/// A trait for archive formats that can be extracted.
///
/// Implementers of this trait can extract files from an archive to the filesystem,
/// using a mapper function to determine whether and where to extract each file.
pub trait Archive {
    /// Extracts the archive contents.
    ///
    /// # Parameters
    ///
    /// * `mapper` - A function that takes a path from the archive and returns either a destination
    ///   path to extract the file to, or `None` to skip extracting this file.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure of the extraction.
    fn extract(self, ui: ProgressBar, mapper: impl FnMut(&Path) -> Option<PathBuf>) -> Result<()>;
}

/// A wrapper for archive files that can be extracted.
pub struct ArchiveFile<'a>(&'a Path);

impl<'a> ArchiveFile<'a> {
    /// Create a new archive file wrapper.
    pub fn new(path: &'a Path) -> Self {
        Self(path)
    }

    /// Extract the archive using the provided mapper function.
    pub fn extract(
        self,
        ui: ProgressBar,
        mapper: impl FnMut(&Path) -> Option<PathBuf>,
    ) -> Result<()> {
        let file = File::open(self.0)
            .then_with_desc(|| format!("Failed to open archive: {}", self.0.display()))?;

        // Check for compound extensions like .tar.gz
        let file_name = self.0.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match () {
            #[cfg(feature = "zip")]
            _ if file_name.ends_with(".zip") => {
                let archive =
                    ::zip::ZipArchive::new(file).with_desc("Failed to read zip archive")?;
                archive.extract(ui, mapper)
            }
            #[cfg(feature = "gz")]
            _ if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") => {
                let decoder = flate2::read::GzDecoder::new(file);
                let archive = ::tar::Archive::new(decoder);
                archive.extract(ui, mapper)
            }
            #[cfg(feature = "tar")]
            _ if file_name.ends_with(".tar") => {
                let archive = ::tar::Archive::new(file);
                archive.extract(ui, mapper)
            }
            _ => {
                let extension = self.0.extension().and_then(|e| e.to_str()).unwrap_or("");
                Err(Error::new(ErrorKind::Extract)
                    .with_desc(format!("Unsupported archive format: {}", extension)))
            }
        }
    }
}

#[cfg(feature = "zip")]
mod zip {
    use std::io::{Read, Seek};

    use ::zip::{ZipArchive, result::ZipError};

    use super::*;

    impl<R: Read + Seek> Archive for ZipArchive<R> {
        fn extract(
            mut self,
            ui: ProgressBar,
            mut mapper: impl FnMut(&Path) -> Option<PathBuf>,
        ) -> Result<()> {
            for i in 0..self.len() {
                let mut file = self
                    .by_index(i)
                    .with_desc("Failed to get file from zip archive")?;

                let src_path = file.enclosed_name().ok_or_else(|| {
                    Error::new(ErrorKind::Extract).with_desc("Bad file path in zip archive")
                })?;
                let dst = match mapper(&src_path) {
                    Some(path) => {
                        ui.set_message(format!(
                            "Extracting {} to {}",
                            src_path.display(),
                            path.display()
                        ));
                        ui.tick();
                        path
                    }
                    None => continue,
                };
                let dst = dst.as_path();

                if file.is_dir() {
                    continue;
                }

                if let Some(dir) = dst.parent() {
                    ensure_dir_exists(dir)?;
                }

                // Resolve symlinks
                #[cfg(unix)]
                {
                    use std::os::unix::{ffi::OsStringExt, fs::symlink};

                    const S_IFLNK: u32 = 0o120000;

                    if let Some(mode) = file.unix_mode()
                        && mode & S_IFLNK == S_IFLNK
                    {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents)?;
                        let link_target = std::ffi::OsString::from_vec(contents);
                        if dst.exists() {
                            std::fs::remove_file(dst)?;
                        }
                        symlink(link_target, dst).then_with_desc(|| {
                            format!("Failed to extract file: {}", dst.display())
                        })?;
                        continue;
                    }
                }

                let mut outfile = File::create(dst)
                    .then_with_desc(|| format!("Failed to create file: {}", dst.display()))?;
                std::io::copy(&mut file, &mut outfile)
                    .then_with_desc(|| format!("Failed to extract file: {}", dst.display()))?;

                #[cfg(unix)]
                {
                    use std::{
                        fs::{Permissions, set_permissions},
                        os::unix::fs::PermissionsExt,
                    };

                    if let Some(mode) = file.unix_mode() {
                        set_permissions(dst, Permissions::from_mode(mode)).then_with_desc(
                            || format!("Failed to set permissions: {}", dst.display()),
                        )?;
                    }
                }
            }

            Ok(())
        }
    }

    impl From<ZipError> for Error {
        fn from(err: ZipError) -> Self {
            match err {
                ZipError::Io(e) => Error::new(ErrorKind::Io).with_source(e),
                err => Error::new(ErrorKind::Extract).with_source(err),
            }
        }
    }
}

#[cfg(feature = "tar")]
mod tar {
    use std::io::Read;

    use super::*;

    impl<R: Read> Archive for ::tar::Archive<R> {
        fn extract(
            mut self,
            ui: ProgressBar,
            mut mapper: impl FnMut(&Path) -> Option<PathBuf>,
        ) -> Result<()> {
            for entry in self
                .entries()
                .with_desc("Failed to read file entry in archive")?
            {
                let mut entry = entry.with_desc("Invalid file entry in archive")?;
                let entry_path = entry.path().with_desc("Invalid file path in archive")?;
                let dst = match mapper(entry_path.as_ref()) {
                    Some(path) => {
                        ui.set_message(format!(
                            "Extracting {} to {}",
                            entry_path.display(),
                            path.display()
                        ));
                        ui.tick();
                        path
                    }
                    None => continue,
                };

                if let Some(parent) = dst.parent() {
                    ensure_dir_exists(parent)?;
                }

                entry.unpack(&dst)?;
            }

            Ok(())
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::*;

    /// Helper to create a test directory structure
    fn create_test_files(dir: &Path) -> Result<()> {
        fs::write(dir.join("file1.txt"), b"content1")?;
        fs::write(dir.join("file2.txt"), b"content2")?;
        fs::create_dir(dir.join("subdir"))?;
        fs::write(dir.join("subdir").join("file3.txt"), b"content3")?;
        Ok(())
    }

    #[cfg(all(unix, feature = "zip"))]
    #[test]
    fn test_extract_zip_with_zip_command() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.zip");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test files
        fs::create_dir(&source_dir).unwrap();
        create_test_files(&source_dir).unwrap();

        // Create zip archive using zip command
        let output = Command::new("zip")
            .arg("-r")
            .arg(&archive_path)
            .arg(".")
            .current_dir(&source_dir)
            .output()
            .expect("Failed to execute zip command - is zip installed?");

        assert!(output.status.success(), "zip command failed");
        assert!(archive_path.exists());

        // Extract using our implementation
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify extracted files
        assert_eq!(
            fs::read_to_string(extract_dir.join("file1.txt")).unwrap(),
            "content1"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join("file2.txt")).unwrap(),
            "content2"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join("subdir/file3.txt")).unwrap(),
            "content3"
        );
    }

    #[cfg(all(unix, feature = "tar"))]
    #[test]
    fn test_extract_tar_with_tar_command() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tar");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test files
        fs::create_dir(&source_dir).unwrap();
        create_test_files(&source_dir).unwrap();

        // Create tar archive using tar command
        let output = Command::new("tar")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg(".")
            .output()
            .expect("Failed to execute tar command");

        assert!(output.status.success(), "tar command failed");
        assert!(archive_path.exists());

        // Extract using our implementation
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify extracted files
        assert_eq!(
            fs::read_to_string(extract_dir.join("file1.txt")).unwrap(),
            "content1"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join("file2.txt")).unwrap(),
            "content2"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join("subdir/file3.txt")).unwrap(),
            "content3"
        );
    }

    #[cfg(all(unix, feature = "gz"))]
    #[test]
    fn test_extract_tar_gz_with_tar_command() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tar.gz");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test files
        fs::create_dir(&source_dir).unwrap();
        create_test_files(&source_dir).unwrap();

        // Create tar.gz archive using tar command
        let output = Command::new("tar")
            .arg("-czf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg(".")
            .output()
            .expect("Failed to execute tar command");

        assert!(output.status.success(), "tar command failed");
        assert!(archive_path.exists());

        // Extract using our implementation
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify extracted files
        assert_eq!(
            fs::read_to_string(extract_dir.join("file1.txt")).unwrap(),
            "content1"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join("file2.txt")).unwrap(),
            "content2"
        );
        assert_eq!(
            fs::read_to_string(extract_dir.join("subdir/file3.txt")).unwrap(),
            "content3"
        );
    }

    #[cfg(all(unix, feature = "gz"))]
    #[test]
    fn test_extract_tgz_extension() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tgz");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test files
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("test.txt"), b"test content").unwrap();

        // Create .tgz archive
        let output = Command::new("tar")
            .arg("-czf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg(".")
            .output()
            .expect("Failed to execute tar command");

        assert!(output.status.success(), "tar command failed");

        // Extract using our implementation
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify
        assert_eq!(
            fs::read_to_string(extract_dir.join("test.txt")).unwrap(),
            "test content"
        );
    }

    #[cfg(feature = "zip")]
    #[test]
    fn test_extract_with_filter() {
        use std::io::Write;

        use ::zip::{ZipWriter, write::SimpleFileOptions};

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.zip");
        let extract_dir = temp_dir.path().join("extract");

        // Create zip manually
        {
            let file = fs::File::create(&archive_path).unwrap();
            let mut zip = ZipWriter::new(file);

            zip.start_file("file1.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"content1").unwrap();

            zip.start_file("file2.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"content2").unwrap();

            zip.start_file("skip_me.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"skip").unwrap();

            zip.finish().unwrap();
        }

        // Extract only files that don't contain "skip"
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| {
                if path.to_str().unwrap().contains("skip") {
                    None
                } else {
                    Some(extract_dir.join(path))
                }
            })
            .unwrap();

        // Verify
        assert!(extract_dir.join("file1.txt").exists());
        assert!(extract_dir.join("file2.txt").exists());
        assert!(!extract_dir.join("skip_me.txt").exists());
    }

    #[cfg(all(unix, feature = "zip"))]
    #[test]
    fn test_extract_preserves_permissions() {
        use std::{os::unix::fs::PermissionsExt, process::Command};

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.zip");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test file with specific permissions
        fs::create_dir(&source_dir).unwrap();
        let executable = source_dir.join("script.sh");
        fs::write(&executable, b"#!/bin/sh\necho hello").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

        // Create zip
        Command::new("zip")
            .arg("-r")
            .arg(&archive_path)
            .arg(".")
            .current_dir(&source_dir)
            .output()
            .expect("Failed to execute zip command");

        // Extract
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify permissions
        let extracted = extract_dir.join("script.sh");
        let metadata = fs::metadata(&extracted).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(mode & 0o777, 0o755, "Permissions should be preserved");
    }

    #[test]
    fn test_unsupported_format() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.rar");

        // Create a dummy file
        fs::write(&archive_path, b"not a real archive").unwrap();

        // Try to extract
        let result = ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(path.to_path_buf()));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Extract);
    }

    #[test]
    fn test_gz_without_tar_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.gz");

        // Create a dummy .gz file (not .tar.gz)
        fs::write(&archive_path, b"not a tar.gz").unwrap();

        // Try to extract - should fail because .gz alone is not supported
        let result = ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(path.to_path_buf()));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Extract);
    }

    #[test]
    fn test_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("nonexistent.zip");

        let result = ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(path.to_path_buf()));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Io);
    }

    #[cfg(all(unix, feature = "gz"))]
    #[test]
    fn test_extract_nested_directories() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tar.gz");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create nested directory structure
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir_all(source_dir.join("a/b/c")).unwrap();
        fs::write(source_dir.join("a/b/c/deep.txt"), b"deep content").unwrap();

        // Create archive
        Command::new("tar")
            .arg("-czf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg(".")
            .output()
            .expect("Failed to execute tar command");

        // Extract
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify
        assert_eq!(
            fs::read_to_string(extract_dir.join("a/b/c/deep.txt")).unwrap(),
            "deep content"
        );
    }

    #[cfg(feature = "zip")]
    #[test]
    fn test_extract_empty_archive() {
        use ::zip::ZipWriter;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("empty.zip");
        let extract_dir = temp_dir.path().join("extract");

        // Create empty zip
        {
            let file = fs::File::create(&archive_path).unwrap();
            let zip = ZipWriter::new(file);
            zip.finish().unwrap();
        }

        // Extract
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify no files extracted (only the directory exists)
        let entries: Vec<_> = fs::read_dir(&extract_dir).unwrap().collect();
        assert_eq!(entries.len(), 0);
    }

    #[cfg(all(unix, feature = "zip"))]
    #[test]
    fn test_extract_zip_with_symlinks() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.zip");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test files and symlinks
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("target.txt"), b"target content").unwrap();

        // Create a symlink
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("target.txt", source_dir.join("link.txt")).unwrap();
        }

        // Create zip archive using zip command (preserves symlinks)
        let output = Command::new("zip")
            .arg("-ry") // -y preserves symlinks
            .arg(&archive_path)
            .arg(".")
            .current_dir(&source_dir)
            .output()
            .expect("Failed to execute zip command - is zip installed?");

        assert!(output.status.success(), "zip command failed");
        assert!(archive_path.exists());

        // Extract using our implementation
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify target file exists
        assert!(extract_dir.join("target.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("target.txt")).unwrap(),
            "target content"
        );

        // Verify symlink was created and points to the correct target
        let link_path = extract_dir.join("link.txt");
        assert!(link_path.exists(), "Symlink should exist");

        #[cfg(unix)]
        {
            let metadata = fs::symlink_metadata(&link_path).unwrap();
            assert!(metadata.is_symlink(), "Should be a symlink");

            // Verify link target
            let link_target = fs::read_link(&link_path).unwrap();
            assert_eq!(link_target, Path::new("target.txt"));

            // Verify we can read through the symlink
            assert_eq!(fs::read_to_string(&link_path).unwrap(), "target content");
        }
    }

    #[cfg(all(unix, feature = "tar"))]
    #[test]
    fn test_extract_tar_with_symlinks() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tar");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test files and symlinks
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("file.txt"), b"file content").unwrap();

        // Create symlinks
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("file.txt", source_dir.join("link_to_file.txt")).unwrap();

            // Create directory and symlink to directory
            fs::create_dir(source_dir.join("dir")).unwrap();
            fs::write(source_dir.join("dir/inner.txt"), b"inner content").unwrap();
            symlink("dir", source_dir.join("link_to_dir")).unwrap();
        }

        // Create tar archive using tar command
        let output = Command::new("tar")
            .arg("-chf") // -h dereferences symlinks, -c creates, -f specifies file
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg(".")
            .output()
            .expect("Failed to execute tar command");

        assert!(output.status.success(), "tar command failed");
        assert!(archive_path.exists());

        // Extract using our implementation
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify files exist
        assert!(extract_dir.join("file.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("file.txt")).unwrap(),
            "file content"
        );

        // Note: with -h flag, symlinks are dereferenced so they become regular files
        assert!(extract_dir.join("link_to_file.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("link_to_file.txt")).unwrap(),
            "file content"
        );
    }

    #[cfg(all(unix, feature = "tar"))]
    #[test]
    fn test_extract_tar_preserves_symlinks() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tar");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test files and symlinks
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("original.txt"), b"original").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("original.txt", source_dir.join("symlink.txt")).unwrap();
        }

        // Create tar archive WITHOUT -h flag to preserve symlinks
        let output = Command::new("tar")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg(".")
            .output()
            .expect("Failed to execute tar command");

        assert!(output.status.success(), "tar command failed");

        // Extract using our implementation
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify symlink is preserved
        #[cfg(unix)]
        {
            let link_path = extract_dir.join("symlink.txt");
            let metadata = fs::symlink_metadata(&link_path).unwrap();
            assert!(metadata.is_symlink(), "Should be a symlink");

            let link_target = fs::read_link(&link_path).unwrap();
            assert_eq!(link_target, Path::new("original.txt"));
        }
    }

    #[cfg(all(unix, feature = "gz"))]
    #[test]
    fn test_extract_tar_gz_with_symlinks() {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.tar.gz");
        let source_dir = temp_dir.path().join("source");
        let extract_dir = temp_dir.path().join("extract");

        // Create test structure with symlinks
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("data.txt"), b"data").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("data.txt", source_dir.join("link.txt")).unwrap();

            // Relative symlink
            fs::create_dir(source_dir.join("subdir")).unwrap();
            fs::write(source_dir.join("subdir/file.txt"), b"subfile").unwrap();
            symlink("../data.txt", source_dir.join("subdir/parent_link.txt")).unwrap();
        }

        // Create tar.gz archive preserving symlinks
        let output = Command::new("tar")
            .arg("-czf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&source_dir)
            .arg(".")
            .output()
            .expect("Failed to execute tar command");

        assert!(output.status.success(), "tar command failed");

        // Extract
        fs::create_dir(&extract_dir).unwrap();
        ArchiveFile::new(&archive_path)
            .extract(ProgressBar::hidden(), |path| Some(extract_dir.join(path)))
            .unwrap();

        // Verify symlinks
        #[cfg(unix)]
        {
            // Check simple symlink
            let link1 = extract_dir.join("link.txt");
            assert!(fs::symlink_metadata(&link1).unwrap().is_symlink());
            assert_eq!(fs::read_link(&link1).unwrap(), Path::new("data.txt"));

            // Check relative symlink
            let link2 = extract_dir.join("subdir/parent_link.txt");
            assert!(fs::symlink_metadata(&link2).unwrap().is_symlink());
            assert_eq!(fs::read_link(&link2).unwrap(), Path::new("../data.txt"));

            // Verify we can read through symlinks
            assert_eq!(fs::read_to_string(&link1).unwrap(), "data");
            assert_eq!(fs::read_to_string(&link2).unwrap(), "data");
        }
    }
}
