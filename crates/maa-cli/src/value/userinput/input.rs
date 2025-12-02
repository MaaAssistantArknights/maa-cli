use std::{
    fmt::Display,
    io::{self, Write},
    str::FromStr,
};

use serde::Deserialize;

use super::UserInput;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
/// A generic struct that represents a user input that queries the user for input.
///
/// For example, `Input::<i64>::new(Some(0), Some("medicine to use"))` represents a user input
/// that queries the user for an integer input, with default value 0 and description "medicine to
/// use".
///
/// If you want to query a boolean input, use [`super::BoolInput`].
pub struct Input<F> {
    /// Default value for this parameter.
    default: Option<F>,
    /// Description of this parameter
    description: Option<String>,
}

impl<F> Input<F> {
    pub fn new(default: Option<F>, description: Option<&str>) -> Self {
        Self {
            default,
            description: description.map(|s| s.to_string()),
        }
    }
}

impl<F: FromStr + Display + Clone> UserInput for Input<F> {
    type Value = F;

    fn default(self) -> Result<Self::Value, Self> {
        match self.default {
            Some(v) => Ok(v),
            None => Err(self),
        }
    }

    fn prompt(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "Please input")?;
        if let Some(description) = self.description.as_deref() {
            write!(writer, " {description}")?;
        } else {
            write!(writer, " a {}", std::any::type_name::<F>())?;
        }
        if let Some(default) = &self.default {
            write!(writer, " [default: {default}]")?;
        }
        Ok(())
    }

    fn prompt_no_default(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "Default value not set, please input")?;
        if let Some(description) = self.description.as_deref() {
            write!(writer, " {description}")?;
        } else {
            write!(writer, " a {}", std::any::type_name::<F>())?;
        }
        Ok(())
    }

    fn parse(self, input: &str, writer: &mut impl Write) -> Result<Self::Value, io::Result<Self>> {
        if let Ok(value) = input.parse() {
            Ok(value)
        } else {
            err_err!(write!(
                writer,
                "Invalid input \"{}\", please try again",
                input
            ));
            Err(Ok(self))
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_test::{Token, assert_de_tokens};

    use super::*;
    use crate::assert_matches;

    #[test]
    fn serde() {
        let values: Vec<Input<i64>> = vec![
            Input::new(Some(0), Some("how many medicine to use")),
            Input::new(Some(0), None),
            Input::new(None, Some("how many medicine to use")),
            Input::new(None, None),
        ];

        assert_de_tokens(
            &values,
            &[
                Token::Seq { len: Some(4) },
                Token::Map { len: Some(2) },
                Token::Str("default"),
                Token::Some,
                Token::I32(0),
                Token::Str("description"),
                Token::Some,
                Token::Str("how many medicine to use"),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::Str("default"),
                Token::Some,
                Token::I32(0),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::Str("description"),
                Token::Some,
                Token::Str("how many medicine to use"),
                Token::MapEnd,
                Token::Map { len: Some(0) },
                Token::MapEnd,
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn construct() {
        assert_matches!(
            Input::new(Some(0), Some("medicine to use")),
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
        assert_eq!(Input::new(Some(0), None).default(), Ok(0));
        assert_eq!(
            Input::new(None::<i64>, None).default(),
            Err(Input::new(None, None))
        );
    }

    #[test]
    fn prompt() {
        let mut buffer = Vec::new();

        Input::<i64>::new(Some(0), Some("medicine to use"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input medicine to use [default: 0]");
        buffer.clear();

        Input::<i64>::new(None::<i64>, Some("medicine to use"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input medicine to use");
        buffer.clear();

        Input::<i64>::new(Some(0), None::<&str>)
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input a i64 [default: 0]");
        buffer.clear();

        Input::<i64>::new(None::<i64>, None::<&str>)
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Please input a i64");
        buffer.clear();
    }

    #[test]
    fn prompt_no_default() {
        let mut buffer = Vec::new();

        Input::<i64>::new(Some(0), Some("medicine to use"))
            .prompt_no_default(&mut buffer)
            .unwrap();
        assert_eq!(
            buffer,
            b"Default value not set, please input medicine to use"
        );
        buffer.clear();
    }

    #[test]
    fn parse() {
        let input = Input::new(Some(0), None);

        let mut output = Vec::new();

        assert_eq!(input.clone().parse("1", &mut output).unwrap(), 1);
        assert_eq!(
            input.clone().parse("a", &mut output).unwrap_err().unwrap(),
            input.clone()
        );
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Invalid input \"a\", please try again",
        );
    }
}
