use std::cmp::min;
use std::fs::{remove_file, File};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use digest::Digest;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::Sha256;

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Verify,
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Reqwest(e) => e.fmt(f),
            Error::Io(e) => e.fmt(f),
            Error::Verify => write!(f, "Checksum verification failed"),
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

/// Checksum checker.
///
/// Currently only support sha256.
/// Used to verify the integrity of downloaded files.
pub enum Checker<'a> {
    Sha256(&'a str),
}

impl<'a> Checker<'a> {
    fn hasher(&self) -> Hasher {
        match self {
            Self::Sha256(_) => Hasher::Sha256(Sha256::new()),
        }
    }

    fn checksum(&self) -> &str {
        match self {
            Self::Sha256(checksum) => checksum,
        }
    }
}

enum Hasher {
    Sha256(Sha256),
}

impl Hasher {
    pub fn update(&mut self, data: &[u8]) {
        match self {
            Self::Sha256(hasher) => hasher.update(data),
        }
    }

    pub fn verify(self, checksum: &str) -> bool {
        match self {
            Self::Sha256(hasher) => {
                let digest = format!("{:x}", hasher.finalize());
                digest == *checksum
            }
        }
    }
}

// download a file with given url and size to a given path,
// with optional checksum verification.
//
// # Arguments
// * `client` - A reqwest client.
// * `url` - The url to download from.
// * `path` - The path to save the downloaded file.
// * `size` - The size of the file.
// * `checker` - The optional checksum checker.
pub async fn download<'a>(
    client: &Client,
    url: &str,
    path: &Path,
    size: u64,
    checker: Option<Checker<'a>>,
) -> Result<()> {
    let resp = client.get(url).send().await?;

    let progress_bar = ProgressBar::new(size);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress_bar.set_message("Downloading...");

    let mut stream = resp.bytes_stream();
    let mut file = File::create(path)?;

    if let Some(checker) = checker {
        let mut downloaded: u64 = 0;
        let mut hasher = checker.hasher();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            hasher.update(&chunk);
            downloaded = min(downloaded + chunk.len() as u64, size);
            progress_bar.set_position(downloaded);
        }

        progress_bar.finish_with_message("Downloaded, verifying checksum...");

        if hasher.verify(checker.checksum()) {
            println!("Checksum verified");
        } else {
            remove_file(path)?;
            return Err(Error::Verify);
        }
    } else {
        let mut downloaded: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded = min(downloaded + chunk.len() as u64, size);
            progress_bar.set_position(downloaded);
        }

        progress_bar.finish_with_message("Downloaded.");
    }

    Ok(())
}

/// Try to download a file within a timeout.
///
/// # Arguments
/// * `client` - A reqwest client.
/// * `url` - The url to download from.
/// * `timeout` - The timeout.
///
/// # Returns
/// The number of bytes downloaded.
async fn try_download(client: &Client, url: &str, timeout: Duration) -> Result<u64> {
    let resp = client.get(url).send().await?;

    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let start = Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        if start.elapsed() > timeout {
            return Ok(downloaded);
        }
    }

    Ok(downloaded)
}

/// Download from mutiple mirrors and choose the fastest one.
///
/// # Arguments
/// * `client` - A reqwest client.
/// * `fallback` - The fallback url.
/// * `mirrors` - The mirrors to choose from.
/// * `path` - The path to save the downloaded file.
/// * `size` - The size of the file.
/// * `checker` - The optional checksum checker.
///
/// *Note*: This function will skip the speed test if running in CI
/// to reduce the load of the mirror server.
pub async fn download_mirrors<'a>(
    client: &Client,
    fallback: &str,
    mirrors: Vec<String>,
    path: &Path,
    size: u64,
    checker: Option<Checker<'a>>,
) -> Result<()> {
    if std::env::var_os("CI").is_some() {
        println!("Running in CI, skipping speed test...");
        download(client, fallback, path, size, checker).await?;
        return Ok(());
    }

    let duration = Duration::from_secs(3);
    let mut fast_link = fallback;
    let mut largest: u64 = 0;

    println!("Speed test for mirrors...");
    for link in mirrors.iter() {
        let downloaded = try_download(client, link, duration).await?;
        if downloaded > largest {
            largest = downloaded;
            fast_link = link;
        }
    }

    println!("Downloading from fastest mirror...");
    download(client, fast_link, path, size, checker).await?;

    Ok(())
}
