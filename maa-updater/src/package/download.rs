use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Failed,
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
            Error::Failed => write!(f, "Download failed"),
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

pub trait ResultExt {
    fn is_timeout(self) -> Result<bool>;
}

impl ResultExt for Result<()> {
    fn is_timeout(self) -> Result<bool> {
        match self {
            Ok(_) => Ok(false),
            Err(Error::Reqwest(e)) => {
                if e.is_timeout() {
                    Ok(true)
                } else {
                    Err(Error::Reqwest(e))
                }
            }
            Err(e) => Err(e),
        }
    }
}

pub async fn download(client: &Client, url: &str, path: &Path, size: u64) -> Result<()> {
    let resp = client.get(url).send().await?;

    let progress_bar = ProgressBar::new(size);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut stream = resp.bytes_stream();
    let mut file = File::create(path)?;
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded = min(downloaded + chunk.len() as u64, size);
        progress_bar.set_position(downloaded);
    }

    Ok(())
}

pub async fn download_package(
    client: &Client,
    url: &str,
    mirrors: Vec<String>,
    path: &Path,
    size: u64,
) -> Result<()> {
    println!("Trying download from origin");
    if download(client, url, path, size).await.is_timeout()? {
        print!("Timeout, trying download from mirror!")
    } else {
        println!("Download succeed!");
        return Ok(());
    }

    for mirror in mirrors {
        if download(client, &mirror, path, size).await.is_timeout()? {
            print!("Timeout, try another mirror!")
        } else {
            println!("Download succeed!");
            return Ok(());
        }
    }

    Err(Error::Failed)
}
