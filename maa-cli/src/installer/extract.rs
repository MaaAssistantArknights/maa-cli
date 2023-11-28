use crate::dirs::Ensure;

use std::{
    fs::File,
    io::copy,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};

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
pub struct Archive {
    file: PathBuf,
    file_type: ArchiveType,
}

impl Archive {
    /// Create a new `Archive` from a file with specified archive type.
    pub fn new(file: PathBuf, file_type: ArchiveType) -> Self {
        Self { file, file_type }
    }

    /// Create a new `Archive` from a file with automatically detected archive type.
    ///
    /// The archive type is determined by the file extension.
    ///
    /// # Errors
    ///
    /// Returns an error if the file extension is not supported.
    /// Currently only zip and tar.gz are supported.
    /// Or returns an error if the file extension cannot be determined.
    pub fn try_from(file: impl AsRef<Path>) -> Result<Self> {
        let file = file.as_ref();
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
            Ok(Self::new(file.to_path_buf(), archive_type))
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
    pub fn extract(&self, mapper: impl Fn(&Path) -> Option<PathBuf>) -> Result<()> {
        println!("Extracting archive file...");
        match self.file_type {
            ArchiveType::Zip => extract_zip(&self.file, mapper),
            ArchiveType::TarGz => extract_tar_gz(&self.file, mapper),
        }
    }
}

fn extract_zip(file: &Path, mapper: impl Fn(&Path) -> Option<PathBuf>) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(file)?)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        let outpath = match file.enclosed_name() {
            Some(path) => match mapper(path) {
                Some(path) => path,
                None => continue,
            },
            None => continue,
        };

        if file.is_dir() {
            continue;
        } else {
            if let Some(p) = outpath.parent() {
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

                if let Some(mode) = file.unix_mode() {
                    if mode & S_IFLNK == S_IFLNK {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents)?;
                        let link_target = std::ffi::OsString::from_vec(contents);
                        if outpath.exists() {
                            remove_file(&outpath).with_context(|| {
                                format!("Failed to remove existing file: {}", outpath.display())
                            })?;
                        }
                        symlink(link_target, &outpath).with_context(|| {
                            format!("Failed to extract file: {}", outpath.display())
                        })?;
                        continue;
                    }
                }
            }

            let mut outfile = File::create(&outpath)
                .with_context(|| format!("Failed to create file: {}", outpath.display()))?;
            copy(&mut file, &mut outfile)
                .with_context(|| format!("Failed to extract file: {}", outpath.display()))?;
        }

        #[cfg(unix)]
        {
            use std::fs::{set_permissions, Permissions};
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                set_permissions(&outpath, Permissions::from_mode(mode))
                    .with_context(|| format!("Failed to set permissions: {}", outpath.display()))?;
            }
        }
    }

    Ok(())
}

fn extract_tar_gz(file: &Path, mapper: impl Fn(&Path) -> Option<PathBuf>) -> Result<()> {
    let gz_decoder = flate2::read::GzDecoder::new(File::open(file)?);
    let mut archive = tar::Archive::new(gz_decoder);

    for entry in archive.entries()? {
        let mut file = entry?;

        let outpath = match &file.path() {
            Ok(path) => match mapper(path) {
                Some(path) => path,
                None => continue,
            },
            Err(e) => return Err(anyhow!("Error while reading tar entry: {}", e)),
        };

        if let Some(p) = outpath.parent() {
            p.ensure()?;
        }

        file.unpack(&outpath)?;
    }

    println!("Done!");

    Ok(())
}
