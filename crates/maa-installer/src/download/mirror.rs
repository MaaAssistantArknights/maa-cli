//! Module for selecting the fastest mirror based on speedtest.

use std::{
    borrow::Cow,
    io::Read,
    time::{Duration, Instant},
};

use indicatif::ProgressBar;

use crate::{
    error::{Result, WithDesc},
    manifest::MirrorOptions,
};

#[derive(Clone, Copy, Debug)]
/// A struct to represent either the downloaded bytes or the time taken to download.
///
/// If the download is completed before the time limit, the value will be `Time`.
/// If the download is not completed before the time limit, the value will be `Bytes`.
///
/// When comparing two `BytesOrTime`, the `Time` is always greater than `Bytes`.
/// For two `Bytes`, the one with larger value is greater.
/// For two `Time`, the one with smaller value is greater.
enum BytesOrTime {
    Bytes(u64),
    Time(Duration),
}

impl BytesOrTime {
    /// # Note
    ///
    /// Make sure that those values are generated for the same `max_bytes` and `max_time`,
    /// otherwise the result might be incorrect.
    fn gt(self, other: Self) -> bool {
        match (self, other) {
            (BytesOrTime::Bytes(a), BytesOrTime::Bytes(b)) => a > b,
            (BytesOrTime::Time(a), BytesOrTime::Time(b)) => a < b,
            (BytesOrTime::Time(_), BytesOrTime::Bytes(_)) => true,
            (BytesOrTime::Bytes(_), BytesOrTime::Time(_)) => false,
        }
    }
}

fn speedtest(
    agent: &ureq::Agent,
    url: &str,
    max_bytes: u64,
    max_time: Duration,
) -> Result<BytesOrTime> {
    let start = Instant::now();
    let mut downloaded: u64 = 0;
    let mut buffer = vec![0; 8192];

    let mut resp = agent
        .get(url)
        .call()
        .with_desc("Failed to send download request")?;
    let mut reader = resp.body_mut().as_reader();
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            return Ok(BytesOrTime::Time(start.elapsed()));
        }
        downloaded += bytes_read as u64;
        if downloaded >= max_bytes {
            return Ok(BytesOrTime::Time(start.elapsed()));
        }
        if start.elapsed() >= max_time {
            return Ok(BytesOrTime::Bytes(downloaded));
        }
    }
}

pub fn fastest_mirror<'a, M: Iterator<Item = std::borrow::Cow<'a, str>>>(
    agent: &ureq::Agent,
    url: Cow<'a, str>,
    max_time: Duration,
    opts: MirrorOptions<'a, M>,
    ui: ProgressBar,
) -> Cow<'a, str> {
    let max_bytes = opts.max_bytes;

    ui.set_message(format!("Testing speed of {url}"));
    ui.tick();
    let mut fastest_speed =
        speedtest(agent, &url, max_bytes, max_time).unwrap_or(BytesOrTime::Bytes(0));
    let mut fastest_mirror = url;

    for url in opts.mirrors {
        ui.set_message(format!("Testing speed of {url}"));
        ui.tick();
        let speed = speedtest(agent, url.as_ref(), opts.max_bytes, max_time);
        // Do not return error if one mirror fails, just skip it
        match speed {
            Ok(speed) => {
                if speed.gt(fastest_speed) {
                    ui.set_message(format!("Found new fastest mirror: {url}"));
                    ui.tick();
                    fastest_mirror = url;
                    fastest_speed = speed;
                }
            }
            Err(err) => {
                ui.set_message(format!("Failed to test speed of {url}: {err}"));
                ui.tick();
            }
        }
    }

    fastest_mirror
}
