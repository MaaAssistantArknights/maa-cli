//! Error types for the MAA installer library.

use std::{error::Error as StdError, fmt};

/// Categorization of different error types that can occur during installation operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// I/O operations failed (file system errors, permission issues, etc.)
    Io,

    /// Failed to build or parse a verifier (invalid hash format, incorrect hash length, etc.)
    Verifier,

    /// Failed to verify the downloaded files (hash mismatch, etc.)
    Verify,

    /// Failed to extract or decompress archive files
    Extract,

    /// Network-related failures (connection issues, timeouts, HTTP errors)
    Network,

    /// Any other error not covered by the specific categories above
    Other,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io => f.write_str("I/O error"),
            Self::Verifier => f.write_str("Failed to build verifier"),
            Self::Verify => f.write_str("Verification failed"),
            Self::Extract => f.write_str("Extraction error"),
            Self::Network => f.write_str("Network error"),
            Self::Other => f.write_str("Other error"),
        }
    }
}

/// The primary error type for the MAA installer library.
///
/// This error type provides:
/// - Categorization via [`ErrorKind`]
/// - Optional descriptive message for user-facing error details
/// - Source error chaining for debugging and root cause analysis
/// - Send + Sync bounds for thread safety
#[derive(Debug, thiserror::Error)]
pub struct Error {
    /// The category of error that occurred
    kind: ErrorKind,
    /// The underlying source error, if any
    #[source]
    source: Option<Box<dyn StdError + Send + Sync>>,
    /// Human-readable description of what went wrong
    description: Option<std::borrow::Cow<'static, str>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if let Some(description) = &self.description {
            write!(f, ": {}", description)?;
        }

        Ok(())
    }
}

impl Error {
    /// Creates a new error with the specified kind.
    pub const fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            source: None,
            description: None,
        }
    }

    /// Attaches a source error to this error.
    ///
    /// This is useful for preserving the original error while adding context.
    /// The source error will be available through the [`std::error::Error::source`] method.
    pub fn with_source(mut self, source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Attaches a description to this error.
    ///
    /// The description provides additional context about what went wrong
    /// and is displayed to users when the error is formatted.
    pub fn with_desc(mut self, desc: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Returns the kind of error that occurred.
    ///
    /// This allows callers to categorize and handle errors without
    /// needing to inspect the underlying source error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the human-readable description of this error, if any.
    ///
    /// The description provides additional context about what went wrong
    /// and is suitable for displaying to end users.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

// Convenience From implementations for common error types

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io).with_source(error)
    }
}

impl From<ureq::Error> for Error {
    fn from(error: ureq::Error) -> Self {
        use ureq::Error::*;
        match error {
            Json(e) => Self::new(ErrorKind::Other).with_source(e),
            e => Self::new(ErrorKind::Network).with_source(e),
        }
    }
}

/// A convenience trait for appending descriptions to errors in result chains.
///
/// This trait provides methods to add descriptive context to errors
/// while preserving the original error information.
pub trait WithDesc<T> {
    /// Appends a static string description to an error if the result is `Err`.
    ///
    /// # Examples
    fn with_desc(self, desc: &'static str) -> Result<T>;

    /// Lazily appends a description to an error if the result is `Err`.
    ///
    /// The description is computed only when needed (when the result is `Err`).
    fn then_with_desc(self, f: impl FnOnce() -> String) -> Result<T>;
}

impl<T, E: Into<Error>> WithDesc<T> for std::result::Result<T, E> {
    fn with_desc(self, desc: &'static str) -> Result<T> {
        self.map_err(|err| err.into().with_desc(desc))
    }

    fn then_with_desc(self, f: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|err| err.into().with_desc(f()))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
