// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The requirement list of a work.
//!
//! What a specification asks for is not something the product derives: it consumes a
//! closed list (a manifest of ids anchored in the text, and a sidecar holding the
//! words each id quotes) and measures a design against it. The authoring client reads
//! the text and quotes what states each requirement, the authoring package refuses a
//! quote that is not in the text word for word, and the two files it makes are kept
//! here, beside the text they were read from.
//!
//! The store keeps one text per revision, so the two files are ONE JSON text holding
//! both as strings, byte for byte. The manifest's bytes matter: an acceptance pins
//! them by their digest, so a manifest that went through a parser and back would be
//! another manifest. This module never rewrites either file; it checks that they are
//! files at all, and what a manifest says is the product's to refuse.

use serde::{Deserialize, Serialize};

const FORMAT: &str = "sce-requirements";
const VERSION: u32 = 1;

/// Why a requirement list cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementsError {
    /// What a caller asked to save is not a requirement list.
    Invalid(String),
    /// What the store holds is not what it wrote.
    Corrupt(String),
}

impl std::fmt::Display for RequirementsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequirementsError::Invalid(reason) => {
                write!(f, "this is not a requirement list: {reason}")
            }
            RequirementsError::Corrupt(reason) => {
                write!(
                    f,
                    "the saved requirement list is not what the store wrote: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for RequirementsError {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    v: u32,
    manifest: String,
    sidecar: Option<String>,
}

/// The two files of a requirement list, as the authoring package wrote them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirements {
    /// The manifest: the ids and where in the text each is anchored. Pinned by an
    /// acceptance by its bytes.
    pub manifest: String,
    /// The sidecar: the words each id quotes. Read by the acceptance report, which
    /// shows the owner what each requirement says. Absent for a list that came
    /// without one.
    pub sidecar: Option<String>,
}

impl Requirements {
    /// A list of `manifest` and `sidecar`, each checked to be a JSON object: the
    /// shape every file the product reads of them has.
    pub fn new(
        manifest: String,
        sidecar: Option<String>,
    ) -> Result<Requirements, RequirementsError> {
        check_object("manifest", &manifest)?;
        if let Some(sidecar) = &sidecar {
            check_object("sidecar", sidecar)?;
        }
        Ok(Requirements { manifest, sidecar })
    }

    /// What a stored text holds.
    pub fn parse(text: &str) -> Result<Requirements, RequirementsError> {
        let stored: Stored =
            serde_json::from_str(text).map_err(|e| RequirementsError::Corrupt(e.to_string()))?;
        if stored.format != FORMAT || stored.v != VERSION {
            return Err(RequirementsError::Corrupt(format!(
                "format `{}` v{}, expected `{FORMAT}` v{VERSION}",
                stored.format, stored.v
            )));
        }
        Requirements::new(stored.manifest, stored.sidecar)
            .map_err(|e| RequirementsError::Corrupt(e.to_string()))
    }

    /// The text the store keeps: the same list is the same bytes.
    pub fn stored_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(&Stored {
            format: FORMAT.to_string(),
            v: VERSION,
            manifest: self.manifest.clone(),
            sidecar: self.sidecar.clone(),
        })
        .expect("two strings serialise");
        text.push('\n');
        text
    }
}

fn check_object(what: &str, text: &str) -> Result<(), RequirementsError> {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::Object(_)) => Ok(()),
        Ok(_) => Err(RequirementsError::Invalid(format!(
            "the {what} is JSON that is not an object"
        ))),
        Err(e) => Err(RequirementsError::Invalid(format!(
            "the {what} is not JSON: {e}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str =
        "{\n  \"doc_id\": \"door\",\n  \"rev\": \"1\",\n  \"requirements\": []\n}\n";
    const SIDECAR: &str = "{\"doc_id\": \"door\", \"rev\": \"1\", \"text\": {}}\n";

    #[test]
    fn the_two_files_come_back_byte_for_byte() {
        let list = Requirements::new(MANIFEST.to_string(), Some(SIDECAR.to_string())).unwrap();
        let again = Requirements::parse(&list.stored_text()).unwrap();
        assert_eq!(again.manifest, MANIFEST, "an acceptance pins these bytes");
        assert_eq!(again.sidecar.as_deref(), Some(SIDECAR));
        assert_eq!(again, list);
    }

    #[test]
    fn a_list_without_a_sidecar_is_a_list() {
        let list = Requirements::new(MANIFEST.to_string(), None).unwrap();
        assert_eq!(
            Requirements::parse(&list.stored_text()).unwrap().sidecar,
            None
        );
    }

    #[test]
    fn the_same_list_is_the_same_bytes() {
        let one = Requirements::new(MANIFEST.to_string(), Some(SIDECAR.to_string())).unwrap();
        let two = Requirements::new(MANIFEST.to_string(), Some(SIDECAR.to_string())).unwrap();
        assert_eq!(one.stored_text(), two.stored_text());
    }

    #[test]
    fn what_is_not_a_file_of_the_product_is_refused() {
        for bad in ["", "not json", "[]", "3", "\"x\""] {
            assert!(
                matches!(
                    Requirements::new(bad.to_string(), None),
                    Err(RequirementsError::Invalid(_))
                ),
                "manifest {bad:?}"
            );
            assert!(
                matches!(
                    Requirements::new(MANIFEST.to_string(), Some(bad.to_string())),
                    Err(RequirementsError::Invalid(_))
                ),
                "sidecar {bad:?}"
            );
        }
    }

    #[test]
    fn a_text_that_is_not_ours_is_corrupt() {
        for bad in [
            "",
            "{}",
            "{\"format\":\"other\",\"v\":1,\"manifest\":\"{}\",\"sidecar\":null}",
            "{\"format\":\"sce-requirements\",\"v\":2,\"manifest\":\"{}\",\"sidecar\":null}",
            "{\"format\":\"sce-requirements\",\"v\":1,\"manifest\":\"nope\",\"sidecar\":null}",
            "{\"format\":\"sce-requirements\",\"v\":1,\"manifest\":\"{}\",\"sidecar\":null,\"x\":1}",
        ] {
            assert!(
                matches!(
                    Requirements::parse(bad),
                    Err(RequirementsError::Corrupt(_))
                ),
                "{bad:?}"
            );
        }
    }
}
