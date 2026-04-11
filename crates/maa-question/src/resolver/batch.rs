use std::collections::HashMap;

use crate::{Question, Resolve};

#[derive(Debug, thiserror::Error)]
pub enum BatchError {
    #[error("Invalid answer for question `{id}`: {message}")]
    InvalidAnswer { id: String, message: String },
}

/// A [`Resolve`] for non-interactive (batch) mode.
///
/// A predefined answer map can be provided to answer questions with matching ids.
///
/// If no matching answer exists, the question's default answer is returned.
#[derive(Debug, Default)]
pub struct BatchResolver {
    predefined_answers: HashMap<String, String>,
}

impl BatchResolver {
    pub fn new(predefined_answers: HashMap<String, String>) -> Self {
        Self { predefined_answers }
    }

    pub fn answers(&self) -> &HashMap<String, String> {
        &self.predefined_answers
    }

    pub fn answers_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.predefined_answers
    }
}

impl<Q: Question> Resolve<Q> for BatchResolver {
    type Error = BatchError;

    fn resolve(&mut self, question: Q) -> Result<Q::Answer, Self::Error> {
        let id = question.id().map(str::to_owned);
        if let Some(id) = id
            && let Some(answer) = self.predefined_answers.get(&id)
        {
            match question.interpret(answer) {
                Ok(answer) => Ok(answer),
                Err((_, message)) => Err(BatchError::InvalidAnswer { id, message }),
            }
        } else {
            Ok(question.default())
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{
        Resolve,
        question::{Confirm, Inquiry},
    };

    #[test]
    fn returns_default_when_no_predefined_answer_exists() {
        let mut resolver = BatchResolver::default();
        assert!(resolver.resolve(Confirm::new(true)).unwrap());
    }

    #[test]
    fn uses_predefined_answer_when_question_id_matches() {
        let mut resolver =
            BatchResolver::new(HashMap::from([(String::from("count"), String::from("42"))]));

        let answer = resolver.resolve(Inquiry::new(0).with_id("count")).unwrap();

        assert_eq!(answer, 42);
    }

    #[test]
    fn invalid_predefined_answer_returns_batch_error() {
        let mut resolver = BatchResolver::new(HashMap::from([(
            String::from("flag"),
            String::from("maybe"),
        )]));

        let error = resolver
            .resolve(Confirm::new(true).with_id("flag"))
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Invalid answer for question `flag`: Invalid input \"maybe\""
        );
    }

    #[test]
    fn answers_mut_allows_updating_predefined_answers() {
        let mut resolver = BatchResolver::default();
        resolver
            .answers_mut()
            .insert(String::from("name"), String::from("maa"));

        let answer = resolver
            .resolve(Inquiry::new(String::new()).with_id("name"))
            .unwrap();

        assert_eq!(resolver.answers().len(), 1);
        assert_eq!(answer, "maa");
    }
}
