use std::{
    path::Path,
    sync::mpsc::{self, Receiver},
};

use anyhow::{Context, Result, ensure};
use maa_core::Callback;
use maa_types::{MessageKind, primitive::AsstAsyncCallId};
use serde::Deserialize;

use super::{
    ConnectionArgs, callback::MaaCallback, create_assistant, load_config, with_connection,
};
use crate::config::task::ClientType;

pub fn screenshot(output: &Path, args: ConnectionArgs) -> Result<()> {
    let config = load_config(&args)?;
    let (callback, results) = screenshot_callback();
    let asst = create_assistant(&config, callback)?;

    with_connection(&asst, &config, ClientType::default(), false, false, || {
        ensure!(asst.connected(), "Failed to connect to device");
        let id = asst
            .async_screencap(true)
            .context("Failed to take screenshot")?;
        wait_for_screencap(&results, id)?;
        let image = asst.get_image().context("Failed to get screenshot")?;
        save_image(output, image.as_deref())
    })
}

#[derive(Deserialize)]
struct AsyncCallInfo {
    async_call_id: AsstAsyncCallId,
    details: AsyncCallDetails,
}

#[derive(Deserialize)]
struct AsyncCallDetails {
    ret: bool,
}

fn screenshot_callback() -> (impl Callback, Receiver<Result<AsyncCallInfo>>) {
    let (callback, _) = MaaCallback::new(false);
    let (sender, receiver) = mpsc::channel();
    let callback = move |kind: MessageKind, msg: Option<&str>| {
        callback.on_message(kind, msg);
        if kind == MessageKind::AsyncCallInfo {
            let result = msg
                .context("Missing async call result")
                .and_then(|msg| serde_json::from_str(msg).context("Invalid async call result"));
            let _ = sender.send(result);
        }
    };
    (callback, receiver)
}

fn wait_for_screencap(
    results: &Receiver<Result<AsyncCallInfo>>,
    id: AsstAsyncCallId,
) -> Result<()> {
    // The blocking API can return before its completion callback is delivered.
    loop {
        let result = results
            .recv()
            .context("Screenshot callback disconnected")??;
        if result.async_call_id == id {
            ensure!(result.details.ret, "Failed to take screenshot");
            return Ok(());
        }
    }
}

fn save_image(output: &Path, image: Option<&[u8]>) -> Result<()> {
    let image = image.context("No screenshot returned by MaaCore")?;
    maa_atomic_fs::write(output, image)
        .with_context(|| format!("Failed to save screenshot to {}", output.display()))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn waits_for_matching_result() -> Result<()> {
        let (callback, results) = screenshot_callback();
        callback.on_message(
            MessageKind::AsyncCallInfo,
            Some(r#"{"what":"Connect","async_call_id":1,"details":{"ret":false}}"#),
        );
        callback.on_message(
            MessageKind::AsyncCallInfo,
            Some(r#"{"what":"Screencap","async_call_id":2,"details":{"ret":true}}"#),
        );
        wait_for_screencap(&results, 2)
    }

    #[test]
    fn rejects_failed_screencap() {
        let (callback, results) = screenshot_callback();
        callback.on_message(
            MessageKind::AsyncCallInfo,
            Some(r#"{"what":"Screencap","async_call_id":2,"details":{"ret":false}}"#),
        );

        assert!(wait_for_screencap(&results, 2).is_err());
    }

    #[test]
    fn rejects_invalid_results() {
        for msg in [None, Some("invalid json")] {
            let (callback, results) = screenshot_callback();
            callback.on_message(MessageKind::AsyncCallInfo, msg);
            assert!(wait_for_screencap(&results, 2).is_err());
        }
    }

    #[test]
    fn missing_image_preserves_output() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let output = dir.path().join("screen.png");

        fs::write(&output, "previous image")?;
        assert!(save_image(&output, None).is_err());
        assert_eq!(fs::read_to_string(output)?, "previous image");
        Ok(())
    }
}
