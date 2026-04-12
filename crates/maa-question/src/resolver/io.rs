use std::io::{self, BufRead, Write};

use crate::{Question, Resolve};

/// A resolver backed by a pair of IO streams.
#[derive(Debug)]
pub struct IoResolver<I: Io = StdIo>(pub I);

/// Trait for question types that can be rendered on an [`IoResolver`].
///
/// There are three required methods:
///
/// - [`write_description_to`](Self::write_description_to): the question description,
/// - [`write_first_prefix`](Self::write_first_prefix): the prefix for the first time asking the
///   question,
/// - [`write_invalid_prefix`](Self::write_invalid_prefix): the prefix for when an invalid answer is
///   provided and the question is asked again.
///
/// For a question type `Q` implementing `PromptIo`, the first prompt will look like this:
///
/// ```text
/// {prefix_first} {question_description} ({default}): {answer}
/// ```
///
/// After an invalid answer, the retry prompt will look like this:
///
/// ```text
/// {prefix_invalid} {question_description} ({default}): {answer}
/// ```
pub trait PromptIo: Question {
    /// Writes the question description to the given writer.
    fn write_description_to(&self, writer: &mut dyn Write) -> io::Result<()>;

    /// Writes the prefix for the first time asking the question to the given writer.
    fn write_first_prefix(&self, writer: &mut dyn Write) -> io::Result<()>;

    /// Writes the prefix for when an invalid answer is provided and re-asking the question to the
    /// given writer.
    fn write_invalid_prefix(&self, writer: &mut dyn Write) -> io::Result<()>;
}

impl<Q: PromptIo, I: Io> Resolve<Q> for IoResolver<I> {
    type Error = io::Error;

    fn resolve(&mut self, mut question: Q) -> Result<Q::Answer, Self::Error> {
        let (reader, writer) = self.0.io();
        question.write_first_prefix(writer)?;
        question.write_description_to(writer)?;
        writer.write_all(b": ")?;
        writer.flush()?;
        let mut line = String::new();
        loop {
            reader.read_line(&mut line)?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break Ok(question.default());
            } else {
                match question.interpret(trimmed) {
                    Ok(value) => break Ok(value),
                    Err((orig, msg)) => {
                        question = orig;
                        writeln!(writer, "{}", msg)?;
                        question.write_invalid_prefix(writer)?;
                        question.write_description_to(writer)?;
                        writer.write_all(b": ")?;
                        writer.flush()?;
                    }
                }
            }
            line.clear();
        }
    }
}

/// A pair of input and output streams.
pub trait Io {
    fn io(&mut self) -> (&mut dyn BufRead, &mut dyn Write);
}

#[derive(Debug)]
pub struct StdIo {
    stdin: std::io::StdinLock<'static>,
    stdout: std::io::StdoutLock<'static>,
}

impl StdIo {
    pub fn new() -> Self {
        Self {
            stdin: std::io::stdin().lock(),
            stdout: std::io::stdout().lock(),
        }
    }
}

impl Default for StdIo {
    fn default() -> Self {
        Self::new()
    }
}

impl Io for StdIo {
    fn io(&mut self) -> (&mut dyn BufRead, &mut dyn Write) {
        (&mut self.stdin, &mut self.stdout)
    }
}

/// A helper to ask a question via stdout/stdin.
pub fn ask<Q: PromptIo>(question: Q) -> Result<Q::Answer, io::Error> {
    let mut io = IoResolver(StdIo::new());
    io.resolve(question)
}

#[cfg(test)]
#[track_caller]
pub(crate) fn assert_prompt<I: PromptIo>(
    ui: &I,
    expected: &str,
    prompt_fn: impl FnOnce(&I, &mut dyn io::Write) -> io::Result<()>,
) {
    let mut buffer: Vec<u8> = Vec::new();
    prompt_fn(ui, &mut buffer).unwrap();
    assert_eq!(String::from_utf8(buffer).unwrap(), expected);
}

#[cfg(test)]
#[track_caller]
pub(crate) fn assert_first_prompt<I: PromptIo>(ui: &I, expected: &str) {
    assert_prompt(ui, expected, |ui, writer| {
        ui.write_first_prefix(writer)?;
        ui.write_description_to(writer)
    });
}

#[cfg(test)]
#[track_caller]
pub(crate) fn assert_output<I, E>(ui: I, input: &str, expected_output: &str, expected_value: E)
where
    I: PromptIo,
    I::Answer: PartialEq + std::fmt::Debug + std::cmp::PartialEq<E>,
    E: std::fmt::Debug,
{
    #[derive(Debug)]
    struct TestIo {
        input: io::Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl Io for TestIo {
        fn io(&mut self) -> (&mut dyn BufRead, &mut dyn Write) {
            (&mut self.input, &mut self.output)
        }
    }

    let mut resolver = IoResolver(TestIo {
        input: io::Cursor::new(input.as_bytes().to_vec()),
        output: Vec::new(),
    });
    let value = resolver.resolve(ui).unwrap();
    let output = String::from_utf8(resolver.0.output).unwrap();
    assert_eq!(output, expected_output);
    assert_eq!(value, expected_value);
}
