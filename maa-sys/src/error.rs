use std::{ffi::NulError, str::Utf8Error};

#[derive(Debug, Clone)]
pub enum Error {
    MAAError,
    BufferTooSmall,
    NulError(NulError),
    Utf8Error(Option<Utf8Error>),
    Custom(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MAAError => f.write_str("MAAError"),
            Error::BufferTooSmall => f.write_str("Buffer Too Small"),
            Error::NulError(err) => write!(f, "{}", err),
            Error::Utf8Error(Some(err)) => write!(f, "{}", err),
            Error::Utf8Error(None) => f.write_str("Invalid UTF-8"),
            Error::Custom(msg) => f.write_str(msg),
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

pub type Result<T> = std::result::Result<T, Error>;
