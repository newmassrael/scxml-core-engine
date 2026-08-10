// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Statechart graph reachability.
//
// Computes the design-time reach set of an SCXML model after parse +
// `analyzer::analyze` finalize the state graph, and rejects documents
// that declare states the BFS cannot reach. Two diagnostic surfaces:
//
//   * `scxml/unreachable-state` — orphan state with zero transitions.
//   * `scxml/dead-transition`   — orphan state with one or more
//                                  transitions; emits one (state, target)
//                                  pair so the author lands on the
//                                  concrete element to repair.
//
// Algorithm (§scxml-3 entry semantics):
//   1. Seed the BFS queue with the document `initial` target(s) — or
//      the first top-level state in document order when the attribute
//      is empty (§scxml-3.2 default).
//   2. For each dequeued id:
//        - If the id names a history pseudostate (key in
//          `model.history_states`), redirect to its `default_target`
//          per §scxml-3.10. History pseudostates themselves are
//          never in the reach set — they are pure redirection nodes.
//        - Otherwise mark the state reached and follow:
//            * compound state with non-empty `initial` → enqueue each
//              whitespace-separated target (§scxml-3.3 initial-cascade);
//            * `<parallel>` → enqueue every direct child whose id is
//              not a history pseudostate (§scxml-3.4 all-regions);
//            * `<final>` → no further entry;
//            * every `<transition>`'s `target` (split on whitespace
//              for the multi-target parallel-entry case §scxml-3.13).
//   3. After the closure stabilises, walk every state in
//      `model.states` in document order and short-circuit on the
//      first orphan. The per-transition variant outranks the bare
//      state form when the orphan carries at least one transition
//      with a non-empty target.
//
// This pass runs after `analyzer::analyze` (the analyzer pipeline is
// infallible and only enriches derived fields) and after
// `guard_static_generatable`, so the more fundamental diagnostics —
// unresolved references, a missing/unknown document `initial`, a
// rejected top-level `<script>` — fire ahead of the orphan walk. An
// orphan is a consequence of a broken topology; reporting it in place
// of the reference that broke it points the author at the wrong
// element. See `sce-build/src/lib.rs::compile_model`.
//
// ⚠ Unlike `scxml_references`, this pass is reached only through
// `compile_model` — the `sce-codegen` CLI re-implements parse →
// analyze → generate and does not call it. That asymmetry is
// deliberate for now and load-bearing: 34 W3C IRP documents in
// `resources/` declare `fail` / `pass` states that the design-time BFS
// cannot reach (they are entered through error events and invoke
// completions the walk does not model), so routing the CLI through
// this pass would reject the conformance corpus. Any change here must
// re-measure that set first.

use std::collections::{HashSet, VecDeque};

use crate::forge::error::{ForgeError, Located};
use crate::model::SCXMLModel;
use crate::scxml_semantic::ScxmlSemanticError;

/// Reject the document on the first reachability violation. Mirrors
/// the short-circuit convention used by sibling validators
/// (`validate_axis3_accept_side_state_naming`,
/// `validate_on_sample_*`) — emitting every orphan in a single pass
/// would require collecting multiple `Located<ForgeError>` records,
/// which the wire layer (`Result<_, Located<ForgeError>>`) does not
/// model today.
pub fn validate(model: &SCXMLModel, source: &str) -> Result<(), Located<ForgeError>> {
    // Empty graph short-circuit: `ScxmlSemanticError::NoStates` fires
    // earlier in the pipeline; reachability has nothing to walk.
    if model.states.is_empty() {
        return Ok(());
    }

    let reach = compute_reach_set(model);

    // Walk in document order so the first-fired diagnostic is
    // deterministic across re-runs (BTreeMap iteration is keyed by
    // state id; we re-sort by `document_order` to match author
    // intuition).
    let mut ordered: Vec<&crate::model::State> = model.states.values().collect();
    ordered.sort_by_key(|s| s.document_order);

    for state in ordered {
        if reach.contains(&state.id) {
            continue;
        }
        // Per-transition form takes priority — surface the first
        // transition with a non-empty target so the author repair
        // lands on the orphan edge.
        for trans in &state.transitions {
            if let Some(target) = first_nonempty_target(&trans.target) {
                return Err(Located::new(
                    ScxmlSemanticError::DeadTransition {
                        state: state.id.clone(),
                        target,
                    }
                    .into(),
                    source,
                    None,
                    None,
                ));
            }
        }
        return Err(Located::new(
            ScxmlSemanticError::UnreachableState {
                state_id: state.id.clone(),
            }
            .into(),
            source,
            None,
            None,
        ));
    }

    Ok(())
}

/// First whitespace-separated non-empty target on a transition's
/// `target` attribute. Returns `None` for targetless transitions
/// (internal / executable-only) and for attributes containing only
/// whitespace, which the parser already accepts as the targetless
/// shape.
fn first_nonempty_target(raw: &str) -> Option<String> {
    raw.split_whitespace().next().map(String::from)
}

/// BFS over the design-time entry-edge set. Returns the set of state
/// ids in `model.states` that are reachable from the seed
/// configuration; history pseudostates themselves are never in the
/// set (they are redirection nodes, not states the runtime ever
/// enters).
fn compute_reach_set(model: &SCXMLModel) -> HashSet<String> {
    let mut reach: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    // Seed: explicit document `initial` attribute(s), or the
    // document-order first top-level state if absent (§scxml-3.2
    // default).
    if model.initial.is_empty() {
        if let Some((id, _)) = model
            .states
            .iter()
            .filter(|(_, s)| s.parent.is_none())
            .min_by_key(|(_, s)| s.document_order)
        {
            queue.push_back(id.clone());
        }
    } else {
        for target in model.initial.split_whitespace() {
            queue.push_back(target.to_string());
        }
    }

    while let Some(id) = queue.pop_front() {
        // History pseudostate: redirect to default_target per W3C
        // SCXML §3.10. The history id itself is not added to the
        // reach set — it is a redirection node, not a runtime state.
        if let Some(hi) = model.history_states.get(&id) {
            if !hi.default_target.is_empty() {
                for target in hi.default_target.split_whitespace() {
                    queue.push_back(target.to_string());
                }
            }
            continue;
        }

        // Unknown id. `scxml_references::validate` — reached through
        // `analyzer::can_generate_static`, which runs ahead of this
        // pass — rejects every unresolved `<transition target>`,
        // `<state initial>` and `<history>` default, so an id that is
        // neither a state nor a history pseudostate cannot arrive from
        // the document text. What can still arrive is a synthesised id
        // (the analyzer's derived entry chains), so the arm skips
        // silently rather than asserting.
        let state = match model.states.get(&id) {
            Some(s) => s,
            None => continue,
        };

        if !reach.insert(id.clone()) {
            continue;
        }
        // §scxml-3.6 — entering a state implicitly enters every
        // compound ancestor up to the document root. Walk the parent
        // chain so the validator sees those ancestors as reached
        // even when the BFS seed (`model.initial`) is the deep-
        // resolved leaf and never goes through the compound parent
        // directly. Without this, a top-level
        // `<state id="P" initial="C"><state id="C"/></state>` shape
        // surfaces `P` as orphan after the parser's
        // `resolve_deep_initial` collapses `model.initial` to `C`.
        let mut anc = state.parent.clone();
        while let Some(parent_id) = anc {
            if !reach.insert(parent_id.clone()) {
                break;
            }
            anc = model.states.get(&parent_id).and_then(|s| s.parent.clone());
        }

        // Entry cascade.
        if state.is_final {
            // Final states terminate the cascade.
        } else if state.is_parallel {
            // §scxml-3.4 — entering a `<parallel>` enters every
            // non-history child region simultaneously.
            for (cid, child) in model.states.iter() {
                if child.parent.as_deref() == Some(&state.id) {
                    queue.push_back(cid.clone());
                }
            }
        } else if !state.initial.is_empty() {
            // Compound state with explicit `initial` (the parser
            // populates this for every compound state — either from
            // the attribute or the document-order default-first-child
            // fallback). Enqueue every whitespace-separated target
            // to honor the parallel-initial-entry shape.
            for target in state.initial.split_whitespace() {
                queue.push_back(target.to_string());
            }
        }

        // Transition `target` edges contribute regardless of state
        // category (compound / parallel / atomic). Multi-target
        // attributes (§scxml-3.13) enqueue every part.
        for trans in &state.transitions {
            for target in trans.target.split_whitespace() {
                queue.push_back(target.to_string());
            }
        }
    }

    reach
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::diagnostic::ToDiagnostics;
    use crate::model::{HistoryInfo, SCXMLModel, State, Transition};

    fn state(id: &str, document_order: u32) -> State {
        State {
            id: id.to_string(),
            document_order,
            ..Default::default()
        }
    }

    fn child(id: &str, parent: &str, document_order: u32) -> State {
        let mut s = state(id, document_order);
        s.parent = Some(parent.to_string());
        s
    }

    fn transition_to(target: &str) -> Transition {
        Transition {
            target: target.to_string(),
            ..Default::default()
        }
    }

    fn insert_states(model: &mut SCXMLModel, states: Vec<State>) {
        for s in states {
            model.states.insert(s.id.clone(), s);
        }
    }

    /// Two-state happy path: initial leads to a state with a self
    /// loop. Both states are reachable; the validator passes.
    #[test]
    fn flat_initial_plus_loop_is_reachable() {
        let mut model = SCXMLModel {
            initial: "idle".to_string(),
            ..Default::default()
        };
        let mut idle = state("idle", 0);
        idle.transitions.push(transition_to("running"));
        let mut running = state("running", 1);
        running.transitions.push(transition_to("idle"));
        insert_states(&mut model, vec![idle, running]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// State declared but with no path from initial — `UnreachableState`
    /// fires because the orphan has no transitions to surface.
    #[test]
    fn orphan_state_without_transitions_emits_unreachable_state() {
        let mut model = SCXMLModel {
            initial: "idle".to_string(),
            ..Default::default()
        };
        let idle = state("idle", 0);
        let orphan = state("orphan", 1);
        insert_states(&mut model, vec![idle, orphan]);
        let err = validate(&model, "test.scxml").expect_err("orphan must be rejected");
        let diags = err.error.to_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.as_str(), "scxml/unreachable-state");
        assert_eq!(diags[0].actual.as_deref(), Some("orphan"));
    }

    /// Orphan state carrying a transition fires the per-transition
    /// variant in preference — the (state, target) pair points the
    /// author at the concrete element to repair.
    #[test]
    fn orphan_state_with_transition_emits_dead_transition() {
        let mut model = SCXMLModel {
            initial: "idle".to_string(),
            ..Default::default()
        };
        let idle = state("idle", 0);
        let mut orphan = state("orphan", 1);
        orphan.transitions.push(transition_to("idle"));
        insert_states(&mut model, vec![idle, orphan]);
        let err = validate(&model, "test.scxml").expect_err("orphan must be rejected");
        let diags = err.error.to_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.as_str(), "scxml/dead-transition");
        assert_eq!(diags[0].actual.as_deref(), Some("idle"));
    }

    /// Compound state initial-cascade — entering parent `<state>` with
    /// `initial="child_a"` enters child_a; sibling child_b stays
    /// reachable only if a transition leads to it.
    #[test]
    fn compound_initial_cascade_enters_initial_child() {
        let mut model = SCXMLModel {
            initial: "parent".to_string(),
            ..Default::default()
        };
        let mut parent = state("parent", 0);
        parent.initial = "child_a".to_string();
        let child_a = child("child_a", "parent", 1);
        let child_b = child("child_b", "parent", 2);
        insert_states(&mut model, vec![parent, child_a, child_b]);
        // child_b has no incoming edge and is not parent's initial →
        // unreachable (parent is reachable; child_a is reachable via
        // cascade; child_b is the orphan).
        let err = validate(&model, "test.scxml").expect_err("child_b must be orphan");
        let diags = err.error.to_diagnostics();
        assert_eq!(diags[0].actual.as_deref(), Some("child_b"));
    }

    /// `<parallel>` entry enters every non-history child region per
    /// §scxml-3.4 — both regions stay reachable even without an
    /// explicit `initial`.
    #[test]
    fn parallel_enters_every_child_region() {
        let mut model = SCXMLModel {
            initial: "par".to_string(),
            ..Default::default()
        };
        let mut par = state("par", 0);
        par.is_parallel = true;
        let region_a = child("region_a", "par", 1);
        let region_b = child("region_b", "par", 2);
        insert_states(&mut model, vec![par, region_a, region_b]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// History pseudostate's `default_target` makes the target
    /// reachable per §scxml-3.10, even when no transition
    /// directly names the state.
    #[test]
    fn history_default_target_seeds_reachability() {
        let mut model = SCXMLModel {
            initial: "h".to_string(),
            ..Default::default()
        };
        // The history pseudostate is the document `initial`; it
        // redirects to `armed`. Without the redirect, `armed` would
        // be unreachable.
        let armed = state("armed", 0);
        insert_states(&mut model, vec![armed]);
        model.history_states.insert(
            "h".to_string(),
            HistoryInfo {
                parent: String::new(),
                history_type: "shallow".to_string(),
                default_target: "armed".to_string(),
                leaf_target: "armed".to_string(),
                default_actions: Vec::new(),
            },
        );
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// Empty `initial` → parser's first-top-level-state default
    /// applies. Verify a single top-level state without any
    /// `initial` attribute still reaches itself.
    #[test]
    fn empty_initial_falls_back_to_first_top_level_state() {
        let mut model = SCXMLModel::default();
        let only = state("only", 0);
        insert_states(&mut model, vec![only]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// Empty model short-circuits — `NoStates` is the earlier-firing
    /// diagnostic; reachability has nothing to walk.
    #[test]
    fn empty_model_is_silent() {
        let model = SCXMLModel::default();
        assert!(validate(&model, "test.scxml").is_ok());
    }
}
