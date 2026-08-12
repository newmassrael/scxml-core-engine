// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Statechart state-reference resolution.
//
// Every id a document uses to name a state — `<transition target>`,
// `<state initial>`, `<initial>`'s transition target, and a
// `<history>`'s default configuration — must resolve to a
// `<state>` / `<parallel>` / `<final>` / `<history>` declared in that
// document. This pass rejects the ones that do not.
//
// Two diagnostic surfaces, both REUSED from the existing wire-code
// inventory (no new wire code):
//
//   * `ScxmlSemanticError::TransitionTargetUnknown` — a transition
//     target token names nothing. Mirrors the C++ Interpreter's
//     `SCXMLParser::validateModel` throw of
//     `SemanticTransitionTargetUnknown`, which is the producer this
//     module's Rust counterpart had been missing: the variant, its
//     wire mapping, and both cross-side drift tests existed while no
//     Rust site ever constructed it.
//   * `ScxmlSemanticError::InitialStateUnknown` (compound scope) — a
//     compound state's `initial` token names nothing. The root-scope
//     form is produced earlier by `analyzer::can_generate_static`.
//
// Why the pass is load-bearing rather than a quality check: a target
// that does not resolve reaches the emitters as a plain state name and
// lowers to `<Machine>State::<Variant>` — a variant the generated
// `State` enum never declares, because the id names nothing. The
// document therefore passes `check` with `status: ok`, passes
// `generate`, and fails in the consumer's compiler. SCE must never
// answer "this document lowers to <language>" and then emit code that
// language rejects; `sce-build/tests/scxml_references.rs` pins each
// shape that used to do exactly that.
//
// Placement: called from `analyzer::can_generate_static`, which is the
// one gate both pipelines already share — the library entry reaches it
// through `lib.rs::guard_static_generatable`, and every `sce-codegen`
// subcommand calls it directly. That matters because the CLI does not
// route through `compile_model`: it re-implements parse → analyze →
// generate and therefore never runs the validators that live only in
// that chain (`scxml_reachability`, `scxml_exhaustiveness`,
// `scxml_guard_analysis`). A reference rule installed in the chain
// would have covered the library and left `sce-codegen check` — the
// surface consumers actually read — accepting the document.
//
// The ordering the chain wanted still holds: `can_generate_static`
// runs before `scxml_reachability::validate`, and the reachability BFS
// skips ids it cannot resolve on the stated assumption that this pass
// already rejected them. Inverted, an unresolved target would surface
// as an orphan region (a consequence) instead of as the typo (the
// cause).
//
// Rule order within the pass is deliberate: history default
// configurations are checked before transition targets. A transition
// naming a history pseudostate has already been rewritten by
// `parser::resolve_history_targets` to carry the history's default
// target, so a bad default would otherwise surface at every
// *referencing* transition instead of at the `<history>` element that
// declares it.

use crate::forge::error::{ForgeError, Located, SourceLocation};
use crate::model::SCXMLModel;
use crate::scxml_semantic::{InitialStateScope, ScxmlSemanticError};

/// Reject the document on the first unresolved state reference.
/// Short-circuits like its sibling validators
/// (`scxml_reachability::validate`, `scxml_exhaustiveness::validate`)
/// because the wire layer models one rejection per document.
///
/// Returns a [`Located`] error rather than a bare [`ForgeError`]: the
/// unresolved name sits on a `<transition>` or `<state>` the model
/// already carries a `source_location` for, and
/// SCE_ERROR_CONTRACT.md §2.2 has the consumer open `location.file`
/// and edit there. Handing the caller a bare error meant the shared
/// wrapping site could only supply the file, so every
/// `validation/invalid-reference` reached the wire with no line — and
/// after `<sce:use>` expansion the rejected value is often a
/// substituted string that appears nowhere in the file the record
/// names, which leaves a whole-file search as the consumer's only
/// strategy and that search finding nothing.
///
/// `diag_label` is the document as the caller named it (§2.2), not
/// the artifact-facing basename the model's own `source_location`
/// carries.
pub fn validate(model: &SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    // `ScxmlSemanticError::NoStates` fires earlier in the pipeline;
    // there is nothing to resolve against here.
    if model.states.is_empty() {
        return Ok(());
    }

    // §scxml-3.5: the legal target set is every declared state plus
    // every history pseudostate. History ids belong in it because a
    // transition may name one (§scxml-3.10) even though the runtime
    // never occupies it.
    let declared_states: Vec<String> = model.states.keys().cloned().collect();

    // §scxml-3.10.2 — a history's default configuration is a transition
    // target and resolves by the same rule. Checked first so the
    // diagnostic lands on the `<history>` element rather than on each
    // transition the parser rewrote to point at its default.
    for (history_id, info) in &model.history_states {
        for token in info.default_target.split_whitespace() {
            if !resolves(model, token) {
                // The `<history>` element's own coordinate is the
                // parent state's: `HistoryInfo` records the parent id,
                // not a node position.
                let at = model
                    .states
                    .get(&info.parent)
                    .and_then(|s| s.source_location.as_ref());
                return Err(reject_target(
                    history_id,
                    token,
                    &declared_states,
                    at,
                    diag_label,
                    model,
                ));
            }
        }
    }

    // Document order keeps the first-fired diagnostic stable across
    // runs (`model.states` is keyed by id, not by position).
    let mut ordered: Vec<&crate::model::State> = model.states.values().collect();
    ordered.sort_by_key(|s| s.document_order);

    for state in ordered {
        // §scxml-3.3 / §scxml-3.6: `initial` — whether written as the
        // attribute or folded in from an `<initial>` child — must name
        // children of this state.
        for token in state.initial.split_whitespace() {
            if !resolves(model, token) {
                // Candidates are this state's children, not every
                // declared id: §scxml-3.3 restricts the initial
                // configuration to descendants of the owning state, so
                // a wider list would put illegal values on the
                // `Fix::ReplaceOneOf` wire.
                let children: Vec<String> = child_ids(model, &state.id);
                return Err(located(
                    ScxmlSemanticError::InitialStateUnknown {
                        state_id: token.to_string(),
                        scope: InitialStateScope::CompoundState {
                            parent_id: state.id.clone(),
                        },
                        available: children,
                    }
                    .into(),
                    state.source_location.as_ref(),
                    diag_label,
                    model,
                ));
            }
        }

        // §scxml-3.5 / §scxml-3.13: every whitespace-separated token of
        // a multi-target attribute resolves independently.
        for trans in &state.transitions {
            for token in trans.target.split_whitespace() {
                if !resolves(model, token) {
                    return Err(reject_target(
                        &state.id,
                        token,
                        &declared_states,
                        trans.source_location.as_ref(),
                        diag_label,
                        model,
                    ));
                }
            }
        }
    }

    Ok(())
}

/// True when `id` names a declared state or history pseudostate.
fn resolves(model: &SCXMLModel, id: &str) -> bool {
    model.states.contains_key(id) || model.history_states.contains_key(id)
}

/// Direct children of `parent_id` in document order — the legal set
/// for that state's `initial` (the rule is cited at the call site).
fn child_ids(model: &SCXMLModel, parent_id: &str) -> Vec<String> {
    let mut children: Vec<&crate::model::State> = model
        .states
        .values()
        .filter(|s| s.parent.as_deref() == Some(parent_id))
        .collect();
    children.sort_by_key(|s| s.document_order);
    children.into_iter().map(|s| s.id.clone()).collect()
}

fn reject_target(
    owner: &str,
    token: &str,
    declared_states: &[String],
    at: Option<&SourceLocation>,
    diag_label: &str,
    model: &SCXMLModel,
) -> Located<ForgeError> {
    located(
        ScxmlSemanticError::TransitionTargetUnknown {
            state: owner.to_string(),
            target: token.to_string(),
            available: declared_states.to_vec(),
        }
        .into(),
        at,
        diag_label,
        model,
    )
}

/// Anchor an error on a node position the model recorded.
///
/// The recorded position indexes into the *expanded* document, so it
/// is resolved through the model's own mapping first. Two things can
/// come back:
///
/// * Nothing to resolve (no preprocessor ran) — the row is already an
///   authored row of `diag_label`, and only the file half is taken
///   from there. The recorded [`SourceLocation`] cannot supply it: it
///   carries the artifact spelling (a basename, so an SCE-MAP marker
///   does not bake one checkout into the generated tree) and a
///   diagnostic must name the document the way the caller named it
///   (§2.2).
/// * An authored origin — which after `<sce:use>` / `<xi:include>`
///   expansion is often a *different file* than the one parsed. The
///   record then names that file, because that is where the consumer
///   edits.
fn located(
    err: ForgeError,
    at: Option<&SourceLocation>,
    diag_label: &str,
    model: &SCXMLModel,
) -> Located<ForgeError> {
    let (line, col) = match at {
        Some(loc) => (loc.line, loc.col),
        None => (None, None),
    };
    let positions = model.authored_positions.as_ref();
    let located = match positions.and_then(|p| p.resolve(line, col)) {
        Some((file, row, col)) => Located::new(err, file, Some(row), Some(col)),
        None => Located::new(err, diag_label, line, col),
    };
    match positions.and_then(|p| p.call_site_on(line)) {
        Some((file, row, col)) => located.expanded_from(file, row, col),
        None => located,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HistoryInfo, State, Transition};

    /// Minimal model builder — the pass only reads `states`,
    /// `history_states`, and each state's `initial` / `transitions` /
    /// `parent` / `document_order`.
    fn state(id: &str, order: u32) -> State {
        State {
            id: id.to_string(),
            document_order: order,
            ..Default::default()
        }
    }

    fn model_with(states: Vec<State>) -> SCXMLModel {
        let mut model = SCXMLModel::default();
        for s in states {
            model.states.insert(s.id.clone(), s);
        }
        model
    }

    fn transition_to(target: &str) -> Transition {
        Transition {
            target: target.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_model_is_accepted() {
        // `NoStates` owns the empty-document rejection; this pass must
        // not double-report it.
        let model = SCXMLModel::default();
        assert!(validate(&model, "probe.scxml").is_ok());
    }

    #[test]
    fn resolved_transition_target_is_accepted() {
        let mut a = state("a", 0);
        a.transitions.push(transition_to("b"));
        let model = model_with(vec![a, state("b", 1)]);
        assert!(validate(&model, "probe.scxml").is_ok());
    }

    #[test]
    fn unresolved_transition_target_is_rejected() {
        let mut a = state("a", 0);
        a.transitions.push(transition_to("ghost"));
        let model = model_with(vec![a, state("b", 1)]);
        let err = validate(&model, "probe.scxml").expect_err("must reject");
        match err.error {
            ForgeError::Scxml(boxed) => match *boxed {
                ScxmlSemanticError::TransitionTargetUnknown {
                    state,
                    target,
                    available,
                } => {
                    assert_eq!(state, "a");
                    assert_eq!(target, "ghost");
                    assert_eq!(available, vec!["a".to_string(), "b".to_string()]);
                }
                other => panic!("expected TransitionTargetUnknown, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }

    #[test]
    fn history_pseudostate_is_a_legal_transition_target() {
        // The history id is not in `states`, so a
        // validator that only consulted `states` would reject the
        // legal shape.
        let mut a = state("a", 0);
        a.transitions.push(transition_to("h"));
        let mut model = model_with(vec![a, state("b", 1)]);
        model.history_states.insert(
            "h".to_string(),
            HistoryInfo {
                parent: "b".to_string(),
                history_type: "shallow".to_string(),
                leaf_target: String::new(),
                default_target: "b".to_string(),
                default_actions: Vec::new(),
            },
        );
        assert!(validate(&model, "probe.scxml").is_ok());
    }

    #[test]
    fn unresolved_history_default_target_names_the_history() {
        let mut model = model_with(vec![state("a", 0)]);
        model.history_states.insert(
            "h".to_string(),
            HistoryInfo {
                parent: "a".to_string(),
                history_type: "deep".to_string(),
                leaf_target: String::new(),
                default_target: "ghost".to_string(),
                default_actions: Vec::new(),
            },
        );
        let err = validate(&model, "probe.scxml").expect_err("must reject");
        match err.error {
            ForgeError::Scxml(boxed) => match *boxed {
                ScxmlSemanticError::TransitionTargetUnknown { state, target, .. } => {
                    assert_eq!(state, "h", "the diagnostic must name the <history>");
                    assert_eq!(target, "ghost");
                }
                other => panic!("expected TransitionTargetUnknown, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }

    #[test]
    fn every_multi_target_token_is_checked() {
        // A first-token-only check would accept this.
        let mut a = state("a", 0);
        a.transitions.push(transition_to("b ghost"));
        let model = model_with(vec![a, state("b", 1)]);
        let err = validate(&model, "probe.scxml").expect_err("must reject");
        match err.error {
            ForgeError::Scxml(boxed) => match *boxed {
                ScxmlSemanticError::TransitionTargetUnknown { target, .. } => {
                    assert_eq!(target, "ghost");
                }
                other => panic!("expected TransitionTargetUnknown, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }

    #[test]
    fn targetless_transition_is_not_a_reference() {
        // Executable-only transitions carry no target.
        let mut a = state("a", 0);
        a.transitions.push(transition_to(""));
        a.transitions.push(transition_to("   "));
        let model = model_with(vec![a]);
        assert!(validate(&model, "probe.scxml").is_ok());
    }

    #[test]
    fn compound_initial_candidates_are_scoped_to_children() {
        // The spec restricts the initial configuration to the owning
        // state's descendants, so the repair candidates must
        // not offer unrelated top-level ids.
        let mut outer = state("outer", 0);
        outer.initial = "ghost".to_string();
        let mut child = state("child", 1);
        child.parent = Some("outer".to_string());
        let unrelated = state("elsewhere", 2);
        let model = model_with(vec![outer, child, unrelated]);
        let err = validate(&model, "probe.scxml").expect_err("must reject");
        match err.error {
            ForgeError::Scxml(boxed) => match *boxed {
                ScxmlSemanticError::InitialStateUnknown {
                    state_id,
                    scope,
                    available,
                } => {
                    assert_eq!(state_id, "ghost");
                    assert_eq!(
                        scope,
                        InitialStateScope::CompoundState {
                            parent_id: "outer".to_string()
                        }
                    );
                    assert_eq!(available, vec!["child".to_string()]);
                }
                other => panic!("expected InitialStateUnknown, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }
}
