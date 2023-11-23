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
        const INPUT_EMPTY: &[u8] = b"\n";

        #[test]
        fn bool_input() {
            let mut output = b"".to_vec();

            let value: BoolInput = BoolInput::new(Some(true), Some("fight"));
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Whether to fight [Y/n]: ");
            output.clear();

            let input_invalid = b"invalid\ny\n";
            assert!(value.ask(&mut output, &input_invalid[..]).unwrap());
            assert_eq!(&output, b"Invalid input, please input y/n: ");
            output.clear();
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(value.ask(&mut output, INPUT_EMPTY).unwrap());
            assert_eq!(&output, b"");

            let value: BoolInput = BoolInput::new(Some(false), Some("fight"));
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Whether to fight [y/N]: ");
            output.clear();
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(!value.ask(&mut output, INPUT_EMPTY).unwrap());

            let input_empty_then_yes = b"\ny\n";
            let value: BoolInput = BoolInput::new(None, Some("fight"));
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Whether to fight [y/n]: ");
            output.clear();
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(value.ask(&mut output, &input_empty_then_yes[..]).unwrap());
            assert_eq!(&output, b"Default value not set, please input y/n: ");
            output.clear();

            let value: BoolInput = BoolInput::new::<&str>(None, None);
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Whether to do something [y/n]: ");
            output.clear();
            assert!(value.ask(&mut output, INPUT_YES).unwrap());
            assert!(!value.ask(&mut output, INPUT_NO).unwrap());
            assert!(value.ask(&mut output, &input_empty_then_yes[..]).unwrap());
            assert_eq!(&output, b"Default value not set, please input y/n: ");
            output.clear();
        }

        #[test]
        fn input() {
            let value: Input<i64> = Input::new(Some(1), Some("a number"));
            let input = b"a\n2\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Please input a number [default: 1]: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 2);
            assert_eq!(&output, b"Invalid input, please try again: ");
        }

        #[test]
        fn input_empty() {
            let value: Input<i64> = Input::new(Some(1), Some("a number"));
            let input = b"\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Please input a number [default: 1]: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 1);
            assert_eq!(&output, b"");
        }

        #[test]
        fn input_no_default() {
            let value: Input<i64> = Input::new::<i64, &str>(None, Some("a number"));
            let input = b"\n2\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Please input a number: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 2);
            assert_eq!(&output, b"Default value not set, please input a value: ");
        }

        #[test]
        fn input_no_description() {
            let value: Input<i64> = Input::new::<i64, &str>(Some(1), None);
            let input = b"2\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Please input a i64 [default: 1]: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 2);
            assert_eq!(&output, b"");
        }

        #[test]
        fn input_empty_no_default() {
            let value: Input<i64> = Input::new::<i64, &str>(None, Some("a number"));
            let input = b"\n2\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"Please input a number: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 2);
            assert_eq!(&output, b"Default value not set, please input a value: ");
        }

        #[test]
        fn select() {
            let value: Select<char> = Select::new(vec!['A', 'B'], Some("a char"));
            let input = b"3\na\n2\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"1. A\n2. B\nPlease select a char: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 'B');
            assert_eq!(
                &output,
                b"Index out of range, must be between 1 and 2: Invalid input, please try again: "
            );
        }

        #[test]
        fn select_no_description() {
            let value: Select<char> = Select::new::<char, &str>(vec!['A', 'B'], None);
            let input = b"2\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"1. A\n2. B\nPlease select a char: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 'B');
            assert_eq!(&output, b"");
        }

        #[test]
        fn select_empty() {
            let value: Select<char> = Select::new(vec!['A', 'B'], Some("a char"));
            let input = b"\n2\n";
            let mut output = b"".to_vec();
            value.prompt(&mut output).unwrap();
            assert_eq!(&output, b"1. A\n2. B\nPlease select a char: ");
            output.clear();
            assert_eq!(value.ask(&mut output, &input[..]).unwrap(), 'B');
            assert_eq!(&output, b"Please select one of the alternatives: ");
        }
    }
}
