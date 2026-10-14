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
/// A list with no lineage: written as it always was, so every digest already kept, and every
/// acceptance that pins one, is undisturbed.
const VERSION: u32 = 1;
/// A list with a lineage. A build that predates the lineage refuses it as a list it does not
/// know (an unknown field and an unknown version), and does not read it as one without.
const LINEAGE_VERSION: u32 = 2;
/// What a lineage says of itself, which is all this module checks of one: what a lineage MEANS is
/// the authoring package's to define, and two implementations of that would be two definitions.
const LINEAGE_KIND: &str = "sce-requirement-lineage";

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
    /// Present exactly when `v` is [`LINEAGE_VERSION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lineage: Option<String>,
}

/// The files of a requirement list, as the authoring package wrote them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirements {
    /// The manifest: the ids and where in the text each is anchored. Pinned by an
    /// acceptance by its bytes.
    pub manifest: String,
    /// The sidecar: the words each id quotes. Read by the acceptance report, which
    /// shows the owner what each requirement says. Absent for a list that came
    /// without one.
    pub sidecar: Option<String>,
    /// The lineage: which id was issued for which requirement across the revisions of the
    /// specification, as the authoring package returned it, byte for byte. It holds hashes
    /// and numbers and no word of the specification. What the NEXT revision of the text is
    /// built against, so that an id is issued once and never again
    /// (`docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`).
    /// Absent for a list made before lineages, or by a client that did not give one.
    ///
    /// Not staged for the product: an acceptance pins the manifest's bytes, and the product
    /// never reads a lineage.
    pub lineage: Option<String>,
}

impl Requirements {
    /// A list of `manifest` and `sidecar`, each checked to be a JSON object: the
    /// shape every file the product reads of them has. It has no lineage; see
    /// [`Requirements::with_lineage`].
    pub fn new(
        manifest: String,
        sidecar: Option<String>,
    ) -> Result<Requirements, RequirementsError> {
        check_object("manifest", &manifest)?;
        if let Some(sidecar) = &sidecar {
            check_object("sidecar", sidecar)?;
        }
        Ok(Requirements {
            manifest,
            sidecar,
            lineage: None,
        })
    }

    /// The same list with `lineage`, which has to be a JSON object that says it is a
    /// requirement lineage. `None` is a list without one.
    pub fn with_lineage(
        mut self,
        lineage: Option<String>,
    ) -> Result<Requirements, RequirementsError> {
        if let Some(lineage) = &lineage {
            check_lineage(lineage)?;
        }
        self.lineage = lineage;
        Ok(self)
    }

    /// The lineage a list made before lineages stands on: the one adopting it makes (its ids,
    /// the words behind them from its sidecar, the revision its manifest says), which is what a
    /// later list is built against in the first place (ADR 0011 item 7). The core derives it, and
    /// the client is not left to: a client that is not told renumbers the list from a first-list
    /// build, and the first lineage of a work was then judged on its own (review of 2026-10-09).
    ///
    /// Derived and never stored: the same bytes of a manifest and a sidecar are the same lineage,
    /// and the list is left as it was, so that an acceptance that pins it still does. `None` for
    /// a list that keeps a lineage, and for one that cannot be adopted (it keeps no sidecar, so
    /// the words behind its ids are not known, or its ids are the source's own): the weaker
    /// guarantee ADR 0011 names remains for those, and nothing is said that is not known.
    pub fn adopted_lineage(&self) -> Option<serde_json::Value> {
        if self.lineage.is_some() {
            return None;
        }
        let sidecar = self.sidecar.as_deref()?;
        sce_revision::of_list(
            &serde_json::json!({ "manifest_text": self.manifest, "sidecar_text": sidecar }),
            "the work keeps",
        )
        .ok()
    }

    /// What a stored text holds: a list without a lineage (`v` 1) or with one (`v` 2).
    pub fn parse(text: &str) -> Result<Requirements, RequirementsError> {
        let stored: Stored =
            serde_json::from_str(text).map_err(|e| RequirementsError::Corrupt(e.to_string()))?;
        let corrupt = |why: String| RequirementsError::Corrupt(why);
        let lineage = match (stored.v, stored.lineage) {
            (VERSION, None) => None,
            (LINEAGE_VERSION, Some(lineage)) => Some(lineage),
            (VERSION, Some(_)) => {
                return Err(corrupt(format!("a v{VERSION} list holds a lineage")));
            }
            (LINEAGE_VERSION, None) => {
                return Err(corrupt(format!(
                    "a v{LINEAGE_VERSION} list holds no lineage"
                )));
            }
            (other, _) => {
                return Err(corrupt(format!(
                    "format `{}` v{other}, expected `{FORMAT}` v{VERSION} or v{LINEAGE_VERSION}",
                    stored.format
                )));
            }
        };
        if stored.format != FORMAT {
            return Err(corrupt(format!(
                "format `{}` v{}, expected `{FORMAT}`",
                stored.format, stored.v
            )));
        }
        Requirements::new(stored.manifest, stored.sidecar)
            .and_then(|list| list.with_lineage(lineage))
            .map_err(|e| corrupt(e.to_string()))
    }

    /// The text the store keeps: the same list is the same bytes, and a list without a
    /// lineage is the bytes it was before lineages existed.
    pub fn stored_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(&Stored {
            format: FORMAT.to_string(),
            v: if self.lineage.is_some() {
                LINEAGE_VERSION
            } else {
                VERSION
            },
            manifest: self.manifest.clone(),
            sidecar: self.sidecar.clone(),
            lineage: self.lineage.clone(),
        })
        .expect("strings serialise");
        text.push('\n');
        text
    }
}

/// A lineage as it is read from a caller: a JSON object that names itself. Nothing more is
/// asked here: the record is a text the store keeps, and whether it is a lineage, the list's own
/// and the work's continued is judged where a list is saved or published
/// (`WorkStore::refuse_a_lineage_not_kept`, by `sce-revision`), so that a record this build
/// reads stays readable whatever it says.
fn check_lineage(text: &str) -> Result<(), RequirementsError> {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::Object(object))
            if object.get("lineage").and_then(|v| v.as_str()) == Some(LINEAGE_KIND) =>
        {
            Ok(())
        }
        Ok(serde_json::Value::Object(_)) => Err(RequirementsError::Invalid(format!(
            "the lineage does not say it is a `{LINEAGE_KIND}`"
        ))),
        Ok(_) => Err(RequirementsError::Invalid(
            "the lineage is JSON that is not an object".to_string(),
        )),
        Err(e) => Err(RequirementsError::Invalid(format!(
            "the lineage is not JSON: {e}"
        ))),
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

    const LINEAGE: &str = "{\"lineage\":\"sce-requirement-lineage\",\"v\":1,\"doc_id\":\"door\",\
                           \"next\":2,\"revisions\":[],\"requirements\":[]}";

    #[test]
    fn a_list_without_a_lineage_is_the_bytes_it_was_before_lineages() {
        let list = Requirements::new(MANIFEST.to_string(), Some(SIDECAR.to_string())).unwrap();
        let text = list.stored_text();
        assert!(text.contains("\"v\": 1"), "{text}");
        assert!(!text.contains("lineage"), "{text}");
        assert_eq!(Requirements::parse(&text).unwrap().lineage, None);
    }

    #[test]
    fn a_lineage_comes_back_byte_for_byte_and_is_a_v2_list() {
        let list = Requirements::new(MANIFEST.to_string(), Some(SIDECAR.to_string()))
            .unwrap()
            .with_lineage(Some(LINEAGE.to_string()))
            .unwrap();
        let text = list.stored_text();
        assert!(text.contains("\"v\": 2"), "{text}");
        let again = Requirements::parse(&text).unwrap();
        assert_eq!(again.lineage.as_deref(), Some(LINEAGE));
        assert_eq!(again.manifest, MANIFEST);
        assert_eq!(again, list);
        assert_eq!(again.stored_text(), text);
    }

    #[test]
    fn a_lineage_changes_the_text_and_so_the_revision_of_the_list() {
        let bare = Requirements::new(MANIFEST.to_string(), None).unwrap();
        let held = bare
            .clone()
            .with_lineage(Some(LINEAGE.to_string()))
            .unwrap();
        assert_ne!(bare.stored_text(), held.stored_text());
    }

    #[test]
    fn what_is_not_a_lineage_is_refused() {
        for bad in [
            "",
            "not json",
            "[]",
            "3",
            "{}",
            "{\"lineage\":\"something-else\"}",
            "{\"lineage\":7}",
        ] {
            assert!(
                matches!(
                    Requirements::new(MANIFEST.to_string(), None)
                        .unwrap()
                        .with_lineage(Some(bad.to_string())),
                    Err(RequirementsError::Invalid(_))
                ),
                "lineage {bad:?}"
            );
        }
    }

    #[test]
    fn a_version_and_a_lineage_that_disagree_are_corrupt() {
        let lineage = serde_json::to_string(LINEAGE).unwrap();
        let v1_with = format!(
            "{{\"format\":\"sce-requirements\",\"v\":1,\"manifest\":\"{{}}\",\"sidecar\":null,\"lineage\":{lineage}}}"
        );
        let v2_without =
            "{\"format\":\"sce-requirements\",\"v\":2,\"manifest\":\"{}\",\"sidecar\":null}";
        for bad in [v1_with.as_str(), v2_without] {
            assert!(
                matches!(Requirements::parse(bad), Err(RequirementsError::Corrupt(_))),
                "{bad:?}"
            );
        }
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
