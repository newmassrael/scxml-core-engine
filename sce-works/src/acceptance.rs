// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the owner accepted, and the folder the product reads it from.
//!
//! An acceptance is the product's record (`sce-codegen accept`): the design's files,
//! the requirement manifest, and what the design was authored from, each pinned by its
//! SHA-256 under a path relative to one root. The product decides whether such a record
//! still holds (`acceptance-check`), and that is the only place "stale" is defined: the
//! application keeps the record and asks, and never works out for itself that something
//! moved.
//!
//! # A fixed layout, so a record can be asked again later
//!
//! A record names its files by path under a root, and the question "does it still hold"
//! is asked about the work as it is NOW. So the work is laid out the same way every time
//! it is put in front of the product ([`Snapshot::stage`]): the model's documents under
//! `design/` by the names their imports know them by, the requirement manifest and the
//! text under `spec/`. A record taken from one staging and checked against another is a
//! comparison of the same paths' bytes, and what moved is said as the paths it moved at.
//!
//! # What the record leaves out
//!
//! The requirement sidecar (the words each id quotes) is shown to the owner and is not
//! pinned by the product. The ids are anchored by sentence in the text, which IS pinned,
//! and a sidecar whose quotes moved away from their sentences is refused when the list is
//! made (a quote must be in the text word for word); the residue is a sidecar edited by
//! hand to quote another sentence of the same text, which this does not notice.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model_set::ModelFiles;
use crate::requirements::Requirements;
use crate::revision::Revision;

/// The variant every acceptance of a work is for. A work has one design.
pub const VARIANT: &str = "base";

/// The channel the application states: the owner pressed its own button.
pub const CHANNEL: &str = "direct";

/// Where the model's documents are staged, under the root.
pub const DESIGN_DIR: &str = "design";

/// Where the text, the requirement list and the owner's answers are staged.
pub const SPEC_DIR: &str = "spec";

const SOURCE_FILE: &str = "source.txt";
const MANIFEST_FILE: &str = "requirements.manifest.json";
const SIDECAR_FILE: &str = "requirements.sidecar.json";
const ANSWERS_FILE: &str = "answers.json";

const FORMAT: &str = "sce-acceptance";
const VERSION: u32 = 1;

/// Why an acceptance cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCorrupt(pub String);

impl std::fmt::Display for AcceptanceCorrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the saved acceptance is not what the store wrote: {}",
            self.0
        )
    }
}

impl std::error::Error for AcceptanceCorrupt {}

/// The revisions the owner was looking at when they accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Basis {
    pub source: Revision,
    pub model: Revision,
    pub requirements: Revision,
    /// Absent when the owner had answered nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Revision>,
}

/// One acceptance, as the store keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    format: String,
    v: u32,
    /// When the owner accepted, by the store's clock.
    pub accepted_at: String,
    /// Which surface stated the acceptance (the product's `channel`).
    pub channel: String,
    /// The revisions of the work's own chains the owner was looking at.
    pub basis: Basis,
    /// The product's record, byte for byte.
    pub record: String,
    /// What the design left to a person when it was accepted, in the product's words:
    /// the record's `open_at_acceptance`, so a reader sees what the owner accepted WITH.
    pub open: Vec<String>,
}

impl Acceptance {
    pub fn new(accepted_at: String, basis: Basis, record: String, open: Vec<String>) -> Acceptance {
        Acceptance {
            format: FORMAT.to_string(),
            v: VERSION,
            accepted_at,
            channel: CHANNEL.to_string(),
            basis,
            record,
            open,
        }
    }

    /// What a stored text holds.
    pub fn parse(text: &str) -> Result<Acceptance, AcceptanceCorrupt> {
        let acceptance: Acceptance =
            serde_json::from_str(text).map_err(|e| AcceptanceCorrupt(e.to_string()))?;
        if acceptance.format != FORMAT || acceptance.v != VERSION {
            return Err(AcceptanceCorrupt(format!(
                "format `{}` v{}, expected `{FORMAT}` v{VERSION}",
                acceptance.format, acceptance.v
            )));
        }
        Ok(acceptance)
    }

    pub fn stored_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("strings serialise");
        text.push('\n');
        text
    }
}

/// The work as the product is asked about it: the design, what it was measured
/// against, and what it was authored from.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub model: ModelFiles,
    pub requirements: Requirements,
    pub source: String,
    /// The owner's answers, when they have given some.
    pub answers: Option<String>,
}

/// A [`Snapshot`] written out under a root, and where each file is.
#[derive(Debug, Clone)]
pub struct Staged {
    pub root: PathBuf,
    /// The entry document, under `design/`.
    pub entry: PathBuf,
    pub manifest: PathBuf,
    pub sidecar: Option<PathBuf>,
    pub source: PathBuf,
    pub answers: Option<PathBuf>,
}

impl Snapshot {
    /// Write the snapshot under `root`, in the fixed layout. `root` is made if it
    /// is not there.
    pub fn stage(&self, root: &Path) -> io::Result<Staged> {
        let design = root.join(DESIGN_DIR);
        let spec = root.join(SPEC_DIR);
        fs::create_dir_all(&design)?;
        fs::create_dir_all(&spec)?;

        // Every document by the name its imports know it by (a single document is
        // `model.scxml`): the names were checked when the model was saved and are
        // checked again where they are about to become paths.
        let documents = self.model.documents();
        for document in &documents {
            crate::model_set::check_name(&document.name)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
            fs::write(design.join(&document.name), &document.text)?;
        }
        let entry = design.join(self.model.entry_file());

        let manifest = spec.join(MANIFEST_FILE);
        fs::write(&manifest, &self.requirements.manifest)?;
        let sidecar = match &self.requirements.sidecar {
            Some(text) => {
                let path = spec.join(SIDECAR_FILE);
                fs::write(&path, text)?;
                Some(path)
            }
            None => None,
        };
        let source = spec.join(SOURCE_FILE);
        fs::write(&source, &self.source)?;
        let answers = match &self.answers {
            Some(text) => {
                let path = spec.join(ANSWERS_FILE);
                fs::write(&path, text)?;
                Some(path)
            }
            None => None,
        };
        Ok(Staged {
            root: root.to_path_buf(),
            entry,
            manifest,
            sidecar,
            source,
            answers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basis() -> Basis {
        let rev = |s: &str| Revision::of(s.as_bytes());
        Basis {
            source: rev("source"),
            model: rev("model"),
            requirements: rev("requirements"),
            answers: None,
        }
    }

    #[test]
    fn an_acceptance_comes_back_as_it_was_kept() {
        let kept = Acceptance::new(
            "2026-10-03T09:00:00Z".to_string(),
            basis(),
            "{\"record\":\"x\"}\n".to_string(),
            vec!["1 question(s) the specification leaves open (q)".to_string()],
        );
        let again = Acceptance::parse(&kept.stored_text()).unwrap();
        assert_eq!(again, kept);
        assert_eq!(again.channel, "direct");
        assert_eq!(again.record, "{\"record\":\"x\"}\n", "the product's bytes");
        // Absent answers are absent, not null.
        assert!(!kept.stored_text().contains("answers"));
    }

    #[test]
    fn a_text_that_is_not_an_acceptance_is_corrupt() {
        for bad in ["", "{}", "[]", "{\"format\":\"other\"}"] {
            assert!(Acceptance::parse(bad).is_err(), "{bad:?}");
        }
        let mut wrong =
            Acceptance::new(String::new(), basis(), String::new(), vec![]).stored_text();
        wrong = wrong.replace("\"v\": 1", "\"v\": 9");
        assert!(Acceptance::parse(&wrong).is_err());
    }

    #[test]
    fn the_work_is_staged_in_the_same_layout_every_time() {
        let dir = std::env::temp_dir().join(format!("sce-acceptance-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let requirements = Requirements::new("{\"doc_id\":\"d\"}".to_string(), None).unwrap();
        let snapshot = Snapshot {
            model: ModelFiles::set(
                vec![
                    crate::model_set::Document {
                        name: "schema.scxml".into(),
                        text: "S".into(),
                    },
                    crate::model_set::Document {
                        name: "door.scxml".into(),
                        text: "D".into(),
                    },
                ],
                Some("door.scxml"),
            )
            .unwrap(),
            requirements,
            source: "text".to_string(),
            answers: Some("{}".to_string()),
        };
        let first = snapshot.stage(&dir.join("one")).expect("staged");
        let second = snapshot.stage(&dir.join("two")).expect("staged");
        for (staged, root) in [(&first, "one"), (&second, "two")] {
            let rel = |p: &Path| p.strip_prefix(dir.join(root)).unwrap().to_path_buf();
            assert_eq!(rel(&staged.entry), Path::new("design/door.scxml"));
            assert_eq!(
                rel(&staged.manifest),
                Path::new("spec/requirements.manifest.json")
            );
            assert_eq!(rel(&staged.source), Path::new("spec/source.txt"));
            assert_eq!(
                staged.answers.as_deref().map(rel),
                Some(PathBuf::from("spec/answers.json"))
            );
            assert!(staged.sidecar.is_none());
            assert_eq!(
                fs::read_to_string(dir.join(root).join("design/schema.scxml")).unwrap(),
                "S"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
