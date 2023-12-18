use super::UserInput;

use std::{
    convert::Infallible,
    fmt::Display,
    io::{self, Write},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

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

        let helper = SelectHelper::deserialize(deserializer)?;

        if helper.default_index.is_some()
            && helper.default_index.unwrap() >= helper.alternatives.len()
        {
            return Err(serde::de::Error::custom("default_index out of range"));
        }

        Ok(Select {
            alternatives: helper.alternatives,
            default_index: helper.default_index,
            description: helper.description,
            allow_custom: helper.allow_custom,
        })
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
    /// * `allow_custom` - Allow custom input, if set to true and input is not a number, try to parse it;
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::config::task::value::input::Select;
    ///
    /// let select = Select::<String>::new(
    ///    vec!["CE-5", "CE-6"],
    ///    Some(2),
    ///    Some("a stage to fight"),
    ///    true,
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
    /// # panic
    ///
    /// panic if `default_index` is out of range of `alternatives`.
    pub fn new<Item, Iter, Desc>(
        alternatives: Iter,
        default_index: Option<usize>,
        description: Option<Desc>,
        allow_custom: bool,
    ) -> Self
    where
        Item: Into<A>,
        Iter: IntoIterator<Item = Item>,
        Desc: Into<String>,
    {
        let alternatives: Vec<A> = alternatives.into_iter().map(|i| i.into()).collect();
        if let Some(default_index) = default_index {
            if default_index > alternatives.len() || default_index == 0 {
                panic!("default_index out of range");
            }
        }
        Self {
            alternatives,
            default_index,
            description: description.map(|s| s.into()),
            allow_custom,
        }
    }
}

impl<S: Selectable> UserInput for Select<S> {
    type Value = S::Value;

    fn default(&self) -> Option<Self::Value> {
        self.default_index
            .map(|i| {
                self.alternatives
                    .get(i.saturating_sub(1))
                    .map(|a| a.value())
            })
            .flatten()
    }

    /// Get the first alternative as default value if default_index is not set.
    fn batch_default(&self) -> Option<Self::Value> {
        self.alternatives
            .get(self.default_index.unwrap_or(1).saturating_sub(1))
            .map(|a| a.value())
    }

    fn prompt(&self, mut writer: impl Write) -> io::Result<()> {
        for (i, alternative) in self.alternatives.iter().enumerate() {
            write!(writer, "{}. ", i + 1);
            alternative.desc(writer)?;
            if self.default_index.is_some_and(|d| d == i + 1) {
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
        write!(writer, ": ")
    }

    fn prompt_no_default(&self, mut writer: impl Write) -> io::Result<()> {
        write!(writer, "Default not set, please select")?;
        if let Some(description) = &self.description {
            writeln!(writer, " {}", description)?;
        } else {
            writeln!(writer, "one of the alternatives")?;
        }
        if self.allow_custom {
            write!(writer, " or input a custom value")?;
        }
        write!(writer, ": ")
    }

    fn parse(&self, input: &str) -> Result<Self::Value, String> {
        match input.parse::<usize>() {
            Ok(index) => match self.alternatives.get(index - 1) {
                Some(alternative) => Ok(alternative.value()),
                None => Err(format!(
                    "Index {} out of range, please try again (1 - {}): ",
                    index,
                    self.alternatives.len()
                )),
            },
            Err(_) if self.allow_custom => <S as Selectable>::parse(input).map_err(|_| {
                format!(
                    "Invalid input \"{}\"\
                    please input an index number (1 - {}) or a custom value",
                    input,
                    self.alternatives.len()
                )
            }),
            Err(_) => Err(format!(
                "Invalid index \"{}\", please input an index number (1 - {})",
                input,
                self.alternatives.len()
            )),
        }
    }
}

impl<S> Serialize for Select<S>
where
    S: Selectable + Serialize,
    S::Value: Serialize,
{
    fn serialize<Se: serde::Serializer>(&self, serializer: Se) -> Result<Se::Ok, Se::Error> {
        self.value()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

pub trait Selectable {
    type Value;
    type Error;

    /// Get the value of this element, consum self.
    fn value(self) -> Self::Value;

    /// Return the description of this element.
    fn desc(&self, writer: impl Write) -> io::Result<()>;

    /// Parse a string to value of this element.
    ///
    /// This function parse a string to value of this element
    /// instead of the element itself to allow custom input.
    fn parse(input: &str) -> Result<Self::Value, Self::Error>;
}

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum ValueWithDesc<T> {
    Value(T),
    WithDesc { value: T, desc: String },
}

impl<T: Display> ValueWithDesc<T> {
    pub fn new<V, D>(value: V, description: Option<D>) -> Self
    where
        V: Into<T>,
        D: Into<String>,
    {
        match description {
            Some(description) => ValueWithDesc::WithDesc {
                value: value.into(),
                desc: description.into(),
            },
            None => ValueWithDesc::Value(value.into()),
        }
    }

    fn value(self) -> T {
        match self {
            ValueWithDesc::Value(value) => value,
            ValueWithDesc::WithDesc { value, .. } => value,
        }
    }

    fn desc(&self, mut writer: impl Write) -> io::Result<()> {
        match self {
            ValueWithDesc::Value(value) => write!(writer, "{}", value),
            ValueWithDesc::WithDesc { desc, .. } => write!(writer, "{}", desc),
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

impl<S: Serialize> Serialize for ValueWithDesc<S> {
    fn serialize<Se: serde::Serializer>(&self, serializer: Se) -> Result<Se::Ok, Se::Error> {
        match self {
            ValueWithDesc::Value(value) => value.serialize(serializer),
            ValueWithDesc::WithDesc { value, .. } => value.serialize(serializer),
        }
    }
}

impl Selectable for ValueWithDesc<i64> {
    type Value = i64;
    type Error = <i64 as FromStr>::Err;

    fn value(self) -> i64 {
        self.value()
    }

    fn desc(&self, mut f: impl Write) -> io::Result<()> {
        self.desc(&mut f)
    }

    fn parse(input: &str) -> Result<i64, Self::Error> {
        input.parse()
    }
}

impl Selectable for ValueWithDesc<f64> {
    type Value = f64;
    type Error = <f64 as FromStr>::Err;

    fn value(self) -> f64 {
        self.value()
    }

    fn desc(&self, writer: impl Write) -> io::Result<()> {
        self.desc(writer)
    }

    fn parse(input: &str) -> Result<f64, Self::Error> {
        input.parse()
    }
}

impl Selectable for ValueWithDesc<String> {
    type Value = String;
    type Error = Infallible;

    fn value(self) -> String {
        self.value()
    }

    fn desc(&self, mut writer: impl Write) -> io::Result<()> {
        self.desc(writer)
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
    use super::*;

    use crate::assert_matches;

    use serde_test::{assert_de_tokens, assert_ser_tokens_error, Token};

    // Use this function to get a Select with most fields set to Some.
    fn test_full() -> SelectD<String> {
        SelectD::<String>::new(
            vec![
                ValueWithDesc::new("CE-5", Some("LMB stage 5")),
                ValueWithDesc::new("CE-6", Some("LMB stage 6")),
            ],
            Some(2),
            Some("stage to fight"),
            true,
        )
    }

    // Use this function to get a Select with most fields set to None.
    fn test_none() -> SelectD<String> {
        SelectD::<String>::new(vec!["CE-5", "CE-6"], None, None::<&str>, false)
    }

    #[test]
    fn serde() {
        let values = [test_full(), test_none()];

        assert_de_tokens(
            &values,
            &[
                Token::Seq { len: Some(2) },
                Token::Map { len: Some(4) },
                Token::Str("alternatives"),
                Token::Seq { len: Some(2) },
                Token::Map { len: Some(2) },
                Token::Str("value"),
                Token::Str("CE-5"),
                Token::Str("description"),
                Token::Str("LMB stage 5"),
                Token::MapEnd,
                Token::Map { len: Some(2) },
                Token::Str("value"),
                Token::Str("CE-6"),
                Token::Str("description"),
                Token::Str("LMB stage 6"),
                Token::MapEnd,
                Token::SeqEnd,
                Token::Str("default_index"),
                Token::Some,
                Token::U64(2),
                Token::Str("description"),
                Token::Some,
                Token::Str("stage to fight"),
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
            ],
        );

        assert_ser_tokens_error(
            &values,
            &[Token::Seq { len: Some(2) }, Token::String("CE-6")],
            "can not get default value in batch mode",
        );
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
            } if alternatives == ["CE-5", "CE-6"].into_iter().map(|s| s.into()).collect::<Vec<_>>()
                && description == "a number"
        );

        assert_matches!(
            test_none(),
            SelectD {
                alternatives,
                default_index: None,
                description: None,
                allow_custom: false,
            } if alternatives == vec!["CE-5", "CE-6"].into_iter().map(|s| s.into()).collect::<Vec<_>>()
        )
    }

    #[test]
    fn default() {
        assert_eq!(test_full().default().unwrap(), "CE-6");
        assert_eq!(test_none().default(), None)
    }

    #[test]
    fn batch_default() {
        assert_eq!(test_full().batch_default().unwrap(), "CE-6");
        assert_eq!(test_none().batch_default().unwrap(), "CE-5")
    }

    #[test]
    fn prompt() {
        let mut buffer = Vec::new();

        test_full().prompt(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "1. LMB stage 5\n\
             2. LMB stage 6 [default]\n\
             Please select a stage to fight or input a custom value (empty for default): "
        );
        buffer.clear();

        test_none().prompt(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "1. CE-5\n\
             2. CE-6\n\
             Please select one of the alternatives: "
        );
        buffer.clear();
    }

    #[test]
    fn prompt_no_default() {
        let mut buffer = Vec::new();

        test_full().prompt_no_default(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Default not set, please select a stage to fight or input a custom value: "
        );
        buffer.clear();

        test_none().prompt_no_default(&mut buffer).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Default not set, please select one of the alternatives: "
        );
    }

    #[test]
    fn parse() {
        let select =
            SelectD::<String>::new(["CE-5", "CE-6"], Some(2), Some("a stage to fight"), true);

        assert_eq!(select.parse("").unwrap(), "CE-6");
        assert_eq!(select.parse("1").unwrap(), "CE-5");
        assert_eq!(select.parse("CE-4").unwrap(), "CE-4");
        assert_eq!(
            select.parse("3").unwrap_err(),
            "Index 3 out of range, please try again (1 - 2): "
        );

        let select = SelectD::<f64>::new([1.0, 2.0], Some(2), Some("a float"), false);

        assert_eq!(select.parse("").unwrap(), 2.0);
        assert_eq!(select.parse("1").unwrap(), 1.0);
        assert_eq!(
            select.parse("Invalid").unwrap_err(),
            "Invalid index \"Invalid\", please input an index number (1 - 2)"
        );
    }

    mod selectable {
        use super::*;

        #[test]
        fn int() {
            let value = ValueWithDesc::<i64>::new(1, None::<&str>);
            assert_eq!(value.value(), 1);
            assert_eq!(ValueWithDesc::<i64>::parse("1").unwrap(), 1);
            assert!(ValueWithDesc::<i64>::parse("a").is_err())
        }

        #[test]
        fn float() {
            let value = ValueWithDesc::<f64>::new(1.0, None::<&str>);
            assert_eq!(value.value(), 1.0);
            assert_eq!(ValueWithDesc::<f64>::parse("1.0").unwrap(), 1.0);
            assert!(ValueWithDesc::<f64>::parse("a").is_err())
        }

        #[test]
        fn string() {
            let value = ValueWithDesc::<String>::new("a", None::<&str>);
            assert_eq!(value.value(), "a");
            assert_eq!(ValueWithDesc::<String>::parse("a").unwrap(), "a");
        }
    }
}
