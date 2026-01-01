use std::{
    borrow::Cow,
    fmt::Display,
    io::{self, Write},
    str::FromStr,
};

use serde::Deserialize;

use super::{Outcome, UserInput};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
/// A generic struct that represents a user input that queries the user for input.
///
/// For example, `Input::<i64>::new(Some(0), Some("medicine to use"))` represents a user input
/// that queries the user for an integer input, with default value 0 and description "medicine to
/// use".
///
/// If you want to query a boolean input, use [`super::BoolInput`].
pub struct Input<F> {
    /// Default value for this input
    default: Option<F>,
    /// Description of this input
    description: Option<Cow<'static, str>>,
}

impl<F> Input<F> {
    pub fn new(default: Option<F>) -> Self {
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

impl<F: FromStr + Display + Clone> UserInput for Input<F> {
    type Value = F;

    fn default(self) -> Outcome<Self::Value, Self> {
        match self.default {
            Some(v) => Outcome::Value(v),
            None => Outcome::Original(self),
        }
    }

    fn prompt_prefix_first(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "Please input")
    }

    fn prompt_prefix_empty(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "Default value not set, please input")
    }

    fn prompt_prefix_invalid(&self, writer: &mut impl Write, msg: &str) -> io::Result<()> {
        write!(writer, "Invalid input \"{}\", please input", msg)
    }

    fn prompt_description(&self, writer: &mut impl Write) -> io::Result<()> {
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

    fn parse(self, input: &str) -> Outcome<Self::Value, (Self, Cow<'_, str>)> {
        match input.parse() {
            Ok(value) => Outcome::Value(value),
            Err(_) => Outcome::Original((self, Cow::Borrowed(input))),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_test::{Token, assert_de_tokens};

    use super::{super::*, *};

    #[test]
    fn serde() {
        let values: Vec<Input<i64>> = vec![
            Input::new(Some(0)).with_description("how many apples to eat"),
            Input::new(Some(0)),
            Input::new(None).with_description("how many bananas to eat"),
            Input::new(None),
        ];

        assert_de_tokens(&values, &[
            Token::Seq { len: Some(4) },
            Token::Map { len: Some(2) },
            Token::Str("default"),
            Token::Some,
            Token::I32(0),
            Token::Str("description"),
            Token::Some,
            Token::Str("how many apples to eat"),
            Token::MapEnd,
            Token::Map { len: Some(1) },
            Token::Str("default"),
            Token::Some,
            Token::I32(0),
            Token::MapEnd,
            Token::Map { len: Some(1) },
            Token::Str("description"),
            Token::Some,
            Token::Str("how many bananas to eat"),
            Token::MapEnd,
            Token::Map { len: Some(0) },
            Token::MapEnd,
            Token::SeqEnd,
        ]);
    }

    #[test]
    fn default() {
        assert_eq!(Input::new(Some(0)).default(), Outcome::Value(0));
        assert_eq!(
            Input::new(None::<i64>).default(),
            Outcome::Original(Input::new(None))
        );
    }

    mod prompt_prefix_first {
        use super::*;

        #[test]
        fn always_returns_please_input() {
            assert_prompt(
                &Input::<i64>::new(None),
                "Please input",
                UserInput::prompt_prefix_first,
            );
        }
    }

    mod prompt_prefix_empty {
        use super::*;

        #[test]
        fn always_returns_default_not_set() {
            assert_prompt(
                &Input::<i64>::new(None),
                "Default value not set, please input",
                UserInput::prompt_prefix_empty,
            );
        }
    }

    mod prompt_prefix_invalid {
        use super::*;

        #[test]
        fn includes_invalid_input_message() {
            let mut buffer = Vec::new();
            Input::<i64>::new(None)
                .prompt_prefix_invalid(&mut buffer, "abc")
                .unwrap();
            assert_eq!(
                String::from_utf8(buffer).unwrap(),
                "Invalid input \"abc\", please input"
            );
        }
    }

    mod prompt_description {
        use super::*;

        #[test]
        fn with_default_and_description() {
            assert_prompt(
                &Input::<i64>::new(Some(0)).with_description("how many apples to eat"),
                " how many apples to eat [default: 0]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn with_description_no_default() {
            assert_prompt(
                &Input::<i64>::new(None).with_description("how many bananas to eat"),
                " how many bananas to eat",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn with_default_no_description() {
            assert_prompt(
                &Input::<i64>::new(Some(0)),
                " a i64 [default: 0]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn no_default_no_description() {
            assert_prompt(
                &Input::<i64>::new(None),
                " a i64",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn string_with_default() {
            assert_prompt(
                &Input::<String>::new(Some("hello".to_string())).with_description("your name"),
                " your name [default: hello]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn float_with_default() {
            assert_prompt(
                &Input::<f32>::new(Some(2.7)).with_description("a number"),
                " a number [default: 2.7]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn negative_int_with_default() {
            assert_prompt(
                &Input::<i64>::new(Some(-42)).with_description("temperature"),
                " temperature [default: -42]",
                UserInput::prompt_description,
            );
        }

        #[test]
        fn empty_string_default() {
            assert_prompt(
                &Input::<String>::new(Some(String::new())).with_description("optional text"),
                " optional text [default: ]",
                UserInput::prompt_description,
            );
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn valid_integer() {
            let input = Input::<i64>::new(None);
            assert_eq!(input.clone().parse("42"), Outcome::Value(42));
            assert_eq!(input.clone().parse("0"), Outcome::Value(0));
            assert_eq!(input.clone().parse("-123"), Outcome::Value(-123));
        }

        #[test]
        fn invalid_integer_returns_original() {
            let input = Input::<i64>::new(Some(0));
            match input.clone().parse("abc") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_input, msg)) => {
                    assert_eq!(returned_input, input);
                    assert_eq!(msg, "abc");
                }
            }
        }

        #[test]
        fn valid_float() {
            let input = Input::<f64>::new(None);
            assert_eq!(input.clone().parse("2.14"), Outcome::Value(2.14));
            assert_eq!(input.clone().parse("-0.5"), Outcome::Value(-0.5));
        }

        #[test]
        fn string_always_parses() {
            let input = Input::<String>::new(None);
            assert_eq!(
                input.clone().parse("hello"),
                Outcome::Value("hello".to_string())
            );
            assert_eq!(input.clone().parse(""), Outcome::Value("".to_string()));
            assert_eq!(
                input.clone().parse("123"),
                Outcome::Value("123".to_string())
            );
        }

        #[test]
        fn overflow_returns_original() {
            let input = Input::<i8>::new(None);
            match input.clone().parse("999") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_input, msg)) => {
                    assert_eq!(returned_input, input);
                    assert_eq!(msg, "999");
                }
            }
        }
    }

    mod ask {
        use super::{super::super::assert_output, *};

        #[test]
        fn empty_input_without_default_reprompts() {
            assert_output(
                Input::<i64>::new(None).with_description("a number"),
                "\n42\n",
                "Please input a number: \
                Default value not set, please input a number: ",
                42,
            );
        }

        #[test]
        fn multiple_retries_until_valid() {
            assert_output(
                Input::<i64>::new(None).with_description("a number"),
                "abc\ndef\n123\n",
                "Please input a number: \
                 Invalid input \"abc\", please input a number: \
                 Invalid input \"def\", please input a number: ",
                123,
            );
        }
    }
}
