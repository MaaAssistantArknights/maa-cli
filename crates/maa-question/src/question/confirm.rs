use std::io::{self, Write};

use serde::Deserialize;

use super::CowStr;
use crate::{Question, resolver::io::PromptIo};

/// A confirmation question that asks the user for a yes/no answer.
///
/// When resolved through [`PromptIo`](crate::resolver::io::PromptIo), this renders as a
/// `[Y/n]` or `[y/N]` prompt.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confirm {
    /// Stable identifier for variable injection.
    id: Option<CowStr>,
    /// Default value.
    default: bool,
    /// Human-readable description shown in the prompt.
    description: Option<CowStr>,
}

impl Confirm {
    pub fn new(default: bool) -> Self {
        Self {
            id: None,
            default,
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<CowStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_id(mut self, id: impl Into<CowStr>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl Question for Confirm {
    type Answer = bool;

    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn default(self) -> Self::Answer {
        self.default
    }

    fn interpret(self, input: &str) -> Result<Self::Answer, (Self, String)> {
        match input {
            s if s.eq_ignore_ascii_case("y")
                || s.eq_ignore_ascii_case("yes")
                || s.eq_ignore_ascii_case("true") =>
            {
                Ok(true)
            }
            s if s.eq_ignore_ascii_case("n")
                || s.eq_ignore_ascii_case("no")
                || s.eq_ignore_ascii_case("false") =>
            {
                Ok(false)
            }
            _ => Err((self, format!("Invalid input \"{input}\""))),
        }
    }
}

impl PromptIo for Confirm {
    fn write_first_prefix(&self, writer: &mut dyn Write) -> io::Result<()> {
        write!(writer, "Whether to")
    }

    fn write_invalid_prefix(&self, writer: &mut dyn Write) -> io::Result<()> {
        write!(writer, "whether to")
    }

    fn write_description_to(&self, writer: &mut dyn Write) -> io::Result<()> {
        if let Some(description) = &self.description {
            write!(writer, " {description}")?;
        } else {
            write!(writer, " do something")?;
        }
        if self.default {
            write!(writer, " [Y/n]")
        } else {
            write!(writer, " [y/N]")
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_test::{Token, assert_de_tokens, assert_de_tokens_error};

    use super::*;
    use crate::resolver::io::*;

    #[test]
    fn serde() {
        let values = vec![
            Confirm::new(true).with_description("do something"),
            Confirm::new(false),
        ];

        assert_de_tokens(&values, &[
            Token::Seq { len: Some(2) },
            Token::Map { len: Some(2) },
            Token::Str("default"),
            Token::Bool(true),
            Token::Str("description"),
            Token::Some,
            Token::Str("do something"),
            Token::MapEnd,
            Token::Map { len: Some(1) },
            Token::Str("default"),
            Token::Bool(false),
            Token::MapEnd,
            Token::SeqEnd,
        ]);
    }

    #[test]
    fn empty_map_is_rejected() {
        assert_de_tokens_error::<Confirm>(
            &[Token::Map { len: Some(0) }, Token::MapEnd],
            "missing field `default`",
        );
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn construct() {
        let input = Confirm::new(true).with_description("do something");
        assert_eq!(input.default, true);
        assert_eq!(input.description.as_deref(), Some("do something"));
        assert_eq!(input.id, None);

        let input = Confirm::new(false).with_id("flag");
        assert_eq!(input.id.as_deref(), Some("flag"));
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn default() {
        assert_eq!(Confirm::new(true).default(), true);
        assert_eq!(Confirm::new(false).default(), false);
    }

    mod prompt_prefix_first {
        use super::*;

        #[test]
        fn always_returns_whether_to() {
            assert_prompt(&Confirm::new(true), "Whether to", |ui, w| {
                ui.write_first_prefix(w)
            });
        }
    }

    mod prompt_prefix_invalid {
        use super::*;

        #[test]
        fn includes_invalid_input_message() {
            let mut buffer: Vec<u8> = Vec::new();
            Confirm::new(true)
                .write_invalid_prefix(&mut buffer)
                .unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), "whether to");
        }
    }

    mod prompt_description {
        use super::*;

        #[test]
        fn with_default_true_no_description() {
            assert_prompt(&Confirm::new(true), " do something [Y/n]", |ui, w| {
                ui.write_description_to(w)
            });
        }

        #[test]
        fn with_default_true_and_description() {
            assert_prompt(
                &Confirm::new(true).with_description("do other thing"),
                " do other thing [Y/n]",
                |ui, w| ui.write_description_to(w),
            );
        }

        #[test]
        fn with_default_false_and_description() {
            assert_prompt(
                &Confirm::new(false).with_description("skip this"),
                " skip this [y/N]",
                |ui, w| ui.write_description_to(w),
            );
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn valid_yes_inputs() {
            let confirm = Confirm::new(true);
            for input in &["y", "Y", "yes", "Yes", "YES"] {
                assert_eq!(confirm.clone().interpret(input), Ok(true));
            }
        }

        #[test]
        fn valid_no_inputs() {
            let confirm = Confirm::new(true);
            for input in &["n", "N", "no", "No", "NO"] {
                assert_eq!(confirm.clone().interpret(input), Ok(false));
            }
        }

        #[test]
        fn invalid_input_returns_original() {
            let confirm = Confirm::new(true);
            match confirm.clone().interpret("invalid") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_input, msg)) => {
                    assert_eq!(returned_input, confirm);
                    assert_eq!(msg, "Invalid input \"invalid\"");
                }
            }
        }
    }

    mod ask {
        use super::*;

        #[test]
        fn empty_input_returns_default() {
            assert_output(
                Confirm::new(true).with_description("test"),
                "\n",
                "Whether to test [Y/n]: ",
                true,
            );
        }

        #[test]
        fn invalid_input_reprompts() {
            assert_output(
                Confirm::new(true).with_description("test"),
                "invalid\ny\n",
                "Whether to test [Y/n]: Invalid input \"invalid\"\n\
                 whether to test [Y/n]: ",
                true,
            );
        }
    }
}
