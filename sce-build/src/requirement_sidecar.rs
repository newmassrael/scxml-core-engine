// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The sentences a manifest deliberately does not carry.
//!
//! [`crate::requirement_manifest`] splits a requirement set in two, and
//! says why in its own words: a specification is usually someone else's
//! copyrighted document, a manifest is a checked-in file in a public
//! repository, and a format with a `text` field invites exactly one
//! mistake, makes it silently, and makes it permanent in the history.
//!
//! ```text
//!   manifest   id, section, page — COORDINATES ONLY.  committed.
//!   sidecar    id -> verbatim sentence.               never committed.
//! ```
//!
//! # ⭐ Why this exists now and not earlier
//!
//! The manifest module recorded the reason it stopped where it did:
//! *"The sidecar is not implemented here, because Atomic A has no
//! consumer for it; what Atomic A owes it is a format that leaves room
//! for it."* The acceptance report is that consumer. Its block A puts
//! the verbatim sentence beside the rendered design so that *is this
//! sentence really what the page says* and *does the design do what the
//! sentence says* are answered in one sitting, without moving between
//! artefacts. A block A without the sentence is half a block, so the
//! reader is built rather than the report shipped without it.
//!
//! # What this module refuses, and what it merely reports
//!
//! A sidecar for a different document, or for a different revision of
//! the same one, is REFUSED. Its sentences would be read as
//! confirmation of a design they were never about, which is worse than
//! having no sentence at all — the reviewer's whole job in block A is
//! comparing a sentence to a page.
//!
//! A sidecar that is merely INCOMPLETE is usable, and its gaps are
//! reported per id rather than left blank. A blank where a sentence
//! belongs looks exactly like a sentence nobody needed to check.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::requirement_manifest::RequirementManifest;

/// Requirement id to the sentence the specification states, verbatim.
///
/// ⚠ Loaded only from a path the caller names. It is never discovered
/// beside the manifest by a naming convention: a file found by
/// convention is a file that gets committed by accident, and the whole
/// point of the split is that these sentences never enter the history.
///
/// ⚠⚠ Deliberately NOT `Serialize`. Nothing needs to write a sidecar
/// back out, and a type that can be serialised is one a later caller
/// can dump into a committed artefact in a single line. The manifest
/// module made its half of this split structural rather than advisory
/// — `deny_unknown_fields` refusing a `text` field — and this is the
/// same move from the other side.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementSidecar {
    /// The document these sentences were read out of. Must equal the
    /// manifest's own `doc_id`.
    pub doc_id: String,
    /// The revision they were read at. Must equal the manifest's `rev`.
    pub rev: String,
    /// Requirement id to its verbatim sentence.
    pub text: BTreeMap<String, String>,
}

/// Why a sidecar could not be used at all.
#[derive(Debug)]
pub enum SidecarError {
    Read {
        path: String,
        source: std::io::Error,
    },
    Parse {
        path: String,
        source: serde_json::Error,
    },
    /// No sentences. A sidecar like this reports every requirement as
    /// having no sentence, which reads as "the document says nothing"
    /// rather than "this file was never filled in".
    Empty { path: String },
    /// The sidecar names a different document than the manifest.
    DifferentDocument {
        path: String,
        sidecar: String,
        manifest: String,
    },
    /// Same document, different revision. Refused for the same reason
    /// as a different document: §5.6's freeze is per revision, so a
    /// sentence from another edition is not evidence about this one.
    DifferentRevision {
        path: String,
        doc_id: String,
        sidecar: String,
        manifest: String,
    },
}

impl std::fmt::Display for SidecarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SidecarError::Read { path, source } => {
                write!(f, "{path}: cannot read the sidecar: {source}")
            }
            SidecarError::Parse { path, source } => {
                write!(f, "{path}: cannot parse the sidecar: {source}")
            }
            SidecarError::Empty { path } => write!(
                f,
                "{path}: the sidecar carries no sentences, so every requirement \
                 would report as having none. An unfilled file and a document \
                 that states nothing are not the same thing"
            ),
            SidecarError::DifferentDocument {
                path,
                sidecar,
                manifest,
            } => write!(
                f,
                "{path}: this sidecar holds sentences from '{sidecar}' but the \
                 manifest is for '{manifest}'. Its sentences would be read as \
                 confirming a design they were never about"
            ),
            SidecarError::DifferentRevision {
                path,
                doc_id,
                sidecar,
                manifest,
            } => write!(
                f,
                "{path}: this sidecar holds '{doc_id}' at revision '{sidecar}' \
                 but the manifest is frozen at '{manifest}'. A requirement set \
                 is frozen per revision, so these sentences are not evidence \
                 about that one"
            ),
        }
    }
}

impl std::error::Error for SidecarError {}

/// A gap between a usable sidecar and its manifest. Neither half is
/// fatal, and both must reach the page rather than being dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gap {
    /// The manifest lists this requirement and the sidecar has no
    /// sentence for it. Block A prints the absence in words; it never
    /// leaves the line blank.
    NoSentence { id: String },
    /// The sidecar carries a sentence for an id the manifest does not
    /// list. Dropping it silently would hide either a typo in the id or
    /// a requirement somebody extracted and never registered.
    SentenceForUnlistedId { id: String },
}

impl RequirementSidecar {
    /// Load a sidecar and check it belongs to this manifest.
    pub fn load(path: &Path, manifest: &RequirementManifest) -> Result<Self, SidecarError> {
        let display = path.display().to_string();
        let raw = std::fs::read_to_string(path).map_err(|source| SidecarError::Read {
            path: display.clone(),
            source,
        })?;
        Self::from_json(&raw, &display, manifest)
    }

    /// The parsing and validation half, without the filesystem — so
    /// every refusal below is reachable from a test without writing a
    /// file for each one.
    pub fn from_json(
        raw: &str,
        label: &str,
        manifest: &RequirementManifest,
    ) -> Result<Self, SidecarError> {
        let sidecar: RequirementSidecar =
            serde_json::from_str(raw).map_err(|source| SidecarError::Parse {
                path: label.to_string(),
                source,
            })?;

        if sidecar.text.is_empty() {
            return Err(SidecarError::Empty {
                path: label.to_string(),
            });
        }
        if sidecar.doc_id != manifest.doc_id {
            return Err(SidecarError::DifferentDocument {
                path: label.to_string(),
                sidecar: sidecar.doc_id,
                manifest: manifest.doc_id.clone(),
            });
        }
        if sidecar.rev != manifest.rev {
            return Err(SidecarError::DifferentRevision {
                path: label.to_string(),
                doc_id: sidecar.doc_id,
                sidecar: sidecar.rev,
                manifest: manifest.rev.clone(),
            });
        }
        Ok(sidecar)
    }

    /// The sentence for `id`, or `None` when the sidecar has none.
    pub fn sentence(&self, id: &str) -> Option<&str> {
        self.text.get(id).map(String::as_str)
    }

    /// Every gap between this sidecar and its manifest, in id order.
    ///
    /// Both directions are reported. A requirement with no sentence is
    /// a line block A must still print; a sentence for an unlisted id
    /// is either a mistyped id or an extraction nobody registered, and
    /// both are worth a reader's attention.
    pub fn gaps(&self, manifest: &RequirementManifest) -> Vec<Gap> {
        let listed: BTreeSet<&str> = manifest
            .requirements
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();

        let mut gaps: Vec<Gap> = listed
            .iter()
            .filter(|id| !self.text.contains_key(**id))
            .map(|id| Gap::NoSentence { id: (*id).into() })
            .collect();
        gaps.extend(
            self.text
                .keys()
                .filter(|id| !listed.contains(id.as_str()))
                .map(|id| Gap::SentenceForUnlistedId { id: id.clone() }),
        );
        gaps
    }
}
