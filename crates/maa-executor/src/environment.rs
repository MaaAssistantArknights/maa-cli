use std::error::Error as StdError;

use crate::{Error, ErrorKind, Result};

/// An execution-wide external environment that may provide a runtime address.
///
/// `prepare` must roll back its own partial work when it returns an error. After it succeeds, the
/// executor guarantees one `close` attempt before completing the execution.
pub trait ExecutionEnvironment {
    type Error: StdError + Send + Sync + 'static;

    /// Prepares the environment and optionally returns a connection address.
    fn prepare(&mut self) -> std::result::Result<Option<String>, Self::Error>;

    /// Closes the prepared environment.
    fn close(&mut self) -> std::result::Result<(), Self::Error>;
}

pub(crate) struct PreparedEnvironment<'a, E: ExecutionEnvironment> {
    environment: &'a mut E,
    closed: bool,
}

impl<'a, E: ExecutionEnvironment> PreparedEnvironment<'a, E> {
    pub(crate) fn new(environment: &'a mut E) -> Self {
        Self {
            environment,
            closed: false,
        }
    }

    pub(crate) fn close(mut self) -> Result<()> {
        self.closed = true;
        self.environment.close().map_err(|source| {
            Error::new(ErrorKind::Environment)
                .with_source(source)
                .with_desc("failed to close execution environment")
        })
    }
}

impl<E: ExecutionEnvironment> Drop for PreparedEnvironment<'_, E> {
    fn drop(&mut self) {
        if !self.closed
            && let Err(error) = self.environment.close()
        {
            log::error!("Failed to close execution environment during cleanup: {error}");
        }
    }
}
