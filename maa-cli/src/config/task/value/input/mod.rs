mod bool_input;
pub use bool_input::BoolInput;

mod input;
pub use input::Input;

mod select;
pub use select::Select;

use std::{
    fmt::Display,
    io::{self, BufRead, Write},
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};

use serde::Deserialize;

// Use batch mode in tests by default to avoid blocking tests.
// This variable can also be change at runtime by cli argument
static BATCH_MODE: AtomicBool = AtomicBool::new(cfg!(test));

pub fn enable_batch_mode() {
    BATCH_MODE.store(true, Ordering::Relaxed);
}

fn is_batch_mode() -> bool {
    BATCH_MODE.load(Ordering::Relaxed)
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum InputOrSelect<F> {
    Input(Input<F>),
    Select(Select<F>),
}

impl<F: FromStr + Display> UserInput for InputOrSelect<F> {
    type Value = F;

    fn default(&self) -> Option<F> {
        match self {
            Self::Input(input) => input.default(),
            Self::Select(select) => select.default(),
        }
    }

    fn batch_default(&self) -> Option<F> {
        match self {
            Self::Input(input) => input.batch_default(),
            Self::Select(select) => select.batch_default(),
        }
    }

    fn prompt(&self, writer: impl Write) -> io::Result<()> {
        match self {
            Self::Input(input) => input.prompt(writer),
            Self::Select(select) => select.prompt(writer),
        }
    }

    fn prompt_no_default(&self, writer: impl Write) -> io::Result<()> {
        match self {
            Self::Input(input) => input.prompt_no_default(writer),
            Self::Select(select) => select.prompt_no_default(writer),
        }
    }

    fn parse(&self, input: &str) -> std::result::Result<Self::Value, String> {
        match self {
            Self::Input(input) => input.parse(input),
            Self::Select(select) => select.parse(input),
        }
    }
}

impl<F> From<Input<F>> for InputOrSelect<F> {
    fn from(input: Input<F>) -> Self {
        InputOrSelect::Input(input)
    }
}

impl<F> From<Select<F>> for InputOrSelect<F> {
    fn from(select: Select<F>) -> Self {
        InputOrSelect::Select(select)
    }
}

pub trait UserInput {
    type Value;

    fn get(&self) -> Result<Self::Value> {
        if is_batch_mode() {
            self.batch_default().ok_or(Error::DefaultNotSet)
        } else {
            let writer = std::io::stdout();
            let reader = std::io::stdin().lock();
            self.ask(writer, reader)
        }
    }

    /// Get the default value when user input is empty.
    ///
    /// If this method returns `None`, the user will be prompted to re-input a value.
    fn default(&self) -> Option<Self::Value>;

    /// The default value used in batch mode
    ///
    /// If this method returns `None`, `get` will return `Err(Error::DefaultNotSet)`.
    fn batch_default(&self) -> Option<Self::Value> {
        self.default()
    }

    fn ask(&self, writer: impl Write, reader: impl BufRead) -> Result<Self::Value> {
        self.prompt(writer)?;
        writer.flush()?;
        let mut input = String::new();
        loop {
            reader.read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                if let Some(default) = self.default() {
                    break Ok(default);
                } else {
                    self.prompt_no_default(writer)?;
                    writer.flush()?;
                }
            } else {
                match self.parse(trimmed) {
                    Ok(value) => break Ok(value),
                    Err(msg) => {
                        writer.write_all(msg.as_bytes())?;
                        writer.flush()?;
                    }
                };
            }
            input.clear();
        }
    }

    /// Prompt user to input a value for this parameter.
    ///
    /// Don't flush the writer after writing to it.
    /// The caller will flush it when necessary.
    fn prompt(&self, writer: impl Write) -> io::Result<()>;

    /// Prompt user to re-input a value for this parameter when the input is empty and default value is not set.
    ///
    /// Don't flush the writer after writing to it.
    /// The caller will flush it when necessary.
    fn prompt_no_default(&self, writer: impl Write) -> io::Result<()>;

    /// Function to parse a string to the value of this parameter.
    /// If the input is invalid, return an error message.
    fn parse(&self, input: &str) -> std::result::Result<Self::Value, String>;
}

#[derive(Debug)]
pub enum Error {
    DefaultNotSet,
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::DefaultNotSet => write!(f, "Failed to get default value"),
            Self::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

// This should based on locale, if we want to support other languages.
pub fn indefinite_article(s: &str) -> &'static str {
    match s.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        Some('A' | 'E' | 'I' | 'O' | 'U') => "an",
        _ => "a",
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     mod serde {
//         use super::*;
//
//         use serde_test::{assert_de_tokens, assert_ser_tokens, Token};
//
//         #[test]
//         fn input() {
//             let value: Vec<InputOrSelect<i64>> = vec![
//                 InputOrSelect::Input(Input::new(Some(1), Some("a number"))),
//                 InputOrSelect::Input(Input::new::<i64, &str>(Some(2), None)),
//                 InputOrSelect::Input(Input::new::<i64, &str>(None, Some("a number"))),
//                 InputOrSelect::Input(Input::new::<i64, &str>(None, None)),
//             ];
//
//             assert_de_tokens(
//                 &value,
//                 &[
//                     Token::Seq { len: Some(4) },
//                     Token::Map { len: Some(2) },
//                     Token::Str("default"),
//                     Token::I64(1),
//                     Token::Str("description"),
//                     Token::Str("a number"),
//                     Token::MapEnd,
//                     Token::Map { len: Some(1) },
//                     Token::Str("default"),
//                     Token::I64(2),
//                     Token::MapEnd,
//                     Token::Map { len: Some(1) },
//                     Token::Str("description"),
//                     Token::Str("a number"),
//                     Token::MapEnd,
//                     Token::Map { len: Some(0) },
//                     Token::MapEnd,
//                     Token::SeqEnd,
//                 ],
//             );
//
//             assert_ser_tokens(
//                 &value[..2],
//                 &[
//                     Token::Seq { len: Some(2) },
//                     Token::I64(1),
//                     Token::I64(2),
//                     Token::SeqEnd,
//                 ],
//             );
//         }
//
//         #[test]
//         fn select() {
//             let value: Vec<InputOrSelect<i64>> = vec![
//                 InputOrSelect::Select(Select::new(vec![1, 2], Some("a number"))),
//                 InputOrSelect::Select(Select::new::<i64, &str>(vec![3], None)),
//             ];
//
//             assert_de_tokens(
//                 &value,
//                 &[
//                     Token::Seq { len: Some(2) },
//                     Token::Map { len: Some(2) },
//                     Token::Str("alternatives"),
//                     Token::Seq { len: Some(2) },
//                     Token::I64(1),
//                     Token::I64(2),
//                     Token::SeqEnd,
//                     Token::Str("description"),
//                     Token::Str("a number"),
//                     Token::MapEnd,
//                     Token::Map { len: Some(1) },
//                     Token::Str("alternatives"),
//                     Token::Seq { len: Some(1) },
//                     Token::I64(3),
//                     Token::SeqEnd,
//                     Token::MapEnd,
//                     Token::SeqEnd,
//                 ],
//             );
//
//             assert_ser_tokens(
//                 &value,
//                 &[
//                     Token::Seq { len: Some(2) },
//                     Token::I64(1),
//                     Token::I64(3),
//                     Token::SeqEnd,
//                 ],
//             );
//         }
//     }
//
//     mod get {
//         use super::*;
//
//         use crate::assert_matches;
//
//         const INPUT_YES: &[u8] = b"y\n";
//         const INPUT_NO: &[u8] = b"n\n";
//         const INPUT_ONE: &[u8] = b"1\n";
//         const INPUT_TWO: &[u8] = b"2\n";
//         const INPUT_THREE: &[u8] = b"3\n";
//         const INPUT_EMPTY: &[u8] = b"\n";
//         const INPUT_INVALID: &[u8] = b"invalid\n";
//
//         macro_rules! combine_input {
//             ($input1:ident, $input2:ident) => {
//                 &[$input1, $input2].concat()[..]
//             };
//         }
//
//         macro_rules! assert_output_then_clear {
//             ($output:ident, $expected:expr) => {
//                 assert_eq!(&$output, $expected);
//                 $output.clear();
//             };
//             ($output:ident) => {
//                 assert_eq!(&$output, b"");
//             };
//         }
//
//         #[test]
//         fn bool_input() {
//             let mut output = b"".to_vec();
//             let input_empty_then_yes = combine_input!(INPUT_EMPTY, INPUT_YES);
//             let input_invalid = combine_input!(INPUT_INVALID, INPUT_YES);
//
//             let value: BoolInput = BoolInput::new(Some(true), Some("fight"));
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Whether to fight [Y/n]: ");
//             assert!(value.ask(&mut output, INPUT_YES).unwrap());
//             assert!(!value.ask(&mut output, INPUT_NO).unwrap());
//             assert!(value.ask(&mut output, INPUT_EMPTY).unwrap());
//             assert!(value.ask(&mut output, input_invalid).unwrap());
//             assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
//             assert!(value.get().unwrap());
//
//             let value: BoolInput = BoolInput::new(Some(false), Some("fight"));
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Whether to fight [y/N]: ");
//             assert!(value.ask(&mut output, INPUT_YES).unwrap());
//             assert!(!value.ask(&mut output, INPUT_NO).unwrap());
//             assert!(!value.ask(&mut output, INPUT_EMPTY).unwrap());
//             assert!(value.ask(&mut output, input_invalid).unwrap());
//             assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
//             assert!(!value.get().unwrap());
//
//             let value: BoolInput = BoolInput::new(None, Some("fight"));
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Whether to fight [y/n]: ");
//             assert!(value.ask(&mut output, INPUT_YES).unwrap());
//             assert!(!value.ask(&mut output, INPUT_NO).unwrap());
//             assert!(value.ask(&mut output, input_empty_then_yes).unwrap());
//             assert_output_then_clear!(output, b"Default value not set, please input y/n: ");
//             assert!(value.ask(&mut output, input_invalid).unwrap());
//             assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
//             assert_matches!(value.get(), Err(Error::DefaultNotSet));
//
//             let value: BoolInput = BoolInput::new::<&str>(None, None);
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Whether to do something [y/n]: ");
//             assert!(value.ask(&mut output, INPUT_YES).unwrap());
//             assert!(!value.ask(&mut output, INPUT_NO).unwrap());
//             assert!(value.ask(&mut output, input_empty_then_yes).unwrap());
//             assert_output_then_clear!(output, b"Default value not set, please input y/n: ");
//             assert!(value.ask(&mut output, input_invalid).unwrap());
//             assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
//             assert!(matches!(value.get(), Err(Error::DefaultNotSet)));
//
//             // test other valid inputs
//             let value: BoolInput = BoolInput::new::<&str>(None, None);
//             assert!(value.ask(&mut output, &b"y\n"[..]).unwrap());
//             assert!(value.ask(&mut output, &b"Y\n"[..]).unwrap());
//             assert!(value.ask(&mut output, &b"yes\n"[..]).unwrap());
//             assert!(value.ask(&mut output, &b"Yes\n"[..]).unwrap());
//             assert!(value.ask(&mut output, &b"YES\n"[..]).unwrap());
//             assert!(!value.ask(&mut output, &b"n\n"[..]).unwrap());
//             assert!(!value.ask(&mut output, &b"N\n"[..]).unwrap());
//             assert!(!value.ask(&mut output, &b"no\n"[..]).unwrap());
//             assert!(!value.ask(&mut output, &b"No\n"[..]).unwrap());
//             assert!(!value.ask(&mut output, &b"NO\n"[..]).unwrap());
//         }
//
//         #[test]
//         fn input() {
//             let mut output = b"".to_vec();
//             let input_empty_then_one = combine_input!(INPUT_EMPTY, INPUT_ONE);
//             let input_invalid_then_one = combine_input!(INPUT_INVALID, INPUT_ONE);
//
//             let value: Input<i64> = Input::new(Some(1), Some("a number"));
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Please input a number [default: 1]: ");
//             assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
//             assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
//             assert_matches!(value.ask(&mut output, INPUT_EMPTY), Ok(1));
//             assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok(1));
//             assert_output_then_clear!(output, b"Invalid input, please try again: ");
//             assert_matches!(value.get_default(), Ok(1));
//
//             let value: Input<i64> = Input::new::<i64, &str>(Some(1), None);
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Please input a i64 [default: 1]: ");
//             assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
//             assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
//             assert_matches!(value.ask(&mut output, INPUT_EMPTY), Ok(1));
//             assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok(1));
//             assert_output_then_clear!(output, b"Invalid input, please try again: ");
//             assert_matches!(value.get_default(), Ok(1));
//
//             let value: Input<i64> = Input::new::<i64, &str>(None, Some("a number"));
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Please input a number: ");
//             assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
//             assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
//             assert_matches!(value.ask(&mut output, input_empty_then_one), Ok(1));
//             assert_output_then_clear!(output, b"Default value not set, please input a value: ");
//             assert_matches!(value.get_default(), Err(Error::DefaultNotSet));
//
//             let value: Input<i64> = Input::new::<i64, &str>(None, None);
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"Please input a i64: ");
//             assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
//             assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
//             assert_matches!(value.ask(&mut output, input_empty_then_one), Ok(1));
//             assert_output_then_clear!(output, b"Default value not set, please input a value: ");
//             assert_matches!(value.get_default(), Err(Error::DefaultNotSet));
//         }
//
//         #[test]
//         fn select() {
//             let mut output = b"".to_vec();
//             let input_empty_then_one = combine_input!(INPUT_EMPTY, INPUT_ONE);
//             let input_invalid_then_one = combine_input!(INPUT_INVALID, INPUT_ONE);
//             let input_out_of_range_then_one = combine_input!(INPUT_THREE, INPUT_ONE);
//
//             let value: Select<char> = Select::new(vec!['A', 'B'], Some("an option"));
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"1. A\n2. B\nPlease select an option: ");
//             assert_matches!(value.ask(&mut output, INPUT_ONE), Ok('A'));
//             assert_matches!(value.ask(&mut output, INPUT_TWO), Ok('B'));
//             assert_matches!(value.ask(&mut output, input_empty_then_one), Ok('A'));
//             assert_output_then_clear!(output, b"Please select one of the alternatives: ");
//             assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok('A'));
//             assert_output_then_clear!(output, b"Invalid input, please try again: ");
//             assert_matches!(value.ask(&mut output, input_out_of_range_then_one), Ok('A'));
//             assert_output_then_clear!(output, b"Index out of range, must be between 1 and 2: ");
//             assert_matches!(value.get_first(), Ok('A'));
//
//             let value: Select<char> = Select::new::<char, &str>(vec!['A', 'B'], None);
//             value.prompt(&mut output).unwrap();
//             assert_output_then_clear!(output, b"1. A\n2. B\nPlease select a char: ");
//             assert_matches!(value.ask(&mut output, INPUT_ONE), Ok('A'));
//             assert_matches!(value.ask(&mut output, INPUT_TWO), Ok('B'));
//             assert_matches!(value.ask(&mut output, input_empty_then_one), Ok('A'));
//             assert_output_then_clear!(output, b"Please select one of the alternatives: ");
//             assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok('A'));
//             assert_output_then_clear!(output, b"Invalid input, please try again: ");
//             assert_matches!(value.ask(&mut output, input_out_of_range_then_one), Ok('A'));
//             assert_output_then_clear!(output, b"Index out of range, must be between 1 and 2: ");
//
//             let value: Select<char> = Select::new::<char, &str>(vec![], Some("a char"));
//             assert_matches!(value.get_first(), Err(Error::DefaultNotSet));
//         }
//     }
// }
