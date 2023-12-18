mod bool_input;
pub use bool_input::BoolInput;

mod input;
pub use input::Input;

mod select;
pub use select::{Select, SelectD, Selectable, ValueWithDesc};

use std::{
    io::{self, BufRead, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use serde::Serialize;

// Use batch mode in tests by default to avoid blocking tests.
// This variable can also be change at runtime by cli argument
static BATCH_MODE: AtomicBool = AtomicBool::new(cfg!(test));

pub fn enable_batch_mode() {
    BATCH_MODE.store(true, Ordering::Relaxed);
}

fn is_batch_mode() -> bool {
    BATCH_MODE.load(Ordering::Relaxed)
}

type Result<T, E = io::Error> = std::result::Result<T, E>;

pub trait UserInput {
    type Value;

    /// Get the value of this parameter from user input.
    ///
    /// If in batch mode, try to get the default value by calling `batch_default`.
    /// If `batch_default` returns `None`, panic.
    /// If not in batch mode, prompt user to input a value by calling `ask`,
    /// and return the value returned by `ask`.
    ///
    /// # Panics
    ///
    /// If `batch_default` returns `None` in batch mode.
    fn value(&self) -> io::Result<Self::Value> {
        if is_batch_mode() {
            self.batch_default().ok_or(io::Error::new(
                io::ErrorKind::Other,
                "can not get default value in batch mode",
            ))
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
    /// If this method returns `None`, `get` will return an io::Error with kind other
    /// in batch mod
    fn batch_default(&self) -> Option<Self::Value> {
        self.default()
    }

    /// Prompt user to input a value for this parameter and return the value when success.
    fn ask(&self, writer: impl Write, reader: impl BufRead) -> io::Result<Self::Value> {
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

pub fn serialize_userinput<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: UserInput,
    T::Value: Serialize,
{
    value
        .value()
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        assert!(BoolInput::new(Some(true), Some("")).value().unwrap());
        assert_eq!(
            BoolInput::new(None, Some("")).value().unwrap_err().kind(),
            io::ErrorKind::Other
        );

        assert_eq!(Input::<i64>::new(Some(1), Some("")).value().unwrap(), 1);
        assert_eq!(
            Input::<i64>::new(None::<i64>, Some(""))
                .value()
                .unwrap_err()
                .kind(),
            io::ErrorKind::Other
        );

        assert_eq!(
            SelectD::<i64>::new([1, 2], Some(2), Some(""), true)
                .value()
                .unwrap(),
            2
        );
        assert_eq!(
            SelectD::<i64>::new([1, 2], None, Some(""), true)
                .value()
                .unwrap(),
            1
        );
    }

    #[test]
    fn ask() {
        let mut output_buf = Vec::new();
        let mut output = io::BufWriter::new(&mut output_buf);

        // Test good input
        let bool_input = BoolInput::new(Some(true), Some("hello"));
        assert!(bool_input.ask(&mut output, b"\n".as_ref()).unwrap());
        assert!(bool_input.ask(&mut output, b" \n".as_ref()).unwrap());
        assert!(bool_input.ask(&mut output, b"y\n".as_ref()).unwrap());
        assert!(bool_input.ask(&mut output, b"y \n".as_ref()).unwrap());
        assert!(!bool_input.ask(&mut output, b"n\n".as_ref()).unwrap());
        assert!(!bool_input.ask(&mut output, b"n \n".as_ref()).unwrap());

        assert_eq!(
            String::from_utf8(output_buf).unwrap(),
            "Whether to hello [Y/n]: Whether to hello [Y/n]: Whether to hello [Y/n]: \
             Whether to hello [Y/n]: Whether to hello [Y/n]: Whether to hello [Y/n]: "
        );

        output_buf.clear();

        // Test empty input when default is not set
        let bool_input = BoolInput::new(None, Some("hello"));
        assert!(bool_input.ask(&mut output, b"\ny\n".as_ref()).unwrap());
        assert_eq!(
            String::from_utf8(output_buf).unwrap(),
            "Whether to hello [y/n]: Default value not set, please input y/n: "
        );
        output_buf.clear();

        // Test invalid input
        assert!(!bool_input
            .ask(&mut output, b"invalid\ny\n".as_ref())
            .unwrap());
        assert_eq!(
            String::from_utf8(output_buf).unwrap(),
            "Whether to hello [y/n]: Invalid input, please input y/n: "
        );
    }
}
