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
            Self::Verifier => f.write_str("Build verifier failed"),
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

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    mod error_kind {
        use super::*;

        #[test]
        fn test_display() {
            assert_eq!(ErrorKind::Io.to_string(), "I/O error");
            assert_eq!(ErrorKind::Verifier.to_string(), "Build verifier failed");
            assert_eq!(ErrorKind::Verify.to_string(), "Verification failed");
            assert_eq!(ErrorKind::Extract.to_string(), "Extraction error");
            assert_eq!(ErrorKind::Network.to_string(), "Network error");
            assert_eq!(ErrorKind::Other.to_string(), "Other error");
        }

        #[test]
        fn test_equality() {
            assert_eq!(ErrorKind::Io, ErrorKind::Io);
            assert_eq!(ErrorKind::Verifier, ErrorKind::Verifier);
            assert_ne!(ErrorKind::Io, ErrorKind::Network);
        }

        #[test]
        fn test_debug() {
            let kind = ErrorKind::Io;
            assert_eq!(format!("{:?}", kind), "Io");
        }
    }

    mod error {
        use super::*;

        #[test]
        fn test_new() {
            let error = Error::new(ErrorKind::Io);
            assert_eq!(error.kind(), ErrorKind::Io);
            assert!(error.source().is_none());
            assert!(error.description().is_none());
        }

        #[test]
        fn test_with_desc_static() {
            let error = Error::new(ErrorKind::Network).with_desc("Connection failed");

            assert_eq!(error.kind(), ErrorKind::Network);
            assert_eq!(error.description(), Some("Connection failed"));
        }

        #[test]
        fn test_with_desc_string() {
            let desc = format!("Failed with code {}", 404);
            let error = Error::new(ErrorKind::Network).with_desc(desc.clone());

            assert_eq!(error.description(), Some(desc.as_str()));
        }

        #[test]
        fn test_with_source() {
            let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let error = Error::new(ErrorKind::Io).with_source(io_error);

            assert!(error.source().is_some());
        }

        #[test]
        fn test_chaining() {
            let error = Error::new(ErrorKind::Extract)
                .with_desc("Failed to extract archive")
                .with_source(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "permission denied",
                ));

            assert_eq!(error.kind(), ErrorKind::Extract);
            assert_eq!(error.description(), Some("Failed to extract archive"));
            assert!(error.source().is_some());
        }

        #[test]
        fn test_display_without_desc() {
            let error = Error::new(ErrorKind::Verify);
            assert_eq!(error.to_string(), "Verification failed");
        }

        #[test]
        fn test_display_with_desc() {
            let error = Error::new(ErrorKind::Verify).with_desc("Hash mismatch detected");
            assert_eq!(
                error.to_string(),
                "Verification failed: Hash mismatch detected"
            );
        }

        #[test]
        fn test_display_all_kinds() {
            let test_cases = [
                ErrorKind::Io,
                ErrorKind::Verifier,
                ErrorKind::Verify,
                ErrorKind::Extract,
                ErrorKind::Network,
                ErrorKind::Other,
            ];

            for kind in test_cases {
                let error = Error::new(kind).with_desc("Description");
                assert_eq!(error.to_string(), format!("{kind}: Description"));
            }
        }

        #[test]
        fn test_debug() {
            let error = Error::new(ErrorKind::Network).with_desc("Connection timeout");

            let debug_str = format!("{:?}", error);
            assert!(debug_str.contains("Network"));
            assert!(debug_str.contains("Connection timeout"));
        }
    }

    mod from_impls {
        use super::*;

        #[test]
        fn test_from_io_error() {
            let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let error: Error = io_error.into();

            assert_eq!(error.kind(), ErrorKind::Io);
            assert!(error.source().is_some());
        }

        #[test]
        fn test_from_ureq_transport_error() {
            // Create a transport error by making a request to invalid URL
            let result = ureq::get("http://invalid.local.test").call();

            if let Err(ureq_error) = result {
                let error: Error = ureq_error.into();
                assert_eq!(error.kind(), ErrorKind::Network);
                assert!(error.source().is_some());
            }
        }
    }

    mod with_desc_trait {
        use super::*;

        #[test]
        fn test_with_desc_on_ok() {
            let result: Result<i32> = Ok(42);
            let result_with_desc = result.with_desc("This should not be added");

            assert!(result_with_desc.is_ok());
            assert_eq!(result_with_desc.unwrap(), 42);
        }

        #[test]
        fn test_with_desc_on_err() {
            let result: Result<i32> = Err(Error::new(ErrorKind::Network));
            let result_with_desc = result.with_desc("Connection failed");

            assert!(result_with_desc.is_err());
            let error = result_with_desc.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Network);
            assert_eq!(error.description(), Some("Connection failed"));
        }

        #[test]
        fn test_then_with_desc_on_ok() {
            let result: Result<i32> = Ok(42);
            let mut called = false;

            let result_with_desc = result.then_with_desc(|| {
                called = true;
                "This should not be called".to_string()
            });

            assert!(result_with_desc.is_ok());
            assert_eq!(result_with_desc.unwrap(), 42);
            assert!(!called, "Function should not be called for Ok variant");
        }

        #[test]
        fn test_then_with_desc_on_err() {
            let result: Result<i32> = Err(Error::new(ErrorKind::Verify));
            let mut called = false;

            let result_with_desc = result.then_with_desc(|| {
                called = true;
                format!("Verification failed at line {}", 42)
            });

            assert!(result_with_desc.is_err());
            assert!(called, "Function should be called for Err variant");

            let error = result_with_desc.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Verify);
            assert_eq!(error.description(), Some("Verification failed at line 42"));
        }

        #[test]
        fn test_with_desc_chaining() {
            fn might_fail() -> Result<i32> {
                Err(Error::new(ErrorKind::Io))
            }

            let result = might_fail().with_desc("Failed to read file").map(|x| x * 2);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Io);
            assert_eq!(error.description(), Some("Failed to read file"));
        }

        #[test]
        fn test_with_desc_from_io_error() {
            fn read_file() -> std::io::Result<String> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "file not found",
                ))
            }

            let result: Result<String> = read_file().with_desc("Failed to read config file");

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Io);
            assert_eq!(error.description(), Some("Failed to read config file"));
            assert!(error.source().is_some());
        }

        #[test]
        fn test_multiple_with_desc() {
            let result: Result<i32> = Err(Error::new(ErrorKind::Extract));

            // Only the first description should be kept
            let result = result
                .with_desc("First description")
                .map_err(|e| e.with_desc("Second description"));

            assert!(result.is_err());
            let error = result.unwrap_err();
            // The second description overwrites the first
            assert_eq!(error.description(), Some("Second description"));
        }
    }
}
