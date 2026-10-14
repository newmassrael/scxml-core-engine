// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The owner's answers to the questions a model leaves open.
//!
//! A model marks what the specification did not decide (`sce:unresolved="<id>"`),
//! and the owner answers each such question once, in their own words. This is the
//! document those answers are kept in. It is the workbench's own: the authoring
//! tools turn it into the decision record the product's checks read, so the format
//! of THAT record stays defined where it is read, and this one stays a plain map
//! from a question's id to what the owner said.
//!
//! ⚠ The stamp of an answer is the time its words last CHANGED, not the time of
//! the save that carried it. Saving ten answers with one edited moves the stamp of
//! one, so the date a decision record gives an answer is the date the owner said it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The most questions one work holds answers for.
pub const MAX_ANSWERS: usize = 500;

/// The longest question id, in characters.
pub const MAX_ID_CHARS: usize = 200;

/// The longest answer, in characters.
pub const MAX_ANSWER_CHARS: usize = 8000;

const FORMAT: &str = "sce-answers";
const VERSION: u32 = 1;

/// Why a set of answers cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswersError {
    /// What a caller asked to save is not answers.
    Invalid(String),
    /// What the store holds is not the document it wrote.
    Corrupt(String),
}

impl std::fmt::Display for AnswersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnswersError::Invalid(reason) => write!(f, "these are not answers: {reason}"),
            AnswersError::Corrupt(reason) => {
                write!(
                    f,
                    "the saved answers are not what the store wrote: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for AnswersError {}

/// One answer: the owner's words, and when they last changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub answer: String,
    pub answered_at: String,
}

#[derive(Serialize, Deserialize)]
struct Document {
    format: String,
    v: u32,
    answers: BTreeMap<String, Entry>,
}

/// The answers of a work, by the id of the question each answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers(BTreeMap<String, Entry>);

impl Answers {
    /// The answers a saved document holds.
    pub fn parse(text: &str) -> Result<Answers, AnswersError> {
        let document: Document =
            serde_json::from_str(text).map_err(|e| AnswersError::Corrupt(e.to_string()))?;
        if document.format != FORMAT || document.v != VERSION {
            return Err(AnswersError::Corrupt(format!(
                "format `{}` v{}, expected `{FORMAT}` v{VERSION}",
                document.format, document.v
            )));
        }
        Ok(Answers(document.answers))
    }

    /// The document that holds these answers: the same bytes for the same answers,
    /// so saving what is already saved is no change.
    pub fn text(&self) -> String {
        let mut text = serde_json::to_string_pretty(&Document {
            format: FORMAT.to_string(),
            v: VERSION,
            answers: self.0.clone(),
        })
        .expect("a map of strings serialises");
        text.push('\n');
        text
    }

    pub fn entries(&self) -> &BTreeMap<String, Entry> {
        &self.0
    }

    /// These answers as `wanted` says they now are, `now` stamping each answer whose
    /// words differ from what is held (or are new) and keeping the stamp of each
    /// that does not. A question that is not in `wanted` is no longer answered.
    pub fn amended(
        &self,
        wanted: &BTreeMap<String, String>,
        now: &str,
    ) -> Result<Answers, AnswersError> {
        if wanted.len() > MAX_ANSWERS {
            return Err(AnswersError::Invalid(format!(
                "{} answers, and a work holds at most {MAX_ANSWERS}",
                wanted.len()
            )));
        }
        let mut next = BTreeMap::new();
        for (id, words) in wanted {
            check_id(id)?;
            let words = words.trim();
            if words.is_empty() {
                return Err(AnswersError::Invalid(format!(
                    "the answer to `{id}` is empty; leave the question out to leave it unanswered"
                )));
            }
            if words.chars().count() > MAX_ANSWER_CHARS {
                return Err(AnswersError::Invalid(format!(
                    "the answer to `{id}` is longer than {MAX_ANSWER_CHARS} characters"
                )));
            }
            let entry = match self.0.get(id) {
                Some(held) if held.answer == words => held.clone(),
                _ => Entry {
                    answer: words.to_string(),
                    answered_at: now.to_string(),
                },
            };
            next.insert(id.clone(), entry);
        }
        Ok(Answers(next))
    }
}

/// A question's id is what a draft cites it by (`sce:assumed="<id>"`): one token,
/// with no space in it.
fn check_id(id: &str) -> Result<(), AnswersError> {
    if id.is_empty() || id.chars().count() > MAX_ID_CHARS {
        return Err(AnswersError::Invalid(format!(
            "a question id is 1 to {MAX_ID_CHARS} characters, not {id:?}"
        )));
    }
    if id.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AnswersError::Invalid(format!(
            "the question id {id:?} holds a space or a control character"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wanted(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn an_answer_is_stamped_when_its_words_change_and_not_when_it_is_carried() {
        let first = Answers::default()
            .amended(&wanted(&[("a", "yes"), ("b", "no")]), "T1")
            .unwrap();
        let second = first
            .amended(&wanted(&[("a", "yes"), ("b", "never")]), "T2")
            .unwrap();
        assert_eq!(second.entries()["a"].answered_at, "T1", "carried, not new");
        assert_eq!(second.entries()["b"].answered_at, "T2", "its words changed");
        assert_eq!(second.entries()["b"].answer, "never");
    }

    #[test]
    fn the_same_answers_are_the_same_bytes() {
        let a = Answers::default()
            .amended(&wanted(&[("b", "2"), ("a", "1")]), "T1")
            .unwrap();
        let again = a.amended(&wanted(&[("a", "1"), ("b", "2")]), "T9").unwrap();
        assert_eq!(
            a.text(),
            again.text(),
            "nothing changed, so nothing is restamped"
        );
    }

    #[test]
    fn a_question_left_out_is_no_longer_answered() {
        let held = Answers::default()
            .amended(&wanted(&[("a", "1"), ("b", "2")]), "T1")
            .unwrap();
        let next = held.amended(&wanted(&[("a", "1")]), "T2").unwrap();
        assert_eq!(next.entries().keys().collect::<Vec<_>>(), ["a"]);
    }

    #[test]
    fn the_words_are_trimmed_and_cannot_be_empty() {
        let kept = Answers::default()
            .amended(&wanted(&[("a", "  yes \n")]), "T")
            .unwrap();
        assert_eq!(kept.entries()["a"].answer, "yes");
        for empty in ["", "  \n\t"] {
            assert!(matches!(
                Answers::default().amended(&wanted(&[("a", empty)]), "T"),
                Err(AnswersError::Invalid(_))
            ));
        }
    }

    #[test]
    fn an_id_is_one_token_a_draft_can_cite() {
        for bad in ["", "a b", "a\tb", "a\nb", &"x".repeat(MAX_ID_CHARS + 1)] {
            assert!(
                matches!(
                    Answers::default().amended(&wanted(&[(bad, "yes")]), "T"),
                    Err(AnswersError::Invalid(_))
                ),
                "{bad:?}"
            );
        }
        assert!(Answers::default()
            .amended(&wanted(&[("open-guard", "yes")]), "T")
            .is_ok());
    }

    #[test]
    fn the_size_of_what_is_kept_is_bounded() {
        let long = "x".repeat(MAX_ANSWER_CHARS + 1);
        assert!(Answers::default()
            .amended(&wanted(&[("a", &long)]), "T")
            .is_err());
        let many: BTreeMap<String, String> = (0..=MAX_ANSWERS)
            .map(|n| (format!("q{n}"), "yes".to_string()))
            .collect();
        assert!(Answers::default().amended(&many, "T").is_err());
    }

    #[test]
    fn a_document_that_is_not_ours_is_corrupt() {
        let held = Answers::default()
            .amended(&wanted(&[("a", "1")]), "T")
            .unwrap();
        assert_eq!(Answers::parse(&held.text()).unwrap(), held);
        for bad in [
            "",
            "[]",
            "{\"format\":\"other\",\"v\":1,\"answers\":{}}",
            "{\"format\":\"sce-answers\",\"v\":2,\"answers\":{}}",
        ] {
            assert!(
                matches!(Answers::parse(bad), Err(AnswersError::Corrupt(_))),
                "{bad:?}"
            );
        }
    }
}
