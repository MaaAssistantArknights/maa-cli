use std::{
    io::{self, BufRead, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{Error, Result};

// Use batch mode in tests by default to avoid blocking tests.
// This variable can also be change at runtime by cli argument
static BATCH_MODE: AtomicBool = AtomicBool::new(cfg!(test) || cfg!(feature = "default_batch_mode"));

pub fn enable_batch_mode() {
    BATCH_MODE.store(true, Ordering::Relaxed);
}

fn is_batch_mode() -> bool {
    BATCH_MODE.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Outcome<V, O> {
    Value(V),
    Original(O),
}

use Outcome::*;

pub trait UserInput: Sized {
    type Value: Sized;

    /// Get the value of this parameter from user input.
    ///
    /// If in batch mode, try to get the default value by calling `batch_default`.
    /// If not in batch mode, prompt user to input a value by calling `ask`,
    /// and return the value returned by `ask`.
    ///
    /// Errors:
    ///
    /// - If in batch mode and `default` returns `None`, return an io::Error with kind other.
    /// - If not in batch mode and `ask` returns an io::Error, return the error.
    fn value(self) -> Result<Self::Value> {
        if is_batch_mode() {
            if let Value(default) = self.default() {
                Ok(default)
            } else {
                Err(Error::NoDefaultInBatchMode)
            }
        } else {
            self.ask(&mut std::io::stdout(), &mut std::io::stdin().lock())
        }
    }

    /// Get the default value when user input is empty.
    ///
    /// If there is a default value, return it.
    /// If there is no default value, give back the ownership of self.
    fn default(self) -> Outcome<Self::Value, Self>;

    /// Prompt user to input a value for this parameter and return the value when success.
    fn ask(mut self, writer: &mut impl Write, reader: &mut impl BufRead) -> Result<Self::Value> {
        self.prompt_prefix_first(writer)?;
        self.prompt_description(writer)?;
        writer.write_all(b": ")?;
        writer.flush()?;
        let mut input = String::new();
        loop {
            reader.read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                match self.default() {
                    Value(value) => break Ok(value),
                    Original(orig) => {
                        self = orig;
                        self.prompt_prefix_empty(writer)?;
                        self.prompt_description(writer)?;
                        writer.write_all(b": ")?;
                        writer.flush()?;
                    }
                };
            } else {
                match self.parse(trimmed) {
                    Value(value) => break Ok(value),
                    Original((orig, msg)) => {
                        self = orig;
                        self.prompt_prefix_invalid(writer, msg.as_ref())?;
                        self.prompt_description(writer)?;
                        writer.write_all(b": ")?;
                        writer.flush()?;
                    }
                };
            }
            input.clear();
        }
    }

    /// The description of this user input
    ///
    /// It can be something like "a kind of fruit" or "a number between 1 and 10".
    fn prompt_description(&self, writer: &mut impl Write) -> io::Result<()>;

    /// The prefix of first time prompt
    ///
    /// It can be something like "Please input".
    fn prompt_prefix_first(&self, writer: &mut impl Write) -> io::Result<()>;

    /// The prefix of prompt when input is empty and no default value
    ///
    /// It can be something like "No default value, please input".
    fn prompt_prefix_empty(&self, writer: &mut impl Write) -> io::Result<()>;

    /// Prompt user to re-input a value when the input is invalid
    ///
    /// It can be something like "Invalid input, please input".
    fn prompt_prefix_invalid(&self, writer: &mut impl Write, msg: &str) -> io::Result<()>;

    /// Parse the user input
    ///
    /// If the input is valid, return the parsed value.
    /// If the input is invalid, give back the ownership and an error message.
    fn parse(self, input: &str) -> Outcome<Self::Value, (Self, std::borrow::Cow<'_, str>)>;
}

mod bool_input;
pub use bool_input::BoolInput;

mod input;
pub use input::Input;

mod select;
pub use select::{SelectD, Selectable, ValueWithDesc};

#[cfg(test)]
#[track_caller]
fn assert_prompt<I: UserInput>(
    ui: &I,
    expected: &str,
    prompt_fn: impl FnOnce(&I, &mut Vec<u8>) -> io::Result<()>,
) {
    let mut buffer = Vec::new();
    prompt_fn(ui, &mut buffer).unwrap();
    assert_eq!(String::from_utf8(buffer).unwrap(), expected);
}

#[cfg(test)]
#[track_caller]
fn assert_output<I, E>(ui: I, input: &str, expected_output: &str, expected_value: E)
where
    I: UserInput,
    I::Value: PartialEq + std::fmt::Debug + std::cmp::PartialEq<E>,
    E: std::fmt::Debug,
{
    let mut output = Vec::new();
    let mut input = io::BufReader::new(input.as_bytes());
    let value = ui.ask(&mut output, &mut input).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert_eq!(output, expected_output);
    assert_eq!(value, expected_value);
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::num::NonZero;

    use super::*;

    mod value {
        use super::*;

        mod bool_input {
            use super::*;

            #[test]
            fn with_default() {
                assert!(BoolInput::new(Some(true)).value().unwrap());
                assert!(!BoolInput::new(Some(false)).value().unwrap());
            }

            #[test]
            fn without_default() {
                assert!(matches!(
                    BoolInput::new(None).value().unwrap_err(),
                    crate::Error::NoDefaultInBatchMode
                ));
            }
        }

        mod input {
            use super::*;

            #[test]
            fn with_default() {
                assert_eq!(Input::<i64>::new(Some(1)).value().unwrap(), 1);
                assert_eq!(Input::<i64>::new(Some(-42)).value().unwrap(), -42);
                assert_eq!(
                    Input::<String>::new(Some("test".to_string()))
                        .value()
                        .unwrap(),
                    "test"
                );
            }

            #[test]
            fn without_default() {
                assert!(matches!(
                    Input::<i64>::new(None).value().unwrap_err(),
                    crate::Error::NoDefaultInBatchMode
                ));
            }
        }

        mod select {
            use super::*;

            #[test]
            fn with_explicit_default() {
                assert_eq!(
                    SelectD::<i32>::from_iter([1, 2], NonZero::new(2))
                        .unwrap()
                        .value()
                        .unwrap(),
                    2
                );
                assert_eq!(
                    SelectD::<i32>::from_iter([1, 2], NonZero::new(1))
                        .unwrap()
                        .value()
                        .unwrap(),
                    1
                );
            }

            #[test]
            fn with_implicit_default() {
                assert_eq!(
                    SelectD::<i32>::from_iter([1, 2], None)
                        .unwrap()
                        .value()
                        .unwrap(),
                    1
                );
            }
        }

        mod value_errors {
            use super::*;

            #[test]
            fn no_default_in_batch_mode_bool() {
                let result = BoolInput::new(None).value();
                assert!(matches!(result, Err(Error::NoDefaultInBatchMode)));
            }

            #[test]
            fn no_default_in_batch_mode_input() {
                let result = Input::<i32>::new(None).value();
                assert!(matches!(result, Err(Error::NoDefaultInBatchMode)));
            }
        }
    }

    mod ask {
        use super::*;

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
        fn empty_input_without_default_reprompts() {
            assert_output(
                Input::<i64>::new(None).with_description("number"),
                "\n42\n",
                "Please input number: Default value not set, please input number: ",
                42,
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

        #[test]
        fn multiple_retries_until_valid() {
            assert_output(
                Input::<i64>::new(None).with_description("number"),
                "abc\ndef\n123\n",
                "Please input number: \
                 Invalid input \"abc\", please input number: \
                 Invalid input \"def\", please input number: ",
                123,
            );
        }
    }
}
