use std::sync::Arc;

use maa_core::Assistant;
use maa_types::InstanceOptionKey;

use crate::{
    Error, ErrorKind, Result,
    callback::{CallbackRuntime, MaaCallback, SessionRuntimeState},
    config::{ConnectionConfig, ExecutionOptions, InstanceOptions, MaaTask, Session},
    events::{EventSender, ExecutionEvent, StopToken},
    report::{SessionFailure, SessionReport, SessionStatus},
};

pub(crate) struct RunnerConfig {
    pub execution: ExecutionOptions,
    pub options: InstanceOptions,
    pub connection: ConnectionConfig,
    pub address: String,
    pub stop: StopToken,
    pub events: EventSender,
}

pub(crate) struct SessionContext {
    pub session_index: usize,
    pub session: Session,
}

pub(crate) trait SessionRunner {
    fn run_session(&mut self, ctx: SessionContext) -> SessionReport;
}

pub(crate) struct MaaSessionRunner {
    asst: Assistant,
    callback: Arc<CallbackRuntime>,
    stop: StopToken,
}

impl MaaSessionRunner {
    pub(crate) fn connect(config: RunnerConfig) -> Result<Self> {
        let callback = Arc::new(CallbackRuntime::new(
            config.execution.auto_reconnect,
            config.events.clone(),
        ));
        let asst = Assistant::new_with_callback(MaaCallback::new(Arc::clone(&callback))).map_err(
            |source| {
                Error::new(ErrorKind::Instance)
                    .with_source(source)
                    .with_desc("failed to create Assistant")
            },
        )?;

        apply_instance_options(config.options, &asst)?;

        config.events.send(ExecutionEvent::Connecting {
            address: config.address.clone(),
        });
        asst.async_connect(
            config.connection.adb_path.as_path(),
            config.address.as_str(),
            config.connection.config.as_str(),
            true,
        )
        .map_err(|source| {
            Error::new(ErrorKind::Connection)
                .with_source(source)
                .with_desc(format!("failed to connect to {}", config.address))
        })?;
        if !asst.connected() {
            return Err(Error::new(ErrorKind::Connection)
                .with_desc(format!("failed to connect to {}", config.address)));
        }
        config.events.send(ExecutionEvent::Connected);

        Ok(Self {
            asst,
            callback,
            stop: config.stop,
        })
    }
}

impl SessionRunner for MaaSessionRunner {
    fn run_session(&mut self, ctx: SessionContext) -> SessionReport {
        let name = ctx.session.name;
        if self.stop.is_requested() {
            return SessionReport {
                name,
                status: SessionStatus::Interrupted,
                failure: None,
            };
        }

        let state = self.callback.create_session(ctx.session_index);
        let result = PreparedSession::new(&self.asst, &self.callback, Arc::clone(&state))
            .execute(&ctx.session.tasks, &self.stop);

        let (status, failure) = match result {
            Err(error) => (
                SessionStatus::Failed,
                Some(SessionFailure::from_error(error)),
            ),
            Ok(SessionEnd::Interrupted) => (SessionStatus::Interrupted, None),
            Ok(SessionEnd::Offline) => (
                SessionStatus::Failed,
                Some(SessionFailure::new(
                    ErrorKind::Connection,
                    "lost connection to the game server",
                )),
            ),
            Ok(SessionEnd::Completed) if state.core_errored() => (
                SessionStatus::Failed,
                Some(SessionFailure::new(
                    ErrorKind::Task,
                    "MaaCore reported a task-chain error",
                )),
            ),
            Ok(SessionEnd::Completed) => (SessionStatus::Succeeded, None),
        };

        SessionReport {
            name,
            status,
            failure,
        }
    }
}

enum SessionEnd {
    Completed,
    Interrupted,
    Offline,
}

struct PreparedSession<'a> {
    asst: &'a Assistant,
    callback: &'a CallbackRuntime,
    state: Arc<SessionRuntimeState>,
    closed: bool,
}

impl<'a> PreparedSession<'a> {
    fn new(
        asst: &'a Assistant,
        callback: &'a CallbackRuntime,
        state: Arc<SessionRuntimeState>,
    ) -> Self {
        Self {
            asst,
            callback,
            state,
            closed: false,
        }
    }

    fn execute(mut self, tasks: &[MaaTask], stop: &StopToken) -> Result<SessionEnd> {
        for task in tasks {
            let task_name = task.name_or_default();
            let params = serde_json::to_string_pretty(&task.params).map_err(|source| {
                Error::new(ErrorKind::Task)
                    .with_source(source)
                    .with_desc(format!("failed to serialize task params for {task_name}"))
            })?;
            let task_id = self
                .asst
                .append_task(task.task_type, params.as_str())
                .map_err(|source| {
                    Error::new(ErrorKind::Task)
                        .with_source(source)
                        .with_desc(format!("failed to append task {task_name}"))
                })?;
            self.callback.register_task(task_id, &self.state);
        }

        if tasks.is_empty() {
            self.close()?;
            return Ok(SessionEnd::Completed);
        }
        if stop.is_requested() {
            self.close()?;
            return Ok(SessionEnd::Interrupted);
        }

        self.asst.start().map_err(|source| {
            Error::new(ErrorKind::Instance)
                .with_source(source)
                .with_desc("failed to start assistant")
        })?;

        let end = loop {
            if stop.is_requested() {
                break SessionEnd::Interrupted;
            }
            if self.state.offline_stop_requested() {
                break SessionEnd::Offline;
            }
            if !self.asst.running() {
                break SessionEnd::Completed;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        };

        if matches!(end, SessionEnd::Completed) {
            self.state.wait_for_terminal_callback();
        }
        self.close()?;
        Ok(end)
    }

    fn close(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.asst.stop().map_err(|source| {
            Error::new(ErrorKind::Instance)
                .with_source(source)
                .with_desc("failed to stop assistant")
        })
    }
}

impl Drop for PreparedSession<'_> {
    fn drop(&mut self) {
        if !self.closed
            && let Err(error) = self.asst.stop()
        {
            log::error!("Failed to stop Assistant while cleaning up session: {error}");
        }
    }
}

fn apply_instance_options(options: InstanceOptions, asst: &Assistant) -> Result<()> {
    if let Some(touch_mode) = options.touch_mode {
        asst.set_instance_option(InstanceOptionKey::TouchMode, touch_mode)
            .map_err(|source| {
                Error::new(ErrorKind::Instance)
                    .with_source(source)
                    .with_desc("failed to set instance option TouchMode")
            })?;
    }
    if let Some(deployment_with_pause) = options.deployment_with_pause {
        asst.set_instance_option(
            InstanceOptionKey::DeploymentWithPause,
            deployment_with_pause,
        )
        .map_err(|source| {
            Error::new(ErrorKind::Instance)
                .with_source(source)
                .with_desc("failed to set instance option DeploymentWithPause")
        })?;
    }
    if let Some(adb_lite_enabled) = options.adb_lite_enabled {
        asst.set_instance_option(InstanceOptionKey::AdbLiteEnabled, adb_lite_enabled)
            .map_err(|source| {
                Error::new(ErrorKind::Instance)
                    .with_source(source)
                    .with_desc("failed to set instance option AdbLiteEnabled")
            })?;
    }
    if let Some(kill_adb_on_exit) = options.kill_adb_on_exit {
        asst.set_instance_option(InstanceOptionKey::KillAdbOnExit, kill_adb_on_exit)
            .map_err(|source| {
                Error::new(ErrorKind::Instance)
                    .with_source(source)
                    .with_desc("failed to set instance option KillAdbOnExit")
            })?;
    }

    Ok(())
}
