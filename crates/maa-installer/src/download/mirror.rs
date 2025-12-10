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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_or_time_gt() {
        // Time comparison: Smaller time is better (greater)
        let fast = BytesOrTime::Time(Duration::from_secs(1));
        let slow = BytesOrTime::Time(Duration::from_secs(5));
        assert!(fast.gt(slow));
        assert!(!slow.gt(fast));

        // Bytes comparison: More bytes is better (greater)
        let more = BytesOrTime::Bytes(1000);
        let less = BytesOrTime::Bytes(500);
        assert!(more.gt(less));
        assert!(!less.gt(more));

        // Mixed comparison: Completed download (Time) is always better than incomplete (Bytes)
        let completed_slow = BytesOrTime::Time(Duration::from_secs(100));
        let incomplete_fast = BytesOrTime::Bytes(999999);
        assert!(completed_slow.gt(incomplete_fast));
        assert!(!incomplete_fast.gt(completed_slow));

        // Equal values should return false for gt
        let time1 = BytesOrTime::Time(Duration::from_secs(5));
        let time2 = BytesOrTime::Time(Duration::from_secs(5));
        assert!(!time1.gt(time2));

        let bytes1 = BytesOrTime::Bytes(1000);
        let bytes2 = BytesOrTime::Bytes(1000);
        assert!(!bytes1.gt(bytes2));

        // Edge cases: Zero values
        let zero_time = BytesOrTime::Time(Duration::from_secs(0));
        let some_time = BytesOrTime::Time(Duration::from_secs(1));
        assert!(zero_time.gt(some_time));

        let zero_bytes = BytesOrTime::Bytes(0);
        let some_bytes = BytesOrTime::Bytes(1);
        assert!(some_bytes.gt(zero_bytes));

        // Edge case: Very small time is still better than any bytes
        let tiny_time = BytesOrTime::Time(Duration::from_millis(1));
        let large_bytes = BytesOrTime::Bytes(u64::MAX);
        assert!(tiny_time.gt(large_bytes));
    }
}
