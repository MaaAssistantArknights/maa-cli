use serde::Deserialize;

use crate::{
    primitive::MAAPrimitive,
    userinput::{BoolInput, Input, SelectD, UserInput},
    value::MAAValue,
};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, Debug, PartialEq)]
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
    pub(super) fn into_primitive(self) -> crate::error::Result<MAAPrimitive> {
        use MAAInput::*;
        use MAAPrimitive::*;
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn sstr(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    #[test]
    fn deserialize() {
        use std::num::NonZero;

        use serde_test::{Token, assert_de_tokens};

        let values: Vec<MAAInput> = vec![
            BoolInput::new(Some(true)).into(),
            Input::new(Some(1)).into(),
            Input::new(Some(1.0)).into(),
            Input::new(sstr("1")).into(),
            SelectD::from_iter([1, 2], NonZero::new(2)).unwrap().into(),
            SelectD::from_iter([1.0, 2.0], NonZero::new(2))
                .unwrap()
                .into(),
            SelectD::<String>::from_iter(["1", "2"], NonZero::new(2))
                .unwrap()
                .into(),
        ];

        assert_de_tokens(&values, &[
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
        ]);
    }

    #[test]
    fn to_primitive() {
        use std::num::NonZero;

        assert_eq!(
            MAAInput::from(BoolInput::new(Some(true)))
                .into_primitive()
                .unwrap(),
            true.into()
        );
        assert_eq!(
            MAAInput::InputInt(Input::new(Some(1)))
                .into_primitive()
                .unwrap(),
            1.into()
        );
        assert_eq!(
            MAAInput::InputFloat(Input::new(Some(1.0)))
                .into_primitive()
                .unwrap(),
            1.0.into()
        );
        assert_eq!(
            MAAInput::InputString(Input::new(sstr("1")))
                .into_primitive()
                .unwrap(),
            "1".into()
        );
        assert_eq!(
            MAAInput::SelectInt(SelectD::from_iter([1, 2], NonZero::new(2)).unwrap())
                .into_primitive()
                .unwrap(),
            2.into()
        );
        assert_eq!(
            MAAInput::SelectFloat(SelectD::from_iter([1.0, 2.0], NonZero::new(2)).unwrap())
                .into_primitive()
                .unwrap(),
            2.0.into()
        );

        assert_eq!(
            MAAInput::from(SelectD::<String>::from_iter(["1", "2"], NonZero::new(2)).unwrap())
                .into_primitive()
                .unwrap(),
            "2".into()
        );
    }

    #[test]
    fn from_variants() {
        use std::num::NonZero;

        // Test From implementations for each MAAInput variant
        let input = BoolInput::new(Some(true));
        let maa_input: MAAInput = input.clone().into();
        assert_eq!(maa_input, MAAInput::InputBool(input));

        let input = Input::new(Some(42));
        let maa_input: MAAInput = input.clone().into();
        assert_eq!(maa_input, MAAInput::InputInt(input));

        let select = SelectD::<String>::from_iter(["a", "b"], NonZero::new(1)).unwrap();
        let maa_input: MAAInput = select.clone().into();
        assert_eq!(maa_input, MAAInput::SelectString(select));
    }

    #[test]
    fn input_to_maa_value() {
        use std::num::NonZero;

        // Test conversion from input types to MAAValue
        let input = BoolInput::new(Some(true));
        let value: MAAValue = input.clone().into();
        assert_eq!(value, MAAValue::Input(MAAInput::InputBool(input)));

        let select = SelectD::from_iter([1, 2], NonZero::new(1)).unwrap();
        let value: MAAValue = select.clone().into();
        assert_eq!(value, MAAValue::Input(MAAInput::SelectInt(select)));
    }

    #[test]
    fn to_primitive_all_variants() {
        use std::num::NonZero;

        // Test each variant type once to cover all branches
        assert_eq!(
            MAAInput::InputBool(BoolInput::new(Some(false)))
                .into_primitive()
                .unwrap(),
            false.into()
        );

        assert_eq!(
            MAAInput::InputInt(Input::new(Some(-100)))
                .into_primitive()
                .unwrap(),
            (-100).into()
        );

        assert_eq!(
            MAAInput::InputFloat(Input::new(Some(-2.5)))
                .into_primitive()
                .unwrap(),
            (-2.5).into()
        );

        assert_eq!(
            MAAInput::InputString(Input::new(sstr("hello")))
                .into_primitive()
                .unwrap(),
            "hello".into()
        );

        assert_eq!(
            MAAInput::SelectInt(SelectD::from_iter([10, 20], NonZero::new(1)).unwrap())
                .into_primitive()
                .unwrap(),
            10.into()
        );

        assert_eq!(
            MAAInput::SelectFloat(SelectD::from_iter([1.1, 2.2], NonZero::new(2)).unwrap())
                .into_primitive()
                .unwrap(),
            2.2.into()
        );

        assert_eq!(
            MAAInput::SelectString(
                SelectD::<String>::from_iter(["first", "second"], NonZero::new(1)).unwrap()
            )
            .into_primitive()
            .unwrap(),
            "first".into()
        );
    }
}
