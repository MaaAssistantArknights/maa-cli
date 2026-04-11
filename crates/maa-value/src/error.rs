/// Error type for the maa-value crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Circular dependencies detected among optional fields.
    ///
    /// # Example
    ///
    /// ```
    /// use maa_value::prelude::*;
    ///
    /// template!(
    ///     "field_a" if "field_b" == 1 => 1,
    ///     "field_b" if "field_a" == 1 => 1,
    /// );
    /// ```
    #[error("Circular dependency detected among optional fields")]
    CircularDependency,

    /// An `Optional` variant was encountered outside of an object context.
    #[error("Optional fields can only exist within objects")]
    OptionalNotInObject,

    /// An error occurred during resolve input
    #[error("Failed to resolve input: {0}")]
    Resolve(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

    /// Invalid UTF-8 encoding encountered during string conversion.
    #[error("Invalid UTF-8 encoding")]
    InvalidUtf8(#[from] maa_str_ext::Error),
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
            "Circular dependency detected among optional fields"
        );
        assert_eq!(
            Error::OptionalNotInObject.to_string(),
            "Optional fields can only exist within objects"
        );
        let invalid_bytes = vec![0xFF];
        let utf8_err = invalid_bytes.to_utf8_string().unwrap_err();
        assert_eq!(
            Error::InvalidUtf8(utf8_err).to_string(),
            "Invalid UTF-8 encoding"
        );
    }
}
