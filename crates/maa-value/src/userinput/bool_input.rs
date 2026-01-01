use std::{
    borrow::Cow,
    io::{self, Write},
};

use serde::Deserialize;

use super::{Outcome, UserInput};

/// A struct that represents a user input that queries the user for boolean input.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoolInput {
    /// Default value for this parameter.
    default: Option<bool>,
    /// Description of this parameter
    description: Option<Cow<'static, str>>,
}

impl BoolInput {
    pub fn new(default: Option<bool>) -> Self {
        Self {
            default,
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl UserInput for BoolInput {
    type Value = bool;

    fn default(self) -> Outcome<Self::Value, Self> {
        match self.default {
            Some(v) => Outcome::Value(v),
            None => Outcome::Original(self),
        }
    }

    fn prompt_prefix_first(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "Whether to")
    }

    fn prompt_prefix_empty(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "Default value not set, whether to")
    }

    fn prompt_prefix_invalid(&self, writer: &mut impl Write, msg: &str) -> io::Result<()> {
        write!(writer, "Invalid input \"{msg}\", whether to")
    }

    fn prompt_description(&self, writer: &mut impl Write) -> io::Result<()> {
        if let Some(description) = &self.description {
            write!(writer, " {description}")?;
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

    fn parse(self, input: &str) -> Outcome<Self::Value, (Self, Cow<'_, str>)> {
        match input {
            "y" | "Y" | "yes" | "Yes" | "YES" => Outcome::Value(true),
            "n" | "N" | "no" | "No" | "NO" => Outcome::Value(false),
            _ => Outcome::Original((self, Cow::Borrowed(input))),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_test::{Token, assert_de_tokens};

    use super::{super::assert_prompt, *};

    #[test]
    fn serde() {
        let values = vec![
            BoolInput::new(Some(true)).with_description("do something"),
            BoolInput::new(Some(false)),
            BoolInput::new(None).with_description("do something"),
            BoolInput::new(None),
        ];

        assert_de_tokens(&values, &[
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
        ]);
    }

    #[test]
    fn construct() {
        let input = BoolInput::new(Some(true)).with_description("do something");
        assert_eq!(input.default, Some(true));
        assert_eq!(input.description.as_deref(), Some("do something"));

        let input = BoolInput::new(Some(true));
        assert_eq!(input.default, Some(true));
        assert_eq!(input.description, None);

        let input = BoolInput::new(None).with_description("do something");
        assert_eq!(input.default, None);
        assert_eq!(input.description.as_deref(), Some("do something"));

        let input = BoolInput::new(None);
        assert_eq!(input.default, None);
        assert_eq!(input.description, None);
    }

    #[test]
    fn default() {
        assert_eq!(BoolInput::new(Some(true)).default(), Outcome::Value(true));

        assert_eq!(
            BoolInput::new(None).default(),
            Outcome::Original(BoolInput::new(None))
        );
    }

    mod prompt_prefix_first {
        use super::*;

        #[test]
        fn always_returns_whether_to() {
            assert_prompt(
                &BoolInput::new(None),
                "Whether to",
                UserInput::prompt_prefix_first,
            );
        }
    }

    mod prompt_prefix_empty {
        use super::*;

        #[test]
        fn always_returns_default_not_set() {
            assert_prompt(
                &BoolInput::new(None),
                "Default value not set, whether to",
                UserInput::prompt_prefix_empty,
            );
        }
    }

    mod prompt_prefix_invalid {
        use super::*;

        #[test]
        fn includes_invalid_input_message() {
            let mut buffer = Vec::new();
            BoolInput::new(None)
                .prompt_prefix_invalid(&mut buffer, "maybe")
                .unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                "Invalid input \"maybe\", whether to"
            );
        }
    }

    mod prompt_description {
        use super::*;

        #[test]
        fn with_default_true_no_description() {
            assert_prompt(
                &BoolInput::new(Some(true)),
                " do something [Y/n]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn with_default_true_and_description() {
            assert_prompt(
                &BoolInput::new(Some(true)).with_description("do other thing"),
                " do other thing [Y/n]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn with_default_false_and_description() {
            assert_prompt(
                &BoolInput::new(Some(false)).with_description("skip this"),
                " skip this [y/N]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn no_default_with_description() {
            assert_prompt(
                &BoolInput::new(None).with_description("do other thing"),
                " do other thing [y/n]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn no_default_no_description() {
            assert_prompt(
                &BoolInput::new(None),
                " do something [y/n]",
                UserInput::prompt_description,
            );
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn valid_yes_inputs() {
            let bool_input = BoolInput::new(None);
            for input in &["y", "Y", "yes", "Yes", "YES"] {
                assert_eq!(bool_input.clone().parse(input), Outcome::Value(true));
            }
        }

        #[test]
        fn valid_no_inputs() {
            let bool_input = BoolInput::new(None);
            for input in &["n", "N", "no", "No", "NO"] {
                assert_eq!(bool_input.clone().parse(input), Outcome::Value(false));
            }
        }

        #[test]
        fn invalid_input_returns_original() {
            let bool_input = BoolInput::new(None);
            match bool_input.clone().parse("invalid") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_input, msg)) => {
                    assert_eq!(returned_input, bool_input);
                    assert_eq!(msg, "invalid");
                }
            }
        }

        #[test]
        fn empty_string_is_invalid() {
            let bool_input = BoolInput::new(None);
            match bool_input.clone().parse("") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_input, msg)) => {
                    assert_eq!(returned_input, bool_input);
                    assert_eq!(msg, "");
                }
            }
        }

        #[test]
        fn partial_matches_are_invalid() {
            let bool_input = BoolInput::new(None);
            for input in &["ye", "no ", " yes", "N O"] {
                match bool_input.clone().parse(input) {
                    Outcome::Value(_) => panic!("Expected Original for '{input}', got Value"),
                    Outcome::Original(_) => {}
                }
            }
        }
    }

    mod ask {
        use super::{super::super::assert_output, *};

        #[test]
        fn empty_input_returns_default() {
            assert_output(
                BoolInput::new(Some(true)).with_description("test"),
                "\n",
                "Whether to test [Y/n]: ",
                true,
            );
        }

        #[test]
        fn invalid_input_reprompts() {
            assert_output(
                BoolInput::new(None).with_description("test"),
                "invalid\ny\n",
                "Whether to test [y/n]: Invalid input \"invalid\", whether to test [y/n]: ",
                true,
            );
        }
    }
}
