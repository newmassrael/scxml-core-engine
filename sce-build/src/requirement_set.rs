// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! More than one manifest at once, so a cross-document claim can be
//! asked about — Requirement-closure RFC §5.2e/§5.2f.
//!
//! [`crate::requirement_manifest`] models one document's requirements.
//! Two of its claims point OUT of that document and so cannot be judged
//! from inside it:
//!
//! - `delegated` names `(to_doc, to_id)`: the requirement is met over
//!   there. The manifest checks only that the naming is not blank and
//!   not self-referential, which stops an EMPTY stamp, not a WRONG one.
//! - decomposition names the children a requirement is split into, which
//!   may live in another document entirely.
//!
//! Both need a set. That is this module: manifests keyed by `doc_id`,
//! and the two walks over it.
//!
//! ## What "arrived" means, and the two questions that sank the first try
//!
//! RFC §5.2e records a draft arrival check that was written and removed
//! before landing. It took ONE target manifest, and it broke on two
//! questions this module answers by construction rather than by policy:
//!
//! 1. *It reported every delegation to any other document as broken.*
//!    With one target, "not that document" and "not anywhere" are the
//!    same observation. Here the answer is three-valued: a delegation to
//!    a document the set does not hold is [`Arrival::Unchecked`], which
//!    is neither pass nor fail but a statement that the set was too
//!    small to answer. ⚠ It is COUNTED and reported, never silent — a
//!    check that quietly passed what it could not see would be defeated
//!    by leaving a manifest off the command line, which is the cheapest
//!    possible way to defeat it.
//! 2. *It left "arrived" undefined when the target is itself
//!    `out_of_scope` or delegated onward.* Conflating those with the
//!    destination is what made it undefinable. Arrival is PRESENCE — the
//!    named document carries the named id — and what that entry then
//!    says is a separate fact, reported separately as the chain's
//!    terminus. A delegation into an `out_of_scope` entry arrived; that
//!    it died there is true as well, and a reader wants both.
//!
//! ## The failure this exists to catch
//!
//! `delegated` is a "not our problem" stamp. Unchecked, it can be
//! pressed anywhere. The sharpest shape is not a typo but a CYCLE: `A`
//! delegates to `B` and `B` delegates back to `A`. Every row says the
//! requirement is met elsewhere, every naming resolves, and the
//! requirement is met NOWHERE. A check that stopped at one hop would
//! call both ends arrived, so the walk follows the chain and refuses a
//! document it has already stood in.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::requirement_manifest::{Disposition, RequirementEntry, RequirementManifest};

/// One requirement, named from outside the document that holds it.
///
/// ⭐ The single shape for "a requirement over there", so the delegation
/// walk and the decomposition walk resolve through one function rather
/// than two that can drift. `Disposition::Delegated` spells the same
/// pair as `to_doc`/`to_id` on the wire and converts here — the wire
/// keeps the field names it shipped with, and the QUERY path is shared.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct RequirementRef {
    /// The `doc_id` of the manifest that carries it.
    pub doc: String,
    /// The id it is spelled as in that document. Named separately
    /// because a layered standard renumbers — the reason
    /// [`Disposition::Delegated`] gives for `to_id`.
    pub id: String,
}

impl fmt::Display for RequirementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.id, self.doc)
    }
}

/// Where a chain of delegations ended, once it stopped delegating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminus {
    /// Met in that document by its own evidence.
    Implemented,
    /// Reached, and declared not carryable there.
    ///
    /// ⚠ Arrived and dead are BOTH true. Reported as its own terminus
    /// rather than folded into arrival, because a reviewer reading "the
    /// delegation resolves" would otherwise never learn that the chain
    /// ends in a refusal.
    OutOfScope { reason: String },
    /// Reached, and realised by the deployment rather than behaviour.
    SystemLevel { realised_by: String },
}

/// What became of one `delegated` claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arrival {
    /// The named document carries the named id. `hops` is the chain
    /// walked, so a reader can see a delegation that travelled.
    Arrived {
        at: RequirementRef,
        hops: Vec<RequirementRef>,
        terminus: Terminus,
    },
    /// The named document IS in the set and does NOT carry the id.
    ///
    /// This is the defect the row exists for: a stamp pressed at a
    /// destination that never agreed to take it.
    NotArrived { at: RequirementRef },
    /// The chain returned to a document it had already stood in.
    ///
    /// Every link resolves and the requirement is met nowhere.
    Cycle { path: Vec<RequirementRef> },
    /// The named document is not in the set, so the set cannot answer.
    ///
    /// ⚠ Neither pass nor fail. [`SetReport::unchecked`] counts these so
    /// a caller cannot mistake "not asked" for "asked and fine".
    Unchecked { at: RequirementRef },
}

impl Arrival {
    /// Whether this verdict is a defect in the manifests.
    ///
    /// `Unchecked` is deliberately NOT one: the set was too small, which
    /// is a fact about the invocation and not about the documents.
    pub fn is_defect(&self) -> bool {
        matches!(self, Arrival::NotArrived { .. } | Arrival::Cycle { .. })
    }

    /// A short name for the wire and for messages.
    pub fn as_str(&self) -> &'static str {
        match self {
            Arrival::Arrived { .. } => "arrived",
            Arrival::NotArrived { .. } => "not-arrived",
            Arrival::Cycle { .. } => "cycle",
            Arrival::Unchecked { .. } => "unchecked",
        }
    }
}

impl fmt::Display for Arrival {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arrival::Arrived { at, terminus, .. } => match terminus {
                Terminus::Implemented => write!(f, "arrived at {at}"),
                Terminus::OutOfScope { reason } => {
                    write!(f, "arrived at {at}, which is out of scope: {reason}")
                }
                Terminus::SystemLevel { realised_by } => {
                    write!(f, "arrived at {at}, realised by {realised_by}")
                }
            },
            Arrival::NotArrived { at } => write!(
                f,
                "does not arrive: {} carries no requirement {}",
                at.doc, at.id
            ),
            Arrival::Cycle { path } => write!(
                f,
                "delegation cycle: {}",
                path.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ),
            Arrival::Unchecked { at } => write!(
                f,
                "not checked: no manifest for {} was given to this run",
                at.doc
            ),
        }
    }
}

/// One requirement's cross-document verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossDocVerdict {
    /// The requirement making the claim.
    pub from: RequirementRef,
    /// What the claim was and what became of it.
    pub arrival: Arrival,
}

impl fmt::Display for CrossDocVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.from, self.arrival)
    }
}

/// Why a set could not be built.
#[derive(Debug)]
pub enum SetError {
    /// Two manifests claim the same `doc_id`.
    ///
    /// Refused rather than last-wins: a set that silently dropped one
    /// would answer every question about that document from the other,
    /// and the answers would be confident and wrong.
    DuplicateDoc { doc_id: String },
}

impl fmt::Display for SetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SetError::DuplicateDoc { doc_id } => write!(
                f,
                "two manifests both claim doc-id {doc_id}; a set cannot hold both"
            ),
        }
    }
}

impl std::error::Error for SetError {}

impl SetError {
    /// Which refusal this is, in words carrying no path — the shape
    /// [`crate::requirement_manifest::ManifestError::kind`] uses.
    pub fn kind(&self) -> &'static str {
        match self {
            SetError::DuplicateDoc { .. } => "duplicate-doc-id",
        }
    }
}

/// What a whole set measured.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetReport {
    /// Every cross-document claim, in a stable order.
    pub verdicts: Vec<CrossDocVerdict>,
}

impl SetReport {
    /// Claims that are defects in the documents.
    pub fn defects(&self) -> impl Iterator<Item = &CrossDocVerdict> {
        self.verdicts.iter().filter(|v| v.arrival.is_defect())
    }

    /// Claims the set was too small to judge.
    ///
    /// ⚠ Published beside [`Self::defects`] on purpose. A caller that
    /// reads only the defect count and finds zero has learned nothing
    /// until it also reads this: zero defects out of zero questions
    /// asked is what an empty set returns.
    pub fn unchecked(&self) -> impl Iterator<Item = &CrossDocVerdict> {
        self.verdicts
            .iter()
            .filter(|v| matches!(v.arrival, Arrival::Unchecked { .. }))
    }

    /// Claims that were actually answered — the denominator a zero
    /// defect count is only meaningful against.
    pub fn answered(&self) -> usize {
        self.verdicts
            .iter()
            .filter(|v| !matches!(v.arrival, Arrival::Unchecked { .. }))
            .count()
    }
}

/// Several manifests, keyed by the `doc_id` each one declares.
#[derive(Debug, Clone, Default)]
pub struct RequirementSet {
    by_doc: BTreeMap<String, RequirementManifest>,
}

impl RequirementSet {
    /// Build a set, refusing two manifests that claim one `doc_id`.
    pub fn new(manifests: Vec<RequirementManifest>) -> Result<Self, SetError> {
        let mut by_doc = BTreeMap::new();
        for manifest in manifests {
            if by_doc.contains_key(&manifest.doc_id) {
                return Err(SetError::DuplicateDoc {
                    doc_id: manifest.doc_id,
                });
            }
            by_doc.insert(manifest.doc_id.clone(), manifest);
        }
        Ok(Self { by_doc })
    }

    /// The `doc_id`s this set can answer about.
    pub fn doc_ids(&self) -> impl Iterator<Item = &str> {
        self.by_doc.keys().map(String::as_str)
    }

    /// How many manifests the set holds.
    pub fn len(&self) -> usize {
        self.by_doc.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_doc.is_empty()
    }

    /// One requirement, if the set holds its document and that document
    /// names it.
    ///
    /// ⭐ The single resolver. Both walks go through it, so "what does
    /// it mean to find a requirement over there" has one answer.
    pub fn resolve(&self, at: &RequirementRef) -> Resolution<'_> {
        match self.by_doc.get(&at.doc) {
            None => Resolution::NoSuchDoc,
            Some(manifest) => match manifest.requirements.iter().find(|e| e.id == at.id) {
                None => Resolution::NoSuchId,
                Some(entry) => Resolution::Found(entry),
            },
        }
    }

    /// Every cross-document claim in the set, judged.
    ///
    /// Covers both kinds in one walk because both are "a requirement
    /// names one elsewhere": a `delegated` disposition, and each child
    /// of a decomposition.
    pub fn report(&self) -> SetReport {
        let mut verdicts = Vec::new();
        for (doc_id, manifest) in &self.by_doc {
            for entry in &manifest.requirements {
                let from = RequirementRef {
                    doc: doc_id.clone(),
                    id: entry.id.clone(),
                };
                if let Disposition::Delegated { .. } = entry.disposition {
                    if let Some(target) = delegation_target(entry) {
                        verdicts.push(CrossDocVerdict {
                            from: from.clone(),
                            arrival: self.follow(&from, target),
                        });
                    }
                }
                for child in &entry.decomposes_into {
                    verdicts.push(CrossDocVerdict {
                        from: from.clone(),
                        arrival: self.follow(&from, child.clone()),
                    });
                }
            }
        }
        SetReport { verdicts }
    }

    /// Walk a claim to its end, refusing a document already stood in.
    ///
    /// The seen-set is keyed on the whole reference rather than the
    /// document, so a document legitimately carrying several links of
    /// one chain is not mistaken for a loop.
    fn follow(&self, from: &RequirementRef, first: RequirementRef) -> Arrival {
        let mut seen: BTreeSet<RequirementRef> = BTreeSet::new();
        seen.insert(from.clone());
        let mut hops: Vec<RequirementRef> = vec![from.clone()];
        let mut at = first;
        loop {
            if seen.contains(&at) {
                hops.push(at);
                return Arrival::Cycle { path: hops };
            }
            seen.insert(at.clone());
            hops.push(at.clone());
            match self.resolve(&at) {
                Resolution::NoSuchDoc => return Arrival::Unchecked { at },
                Resolution::NoSuchId => return Arrival::NotArrived { at },
                Resolution::Found(entry) => match &entry.disposition {
                    Disposition::Implemented {} => {
                        return Arrival::Arrived {
                            at,
                            hops,
                            terminus: Terminus::Implemented,
                        }
                    }
                    Disposition::OutOfScope { reason } => {
                        return Arrival::Arrived {
                            at,
                            hops,
                            terminus: Terminus::OutOfScope {
                                reason: reason.clone(),
                            },
                        }
                    }
                    Disposition::SystemLevel { realised_by } => {
                        return Arrival::Arrived {
                            at,
                            hops,
                            terminus: Terminus::SystemLevel {
                                realised_by: realised_by.clone(),
                            },
                        }
                    }
                    Disposition::Delegated { .. } => match delegation_target(entry) {
                        // The manifest loader refuses a blank naming, so
                        // this is unreachable through `load`. Treated as
                        // arrival rather than panicking: a set built by
                        // hand in a test is still a set.
                        None => {
                            return Arrival::Arrived {
                                at,
                                hops,
                                terminus: Terminus::Implemented,
                            }
                        }
                        Some(next) => at = next,
                    },
                },
            }
        }
    }
}

/// What the set found at a reference.
#[derive(Debug)]
pub enum Resolution<'a> {
    /// The set holds no manifest for that document.
    NoSuchDoc,
    /// It holds the document, which does not name that id.
    NoSuchId,
    Found(&'a RequirementEntry),
}

/// The reference a `delegated` entry names, or `None` for any other
/// disposition.
///
/// ⭐ The one place `to_doc`/`to_id` becomes a [`RequirementRef`], so the
/// wire spelling is converted once. A second conversion site is how the
/// two spellings would drift apart.
pub fn delegation_target(entry: &RequirementEntry) -> Option<RequirementRef> {
    match &entry.disposition {
        Disposition::Delegated { to_doc, to_id } => Some(RequirementRef {
            doc: to_doc.clone(),
            id: to_id.clone(),
        }),
        _ => None,
    }
}
