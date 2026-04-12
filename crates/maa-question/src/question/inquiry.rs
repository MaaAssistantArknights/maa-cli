use std::{
    fmt::Display,
    io::{self, Write},
    str::FromStr,
};

use serde::Deserialize;

use super::CowStr;
use crate::{Question, resolver::io::PromptIo};

/// An open-ended question where the user types a free-form answer.
///
/// The answer string is parsed into `F` via [`FromStr`].  For a yes/no
/// confirmation use [`super::Confirm`] instead.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inquiry<F> {
    /// Stable identifier for variable injection.
    id: Option<CowStr>,
    /// Default value.
    default: F,
    /// Human-readable description shown in the prompt.
    description: Option<CowStr>,
}

impl<F> Inquiry<F> {
    pub fn new(default: F) -> Self {
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

impl<F: FromStr> Question for Inquiry<F>
where
    <F as std::str::FromStr>::Err: std::fmt::Display,
{
    type Answer = F;

    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn default(self) -> Self::Answer {
        self.default
    }

    fn interpret(self, input: &str) -> Result<Self::Answer, (Self, String)> {
        input
            .parse()
            .map_err(|e| (self, format!("Failed to parse input: `{e}`")))
    }
}

impl<F: FromStr + Display> PromptIo for Inquiry<F>
where
    <F as std::str::FromStr>::Err: std::fmt::Display,
{
    fn write_first_prefix(&self, writer: &mut dyn Write) -> io::Result<()> {
        write!(writer, "Please input")
    }

    fn write_invalid_prefix(&self, writer: &mut dyn Write) -> io::Result<()> {
        write!(writer, "please input")
    }

    fn write_description_to(&self, writer: &mut dyn Write) -> io::Result<()> {
        if let Some(description) = self.description.as_deref() {
            write!(writer, " {description}")?;
        } else if let Some(id) = self.id.as_deref() {
            write!(writer, " {id}")?;
        } else {
            write!(writer, " a {}", std::any::type_name::<F>())?;
        }
        write!(writer, " [default: {}]", self.default)
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
        let values: Vec<Inquiry<i64>> = vec![
            Inquiry::new(0).with_description("how many apples to eat"),
            Inquiry::new(0),
        ];

        assert_de_tokens(&values, &[
            Token::Seq { len: Some(2) },
            Token::Map { len: Some(2) },
            Token::Str("default"),
            Token::I32(0),
            Token::Str("description"),
            Token::Some,
            Token::Str("how many apples to eat"),
            Token::MapEnd,
            Token::Map { len: Some(1) },
            Token::Str("default"),
            Token::I32(0),
            Token::MapEnd,
            Token::SeqEnd,
        ]);
    }

    #[test]
    fn empty_map_is_rejected() {
        assert_de_tokens_error::<Inquiry<i64>>(
            &[Token::Map { len: Some(0) }, Token::MapEnd],
            "missing field `default`",
        );
    }

    #[test]
    fn construct() {
        let input = Inquiry::<i64>::new(0).with_id("count");
        assert_eq!(input.id.as_deref(), Some("count"));

        let input = Inquiry::<i64>::new(0).with_description("a number");
        assert_eq!(input.description.as_deref(), Some("a number"));
    }

    #[test]
    fn default() {
        assert_eq!(Inquiry::new(0).default(), 0);
    }

    mod prompt_prefix_first {
        use super::*;

        #[test]
        fn always_returns_please_input() {
            assert_prompt(&Inquiry::<i64>::new(0), "Please input", |ui, w| {
                ui.write_first_prefix(w)
            });
        }
    }

    mod prompt_prefix_invalid {
        use super::*;

        #[test]
        fn includes_invalid_input_message() {
            let mut buffer: Vec<u8> = Vec::new();
            Inquiry::<i64>::new(0)
                .write_invalid_prefix(&mut buffer as &mut dyn Write)
                .unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), "please input");
        }
    }

    mod prompt_description {
        use super::*;

        #[test]
        fn with_default_and_description() {
            assert_prompt(
                &Inquiry::<i64>::new(0).with_description("how many apples to eat"),
                " how many apples to eat [default: 0]",
                |ui, w| ui.write_description_to(w),
            );
        }

        #[test]
        fn with_default_no_description() {
            assert_prompt(&Inquiry::<i64>::new(0), " a i64 [default: 0]", |ui, w| {
                ui.write_description_to(w)
            });
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn valid_integer() {
            let input = Inquiry::<i64>::new(0);
            assert_eq!(input.clone().interpret("42"), Ok(42));
            assert_eq!(input.clone().interpret("0"), Ok(0));
            assert_eq!(input.clone().interpret("-123"), Ok(-123));
        }

        #[test]
        fn invalid_integer_returns_original() {
            let input = Inquiry::<i64>::new(0);
            match input.clone().interpret("abc") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_input, msg)) => {
                    assert_eq!(returned_input, input);
                    assert_eq!(
                        msg,
                        "Failed to parse input: `invalid digit found in string`"
                    );
                }
            }
        }

        #[test]
        fn string_always_parses() {
            let input = Inquiry::new("".to_string());
            assert_eq!(input.clone().interpret("hello"), Ok("hello".to_string()));
        }
    }

    mod ask {
        use super::*;

        #[test]
        fn empty_input_returns_default() {
            assert_output(
                Inquiry::new(0).with_description("a number"),
                "\n",
                "Please input a number [default: 0]: ",
                0,
            );
        }

        #[test]
        fn multiple_retries_until_valid() {
            assert_output(
                Inquiry::new(0).with_description("a number"),
                "abc\ndef\n123\n",
                "Please input a number [default: 0]: \
                 Failed to parse input: `invalid digit found in string`\n\
                 please input a number [default: 0]: \
                 Failed to parse input: `invalid digit found in string`\n\
                 please input a number [default: 0]: ",
                123,
            );
        }
    }
}
