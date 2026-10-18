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

/// Read an optional variable, using `default` only when it is unset.
pub fn read_or<T: FromStr>(name: &str, default: T) -> Result<T, Error<T::Err>> {
    value_or(name, env::var(name), default)
}

fn value_or<T: FromStr>(
    name: &str,
    value: Result<String, env::VarError>,
    default: T,
) -> Result<T, Error<T::Err>> {
    match value {
        Ok(value) => parse(name, &value),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(source) => Err(Error::Read {
            name: name.to_owned(),
            source,
        }),
    }
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
    fn optional_values_default_only_when_unset() {
        let _ = read_or::<bool>;
        assert!(matches!(
            value_or("VALUE", Err(env::VarError::NotPresent), true),
            Ok(true)
        ));
        assert!(matches!(
            value_or("VALUE", Ok("false".to_owned()), true),
            Ok(false)
        ));
        assert!(matches!(
            value_or("VALUE", Ok("off".to_owned()), true),
            Err(Error::Parse { .. })
        ));
        assert!(matches!(
            value_or("VALUE", Err(env::VarError::NotUnicode("x".into())), true),
            Err(Error::Read { .. })
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
