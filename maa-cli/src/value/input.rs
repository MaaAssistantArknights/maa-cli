use super::{
    primate::MAAPrimate,
    userinput::{BoolInput, Input, SelectD, UserInput},
    MAAValue,
};

use std::io;

use serde::Deserialize;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum MAAInput {
    InputString(Input<String>),
    InputBool(BoolInput),
    InputInt(Input<i64>),
    InputFloat(Input<f64>),
    SelectInt(SelectD<i64>),
    SelectFloat(SelectD<f64>),
    SelectString(SelectD<String>),
}

impl MAAInput {
    pub(super) fn into_primate(self) -> io::Result<MAAPrimate> {
        use MAAInput::*;
        use MAAPrimate::*;
        match self {
            InputBool(v) => Ok(Bool(v.value()?)),
            InputInt(v) => Ok(Int(v.value()?)),
            InputFloat(v) => Ok(Float(v.value()?)),
            InputString(v) => Ok(String(v.value()?)),
            SelectInt(v) => Ok(Int(v.value()?)),
            SelectFloat(v) => Ok(Float(v.value()?)),
            SelectString(v) => Ok(String(v.value()?)),
        }
    }
}

impl From<BoolInput> for MAAInput {
    fn from(v: BoolInput) -> Self {
        Self::InputBool(v)
    }
}

impl From<Input<i64>> for MAAInput {
    fn from(v: Input<i64>) -> Self {
        Self::InputInt(v)
    }
}

impl From<Input<f64>> for MAAInput {
    fn from(v: Input<f64>) -> Self {
        Self::InputFloat(v)
    }
}

impl From<Input<String>> for MAAInput {
    fn from(v: Input<String>) -> Self {
        Self::InputString(v)
    }
}

impl From<SelectD<i64>> for MAAInput {
    fn from(v: SelectD<i64>) -> Self {
        Self::SelectInt(v)
    }
}

impl From<SelectD<f64>> for MAAInput {
    fn from(v: SelectD<f64>) -> Self {
        Self::SelectFloat(v)
    }
}

impl From<SelectD<String>> for MAAInput {
    fn from(v: SelectD<String>) -> Self {
        Self::SelectString(v)
    }
}

impl From<MAAInput> for MAAValue {
    fn from(v: MAAInput) -> Self {
        Self::Input(v)
    }
}

macro_rules! impl_into_maa_value {
    ($($t:ty),*) => {
        $(
            impl From<$t> for MAAValue {
                fn from(v: $t) -> Self {
                    Self::Input(v.into())
                }
            }
        )*
    };
}

impl_into_maa_value!(
    BoolInput,
    Input<i64>,
    Input<f64>,
    Input<String>,
    SelectD<i64>,
    SelectD<f64>,
    SelectD<String>
);

#[cfg(test)]
mod tests {
    use super::*;

    fn sstr(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    #[test]
    fn deserialize() {
        use serde_test::{assert_de_tokens, Token};

        let values: Vec<MAAInput> = vec![
            BoolInput::new(Some(true), None).into(),
            Input::new(Some(1), None).into(),
            Input::new(Some(1.0), None).into(),
            Input::new(sstr("1"), None).into(),
            SelectD::new([1, 2], Some(2), None, false).unwrap().into(),
            SelectD::new([1.0, 2.0], Some(2), None, false)
                .unwrap()
                .into(),
            SelectD::<String>::new(["1", "2"], Some(2), None, false)
                .unwrap()
                .into(),
        ];

        assert_de_tokens(
            &values,
            &[
                Token::Seq { len: Some(7) },
                Token::Map { len: Some(1) },
                Token::String("default"),
                Token::Bool(true),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::String("default"),
                Token::I64(1),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::String("default"),
                Token::F64(1.0),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::String("default"),
                Token::String("1"),
                Token::MapEnd,
                Token::Map { len: Some(2) },
                Token::String("default_index"),
                Token::U64(2),
                Token::String("alternatives"),
                Token::Seq { len: Some(2) },
                Token::I64(1),
                Token::I64(2),
                Token::SeqEnd,
                Token::MapEnd,
                Token::Map { len: Some(2) },
                Token::String("default_index"),
                Token::U64(2),
                Token::String("alternatives"),
                Token::Seq { len: Some(2) },
                Token::F64(1.0),
                Token::F64(2.0),
                Token::SeqEnd,
                Token::MapEnd,
                Token::Map { len: Some(2) },
                Token::String("default_index"),
                Token::U64(2),
                Token::String("alternatives"),
                Token::Seq { len: Some(2) },
                Token::String("1"),
                Token::String("2"),
                Token::SeqEnd,
                Token::MapEnd,
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn to_primate() {
        assert_eq!(
            MAAInput::from(BoolInput::new(Some(true), None))
                .into_primate()
                .unwrap(),
            true.into()
        );
        assert_eq!(
            MAAInput::InputInt(Input::new(Some(1), None))
                .into_primate()
                .unwrap(),
            1.into()
        );
        assert_eq!(
            MAAInput::InputFloat(Input::new(Some(1.0), None))
                .into_primate()
                .unwrap(),
            1.0.into()
        );
        assert_eq!(
            MAAInput::InputString(Input::new(sstr("1"), None))
                .into_primate()
                .unwrap(),
            "1".into()
        );
        assert_eq!(
            MAAInput::SelectInt(SelectD::new([1, 2], Some(2), None, false).unwrap())
                .into_primate()
                .unwrap(),
            2.into()
        );
        assert_eq!(
            MAAInput::SelectFloat(SelectD::new([1.0, 2.0], Some(2), None, false).unwrap())
                .into_primate()
                .unwrap(),
            2.0.into()
        );

        assert_eq!(
            MAAInput::from(SelectD::<String>::new(["1", "2"], Some(2), None, false).unwrap())
                .into_primate()
                .unwrap(),
            "2".into()
        );
    }
}
