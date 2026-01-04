//! Archive creation utilities for release packaging.

use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

use anyhow::{Context, Result};
use flate2::{Compression, write::GzEncoder};
use sha2::{Digest, Sha256};

/// A writer that calculates a hash while writing through to another writer.
///
/// This is generic over the digest algorithm, allowing for easy switching
/// between different hash functions (SHA256, SHA512, BLAKE3, etc.).
struct HashingWriter<W: Write, D: Digest> {
    inner: W,
    hasher: D,
}

impl<W: Write, D: Digest> HashingWriter<W, D> {
    fn new(inner: W, hasher: D) -> Self {
        Self { inner, hasher }
    }

    fn finalize(self) -> (W, String)
    where
        D: Digest,
    {
        let hash = self.hasher.finalize();
        // Convert to hex string manually to avoid trait bound issues
        let hex = hash
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();
        (self.inner, hex)
    }
}

impl<W: Write, D: Digest> Write for HashingWriter<W, D> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Archive format for creating release packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// Tar with gzip compression (.tar.gz)
    TarGz,
    /// Zip (.zip)
    Zip,
}

impl ArchiveFormat {
    /// Get the file extension for this archive format.
    pub fn extension(&self) -> &'static str {
        match self {
            ArchiveFormat::TarGz => "tar.gz",
            ArchiveFormat::Zip => "zip",
        }
    }

    /// Create an archive with the specified files and return the SHA256 hash.
    ///
    /// The hash is calculated during archive creation without additional I/O.
    pub fn create<P: AsRef<Path>>(&self, output_path: P, files: &[(&str, &str)]) -> Result<String> {
        match self {
            ArchiveFormat::TarGz => create_tar_gz(output_path, files),
            ArchiveFormat::Zip => create_zip(output_path, files),
        }
    }
}

/// Extract a tar archive to a directory.
///
/// # Arguments
///
/// * `archive_path` - Path to the .tar file
/// * `output_dir` - Directory to extract to
pub fn extract_tar<P: AsRef<Path>, Q: AsRef<Path>>(archive_path: P, output_dir: Q) -> Result<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

    let mut archive = tar::Archive::new(file);
    archive
        .unpack(output_dir)
        .with_context(|| format!("Failed to extract archive to {}", output_dir.display()))?;

    Ok(())
}

/// Create a tar.gz archive containing specific files and return the SHA256 hash.
///
/// The tar format preserves the source file's permissions automatically.
/// The SHA256 hash is calculated during archive creation without additional I/O.
///
/// # Arguments
///
/// - `output_path` - Output path for the .tar.gz file
/// - `files` - Slice of (source_path, archive_path) tuples
///
/// # Returns
///
/// The SHA256 hash of the created archive as a lowercase hex string
pub fn create_tar_gz<P: AsRef<Path>>(output_path: P, files: &[(&str, &str)]) -> Result<String> {
    let output_path = output_path.as_ref();

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    let hashing_writer = HashingWriter::new(file, Sha256::new());
    let gz = GzEncoder::new(hashing_writer, Compression::default());
    let mut tar = tar::Builder::new(gz);

    for (source, archive_path) in files {
        tar.append_path_with_name(source, archive_path)
            .with_context(|| format!("Failed to add {source} to archive as {archive_path}"))?;
    }

    let gz = tar.into_inner().context("Failed to finalize tar archive")?;
    let hashing_writer = gz.finish().context("Failed to finalize gzip compression")?;
    let (file, hash) = hashing_writer.finalize();

    file.sync_all().context("Failed to sync file")?;

    Ok(hash)
}

/// Create a zip archive containing specific files and return the SHA256 hash.
///
/// Used primarily for Windows packages where permissions are not critical.
/// The SHA256 hash is calculated after archive creation (zip requires Seek, so streaming isn't
/// possible).
///
/// # Arguments
/// * `output_path` - Output path for the .zip file
/// * `files` - Slice of (source_path, archive_path) tuples
///
/// # Returns
///
/// The SHA256 hash of the created archive as a lowercase hex string
pub fn create_zip<P: AsRef<Path>>(output_path: P, files: &[(&str, &str)]) -> Result<String> {
    let output_path = output_path.as_ref();

    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (source, archive_path) in files {
        let source_path = Path::new(source);
        let mut source_file =
            File::open(source_path).with_context(|| format!("Failed to open file: {source}"))?;

        zip.start_file(*archive_path, options)
            .with_context(|| format!("Failed to start zip entry: {archive_path}"))?;

        io::copy(&mut source_file, &mut zip)
            .with_context(|| format!("Failed to write {source} to archive"))?;
    }

    zip.finish().context("Failed to finalize zip archive")?;

    // Calculate hash after creation (zip requires Seek which prevents streaming hash)
    let mut file = File::open(output_path).with_context(|| {
        format!(
            "Failed to open archive for hashing: {}",
            output_path.display()
        )
    })?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)
        .with_context(|| format!("Failed to hash archive: {}", output_path.display()))?;

    Ok(format!("{:x}", hasher.finalize()))
}
