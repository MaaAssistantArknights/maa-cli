#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

/// A question whose answer is decided at runtime.
///
/// An implementor holds the *state* of a single question:
/// its optional stable identifier, its required default answer, and
/// how to interpret a raw string typed by the user.
pub trait Question: Sized {
    /// The type that a successful answer produces.
    type Answer;

    /// A stable string identifier used to look up pre-supplied values
    ///
    /// Returns `None` when no id was set.
    fn id(&self) -> Option<&str>;

    /// Consume `self` and return the default answer.
    fn default(self) -> Self::Answer;

    /// Interpret `input` as an answer for this question.
    ///
    /// If the input is valid, returns the interpreted answer. Otherwise, given the ownership
    /// back alongside with an error message, so a resolver can re-ask the question or raise an
    /// error.
    fn interpret(self, input: &str) -> Result<Self::Answer, (Self, String)>;
}

/// Trait for how a resolver answers a concrete question type.
pub trait Resolve<Q: Question> {
    /// Error type produced by this resolver when resolving `Q`.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Resolve a `question` and return an answer or an error.
    fn resolve(&mut self, question: Q) -> Result<Q::Answer, Self::Error>;
}

pub mod question;
pub mod resolver;

/// Convenience re-exports for the most commonly used items.
///
/// ```
/// use maa_question::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        Question, Resolve,
        question::{Confirm, Inquiry, Select, SelectD, Selectable, ValueWithDesc},
        resolver::{
            batch::{BatchError, BatchResolver},
            io::{Io, IoResolver, PromptIo, StdIo},
        },
    };
}
