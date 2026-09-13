// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a requirement's evidence actually depends on.
//!
//! Requirement-closure RFC §7a: the acceptance report puts one block per
//! requirement in front of a person, and that block has to contain
//! everything a violation could hide in. Choosing its contents by
//! `sce:req` does not do that.
//!
//! # ⭐ Why a closure and not "the nodes carrying the id"
//!
//! Measured over the real pair in `tests/fixtures/requirement_closure/`:
//! of 72 (requirement, dependency) pairs reachable from annotated
//! transitions, **39 lie on nodes that do not carry the id** — every
//! delay, every `<cancel>` of that delay, every target entry action,
//! every source exit action and every one of the transition's own
//! actions. Ten of the thirteen annotated requirements have at least
//! one.
//!
//! So a block chosen by id would stay byte-identical under RFC §7a.1's
//! own first example — retiming a delay from three seconds to five —
//! which is the completion test for this atomic. The fragment is the
//! DEPENDENCY CLOSURE of each cited node instead, over relations the IR
//! already carries.
//!
//! # The relations, and why each one is here
//!
//! ```text
//!   cited transition ──┬─ its own row: event, guard, target, actions
//!                      ├─ the <send> whose event IS this transition's
//!                      │  event — the timer that arms it. Without this
//!                      │  edge no delay is in the fragment at all.
//!                      ├─ the <cancel> naming that send — a requirement
//!                      │  about a timer is violated by cancelling it
//!                      │  as surely as by retiming it
//!                      ├─ the target state's entry actions
//!                      └─ the source state's exit actions
//! ```
//!
//! # One renderer, not two
//!
//! A fragment is a set of `node_path`s, and the report prints the
//! [`crate::transition_table`] rows those paths name. It never renders a
//! node itself. C's earlier round had to repair exactly that shape: the
//! table's action cell printed only the action's kind, so renaming a
//! timer's event left the page identical while the behaviour moved. A
//! second renderer is how the two drift apart again.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::model::{Action, SCXMLModel};
use crate::requirement_manifest::{classify, Outcome, RequirementManifest};
use crate::requirement_sidecar::RequirementSidecar;
use crate::requirements_report::{walk_nodes, ActionSite, NodeSubject};
use crate::transition_table::{transition_table, TransitionRow};

/// Why a node is in a requirement's fragment.
///
/// Carried rather than flattened away so a sweep can say which relation
/// it is exercising — "the arming send" is a finding a reader can act
/// on, where a bare node path is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relation {
    /// The node carries the requirement id itself.
    Cited,
    /// A `<send>` that raises the event a cited transition waits for.
    ArmingSend,
    /// A `<cancel>` naming that send.
    CancelOfArmingSend,
    /// An entry action of a cited transition's target state.
    TargetEntry,
    /// An exit action of a cited transition's source state.
    SourceExit,
    /// Executable content of the cited transition itself.
    TransitionAction,
}

/// One node a requirement's evidence depends on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependency {
    /// Identity, as the shared walk assigns it — the same string the
    /// transition table puts in its own rows, so a fragment is a
    /// selection of that table rather than a second rendering.
    pub node_path: String,
    pub relation: Relation,
}

/// Every node the requirement's behaviour depends on, cited or not.
///
/// Returns an empty set when the document does not annotate `id` at
/// all; a caller reporting coverage must treat that as "no evidence"
/// rather than as a fragment that happens to be small.
pub fn fragment(model: &SCXMLModel, id: &str) -> Vec<Dependency> {
    let nodes = walk_nodes(model);

    // Pass 1 — what the document itself attributes to this requirement,
    // and the context each cited transition drags in with it.
    let mut found: Vec<Dependency> = Vec::new();
    let mut cited_paths: HashSet<String> = HashSet::new();
    let mut target_states: HashSet<&str> = HashSet::new();
    let mut source_states: HashSet<&str> = HashSet::new();
    let mut awaited_events: HashSet<&str> = HashSet::new();
    // Keyed by (state, position), never by state alone: a state holds
    // several transitions, and only the cited one's executable content
    // is this requirement's evidence. Keying on the state would pull in
    // a sibling transition's actions, and the fragment would then move
    // when something unrelated to the requirement changed — a page that
    // cries wolf is read exactly as carefully as one that stays silent.
    let mut cited_transitions: HashSet<(&str, usize)> = HashSet::new();

    for node in &nodes {
        if !node.record.requirement_ids.contains(&id) {
            continue;
        }
        cited_paths.insert(node.record.node_path.clone());
        found.push(Dependency {
            node_path: node.record.node_path.clone(),
            relation: Relation::Cited,
        });

        match &node.subject {
            NodeSubject::Transition {
                state,
                transition,
                index,
            } => {
                cited_transitions.insert((state.id.as_str(), *index));
                source_states.insert(state.id.as_str());
                if !transition.target.is_empty() {
                    target_states.insert(transition.target.as_str());
                }
                if !transition.event.is_empty() {
                    awaited_events.insert(transition.event.as_str());
                }
            }
            NodeSubject::State(state) => {
                // A requirement on a state is about being in it, so both
                // sides of that boundary are its evidence.
                source_states.insert(state.id.as_str());
                target_states.insert(state.id.as_str());
            }
            _ => {}
        }
    }

    // The ids of the sends that raise an awaited event. Collected before
    // pass 2 so a `<cancel>` can be matched against them wherever it
    // sits, which is rarely the state the requirement is annotated on.
    let mut arming_send_ids: BTreeSet<&str> = BTreeSet::new();
    for node in &nodes {
        if let NodeSubject::Action { action, .. } = &node.subject {
            if is_arming_send(action, &awaited_events) {
                for id in [action.id.as_str(), action.auto_send_id.as_str()] {
                    if !id.is_empty() {
                        arming_send_ids.insert(id);
                    }
                }
            }
        }
    }

    // Pass 2 — the nodes that carry no id and hold the behaviour anyway.
    for node in &nodes {
        let path = &node.record.node_path;
        if cited_paths.contains(path) {
            continue;
        }
        let NodeSubject::Action {
            state,
            action,
            site,
        } = &node.subject
        else {
            continue;
        };

        let relation = if is_arming_send(action, &awaited_events) {
            Some(Relation::ArmingSend)
        } else if is_cancel_of(action, &arming_send_ids) {
            Some(Relation::CancelOfArmingSend)
        } else {
            match site {
                ActionSite::Entry if target_states.contains(state.id.as_str()) => {
                    Some(Relation::TargetEntry)
                }
                ActionSite::Exit if source_states.contains(state.id.as_str()) => {
                    Some(Relation::SourceExit)
                }
                ActionSite::Transition { index }
                    if cited_transitions.contains(&(state.id.as_str(), *index)) =>
                {
                    Some(Relation::TransitionAction)
                }
                _ => None,
            }
        };

        if let Some(relation) = relation {
            found.push(Dependency {
                node_path: path.clone(),
                relation,
            });
        }
    }

    found.sort();
    found.dedup();
    found
}

/// The whole report, as the page a person reads in one sitting.
///
/// RFC §7a's three blocks: A is one block per requirement, B is the
/// risk surface, C is what the source says that nothing here claims.
///
/// ⚠ When `sidecar` is given, the output carries sentences from someone
/// else's document. It says so on its own face, because the only way
/// the manifest/sidecar split fails is a helpful person checking the
/// rendered page into the repository.
pub fn render(
    model: &SCXMLModel,
    manifest: &RequirementManifest,
    sidecar: Option<&RequirementSidecar>,
) -> String {
    let classification = classify(model, manifest);
    let table = transition_table(model);
    let by_path: BTreeMap<&str, &TransitionRow> = table
        .iter()
        .map(|row| (row.node_path.as_str(), row))
        .collect();

    let mut out = String::new();
    out.push_str(&format!(
        "ACCEPTANCE REPORT  {}@{}  <->  {}\n",
        manifest.doc_id, manifest.rev, model.name
    ));
    if sidecar.is_some() {
        out.push_str(
            "⚠ carries verbatim text from the source document. A local artefact \
             for one sitting: do not commit it.\n",
        );
    }
    if let Some(note) = &classification.revision_note {
        out.push_str(&format!("⚠ {note}\n"));
    }

    out.push_str("\nA. REQUIREMENTS\n");
    for entry in &manifest.requirements {
        out.push_str(&format!("\n  {}", entry.id));
        if let Some(section) = &entry.section {
            out.push_str(&format!("   section {section}"));
        }
        out.push('\n');

        match sidecar.and_then(|side| side.sentence(&entry.id)) {
            Some(sentence) => out.push_str(&format!("    says: {sentence}\n")),
            None if sidecar.is_some() => out.push_str(
                "    says: (no sentence for this id in the sidecar — check the \
                 source before accepting)\n",
            ),
            None => out.push_str("    says: (no sidecar supplied)\n"),
        }

        let deps = fragment(model, &entry.id);
        if deps.is_empty() {
            out.push_str("    evidence: (this document annotates nothing for it)\n");
            continue;
        }
        out.push_str("    evidence:\n");
        for dep in &deps {
            let Some(row) = by_path.get(dep.node_path.as_str()) else {
                continue;
            };
            out.push_str(&format!(
                "      {:<18} {} | {} | {} | {} | {} | {}\n",
                format!("{:?}", dep.relation),
                row.from,
                row.event,
                row.guard,
                row.after,
                row.to,
                row.action
            ));
        }
    }

    out.push_str("\nB. NEEDING ATTENTION\n");
    let mut flagged = 0usize;
    for outcome in &classification.outcomes {
        if outcome.outcome == Outcome::Implemented {
            continue;
        }
        flagged += 1;
        out.push_str(&format!(
            "  {:<14} {}\n",
            outcome.outcome.as_str(),
            outcome.id
        ));
    }
    let unclaimed = table.iter().filter(|row| row.is_unclaimed()).count();
    if unclaimed > 0 {
        out.push_str(&format!(
            "  {:<14} {unclaimed} element(s) the source never asked for\n",
            "unclaimed"
        ));
    }
    if flagged == 0 && unclaimed == 0 {
        out.push_str("  (nothing)\n");
    }

    out.push_str("\nC. SOURCE COVERAGE\n");
    for (section, count) in &classification.section_counts {
        let marker = if *count == 0 { "   <- open this" } else { "" };
        out.push_str(&format!("  {section:<12} {count}{marker}\n"));
    }

    out
}

/// A `<send>` that raises one of the events a cited transition waits
/// for — the timer arming it, when the send carries a delay.
fn is_arming_send(action: &Action, awaited: &HashSet<&str>) -> bool {
    action.action_type == "send"
        && !action.event.is_empty()
        && awaited.contains(action.event.as_str())
}

/// A `<cancel>` naming one of those sends.
///
/// ⚠ `sendidexpr` is an expression, not a value: it cannot be resolved
/// at build time, so a cancel written that way is included whenever the
/// requirement has any arming send at all. Leaving it out would drop the
/// one node that can stop the timer.
fn is_cancel_of(action: &Action, arming_send_ids: &BTreeSet<&str>) -> bool {
    if action.action_type != "cancel" {
        return false;
    }
    if !action.sendidexpr.is_empty() {
        return !arming_send_ids.is_empty();
    }
    arming_send_ids.contains(action.sendid.as_str())
}
