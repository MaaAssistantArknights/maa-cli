use super::{Result, UserInput};

use std::{fmt::Display, io::Write, str::FromStr};

use serde::{Deserialize, Serialize};

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
/// A generic struct that represents a user input that queries the user for input.
///
/// For example, `Input::<i64>::new(Some(0), Some("medicine to use"))` represents a user input
/// that queries the user for an integer input, with default value 0 and description "medicine to
/// use".
///
/// If you want to query a boolean input, use [`BoolInput`].
pub struct Input<F> {
    /// Default value for this parameter.
    default: Option<F>,
    /// Description of this parameter
    description: Option<String>,
}

impl<F> Input<F> {
    pub fn new<I, S>(default: Option<I>, description: Option<S>) -> Self
    where
        I: Into<F>,
        S: Into<String>,
    {
        Self {
            default: default.map(|i| i.into()),
            description: description.map(|s| s.into()),
        }
    }
}

impl<F: FromStr + Display> UserInput for Input<F> {
    type Value = F;

    fn default(&self) -> Option<F> {
        self.default
    }

    fn prompt(&self, mut writer: impl Write) -> Result<()> {
        write!(writer, "Please input")?;
        if let Some(description) = self.description {
            write!(writer, " {}", description)?;
        } else {
            write!(writer, " a {}", std::any::type_name::<F>())?;
        }
        if let Some(default) = &self.default {
            write!(writer, " [default: {}]", default)?;
        }
        write!(writer, ": ")?;
        Ok(())
    }

    fn prompt_no_default(&self, mut writer: impl Write) -> Result<()> {
        write!(writer, "Default value not set, please input")
    }

    fn parse(&self, input: &str) -> Result<F, String> {
        match input.parse() {
            Ok(value) => Ok(value),
            Err(_) => Err(format!("Invalid input \"{}\", please try again", input)),
        }
    }
}

impl<F: FromStr + Display + Serialize> Serialize for Input<F> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        super::serialize_userinput(self, serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::assert_matches;

    use serde_test::{assert_de_tokens, assert_ser_tokens_error, Token};

    #[test]
    fn serde() {
        let values: Vec<Input<i64>> = vec![
            Input::new(Some(0), Some("how many medicine to use")),
            Input::new(Some(0), None::<&str>),
            Input::new(None::<i64>, Some("how many medicine to use")),
            Input::new(None::<i64>, None::<&str>),
        ];

        assert_de_tokens(
            &values,
            &[
                Token::Seq { len: Some(4) },
                Token::Map { len: Some(2) },
                Token::Str("default"),
                Token::I64(0),
                Token::Str("description"),
                Token::Str("how many medicine to use"),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::Str("default"),
                Token::I64(0),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::Str("description"),
                Token::Str("how many medicine to use"),
                Token::MapEnd,
                Token::Map { len: Some(0) },
                Token::MapEnd,
                Token::SeqEnd,
            ],
        );

        assert_ser_tokens_error(
            &values,
            &[Token::Seq { len: Some(4) }, Token::I64(0), Token::I64(0)],
            "can not get default value in batch mode",
        )
    }

    #[test]
    fn construct() {
        assert_matches!(
            Input::<i64>::new(Some(0), Some("medicine to use")),
            Input::<i64> {
                default: Some(0),
                description: Some(s)
            } if s == "medicine to use",
        );
        assert_matches!(
            Input::<i64>::new(None::<i64>, Some("medicine to use")),
            Input::<i64> {
                default: None,
                description: Some(s)
            } if s == "medicine to use",
        );
        assert_matches!(
            Input::<i64>::new(Some(0), None::<&str>),
            Input::<i64> {
                default: Some(0),
                description: None,
            },
        );
        assert_matches!(
            Input::<i64>::new(None::<i64>, None::<&str>),
            Input::<i64> {
                default: None,
                description: None,
            },
        );
    }

    #[test]
    fn default() {
        assert_eq!(
            Input::<i64>::new(Some(0), Some("medicine to use")).default(),
            Some(0)
        );
        assert_eq!(
            Input::<i64>::new(None::<i64>, Some("medicine to use")).default(),
            None
        );
        assert_eq!(Input::<i64>::new(Some(0), None::<&str>).default(), Some(0));
        assert_eq!(Input::<i64>::new(None::<i64>, None::<&str>).default(), None);
    }

    #[test]
    fn prompt() {
        let mut buffer = Vec::new();

        Input::<i64>::new(Some(0), Some("medicine to use"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input medicine to use [default: 0]: ");
        buffer.clear();

        Input::<i64>::new(None::<i64>, Some("medicine to use"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input medicine to use: ");
        buffer.clear();

        Input::<i64>::new(Some(0), None::<&str>)
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input a i64 [default: 0]: ");
        buffer.clear();

        Input::<i64>::new(None::<i64>, None::<&str>)
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input a i64: ");
        buffer.clear();
    }

    #[test]
    fn prompt_no_default() {
        let mut buffer = Vec::new();

        Input::<i64>::new(Some(0), Some("medicine to use"))
            .prompt_no_default(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Default value not set, please input");
        buffer.clear();
    }

    #[test]
    fn parse() {
        let input = Input::<i64>::new(Some(0), Some("medicine to use"));

        assert_eq!(input.parse("0"), Ok(0));
        assert_eq!(input.parse("1"), Ok(1));
        assert_eq!(
            input.parse("1.0"),
            Err("Invalid input \"1.0\", please try again".to_owned())
        );
        assert_eq!(
            input.parse("a"),
            Err("Invalid input \"a\", please try again".to_owned())
        );
    }
}
