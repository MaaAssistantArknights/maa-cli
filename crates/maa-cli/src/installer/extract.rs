use std::{
    borrow::Cow,
    fs::File,
    io::copy,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::dirs::Ensure;

/// Supported archive types.
///
/// Currently only zip and tar.gz are supported.
/// This enum is used to determine which extractor to use.
pub enum ArchiveType {
    Zip,
    TarGz,
}

/// An archive file.
///
/// This struct is used to represent an archive file.
/// It can be used to extract the archive file to a directory.
/// The archive type can be specified manually or automatically detected from the file extension.
/// Currently only zip and tar.gz are supported.
pub struct Archive<'f> {
    file: Cow<'f, Path>,
    archive_type: ArchiveType,
}

impl<'f> Archive<'f> {
    /// Create a new `Archive` from a file with automatically detected archive type.
    ///
    /// The archive type is determined by the file extension.
    ///
    /// # Errors
    ///
    /// Returns an error if the file extension is not supported.
    /// Currently only zip and tar.gz are supported.
    /// Or returns an error if the file extension cannot be determined.
    pub fn new(file: Cow<'f, Path>) -> Result<Self> {
        if let Some(extension) = file.extension() {
            let archive_type = match extension.to_str() {
                Some("zip") => ArchiveType::Zip,
                Some("gz") => {
                    let stem = file.file_stem().map(PathBuf::from);
                    if stem.is_some_and(|s| s.extension().is_some_and(|e| e == "tar")) {
                        ArchiveType::TarGz
                    } else {
                        bail!("Unsupported archive type")
                    }
                }
                _ => bail!("Unsupported archive type"),
            };

            Ok(Self { file, archive_type })
        } else {
            Err(anyhow!("Failed to get file extension"))
        }
    }

    /// Extract the archive file with a mapper function.
    ///
    /// The mapper function is used to map the file path in the archive to the output path.
    /// If the mapper function returns `None`, the file will be skipped.
    /// This is useful when you want to extract only some files from the archive.
    /// If the output path does not exist, it will be created.
    /// If the output path exists, the file will be skipped if the file size matches.
    /// Otherwise, the file will be overwritten.
    /// The file permissions will be preserved.
    pub fn extract<F>(&self, mapper: F) -> Result<()>
    where
        F: FnMut(Cow<Path>) -> Option<PathBuf>,
    {
        println!("Extracting archive file...");
        match self.archive_type {
            ArchiveType::Zip => extract_zip(&self.file, mapper),
            ArchiveType::TarGz => extract_tar_gz(&self.file, mapper),
        }
    }
}

fn extract_zip<F>(file: &Path, mut mapper: F) -> Result<()>
where
    F: FnMut(Cow<Path>) -> Option<PathBuf>,
{
    let mut archive = zip::ZipArchive::new(File::open(file)?)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        let src_path = file
            .enclosed_name()
            .context("Bad file path in zip archive")?
            .into();
        let dst = match mapper(src_path) {
            Some(path) => path,
            None => continue,
        };
        let dst = dst.as_path();

        if file.is_dir() {
            continue;
        } else {
            if let Some(p) = dst.parent() {
                p.ensure()?;
            }

            #[cfg(unix)]
            {
                use std::{
                    fs::remove_file,
                    io::Read,
                    os::unix::{ffi::OsStringExt, fs::symlink},
                };

                const S_IFLNK: u32 = 0o120000;

                if let Some(mode) = file.unix_mode()
                    && mode & S_IFLNK == S_IFLNK
                {
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    let link_target = std::ffi::OsString::from_vec(contents);
                    if dst.exists() {
                        remove_file(dst).with_context(|| {
                            format!("Failed to remove existing file: {}", dst.display())
                        })?;
                    }
                    symlink(link_target, dst)
                        .with_context(|| format!("Failed to extract file: {}", dst.display()))?;
                    continue;
                }
            }

            let mut outfile = File::create(dst)
                .with_context(|| format!("Failed to create file: {}", dst.display()))?;
            copy(&mut file, &mut outfile)
                .with_context(|| format!("Failed to extract file: {}", dst.display()))?;
        }

        #[cfg(unix)]
        {
            use std::{
                fs::{Permissions, set_permissions},
                os::unix::fs::PermissionsExt,
            };

            if let Some(mode) = file.unix_mode() {
                set_permissions(dst, Permissions::from_mode(mode))
                    .with_context(|| format!("Failed to set permissions: {}", dst.display()))?;
            }
        }
    }

    Ok(())
}

fn extract_tar_gz<F>(file: &Path, mut mapper: F) -> Result<()>
where
    F: FnMut(Cow<Path>) -> Option<PathBuf>,
{
    let gz_decoder = flate2::read::GzDecoder::new(File::open(file)?);
    let mut archive = tar::Archive::new(gz_decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path().context("Bad file path in tar.gz archive")?;
        let dst = match mapper(entry_path) {
            Some(path) => path,
            None => continue,
        };

        if let Some(p) = dst.parent() {
            p.ensure()?;
        }

        entry.unpack(&dst)?;
    }

    println!("Done!");

    Ok(())
}
