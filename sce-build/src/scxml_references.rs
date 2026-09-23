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
// Three diagnostic surfaces, all REUSED from the existing wire-code
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
//   * `ScxmlSemanticError::IllegalStateSpecification` — every token
//     resolves and the value, taken whole, is not a legal state
//     specification (§scxml-3.11). Asked after the tokens resolve, of all
//     four positions. Mirrors C++ `SemanticIllegalStateSpecification`,
//     whose judgement is the shared `SCE::Core::checkStateSpecification`.
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
use crate::scxml_semantic::{
    InitialStateScope, ScxmlSemanticError, StateSpecificationBreach, StateSpecificationPosition,
};

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
        // §scxml-3.11: a legal state specification, restricted to the
        // descendants of the state the `<history>` is declared in.
        check_specification(
            model,
            history_id,
            StateSpecificationPosition::HistoryDefault,
            &info.default_target,
            Some(&info.parent),
            model
                .states
                .get(&info.parent)
                .and_then(|s| s.source_location.as_ref()),
            diag_label,
        )?;
    }

    // §scxml-3.11: `<scxml initial>` is a legal state specification. Its
    // tokens were resolved before this pass (`analyzer::can_generate_static`).
    check_specification(
        model,
        "",
        StateSpecificationPosition::DocumentInitial,
        &model.initial,
        None,
        None,
        diag_label,
    )?;

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
        // §scxml-3.11: a legal state specification of this state's own
        // descendants.
        check_specification(
            model,
            &state.id,
            StateSpecificationPosition::StateInitial,
            &state.initial,
            Some(&state.id),
            state.source_location.as_ref(),
            diag_label,
        )?;

        // §scxml-3.5 / §scxml-3.13: every whitespace-separated token of
        // a multi-target attribute resolves independently — and then,
        // §scxml-3.11, the tokens together are a legal state specification.
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
            check_specification(
                model,
                &state.id,
                StateSpecificationPosition::TransitionTarget,
                &trans.target,
                None,
                trans.source_location.as_ref(),
                diag_label,
            )?;
        }
    }

    Ok(())
}

/// True when `id` names a declared state or history pseudostate.
fn resolves(model: &SCXMLModel, id: &str) -> bool {
    model.states.contains_key(id) || model.history_states.contains_key(id)
}

/// The state `id` sits directly under: a state's parent, or the state a
/// `<history>` is declared in. `None` at the top of the document.
fn parent_of<'a>(model: &'a SCXMLModel, id: &str) -> Option<&'a str> {
    match model.states.get(id) {
        Some(state) => state.parent.as_deref(),
        None => model.history_states.get(id).map(|h| h.parent.as_str()),
    }
}

/// `id`'s proper ancestors, nearest first.
fn proper_ancestors<'a>(model: &'a SCXMLModel, id: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut current = parent_of(model, id);
    while let Some(parent) = current {
        out.push(parent);
        current = parent_of(model, parent);
    }
    out
}

/// The first breach of §scxml-3.11 in a resolved list of states, if any.
///
/// Rule 1 and rule 2 are asked of every pair. Rule 2 is the rule stated as
/// "a full legal configuration results when all ancestors and default
/// descendants have been added", read pairwise: adding two states'
/// ancestors adds the state they meet at, and a configuration holding that
/// state holds exactly one of its children unless it is a `<parallel>` —
/// so any meeting point that is not a `<parallel>` (the `<scxml>` element
/// included, whose configuration child is also unique) cannot hold both.
///
/// `container` is the state whose descendants an `initial` value or a
/// `<history>` default is restricted to; `None` for a transition target
/// and for `<scxml initial>`, whose states may lie anywhere.
///
/// A state named twice is not a breach: the list is a set.
fn specification_breach(
    model: &SCXMLModel,
    states: &[&str],
    container: Option<&str>,
) -> Option<StateSpecificationBreach> {
    if let Some(container) = container {
        if let Some(outside) = states
            .iter()
            .find(|s| !proper_ancestors(model, s).contains(&container))
        {
            return Some(StateSpecificationBreach::OutsideContainer {
                state: outside.to_string(),
                container: container.to_string(),
            });
        }
    }
    for (i, first) in states.iter().enumerate() {
        for second in &states[i + 1..] {
            if first == second {
                continue;
            }
            let above_first = proper_ancestors(model, first);
            let above_second = proper_ancestors(model, second);
            if above_second.contains(first) {
                return Some(StateSpecificationBreach::Ancestor {
                    ancestor: first.to_string(),
                    descendant: second.to_string(),
                });
            }
            if above_first.contains(second) {
                return Some(StateSpecificationBreach::Ancestor {
                    ancestor: second.to_string(),
                    descendant: first.to_string(),
                });
            }
            let meet = above_first.iter().find(|a| above_second.contains(a));
            let parallel = meet
                .and_then(|m| model.states.get(*m))
                .is_some_and(|s| s.is_parallel);
            if !parallel {
                return Some(StateSpecificationBreach::NotParallel {
                    first: first.to_string(),
                    second: second.to_string(),
                    meet: meet.map(|m| m.to_string()),
                });
            }
        }
    }
    None
}

/// Refuse `value` at `position` when its resolved states break §scxml-3.11.
fn check_specification(
    model: &SCXMLModel,
    owner: &str,
    position: StateSpecificationPosition,
    value: &str,
    container: Option<&str>,
    at: Option<&SourceLocation>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let states: Vec<&str> = value.split_whitespace().collect();
    match specification_breach(model, &states, container) {
        None => Ok(()),
        Some(breach) => Err(located(
            ScxmlSemanticError::IllegalStateSpecification {
                owner: owner.to_string(),
                position,
                value: value.to_string(),
                breach,
            }
            .into(),
            at,
            diag_label,
            model,
        )),
    }
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
/// A thin argument-order adapter over [`SCXMLModel::locate`], which is
/// where the rule itself lives — the same positioning is now owed by
/// every stage that holds the model and rejects (Item 8 Atomic 3 wired
/// the three design-time lints), and two copies of it would be two
/// answers to which coordinate space a record names.
fn located(
    err: ForgeError,
    at: Option<&SourceLocation>,
    diag_label: &str,
    model: &SCXMLModel,
) -> Located<ForgeError> {
    model.locate(err, at, diag_label)
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
        let mut b1 = state("b1", 2);
        b1.parent = Some("b".to_string());
        let mut model = model_with(vec![a, state("b", 1), b1]);
        model.history_states.insert(
            "h".to_string(),
            HistoryInfo {
                parent: "b".to_string(),
                history_type: "shallow".to_string(),
                leaf_target: String::new(),
                // A history default names descendants of the state it is
                // declared in — `b1`, not `b` itself.
                default_target: "b1".to_string(),
                default_targets: vec!["b1".to_string()],
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
                default_targets: vec!["ghost".to_string()],
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

    // ── A legal state specification ─────────────────────────────────────

    fn under(id: &str, order: u32, parent: &str) -> State {
        let mut s = state(id, order);
        s.parent = Some(parent.to_string());
        s
    }

    /// `top` holds `p`, a `<parallel>` of regions `ra` and `rb`, each with
    /// two leaves; `other` is a second top-level state.
    fn two_regions() -> Vec<State> {
        let mut p = under("p", 1, "top");
        p.is_parallel = true;
        vec![
            state("top", 0),
            p,
            under("ra", 2, "p"),
            under("a1", 3, "ra"),
            under("a2", 4, "ra"),
            under("rb", 5, "p"),
            under("b1", 6, "rb"),
            state("other", 7),
        ]
    }

    fn breach_of(
        model: &SCXMLModel,
    ) -> (StateSpecificationPosition, String, StateSpecificationBreach) {
        let err = validate(model, "probe.scxml").expect_err("must reject");
        match err.error {
            ForgeError::Scxml(boxed) => match *boxed {
                ScxmlSemanticError::IllegalStateSpecification {
                    position,
                    value,
                    breach,
                    ..
                } => (position, value, breach),
                other => panic!("expected IllegalStateSpecification, got {other:?}"),
            },
            other => panic!("expected ForgeError::Scxml, got {other:?}"),
        }
    }

    fn with_transition(mut states: Vec<State>, from: &str, target: &str) -> SCXMLModel {
        states
            .iter_mut()
            .find(|s| s.id == from)
            .expect("source state")
            .transitions
            .push(transition_to(target));
        model_with(states)
    }

    #[test]
    fn a_target_set_across_the_regions_of_a_parallel_is_legal() {
        let model = with_transition(two_regions(), "other", "a1 b1");
        assert!(validate(&model, "probe.scxml").is_ok());
    }

    #[test]
    fn two_children_of_one_compound_state_are_not() {
        let model = with_transition(two_regions(), "other", "a1 a2");
        let (position, value, breach) = breach_of(&model);
        assert_eq!(position, StateSpecificationPosition::TransitionTarget);
        assert_eq!(value, "a1 a2");
        assert_eq!(
            breach,
            StateSpecificationBreach::NotParallel {
                first: "a1".into(),
                second: "a2".into(),
                meet: Some("ra".into()),
            }
        );
    }

    #[test]
    fn two_top_level_states_meet_at_the_document_and_are_not_legal() {
        let model = with_transition(two_regions(), "a1", "top other");
        let (_, _, breach) = breach_of(&model);
        assert_eq!(
            breach,
            StateSpecificationBreach::NotParallel {
                first: "top".into(),
                second: "other".into(),
                meet: None,
            }
        );
    }

    #[test]
    fn a_state_and_its_own_descendant_are_not() {
        let model = with_transition(two_regions(), "other", "b1 p");
        let (_, _, breach) = breach_of(&model);
        assert_eq!(
            breach,
            StateSpecificationBreach::Ancestor {
                ancestor: "p".into(),
                descendant: "b1".into(),
            }
        );
    }

    #[test]
    fn a_state_named_twice_is_a_set_of_one() {
        let model = with_transition(two_regions(), "other", "a1 a1");
        assert!(validate(&model, "probe.scxml").is_ok());
    }

    #[test]
    fn an_initial_names_only_descendants_of_its_state() {
        let mut states = two_regions();
        states.iter_mut().find(|s| s.id == "ra").unwrap().initial = "a1 b1".into();
        let (position, _, breach) = breach_of(&model_with(states));
        assert_eq!(position, StateSpecificationPosition::StateInitial);
        assert_eq!(
            breach,
            StateSpecificationBreach::OutsideContainer {
                state: "b1".into(),
                container: "ra".into(),
            }
        );
    }

    #[test]
    fn a_deep_initial_across_regions_is_legal() {
        let mut states = two_regions();
        states.iter_mut().find(|s| s.id == "top").unwrap().initial = "a2 b1".into();
        assert!(validate(&model_with(states), "probe.scxml").is_ok());
    }

    #[test]
    fn a_history_default_is_held_to_the_same_rule() {
        let mut model = model_with(two_regions());
        model.history_states.insert(
            "h".into(),
            HistoryInfo {
                parent: "ra".into(),
                history_type: "deep".into(),
                default_target: "a1 a2".into(),
                ..Default::default()
            },
        );
        let (position, _, breach) = breach_of(&model);
        assert_eq!(position, StateSpecificationPosition::HistoryDefault);
        assert!(matches!(
            breach,
            StateSpecificationBreach::NotParallel { .. }
        ));
    }
}
