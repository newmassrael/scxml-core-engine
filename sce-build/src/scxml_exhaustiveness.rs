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
//   4. The child leaving the event unhandled has not declared it in
//      `sce:unhandled`.
//
// Condition 4 is checked per (child, event) pair, not per parent. A
// parent-level opt-out existed once and was withdrawn: it silenced
// every gap under the parent, including gaps introduced after it was
// written, so a sibling added later inherited an exemption nobody had
// judged — the same defect this validator exists to catch, one level
// down. Declaring the absence on the child that has it keeps the
// annotation at the grain the author's claim is actually true at, and
// makes it checkable in both directions: a declared event the child
// handles is a contradiction, and a declared event that is not a gap
// is stale. Neither can rot into unverified prose.
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
// It guards the REPORT, and only the report. Condition 1 above is not
// part of what makes an `sce:unhandled` declaration true: a sibling
// handles the event, this child does not, and that is so whatever the
// rest of the compound looks like. Judging declarations against the
// filtered set refused true declarations in every compound the filter
// excluded, which is what made this check impossible to pay in advance
// — see `Inconsistencies`.
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

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::forge::error::{ForgeError, Located};
use crate::model::{SCXMLModel, State, Transition};
use crate::scxml_semantic::ScxmlSemanticError;

/// Reject the document on the first exhaustiveness violation.
/// Mirrors the short-circuit convention of `scxml_reachability::validate`
/// — emitting every gap in one pass would require collecting multiple
/// `Located<ForgeError>` records, which the wire layer does not model.
///
/// Two passes, in this order:
///
///   1. Every `sce:unhandled` declaration in the document is checked
///      against the gap set. A declaration is the author telling the
///      build a fact about their machine; validating it before
///      consuming it means a gap report is never suppressed by a
///      declaration that turns out to be untrue.
///   2. Gaps that no declaration covers are reported.
///
/// Both walk in document order, so which violation surfaces first is
/// stable across re-runs.
pub fn validate(model: &SCXMLModel, source: &str) -> Result<(), Located<ForgeError>> {
    if model.states.is_empty() {
        return Ok(());
    }

    let found = collect_gaps(model);
    check_declarations(model, &found.inconsistent, source)?;
    report_uncovered_gaps(model, &found.reportable, source)
}

/// Every gap in the document, keyed by compound parent id and then by
/// event, valued by the child ids that leave that event unhandled.
type GapsByParent = BTreeMap<String, GapSet>;

/// The two questions this walk answers, kept apart because they are two
/// questions.
///
/// They were one map, shared by both passes so that "the declaration
/// check and the gap report can never disagree about what a gap is".
/// The agreement was real and the cost was too: the common-ground
/// precondition is a REPORTING heuristic, and a map filtered by it
/// cannot state a fact about a compound the heuristic excludes.
/// Measured on `examples/ai_loop/ai_loop.scxml`, whose `watch` region is
/// exactly the excluded shape — `alive` handles `session.lost`,
/// `rebuilding` handles `session.ready`, no event in common. Declaring
/// `sce:unhandled="session.ready"` on `alive` — a true statement about
/// that document — was refused with "that state has no
/// inconsistently-handled events at all". So an author could not pay
/// this check in advance: the declaration became sayable only in the
/// round where the shape acquired common ground and the lint started
/// demanding it.
///
/// The agreement that has to hold is one-directional, and it still
/// does: `reportable` is a subset of `inconsistent`, so a declaration
/// covering a reported gap is always valid and always silences it. What
/// is no longer true is the converse — a declaration may be valid about
/// a compound nothing is reported for, which is precisely the
/// pre-payment the shared map made impossible.
struct Inconsistencies {
    /// The FACT: every (parent, event) where some transition-carrying
    /// sibling handles the event and another does not, with no
    /// heuristic applied. What a `sce:unhandled` declaration is checked
    /// against, because the declaration is a claim about the document
    /// rather than about what this validator chooses to report.
    inconsistent: GapsByParent,
    /// The HEURISTIC: the subset whose parent passed the common-ground
    /// precondition. What is reported, because a compound whose
    /// children dispatch disjoint event families is the sequential
    /// protocol-stage pattern rather than a dispatch table with a hole.
    reportable: GapsByParent,
}

/// One parent's gaps: event → the child ids not handling it, both in
/// the order the validator found them (`BTreeMap` over the event
/// universe, document order within a gap).
type GapSet = BTreeMap<String, Vec<String>>;

/// Walk every compound parent and collect its gaps.
fn collect_gaps(model: &SCXMLModel) -> Inconsistencies {
    let mut out = Inconsistencies {
        inconsistent: BTreeMap::new(),
        reportable: BTreeMap::new(),
    };

    // Walk parents in document order so the first-fired diagnostic is
    // deterministic across re-runs.
    let mut parents: Vec<&State> = model
        .states
        .values()
        .filter(|s| !s.is_parallel && !s.is_final)
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
        // families — this is the protocol-stage pattern, and an
        // inconsistency there is not worth REPORTING.
        //
        // No longer a `continue`. The walk below still runs, and what
        // it finds still lands in `inconsistent` — the facts about this
        // compound do not change with the shape of its neighbours. Only
        // `reportable` is gated, which is the half the precondition was
        // ever about. See `Inconsistencies` for the measurement that
        // separated them.
        let mut common_ground = false;
        for event in &universe {
            if children.iter().all(|c| state_handles_event(c, event)) {
                common_ground = true;
                break;
            }
        }

        // Walk each candidate event. Skip those the parent already
        // absorbs via its own transitions (§scxml-3.13 bubble
        // semantics — a parent handler turns the gap into a
        // deliberate fallthrough).
        //
        // Every gap under this parent is collected, not just the
        // first: the author repairing this compound decides about all
        // of them, and a report that stops at the first costs a build
        // round per gap. Measured on this tree's own documents:
        // `combo_state`'s `active` has three gaps and the author's
        // comment reasoned about one.
        //
        // `sce:unhandled` declarations are deliberately NOT consulted
        // here. This map is what a declaration is checked against, so
        // letting declarations shrink it would make a declaration
        // self-justifying.
        let mut gaps: GapSet = BTreeMap::new();
        for event in &universe {
            if transitions_match_event(&parent.transitions, event) {
                continue;
            }
            let mut any_handler = false;
            let mut non_handlers: Vec<String> = Vec::new();
            for c in &children {
                if state_handles_event(c, event) {
                    any_handler = true;
                } else {
                    non_handlers.push(c.id.clone());
                }
            }
            if any_handler && !non_handlers.is_empty() {
                gaps.insert(event.clone(), non_handlers);
            }
        }
        if !gaps.is_empty() {
            if common_ground {
                out.reportable.insert(parent.id.clone(), gaps.clone());
            }
            out.inconsistent.insert(parent.id.clone(), gaps);
        }
    }

    out
}

/// Check every `sce:unhandled` declaration in the document against the
/// inconsistency FACTS, rejecting the first that is untrue.
///
/// `gaps_by_parent` here is `Inconsistencies::inconsistent`, not the
/// reportable subset: a declaration is the author stating something
/// about their own document, and whether this validator would have
/// reported it is not part of whether it is so.
///
/// Walks all states, not just the children the gap walk considered: a
/// declaration on a `<final>`, on a transition-less state, or on a
/// top-level state is exactly as stale as one naming the wrong event,
/// and a walk restricted to gap-eligible children would let those three
/// stand unexamined.
fn check_declarations(
    model: &SCXMLModel,
    gaps_by_parent: &GapsByParent,
    source: &str,
) -> Result<(), Located<ForgeError>> {
    let mut declarers: Vec<&State> = model
        .states
        .values()
        .filter(|s| !s.unhandled.is_empty())
        .collect();
    declarers.sort_by_key(|s| s.document_order);

    for state in declarers {
        // `None` covers both "no compound parent" and "a parent with no
        // gaps at all"; every declaration under either is stale, and the
        // message distinguishes them by reporting the parent id.
        let parent_gaps = state.parent.as_deref().and_then(|p| gaps_by_parent.get(p));

        for event in &state.unhandled {
            // The local contradiction first: a state that handles the
            // event it declares unhandled is wrong regardless of what
            // its siblings do, and reporting the sibling-scoped
            // staleness instead would send the author looking at the
            // wrong half of the document.
            if state_handles_event(state, event) {
                return Err(Located::new(
                    ScxmlSemanticError::ContradictoryUnhandledDeclaration {
                        state: state.id.clone(),
                        event: event.clone(),
                    }
                    .into(),
                    source,
                    None,
                    None,
                ));
            }

            let covers = parent_gaps.is_some_and(|gaps| {
                gaps.get(event)
                    .is_some_and(|non_handlers| non_handlers.contains(&state.id))
            });
            if !covers {
                return Err(Located::new(
                    ScxmlSemanticError::StaleUnhandledDeclaration {
                        state: state.id.clone(),
                        parent: state.parent.clone().unwrap_or_else(|| "(none)".to_string()),
                        event: event.clone(),
                        gaps: parent_gaps
                            .map(|gaps| {
                                gaps.iter()
                                    .filter(|(_, non_handlers)| non_handlers.contains(&state.id))
                                    .map(|(e, _)| e.clone())
                                    .collect()
                            })
                            .unwrap_or_default(),
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

/// Report the first gap no `sce:unhandled` declaration covers.
///
/// A gap survives only for the children that did not declare it, so a
/// compound where two of three non-handlers declared the event is
/// reported against the third alone — the report tracks what is left to
/// decide, not what the gap looked like before anyone decided anything.
fn report_uncovered_gaps(
    model: &SCXMLModel,
    gaps_by_parent: &GapsByParent,
    source: &str,
) -> Result<(), Located<ForgeError>> {
    let mut parents: Vec<&State> = model
        .states
        .values()
        .filter(|s| gaps_by_parent.contains_key(&s.id))
        .collect();
    parents.sort_by_key(|s| s.document_order);

    for parent in parents {
        let gaps = &gaps_by_parent[&parent.id];

        // Only the undeclared remainder of each gap, in the event
        // order the gap walk produced.
        let mut open: Vec<(String, Vec<String>)> = Vec::new();
        for (event, non_handlers) in gaps {
            let undeclared: Vec<String> = non_handlers
                .iter()
                .filter(|id| !declares_unhandled(model, id, event))
                .cloned()
                .collect();
            if !undeclared.is_empty() {
                open.push((event.clone(), undeclared));
            }
        }

        let Some((event, non_handlers)) = open.first().cloned() else {
            continue;
        };
        let also: Vec<String> = open.iter().skip(1).map(|(e, _)| e.clone()).collect();
        let mut handling: Vec<&State> = model
            .states
            .values()
            .filter(|c| c.parent.as_deref() == Some(&parent.id))
            .filter(|c| !c.is_final && !c.transitions.is_empty())
            .filter(|c| state_handles_event(c, &event))
            .collect();
        handling.sort_by_key(|c| c.document_order);
        let handlers: Vec<String> = handling.iter().map(|c| c.id.clone()).collect();

        return Err(Located::new(
            ScxmlSemanticError::NonExhaustiveEventHandling {
                parent: parent.id.clone(),
                event,
                handlers,
                non_handlers,
                also,
            }
            .into(),
            source,
            None,
            None,
        ));
    }

    Ok(())
}

/// Does the named state declare `event` in its `sce:unhandled`?
///
/// Declarations are matched literally against the literal gap event —
/// the parser rejects wildcards in the attribute precisely so this
/// comparison never needs a second matching rule.
fn declares_unhandled(model: &SCXMLModel, state_id: &str, event: &str) -> bool {
    model
        .states
        .get(state_id)
        .is_some_and(|s| s.unhandled.iter().any(|e| e == event))
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

    /// The gap shape the parent-level opt-out used to silence, now
    /// declared on the child that actually leaves `cmd.start`
    /// unhandled.
    #[test]
    fn unhandled_declaration_on_the_non_handling_child_accepted() {
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
        active.unhandled = vec!["cmd.start".to_string()];
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.start", "active"));
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped]);
        assert!(validate(&model, "test.scxml").is_ok());
    }

    /// The defect this attribute shape exists to prevent: a sibling
    /// added after the exemption was written inherits nothing.
    ///
    /// `active` declares `cmd.start` unhandled, which is true of
    /// `active`. A later `draining` sibling also fails to handle it
    /// and declares nothing — under a parent-level opt-out the whole
    /// compound would still be silent, and nobody would ever judge
    /// `draining`.
    #[test]
    fn a_sibling_added_later_inherits_no_exemption() {
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
        active.unhandled = vec!["cmd.start".to_string()];
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.start", "active"));
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        let mut draining = child("draining", "dispatch", 4);
        draining.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped, draining]);

        let err = validate(&model, "test.scxml").expect_err("draining must be judged on its own");
        match &err.error {
            ForgeError::Scxml(boxed) => match boxed.as_ref() {
                ScxmlSemanticError::NonExhaustiveEventHandling {
                    event,
                    non_handlers,
                    ..
                } => {
                    assert_eq!(event, "cmd.start");
                    // `active` declared it; only `draining` is left to
                    // decide about.
                    assert_eq!(non_handlers, &vec!["draining".to_string()]);
                }
                other => panic!("expected NonExhaustiveEventHandling, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }

    /// A declaration naming an event the state actually handles is a
    /// contradiction, reported without reference to siblings.
    #[test]
    fn declaring_an_event_the_state_handles_rejects() {
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
        // Declares the very event its own transition handles.
        active.unhandled = vec!["cmd.stop".to_string()];
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.start", "active"));
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped]);

        let err = validate(&model, "test.scxml").expect_err("contradiction must reject");
        match &err.error {
            ForgeError::Scxml(boxed) => match boxed.as_ref() {
                ScxmlSemanticError::ContradictoryUnhandledDeclaration { state, event } => {
                    assert_eq!(state, "active");
                    assert_eq!(event, "cmd.stop");
                }
                other => panic!("expected ContradictoryUnhandledDeclaration, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }

    /// A declaration survives the repair that made it unnecessary:
    /// `idle` and `stopped` both drop `cmd.start`, so it stops being
    /// a gap, and `active`'s declaration now exempts nothing.
    #[test]
    fn a_declaration_that_stopped_being_a_gap_rejects() {
        let mut model = SCXMLModel {
            initial: "dispatch".to_string(),
            ..Default::default()
        };
        let dispatch = state("dispatch", 0);
        let mut idle = child("idle", "dispatch", 1);
        idle.transitions.push(transition("cmd.stop", "stopped"));
        let mut active = child("active", "dispatch", 2);
        active.transitions.push(transition("cmd.stop", "stopped"));
        active.unhandled = vec!["cmd.start".to_string()];
        let mut stopped = child("stopped", "dispatch", 3);
        stopped.transitions.push(transition("cmd.stop", "stopped"));
        insert_states(&mut model, vec![dispatch, idle, active, stopped]);

        let err = validate(&model, "test.scxml").expect_err("stale declaration must reject");
        match &err.error {
            ForgeError::Scxml(boxed) => match boxed.as_ref() {
                ScxmlSemanticError::StaleUnhandledDeclaration {
                    state,
                    parent,
                    event,
                    gaps,
                } => {
                    assert_eq!(state, "active");
                    assert_eq!(parent, "dispatch");
                    assert_eq!(event, "cmd.start");
                    assert!(gaps.is_empty(), "no gap remains under dispatch: {gaps:?}");
                }
                other => panic!("expected StaleUnhandledDeclaration, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }

    /// A declaration on a state with no compound parent has no gap
    /// set to be true against, so it is stale rather than ignored.
    #[test]
    fn a_declaration_on_a_parentless_state_rejects() {
        let mut model = SCXMLModel {
            initial: "lonely".to_string(),
            ..Default::default()
        };
        let mut lonely = state("lonely", 0);
        lonely.unhandled = vec!["cmd.start".to_string()];
        insert_states(&mut model, vec![lonely]);

        let err = validate(&model, "test.scxml").expect_err("parentless declaration must reject");
        match &err.error {
            ForgeError::Scxml(boxed) => match boxed.as_ref() {
                ScxmlSemanticError::StaleUnhandledDeclaration { state, parent, .. } => {
                    assert_eq!(state, "lonely");
                    assert_eq!(parent, "(none)");
                }
                other => panic!("expected StaleUnhandledDeclaration, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
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
