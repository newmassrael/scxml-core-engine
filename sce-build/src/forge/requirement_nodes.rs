// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which nodes of a forge document can claim a requirement — NL→IR
//! closure ledger row G3.
//!
//! [`crate::transition_table`] is the review artefact of the statechart
//! family: one row per node the shared walk yields, with the
//! requirement ids in the first column. Every other kind family had no
//! such artefact, and could not have had one: `sce:req` was refused on
//! every `sce:`-namespace element, and read by nobody where it was
//! admitted, so the column would have been empty on every row by
//! construction.
//!
//! ⚠ "Refused on every `sce:`-namespace element" is the measured claim,
//! and the qualifier is load-bearing. `schemas/sce-forge.xsd` wraps the
//! root's children in `<xs:any processContents="lax">`, and `lax` means
//! *validate against a global declaration if one exists, accept silently
//! otherwise*. So a `sce:req` on a W3C-namespace element inside a forge
//! document — the `<scxml>` root, or a `<data>` — is ACCEPTED and
//! dropped, and was before this module existed. Measured 2026-09-15 with
//! `xmllint --schema` against the grammar as it then stood: `<sce:entry>`
//! and `<sce:field>` rc 3, `<scxml>` and `<data>` rc 0. Declaring
//! `sce:req` globally does not close that — a global attribute
//! declaration constrains the VALUE, not where the attribute may appear.
//! That silent drop is row S2's class, not this row's, and it is filed
//! rather than fixed here.
//!
//! This module is the half that was missing between the grammar and the
//! table: ONE definition of "the requirement-bearing nodes of this
//! document", answered per kind.
//!
//! ## Exhaustive on purpose
//!
//! [`requirement_nodes`] matches every [`ForgeDocument`] variant with no
//! wildcard arm, so a kind added to the enum does not compile until
//! somebody says what its reviewable nodes are. That is the lock-in
//! [`crate::forge::codegen_matrix::kind_class`] already uses for the
//! emit matrix, applied to review instead: the failure it prevents is a
//! new kind quietly inheriting "no rows" and nobody noticing that its
//! specification coverage is unmeasurable.
//!
//! ## ⚠ An empty table is two different facts, and they must not collapse
//!
//! A kind can print no rows because the document claims nothing, or
//! because SCE gives that kind nowhere to put a claim. The first is a
//! finding about the document — the `(none)` block the statechart table
//! exists to surface. The second is a statement about SCE, and reading
//! it as the first would report a clean review of a document nobody can
//! annotate.
//!
//! So the answer is [`ReviewScope`], not a bare `Vec`. This is the same
//! three-valued discipline [`crate::requirement_set::Arrival`] reached
//! for a delegation whose destination was not in the set: a question the
//! tool cannot answer must be reported, never passed.
//!
//! ## ⚠ No second walk
//!
//! The statechart arm delegates to [`crate::requirements_report::walk_nodes`]
//! rather than traversing `SCXMLModel` again. The transition table's own
//! header states why that matters — the classification and the table are
//! two readings of one walk, so a requirement's verdict and its row are
//! answers about the same node. A second traversal here would be a third
//! reading free to disagree with both.

use crate::forge::model::ForgeDocument;

/// One reviewable node: what it is, where it is, and what it claims.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ForgeReqNode {
    /// Hierarchical path identifying the node, stable across re-parses
    /// — the spelling [`crate::requirements_report::RequirementRecord`]
    /// uses, so a reader that already knows one form knows this one.
    pub node_path: String,
    /// Lowercase short tag for routing — `entry`, `state`, ...
    pub node_type: &'static str,
    /// Requirement ids the node claims, verbatim and in document order.
    ///
    /// Empty is meaningful and is emitted: it is the `(none)` row,
    /// behaviour the specification never asked for.
    pub requirements: Vec<String>,
    /// What the node DOES, in the kind's own terms — `3 -> DRIVE` for a
    /// lookup row. The column that makes the artefact reviewable by
    /// somebody holding the specification rather than the IR.
    pub detail: String,
}

/// Whether this kind has anywhere to put a claim, and the rows if so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewScope {
    /// SCE READS `sce:req` on this kind's nodes. The rows are every such
    /// node, annotated or not — an unannotated one is the row the
    /// artefact most needs to show.
    Annotatable(Vec<ForgeReqNode>),
    /// SCE reads `sce:req` nowhere in this kind yet, so it has no review
    /// artefact rather than an empty one.
    ///
    /// ⚠ READS, not "the grammar refuses". The two are not the same and
    /// the difference is measured: the grammar accepts a `sce:req` on a
    /// W3C-namespace element of any forge document (see the module
    /// header) and nothing reads it there. Saying "refuses" would tell
    /// an author their claim was rejected when it was in fact swallowed.
    ///
    /// ⚠ Carries the kind's own name so a reader is told which kind
    /// they asked about; a bare marker would make one unanswerable
    /// question look like every other.
    NoAnnotationSite {
        /// `sce:kind` as authored, e.g. `codec`.
        kind: &'static str,
    },
}

impl ReviewScope {
    /// The rows, or none when the kind has no annotation site.
    ///
    /// ⚠ A caller that only wants rows still has to decide what an
    /// empty answer means; this exists for callers that have already
    /// handled [`ReviewScope::NoAnnotationSite`] explicitly.
    pub fn rows(&self) -> &[ForgeReqNode] {
        match self {
            ReviewScope::Annotatable(rows) => rows,
            ReviewScope::NoAnnotationSite { .. } => &[],
        }
    }
}

/// The reviewable nodes of this document.
///
/// Exhaustive over [`ForgeDocument`] — see the module header.
pub fn requirement_nodes(doc: &ForgeDocument) -> ReviewScope {
    match doc {
        // Delegated, not re-walked. See the module header.
        ForgeDocument::Statechart(model) => ReviewScope::Annotatable(
            crate::requirements_report::walk_nodes(model)
                .into_iter()
                .map(|node| ForgeReqNode {
                    node_path: node.record.node_path.clone(),
                    node_type: node.record.node_type,
                    requirements: node
                        .record
                        .requirement_ids
                        .iter()
                        .map(|id| (*id).to_string())
                        .collect(),
                    // The statechart family's richer reading — event,
                    // guard, target, delay — is the transition table's
                    // business and stays there rather than being
                    // flattened into one string here.
                    detail: String::new(),
                })
                .collect(),
        ),

        // A mapping table is the shape a specification states row by
        // row, so the row carries the claim.
        ForgeDocument::Lookup(model) => ReviewScope::Annotatable(
            model
                .entries
                .iter()
                .map(|entry| ForgeReqNode {
                    node_path: format!("entries[key={}]", entry.key),
                    node_type: "entry",
                    requirements: entry.requirements.iter().map(|id| id.0.clone()).collect(),
                    detail: format!("{} -> {}", entry.key, entry.value),
                })
                .collect(),
        ),

        // ⚠ No annotation site in the grammar YET — not "no rows".
        //
        // Each of these kinds has nodes a specification plainly speaks
        // about (a codec field, a validator rule, an interpolation
        // breakpoint), and giving each one its `sce:req` is the same
        // three edits the lookup row took: admit the attribute in
        // `schemas/sce-forge-ext.xsd`, read it through the shared
        // `collect_sce_req`, and answer here. Listed one per line
        // rather than collapsed behind `_`, so adding a kind to the
        // enum stops the build until somebody decides.
        ForgeDocument::Transform(_) => ReviewScope::NoAnnotationSite { kind: "transform" },
        ForgeDocument::Condition(_) => ReviewScope::NoAnnotationSite { kind: "condition" },
        ForgeDocument::Codec(_) => ReviewScope::NoAnnotationSite { kind: "codec" },
        ForgeDocument::Validator(_) => ReviewScope::NoAnnotationSite { kind: "validator" },
        ForgeDocument::Procedure(_) => ReviewScope::NoAnnotationSite { kind: "procedure" },
        ForgeDocument::Filter(_) => ReviewScope::NoAnnotationSite { kind: "filter" },
        ForgeDocument::Interpolation(_) => ReviewScope::NoAnnotationSite {
            kind: "interpolation",
        },
        ForgeDocument::Timer(_) => ReviewScope::NoAnnotationSite { kind: "timer" },
        ForgeDocument::Observer(_) => ReviewScope::NoAnnotationSite { kind: "observer" },
        ForgeDocument::Algorithm(_) => ReviewScope::NoAnnotationSite { kind: "algorithm" },
        ForgeDocument::Link(_) => ReviewScope::NoAnnotationSite { kind: "link" },
        ForgeDocument::BufferPool(_) => ReviewScope::NoAnnotationSite {
            kind: "buffer-pool",
        },
        ForgeDocument::Worker(_) => ReviewScope::NoAnnotationSite { kind: "worker" },
        ForgeDocument::BoundedCollection(_) => ReviewScope::NoAnnotationSite {
            kind: "bounded-collection",
        },
        ForgeDocument::Enum(_) => ReviewScope::NoAnnotationSite { kind: "enum" },
        ForgeDocument::EventSchema(_) => ReviewScope::NoAnnotationSite {
            kind: "event-schema",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The statechart arm is the SAME walk, not a second one.
    ///
    /// ⚠ This is the axis no CLI run can reach — `review-table` refuses
    /// a statechart, so the delegation is only observable from inside
    /// the crate. Asserted as an equality against `walk_nodes` rather
    /// than as a count, because two walks that happen to visit the same
    /// number of nodes would pass a count and still disagree about
    /// WHICH nodes: `node_path` is what says they are the same nodes in
    /// the same order.
    #[test]
    fn the_statechart_arm_yields_exactly_what_the_shared_walk_yields() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" name="delegation_probe">
  <state id="idle" sce:req="REQ-IDLE">
    <onentry><log label="hello"/></onentry>
    <transition event="go" target="busy"/>
  </state>
  <state id="busy">
    <transition event="stop" target="idle" sce:req="REQ-STOP"/>
  </state>
</scxml>"#;
        let mut parser = crate::parser::SCXMLParser::new();
        let model = parser
            .parse_string(src, "delegation_probe")
            .expect("the probe document parses");

        let expected: Vec<(String, &'static str)> = crate::requirements_report::walk_nodes(&model)
            .into_iter()
            .map(|n| (n.record.node_path.clone(), n.record.node_type))
            .collect();

        let doc = ForgeDocument::Statechart(Box::new(model));
        let rows = match requirement_nodes(&doc) {
            ReviewScope::Annotatable(rows) => rows,
            ReviewScope::NoAnnotationSite { kind } => {
                panic!("statechart must be annotatable, got NoAnnotationSite({kind})")
            }
        };
        let actual: Vec<(String, &'static str)> = rows
            .iter()
            .map(|r| (r.node_path.clone(), r.node_type))
            .collect();

        // Control on the assertion itself: an empty walk would make the
        // equality vacuously true, and the document above has nodes.
        assert!(
            expected.len() >= 4,
            "the probe must yield several nodes or the equality below proves nothing, got {expected:?}"
        );
        assert_eq!(
            actual, expected,
            "the statechart arm re-walked the model instead of delegating to walk_nodes"
        );
    }

    /// A kind with no annotation site is not an empty table.
    #[test]
    fn a_kind_with_no_annotation_site_says_so_rather_than_returning_no_rows() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="condition" name="probe_condition">
  <datamodel>
    <data id="speed" sce:type="uint16" sce:direction="in"/>
    <data id="result" sce:type="bool" sce:direction="out" expr="speed &gt; 10"/>
  </datamodel>
</scxml>"#;
        let parsed = crate::forge::parser::parse_forge_with_imports(
            src,
            crate::DocumentLabel {
                identifier: "probe_condition",
                diagnostic_label: "probe_condition.scxml",
            },
        )
        .expect("the probe document parses")
        .expect("a condition document is a forge document");

        match requirement_nodes(&parsed.document) {
            ReviewScope::NoAnnotationSite { kind } => assert_eq!(kind, "condition"),
            ReviewScope::Annotatable(rows) => panic!(
                "condition has no sce:req site in the grammar, so it must not report {} reviewable row(s)",
                rows.len()
            ),
        }
    }
}
