use std::{ffi::NulError, str::Utf8Error};

/// Error type for MAA
#[derive(Debug, Clone)]
pub enum Error {
    /// MAA returned an error
    MAAError,
    /// Buffer too small
    BufferTooSmall,
    /// Nul byte found in string
    NulError(NulError),
    /// Invalid UTF-8
    Utf8Error(Option<Utf8Error>),
    /// Custom error message
    Custom(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MAAError => f.write_str("MaaCore returned an error, check its log for details"),
            Error::BufferTooSmall => f.write_str("Buffer Too Small"),
            Error::NulError(err) => write!(f, "{}", err),
            Error::Utf8Error(Some(err)) => write!(f, "{}", err),
            Error::Utf8Error(None) => f.write_str("Invalid UTF-8"),
            Error::Custom(custom) => write!(f, "{}", custom),
        }
    }
}

impl std::error::Error for Error {}

impl From<NulError> for Error {
    fn from(err: NulError) -> Self {
        Error::NulError(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Error::Utf8Error(Some(err))
    }
}

impl Error {
    pub fn custom(msg: impl Into<String>) -> Self {
        Error::Custom(msg.into())
    }
}

/// Similar to `anyhow::Result<T>` but the default error type is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
