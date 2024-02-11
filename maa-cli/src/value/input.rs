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
    InputInt(Input<i32>),
    InputFloat(Input<f32>),
    SelectInt(SelectD<i32>),
    SelectFloat(SelectD<f32>),
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

impl From<Input<i32>> for MAAInput {
    fn from(v: Input<i32>) -> Self {
        Self::InputInt(v)
    }
}

impl From<Input<f32>> for MAAInput {
    fn from(v: Input<f32>) -> Self {
        Self::InputFloat(v)
    }
}

impl From<Input<String>> for MAAInput {
    fn from(v: Input<String>) -> Self {
        Self::InputString(v)
    }
}

impl From<SelectD<i32>> for MAAInput {
    fn from(v: SelectD<i32>) -> Self {
        Self::SelectInt(v)
    }
}

impl From<SelectD<f32>> for MAAInput {
    fn from(v: SelectD<f32>) -> Self {
        Self::SelectFloat(v)
    }
}

impl From<SelectD<String>> for MAAInput {
    fn from(v: SelectD<String>) -> Self {
        Self::SelectString(v)
    }
}

macro_rules! impl_into_maa_value {
    ($($t:ty),* $(,)?) => {
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
    Input<i32>,
    Input<f32>,
    Input<String>,
    SelectD<i32>,
    SelectD<f32>,
    SelectD<String>,
    // MAAInput,
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
                Token::I32(1),
                Token::MapEnd,
                Token::Map { len: Some(1) },
                Token::String("default"),
                Token::F32(1.0),
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
                Token::I32(1),
                Token::I32(2),
                Token::SeqEnd,
                Token::MapEnd,
                Token::Map { len: Some(2) },
                Token::String("default_index"),
                Token::U64(2),
                Token::String("alternatives"),
                Token::Seq { len: Some(2) },
                Token::F32(1.0),
                Token::F32(2.0),
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
