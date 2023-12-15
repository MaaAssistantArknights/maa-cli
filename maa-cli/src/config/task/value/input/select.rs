use super::UserInput;

use std::{
    convert::Infallible,
    fmt::Display,
    io::{self, Write},
    str::FromStr,
};

use serde::Deserialize;

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
            write!(writer, "{}. {}", i + 1, alternative.display())?;
            if self.default_index.is_some_and(|d| d == i + 1) {
                writeln!(writer, " (default)")?;
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

pub trait Selectable {
    type Value;
    type Display: Display + ?Sized;
    type Error;

    /// Get the value of this element, consum self.
    fn value(self) -> Self::Value;

    /// Get the description of this element, not consum self.
    fn display(&self) -> &Self::Display;

    /// Parse a string to value of this element.
    fn parse(input: &str) -> Result<Self::Value, Self::Error>;
}

impl Selectable for i64 {
    type Value = Self;
    type Display = Self;
    type Error = <i64 as FromStr>::Err;

    fn value(self) -> Self::Value {
        self
    }

    fn display(&self) -> &Self::Display {
        self
    }

    fn parse(input: &str) -> Result<Self::Value, Self::Error> {
        input.parse()
    }
}

impl Selectable for f64 {
    type Value = Self;
    type Display = Self;
    type Error = <f64 as FromStr>::Err;

    fn value(self) -> Self::Value {
        self
    }

    fn display(&self) -> &Self::Display {
        self
    }

    fn parse(input: &str) -> Result<Self::Value, Self::Error> {
        input.parse()
    }
}

impl Selectable for String {
    type Value = Self;
    type Display = str;
    type Error = Infallible;

    fn value(self) -> Self::Value {
        self
    }

    fn display(&self) -> &Self::Display {
        self
    }

    fn parse(input: &str) -> Result<Self::Value, Self::Error> {
        Ok(input.to_owned())
    }
}

use crate::run::RoguelikeTheme;

impl Selectable for RoguelikeTheme {
    type Value = String;
    type Display = str;
    type Error = Infallible;

    fn value(self) -> Self::Value {
        self.to_str().to_owned()
    }

    fn display(&self) -> &Self::Display {
        self.to_str()
    }

    fn parse(input: &str) -> Result<Self::Value, Self::Error> {
        Ok(input.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_matches;

    use super::*;

    #[test]
    fn construct() {
        assert_matches!(
            Select::<String>::new(
                vec!["CE-5", "CE-6"],
                Some(2),
                Some("stage to fight"),
                true,
            ),
            Select {
                alternatives,
                default_index: Some(1),
                description: Some(description),
                allow_custom: true,
            } if alternatives == vec![String::from("CE-5"), String::from("CE-6")]
                && description == "a number"
        );

        assert_matches!(
            Select::<String>::new(
                vec!["CE-5", "CE-6"],
                None,
                None::<String>,
                false,
            ),
            Select {
                alternatives,
                default_index: None,
                description: None,
                allow_custom: false,
            } if alternatives == vec![String::from("CE-5"), String::from("CE-6")]
        )
    }

    #[test]
    fn default() {
        assert_eq!(
            Select::<String>::new(vec!["CE-5", "CE-6"], Some(2), Some("stage to fight"), true,)
                .default(),
            Some(String::from("CE-6"))
        );

        assert_eq!(
            Select::<String>::new(vec!["CE-5", "CE-6"], None, None::<String>, false,).default(),
            None
        )
    }

    #[test]
    fn batch_default() {
        assert_eq!(
            Select::<String>::new(vec!["CE-5", "CE-6"], Some(2), Some("stage to fight"), true,)
                .batch_default(),
            Some(String::from("CE-5"))
        );

        assert_eq!(
            Select::<String>::new(vec!["CE-5", "CE-6"], None, None::<String>, false,)
                .batch_default(),
            Some(String::from("CE-5"))
        )
    }

    #[test]
    fn prompt() {
        let mut buffer = Vec::new();

        Select::<String>::new(
            vec!["CE-5", "CE-6"],
            Some(2),
            Some("a stage to fight"),
            true,
        )
        .prompt(&mut buffer)
        .unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "1. CE-5\n\
             2. CE-6 (default)\n\
             Please select a stage to fight or input a custom value (empty for default): "
        );
        buffer.clear();

        Select::<String>::new(vec!["CE-5", "CE-6"], None, None::<String>, false)
            .prompt(&mut buffer)
            .unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "1. CE-5\n\
             2. CE-6\n\
             Please select one of the alternatives: "
        );
    }

    #[test]
    fn prompt_no_default() {
        let mut buffer = Vec::new();

        Select::<String>::new(
            vec!["CE-5", "CE-6"],
            Some(2),
            Some("a stage to fight"),
            true,
        )
        .prompt_no_default(&mut buffer)
        .unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Default not set, please select a stage to fight or input a custom value: "
        );
        buffer.clear();

        Select::<String>::new(vec!["CE-5", "CE-6"], None, None::<String>, false)
            .prompt_no_default(&mut buffer)
            .unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "Default not set, please select one of the alternatives: "
        );
    }

    #[test]
    fn parse() {
        let select = Select::<String>::new(
            vec!["CE-5", "CE-6"],
            Some(2),
            Some("a stage to fight"),
            true,
        );

        assert_eq!(select.parse("").unwrap(), "CE-6");
        assert_eq!(select.parse("1").unwrap(), "CE-5");
        assert_eq!(select.parse("CE-4").unwrap(), "CE-4");
        assert_eq!(
            select.parse("3").unwrap_err(),
            "Index 3 out of range, please try again (1 - 2): "
        );

        let select = Select::<RoguelikeTheme>::new(
            vec![
                RoguelikeTheme::Phantom,
                RoguelikeTheme::Mizuki,
                RoguelikeTheme::Sami,
            ],
            Some(3),
            Some("a roguelike theme"),
            false,
        );

        assert_eq!(select.parse("").unwrap(), "Sami");
        assert_eq!(select.parse("1").unwrap(), "Phantom");
        assert_eq!(
            select.parse("Mizuki").unwrap_err(),
            "Invalid index \"Mizuki\", please input an index number (1 - 3)"
        );
    }

    mod selectable {}
}
