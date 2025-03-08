use std::{
    convert::Infallible,
    fmt::Display,
    io::{self, Write},
    str::FromStr,
};

use anyhow::bail;
use serde::Deserialize;

use super::UserInput;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct Select<S> {
    /// Alternatives for this parameter
    alternatives: Vec<S>,
    /// The index of the default value
    default_index: Option<usize>,
    /// Description of this parameter
    description: Option<String>,
    /// Allow custom input
    allow_custom: bool,
}

impl<'de, S: Deserialize<'de>> Deserialize<'de> for Select<S> {
    fn deserialize<D>(deserializer: D) -> Result<Select<S>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SelectHelper<H> {
            #[serde(default = "Vec::new")]
            alternatives: Vec<H>,
            #[serde(default)]
            default_index: Option<usize>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            allow_custom: bool,
        }

        let helper = SelectHelper::<S>::deserialize(deserializer)?;

        Select::raw_new(
            helper.alternatives,
            helper.default_index,
            helper.description,
            helper.allow_custom,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl<A> Select<A> {
    /// Create a new Select
    ///
    /// # Arguments
    ///
    /// * `alternatives` - A list of alternatives for this parameter;
    /// * `default_index` - The 1-based index of the default value;
    /// * `description` - Description of this parameter, default to "one of the alternatives";
    /// * `allow_custom` - Allow custom input, if set to true and input is not a number, try to
    ///   parse it;
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::config::task::value::input::Select;
    ///
    /// let select = Select::<String>::new(
    ///     vec!["CE-5", "CE-6"],
    ///     Some(2),
    ///     Some("a stage to fight"),
    ///     true,
    /// );
    /// ```
    ///
    /// User will be prompt with:
    ///
    /// ```text
    /// 1. CE-5
    /// 2. CE-6 (default)
    /// Please select a stage to fight or input a custom one:
    /// ```
    ///
    /// If user input an empty string, it will be return the default value `CE-6`.
    /// If user input a number in range like `1`, it will be return the first alternative `CE-5`.
    /// If user input a custom value like `CE-4`, it will be return the custom value `CE-4`.
    ///
    /// # Errors
    ///
    /// - `alternatives` is empty;
    /// - `default_index` is out of range;
    pub fn new<Item, Iter>(
        alternatives: Iter,
        default_index: Option<usize>,
        description: Option<&str>,
        allow_custom: bool,
    ) -> anyhow::Result<Self>
    where
        Item: Into<A>,
        Iter: IntoIterator<Item = Item>,
    {
        Self::raw_new(
            alternatives.into_iter().map(Into::into).collect(),
            default_index,
            description.map(|s| s.into()),
            allow_custom,
        )
    }

    fn raw_new(
        alternatives: Vec<A>,
        mut default_index: Option<usize>,
        description: Option<String>,
        allow_custom: bool,
    ) -> anyhow::Result<Self> {
        if alternatives.is_empty() {
            bail!("alternatives is empty");
        }

        if let Some(ref mut default_index) = default_index {
            if *default_index > alternatives.len() || *default_index < 1 {
                bail!("default_index out of range (1 - {})", alternatives.len());
            }
            *default_index -= 1;
        }

        Ok(Self {
            alternatives,
            default_index,
            description,
            allow_custom,
        })
    }
}

impl<S> UserInput for Select<S>
where
    S: Selectable + Display,
{
    type Value = S::Value;

    fn default(mut self) -> Result<Self::Value, Self> {
        self.default_index
            .map(|i| self.alternatives.swap_remove(i).value())
            .ok_or(self)
    }

    /// Get the first alternative as default value if default_index is not set.
    fn batch_default(mut self) -> Result<Self::Value, Self> {
        Ok(self
            .alternatives
            .swap_remove(self.default_index.unwrap_or(0))
            .value())
    }

    fn prompt(&self, writer: &mut impl Write) -> io::Result<()> {
        for (i, alternative) in self.alternatives.iter().enumerate() {
            write!(writer, "{}. {}", i + 1, alternative)?;
            if self.default_index.is_some_and(|d| d == i) {
                writeln!(writer, " [default]")?;
            } else {
                writeln!(writer)?;
            }
        }
        write!(writer, "Please select")?;
        if let Some(description) = &self.description {
            write!(writer, " {}", description)?;
        } else {
            write!(writer, " one of the alternatives")?;
        }
        if self.allow_custom {
            write!(writer, " or input a custom value")?;
        }
        if self.default_index.is_some() {
            write!(writer, " (empty for default)")?;
        }

        Ok(())
    }

    fn prompt_no_default(&self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "Default not set, please select")?;
        if let Some(description) = &self.description {
            write!(writer, " {}", description)?;
        } else {
            write!(writer, " one of the alternatives")?;
        }
        if self.allow_custom {
            write!(writer, " or input a custom value")?;
        }

        Ok(())
    }

    fn parse(
        mut self,
        input: &str,
        writer: &mut impl Write,
    ) -> Result<Self::Value, io::Result<Self>> {
        let len = self.alternatives.len();
        match input.parse::<usize>() {
            Ok(index) => {
                if index > len || index < 1 {
                    err_err!(write!(
                        writer,
                        "Index {} out of range, please try again (1 - {})",
                        index, len
                    ));

                    Err(Ok(self))
                } else {
                    Ok(self
                        .alternatives
                        .swap_remove(index.saturating_sub(1))
                        .value())
                }
            }
            Err(_) if self.allow_custom => match S::parse(input) {
                Ok(value) => Ok(value),
                Err(_) => {
                    err_err!(write!(
                        writer,
                        "Invalid input \"{}\", please input an index number (1 - {}) or a custom value",
                        input, len
                    ));

                    Err(Ok(self))
                }
            },
            Err(_) => {
                err_err!(write!(
                    writer,
                    "Invalid index \"{}\", please input an index number (1 - {})",
                    input, len
                ));

                Err(Ok(self))
            }
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

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Clone)]
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
mod tests {
    use serde_test::{assert_de_tokens, Token};

    use super::*;
    use crate::assert_matches;

    // Use this function to get a Select with most fields set to Some.
    fn test_full() -> SelectD<String> {
        SelectD::<String>::new(
            vec![
                ValueWithDesc::new("CE-5", Some("LMB stage 5")),
                ValueWithDesc::new("CE-6", Some("LMB stage 6")),
            ],
            Some(2),
            Some("a stage to fight"),
            true,
        )
        .unwrap()
    }

    // Use this function to get a Select with most fields set to None.
    fn test_none() -> SelectD<String> {
        SelectD::<String>::new(vec!["CE-5", "CE-6"], None, None, false).unwrap()
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
        assert_matches!(
            test_full(),
            SelectD {
                alternatives,
                default_index: Some(1),
                description: Some(description),
                allow_custom: true,
            } if alternatives == [
                ValueWithDesc::new("CE-5", Some("LMB stage 5")),
                ValueWithDesc::new("CE-6", Some("LMB stage 6")),
            ] && description == "a stage to fight"
        );

        assert_matches!(
            test_none(),
            SelectD {
                alternatives,
                default_index: None,
                description: None,
                allow_custom: false,
            } if alternatives == vec!["CE-5", "CE-6"].into_iter().map(|s| s.into()).collect::<Vec<_>>()
        );

        assert_eq!(
            SelectD::<String>::new::<&str, [_; 0]>([], None, None, false)
                .unwrap_err()
                .to_string(),
            "alternatives is empty"
        );

        assert_eq!(
            SelectD::<String>::new(["CE-5", "CE-6"], Some(3), None, false)
                .unwrap_err()
                .to_string(),
            "default_index out of range (1 - 2)"
        )
    }

    #[test]
    fn default() {
        assert_eq!(test_full().default().unwrap(), "CE-6");
        assert_eq!(test_none().default().unwrap_err(), test_none());
    }

    #[test]
    fn batch_default() {
        assert_eq!(test_full().batch_default().unwrap(), "CE-6");
        assert_eq!(test_none().batch_default().unwrap(), "CE-5");
    }

    #[test]
    fn prompt() {
        let mut buffer = Vec::new();
        test_full().prompt(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "1. CE-5 (LMB stage 5)\n\
             2. CE-6 (LMB stage 6) [default]\n\
             Please select a stage to fight or input a custom value (empty for default)"
        );

        let mut buffer = Vec::new();
        test_none().prompt(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "1. CE-5\n\
             2. CE-6\n\
             Please select one of the alternatives"
        );
    }

    #[test]
    fn prompt_no_default() {
        let mut buffer = Vec::new();
        test_full().prompt_no_default(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Default not set, please select a stage to fight or input a custom value"
        );

        let mut buffer = Vec::new();
        test_none().prompt_no_default(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Default not set, please select one of the alternatives"
        );
    }

    #[test]
    fn parse() {
        let select = SelectD::new([1.0, 3.0], Some(2), None, true).unwrap();

        let mut output = Vec::new();
        assert_eq!(select.clone().parse("1", &mut output).unwrap(), 1.0);
        assert_eq!(select.clone().parse("2.0", &mut output).unwrap(), 2.0);
        assert_eq!(
            select.clone().parse("3", &mut output).unwrap_err().unwrap(),
            select
        );
        assert_eq!(
            select.clone().parse("x", &mut output).unwrap_err().unwrap(),
            select
        );

        let select = SelectD::new([1.0, 3.0], Some(2), None, false).unwrap();
        assert_eq!(
            select.clone().parse("x", &mut output).unwrap_err().unwrap(),
            select.clone()
        );

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Index 3 out of range, please try again (1 - 2)\
             Invalid input \"x\", please input an index number (1 - 2) or a custom value\
             Invalid index \"x\", please input an index number (1 - 2)"
        );
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
}
