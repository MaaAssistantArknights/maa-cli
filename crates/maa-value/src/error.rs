use std::io;

/// Error type for the maa-value crate.
///
/// Represents various errors that can occur during value resolution, user input processing,
/// and I/O operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Circular dependencies detected among optional fields.
    ///
    /// This error occurs when optional fields have dependencies that form a cycle,
    /// such as field A depending on B, and B depending on A (directly or indirectly).
    /// Resolution cannot proceed because there's no valid order to evaluate the fields.
    ///
    /// # Example
    /// ```ignore
    /// object!(
    ///     "field_a" if "field_b" == true => value,
    ///     "field_b" if "field_a" == true => value,
    /// )
    /// ```
    #[error("Circular dependency detected among optional fields")]
    CircularDependency,

    /// An `Optional` variant was encountered outside of an object context.
    ///
    /// Optional fields can only exist within objects, as they require the ability to check
    /// sibling fields for dependency conditions. Attempting to resolve a standalone optional
    /// value results in this error.
    #[error("Optional fields can only exist within objects")]
    OptionalNotInObject,

    /// A selection input has an empty alternatives list.
    ///
    /// Selection inputs must have at least one alternative to choose from.
    #[error("Selection input has an empty alternatives list")]
    EmptyAlternatives,

    /// The default index for a selection is out of the valid range.
    ///
    /// The index is 1-based (not 0-based) and must be between 1 and the number of alternatives.
    #[error("Index {index} is out of range [1, {len}]")]
    IndexOutOfRange {
        /// The invalid index that was provided (1-based).
        index: usize,
        /// The total number of alternatives available.
        len: usize,
    },

    /// No default value available when running in batch mode.
    ///
    /// When user input is required but the application is running in batch mode
    /// (non-interactive), and no default value was provided, this error is returned.
    #[error("No default value available in batch mode")]
    NoDefaultInBatchMode,

    /// Invalid UTF-8 encoding encountered during string conversion.
    #[error("Invalid UTF-8 encoding")]
    InvalidUtf8(#[from] maa_str_ext::Error),

    /// An I/O error occurred during file operations.
    #[error("I/O error")]
    Io(#[from] io::Error),
}

/// Result type alias for maa-value operations
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_str_ext::ToUtf8String;

    use super::*;

    #[test]
    fn display() {
        assert_eq!(
            Error::CircularDependency.to_string(),
            "circular dependencies detected optional fields"
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
}
