use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display},
};

type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Stable category for an executor error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidConfig,
    Bootstrap,
    Environment,
    Connection,
    Instance,
    Task,
    Interrupted,
    Execution,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig => f.write_str("invalid executor config"),
            Self::Bootstrap => f.write_str("executor bootstrap failed"),
            Self::Environment => f.write_str("execution environment failed"),
            Self::Connection => f.write_str("connection failed"),
            Self::Instance => f.write_str("assistant instance failed"),
            Self::Task => f.write_str("task execution failed"),
            Self::Interrupted => f.write_str("execution interrupted"),
            Self::Execution => f.write_str("execution failed"),
        }
    }
}

/// Structured executor error with optional context and source chaining.
#[derive(Debug, thiserror::Error)]
pub struct Error {
    kind: ErrorKind,
    #[source]
    source: Option<BoxError>,
    description: Option<Cow<'static, str>>,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(description) = &self.description {
            write!(f, ": {description}")?;
        }
        Ok(())
    }
}

impl Error {
    /// Creates an error in the given category.
    pub const fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            source: None,
            description: None,
        }
    }

    /// Attaches the underlying source error.
    pub fn with_source(mut self, source: impl Into<BoxError>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Attaches executor-level context.
    pub fn with_desc(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Returns the stable error category.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the attached executor-level context.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn display_error_kind() {
        assert_eq!(
            ErrorKind::InvalidConfig.to_string(),
            "invalid executor config"
        );
        assert_eq!(
            ErrorKind::Bootstrap.to_string(),
            "executor bootstrap failed"
        );
        assert_eq!(
            ErrorKind::Environment.to_string(),
            "execution environment failed"
        );
        assert_eq!(ErrorKind::Connection.to_string(), "connection failed");
        assert_eq!(ErrorKind::Instance.to_string(), "assistant instance failed");
        assert_eq!(ErrorKind::Task.to_string(), "task execution failed");
        assert_eq!(ErrorKind::Interrupted.to_string(), "execution interrupted");
        assert_eq!(ErrorKind::Execution.to_string(), "execution failed");
    }

    #[test]
    fn error_preserves_source_and_description() {
        let error = Error::new(ErrorKind::Environment)
            .with_desc("failed to prepare execution environment")
            .with_source(std::io::Error::other("boom"));

        assert_eq!(error.kind(), ErrorKind::Environment);
        assert_eq!(
            error.to_string(),
            "execution environment failed: failed to prepare execution environment"
        );
        assert_eq!(
            error.description(),
            Some("failed to prepare execution environment")
        );
        assert!(error.source().is_some());
    }
}
