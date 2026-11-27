// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The documents of one design, as the requirement and acceptance commands
//! read them.
//!
//! A design is often more than one document: a statechart imports the event
//! schemas its interface admits and the algorithms its guards and actions call,
//! and a specification may be answered by several statecharts kept side by side.
//! The owner's question is whether the DESIGN carries each requirement, so
//! `requirements`, `acceptance-report` and `accept` take a set, credit a claim
//! wherever in the set it is made, and `acceptance-check` reads again the set
//! the record names. Which documents make up that set is decided here, once,
//! for all of them: a command that decided it for itself would accept one set
//! and report on another.
//!
//! # Which documents
//!
//! - every document the caller named, each once;
//! - with `with_imports`, every document the named ones reach, found by the
//!   walk `generate --with-imports` makes ([`crate::import_closure::walk`]):
//!   the documents an `<sce:import>` names, transitively, the element a
//!   collection names, and a statechart a statechart starts. One walk, so the
//!   set an acceptance answers for is the set a build of it generates;
//! - never a standard document a reference reaches. It is read from the library
//!   compiled into the generator, has no file an acceptance could pin, moves
//!   only with the generator, and is not where the owner's requirements are
//!   claimed. One the caller names is kept, since it was asked for.
//!
//! ⚠ Without `with_imports` nothing is walked, so a document named alone is
//! the design it always was: a claim made in a document it imports is not
//! credited to it. The set is asked for by name, and a command line that did not
//! ask keeps the meaning it had.

use std::collections::BTreeSet;
use std::path::Path;

use crate::forge::error::RelatedSite;
use crate::forge::stdlib;
use crate::import_closure::{Refusal, Role, WalkOptions};

/// The documents a caller names as one design, and whether what they import
/// is part of it. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignSet {
    named: Vec<String>,
    with_imports: bool,
}

/// One document of a design.
#[derive(Debug, Clone)]
pub struct Member {
    /// The document as the generator takes it: the caller's spelling for a
    /// document the caller named, and otherwise the path its reference
    /// resolved to, with `.` and `..` folded.
    pub document: String,
    /// The references that brought it into the set, the named document first;
    /// empty for a document the caller named. What a refusal of this document
    /// carries, so a document three imports down is not reported in a file the
    /// caller never named with nothing saying why it was read.
    pub reached_from: Vec<RelatedSite>,
}

impl DesignSet {
    /// The design `named` makes, with what the named documents reach when
    /// `with_imports`. `None` when `named` is empty: a design is read from
    /// at least one document.
    pub fn new(named: Vec<String>, with_imports: bool) -> Option<Self> {
        (!named.is_empty()).then_some(DesignSet {
            named,
            with_imports,
        })
    }

    /// The design of one document read alone, as every command read it before
    /// it took a set.
    pub fn one(document: impl Into<String>) -> Self {
        DesignSet {
            named: vec![document.into()],
            with_imports: false,
        }
    }

    /// The documents the caller named, in the order named.
    pub fn named(&self) -> &[String] {
        &self.named
    }

    /// Whether the documents the named ones reach are members too.
    pub fn with_imports(&self) -> bool {
        self.with_imports
    }

    /// Every document of the design, each once (one spelling per document,
    /// [`stdlib::identity`]): the named ones in the order named, then, with
    /// `with_imports`, the ones they reach in the order the walk reached them.
    ///
    /// Refused where the walk refuses: a reference that names no document it
    /// can read, located where it is written, with the chain that reached the
    /// document holding it. Without `with_imports` nothing is read here, and
    /// nothing is refused: each document is read, and refused, by its reader.
    pub fn members(&self) -> Result<Vec<Member>, Refusal> {
        let found: Vec<Member> = if self.with_imports {
            crate::import_closure::walk(&self.named, WalkOptions::default())?
                .into_iter()
                .filter(|unit| unit.role == Role::Named || !unit.is_standard())
                .map(|unit| Member {
                    document: unit.document,
                    reached_from: unit.reached_from,
                })
                .collect()
        } else {
            self.named
                .iter()
                .map(|document| Member {
                    document: document.clone(),
                    reached_from: Vec::new(),
                })
                .collect()
        };
        let mut seen: BTreeSet<String> = BTreeSet::new();
        Ok(found
            .into_iter()
            .filter(|member| seen.insert(stdlib::identity(Path::new(&member.document))))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_design_is_read_from_at_least_one_document() {
        assert_eq!(DesignSet::new(Vec::new(), true), None);
        assert_eq!(
            DesignSet::new(vec!["a.scxml".to_string()], false),
            Some(DesignSet::one("a.scxml"))
        );
    }

    /// Without `with_imports` nothing is read: the named documents are the
    /// set, each once, in the order named, whether or not they exist.
    #[test]
    fn without_imports_the_set_is_the_named_documents_each_once() {
        let set = DesignSet::new(
            vec![
                "b.scxml".to_string(),
                "a.scxml".to_string(),
                "b.scxml".to_string(),
            ],
            false,
        )
        .expect("a design");
        let members: Vec<String> = set
            .members()
            .expect("nothing is walked, so nothing is refused")
            .into_iter()
            .map(|member| member.document)
            .collect();
        assert_eq!(members, ["b.scxml", "a.scxml"]);
    }
}
