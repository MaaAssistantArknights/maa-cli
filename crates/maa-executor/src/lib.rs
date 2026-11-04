#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! Runtime execution layer for MaaCore.
//!
//! The crate accepts resolved executor inputs, runs sessions on a reusable Assistant instance,
//! forwards structured MaaCore callbacks, and returns an execution report. It does not parse CLI,
//! configuration, GUI, or RPC data.
//!
//! This is a design prototype. The CLI does not use it yet, and the public API and lifecycle
//! policies remain open for discussion. See `ARCHITECTURE.md` for the known limitations.
//!
//! # Minimal caller
//!
//! The caller resolves paths, connection settings, and task parameters before execution. This
//! example attaches to an already prepared environment; its environment hooks do not launch or
//! close a game. Tasks supplied by the caller can still do so.
//! Event iteration blocks the current thread; an async application must arrange a blocking context
//! for this part. After the stream closes, the handle yields the final report.
//!
//! ```no_run
//! use std::convert::Infallible;
//!
//! use maa_executor::{
//!     ConnectionConfig, ExecutionEnvironment, ExecutionOptions, ExecutionReport, Executor,
//!     ExecutorConfig, InstanceOptions, Result, Session, StopToken,
//! };
//!
//! struct ExistingEnvironment;
//!
//! impl ExecutionEnvironment for ExistingEnvironment {
//!     type Error = Infallible;
//!
//!     fn prepare(&mut self) -> std::result::Result<Option<String>, Self::Error> {
//!         Ok(None) // Use the address supplied in ConnectionConfig.
//!     }
//!
//!     fn close(&mut self) -> std::result::Result<(), Self::Error> {
//!         Ok(()) // The caller owns the existing environment.
//!     }
//! }
//!
//! async fn run(
//!     config: ExecutorConfig,
//!     connection: ConnectionConfig,
//!     sessions: Vec<Session>,
//!     stop: StopToken,
//! ) -> Result<ExecutionReport> {
//!     // The caller can retain a clone of `stop` and request cancellation externally.
//!     let executor = Executor::build(config)?;
//!     let handle = executor
//!         .create_instance(InstanceOptions::default())
//!         .execute(
//!             ExistingEnvironment,
//!             connection,
//!             ExecutionOptions::default(),
//!             sessions,
//!             stop,
//!         )?;
//!
//!     for event in handle.events().iter() {
//!         println!("{event:?}");
//!     }
//!
//!     let report = handle.wait().await?;
//!     report.result()?;
//!     Ok(report)
//! }
//! ```

mod callback;
mod config;
mod driver;
mod environment;
mod error;
mod events;
mod execution;
mod report;
mod session;

pub use config::{
    ConnectionConfig, ExecutionOptions, ExecutorConfig, InstanceOptions, MaaTask, Session,
    StaticOptions,
};
pub use environment::ExecutionEnvironment;
pub use error::{Error, ErrorKind, Result};
pub use events::{CoreEvent, ExecutionEvent, ExecutionHandle, ExecutionSubscription, StopToken};
pub use execution::{Executor, ExecutorInstance};
pub use report::{ExecutionReport, ExecutionStatus, SessionFailure, SessionReport, SessionStatus};

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub(crate) mod testutil;
