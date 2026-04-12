use maa_question::prelude::*;
use serde::Deserialize;

use crate::{
    error::{Error, Result},
    primitive::MAAPrimitive,
    value::MAAValueTemplate,
};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum MAAInput {
    InputString(Inquiry<String>),
    InputBool(Confirm),
    InputInt(Inquiry<i32>),
    InputFloat(Inquiry<f32>),
    SelectInt(SelectD<i32>),
    SelectFloat(SelectD<f32>),
    SelectString(SelectD<String>),
}

/// Resolver capability required to fully resolve any [`MAAInput`].
///
/// This trait groups all question kinds that `maa-value` may emit while resolving
/// [`crate::value::MAAValueTemplate`].
pub trait MAAInputResolver:
    Resolve<Confirm>
    + Resolve<Inquiry<i32>>
    + Resolve<Inquiry<f32>>
    + Resolve<Inquiry<String>>
    + Resolve<SelectD<i32>>
    + Resolve<SelectD<f32>>
    + Resolve<SelectD<String>>
{
}

impl<R> MAAInputResolver for R where
    R: Resolve<Confirm>
        + Resolve<Inquiry<i32>>
        + Resolve<Inquiry<f32>>
        + Resolve<Inquiry<String>>
        + Resolve<SelectD<i32>>
        + Resolve<SelectD<f32>>
        + Resolve<SelectD<String>>
        + ?Sized
{
}

impl MAAInput {
    pub(super) fn into_primitive_with<R>(self, resolver: &mut R) -> Result<MAAPrimitive>
    where
        R: MAAInputResolver + ?Sized,
    {
        use MAAInput::*;
        use MAAPrimitive::*;

        fn into_resolve_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> Error {
            Error::Resolve(Box::new(e))
        }

        match self {
            InputBool(q) => resolver.resolve(q).map(Bool).map_err(into_resolve_error),
            InputInt(q) => resolver.resolve(q).map(Int).map_err(into_resolve_error),
            InputFloat(q) => resolver.resolve(q).map(Float).map_err(into_resolve_error),
            InputString(q) => resolver.resolve(q).map(String).map_err(into_resolve_error),
            SelectInt(q) => resolver.resolve(q).map(Int).map_err(into_resolve_error),
            SelectFloat(q) => resolver.resolve(q).map(Float).map_err(into_resolve_error),
            SelectString(q) => resolver.resolve(q).map(String).map_err(into_resolve_error),
        }
    }
}

impl From<Confirm> for MAAInput {
    fn from(v: Confirm) -> Self {
        Self::InputBool(v)
    }
}

impl From<Inquiry<i32>> for MAAInput {
    fn from(v: Inquiry<i32>) -> Self {
        Self::InputInt(v)
    }
}

impl From<Inquiry<f32>> for MAAInput {
    fn from(v: Inquiry<f32>) -> Self {
        Self::InputFloat(v)
    }
}

impl From<Inquiry<String>> for MAAInput {
    fn from(v: Inquiry<String>) -> Self {
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
            impl From<$t> for MAAValueTemplate {
                fn from(v: $t) -> Self {
                    Self::Input(v.into())
                }
            }
        )*
    };
}

impl_into_maa_value!(
    Confirm,
    Inquiry<i32>,
    Inquiry<f32>,
    Inquiry<String>,
    SelectD<i32>,
    SelectD<f32>,
    SelectD<String>,
);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        use std::num::NonZero;

        use serde_test::{Token, assert_de_tokens};

        let values: Vec<MAAInput> = vec![
            Confirm::new(true).into(),
            Inquiry::new(1).into(),
            Inquiry::new(1.0).into(),
            Inquiry::new("1".to_owned()).into(),
            SelectD::from_iter([1, 2], NonZero::new(2).unwrap())
                .unwrap()
                .into(),
            SelectD::from_iter([1.0, 2.0], NonZero::new(2).unwrap())
                .unwrap()
                .into(),
            SelectD::<String>::from_iter(["1", "2"], NonZero::new(2).unwrap())
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

        let mut ctx = BatchResolver::default();

        assert_eq!(
            MAAInput::from(Confirm::new(true))
                .into_primitive_with(&mut ctx)
                .unwrap(),
            true.into()
        );
        assert_eq!(
            MAAInput::InputInt(Inquiry::new(1))
                .into_primitive_with(&mut ctx)
                .unwrap(),
            1.into()
        );
        assert_eq!(
            MAAInput::InputFloat(Inquiry::new(1.0))
                .into_primitive_with(&mut ctx)
                .unwrap(),
            1.0.into()
        );
        assert_eq!(
            MAAInput::InputString(Inquiry::new("1".to_owned()))
                .into_primitive_with(&mut ctx)
                .unwrap(),
            "1".into()
        );
        assert_eq!(
            MAAInput::SelectInt(SelectD::from_iter([1, 2], NonZero::new(2).unwrap()).unwrap())
                .into_primitive_with(&mut ctx)
                .unwrap(),
            2.into()
        );
        assert_eq!(
            MAAInput::SelectFloat(
                SelectD::from_iter([1.0, 2.0], NonZero::new(2).unwrap()).unwrap()
            )
            .into_primitive_with(&mut ctx)
            .unwrap(),
            2.0.into()
        );
        assert_eq!(
            MAAInput::from(
                SelectD::<String>::from_iter(["1", "2"], NonZero::new(2).unwrap()).unwrap()
            )
            .into_primitive_with(&mut ctx)
            .unwrap(),
            "2".into()
        );
    }

    #[test]
    fn from_variants() {
        use std::num::NonZero;

        let input = Confirm::new(true);
        let maa_input: MAAInput = input.clone().into();
        assert_eq!(maa_input, MAAInput::InputBool(input));

        let input = Inquiry::new(42);
        let maa_input: MAAInput = input.clone().into();
        assert_eq!(maa_input, MAAInput::InputInt(input));

        let select = SelectD::<String>::from_iter(["a", "b"], NonZero::new(1).unwrap()).unwrap();
        let maa_input: MAAInput = select.clone().into();
        assert_eq!(maa_input, MAAInput::SelectString(select));
    }

    #[test]
    fn input_to_maa_value() {
        use std::num::NonZero;

        let input = Confirm::new(true);
        let value: MAAValueTemplate = input.clone().into();
        assert_eq!(value, MAAValueTemplate::Input(MAAInput::InputBool(input)));

        let select = SelectD::from_iter([1, 2], NonZero::new(1).unwrap()).unwrap();
        let value: MAAValueTemplate = select.clone().into();
        assert_eq!(value, MAAValueTemplate::Input(MAAInput::SelectInt(select)));
    }
}
