// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Event-set exhaustiveness.
//
// Walks every compound `<state>` and checks that its sibling children
// agree on whether each event is handled. A genuine intent-gap pattern
// emerges when:
//
//   1. The siblings share at least one event in their declared
//      transition vocabulary (the "common ground" precondition), AND
//   2. Some other event is matched by some siblings but not others,
//      AND
//   3. The compound parent itself has no transition matching that
//      event (no fallthrough), AND
//   4. The parent is not annotated with `sce:exhaustive="false"` to
//      opt out.
//
// The common-ground precondition is the crucial false-positive guard.
// The W3C IRP test corpus is dominated by sequential protocol-stage
// patterns where one child handles `event_a` and the next handles
// `event_b` etc., with disjoint event vocabularies. Without the
// common-ground check the validator would reject those legitimate
// machines. With it, the validator fires only when the siblings
// clearly form a "common dispatch table" that one of them is missing
// an entry from — the AI-generated SCXML failure mode this phase
// targets.
//
// Event matching follows §scxml-5.10 / §scxml-3.12.1 semantics:
//
//   * A transition `event="*"` matches every event.
//   * A transition `event="foo.*"` matches events starting with
//     `foo.` (at least one more token).
//   * A transition `event="foo"` matches the exact `foo` and any
//     event of the form `foo.X` (token-prefix match).
//   * An eventless transition (no `event` attribute, or empty) does
//     not match any event — it fires on condition only and does not
//     contribute to the exhaustiveness analysis.
//   * Multi-token attributes (`event="foo bar baz"`) are treated as
//     the disjunction of their tokens.
//
// Runs after `scxml_reachability::validate` so an orphan state is
// reported as `scxml/unreachable-state` / `scxml/dead-transition`
// before the exhaustiveness pass surfaces a downstream consequence.

use std::collections::{BTreeSet, HashSet};

use crate::forge::error::{ForgeError, Located};
use crate::model::{SCXMLModel, State, Transition};
use crate::scxml_semantic::ScxmlSemanticError;

/// Reject the document on the first exhaustiveness violation.
/// Mirrors the short-circuit convention of `scxml_reachability::validate`
/// — emitting every gap in one pass would require collecting multiple
/// `Located<ForgeError>` records, which the wire layer does not model.
pub fn validate(model: &SCXMLModel, source: &str) -> Result<(), Located<ForgeError>> {
    if model.states.is_empty() {
        return Ok(());
    }

    // Walk parents in document order so the first-fired diagnostic is
    // deterministic across re-runs.
    let mut parents: Vec<&State> = model
        .states
        .values()
        .filter(|s| !s.is_parallel && !s.is_final && !s.exhaustive_optout)
        .collect();
    parents.sort_by_key(|s| s.document_order);

    for parent in parents {
        // Direct child `<state>` / `<parallel>` nodes that carry at
        // least one transition. `<final>` is excluded — final states
        // by definition have no transitions and would be flagged as
        // non-handlers for every event, which is the spec'd terminal
        // behavior, not a bug. History pseudostates are stored in
        // `model.history_states`, not `model.states`, so they never
        // appear here.
        let mut children: Vec<&State> = model
            .states
            .values()
            .filter(|c| c.parent.as_deref() == Some(&parent.id))
            .filter(|c| !c.is_final)
            .filter(|c| !c.transitions.is_empty())
            .collect();
        children.sort_by_key(|c| c.document_order);

        if children.len() < 2 {
            continue;
        }

        // Union of literal event tokens (no wildcards) declared
        // across all siblings. Wildcards expand the matching surface
        // but do not contribute new events to inspect — the spec'd
        // intent-gap pattern is about specific events that some
        // sibling missed, not about wildcard-coverage gaps.
        let mut universe: BTreeSet<String> = BTreeSet::new();
        for c in &children {
            for t in &c.transitions {
                for tok in t.event.split_whitespace() {
                    if let Some(literal) = literal_event_token(tok) {
                        universe.insert(literal);
                    }
                }
            }
        }
        if universe.is_empty() {
            continue;
        }

        // Common-ground precondition: there must exist at least one
        // event that every transition-carrying sibling matches. If
        // none exists, the siblings are dispatching disjoint event
        // families — this is the protocol-stage pattern, not a gap.
        let mut common_ground = false;
        for event in &universe {
            if children.iter().all(|c| state_handles_event(c, event)) {
                common_ground = true;
                break;
            }
        }
        if !common_ground {
            continue;
        }

        // Walk each candidate event. Skip those the parent already
        // absorbs via its own transitions (§scxml-3.13 bubble
        // semantics — a parent handler turns the gap into a
        // deliberate fallthrough).
        for event in &universe {
            if transitions_match_event(&parent.transitions, event) {
                continue;
            }
            let mut handlers: Vec<String> = Vec::new();
            let mut non_handlers: Vec<String> = Vec::new();
            for c in &children {
                if state_handles_event(c, event) {
                    handlers.push(c.id.clone());
                } else {
                    non_handlers.push(c.id.clone());
                }
            }
            if !handlers.is_empty() && !non_handlers.is_empty() {
                return Err(Located::new(
                    ScxmlSemanticError::NonExhaustiveEventHandling {
                        parent: parent.id.clone(),
                        event: event.clone(),
                        handlers,
                        non_handlers,
                    }
                    .into(),
                    source,
                    None,
                    None,
                ));
            }
        }
    }

    Ok(())
}

/// Strip wildcard markers from a single event token. Returns the
/// literal form when the token is a concrete event name (`foo`,
/// `foo.bar`), the dot-prefix when it is a `.*`-suffixed pattern
/// (`foo.*` → `foo`), and `None` for the universal wildcards (`*`,
/// `.*`) which contribute no specific event to the analysis.
fn literal_event_token(tok: &str) -> Option<String> {
    if tok.is_empty() || tok == "*" || tok == ".*" {
        return None;
    }
    if let Some(prefix) = tok.strip_suffix(".*") {
        if prefix.is_empty() {
            return None;
        }
        return Some(prefix.to_string());
    }
    Some(tok.to_string())
}

/// Does any of `state`'s transitions match `event` per W3C SCXML
/// §scxml-3.12.1 semantics (or via the universal wildcards `*` / `.*`)?
fn state_handles_event(state: &State, event: &str) -> bool {
    transitions_match_event(&state.transitions, event)
}

/// Does any transition in the slice match `event`?
fn transitions_match_event(transitions: &[Transition], event: &str) -> bool {
    transitions
        .iter()
        .any(|t| transition_matches_event(t, event))
}

/// Single-transition match per §scxml-3.12.1 token-prefix rules
/// plus the `*` / `.*` universal-wildcard convention this codebase
/// already adopts (mirrors the `is_pure_in_predicate` / event-set
/// collection code in `parser.rs`).
fn transition_matches_event(t: &Transition, event: &str) -> bool {
    if event.is_empty() {
        return false;
    }
    // De-duplicate via HashSet so multi-token attributes with
    // repeated entries do not pay the per-token comparison twice.
    let mut seen: HashSet<&str> = HashSet::new();
    for tok in t.event.split_whitespace() {
        if !seen.insert(tok) {
            continue;
        }
        if tok == "*" || tok == ".*" {
            return true;
        }
        if let Some(prefix) = tok.strip_suffix(".*") {
            // `prefix.*` requires at least one more token after the
            // prefix. `event` must start with `prefix.` (with the
            // dot) and have something after it.
            if prefix.is_empty() {
                return true; // bare `.*` is universal
            }
            if let Some(rest) = event.strip_prefix(prefix) {
                if rest.starts_with('.') && rest.len() > 1 {
                    return true;
                }
            }
        } else {
            // Bare descriptor: matches event == descriptor (exact)
            // or event starts with descriptor + "." (token-prefix).
            if tok == event {
                return true;
            }
            if let Some(rest) = event.strip_prefix(tok) {
                if rest.starts_with('.') {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::diagnostic::ToDiagnostics;
    use crate::model::{SCXMLModel, State, Transition};

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

    fn transition(event: &str, target: &str) -> Transition {
        Transition {
            event: event.to_string(),
            target: target.to_string(),
            ..Default::default()
        }
    }

    fn insert_states(model: &mut SCXMLModel, states: Vec<State>) {
        for s in states {
            model.states.insert(s.id.clone(), s);
        }
    }

    // ── transition_matches_event ─────────────────────────────────

    #[test]
    fn exact_match() {
        let t = transition("foo", "x");
        assert!(transition_matches_event(&t, "foo"));
        assert!(!transition_matches_event(&t, "bar"));
    }

    #[test]
    fn token_prefix_match() {
        let t = transition("foo", "x");
        assert!(transition_matches_event(&t, "foo.bar"));
        assert!(transition_matches_event(&t, "foo.bar.baz"));
        // Not a token-prefix: missing the dot boundary.
        assert!(!transition_matches_event(&t, "foobar"));
    }

    #[test]
    fn dotstar_match() {
        let t = transition("error.*", "x");
        assert!(transition_matches_event(&t, "error.fatal"));
        assert!(transition_matches_event(&t, "error.fatal.detail"));
        // Bare `error` does not match `error.*` — the pattern
        // requires at least one more token.
        assert!(!transition_matches_event(&t, "error"));
    }

    #[test]
    fn star_is_universal() {
        let t = transition("*", "x");
        assert!(transition_matches_event(&t, "anything"));
        assert!(transition_matches_event(&t, "foo.bar"));
    }

    #[test]
    fn empty_event_attribute_matches_nothing() {
        let t = transition("", "x");
        assert!(!transition_matches_event(&t, "foo"));
    }

    #[test]
    fn multi_token_disjunction() {
        let t = transition("foo bar", "x");
        assert!(transition_matches_event(&t, "foo"));
        assert!(transition_matches_event(&t, "bar"));
        assert!(transition_matches_event(&t, "foo.sub"));
        assert!(!transition_matches_event(&t, "baz"));
    }

    // ── validator behaviour ──────────────────────────────────────

    /// Common-ground precondition: with disjoint event vocabularies
    /// across siblings (the W3C-IRP-style protocol-stage pattern),
    /// the validator must stay silent even when each event is
    /// missing from some siblings.
    #[test]
    fn disjoint_event_vocab_accepted() {
        // Mirrors W3C test207-style structure:
        //   parent has two children, each handling a disjoint event
        //   family. No common ground → no flag.
        let mut model = SCXMLModel {
            initial: "parent".to_string(),
            ..Default::default()
        };
        let parent = state("parent", 0);
        let mut a = child("a", "parent", 1);
        a.transitions.push(transition("childToParent", "b"));
        let mut b = child("b", "parent", 2);
        b.transitions.push(transition("pass", "done"));
        b.transitions.push(transition("fail", "done"));
        let done = state("done", 3);
        insert_states(&mut model, vec![parent, a, b, done]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// Genuine intent gap: three siblings share `cmd.stop` as common
    /// ground; `cmd.start` is matched by two but not the third.
    /// The validator must flag the missing handler.
    #[test]
    fn genuine_gap_flagged() {
        let mut model = SCXMLModel {
            initial: "dispatch".to_string(),
            ..Default::default()
        };
        let dispatch = state("dispatch", 0);
        let mut idle = child("idle", "dispatch", 1);
        idle.transitions.push(transition("cmd.start", "active"));
        idle.transitions.push(transition("cmd.stop", "stopped"));
        let mut active = child("active", "dispatch", 2);
        active.transitions.push(transition("cmd.stop", "stopped"));
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.start", "active"));
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped]);
        let err = validate(&model, "test.scxml").expect_err("gap must be rejected");
        let diags = err.error.to_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].code.as_str(),
            "scxml/non-exhaustive-event-handling"
        );
        assert_eq!(diags[0].actual.as_deref(), Some("cmd.start"));
    }

    /// Parent-level fallthrough absorbs the gap: even though only two
    /// of three siblings handle `cmd.start`, the parent itself has a
    /// transition matching it, so bubble semantics handle the third
    /// sibling's case correctly. Accept.
    #[test]
    fn parent_fallthrough_accepted() {
        let mut model = SCXMLModel {
            initial: "dispatch".to_string(),
            ..Default::default()
        };
        let mut dispatch = state("dispatch", 0);
        dispatch
            .transitions
            .push(transition("cmd.start", "dispatch"));
        let mut idle = child("idle", "dispatch", 1);
        idle.transitions.push(transition("cmd.start", "active"));
        idle.transitions.push(transition("cmd.stop", "stopped"));
        let mut active = child("active", "dispatch", 2);
        active.transitions.push(transition("cmd.stop", "stopped"));
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.start", "active"));
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// `sce:exhaustive="false"` on the compound parent silences the
    /// validator for that parent regardless of the gap shape.
    #[test]
    fn exhaustive_optout_accepted() {
        let mut model = SCXMLModel {
            initial: "dispatch".to_string(),
            ..Default::default()
        };
        let mut dispatch = state("dispatch", 0);
        dispatch.exhaustive_optout = true;
        let mut idle = child("idle", "dispatch", 1);
        idle.transitions.push(transition("cmd.start", "active"));
        idle.transitions.push(transition("cmd.stop", "stopped"));
        let mut active = child("active", "dispatch", 2);
        active.transitions.push(transition("cmd.stop", "stopped"));
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.start", "active"));
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// Wildcard handler covers the event in the missing-sibling
    /// position: if a sibling has `event="*"`, every event is
    /// matched, so the validator must not flag.
    #[test]
    fn wildcard_handler_covers_event() {
        let mut model = SCXMLModel {
            initial: "dispatch".to_string(),
            ..Default::default()
        };
        let dispatch = state("dispatch", 0);
        let mut idle = child("idle", "dispatch", 1);
        idle.transitions.push(transition("cmd.start", "active"));
        idle.transitions.push(transition("cmd.stop", "stopped"));
        let mut active = child("active", "dispatch", 2);
        // Wildcard catches everything including `cmd.start`.
        active.transitions.push(transition("*", "stopped"));
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.start", "active"));
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// Only one child has transitions — the validator requires ≥2
    /// transition-carrying siblings to fire. Single-sibling
    /// "compound" states have no analog to compare.
    #[test]
    fn single_transition_carrying_child_accepted() {
        let mut model = SCXMLModel {
            initial: "parent".to_string(),
            ..Default::default()
        };
        let parent = state("parent", 0);
        let mut a = child("a", "parent", 1);
        a.transitions.push(transition("go", "b"));
        // b has no transitions — excluded from the sibling set.
        let b = child("b", "parent", 2);
        insert_states(&mut model, vec![parent, a, b]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// `<final>` children are excluded from the sibling set — a
    /// `<final>` has no transitions by spec and must not be treated
    /// as a "non-handler" of any event.
    #[test]
    fn final_sibling_excluded() {
        let mut model = SCXMLModel {
            initial: "parent".to_string(),
            ..Default::default()
        };
        let parent = state("parent", 0);
        let mut a = child("a", "parent", 1);
        a.transitions.push(transition("go", "done"));
        a.transitions.push(transition("stop", "done"));
        let mut b = child("b", "parent", 2);
        b.transitions.push(transition("go", "a"));
        b.transitions.push(transition("stop", "done"));
        let mut done = child("done", "parent", 3);
        done.is_final = true;
        insert_states(&mut model, vec![parent, a, b, done]);
        // a and b both handle go + stop; `done` is final and
        // excluded; no gap.
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// `<parallel>` parent is excluded from the analysis — parallel
    /// regions are orthogonal by design and each region handles its
    /// own event surface independently.
    #[test]
    fn parallel_parent_excluded() {
        let mut model = SCXMLModel {
            initial: "par".to_string(),
            ..Default::default()
        };
        let mut par = state("par", 0);
        par.is_parallel = true;
        let mut a = child("a", "par", 1);
        a.transitions.push(transition("go", "a"));
        a.transitions.push(transition("stop", "a"));
        let mut b = child("b", "par", 2);
        b.transitions.push(transition("go", "b"));
        // b is missing `stop` — would be flagged if `par` were a
        // compound state, but the parallel exclusion silences it.
        insert_states(&mut model, vec![par, a, b]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// Two siblings share a common-ground event and one is missing
    /// another event in the family — the minimal positive case
    /// matching the user's reference fixture spec.
    #[test]
    fn two_sibling_gap_flagged() {
        let mut model = SCXMLModel {
            initial: "parent".to_string(),
            ..Default::default()
        };
        let parent = state("parent", 0);
        let mut a = child("a", "parent", 1);
        a.transitions.push(transition("go", "b"));
        a.transitions.push(transition("stop", "b"));
        let mut b = child("b", "parent", 2);
        // common ground: both handle `stop`. `go` is missing in b.
        b.transitions.push(transition("stop", "a"));
        insert_states(&mut model, vec![parent, a, b]);
        let err = validate(&model, "test.scxml").expect_err("gap must be rejected");
        let diags = err.error.to_diagnostics();
        assert_eq!(
            diags[0].code.as_str(),
            "scxml/non-exhaustive-event-handling"
        );
        assert_eq!(diags[0].actual.as_deref(), Some("go"));
    }

    /// Empty model — earlier-firing diagnostics (`NoStates`,
    /// reachability) handle this; exhaustiveness short-circuits.
    #[test]
    fn empty_model_is_silent() {
        let model = SCXMLModel::default();
        assert!(validate(&model, "test.scxml").is_ok());
    }
}
