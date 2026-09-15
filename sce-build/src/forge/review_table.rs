// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The review artefact for a non-statechart kind — NL→IR closure ledger
//! row G3.
//!
//! ```text
//!   source      | node             | type  | detail
//!   ------------+------------------+-------+---------------
//!   REQ-DRIVE   | entries[key=3]   | entry | 3 -> DRIVE
//!   (none)      | entries[key=4]   | entry | 4 -> SPORT
//! ```
//!
//! The same object as [`crate::transition_table`] and deliberately the
//! same two readings (Requirement-closure RFC §6.2), because a reviewer
//! should not have to learn a second artefact per kind:
//!
//! - **Sort by `source`.** Every `(none)` row collects into one block:
//!   behaviour the specification never asked for. A mapping row nobody
//!   required is exactly as interesting as an unannotated transition.
//! - **Compare `source` against the manifest.** A requirement with no
//!   row at all is `missing`.
//!
//! What is NOT shared is the column set, and that is the point of
//! having a second module rather than widening the first. The
//! statechart table's columns are `from`/`event`/`guard`/`after`/`to` —
//! a vocabulary a lookup has none of. Forcing one row type onto both
//! would give every non-statechart kind five columns of `-`, which
//! reads as missing data rather than as an inapplicable question.
//! `detail` is the kind's own terms instead.
//!
//! ⚠ The rows come from [`crate::forge::requirement_nodes`], which is
//! also what answers whether the kind has an annotation site at all.
//! This module must not turn [`ReviewScope::NoAnnotationSite`] into an
//! empty table: a document of a kind SCE cannot annotate would then
//! render exactly like a fully reviewed one with nothing to report.

use std::io::{self, Write};

use serde::Serialize;

use crate::forge::model::ForgeDocument;
use crate::forge::requirement_nodes::{requirement_nodes, ReviewScope};

/// The literal printed in `source` for a node claiming no requirement.
///
/// Re-exported from the statechart table rather than spelled again: the
/// two artefacts sort into the same `(none)` block only if the word is
/// the same word, and a reviewer grepping one should find the other.
pub use crate::transition_table::NO_SOURCE;

/// One row of the review table.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReviewRow {
    /// Requirement ids claimed by the node, space-joined, or
    /// [`NO_SOURCE`].
    pub source: String,
    /// Hierarchical path identifying the node.
    pub node: String,
    /// Lowercase short tag — `entry`, `state`, ...
    pub node_type: &'static str,
    /// What the node does, in the kind's own terms.
    pub detail: String,
}

impl ReviewRow {
    /// Whether this row claims nothing — the `(none)` block.
    pub fn is_unclaimed(&self) -> bool {
        self.source == NO_SOURCE
    }
}

/// Why a document has no review table, as opposed to an empty one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoTable {
    /// `sce:kind` as authored.
    pub kind: &'static str,
}

impl std::fmt::Display for NoTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SCE reads sce:req on no node of the '{}' kind, so it cannot \
             say what this document claims — an empty table here would be \
             a statement about SCE, not about the document",
            self.kind
        )
    }
}

/// The review table for this document, or why there is none.
pub fn review_table(doc: &ForgeDocument) -> Result<Vec<ReviewRow>, NoTable> {
    match requirement_nodes(doc) {
        ReviewScope::NoAnnotationSite { kind } => Err(NoTable { kind }),
        ReviewScope::Annotatable(nodes) => Ok(nodes
            .into_iter()
            .map(|node| ReviewRow {
                source: if node.requirements.is_empty() {
                    NO_SOURCE.to_string()
                } else {
                    node.requirements.join(" ")
                },
                node: node.node_path,
                node_type: node.node_type,
                detail: node.detail,
            })
            .collect()),
    }
}

/// Requirement ids the manifest declares that no row claims — the
/// second reading, `missing`.
///
/// Takes the rows rather than the document so the caller cannot get a
/// different table than the one it printed.
pub fn missing_requirements<'a>(
    rows: &[ReviewRow],
    declared: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    let claimed: std::collections::BTreeSet<&str> = rows
        .iter()
        .filter(|row| !row.is_unclaimed())
        .flat_map(|row| row.source.split(' '))
        .collect();
    declared
        .filter(|id| !claimed.contains(id))
        .map(str::to_string)
        .collect()
}

/// Emit the table as NDJSON, one record per row.
///
/// NDJSON for the reason the statechart table gives: it must stay
/// diffable, and a fixed-width grid re-flows every row when one node is
/// renamed.
pub fn emit_review_table_ndjson<W: Write + ?Sized>(
    rows: &[ReviewRow],
    writer: &mut W,
) -> io::Result<()> {
    for row in rows {
        let line = serde_json::to_string(row)
            .expect("ReviewRow serialises; every field is an owned String or &'static str");
        writeln!(writer, "{line}")?;
    }
    Ok(())
}
