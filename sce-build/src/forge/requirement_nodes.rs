// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! ⚠ "Refused on every `sce:`-namespace element" was the measured claim,
//! and the qualifier was load-bearing. `schemas/sce-forge.xsd` wraps the
//! root's children in `<xs:any processContents="lax">`, and `lax` means
//! *validate against a global declaration if one exists, accept silently
//! otherwise*. So a `sce:req` on a W3C-namespace element inside a forge
//! document — the `<scxml>` root, or a `<data>` — was accepted and
//! dropped (measured 2026-09-15 with `xmllint --schema`). That silent drop
//! was closed in the parser on 2026-09-28: every `sce:` attribute on a W3C
//! element that no reader asked for is refused when the parse ends
//! (`crate::read_ledger`, `validation/sce-attribute-unread`).
//!
//! ## The root is a claim site of every kind
//!
//! A forge document is one artifact — a codec, a condition, an enum — and
//! its root is the one node every kind has, so the root is where the
//! document as a whole claims a requirement (`ParsedForge::requirements`,
//! docs/adr/0106, item G3). Every kind therefore answers with at least
//! the root's row, and a kind with finer nodes a specification speaks
//! about row by row (a lookup's entries) answers with those besides.
//!
//! Until 2026-11-16 most kinds answered "no annotation site" instead, a
//! third value kept apart from an empty table so that a document nobody
//! could annotate did not read as a reviewed one with nothing to report.
//! With the root a claim site, that state cannot arise: a document whose
//! root claims nothing prints its root row in the `(none)` block, which
//! is the finding about the document the table exists to surface.
//!
//! ## Exhaustive on purpose
//!
//! [`requirement_nodes`] matches every [`ForgeDocument`] variant with no
//! wildcard arm, so a kind added to the enum does not compile until
//! somebody says which of its nodes beside the root are reviewable. That
//! is the lock-in [`crate::forge::codegen_matrix::kind_class`] already
//! uses for the emit matrix, applied to review instead.
//!
//! ## ⚠ No second walk
//!
//! The statechart arm delegates to [`crate::requirements_report::walk_nodes`]
//! rather than traversing `SCXMLModel` again. The transition table's own
//! header states why that matters — the classification and the table are
//! two readings of one walk, so a requirement's verdict and its row are
//! answers about the same node. A second traversal here would be a third
//! reading free to disagree with both.

use crate::forge::model::{ForgeDocument, ParsedForge};

/// The `node_path` of a document's root row.
pub const ROOT_NODE_PATH: &str = "document";

/// One reviewable node: what it is, where it is, and what it claims.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ForgeReqNode {
    /// Hierarchical path identifying the node, stable across re-parses
    /// — the spelling [`crate::requirements_report::RequirementRecord`]
    /// uses, so a reader that already knows one form knows this one.
    pub node_path: String,
    /// Lowercase short tag for routing — `document`, `entry`, `state`, ...
    pub node_type: &'static str,
    /// Requirement ids the node claims, verbatim and in document order.
    ///
    /// Empty is meaningful and is emitted: it is the `(none)` row,
    /// behaviour the specification never asked for.
    pub requirements: Vec<String>,
    /// What the node DOES, in the kind's own terms — `3 -> DRIVE` for a
    /// lookup row, `codec 'frame'` for a codec's root. The column that
    /// makes the artefact reviewable by somebody holding the specification
    /// rather than the IR.
    pub detail: String,
}

/// The reviewable nodes of this document: its root first, then the nodes
/// its kind reads claims on besides; a statechart's are its own walk's.
///
/// Exhaustive over [`ForgeDocument`] — see the module header.
pub fn requirement_nodes(parsed: &ParsedForge) -> Vec<ForgeReqNode> {
    let doc = &parsed.document;
    let root = || ForgeReqNode {
        node_path: ROOT_NODE_PATH.to_string(),
        node_type: "document",
        requirements: parsed.requirements.iter().map(|id| id.0.clone()).collect(),
        detail: format!("{} '{}'", doc.kind().as_attr(), doc.name()),
    };
    match doc {
        // Delegated, not re-walked. See the module header. The
        // statechart's own walk reads its root among its nodes.
        ForgeDocument::Statechart(model) => crate::requirements_report::walk_nodes(model)
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

        // A mapping table is the shape a specification states row by
        // row, so each row carries a claim besides the document's.
        ForgeDocument::Lookup(model) => std::iter::once(root())
            .chain(model.entries.iter().map(|entry| ForgeReqNode {
                node_path: format!("entries[key={}]", entry.key),
                node_type: "entry",
                requirements: entry.requirements.iter().map(|id| id.0.clone()).collect(),
                detail: format!("{} -> {}", entry.key, entry.value),
            }))
            .collect(),

        // The root alone. A finer node of one of these kinds (a codec
        // field, a validator rule, an interpolation breakpoint) becomes a
        // claim site the way a lookup's entry did: admit `sce:req` on it
        // in `schemas/sce-forge-ext.xsd`, read it through the shared
        // `collect_sce_req`, and answer for it here. Listed rather than
        // collapsed behind `_`, so adding a kind to the enum stops the
        // build until somebody decides.
        ForgeDocument::Transform(_)
        | ForgeDocument::Condition(_)
        | ForgeDocument::Codec(_)
        | ForgeDocument::Validator(_)
        | ForgeDocument::Procedure(_)
        | ForgeDocument::Filter(_)
        | ForgeDocument::Interpolation(_)
        | ForgeDocument::Timer(_)
        | ForgeDocument::Observer(_)
        | ForgeDocument::Algorithm(_)
        | ForgeDocument::Link(_)
        | ForgeDocument::BufferPool(_)
        | ForgeDocument::Worker(_)
        | ForgeDocument::BoundedCollection(_)
        | ForgeDocument::Enum(_)
        | ForgeDocument::EventSchema(_)
        | ForgeDocument::Queue(_)
        | ForgeDocument::FeatureModel(_)
        | ForgeDocument::BroadcastLog(_)
        | ForgeDocument::TextTemplate(_)
        | ForgeDocument::Union(_)
        | ForgeDocument::TestTemplate(_)
        | ForgeDocument::TestPortType(_)
        | ForgeDocument::TestConfiguration(_)
        | ForgeDocument::TestCase(_)
        | ForgeDocument::TestSuite(_) => vec![root()],
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

        let parsed = crate::forge::ast_export::statechart_parsed_forge(model);
        let actual: Vec<(String, &'static str)> = requirement_nodes(&parsed)
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

    fn condition(root_attrs: &str) -> ParsedForge {
        let src = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="condition" name="probe_condition"{root_attrs}>
  <datamodel>
    <data id="speed" sce:type="uint16" sce:direction="in"/>
    <data id="result" sce:type="bool" sce:direction="out" expr="speed &gt; 10"/>
  </datamodel>
</scxml>"#
        );
        crate::forge::parser::parse_forge_with_imports(
            &src,
            crate::DocumentLabel {
                identifier: "probe_condition",
                diagnostic_label: "probe_condition.scxml",
            },
        )
        .expect("the probe document parses")
        .expect("a condition document is a forge document")
    }

    /// A kind with no finer claim site answers with its root: the
    /// document's own claims, in document order.
    #[test]
    fn the_root_of_any_kind_carries_the_documents_claims() {
        let parsed = condition(r#" sce:req="REQ-A REQ-B""#);
        let rows = requirement_nodes(&parsed);
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].node_path, ROOT_NODE_PATH);
        assert_eq!(rows[0].node_type, "document");
        assert_eq!(rows[0].requirements, vec!["REQ-A", "REQ-B"]);
        assert_eq!(rows[0].detail, "condition 'probe_condition'");
    }

    /// A root claiming nothing is still a row: the `(none)` row, a finding
    /// about the document, never a table missing.
    #[test]
    fn a_root_claiming_nothing_is_the_none_row() {
        let rows = requirement_nodes(&condition(""));
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert!(rows[0].requirements.is_empty());
    }
}
