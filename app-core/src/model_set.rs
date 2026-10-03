// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The documents of a model.
//!
//! A statechart written the way the authoring tools ask for one is more than a file:
//! it imports an event schema for each event it takes or sends (`<sce:import
//! src="job_completed.scxml" kind="event-schema"/>`) and closes its interface, so the
//! model is a SET of documents that name each other by file name, and the product
//! checks, writes and draws them from one folder.
//!
//! The store keeps text, and a revision is still one file named by its digest. A set
//! is therefore ONE text: a JSON object that holds every document under its file name.
//! A model of one document is the document itself, as it always was, so a model saved
//! before sets existed reads back exactly as it was written. The two are told apart by
//! their first character: an XML document begins with `<`, and a set with `{`.
//!
//! ⚠ The file name of a document is what an import names, so it is kept exactly. It is
//! also written into a folder when SCE is asked about the model, so it is one plain name
//! (letters, digits, `_`, `.`, `-`; no directory, no `..`) and is checked every time a
//! set is made, not only when it is saved.

use serde::{Deserialize, Serialize};

/// The most documents one model holds.
pub const MAX_DOCUMENTS: usize = 64;

/// The longest file name, in characters.
pub const MAX_NAME_CHARS: usize = 100;

/// What a model of one unnamed document is called when it has to be given a name.
pub const SINGLE_NAME: &str = "model.scxml";

const FORMAT: &str = "sce-model-set";
const VERSION: u32 = 1;

/// Why a model cannot be used as a set of documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// What a caller asked to save is not a usable set.
    Invalid(String),
    /// What the store holds is not what it wrote.
    Corrupt(String),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::Invalid(reason) => write!(f, "these documents are not a model: {reason}"),
            ModelError::Corrupt(reason) => {
                write!(f, "the saved model is not what the store wrote: {reason}")
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// One document of a model: the file name its imports know it by, and its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub name: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    format: String,
    v: u32,
    entry: String,
    documents: Vec<Document>,
}

/// The documents of a model, and which of them is the one SCE is asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelFiles {
    /// One document with no name of its own: what a model has always been.
    Single(String),
    /// Documents that name each other, and the entry among them.
    Set {
        entry: String,
        documents: Vec<Document>,
    },
}

impl ModelFiles {
    /// A model of one unnamed document.
    pub fn single(text: impl Into<String>) -> ModelFiles {
        ModelFiles::Single(text.into())
    }

    /// A model of the named `documents`, `entry` being the one a check starts from
    /// (the first when `None`). The set is put in a canonical order, the entry first
    /// and the rest by name, so the same documents are the same bytes however a caller
    /// listed them.
    pub fn set(documents: Vec<Document>, entry: Option<&str>) -> Result<ModelFiles, ModelError> {
        if documents.is_empty() {
            return Err(ModelError::Invalid("there is no document".to_string()));
        }
        if documents.len() > MAX_DOCUMENTS {
            return Err(ModelError::Invalid(format!(
                "{} documents, and a model holds at most {MAX_DOCUMENTS}",
                documents.len()
            )));
        }
        let entry = entry.unwrap_or(&documents[0].name).to_string();
        let mut seen = std::collections::BTreeSet::new();
        for document in &documents {
            check_name(&document.name)?;
            if !seen.insert(document.name.as_str()) {
                return Err(ModelError::Invalid(format!(
                    "two documents are named `{}`",
                    document.name
                )));
            }
        }
        if !seen.contains(entry.as_str()) {
            return Err(ModelError::Invalid(format!(
                "the entry `{entry}` is not one of the documents"
            )));
        }
        let mut ordered = documents;
        ordered.sort_by(|a, b| (a.name != entry, &a.name).cmp(&(b.name != entry, &b.name)));
        Ok(ModelFiles::Set {
            entry,
            documents: ordered,
        })
    }

    /// What a stored model text holds: a set when it is the set's JSON, otherwise the
    /// one document it is.
    pub fn parse(text: &str) -> Result<ModelFiles, ModelError> {
        if !text.trim_start().starts_with('{') {
            return Ok(ModelFiles::Single(text.to_string()));
        }
        let stored: Stored =
            serde_json::from_str(text).map_err(|e| ModelError::Corrupt(e.to_string()))?;
        if stored.format != FORMAT || stored.v != VERSION {
            return Err(ModelError::Corrupt(format!(
                "format `{}` v{}, expected `{FORMAT}` v{VERSION}",
                stored.format, stored.v
            )));
        }
        ModelFiles::set(stored.documents, Some(&stored.entry))
            .map_err(|e| ModelError::Corrupt(e.to_string()))
    }

    /// The text the store keeps: the document itself for one, the set's JSON for a set.
    pub fn stored_text(&self) -> String {
        match self {
            ModelFiles::Single(text) => text.clone(),
            ModelFiles::Set { entry, documents } => {
                let mut text = serde_json::to_string_pretty(&Stored {
                    format: FORMAT.to_string(),
                    v: VERSION,
                    entry: entry.clone(),
                    documents: documents.clone(),
                })
                .expect("a list of strings serialises");
                text.push('\n');
                text
            }
        }
    }

    /// The name SCE is asked about: the entry of a set, and for one document no name
    /// of its own (the work's name is used where a name is wanted).
    pub fn entry_name(&self) -> Option<&str> {
        match self {
            ModelFiles::Single(_) => None,
            ModelFiles::Set { entry, .. } => Some(entry),
        }
    }

    /// The text of the entry document.
    pub fn entry_text(&self) -> &str {
        match self {
            ModelFiles::Single(text) => text,
            ModelFiles::Set { entry, documents } => documents
                .iter()
                .find(|d| d.name == *entry)
                .map_or("", |d| d.text.as_str()),
        }
    }

    /// Every document that is not the entry: what the entry imports, staged beside it.
    pub fn others(&self) -> Vec<&Document> {
        match self {
            ModelFiles::Single(_) => Vec::new(),
            ModelFiles::Set { entry, documents } => {
                documents.iter().filter(|d| d.name != *entry).collect()
            }
        }
    }

    /// Every document, with a name, the entry first: a single document is named
    /// [`SINGLE_NAME`] here, for a reader that wants to show a list.
    pub fn documents(&self) -> Vec<Document> {
        match self {
            ModelFiles::Single(text) => vec![Document {
                name: SINGLE_NAME.to_string(),
                text: text.clone(),
            }],
            ModelFiles::Set { documents, .. } => documents.clone(),
        }
    }

    /// The entry's file name, with [`SINGLE_NAME`] for a single document.
    pub fn entry_file(&self) -> String {
        self.entry_name().unwrap_or(SINGLE_NAME).to_string()
    }
}

/// A document's file name: one plain name an import can refer to and a folder can hold.
pub fn check_name(name: &str) -> Result<(), ModelError> {
    let plain = !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && name != "."
        && name != ".."
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
    if plain {
        Ok(())
    } else {
        Err(ModelError::Invalid(format!(
            "`{name}` is not a file name an import can name: one plain name of letters, digits, `_`, `.` and `-`, \
             at most {MAX_NAME_CHARS} characters, starting with a letter, a digit or `_`"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(name: &str, text: &str) -> Document {
        Document {
            name: name.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn a_model_of_one_document_is_the_document_itself() {
        let xml = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"/>\n";
        let files = ModelFiles::parse(xml).unwrap();
        assert_eq!(files, ModelFiles::Single(xml.to_string()));
        assert_eq!(
            files.stored_text(),
            xml,
            "read back exactly as it was written"
        );
        assert_eq!(files.entry_name(), None);
        assert_eq!(files.entry_file(), SINGLE_NAME);
        assert!(files.others().is_empty());
        assert_eq!(files.documents(), vec![doc(SINGLE_NAME, xml)]);
    }

    #[test]
    fn a_set_survives_its_own_text_and_puts_the_entry_first() {
        let set = ModelFiles::set(
            vec![
                doc("b.scxml", "B"),
                doc("door.scxml", "D"),
                doc("a.scxml", "A"),
            ],
            Some("door.scxml"),
        )
        .unwrap();
        let again = ModelFiles::parse(&set.stored_text()).unwrap();
        assert_eq!(again, set);
        let names: Vec<String> = set.documents().into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["door.scxml", "a.scxml", "b.scxml"]);
        assert_eq!(set.entry_text(), "D");
        assert_eq!(set.others().len(), 2);
    }

    #[test]
    fn the_same_documents_are_the_same_bytes_however_they_were_listed() {
        let one =
            ModelFiles::set(vec![doc("a", "1"), doc("b", "2"), doc("c", "3")], Some("a")).unwrap();
        let two =
            ModelFiles::set(vec![doc("c", "3"), doc("a", "1"), doc("b", "2")], Some("a")).unwrap();
        assert_eq!(one.stored_text(), two.stored_text());
    }

    #[test]
    fn the_entry_defaults_to_the_first_document() {
        let set = ModelFiles::set(vec![doc("z.scxml", "Z"), doc("a.scxml", "A")], None).unwrap();
        assert_eq!(set.entry_name(), Some("z.scxml"));
    }

    #[test]
    fn a_name_is_one_plain_name_and_never_a_path() {
        for bad in [
            "",
            ".",
            "..",
            "../x",
            "a/b",
            "a\\b",
            "-x",
            ".hidden",
            "a b",
            "a\n",
            "\u{e9}.scxml",
            &"x".repeat(MAX_NAME_CHARS + 1),
        ] {
            assert!(check_name(bad).is_err(), "{bad:?}");
            assert!(
                ModelFiles::set(vec![doc(bad, "x")], None).is_err(),
                "{bad:?}"
            );
        }
        for good in ["door.scxml", "job_completed.v2.scxml", "A-1", "_x"] {
            assert!(check_name(good).is_ok(), "{good:?}");
        }
    }

    #[test]
    fn a_set_is_refused_for_what_makes_it_not_one() {
        assert!(ModelFiles::set(vec![], None).is_err());
        assert!(ModelFiles::set(vec![doc("a", "1"), doc("a", "2")], None).is_err());
        assert!(ModelFiles::set(vec![doc("a", "1")], Some("b")).is_err());
        let many: Vec<Document> = (0..=MAX_DOCUMENTS)
            .map(|n| doc(&format!("d{n}"), "x"))
            .collect();
        assert!(ModelFiles::set(many, None).is_err());
    }

    #[test]
    fn a_text_that_looks_like_a_set_and_is_not_ours_is_corrupt() {
        for bad in [
            "{",
            "{}",
            "{\"format\":\"other\",\"v\":1,\"entry\":\"a\",\"documents\":[]}",
            "{\"format\":\"sce-model-set\",\"v\":2,\"entry\":\"a\",\"documents\":[{\"name\":\"a\",\"text\":\"x\"}]}",
            "{\"format\":\"sce-model-set\",\"v\":1,\"entry\":\"b\",\"documents\":[{\"name\":\"a\",\"text\":\"x\"}]}",
            "{\"format\":\"sce-model-set\",\"v\":1,\"entry\":\"../a\",\"documents\":[{\"name\":\"../a\",\"text\":\"x\"}]}",
        ] {
            assert!(
                matches!(ModelFiles::parse(bad), Err(ModelError::Corrupt(_))),
                "{bad:?}"
            );
        }
    }
}
