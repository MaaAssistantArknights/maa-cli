use std::{
    borrow::Cow,
    convert::Infallible,
    fmt::Display,
    io::{self, Write},
    num::NonZero,
    str::FromStr,
};

use serde::Deserialize;

use super::{Outcome, UserInput};
use crate::error::{Error, Result};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawSelect<S>")]
pub struct Select<S> {
    /// Alternatives for this parameter
    alternatives: Vec<S>,
    /// The index of the default value
    default_index: usize,
    /// Description of this parameter
    description: Option<Cow<'static, str>>,
    /// Allow custom input
    allow_custom: bool,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSelect<S> {
    alternatives: Vec<S>,
    #[serde(default)]
    default_index: Option<NonZero<usize>>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    allow_custom: bool,
}

impl<S> TryFrom<RawSelect<S>> for Select<S> {
    type Error = Error;

    fn try_from(value: RawSelect<S>) -> Result<Self> {
        let mut s = Select::new(value.alternatives, value.default_index)?;
        if let Some(desc) = value.description {
            s = s.with_description(desc);
        }
        s = s.with_allow_custom(value.allow_custom);
        Ok(s)
    }
}

impl<A> Select<A> {
    /// Create a new Select
    ///
    /// # Arguments
    ///
    /// - `alternatives` - An iterator of alternatives will be collected into a vector;
    /// - `default_index` - The 1-based index of the default value;
    ///
    /// # Returns
    ///
    /// Returns `Ok(Select)` if successful, or an `Error` if:
    ///
    /// - `alternatives` is empty.
    /// - `default_index` is out of range (greater than the number of alternatives).
    ///
    /// # Behavior
    ///
    /// - If `default_index` is `None`, the first alternative is used as the default and a
    ///   deprecation warning is logged.
    /// - The internal `default_index` is stored as a 0-based index.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::num::NonZero;
    ///
    /// use maa_value::userinput::SelectD;
    ///
    /// let select = SelectD::<String>::from_iter(vec!["apple", "orange"], NonZero::new(2))
    ///     .unwrap()
    ///     .with_description("a kind of fruit to eat")
    ///     .with_allow_custom(true);
    /// ```
    ///
    /// User will be prompt with:
    ///
    /// ```text
    /// 1. Apple
    /// 2. Orange (default)
    /// Please select a kind of friut to eat or input a custom one:
    /// ```
    ///
    /// If user input an empty string, it will be return the default value `Orange`.
    /// If user input a number in range like `1`, it will be return the first alternative `Apple`.
    /// If user input a custom value like `Banana`, it will be return the custom value `Banana`.
    ///
    /// # Errors
    ///
    /// - `alternatives` is empty;
    /// - `default_index` is out of range;
    pub fn from_iter<IA: Into<A>, I: IntoIterator<Item = IA>>(
        alternatives: I,
        default_index: Option<NonZero<usize>>,
    ) -> Result<Self> {
        Self::new(
            alternatives.into_iter().map(Into::into).collect(),
            default_index,
        )
    }

    /// Creates a new `Select` from a vector of alternatives and an optional default index.
    ///
    /// See also [`Select::from_iter`]
    pub fn new(alternatives: Vec<A>, default_index: Option<NonZero<usize>>) -> Result<Self> {
        if alternatives.is_empty() {
            return Err(Error::EmptyAlternatives);
        }

        let default_index = if let Some(i) = default_index {
            let index = i.get();
            if index > alternatives.len() {
                return Err(Error::IndexOutOfRange {
                    index,
                    len: alternatives.len(),
                });
            }
            index - 1
        } else {
            log::warn!(
                "No default index for select input is deprecated, using the first alternative as default."
            );
            0
        };

        Ok(Self {
            alternatives,
            default_index,
            description: None,
            allow_custom: false,
        })
    }

    /// Set the description of the select input.
    pub fn with_description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set whether the select input allows custom input.
    pub fn with_allow_custom(mut self, allow_custom: bool) -> Self {
        self.allow_custom = allow_custom;
        self
    }
}

impl<S> UserInput for Select<S>
where
    S: Selectable + Display,
{
    type Value = S::Value;

    fn default(mut self) -> Outcome<Self::Value, Self> {
        Outcome::Value(self.alternatives.swap_remove(self.default_index).value())
    }

    fn prompt_prefix_first(&self, writer: &mut impl Write) -> io::Result<()> {
        for (i, alternative) in self.alternatives.iter().enumerate() {
            write!(writer, "{}. {}", i + 1, alternative)?;
            if i == self.default_index {
                write!(writer, " [default]")?;
            }
            writeln!(writer)?;
        }
        write!(writer, "Please select")
    }

    fn prompt_prefix_empty(&self, _writer: &mut impl Write) -> io::Result<()> {
        unreachable!("Select always has a default value")
    }

    fn prompt_prefix_invalid(&self, writer: &mut impl Write, msg: &str) -> io::Result<()> {
        write!(writer, "{}, please select", msg)
    }

    fn prompt_description(&self, writer: &mut impl Write) -> io::Result<()> {
        if let Some(description) = &self.description {
            write!(writer, " {description}")?;
        } else {
            write!(writer, " one of the alternatives")?;
        }
        if self.allow_custom {
            write!(writer, " or input a custom value")?;
        }
        write!(writer, " (empty for default)")?;

        Ok(())
    }

    fn parse(mut self, input: &str) -> Outcome<Self::Value, (Self, Cow<'_, str>)> {
        let len = self.alternatives.len();
        match input.parse::<usize>() {
            Ok(index) => {
                if index > len || index < 1 {
                    Outcome::Original((
                        self,
                        Cow::Owned(format!("Index {index} out of range (1 - {len})")),
                    ))
                } else {
                    Outcome::Value(
                        self.alternatives
                            .swap_remove(index.saturating_sub(1))
                            .value(),
                    )
                }
            }
            Err(_) if self.allow_custom => match S::parse(input) {
                Ok(value) => Outcome::Value(value),
                Err(_) => {
                    Outcome::Original((self, Cow::Owned(format!("Invalid input \"{input}\"",))))
                }
            },
            Err(_) => Outcome::Original((self, Cow::Owned(format!("Invalid index \"{input}\"",)))),
        }
    }
}

pub trait Selectable {
    type Value;
    type Error;

    /// Get the value of this element, consum self.
    fn value(self) -> Self::Value;

    /// Parse a string to value of this element.
    ///
    /// This function parse a string to value of this element
    /// instead of the element itself to allow custom input.
    fn parse(input: &str) -> Result<Self::Value, Self::Error>;
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(untagged, deny_unknown_fields)]
pub enum ValueWithDesc<T> {
    Value(T),
    WithDesc { value: T, desc: String },
}

impl<T: Display> ValueWithDesc<T> {
    pub fn new(value: impl Into<T>, desc: Option<&str>) -> Self {
        match desc {
            Some(desc) => Self::WithDesc {
                value: value.into(),
                desc: desc.to_string(),
            },
            None => Self::Value(value.into()),
        }
    }

    fn value(self) -> T {
        use ValueWithDesc::*;
        match self {
            Value(value) => value,
            WithDesc { value, .. } => value,
        }
    }
}

impl<T> From<T> for ValueWithDesc<T> {
    fn from(value: T) -> Self {
        ValueWithDesc::Value(value)
    }
}

impl From<&str> for ValueWithDesc<String> {
    fn from(value: &str) -> Self {
        Self::from(String::from(value))
    }
}

impl Selectable for ValueWithDesc<i32> {
    type Error = <i32 as FromStr>::Err;
    type Value = i32;

    fn value(self) -> i32 {
        self.value()
    }

    fn parse(input: &str) -> Result<i32, Self::Error> {
        input.parse()
    }
}

impl<T: Display> Display for ValueWithDesc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueWithDesc::Value(value) => write!(f, "{value}"),
            ValueWithDesc::WithDesc { value, desc } => write!(f, "{value} ({desc})"),
        }
    }
}

impl Selectable for ValueWithDesc<f32> {
    type Error = <f32 as FromStr>::Err;
    type Value = f32;

    fn value(self) -> f32 {
        self.value()
    }

    fn parse(input: &str) -> Result<f32, Self::Error> {
        input.parse()
    }
}

impl Selectable for ValueWithDesc<String> {
    type Error = Infallible;
    type Value = String;

    fn value(self) -> String {
        self.value()
    }

    fn parse(input: &str) -> Result<String, Self::Error> {
        Ok(input.to_owned())
    }
}

/// A type alias for `Select<ValueWithDescription<T>>`.
///
/// The `SelectD` type is a `Select` with optional description for each alternative.
/// Value of `SelectD<T>` is the same as `Select<T>`.
pub type SelectD<T> = Select<ValueWithDesc<T>>;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use serde_test::{Token, assert_de_tokens};

    use super::{super::assert_prompt, *};

    // Use this function to get a Select with most fields set to Some.
    fn test_full() -> SelectD<String> {
        SelectD::<String>::from_iter(
            vec![
                ValueWithDesc::new("CE-5", Some("LMB stage 5")),
                ValueWithDesc::new("CE-6", Some("LMB stage 6")),
            ],
            NonZero::new(2),
        )
        .unwrap()
        .with_description("a stage to fight")
        .with_allow_custom(true)
    }

    // Use this function to get a Select with most fields set to None.
    fn test_none() -> SelectD<String> {
        SelectD::<String>::from_iter(vec!["CE-5", "CE-6"], None).unwrap()
    }

    #[test]
    fn serde() {
        let values = [test_full(), test_none()];

        assert_de_tokens(&values, &[
            Token::Seq { len: Some(2) },
            Token::Map { len: Some(4) },
            Token::Str("alternatives"),
            Token::Seq { len: Some(2) },
            Token::Map { len: Some(2) },
            Token::Str("value"),
            Token::Str("CE-5"),
            Token::Str("desc"),
            Token::Str("LMB stage 5"),
            Token::MapEnd,
            Token::Map { len: Some(2) },
            Token::Str("value"),
            Token::Str("CE-6"),
            Token::Str("desc"),
            Token::Str("LMB stage 6"),
            Token::MapEnd,
            Token::SeqEnd,
            Token::Str("default_index"),
            Token::Some,
            Token::U64(2),
            Token::Str("description"),
            Token::Some,
            Token::Str("a stage to fight"),
            Token::Str("allow_custom"),
            Token::Bool(true),
            Token::MapEnd,
            Token::Map { len: Some(1) },
            Token::Str("alternatives"),
            Token::Seq { len: Some(2) },
            Token::Str("CE-5"),
            Token::Str("CE-6"),
            Token::SeqEnd,
            Token::MapEnd,
            Token::SeqEnd,
        ]);
    }

    #[test]
    fn construct() {
        let full = test_full();
        assert_eq!(full.alternatives, vec![
            ValueWithDesc::new("CE-5", Some("LMB stage 5")),
            ValueWithDesc::new("CE-6", Some("LMB stage 6")),
        ]);
        assert_eq!(full.default_index, 1);
        assert_eq!(full.description.as_deref(), Some("a stage to fight"));
        assert!(full.allow_custom);

        let none = test_none();
        assert_eq!(
            none.alternatives,
            vec!["CE-5", "CE-6"]
                .into_iter()
                .map(|s| s.into())
                .collect::<Vec<_>>()
        );
        assert_eq!(none.default_index, 0);
        assert_eq!(none.description, None);
        assert!(!none.allow_custom);

        assert_eq!(
            SelectD::<String>::from_iter::<&str, [_; 0]>([], None)
                .unwrap_err()
                .to_string(),
            "alternatives is empty"
        );

        assert!(matches!(
            SelectD::<String>::from_iter(["CE-5", "CE-6"], NonZero::new(3)).unwrap_err(),
            Error::IndexOutOfRange { index: 3, len: 2 },
        ));
    }

    #[test]
    fn default() {
        match test_full().default() {
            Outcome::Value(v) => assert_eq!(v, "CE-6"),
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }

        match test_none().default() {
            Outcome::Value(v) => assert_eq!(v, "CE-5"),
            Outcome::Original(_) => panic!("Expected Value, got Original"),
        }
    }

    mod prompt_prefix_first {
        use super::*;

        #[test]
        fn with_description_and_custom() {
            assert_prompt(
                &test_full(),
                "1. CE-5 (LMB stage 5)\n\
                 2. CE-6 (LMB stage 6) [default]\n\
                 Please select",
                UserInput::prompt_prefix_first,
            );
        }

        #[test]
        fn without_description() {
            assert_prompt(
                &test_none(),
                "1. CE-5 [default]\n\
                 2. CE-6\n\
                 Please select",
                UserInput::prompt_prefix_first,
            );
        }
    }

    mod prompt_prefix_empty {
        use super::*;

        #[test]
        #[should_panic]
        fn should_unreachable() {
            let mut buffer = Vec::new();
            let _ = test_full().prompt_prefix_empty(&mut buffer);
        }
    }

    mod prompt_prefix_invalid {
        use super::*;

        #[test]
        fn includes_invalid_input_message() {
            let mut buffer = Vec::new();
            test_full()
                .prompt_prefix_invalid(&mut buffer, "xyz")
                .unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), "xyz, please select");
        }
    }

    mod prompt_description {
        use super::*;

        #[test]
        fn with_description_and_custom() {
            assert_prompt(
                &test_full(),
                " a stage to fight or input a custom value (empty for default)",
                |ui, buf| ui.prompt_description(buf),
            );
        }

        #[test]
        fn without_description_or_custom() {
            assert_prompt(
                &test_none(),
                " one of the alternatives (empty for default)",
                |ui, buf| ui.prompt_description(buf),
            );
        }

        #[test]
        fn with_custom_only() {
            assert_prompt(
                &SelectD::<String>::from_iter(vec!["CE-5", "CE-6"], None)
                    .unwrap()
                    .with_allow_custom(true),
                " one of the alternatives or input a custom value (empty for default)",
                |ui, buf| ui.prompt_description(buf),
            );
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn valid_index_first() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2)).unwrap();
            match select.parse("1") {
                Outcome::Value(v) => assert_eq!(v, 1.0),
                Outcome::Original(_) => panic!("Expected Value(1.0), got Original"),
            }
        }

        #[test]
        fn valid_index_last() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2)).unwrap();
            match select.parse("2") {
                Outcome::Value(v) => assert_eq!(v, 3.0),
                Outcome::Original(_) => panic!("Expected Value(3.0), got Original"),
            }
        }

        #[test]
        fn index_out_of_range_high() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2)).unwrap();
            match select.clone().parse("3") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Index 3 out of range (1 - 2)");
                }
            }
        }

        #[test]
        fn index_zero_out_of_range() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2)).unwrap();
            match select.clone().parse("0") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Index 0 out of range (1 - 2)");
                }
            }
        }

        #[test]
        fn valid_custom_value_when_allowed() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2))
                .unwrap()
                .with_allow_custom(true);
            match select.parse("2.0") {
                Outcome::Value(v) => assert_eq!(v, 2.0),
                Outcome::Original(_) => panic!("Expected Value(2.0), got Original"),
            }
        }

        #[test]
        fn invalid_custom_value_when_allowed() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2))
                .unwrap()
                .with_allow_custom(true);
            match select.clone().parse("x") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid input \"x\"");
                }
            }
        }

        #[test]
        fn non_index_input_when_custom_not_allowed() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2)).unwrap();
            match select.clone().parse("x") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid index \"x\"");
                }
            }
        }

        #[test]
        fn string_always_valid_as_custom() {
            let select = SelectD::<String>::from_iter(["CE-5", "CE-6"], NonZero::new(1))
                .unwrap()
                .with_allow_custom(true);
            match select.parse("CE-7") {
                Outcome::Value(v) => assert_eq!(v, "CE-7"),
                Outcome::Original(_) => panic!("Expected Value(CE-7), got Original"),
            }
        }

        #[test]
        fn negative_index_returns_error() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2)).unwrap();
            match select.clone().parse("-1") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid index \"-1\"");
                }
            }
        }

        #[test]
        fn decimal_index_fails() {
            let select = SelectD::from_iter([1.0, 3.0], NonZero::new(2)).unwrap();
            match select.clone().parse("1.5") {
                Outcome::Value(_) => panic!("Expected Original, got Value"),
                Outcome::Original((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid index \"1.5\"");
                }
            }
        }
    }

    mod selectable {
        use super::*;

        #[test]
        fn int() {
            let value = ValueWithDesc::<i32>::new(1, None);
            assert_eq!(value.value(), 1);
            assert_eq!(ValueWithDesc::<i32>::parse("1").unwrap(), 1);
            assert!(ValueWithDesc::<i32>::parse("a").is_err())
        }

        #[test]
        fn float() {
            let value = ValueWithDesc::<f32>::new(1.0, None);
            assert_eq!(value.value(), 1.0);
            assert_eq!(ValueWithDesc::<f32>::parse("1.0").unwrap(), 1.0);
            assert!(ValueWithDesc::<f32>::parse("a").is_err())
        }

        #[test]
        fn string() {
            let value = ValueWithDesc::<String>::new("a", None);
            assert_eq!(value.value(), "a");
            assert_eq!(ValueWithDesc::<String>::parse("a").unwrap(), "a");
        }
    }

    mod ask {
        use super::{super::super::assert_output, *};

        #[test]
        fn empty_input_returns_default() {
            assert_output(
                test_none(),
                "\n",
                "1. CE-5 [default]\n\
                 2. CE-6\n\
                 Please select one of the alternatives (empty for default): ",
                "CE-5",
            );
        }

        #[test]
        fn valid_index_returns_value() {
            assert_output(
                test_none(),
                "2\n",
                "1. CE-5 [default]\n\
                 2. CE-6\n\
                 Please select one of the alternatives (empty for default): ",
                "CE-6",
            );
        }

        #[test]
        fn invalid_index_reprompts() {
            assert_output(
                test_none(),
                "3\n2\n",
                "1. CE-5 [default]\n\
                 2. CE-6\n\
                 Please select one of the alternatives (empty for default): \
                 Index 3 out of range (1 - 2), please select one of the alternatives (empty for default): ",
                "CE-6",
            );
        }

        #[test]
        fn custom_value_when_allowed() {
            assert_output(
                test_full(),
                "CE-4\n",
                "1. CE-5 (LMB stage 5)\n\
                 2. CE-6 (LMB stage 6) [default]\n\
                 Please select a stage to fight or input a custom value (empty for default): ",
                "CE-4",
            );
        }
    }
}
