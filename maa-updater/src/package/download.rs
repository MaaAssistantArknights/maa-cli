use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::time::{timeout, Duration};

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Io(std::io::Error),
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
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

async fn download(client: &Client, url: &str, path: &Path, size: u64) -> Result<()> {
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
    let duration = Duration::from_secs(5);
    let mut fast_link = url;
    let mut largest: u64 = 0;

    println!("Speed test for mirrors...");
    for link in mirrors.iter() {
        let resp = timeout(duration, client.head(link).send()).await;

        if let Ok(Ok(resp)) = resp {
            let content_length = resp.content_length().unwrap_or(0);
            if content_length > largest {
                largest = content_length;
                fast_link = link;
            }
        }
    }

    println!("Downloading from fastest mirror...");
    download(client, fast_link, path, size).await?;

    Ok(())
}
