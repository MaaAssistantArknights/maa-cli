//! Typed environment variable parsing for automation commands.

use std::{env, fmt, str::FromStr};

#[derive(Debug)]
pub enum Error<E> {
    Read { name: String, source: env::VarError },
    Parse { name: String, source: E },
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { name, source } => write!(f, "Failed to read {name}: {source}"),
            Self::Parse { name, source } => write!(f, "Invalid {name}: {source}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { .. } => None,
        }
    }
}

/// Read a required variable, reporting missing, non-Unicode or invalid values.
pub fn read<T: FromStr>(name: &str) -> Result<T, Error<T::Err>> {
    let value = env::var(name).map_err(|source| Error::Read {
        name: name.to_owned(),
        source,
    })?;
    parse(name, &value)
}

fn parse<T: FromStr>(name: &str, value: &str) -> Result<T, Error<T::Err>> {
    value.parse().map_err(|source| Error::Parse {
        name: name.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_preserves_errors_without_formatting_traits() {
        struct ParseError(u8);
        struct Value;

        impl FromStr for Value {
            type Err = ParseError;

            fn from_str(_: &str) -> Result<Self, Self::Err> {
                Err(ParseError(42))
            }
        }

        let _ = read::<Value>;
        assert!(matches!(
            parse::<Value>("VALUE", "invalid"),
            Err(Error::Parse { name, source: ParseError(42) }) if name == "VALUE"
        ));
    }

    #[test]
    fn reporting_formats_the_parse_error() {
        #[derive(Debug)]
        struct ParseError;

        impl fmt::Display for ParseError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "ParseError")
            }
        }

        let error = Error::Parse {
            name: "VALUE".to_owned(),
            source: ParseError,
        };
        let error: &dyn std::error::Error = &error;
        assert_eq!(error.to_string(), "Invalid VALUE: ParseError");
    }
}
