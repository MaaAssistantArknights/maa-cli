use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display},
};

pub type BoxError = Box<dyn StdError + Send + Sync>;

#[derive(Debug)]
pub struct Error<K> {
    kind: K,
    source: Option<BoxError>,
    description: Option<Cow<'static, str>>,
}

impl<K> Error<K> {
    pub const fn new(kind: K) -> Self {
        Self {
            kind,
            source: None,
            description: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<BoxError>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_desc(mut self, desc: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn kind(&self) -> &K {
        &self.kind
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl<K: Display> Display for Error<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;

        if let Some(description) = &self.description {
            write!(f, ": {description}")?;
        }

        Ok(())
    }
}

impl<K: Display + fmt::Debug> StdError for Error<K> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|error| error as _)
    }
}

pub trait WithDesc<T, K> {
    fn with_desc(self, desc: &'static str) -> std::result::Result<T, Error<K>>;

    fn then_with_desc(self, f: impl FnOnce() -> String) -> std::result::Result<T, Error<K>>;
}

impl<T, K, E> WithDesc<T, K> for std::result::Result<T, E>
where
    E: Into<Error<K>>,
{
    fn with_desc(self, desc: &'static str) -> std::result::Result<T, Error<K>> {
        self.map_err(|error| error.into().with_desc(desc))
    }

    fn then_with_desc(self, f: impl FnOnce() -> String) -> std::result::Result<T, Error<K>> {
        self.map_err(|error| error.into().with_desc(f()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Alpha,
        Beta,
    }

    impl Display for Kind {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Alpha => f.write_str("alpha"),
                Self::Beta => f.write_str("beta"),
            }
        }
    }

    type Result<T, E = Error<Kind>> = std::result::Result<T, E>;

    impl From<std::io::Error> for Error<Kind> {
        fn from(error: std::io::Error) -> Self {
            Self::new(Kind::Alpha).with_source(error)
        }
    }

    #[test]
    fn display_includes_description() {
        let error = Error::new(Kind::Alpha).with_desc("boom");
        assert_eq!(error.to_string(), "alpha: boom");
    }

    #[test]
    fn kind_and_description_accessors() {
        let error = Error::new(Kind::Beta).with_desc("detail");
        assert_eq!(error.kind(), &Kind::Beta);
        assert_eq!(error.description(), Some("detail"));
    }

    #[test]
    fn with_desc_trait_appends_context() {
        let result: Result<()> = Err(Error::new(Kind::Alpha));
        let error = result.with_desc("context").unwrap_err();
        assert_eq!(error.to_string(), "alpha: context");
    }

    #[test]
    fn with_desc_trait_converts_foreign_errors() {
        let result: std::io::Result<()> =
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        let error = result.with_desc("read failed").unwrap_err();
        assert_eq!(error.kind(), &Kind::Alpha);
        assert_eq!(error.description(), Some("read failed"));
        assert!(error.source().is_some());
    }
}
