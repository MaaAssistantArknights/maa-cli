use std::{sync::Arc, thread};

use log::debug;

use crate::{
    Error, ErrorKind, Result,
    config::{ConnectionConfig, ExecutionOptions, ExecutorConfig, InstanceOptions, Session},
    driver::CoreDriver,
    environment::{ExecutionEnvironment, PreparedEnvironment},
    events::{CompletionState, EventSender, ExecutionEvent, ExecutionHandle, StopToken, channel},
    report::{ExecutionReport, ExecutionStatus, SessionStatus},
    session::{RunnerConfig, SessionContext, SessionRunner},
};

/// Owns the loaded MaaCore runtime and creates execution instances.
///
/// The runtime remains loaded while any instance or background execution retains the Executor's
/// internal driver.
pub struct Executor {
    driver: Arc<dyn CoreDriver>,
}

impl Executor {
    /// Loads MaaCore, applies static options, and loads all configured resources.
    pub fn build(config: ExecutorConfig) -> Result<Self> {
        let driver = Arc::new(super::driver::MaaCoreDriver::build(&config)?);
        Ok(Self { driver })
    }

    /// Creates an independently configurable execution instance.
    pub fn create_instance(&self, options: InstanceOptions) -> ExecutorInstance {
        ExecutorInstance::new(Arc::clone(&self.driver), options)
    }

    #[cfg(test)]
    pub(crate) fn with_driver(driver: Arc<dyn CoreDriver>) -> Self {
        Self { driver }
    }
}

/// A configured source of executions sharing one MaaCore runtime.
pub struct ExecutorInstance {
    driver: Arc<dyn CoreDriver>,
    options: InstanceOptions,
}

impl ExecutorInstance {
    pub(crate) fn new(driver: Arc<dyn CoreDriver>, options: InstanceOptions) -> Self {
        Self { driver, options }
    }

    /// Starts an execution on a dedicated thread and immediately returns its handle.
    ///
    /// Connection is performed once and the resulting Assistant is reused for every session.
    /// Session failures are recorded and execution continues; a requested stop interrupts the
    /// complete execution.
    pub fn execute<E: ExecutionEnvironment + Send + 'static>(
        self,
        mut environment: E,
        connection: ConnectionConfig,
        execution: ExecutionOptions,
        sessions: Vec<Session>,
        stop: StopToken,
    ) -> Result<ExecutionHandle> {
        let (events, subscription) = channel();
        let completion = CompletionState::new();
        let handle = ExecutionHandle::new(Arc::clone(&completion), subscription);
        let driver = Arc::clone(&self.driver);
        let options = self.options;

        thread::Builder::new()
            .name(String::from("maa-executor"))
            .spawn(move || {
                let mut ctx = RunContext {
                    driver: &*driver,
                    connection,
                    execution,
                    options,
                    stop: &stop,
                    events: &events,
                };
                let result = run_execution(&mut ctx, &mut environment, sessions);
                match &result {
                    Ok(report) => events.send(ExecutionEvent::Finished {
                        status: report.status,
                    }),
                    Err(error) => events.send(ExecutionEvent::Failed {
                        kind: error.kind(),
                        message: error.to_string(),
                    }),
                }
                completion.complete(result);
            })
            .map_err(|source| {
                Error::new(ErrorKind::Execution)
                    .with_source(source)
                    .with_desc("failed to spawn executor thread")
            })?;

        Ok(handle)
    }
}

/// Shared runtime context for the session loop.
struct RunContext<'a> {
    driver: &'a dyn CoreDriver,
    connection: ConnectionConfig,
    execution: ExecutionOptions,
    options: InstanceOptions,
    stop: &'a StopToken,
    events: &'a EventSender,
}

fn run_execution<E: ExecutionEnvironment>(
    ctx: &mut RunContext<'_>,
    environment: &mut E,
    sessions: Vec<Session>,
) -> Result<ExecutionReport> {
    ctx.events.send(ExecutionEvent::EnvironmentPreparing);
    let runtime_address = environment.prepare().map_err(|source| {
        Error::new(ErrorKind::Environment)
            .with_source(source)
            .with_desc("failed to prepare execution environment")
    })?;
    ctx.events.send(ExecutionEvent::EnvironmentPrepared {
        address: runtime_address.clone(),
    });

    let prepared = PreparedEnvironment::new(environment);
    let run_result = runtime_address
        .or(ctx.connection.address.clone())
        .ok_or_else(|| Error::new(ErrorKind::Connection).with_desc("missing connection address"))
        .and_then(|address| run_sessions(ctx, &address, sessions));
    let close_result = prepared.close();

    match (run_result, close_result) {
        (result, Ok(())) => result,
        (Ok(_), Err(close_error)) => Err(close_error),
        (Err(run_error), Err(close_error)) => {
            debug!("Failed to close environment after execution error: {close_error}");
            Err(run_error)
        }
    }
}

fn run_sessions(
    ctx: &RunContext<'_>,
    address: &str,
    sessions: Vec<Session>,
) -> Result<ExecutionReport> {
    if ctx.stop.is_requested() {
        ctx.events.send(ExecutionEvent::Interrupted);
        let status = ExecutionStatus::Interrupted;
        return Ok(ExecutionReport {
            status,
            sessions: Vec::new(),
        });
    }

    let mut reports = Vec::with_capacity(sessions.len());
    let mut runner = connect_runner(ctx, address)?;
    let mut interrupted = false;

    for (index, session) in sessions.into_iter().enumerate() {
        if ctx.stop.is_requested() {
            ctx.events.send(ExecutionEvent::Interrupted);
            interrupted = true;
            break;
        }

        ctx.events.send(ExecutionEvent::SessionStarted {
            index,
            name: session.name.clone(),
        });

        let report = runner.run_session(SessionContext {
            session_index: index,
            session,
        });

        let session_interrupted = matches!(report.status, SessionStatus::Interrupted);
        ctx.events.send(ExecutionEvent::SessionFinished {
            index,
            report: report.clone(),
        });
        reports.push(report);

        if session_interrupted {
            ctx.events.send(ExecutionEvent::Interrupted);
            interrupted = true;
            break;
        }
    }

    let status = if interrupted {
        ExecutionStatus::Interrupted
    } else if reports
        .iter()
        .any(|report| matches!(report.status, SessionStatus::Failed))
    {
        ExecutionStatus::Failed
    } else {
        ExecutionStatus::Succeeded
    };

    Ok(ExecutionReport {
        status,
        sessions: reports,
    })
}

fn connect_runner(ctx: &RunContext<'_>, address: &str) -> Result<Box<dyn SessionRunner>> {
    ctx.driver.connect(RunnerConfig {
        execution: ctx.execution,
        options: ctx.options,
        connection: ctx.connection.clone(),
        address: String::from(address),
        stop: ctx.stop.clone(),
        events: ctx.events.clone(),
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::{Arc, Mutex};

    use maa_value::prelude::*;

    use super::*;
    use crate::{
        ConnectionConfig, ExecutionEnvironment, ExecutionEvent, ExecutionOptions, ExecutionStatus,
        MaaTask, Session, SessionStatus, StopToken,
        testutil::{FakeDriver, FakeEnvironment, block_on, report},
    };

    #[test]
    fn execute_uses_dynamic_address_before_static_fallback() {
        let driver = FakeDriver::with_reports(vec![report(SessionStatus::Succeeded, "main")]);
        let executor = Executor::with_driver(driver.clone());
        let instance = executor.create_instance(InstanceOptions::default());
        let order = Arc::new(Mutex::new(Vec::new()));
        let handle = instance
            .execute(
                FakeEnvironment {
                    prepare_address: Some(String::from("dynamic")),
                    order: order.clone(),
                },
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: Some(String::from("static")),
                    config: String::from("General"),
                },
                ExecutionOptions::default(),
                vec![Session {
                    name: Some(String::from("main")),
                    tasks: vec![MaaTask {
                        name: Some(String::from("fight")),
                        task_type: maa_types::TaskType::Fight,
                        params: object!(),
                    }],
                }],
                StopToken::default(),
            )
            .unwrap();

        let report = block_on(handle).unwrap();
        assert_eq!(report.status, ExecutionStatus::Succeeded);
        assert_eq!(driver.order.lock().unwrap().as_slice(), [
            String::from("connect:dynamic:true"),
            String::from("session:0")
        ]);
        assert_eq!(order.lock().unwrap().as_slice(), [
            String::from("prepare"),
            String::from("close")
        ]);
    }

    #[test]
    fn execute_falls_back_to_static_address() {
        let driver = FakeDriver::with_reports(vec![report(SessionStatus::Succeeded, "main")]);
        let executor = Executor::with_driver(driver.clone());
        let instance = executor.create_instance(InstanceOptions::default());
        let handle = instance
            .execute(
                FakeEnvironment {
                    prepare_address: None,
                    order: Arc::new(Mutex::new(Vec::new())),
                },
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: Some(String::from("static")),
                    config: String::from("General"),
                },
                ExecutionOptions::default(),
                vec![Session {
                    name: Some(String::from("main")),
                    tasks: Vec::new(),
                }],
                StopToken::default(),
            )
            .unwrap();

        block_on(handle).unwrap();
        assert_eq!(driver.order.lock().unwrap().as_slice(), [
            String::from("connect:static:true"),
            String::from("session:0")
        ]);
    }

    #[test]
    fn execute_errors_when_address_is_missing() {
        let driver = FakeDriver::with_reports(Vec::new());
        let executor = Executor::with_driver(driver);
        let instance = executor.create_instance(InstanceOptions::default());
        let order = Arc::new(Mutex::new(Vec::new()));
        let handle = instance
            .execute(
                FakeEnvironment {
                    prepare_address: None,
                    order: Arc::clone(&order),
                },
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: None,
                    config: String::from("General"),
                },
                ExecutionOptions::default(),
                Vec::new(),
                StopToken::default(),
            )
            .unwrap();

        assert_eq!(
            block_on(handle).unwrap_err().to_string(),
            "connection failed: missing connection address"
        );
        assert_eq!(order.lock().unwrap().as_slice(), [
            String::from("prepare"),
            String::from("close")
        ]);
    }

    #[test]
    fn execute_emits_events_and_final_report() {
        let driver = FakeDriver::with_reports(vec![
            report(SessionStatus::Failed, "main"),
            report(SessionStatus::Succeeded, "alt"),
        ]);
        let executor = Executor::with_driver(driver.clone());
        let instance = executor.create_instance(InstanceOptions::default());
        let handle = instance
            .execute(
                FakeEnvironment {
                    prepare_address: Some(String::from("dynamic")),
                    order: Arc::new(Mutex::new(Vec::new())),
                },
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: None,
                    config: String::from("General"),
                },
                ExecutionOptions::default(),
                vec![
                    Session {
                        name: Some(String::from("main")),
                        tasks: Vec::new(),
                    },
                    Session {
                        name: Some(String::from("alt")),
                        tasks: Vec::new(),
                    },
                ],
                StopToken::default(),
            )
            .unwrap();
        let events: Vec<_> = handle.events().iter().collect();
        let report = block_on(handle).unwrap();

        assert_eq!(report.status, ExecutionStatus::Failed);
        assert_eq!(driver.order.lock().unwrap().as_slice(), [
            String::from("connect:dynamic:true"),
            String::from("session:0"),
            String::from("session:1"),
        ]);

        assert!(matches!(events[0], ExecutionEvent::EnvironmentPreparing));
        assert!(matches!(
            events.last(),
            Some(ExecutionEvent::Finished {
                status: ExecutionStatus::Failed
            })
        ));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutionEvent::SessionFinished { index: 0, .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutionEvent::SessionFinished { index: 1, .. }))
        );
    }

    #[test]
    fn stop_interrupts_execution() {
        let driver = FakeDriver::with_reports(vec![report(SessionStatus::Succeeded, "main")]);
        let executor = Executor::with_driver(driver);
        let instance = executor.create_instance(InstanceOptions::default());
        let stop = StopToken::default();
        stop.request_stop();
        let handle = instance
            .execute(
                FakeEnvironment {
                    prepare_address: Some(String::from("dynamic")),
                    order: Arc::new(Mutex::new(Vec::new())),
                },
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: None,
                    config: String::from("General"),
                },
                ExecutionOptions::default(),
                vec![Session {
                    name: Some(String::from("main")),
                    tasks: Vec::new(),
                }],
                stop,
            )
            .unwrap();

        let report = block_on(handle).unwrap();
        assert_eq!(report.status, ExecutionStatus::Interrupted);
    }

    #[test]
    fn execute_passes_execution_options_to_session_runner() {
        let driver = FakeDriver::with_reports(vec![report(SessionStatus::Succeeded, "main")]);
        let executor = Executor::with_driver(driver.clone());
        let instance = executor.create_instance(InstanceOptions::default());
        let handle = instance
            .execute(
                FakeEnvironment {
                    prepare_address: Some(String::from("dynamic")),
                    order: Arc::new(Mutex::new(Vec::new())),
                },
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: None,
                    config: String::from("General"),
                },
                ExecutionOptions {
                    auto_reconnect: false,
                },
                vec![Session {
                    name: Some(String::from("main")),
                    tasks: Vec::new(),
                }],
                StopToken::default(),
            )
            .unwrap();

        block_on(handle).unwrap();
        assert_eq!(driver.order.lock().unwrap().as_slice(), [
            String::from("connect:dynamic:false"),
            String::from("session:0")
        ]);
    }

    #[test]
    fn execute_emits_failed_event_for_top_level_error() {
        #[derive(Default)]
        struct FailingEnvironment;

        impl ExecutionEnvironment for FailingEnvironment {
            type Error = std::io::Error;

            fn prepare(&mut self) -> std::result::Result<Option<String>, Self::Error> {
                Err(std::io::Error::other("prepare failed"))
            }

            fn close(&mut self) -> std::result::Result<(), Self::Error> {
                Ok(())
            }
        }

        let driver = FakeDriver::with_reports(Vec::new());
        let executor = Executor::with_driver(driver);
        let instance = executor.create_instance(InstanceOptions::default());
        let handle = instance
            .execute(
                FailingEnvironment,
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: Some(String::from("static")),
                    config: String::from("General"),
                },
                ExecutionOptions::default(),
                Vec::new(),
                StopToken::default(),
            )
            .unwrap();
        let events: Vec<_> = handle.events().iter().collect();

        let error = block_on(handle).unwrap_err();
        assert_eq!(
            error.to_string(),
            "execution environment failed: failed to prepare execution environment"
        );

        assert!(matches!(events[0], ExecutionEvent::EnvironmentPreparing));
        assert!(matches!(
            events.last(),
            Some(ExecutionEvent::Failed {
                kind: ErrorKind::Environment,
                message,
            }) if message.contains("failed to prepare execution environment")
        ));
    }

    #[test]
    fn close_failure_replaces_success_with_one_failed_terminal_event() {
        #[derive(Default)]
        struct FailingCloseEnvironment;

        impl ExecutionEnvironment for FailingCloseEnvironment {
            type Error = std::io::Error;

            fn prepare(&mut self) -> std::result::Result<Option<String>, Self::Error> {
                Ok(Some(String::from("dynamic")))
            }

            fn close(&mut self) -> std::result::Result<(), Self::Error> {
                Err(std::io::Error::other("close failed"))
            }
        }

        let driver = FakeDriver::with_reports(Vec::new());
        let executor = Executor::with_driver(driver);
        let instance = executor.create_instance(InstanceOptions::default());
        let stop = StopToken::default();
        stop.request_stop();
        let handle = instance
            .execute(
                FailingCloseEnvironment,
                ConnectionConfig {
                    adb_path: "adb".into(),
                    address: None,
                    config: String::from("General"),
                },
                ExecutionOptions::default(),
                Vec::new(),
                stop,
            )
            .unwrap();
        let events: Vec<_> = handle.events().iter().collect();

        let error = block_on(handle).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Environment);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    ExecutionEvent::Failed { .. } | ExecutionEvent::Finished { .. }
                ))
                .count(),
            1
        );
        assert!(matches!(
            events.last(),
            Some(ExecutionEvent::Failed {
                kind: ErrorKind::Environment,
                ..
            })
        ));
    }
}
