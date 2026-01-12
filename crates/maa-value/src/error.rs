use std::{fmt, io};

/// Error type for the maa-value crate
#[derive(Debug)]
pub enum Error {
    /// Circular dependencies detected in optional values
    CircularDependency,
    /// Optional value must be in an object
    OptionalNotInObject,
    /// Alternatives list is empty
    EmptyAlternatives,
    /// Default index is out of range
    IndexOutOfRange { index: usize, len: usize },
    /// No default value available in batch mode
    NoDefaultInBatchMode,
    /// Invalid UTF-8 encoding
    InvalidUtf8(maa_str_ext::Error),
    /// IO error occurred
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CircularDependency => write!(f, "circular dependencies detected"),
            Self::OptionalNotInObject => write!(f, "optional input must be in an object"),
            Self::EmptyAlternatives => write!(f, "alternatives is empty"),
            Self::IndexOutOfRange { index, len } => {
                write!(f, "index out of range expected 1 - {len}, got {index}")
            }
            Self::NoDefaultInBatchMode => write!(f, "can not get default value in batch mode"),
            Self::InvalidUtf8(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidUtf8(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<maa_str_ext::Error> for Error {
    fn from(err: maa_str_ext::Error) -> Self {
        Self::InvalidUtf8(err)
    }
}

/// Result type alias for maa-value operations
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::error::Error as StdError;

    use maa_str_ext::ToUtf8String;

    use super::*;

    #[test]
    fn display() {
        assert_eq!(
            Error::CircularDependency.to_string(),
            "circular dependencies detected"
        );
        assert_eq!(
            Error::OptionalNotInObject.to_string(),
            "optional input must be in an object"
        );
        assert_eq!(
            Error::EmptyAlternatives.to_string(),
            "alternatives is empty"
        );
        assert_eq!(
            Error::IndexOutOfRange { index: 3, len: 2 }.to_string(),
            "index out of range expected 1 - 2, got 3"
        );
        assert_eq!(
            Error::IndexOutOfRange { index: 0, len: 2 }.to_string(),
            "index out of range expected 1 - 2, got 0"
        );
        assert_eq!(
            Error::NoDefaultInBatchMode.to_string(),
            "can not get default value in batch mode"
        );
        assert_eq!(
            Error::Io(io::Error::other("test")).to_string(),
            "I/O error: test"
        );

        let invalid_bytes = vec![0xFF];
        let utf8_err = invalid_bytes.to_utf8_string().unwrap_err();
        let utf8_err_str = utf8_err.to_string();
        assert_eq!(Error::InvalidUtf8(utf8_err).to_string(), utf8_err_str);
    }

    #[test]
    fn debug_format() {
        // Test Debug trait implementation
        assert_eq!(
            format!("{:?}", Error::CircularDependency),
            "CircularDependency"
        );
        assert_eq!(
            format!("{:?}", Error::OptionalNotInObject),
            "OptionalNotInObject"
        );
        assert_eq!(
            format!("{:?}", Error::EmptyAlternatives),
            "EmptyAlternatives"
        );
        assert_eq!(
            format!("{:?}", Error::IndexOutOfRange { index: 3, len: 2 }),
            "IndexOutOfRange { index: 3, len: 2 }"
        );
        assert_eq!(
            format!("{:?}", Error::NoDefaultInBatchMode),
            "NoDefaultInBatchMode"
        );

        let invalid_bytes = vec![0xFF];
        let utf8_err = invalid_bytes.to_utf8_string().unwrap_err();
        assert!(format!("{:?}", Error::InvalidUtf8(utf8_err)).starts_with("InvalidUtf8"));
    }

    #[test]
    fn error_source() {
        // Test that non-IO errors have no source
        assert!(Error::CircularDependency.source().is_none());
        assert!(Error::OptionalNotInObject.source().is_none());
        assert!(Error::EmptyAlternatives.source().is_none());
        assert!(
            Error::IndexOutOfRange { index: 3, len: 2 }
                .source()
                .is_none()
        );
        assert!(Error::NoDefaultInBatchMode.source().is_none());

        // Test that IO error has a source
        let io_err = io::Error::other("test error");
        let err = Error::Io(io_err);
        assert!(err.source().is_some());

        // Test that InvalidUtf8 error has a source
        use maa_str_ext::ToUtf8String;
        let invalid_bytes = vec![0xFF];
        let utf8_err = invalid_bytes.to_utf8_string().unwrap_err();
        let err = Error::InvalidUtf8(utf8_err);
        assert!(err.source().is_some());
        assert!(err.source().unwrap().is::<maa_str_ext::Error>());
    }

    #[test]
    fn from_io_error() {
        // Test From<io::Error> conversion
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert_eq!(err.to_string(), "I/O error: file not found");
    }

    #[test]
    fn from_io_error_different_kinds() {
        // Test various IO error kinds
        let errors = vec![
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
            io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused"),
            io::Error::new(io::ErrorKind::InvalidInput, "invalid input"),
            io::Error::other("other error"),
        ];

        for io_err in errors {
            let msg = io_err.to_string();
            let err: Error = io_err.into();
            assert!(matches!(err, Error::Io(_)));
            assert_eq!(err.to_string(), format!("I/O error: {}", msg));
        }
    }

    #[test]
    fn from_maa_str_ext_error() {
        let invalid_bytes = vec![0xFF];
        let utf8_err = invalid_bytes.to_utf8_string().unwrap_err();
        let err: Error = utf8_err.into();
        assert!(matches!(err, Error::InvalidUtf8(_)));
    }

    #[test]
    fn error_as_std_error() {
        // Test that Error implements std::error::Error properly
        fn takes_error(_err: &impl StdError) {}

        takes_error(&Error::CircularDependency);
        takes_error(&Error::OptionalNotInObject);
        takes_error(&Error::EmptyAlternatives);
        takes_error(&Error::IndexOutOfRange { index: 1, len: 2 });
        takes_error(&Error::NoDefaultInBatchMode);
        takes_error(&Error::Io(io::Error::other("test")));
    }
}
