use std::{
    ffi::OsString,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

use indicatif::ProgressBar;

use crate::{
    error::{Result, WithDesc},
    verify::Verifier,
};

/// Get the partial file path for a destination file
fn partial_path(dest: &Path) -> PathBuf {
    let mut dest_partial: OsString = dest.into();
    dest_partial.push(".partial");
    PathBuf::from(dest_partial)
}

pub fn download(
    client: &ureq::Agent,
    url: &str,
    dest: &Path,
    ui: ProgressBar,
    mut verifier: impl Verifier,
) -> Result<()> {
    // Use a partial file for the download
    let partial_path = partial_path(dest);

    // Check if we have a partial file and its size
    let partial_file_exists = partial_path.exists();
    let mut resume_from = 0;

    // Try to resume if a partial file exists
    if partial_file_exists {
        resume_from = std::fs::metadata(&partial_path)
            .with_desc("Failed to get metadata of partial file")?
            .len();
    }

    // Make the GET request with appropriate headers
    let mut request = client.get(url);
    if resume_from > 0 {
        request = request.header("Range", format!("bytes={}-", resume_from));
    }

    let mut resp = request
        .call()
        .with_desc("Failed to send download request")?;

    // Check if server supports range requests when resuming
    if resume_from > 0 && resp.status() != 206 {
        // Server doesn't support range requests, start from the beginning
        drop(resp);
        std::fs::remove_file(&partial_path).with_desc("Failed to remove partial file")?;

        // Restart download from the beginning
        resume_from = 0;
        resp = client
            .get(url)
            .call()
            .with_desc("Failed to send download request")?;
    }

    // Now we know if we're resuming or starting fresh, open/create the file accordingly
    let mut file = if resume_from > 0 {
        // Resuming: open existing file and update verifier with partial data
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&partial_path)
            .with_desc("Failed to open partial file")?;

        // Read the partial file to update verifier state
        file.seek(std::io::SeekFrom::Start(0))
            .with_desc("Failed to seek to start of partial file")?;
        verifier
            .update_reader(&mut file)
            .with_desc("Failed to update verifier with partial data")?;

        // Seek to end for appending new data
        file.seek(std::io::SeekFrom::End(0))
            .with_desc("Failed to seek to end of partial file")?;

        file
    } else {
        // Starting fresh: create new file
        std::fs::File::create(&partial_path).with_desc("Failed to create new file")?
    };

    // Get content length from response headers
    let content_length = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    // Initialize progress tracking
    if let Some(total) = content_length.map(|len| len + resume_from) {
        ui.set_length(total);
    }
    ui.set_position(resume_from);

    // Download the content
    let mut buffer = [0; 8192];
    let mut reader = resp.body_mut().as_reader();
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let chunk = &buffer[..bytes_read];
        file.write_all(chunk)
            .with_desc("Failed to write data to file")?;
        ui.inc(bytes_read as u64);
        verifier.update(chunk);
    }

    // Make sure all data is written to disk
    file.flush()?;
    drop(file);

    // Verify the downloaded file
    if let Err(e) = verifier.verify() {
        std::fs::remove_file(&partial_path).with_desc("Failed to remove partial file")?;
        return Err(e);
    }

    // Rename the partial file to the destination
    std::fs::rename(partial_path, dest).with_desc("Failed to rename partial file")?;

    Ok(())
}
