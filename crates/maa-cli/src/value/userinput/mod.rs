use std::{
    io::{self, BufRead, Write},
    sync::atomic::{AtomicBool, Ordering},
};

// Use batch mode in tests by default to avoid blocking tests.
// This variable can also be change at runtime by cli argument
static BATCH_MODE: AtomicBool = AtomicBool::new(cfg!(test));

pub fn enable_batch_mode() {
    BATCH_MODE.store(true, Ordering::Relaxed);
}

fn is_batch_mode() -> bool {
    BATCH_MODE.load(Ordering::Relaxed)
}

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
    /// - If in batch mode and `batch_default` returns `None`, return an io::Error with kind other.
    /// - If not in batch mode and `ask` returns an io::Error, return the error.
    fn value(self) -> io::Result<Self::Value> {
        if is_batch_mode() {
            self.batch_default()
                .map_err(|_| io::Error::other("can not get default value in batch mode"))
        } else {
            self.ask(&mut std::io::stdout(), &mut std::io::stdin().lock())
        }
    }

    /// Get the default value when user input is empty.
    ///
    /// If there is a default value, return it.
    /// If there is no default value, give back the ownership of self.
    fn default(self) -> Result<Self::Value, Self>;

    /// The default value used in batch mode
    ///
    /// Fall back to `default` if not implemented.
    fn batch_default(self) -> Result<Self::Value, Self> {
        self.default()
    }

    /// Prompt user to input a value for this parameter and return the value when success.
    fn ask(self, writer: &mut impl Write, reader: &mut impl BufRead) -> io::Result<Self::Value> {
        self.prompt(writer)?;
        writer.write_all(b": ")?;
        writer.flush()?;
        let mut input = String::new();
        let mut self_mut = self;
        loop {
            reader.read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                match self_mut.default() {
                    Ok(value) => break Ok(value),
                    Err(self_) => {
                        self_mut = self_;
                        self_mut.prompt_no_default(writer)?;
                        writer.write_all(b": ")?;
                        writer.flush()?;
                    }
                };
            } else {
                match self_mut.parse(trimmed, writer) {
                    Ok(value) => break Ok(value),
                    Err(err) => match err {
                        Err(err) => break Err(err),
                        Ok(self_) => {
                            self_mut = self_;
                            writer.write_all(b": ")?;
                            writer.flush()?;
                        }
                    },
                };
            }
            input.clear();
        }
    }

    /// Prompt user to input a value for this parameter.
    ///
    /// Don't flush the writer after writing to it.
    /// The caller will flush it when necessary.
    fn prompt(&self, writer: &mut impl Write) -> io::Result<()>;

    /// Prompt user to re-input a value for this parameter when the input is empty and default value
    /// is not set.
    ///
    /// Don't flush the writer after writing to it.
    /// The caller will flush it when necessary.
    fn prompt_no_default(&self, writer: &mut impl Write) -> io::Result<()>;

    /// Function to parse a string to the value of this parameter.
    /// If the input is invalid, give back the ownership and write an error message to the writer.
    /// self should be returned in `Err(Ok(self))`,
    /// while if write failed, the error should be returned in `Err(Error(err))`.
    fn parse(self, input: &str, writer: &mut impl Write) -> Result<Self::Value, io::Result<Self>>;
}

macro_rules! err_err {
    ($err:expr) => {
        if let Err(err) = $err {
            return Err(Err(err.into()));
        }
    };
}

mod bool_input;
pub use bool_input::BoolInput;

mod input;
pub use input::Input;

mod select;
pub use select::{SelectD, Selectable, ValueWithDesc};

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
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
            SelectD::<i32>::new([1, 2], Some(2), Some(""), true)
                .unwrap()
                .value()
                .unwrap(),
            2
        );
        assert_eq!(
            SelectD::<i32>::new([1, 2], None, Some(""), true)
                .unwrap()
                .value()
                .unwrap(),
            1
        );
    }

    #[test]
    fn ask() {
        macro_rules! input {
            ($str:expr) => {
                &mut io::BufReader::new($str.as_bytes())
            };
        }

        let mut output = Vec::new();

        // Test good input
        let bool_input = BoolInput::new(Some(true), Some("hello"));
        assert!(bool_input.clone().ask(&mut output, input!("\n")).unwrap());
        assert!(bool_input.clone().ask(&mut output, input!(" \n")).unwrap());
        assert!(bool_input.clone().ask(&mut output, input!("y\n")).unwrap());
        assert!(bool_input.clone().ask(&mut output, input!("y \n")).unwrap());
        assert!(!bool_input.clone().ask(&mut output, input!("n\n")).unwrap());
        assert!(!bool_input.clone().ask(&mut output, input!("n \n")).unwrap());

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Whether to hello [Y/n]: Whether to hello [Y/n]: Whether to hello [Y/n]: \
             Whether to hello [Y/n]: Whether to hello [Y/n]: Whether to hello [Y/n]: "
        );

        // Test empty input when default is not set
        let mut output = Vec::new();
        let bool_input = BoolInput::new(None, Some("hello"));
        assert!(
            bool_input
                .clone()
                .ask(&mut output, input!("\ny\n"))
                .unwrap()
        );
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Whether to hello [y/n]: Default value not set, please input y/n: "
        );

        // Test invalid input
        let mut output = Vec::new();
        assert!(
            bool_input
                .clone()
                .ask(&mut output, input!("invalid\ny\n"))
                .unwrap()
        );

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Whether to hello [y/n]: Invalid input, please input y/n: "
        );
    }
}
