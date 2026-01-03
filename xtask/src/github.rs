//! GitHub Actions utilities for interacting with workflow context and outputs.

use std::{
    fs::{self, File},
    io::Write,
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{env, release::Channel};

/// GitHub event types that trigger workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventName {
    /// Pull request event
    PullRequest,
    /// Scheduled event (cron)
    Schedule,
    /// Manually triggered workflow
    WorkflowDispatch,
    /// Push event (including tags)
    Push,
}

impl FromStr for EventName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pull_request" => Ok(EventName::PullRequest),
            "schedule" => Ok(EventName::Schedule),
            "workflow_dispatch" => Ok(EventName::WorkflowDispatch),
            "push" => Ok(EventName::Push),
            _ => bail!("Unknown GitHub event: {s}"),
        }
    }
}

impl EventName {
    /// Get the GitHub event name from environment.
    pub fn from_env() -> Result<Self> {
        let event_str = env::var("GITHUB_EVENT_NAME")?;
        event_str.parse()
    }
}

/// GitHub workflow event structure for workflow_dispatch events.
#[derive(Deserialize)]
pub struct WorkflowEvent {
    pub inputs: WorkflowInputs,
}

impl WorkflowEvent {
    /// Read and parse the workflow event from the file specified in GITHUB_EVENT_PATH.
    pub fn from_env() -> Result<Self> {
        let event_path = env::var("GITHUB_EVENT_PATH")?;

        let content = fs::read_to_string(&event_path)
            .with_context(|| format!("Failed to read event file: {event_path}"))?;

        serde_json::from_str(&content)
            .with_context(|| "Failed to parse workflow event JSON:\n{content}")
    }
}

/// Inputs for workflow_dispatch events.
#[derive(Deserialize)]
pub struct WorkflowInputs {
    pub channel: Channel,
    #[serde(deserialize_with = "deserialize_bool_from_string")]
    pub publish: bool,
}

/// Deserialize a boolean from a string.
fn deserialize_bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(serde::de::Error::custom("Invalid boolean string: {s}")),
    }
}

/// Get the GitHub ref (e.g., refs/tags/v1.0.0) from environment.
pub fn github_ref() -> String {
    env::var_or("GITHUB_REF", "")
}

fn open_github_output() -> Result<File> {
    let github_output = env::var("GITHUB_OUTPUT")?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&github_output)
        .with_context(|| format!("Failed to open {github_output}"))
}

fn set_output_to(file: &mut File, key: &str, value: &str) -> Result<()> {
    writeln!(file, "{key}={value}")?;
    Ok(())
}

/// Set a GitHub Actions output variable.
///
/// Writes to the file specified by GITHUB_OUTPUT environment variable.
pub fn set_output(key: &str, value: &str) -> Result<()> {
    let mut file = open_github_output()?;
    set_output_to(&mut file, key, value)
}

/// Set multiple GitHub Actions output variables at once.
pub fn set_outputs(outputs: &[(&str, &str)]) -> Result<()> {
    let mut file = open_github_output()?;
    for (key, value) in outputs {
        set_output_to(&mut file, key, value)?;
    }
    Ok(())
}
