//! GitHub Actions utilities for interacting with workflow context and outputs.

use std::{
    fs::{self, File},
    io::Write,
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};

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
pub struct WorkflowEvent {
    pub inputs: WorkflowInputs,
}

impl<'de> Deserialize<'de> for WorkflowEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Inputs,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`inputs`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "inputs" => Ok(Field::Inputs),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct WorkflowEventVisitor;

        impl<'de> Visitor<'de> for WorkflowEventVisitor {
            type Value = WorkflowEvent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct WorkflowEvent")
            }

            fn visit_map<V>(self, mut map: V) -> Result<WorkflowEvent, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut inputs = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Inputs => {
                            if inputs.is_some() {
                                return Err(de::Error::duplicate_field("inputs"));
                            }
                            inputs = Some(map.next_value()?);
                        }
                    }
                }

                let inputs = inputs.ok_or_else(|| de::Error::missing_field("inputs"))?;
                Ok(WorkflowEvent { inputs })
            }
        }

        const FIELDS: &[&str] = &["inputs"];
        deserializer.deserialize_struct("WorkflowEvent", FIELDS, WorkflowEventVisitor)
    }
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
pub struct WorkflowInputs {
    pub channel: Channel,
    pub publish: bool,
}

impl<'de> Deserialize<'de> for WorkflowInputs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Channel,
            Publish,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`channel` or `publish`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "channel" => Ok(Field::Channel),
                            "publish" => Ok(Field::Publish),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct WorkflowInputsVisitor;

        impl<'de> Visitor<'de> for WorkflowInputsVisitor {
            type Value = WorkflowInputs;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct WorkflowInputs")
            }

            fn visit_map<V>(self, mut map: V) -> Result<WorkflowInputs, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut channel = None;
                let mut publish = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Channel => {
                            if channel.is_some() {
                                return Err(de::Error::duplicate_field("channel"));
                            }
                            let s: String = map.next_value()?;
                            channel = Some(s.parse().map_err(de::Error::custom)?);
                        }
                        Field::Publish => {
                            if publish.is_some() {
                                return Err(de::Error::duplicate_field("publish"));
                            }
                            let s: String = map.next_value()?;
                            publish = Some(match s.as_str() {
                                "true" => true,
                                "false" => false,
                                _ => {
                                    return Err(de::Error::custom(format!(
                                        "Invalid boolean string: {s}"
                                    )));
                                }
                            });
                        }
                    }
                }

                let channel = channel.ok_or_else(|| de::Error::missing_field("channel"))?;
                let publish = publish.ok_or_else(|| de::Error::missing_field("publish"))?;
                Ok(WorkflowInputs { channel, publish })
            }
        }

        const FIELDS: &[&str] = &["channel", "publish"];
        deserializer.deserialize_struct("WorkflowInputs", FIELDS, WorkflowInputsVisitor)
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
