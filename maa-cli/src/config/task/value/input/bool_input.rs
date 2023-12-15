use super::UserInput;

use std::io::{Result, Write};

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
    pub fn new<S>(default: Option<bool>, description: Option<S>) -> Self
    where
        S: Into<String>,
    {
        Self {
            default,
            description: description.map(|s| s.into()),
        }
    }
}

impl UserInput for BoolInput {
    type Value = bool;

    fn default(&self) -> Option<Self::Value> {
        self.default
    }

    fn prompt(&self, mut writer: impl Write) -> Result<()> {
        write!(writer, "Whether to")?;
        if let Some(description) = &self.description {
            write!(writer, " {}", description)?;
        } else {
            write!(writer, " do something")?;
        }
        if let Some(default) = &self.default {
            if *default {
                write!(writer, " [Y/n]: ")?;
            } else {
                write!(writer, " [y/N]: ")?;
            }
        } else {
            write!(writer, " [y/n]: ")?;
        }
        Ok(())
    }

    fn prompt_no_default(&self, mut writer: impl Write) -> Result<()> {
        write!(writer, "Default value not set, please input y/n: ")
    }

    fn parse(&self, trimmed: &str) -> std::result::Result<Self::Value, String> {
        match trimmed {
            "y" | "Y" | "yes" | "Yes" | "YES" => Ok(true),
            "n" | "N" | "no" | "No" | "NO" => Ok(false),
            _ => Err(String::from("Invalid input, please input y/n: ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::assert_matches;

    use serde_test::{assert_de_tokens, Token};

    #[test]
    fn deserialize() {
        let values = vec![
            BoolInput::new(Some(true), Some("do something")),
            BoolInput::new::<&str>(Some(false), None),
            BoolInput::new(None, Some("do something")),
            BoolInput::new::<&str>(None, None),
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
            BoolInput::new::<&str>(Some(true), None),
            BoolInput {
                default: Some(true),
                description: None,
            }
        );

        assert_matches!(
            BoolInput::new::<&str>(None, Some("do something")),
            BoolInput {
                default: None,
                description: Some(description),
            } if description == "do something"
        );

        assert_matches!(
            BoolInput::new::<&str>(None, None),
            BoolInput {
                default: None,
                description: None,
            }
        );
    }

    #[test]
    fn default() {
        assert_matches!(
            BoolInput::new(Some(true), Some("do something")).default(),
            Some(true)
        );

        assert_matches!(
            BoolInput::new::<&str>(None, Some("do something")).default(),
            None
        );
    }

    #[test]
    fn prompt() {
        let mut buffer = Vec::new();

        BoolInput::new::<&str>(Some(true), None)
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Whether to do something [Y/n]: ");
        buffer.clear();

        BoolInput::new(Some(true), Some("do other thing"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Whether to do other thing [Y/n]: ");
        buffer.clear();

        let mut buffer = Vec::new();
        BoolInput::new::<&str>(None, Some("do other thing"))
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Whether to do other thing [y/n]: ");
        buffer.clear();
    }

    #[test]
    fn prompt_no_default() {
        let mut buffer = Vec::new();

        BoolInput::new::<&str>(None, None)
            .prompt_no_default(&mut buffer)
            .unwrap();
        assert_eq!(buffer, b"Default value not set, please input y/n: ");
    }

    #[test]
    fn parse() {
        let bool_input = BoolInput::new::<&str>(None, None);
        for input in &["y", "Y", "yes", "Yes", "YES"] {
            assert!(bool_input.parse(input).unwrap());
        }

        for input in &["n", "N", "no", "No", "NO"] {
            assert!(!bool_input.parse(input).unwrap());
        }

        assert_eq!(
            bool_input.parse("invalid").unwrap_err(),
            "Invalid input, please input y/n: "
        );
    }
}
