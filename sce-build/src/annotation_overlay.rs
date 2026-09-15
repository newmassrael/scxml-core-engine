// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a diagram must be told in order to draw the annotation family —
//! NL→IR closure ledger row G2.
//!
//! The visualizer renders no annotation today, so the picture a reviewer
//! reads cannot say which element a requirement claims and which is
//! unclaimed. It cannot be fixed inside the browser, and the reason is
//! recorded rather than assumed (Requirement-closure RFC, Atomic C, Q1):
//!
//! > **E cannot come first.** The visualizer builds its graph in the
//! > browser from its own `DOMParser` walk and lays it out with a
//! > constraint solver; `sce-build-wasm` exports only codegen. Its output
//! > is not bytes a test can compare, and it is a second walk. So E later
//! > draws the SAME closure C derives, not the other way round.
//!
//! ## ⛔ The one thing this module exists to prevent
//!
//! A requirement's evidence is NOT "the nodes carrying its id". It is the
//! dependency closure of those nodes — the arming `<send>`, the `<cancel>`
//! naming it, the target's entry actions, the source's exit actions.
//! Measured over the real pair, **39 of 72 (requirement, dependency)
//! pairs lie on nodes that do not carry the id** ([`crate::acceptance_report`]).
//!
//! A browser that picked nodes by `sce:req` would therefore draw a
//! *different* answer from the one the acceptance report derives, and
//! nobody would find out: a rendered diagram is not bytes a test can
//! diff. So the closure is computed HERE, by calling
//! [`crate::acceptance_report::fragment`] — the report's own function,
//! not a copy of it — and shipped to the browser as data.
//!
//! ⭐ That is why this module has no walk and no traversal of its own. It
//! composes two existing answers: [`crate::requirements_report::walk_nodes`]
//! for what nodes exist and what each claims, and `fragment` for what each
//! requirement depends on. If either moves, this moves with it, because
//! there is nothing here to move independently.
//!
//! ## Why the coordinates travel beside the path
//!
//! `node_path` is SCE's identity for a node (`states.armed.transitions[0]`).
//! The browser's graph is keyed by state id, so something has to bridge
//! the two — and if the browser bridges it by PARSING the path, the path
//! grammar becomes a second implementation living in JavaScript, free to
//! fall behind the one that writes it.
//!
//! So the bridge is published from the side that owns the identity: every
//! node carries the coordinates the browser already has ([`OverlayNode::state_id`],
//! [`OverlayNode::transition_index`]) next to the path SCE knows it by.
//! The browser matches on those and never parses a path.

use serde::Serialize;

use crate::acceptance_report::{fragment, Relation};
use crate::model::SCXMLModel;
use crate::requirements_report::{walk_nodes, ActionSite, NodeSubject};

/// Wire version of the overlay envelope.
///
/// The browser and the CLI read the same producer, so this exists to tell
/// a CACHED page that it is looking at an overlay shape it does not know —
/// a deployed visualizer outlives the build that produced its data.
pub const OVERLAY_WIRE_VERSION: u32 = 1;

/// Why a node is in a requirement's fragment, as the wire spells it.
///
/// A projection of [`Relation`], not a second enumeration: the match
/// below is exhaustive, so a relation added to the report cannot reach
/// the browser as a silent omission — it stops the build here instead.
fn relation_slug(relation: Relation) -> &'static str {
    match relation {
        Relation::Cited => "cited",
        Relation::ArmingSend => "arming-send",
        Relation::CancelOfArmingSend => "cancel-of-arming-send",
        Relation::TargetEntry => "target-entry",
        Relation::SourceExit => "source-exit",
        Relation::TransitionAction => "transition-action",
    }
}

/// One node of the diagram, and what it claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OverlayNode {
    /// SCE's identity for the node.
    pub node_path: String,
    /// `state` / `transition` / `action` / `invoke` — the short tag the
    /// requirements report routes on.
    pub node_type: &'static str,
    /// The state this node is, or sits in. The browser's graph is keyed
    /// by this, so it is what a lookup matches on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_id: Option<String>,
    /// Position of the transition within its state, where the node is a
    /// transition or sits in one. With `state_id` this is the browser's
    /// whole key for an edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_index: Option<usize>,
    /// Requirement ids the node claims, verbatim and in document order.
    ///
    /// ⚠ Empty is the POINT of this field, not a missing value: an
    /// element no requirement claims is what the reviewer is looking for,
    /// and the row is emitted so the diagram can mark it rather than
    /// leave it indistinguishable from an element nobody drew.
    pub requirements: Vec<String>,
}

impl OverlayNode {
    /// Whether no requirement claims this node.
    pub fn is_unclaimed(&self) -> bool {
        self.requirements.is_empty()
    }
}

/// One node a requirement's evidence depends on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OverlayDependency {
    pub node_path: String,
    /// Why it is in the fragment — see [`relation_slug`].
    pub relation: &'static str,
}

/// One requirement, and the closure its evidence depends on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OverlayFragment {
    /// The id, verbatim. SCE never normalises one.
    pub requirement: String,
    /// The dependency closure, exactly as [`crate::acceptance_report::fragment`]
    /// derives it.
    ///
    /// ⚠ Contains nodes that do NOT carry the id — that is the whole
    /// reason this ships instead of being recomputed in the browser.
    pub dependencies: Vec<OverlayDependency>,
}

/// Everything a diagram needs to draw the annotation family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnnotationOverlay {
    /// Always [`OVERLAY_WIRE_VERSION`].
    pub v: u32,
    /// Every node the shared walk yields, claimed or not.
    pub nodes: Vec<OverlayNode>,
    /// One per requirement the document claims, in sorted order.
    pub fragments: Vec<OverlayFragment>,
}

impl AnnotationOverlay {
    /// Requirement ids the document claims anywhere.
    pub fn claimed_requirements(&self) -> Vec<&str> {
        self.fragments
            .iter()
            .map(|f| f.requirement.as_str())
            .collect()
    }

    /// Nodes no requirement claims.
    pub fn unclaimed(&self) -> impl Iterator<Item = &OverlayNode> {
        self.nodes.iter().filter(|n| n.is_unclaimed())
    }
}

/// Where a node sits, in the coordinates the browser's graph already has.
///
/// Derived from the walk's own [`NodeSubject`], so this is a projection of
/// the shared walk rather than a second opinion about where a node is.
fn coordinates(subject: &NodeSubject<'_>) -> (Option<String>, Option<usize>) {
    match subject {
        NodeSubject::State(state) => (Some(state.id.clone()), None),
        NodeSubject::Transition { state, index, .. } => (Some(state.id.clone()), Some(*index)),
        NodeSubject::Invoke { state, index, .. } => (Some(state.id.clone()), Some(*index)),
        NodeSubject::Action { state, site, .. } => (
            Some(state.id.clone()),
            match site {
                // Only a transition's own content carries an edge index;
                // entry and exit content belongs to the state itself.
                ActionSite::Transition { index } => Some(*index),
                ActionSite::Entry | ActionSite::Exit => None,
                // <initial> and <history> default transitions carry
                // executable content with no transition index of their
                // own; they belong to the state.
                ActionSite::Initial | ActionSite::HistoryDefault { .. } => None,
            },
        ),
        // A top-level <script> sits in no state.
        NodeSubject::GlobalScript { .. } => (None, None),
    }
}

/// The overlay for this document.
///
/// Composes the shared walk with the acceptance report's closure. There
/// is deliberately no third source of truth here — see the module header.
pub fn overlay(model: &SCXMLModel) -> AnnotationOverlay {
    let walked = walk_nodes(model);

    let mut nodes: Vec<OverlayNode> = Vec::with_capacity(walked.len());
    let mut claimed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for node in &walked {
        let (state_id, transition_index) = coordinates(&node.subject);
        for id in &node.record.requirement_ids {
            claimed.insert((*id).to_string());
        }
        nodes.push(OverlayNode {
            node_path: node.record.node_path.clone(),
            node_type: node.record.node_type,
            state_id,
            transition_index,
            requirements: node
                .record
                .requirement_ids
                .iter()
                .map(|id| (*id).to_string())
                .collect(),
        });
    }

    // The closure, per requirement, from the report's own function.
    let fragments = claimed
        .into_iter()
        .map(|requirement| {
            let dependencies = fragment(model, &requirement)
                .into_iter()
                .map(|dep| OverlayDependency {
                    node_path: dep.node_path,
                    relation: relation_slug(dep.relation),
                })
                .collect();
            OverlayFragment {
                requirement,
                dependencies,
            }
        })
        .collect();

    AnnotationOverlay {
        v: OVERLAY_WIRE_VERSION,
        nodes,
        fragments,
    }
}
