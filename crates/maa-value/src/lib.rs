#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// Allow the proc-macro to reference types via `maa_value::` path even inside this crate
extern crate self as maa_value;

pub mod array;
pub mod convert;
pub mod error;
pub mod input;
pub mod map;
pub mod primitive;
pub mod userinput;
pub mod value;

pub mod prelude {
    pub use maa_value_macro::{insert, object};

    pub use crate::{
        array::ArrayOps,
        convert::{AsPrimitive, TryAs},
        input::MAAInput,
        map::MapOps,
        primitive::MAAPrimitive,
        userinput::{BoolInput, Input, Select, SelectD, Selectable, UserInput, ValueWithDesc},
        value::{MAAValue, ResolvedMAAValue},
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Outcome<V, O> {
    Value(V),
    Original(O),
}
