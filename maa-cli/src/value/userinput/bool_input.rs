use super::UserInput;

use std::io::{self, Write};

use serde::Deserialize;

/// A struct that represents a user input that queries the user for boolean input.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BoolInput {
    /// Default value for this parameter.
    default: Option<bool>,
    /// Description of this parameter
    description: Option<String>,
}

impl BoolInput {
    pub fn new(default: Option<bool>, description: Option<&str>) -> Self {
        Self {
            default,
            description: description.map(|s| s.to_string()),
        }
    }
}

impl UserInput for BoolInput {
    type Value = bool;

    fn default(self) -> Result<Self::Value, Self> {
        match self.default {
            Some(v) => Ok(v),
            None => Err(self),
        }
    }

    fn prompt(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        write!(writer, "Whether to")?;
        if let Some(description) = &self.description {
            write!(writer, " {}", description)?;
        } else {
            write!(writer, " do something")?;
        }
        if let Some(default) = &self.default {
            if *default {
                write!(writer, " [Y/n]")?;
            } else {
                write!(writer, " [y/N]")?;
            }
        } else {
            write!(writer, " [y/n]")?;
        }
        Ok(())
    }

    fn prompt_no_default(&self, writer: &mut impl Write) -> Result<(), io::Error> {
        write!(writer, "Default value not set, please input y/n")
    }

    fn parse(
        self,
        trimmed: &str,
        writer: &mut impl Write,
    ) -> Result<Self::Value, io::Result<Self>> {
        match trimmed {
            "y" | "Y" | "yes" | "Yes" | "YES" => Ok(true),
            "n" | "N" | "no" | "No" | "NO" => Ok(false),
            _ => {
                err_err!(writer.write_all(b"Invalid input, please input y/n"));
                Err(Ok(self))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::assert_matches;

    use serde_test::{assert_de_tokens, Token};

    #[test]
    fn serde() {
        let values = vec![
            BoolInput::new(Some(true), Some("do something")),
            BoolInput::new(Some(false), None),
            BoolInput::new(None, Some("do something")),
            BoolInput::new(None, None),
        ];

        assert_de_tokens(
            &values,
            &[
                Token::Seq { len: Some(4) },
                Token::Map { len: Some(2) },
                Token::Str("default"),
                Token::Some,
                Token::Bool(true),
                Token::Str("description"),
                Token::Some,
                Token::Str("do something"),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::Str("default"),
                Token::Some,
                Token::Bool(false),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::Str("description"),
                Token::Some,
                Token::Str("do something"),
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
            BoolInput::new(Some(true), Some("do something")),
            BoolInput {
                default: Some(true),
                description: Some(description),
            } if description == "do something"
        );

        assert_matches!(
            BoolInput::new(Some(true), None),
            BoolInput {
                default: Some(true),
                description: None,
            }
        );

        assert_matches!(
            BoolInput::new(None, Some("do something")),
            BoolInput {
                default: None,
                description: Some(description),
            } if description == "do something"
        );

        assert_matches!(
            BoolInput::new(None, None),
            BoolInput {
                default: None,
                description: None,
            }
        );
    }

    #[test]
    fn default() {
        assert_eq!(BoolInput::new(Some(true), None).default(), Ok(true));

        assert_eq!(
            BoolInput::new(None, None).default(),
            Err(BoolInput::new(None, None))
        );
    }

    #[test]
    fn prompt() {
        let mut buffer = Vec::new();

        BoolInput::new(Some(true), None)
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Whether to do something [Y/n]");
        buffer.clear();

        BoolInput::new(Some(true), Some("do other thing"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Whether to do other thing [Y/n]");
        buffer.clear();

        BoolInput::new(None, Some("do other thing"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Whether to do other thing [y/n]");
        buffer.clear();
    }

    #[test]
    fn prompt_no_default() {
        let mut buffer = Vec::new();

        BoolInput::new(None, None)
            .prompt_no_default(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Default value not set, please input y/n");
    }

    #[test]
    fn parse() {
        let bool_input = BoolInput::new(None, None);
        let mut output = Vec::new();
        for input in &["y", "Y", "yes", "Yes", "YES"] {
            assert!(bool_input.clone().parse(input, &mut output).unwrap())
        }

        for input in &["n", "N", "no", "No", "NO"] {
            assert!(!bool_input.clone().parse(input, &mut output).unwrap())
        }

        assert_eq!(
            bool_input
                .clone()
                .parse("invalid", &mut output)
                .unwrap_err()
                .unwrap(),
            bool_input.clone()
        );

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Invalid input, please input y/n",
        );
    }
}
