use std::{
    fmt::Display,
    io::{BufRead, Write},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

// Use batch mode in tests by default to avoid blocking tests.
// This variable can also be change at runtime by cli argument
static mut BATCH_MODE: bool = cfg!(test);

pub unsafe fn enable_batch_mode() {
    BATCH_MODE = true;
}

/// A struct that represents a user input that queries the user for boolean input.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BoolInput {
    /// Default value for this parameter.
    pub default: Option<bool>,
    /// Description of this parameter
    pub description: Option<String>,
}

impl Serialize for BoolInput {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.get()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
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

    pub fn get(&self) -> Result<bool> {
        if unsafe { BATCH_MODE } {
            // In batch mode, we use default value and do not ask user for input.
            self.default.ok_or(Error::DefaultNotSet)
        } else {
            let writer = std::io::stdout();
            let reader = std::io::stdin().lock();
            self.prompt(&writer)?;
            self.ask(writer, reader)
        }
    }

    pub fn prompt(&self, mut writer: impl Write) -> Result<()> {
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
        writer.flush()?;
        Ok(())
    }

    /// Ask user to input a value for this parameter.
    pub fn ask(&self, mut writer: impl Write, mut reader: impl BufRead) -> Result<bool> {
        let mut input = String::new();
        loop {
            reader.read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                if let Some(default) = self.default {
                    break Ok(default);
                } else {
                    write!(writer, "Default value not set, please input y/n: ")?;
                    writer.flush()?;
                }
            } else {
                match trimmed {
                    "y" | "Y" | "yes" | "Yes" | "YES" => break Ok(true),
                    "n" | "N" | "no" | "No" | "NO" => break Ok(false),
                    _ => {
                        write!(writer, "Invalid input, please input y/n: ")?;
                        writer.flush()?;
                    }
                };
            }
            input.clear();
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum UserInput<F> {
    Input(Input<F>),
    Select(Select<F>),
}

impl<F: FromStr + Clone + Display> UserInput<F> {
    pub fn get(&self) -> Result<F> {
        if unsafe { BATCH_MODE } {
            match self {
                Self::Input(i) => i.get_default(),
                Self::Select(s) => s.get_first(),
            }
        } else {
            let writer = std::io::stdout();
            let reader = std::io::stdin().lock();
            match self {
                Self::Input(i) => {
                    i.prompt(&writer)?;
                    i.ask(&writer, reader)
                }
                Self::Select(s) => {
                    s.prompt(&writer)?;
                    s.ask(writer, reader)
                }
            }
        }
    }
}

impl<F> Serialize for UserInput<F>
where
    F: Serialize + FromStr + Clone + Display,
{
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.get()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Input<F> {
    /// Default value for this parameter.
    pub default: Option<F>,
    /// Description of this parameter
    pub description: Option<String>,
}

impl<F: FromStr + Clone + Display> Input<F> {
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

    pub fn get_default(&self) -> Result<F> {
        self.default.clone().ok_or(Error::DefaultNotSet)
    }

    pub fn prompt(&self, mut writer: impl Write) -> Result<()> {
        write!(writer, "Please input")?;
        if let Some(description) = &self.description {
            write!(writer, " {}", description)?;
        } else {
            write!(writer, " a {}", std::any::type_name::<F>())?;
        }
        if let Some(default) = &self.default {
            write!(writer, " [default: {}]", default)?;
        }
        write!(writer, ": ")?;
        writer.flush()?;
        Ok(())
    }

    /// Ask user to input a value for this parameter.
    pub fn ask(&self, mut writer: impl Write, mut reader: impl BufRead) -> Result<F> {
        let mut input = String::new();
        loop {
            reader.read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                if let Some(default) = &self.default {
                    break Ok(default.clone());
                } else {
                    write!(writer, "Default value not set, please input a value: ")?;
                    writer.flush()?;
                }
            } else {
                match trimmed.parse() {
                    Ok(value) => break Ok(value),
                    Err(_) => {
                        write!(writer, "Invalid input, please try again: ")?;
                        writer.flush()?;
                    }
                };
            }
            input.clear();
        }
    }
}

impl<F> From<Input<F>> for UserInput<F> {
    fn from(input: Input<F>) -> Self {
        UserInput::Input(input)
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Select<F> {
    /// Alternatives for this parameter
    pub alternatives: Vec<F>,
    /// Description of this parameter
    pub description: Option<String>,
}

impl<F: FromStr + Clone + Display> Select<F> {
    pub fn new<I, S>(alternatives: Vec<I>, description: Option<S>) -> Self
    where
        I: Into<F>,
        S: Into<String>,
    {
        Self {
            alternatives: alternatives.into_iter().map(|i| i.into()).collect(),
            description: description.map(|s| s.into()),
        }
    }

    pub fn get_first(&self) -> Result<F> {
        self.alternatives
            .get(0)
            .cloned()
            .ok_or(Error::DefaultNotSet)
    }

    pub fn prompt(&self, mut writer: impl Write) -> Result<()> {
        for (i, alternative) in self.alternatives.iter().enumerate() {
            writeln!(writer, "{}. {}", i + 1, alternative)?;
        }
        write!(writer, "Please select")?;
        if let Some(description) = &self.description {
            write!(writer, " {}", description)?;
        } else {
            write!(writer, " a {}", std::any::type_name::<F>())?;
        }
        write!(writer, ": ")?;
        writer.flush()?;
        Ok(())
    }

    pub fn ask(&self, mut writer: impl Write, mut reader: impl BufRead) -> Result<F> {
        let mut input = String::new();
        loop {
            reader.read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                write!(writer, "Please select one of the alternatives: ")?;
                writer.flush()?;
            } else {
                match trimmed.parse::<usize>() {
                    Ok(value) => {
                        if value > 0 && value <= self.alternatives.len() {
                            break Ok(self.alternatives[value - 1].clone());
                        } else {
                            write!(
                                writer,
                                "Index out of range, must be between 1 and {}: ",
                                self.alternatives.len()
                            )?;
                            writer.flush()?;
                        }
                    }
                    Err(_) => {
                        write!(writer, "Invalid input, please try again: ")?;
                        writer.flush()?;
                    }
                };
            }
            input.clear();
        }
    }
}

impl<F> From<Select<F>> for UserInput<F> {
    fn from(select: Select<F>) -> Self {
        UserInput::Select(select)
    }
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

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    mod serde {
        use super::*;

        use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

        #[test]
        fn bool_input() {
            let value: Vec<BoolInput> = vec![
                BoolInput::new(Some(true), Some("do something")),
                BoolInput::new::<&str>(Some(false), None),
                BoolInput::new(None, Some("do something")),
                BoolInput::new::<&str>(None, None),
            ];

            assert_de_tokens(
                &value,
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

            assert_ser_tokens(
                &value[..2],
                &[
                    Token::Seq { len: Some(2) },
                    Token::Bool(true),
                    Token::Bool(false),
                    Token::SeqEnd,
                ],
            );
        }

        #[test]
        fn input() {
            let value: Vec<UserInput<i64>> = vec![
                UserInput::Input(Input::new(Some(1), Some("a number"))),
                UserInput::Input(Input::new::<i64, &str>(Some(2), None)),
                UserInput::Input(Input::new::<i64, &str>(None, Some("a number"))),
                UserInput::Input(Input::new::<i64, &str>(None, None)),
            ];

            assert_de_tokens(
                &value,
                &[
                    Token::Seq { len: Some(4) },
                    Token::Map { len: Some(2) },
                    Token::Str("default"),
                    Token::I64(1),
                    Token::Str("description"),
                    Token::Str("a number"),
                    Token::MapEnd,
                    Token::Map { len: Some(1) },
                    Token::Str("default"),
                    Token::I64(2),
                    Token::MapEnd,
                    Token::Map { len: Some(1) },
                    Token::Str("description"),
                    Token::Str("a number"),
                    Token::MapEnd,
                    Token::Map { len: Some(0) },
                    Token::MapEnd,
                    Token::SeqEnd,
                ],
            );

            assert_ser_tokens(
                &value[..2],
                &[
                    Token::Seq { len: Some(2) },
                    Token::I64(1),
                    Token::I64(2),
                    Token::SeqEnd,
                ],
            );
        }

        #[test]
        fn select() {
            let value: Vec<UserInput<i64>> = vec![
                UserInput::Select(Select::new(vec![1, 2], Some("a number"))),
                UserInput::Select(Select::new::<i64, &str>(vec![3], None)),
            ];

            assert_de_tokens(
                &value,
                &[
                    Token::Seq { len: Some(2) },
                    Token::Map { len: Some(2) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(2) },
                    Token::I64(1),
                    Token::I64(2),
                    Token::SeqEnd,
                    Token::Str("description"),
                    Token::Str("a number"),
                    Token::MapEnd,
                    Token::Map { len: Some(1) },
                    Token::Str("alternatives"),
                    Token::Seq { len: Some(1) },
                    Token::I64(3),
                    Token::SeqEnd,
                    Token::MapEnd,
                    Token::SeqEnd,
                ],
            );

            assert_ser_tokens(
                &value,
                &[
                    Token::Seq { len: Some(2) },
                    Token::I64(1),
                    Token::I64(3),
                    Token::SeqEnd,
                ],
            );
        }
    }

    mod get {
        use super::*;

        const INPUT_YES: &[u8] = b"y\n";
        const INPUT_NO: &[u8] = b"n\n";
        const INPUT_ONE: &[u8] = b"1\n";
        const INPUT_TWO: &[u8] = b"2\n";
        const INPUT_THREE: &[u8] = b"3\n";
        const INPUT_EMPTY: &[u8] = b"\n";
        const INPUT_INVALID: &[u8] = b"invalid\n";

        macro_rules! combine_input {
            ($input1:ident, $input2:ident) => {
                &[$input1, $input2].concat()[..]
            };
        }

        macro_rules! assert_output_then_clear {
            ($output:ident, $expected:expr) => {
                assert_eq!(&$output, $expected);
                $output.clear();
            };
            ($output:ident) => {
                assert_eq!(&$output, b"");
            };
        }

        #[macro_export]
        macro_rules! assert_matches {
            ($value:expr, $pattern:pat) => {
                assert!(matches!($value, $pattern));
            };
        }

        #[test]
        fn bool_input() {
            let mut output = b"".to_vec();
            let input_empty_then_yes = combine_input!(INPUT_EMPTY, INPUT_YES);
            let input_invalid = combine_input!(INPUT_INVALID, INPUT_YES);

            let value: BoolInput = BoolInput::new(Some(true), Some("fight"));
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Whether to fight [Y/n]: ");
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(value.ask(&mut output, INPUT_EMPTY).unwrap());
            assert!(value.ask(&mut output, input_invalid).unwrap());
            assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
            assert!(value.get().unwrap());

            let value: BoolInput = BoolInput::new(Some(false), Some("fight"));
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Whether to fight [y/N]: ");
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(!value.ask(&mut output, INPUT_EMPTY).unwrap());
            assert!(value.ask(&mut output, input_invalid).unwrap());
            assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
            assert!(!value.get().unwrap());

            let value: BoolInput = BoolInput::new(None, Some("fight"));
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Whether to fight [y/n]: ");
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(value.ask(&mut output, input_empty_then_yes).unwrap());
            assert_output_then_clear!(output, b"Default value not set, please input y/n: ");
            assert!(value.ask(&mut output, input_invalid).unwrap());
            assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
            assert_matches!(value.get(), Err(Error::DefaultNotSet));

            let value: BoolInput = BoolInput::new::<&str>(None, None);
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Whether to do something [y/n]: ");
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(value.ask(&mut output, input_empty_then_yes).unwrap());
            assert_output_then_clear!(output, b"Default value not set, please input y/n: ");
            assert!(value.ask(&mut output, input_invalid).unwrap());
            assert_output_then_clear!(output, b"Invalid input, please input y/n: ");
            assert!(matches!(value.get(), Err(Error::DefaultNotSet)));

            // test other valid inputs
            let value: BoolInput = BoolInput::new::<&str>(None, None);
            assert!(value.ask(&mut output, &b"y\n"[..]).unwrap());
            assert!(value.ask(&mut output, &b"Y\n"[..]).unwrap());
            assert!(value.ask(&mut output, &b"yes\n"[..]).unwrap());
            assert!(value.ask(&mut output, &b"Yes\n"[..]).unwrap());
            assert!(value.ask(&mut output, &b"YES\n"[..]).unwrap());
            assert!(!value.ask(&mut output, &b"n\n"[..]).unwrap());
            assert!(!value.ask(&mut output, &b"N\n"[..]).unwrap());
            assert!(!value.ask(&mut output, &b"no\n"[..]).unwrap());
            assert!(!value.ask(&mut output, &b"No\n"[..]).unwrap());
            assert!(!value.ask(&mut output, &b"NO\n"[..]).unwrap());
        }

        #[test]
        fn input() {
            let mut output = b"".to_vec();
            let input_empty_then_one = combine_input!(INPUT_EMPTY, INPUT_ONE);
            let input_invalid_then_one = combine_input!(INPUT_INVALID, INPUT_ONE);

            let value: Input<i64> = Input::new(Some(1), Some("a number"));
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Please input a number [default: 1]: ");
            assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
            assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
            assert_matches!(value.ask(&mut output, INPUT_EMPTY), Ok(1));
            assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok(1));
            assert_output_then_clear!(output, b"Invalid input, please try again: ");
            assert_matches!(value.get_default(), Ok(1));

            let value: Input<i64> = Input::new::<i64, &str>(Some(1), None);
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Please input a i64 [default: 1]: ");
            assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
            assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
            assert_matches!(value.ask(&mut output, INPUT_EMPTY), Ok(1));
            assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok(1));
            assert_output_then_clear!(output, b"Invalid input, please try again: ");
            assert_matches!(value.get_default(), Ok(1));

            let value: Input<i64> = Input::new::<i64, &str>(None, Some("a number"));
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Please input a number: ");
            assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
            assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
            assert_matches!(value.ask(&mut output, input_empty_then_one), Ok(1));
            assert_output_then_clear!(output, b"Default value not set, please input a value: ");
            assert_matches!(value.get_default(), Err(Error::DefaultNotSet));

            let value: Input<i64> = Input::new::<i64, &str>(None, None);
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"Please input a i64: ");
            assert_matches!(value.ask(&mut output, INPUT_ONE), Ok(1));
            assert_matches!(value.ask(&mut output, INPUT_TWO), Ok(2));
            assert_matches!(value.ask(&mut output, input_empty_then_one), Ok(1));
            assert_output_then_clear!(output, b"Default value not set, please input a value: ");
            assert_matches!(value.get_default(), Err(Error::DefaultNotSet));
        }

        #[test]
        fn select() {
            let mut output = b"".to_vec();
            let input_empty_then_one = combine_input!(INPUT_EMPTY, INPUT_ONE);
            let input_invalid_then_one = combine_input!(INPUT_INVALID, INPUT_ONE);
            let input_out_of_range_then_one = combine_input!(INPUT_THREE, INPUT_ONE);

            let value: Select<char> = Select::new(vec!['A', 'B'], Some("an option"));
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"1. A\n2. B\nPlease select an option: ");
            assert_matches!(value.ask(&mut output, INPUT_ONE), Ok('A'));
            assert_matches!(value.ask(&mut output, INPUT_TWO), Ok('B'));
            assert_matches!(value.ask(&mut output, input_empty_then_one), Ok('A'));
            assert_output_then_clear!(output, b"Please select one of the alternatives: ");
            assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok('A'));
            assert_output_then_clear!(output, b"Invalid input, please try again: ");
            assert_matches!(value.ask(&mut output, input_out_of_range_then_one), Ok('A'));
            assert_output_then_clear!(output, b"Index out of range, must be between 1 and 2: ");
            assert_matches!(value.get_first(), Ok('A'));

            let value: Select<char> = Select::new::<char, &str>(vec!['A', 'B'], None);
            value.prompt(&mut output).unwrap();
            assert_output_then_clear!(output, b"1. A\n2. B\nPlease select a char: ");
            assert_matches!(value.ask(&mut output, INPUT_ONE), Ok('A'));
            assert_matches!(value.ask(&mut output, INPUT_TWO), Ok('B'));
            assert_matches!(value.ask(&mut output, input_empty_then_one), Ok('A'));
            assert_output_then_clear!(output, b"Please select one of the alternatives: ");
            assert_matches!(value.ask(&mut output, input_invalid_then_one), Ok('A'));
            assert_output_then_clear!(output, b"Invalid input, please try again: ");
            assert_matches!(value.ask(&mut output, input_out_of_range_then_one), Ok('A'));
            assert_output_then_clear!(output, b"Index out of range, must be between 1 and 2: ");

            let value: Select<char> = Select::new::<char, &str>(vec![], Some("a char"));
            assert_matches!(value.get_first(), Err(Error::DefaultNotSet));
        }
    }
}
