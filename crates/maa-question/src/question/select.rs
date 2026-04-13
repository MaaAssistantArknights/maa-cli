use std::{
    fmt::Display,
    io::{self, Write},
    num::NonZero,
    str::FromStr,
};

use nonempty_vec::NonEmptyVec;
#[cfg(feature = "serde")]
use serde::{Deserialize, de::Error};

use crate::{Question, question::CowStr, resolver::io::PromptIo};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq)]
pub struct Select<S> {
    /// Stable identifier for variable injection.
    id: Option<CowStr>,
    /// List of alternatives for the user to choose from.
    alternatives: NonEmptyVec<S>,
    /// Index of the default alternative to use if the user doesn't provide a value.
    default_index: usize,
    /// Description of the question to display to the user.
    description: Option<CowStr>,
    /// Whether the user is allowed to provide a custom value not in the list of alternatives.
    allow_custom: bool,
}

#[cfg(feature = "serde")]
impl<'de, S: Deserialize<'de>> Deserialize<'de> for Select<S> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSelect<S> {
            #[serde(default)]
            id: Option<CowStr>,
            alternatives: Vec<S>,
            default_index: usize,
            #[serde(default)]
            description: Option<CowStr>,
            #[serde(default)]
            allow_custom: bool,
        }

        let raw = RawSelect::deserialize(deserializer)?;

        let RawSelect {
            id,
            alternatives,
            default_index,
            description,
            allow_custom,
        } = raw;

        let Some(alternatives) = NonEmptyVec::new(alternatives) else {
            return Err(D::Error::invalid_length(
                0,
                &"at least one alternative is required",
            ));
        };

        if default_index > alternatives.len() || default_index == 0 {
            return Err(D::Error::invalid_value(
                serde::de::Unexpected::Unsigned(default_index as u64),
                &"default index must be less than or equal to the number of alternatives",
            ));
        }

        Ok(Select {
            id,
            alternatives,
            default_index: default_index - 1,
            description,
            allow_custom,
        })
    }
}

impl<A> Select<A> {
    /// Create a new selection question.
    ///
    /// # Arguments
    ///
    /// - `alternatives` - An iterator of alternatives will be collected into a vector;
    /// - `default_index` - The 1-based index of the default value;
    ///
    /// # Returns
    ///
    /// Returns `Some(Select)` if successful, or `None` if:
    ///
    /// - `alternatives` is empty.
    /// - `default_index` is out of range (greater than the number of alternatives).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::num::NonZero;
    ///
    /// use maa_question::prelude::SelectD;
    ///
    /// let select = SelectD::<String>::from_iter(vec!["apple", "orange"], NonZero::new(2).unwrap())
    ///     .unwrap()
    ///     .with_description("a kind of fruit to eat")
    ///     .with_allow_custom(true);
    /// ```
    pub fn from_iter<IA: Into<A>, I: IntoIterator<Item = IA>>(
        alternatives: I,
        default_index: NonZero<usize>,
    ) -> Option<Self> {
        let alternatives = NonEmptyVec::collect(alternatives.into_iter().map(Into::into))?;

        Self::new(alternatives, default_index)
    }

    /// Create a new selection question.
    ///
    /// # Arguments
    ///
    /// - `alternatives` - A `NonEmptyVec` of alternatives for the user to choose from.
    /// - `default_index` - The 1-based index of the default value.
    ///
    /// # Returns
    ///
    /// Returns `Some(Select)` if successful, or `None` if `default_index` is greater than the
    /// number of alternatives.
    pub fn new(alternatives: NonEmptyVec<A>, default_index: NonZero<usize>) -> Option<Self> {
        let index = default_index.get();
        if index > alternatives.len() {
            return None;
        };

        Some(Self {
            id: None,
            alternatives,
            default_index: index - 1,
            description: None,
            allow_custom: false,
        })
    }

    pub fn with_description(mut self, description: impl Into<CowStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_allow_custom(mut self, allow_custom: bool) -> Self {
        self.allow_custom = allow_custom;
        self
    }

    pub fn with_id(mut self, id: impl Into<CowStr>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl<S> Question for Select<S>
where
    S: Selectable + Display,
    S::Error: Display,
{
    type Answer = S::Value;

    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn default(self) -> Self::Answer {
        let mut alternatives = self.alternatives.into_vec();
        alternatives.swap_remove(self.default_index).value()
    }

    fn interpret(self, input: &str) -> Result<Self::Answer, (Self, String)> {
        let len = self.alternatives.len();
        match input.parse::<usize>() {
            Ok(index) => {
                if index > len || index < 1 {
                    Err((self, format!("Index {index} out of range (1 - {len})")))
                } else {
                    let mut alternatives = self.alternatives.into_vec();
                    Ok(alternatives.swap_remove(index - 1).value())
                }
            }
            Err(_) if self.allow_custom => match S::parse(input) {
                Ok(value) => Ok(value),
                Err(e) => Err((self, format!("Invalid input \"{input}\": {e}"))),
            },
            Err(e) => Err((self, format!("Invalid index \"{input}\": {e}"))),
        }
    }
}

impl<S> PromptIo for Select<S>
where
    S: Selectable + Display,
    S::Error: Display,
{
    fn write_first_prefix(&self, writer: &mut dyn Write) -> io::Result<()> {
        for (i, alternative) in self.alternatives.iter().enumerate() {
            write!(writer, "{}. {}", i + 1, alternative)?;
            if i == self.default_index {
                write!(writer, " [default]")?;
            }
            writeln!(writer)?;
        }
        write!(writer, "Please select")
    }

    fn write_invalid_prefix(&self, writer: &mut dyn Write) -> io::Result<()> {
        write!(writer, "please select")
    }

    fn write_description_to(&self, writer: &mut dyn Write) -> io::Result<()> {
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
}

pub trait Selectable {
    type Value;
    type Error;

    fn value(self) -> Self::Value;
    fn parse(input: &str) -> Result<Self::Value, Self::Error>;
}

/// A helper macro to implement [`Selectable`] for a type that implements [`FromStr`].
///
///
/// As rust don't support specialization, we cannot use a blanket implementation for
/// any type that implements [`FromStr`].
macro_rules! impl_selectable {
    ($type:path) => {
        impl Selectable for $type {
            type Error = <$type as FromStr>::Err;
            type Value = $type;

            fn value(self) -> $type {
                self
            }

            fn parse(input: &str) -> Result<$type, Self::Error> {
                input.parse()
            }
        }
    };
    ($($type:path),*) => {
        $(
            impl_selectable!($type);
        )*
    };
}

impl_selectable!(i8, i16, i32, i64, isize);
impl_selectable!(u8, u16, u32, u64, usize);
impl_selectable!(f32, f64);
impl_selectable!(String, std::path::PathBuf);

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged, deny_unknown_fields))]
pub enum ValueWithDesc<T> {
    Value(T),
    WithDesc { value: T, desc: String },
}

impl<T> ValueWithDesc<T> {
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

impl<T: Display> Display for ValueWithDesc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueWithDesc::Value(value) => write!(f, "{value}"),
            ValueWithDesc::WithDesc { value, desc } => write!(f, "{value} ({desc})"),
        }
    }
}

impl<T: Selectable> Selectable for ValueWithDesc<T> {
    type Error = T::Error;
    type Value = T::Value;

    fn value(self) -> T::Value {
        self.value().value()
    }

    fn parse(input: &str) -> Result<T::Value, Self::Error> {
        T::parse(input)
    }
}

/// A type alias for `Select<ValueWithDesc<T>>`.
pub type SelectD<T> = Select<ValueWithDesc<T>>;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::resolver::io::*;

    fn nz(value: usize) -> NonZero<usize> {
        NonZero::new(value).unwrap()
    }

    fn complex_select() -> SelectD<String> {
        SelectD::<String>::from_iter(
            vec![
                ValueWithDesc::new("CE-5", Some("LMB stage 5")),
                ValueWithDesc::new("CE-6", Some("LMB stage 6")),
            ],
            nz(2),
        )
        .unwrap()
        .with_description("a stage to fight")
        .with_allow_custom(true)
    }

    fn simple_select() -> SelectD<String> {
        SelectD::<String>::from_iter(vec!["CE-5", "CE-6"], nz(1)).unwrap()
    }

    #[cfg(feature = "serde")]
    mod serde {
        use serde_test::{Token, assert_de_tokens};

        use super::*;

        #[test]
        fn basic() {
            let values = [complex_select(), simple_select()];

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
                Token::U64(2),
                Token::Str("description"),
                Token::Some,
                Token::Str("a stage to fight"),
                Token::Str("allow_custom"),
                Token::Bool(true),
                Token::MapEnd,
                Token::Map { len: Some(2) },
                Token::Str("alternatives"),
                Token::Seq { len: Some(2) },
                Token::Str("CE-5"),
                Token::Str("CE-6"),
                Token::SeqEnd,
                Token::Str("default_index"),
                Token::U64(1),
                Token::MapEnd,
                Token::SeqEnd,
            ]);
        }
    }

    #[test]
    fn construct() {
        let full = complex_select();
        assert_eq!(&*full.alternatives, &[
            ValueWithDesc::new("CE-5", Some("LMB stage 5")),
            ValueWithDesc::new("CE-6", Some("LMB stage 6")),
        ]);
        assert_eq!(full.default_index, 1);
        assert_eq!(full.description.as_deref(), Some("a stage to fight"));
        assert!(full.allow_custom);

        let none = simple_select();
        assert_eq!(&*none.alternatives, &[
            ValueWithDesc::new("CE-5", None),
            ValueWithDesc::new("CE-6", None),
        ]);
        assert_eq!(none.default_index, 0);
        assert_eq!(none.description, None);
        assert!(!none.allow_custom);
    }

    #[test]
    fn construct_rejects_invalid_inputs() {
        assert!(SelectD::<String>::from_iter::<&str, [_; 0]>([], nz(1)).is_none());
        assert!(SelectD::<String>::from_iter(["CE-5", "CE-6"], nz(3)).is_none());
    }

    #[test]
    fn with_id() {
        let select = simple_select().with_id("stage");
        assert_eq!(select.id.as_deref(), Some("stage"));
    }

    #[test]
    fn default() {
        assert_eq!(complex_select().default(), "CE-6");
        assert_eq!(simple_select().default(), "CE-5");
    }

    mod prompt_prefix_first {
        use super::*;

        #[test]
        fn with_description_and_custom() {
            assert_first_prompt(
                &complex_select(),
                "1. CE-5 (LMB stage 5)\n\
                 2. CE-6 (LMB stage 6) [default]\n\
                 Please select a stage to fight or input a custom value (empty for default)",
            );
        }

        #[test]
        fn without_description() {
            assert_first_prompt(
                &simple_select(),
                "1. CE-5 [default]\n\
                 2. CE-6\n\
                 Please select one of the alternatives (empty for default)",
            );
        }
    }

    mod prompt_prefix_invalid {
        use super::*;

        #[test]
        fn returns_please_select() {
            let mut buffer: Vec<u8> = Vec::new();
            complex_select().write_invalid_prefix(&mut buffer).unwrap();
            assert_eq!(String::from_utf8(buffer).unwrap(), "please select");
        }
    }

    mod prompt_description {
        use super::*;

        #[test]
        fn with_description_and_custom() {
            assert_prompt(
                &complex_select(),
                " a stage to fight or input a custom value (empty for default)",
                |ui, buf| ui.write_description_to(buf),
            );
        }

        #[test]
        fn without_description_or_custom() {
            assert_prompt(
                &simple_select(),
                " one of the alternatives (empty for default)",
                |ui, buf| ui.write_description_to(buf),
            );
        }

        #[test]
        fn with_custom_only() {
            assert_prompt(
                &SelectD::<String>::from_iter(vec!["CE-5", "CE-6"], nz(1))
                    .unwrap()
                    .with_allow_custom(true),
                " one of the alternatives or input a custom value (empty for default)",
                |ui, buf| ui.write_description_to(buf),
            );
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn valid_index_first() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2)).unwrap();
            assert_eq!(select.interpret("1"), Ok(1.0));
        }

        #[test]
        fn valid_index_last() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2)).unwrap();
            assert_eq!(select.interpret("2"), Ok(3.0));
        }

        #[test]
        fn index_out_of_range_high() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2)).unwrap();
            match select.clone().interpret("3") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Index 3 out of range (1 - 2)");
                }
            }
        }

        #[test]
        fn index_zero_out_of_range() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2)).unwrap();
            match select.clone().interpret("0") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Index 0 out of range (1 - 2)");
                }
            }
        }

        #[test]
        fn valid_custom_value_when_allowed() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2))
                .unwrap()
                .with_allow_custom(true);
            assert_eq!(select.interpret("2.0"), Ok(2.0));
        }

        #[test]
        fn invalid_custom_value_when_allowed() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2))
                .unwrap()
                .with_allow_custom(true);
            match select.clone().interpret("x") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid input \"x\": invalid float literal");
                }
            }
        }

        #[test]
        fn non_index_input_when_custom_not_allowed() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2)).unwrap();
            match select.clone().interpret("x") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid index \"x\": invalid digit found in string");
                }
            }
        }

        #[test]
        fn string_always_valid_as_custom() {
            let select = SelectD::<String>::from_iter(["CE-5", "CE-6"], nz(1))
                .unwrap()
                .with_allow_custom(true);
            assert_eq!(select.interpret("CE-7"), Ok(String::from("CE-7")));
        }

        #[test]
        fn negative_index_returns_error() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2)).unwrap();
            match select.clone().interpret("-1") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid index \"-1\": invalid digit found in string");
                }
            }
        }

        #[test]
        fn decimal_index_fails() {
            let select = SelectD::from_iter([1.0, 3.0], nz(2)).unwrap();
            match select.clone().interpret("1.5") {
                Ok(_) => panic!("Expected Err, got Ok"),
                Err((returned_select, msg)) => {
                    assert_eq!(returned_select, select);
                    assert_eq!(msg, "Invalid index \"1.5\": invalid digit found in string");
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
        use super::*;

        #[test]
        fn empty_input_returns_default() {
            assert_output(
                simple_select(),
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
                simple_select(),
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
                simple_select(),
                "3\n2\n",
                "1. CE-5 [default]\n\
                 2. CE-6\n\
                 Please select one of the alternatives (empty for default): Index 3 out of range (1 - 2)\nplease select one of the alternatives (empty for default): ",
                "CE-6",
            );
        }

        #[test]
        fn custom_value_when_allowed() {
            assert_output(
                complex_select(),
                "CE-4\n",
                "1. CE-5 (LMB stage 5)\n\
                 2. CE-6 (LMB stage 6) [default]\n\
                 Please select a stage to fight or input a custom value (empty for default): ",
                "CE-4",
            );
        }
    }
}
