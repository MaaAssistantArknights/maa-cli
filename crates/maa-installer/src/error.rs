//! Error types for the MAA installer library.

use std::{borrow::Cow, error::Error as StdError, fmt};

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

#[derive(Debug)]
pub struct Error(maa_error::Error<ErrorKind>);

impl Error {
    pub const fn new(kind: ErrorKind) -> Self {
        Self(maa_error::Error::new(kind))
    }

    pub fn with_source(mut self, source: impl Into<maa_error::BoxError>) -> Self {
        self.0 = self.0.with_source(source);
        self
    }

    pub fn with_desc(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.0 = self.0.with_desc(desc);
        self
    }

    pub fn kind(&self) -> ErrorKind {
        *self.0.kind()
    }

    pub fn description(&self) -> Option<&str> {
        self.0.description()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io).with_source(error)
    }
}

impl From<ureq::Error> for Error {
    fn from(error: ureq::Error) -> Self {
        let kind = if matches!(error, ureq::Error::Other(_)) {
            ErrorKind::Other
        } else {
            ErrorKind::Network
        };
        Self::new(kind).with_source(error)
    }
}

impl From<maa_error::Error<ErrorKind>> for Error {
    fn from(error: maa_error::Error<ErrorKind>) -> Self {
        Self(error)
    }
}

impl From<Error> for maa_error::Error<ErrorKind> {
    fn from(error: Error) -> Self {
        error.0
    }
}

pub trait WithDesc<T> {
    fn with_desc(self, desc: &'static str) -> Result<T>;

    fn then_with_desc(self, f: impl FnOnce() -> String) -> Result<T>;
}

impl<T, E> WithDesc<T> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn with_desc(self, desc: &'static str) -> Result<T> {
        self.map_err(|error| error.into().with_desc(desc))
    }

    fn then_with_desc(self, f: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|error| error.into().with_desc(f()))
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
            assert_eq!(format!("{kind:?}"), "Io");
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
                assert!(error.to_string().contains("Description"));
            }
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
            let error = ureq::get("http://127.0.0.1:9").call().unwrap_err();
            let error: Error = error.into();
            assert_eq!(error.kind(), ErrorKind::Network);
        }
    }

    mod with_desc_trait {
        use super::*;

        #[test]
        fn test_with_desc_on_ok() {
            let result: Result<i32> = Ok(42);
            let result_with_desc = result.with_desc("This should not be added");
            assert_eq!(result_with_desc.unwrap(), 42);
        }

        #[test]
        fn test_with_desc_on_err() {
            let result: Result<i32> = Err(Error::new(ErrorKind::Network));
            let result_with_desc = result.with_desc("Connection failed");

            let error = result_with_desc.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Network);
            assert_eq!(error.description(), Some("Connection failed"));
        }

        #[test]
        fn test_then_with_desc_on_ok() {
            let result: Result<i32> = Ok(42);
            let result_with_desc = result
                .then_with_desc(|| panic!("This closure should not be called for Ok results"));
            assert_eq!(result_with_desc.unwrap(), 42);
        }

        #[test]
        fn test_then_with_desc_on_err() {
            let result: Result<i32> = Err(Error::new(ErrorKind::Verify));

            let result_with_desc =
                result.then_with_desc(|| format!("Verification failed at line {}", 42));

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

            let error = result.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Io);
            assert_eq!(error.description(), Some("Failed to read config file"));
        }

        #[test]
        fn test_multiple_with_desc() {
            let result: Result<i32> = Err(Error::new(ErrorKind::Extract));

            let result = result
                .with_desc("First description")
                .map_err(|error| error.with_desc("Second description"));

            let error = result.unwrap_err();
            assert_eq!(error.description(), Some("Second description"));
        }
    }
}
