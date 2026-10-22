// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D — what a microstep selects, exits and enters, asked of
// `sce_rust_runtime::helpers::microstep` over a document written out by hand.
//
// Each case states the answer the appendix computes, worked from the
// pseudo-code, and asks the transcription for it. The entry-set and completion
// cases are the ones the C++ engines' shared transcription is held to
// (`tests/states/EntrySetAlgorithmsTest.cpp`,
// `tests/states/CompletionAlgorithmsTest.cpp`), over the same documents, so
// the two transcriptions answer one set of questions. Three answers are the
// reason the entry procedures exist and are pinned by name:
//
//   * a target SET enters every target, and a `<parallel>` region no target
//     descends into still gets its default;
//   * a compound state entered only as an ANCESTOR of a deeper target is not in
//     statesForDefaultEntry, so its initial transition content does not run;
//   * a history that recorded several regions restores all of them.
//
// <scxml initial="s">
//   <state id="s" initial="p">
//     <parallel id="p">
//       <state id="r1" initial="a1"> a1 a2 </state>
//       <state id="r2" initial="b1"> b1 b2
//         <history id="hb" type="shallow"> -> b2 </history> </state>
//       <state id="r3"> c1 c2 </state>                    (no initial: c1)
//     </parallel>
//     <state id="q" initial="q1"> q1 <final id="qf"/> </state>
//     <history id="hs" type="deep"> -> "a2 b2" </history>
//     <state id="m" initial="m1a m2b">
//       <parallel id="mp">
//         <state id="m1"> m1a m1b </state>
//         <state id="m2"> m2a m2b </state>
//       </parallel>
//     </state>
//   </state>
//   <state id="t"/>
// </scxml>

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::helpers::microstep::{
    compute_entry_set, compute_exit_set, compute_states_to_exit, effective_target_states,
    is_descendant, is_in_final_state, microstep, select_transitions, transition_domain, Document,
    EnabledTransition, EntryTarget, EntryTransition, Run, WalkStep,
};

// Declared in document order, so a discriminant IS the document position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    S,
    P,
    R1,
    A1,
    A2,
    R2,
    B1,
    B2,
    R3,
    C1,
    C2,
    Q,
    Q1,
    Qf,
    M,
    Mp,
    M1,
    M1a,
    M1b,
    M2,
    M2a,
    M2b,
    T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum H {
    Hb,
    Hs,
}

type Target = EntryTarget<St, H>;

// The events the transition table below answers; `Null` is the eventless
// selection's "no event".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ev {
    E,
    F,
    G,
    H,
    K,
    R,
    Null,
}

struct Tr {
    event: Ev,
    targets: &'static [Target],
    internal: bool,
    content: bool,
}

const fn tr(event: Ev, targets: &'static [Target]) -> Tr {
    Tr {
        event,
        targets,
        internal: false,
        content: true,
    }
}

// The transitions, per source, in document order:
//
//   a1: E -> a2, F -> a2, H (targetless), K -> a2, R -> a1 (a self-transition)
//   b1: E -> b2, G -> b2, K -> b2, R -> b2
//   p:  F -> t,  G -> t,  H -> t,  K (targetless)
static A1_TRANSITIONS: [Tr; 5] = [
    tr(Ev::E, &[Target::State(St::A2)]),
    tr(Ev::F, &[Target::State(St::A2)]),
    tr(Ev::H, &[]),
    tr(Ev::K, &[Target::State(St::A2)]),
    tr(Ev::R, &[Target::State(St::A1)]),
];
static B1_TRANSITIONS: [Tr; 4] = [
    tr(Ev::E, &[Target::State(St::B2)]),
    tr(Ev::G, &[Target::State(St::B2)]),
    tr(Ev::K, &[Target::State(St::B2)]),
    tr(Ev::R, &[Target::State(St::B2)]),
];
static P_TRANSITIONS: [Tr; 4] = [
    tr(Ev::F, &[Target::State(St::T)]),
    tr(Ev::G, &[Target::State(St::T)]),
    tr(Ev::H, &[Target::State(St::T)]),
    tr(Ev::K, &[]),
];

fn transitions_of(state: St) -> &'static [Tr] {
    match state {
        St::A1 => &A1_TRANSITIONS,
        St::B1 => &B1_TRANSITIONS,
        St::P => &P_TRANSITIONS,
        _ => &[],
    }
}

/// The document above, a configuration, and a record of what the microstep
/// asked the machine to do.
#[derive(Default)]
struct Chart {
    recorded_hb: Option<Vec<St>>,
    recorded_hs: Option<Vec<St>>,
    configuration: Vec<St>,
    log: Vec<String>,
    exits_saw: Vec<Vec<St>>,
}

impl Chart {
    /// The document's initial configuration, entered by hand.
    fn at_initial_configuration() -> Self {
        Chart {
            configuration: vec![St::S, St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1],
            ..Chart::default()
        }
    }
}

impl Document for Chart {
    type State = St;
    type History = H;
    // What the tree above bounds: the largest configuration is `s` with `p` and
    // its three regions (8 states); a selection takes one transition from each
    // atomic state of one (3, under `p`); the deepest state, `m1a`, is at level
    // 5; two parents have a `<history>`.
    type Configuration = BoundedVec<St, 8>;
    type EnabledSet = BoundedVec<EnabledTransition<St, H>, 3>;
    type HistoryDefaults = BoundedVec<(St, H), 2>;
    // The walks' stack: three steps for each of the five levels, one for each of
    // the two histories, and three of headroom.
    type Walk = BoundedVec<WalkStep<St, H>, 20>;
    const MAX_DEPTH: usize = 5;

    fn parent_of(&self, state: St) -> Option<St> {
        match state {
            St::S | St::T => None,
            St::P | St::Q | St::M => Some(St::S),
            St::R1 | St::R2 | St::R3 => Some(St::P),
            St::A1 | St::A2 => Some(St::R1),
            St::B1 | St::B2 => Some(St::R2),
            St::C1 | St::C2 => Some(St::R3),
            St::Q1 | St::Qf => Some(St::Q),
            St::Mp => Some(St::M),
            St::M1 | St::M2 => Some(St::Mp),
            St::M1a | St::M1b => Some(St::M1),
            St::M2a | St::M2b => Some(St::M2),
        }
    }

    fn is_compound(&self, state: St) -> bool {
        matches!(
            state,
            St::S | St::R1 | St::R2 | St::R3 | St::Q | St::M | St::M1 | St::M2
        )
    }

    fn is_parallel(&self, state: St) -> bool {
        matches!(state, St::P | St::Mp)
    }

    fn is_final(&self, state: St) -> bool {
        state == St::Qf
    }

    fn child_states(&self, state: St) -> &'static [St] {
        match state {
            St::S => &[St::P, St::Q, St::M],
            St::P => &[St::R1, St::R2, St::R3],
            St::R1 => &[St::A1, St::A2],
            St::R2 => &[St::B1, St::B2],
            St::R3 => &[St::C1, St::C2],
            St::Q => &[St::Q1, St::Qf],
            St::M => &[St::Mp],
            St::Mp => &[St::M1, St::M2],
            St::M1 => &[St::M1a, St::M1b],
            St::M2 => &[St::M2a, St::M2b],
            _ => &[],
        }
    }

    fn initial_targets(&self, state: St) -> &'static [Target] {
        match state {
            St::S => &[Target::State(St::P)],
            St::R1 => &[Target::State(St::A1)],
            St::R2 => &[Target::State(St::B1)],
            St::R3 => &[Target::State(St::C1)],
            St::Q => &[Target::State(St::Q1)],
            St::M => &[Target::State(St::M1a), Target::State(St::M2b)],
            St::M1 => &[Target::State(St::M1a)],
            St::M2 => &[Target::State(St::M2a)],
            _ => &[],
        }
    }

    fn history_parent(&self, history: H) -> St {
        match history {
            H::Hb => St::R2,
            H::Hs => St::S,
        }
    }

    fn history_value(&self, history: H) -> Option<&[St]> {
        match history {
            H::Hb => self.recorded_hb.as_deref(),
            H::Hs => self.recorded_hs.as_deref(),
        }
    }

    fn history_default_targets(&self, history: H) -> &'static [Target] {
        match history {
            H::Hb => &[Target::State(St::B2)],
            H::Hs => &[Target::State(St::A2), Target::State(St::B2)],
        }
    }

    fn document_order(&self, state: St) -> u32 {
        state as u32
    }
}

impl Run for Chart {
    type Event = Ev;

    fn configuration(&self) -> BoundedVec<St, 8> {
        self.configuration.clone()
    }

    fn first_enabled_transition(
        &mut self,
        state: St,
        event: Ev,
    ) -> Option<EnabledTransition<St, H>> {
        transitions_of(state)
            .iter()
            .enumerate()
            .find(|(_, t)| t.event == event)
            .map(|(index, t)| EnabledTransition {
                source: state,
                targets: t.targets,
                transition_index: index,
                has_actions: t.content,
                is_internal: t.internal,
            })
    }

    fn exit_state(&mut self, state: St, configuration_before_exit: &[St]) {
        self.exits_saw.push(configuration_before_exit.to_vec());
        self.configuration.retain(|&s| s != state);
        self.log.push(format!("exit {state:?}"));
    }

    fn execute_transition_content(&mut self, transition: &EnabledTransition<St, H>) {
        self.log.push(format!(
            "content {:?}#{}",
            transition.source, transition.transition_index
        ));
    }

    fn enter_state(&mut self, state: St, is_default_entry: bool) {
        self.configuration.push(state);
        let how = if is_default_entry { " (default)" } else { "" };
        self.log.push(format!("enter {state:?}{how}"));
    }

    fn execute_history_default_content(&mut self, history: H) {
        self.log.push(format!("history default {history:?}"));
    }
}

fn entering(
    source: Option<St>,
    targets: &'static [Target],
    is_internal: bool,
) -> EntryTransition<St, H> {
    EntryTransition {
        source,
        targets,
        is_internal,
    }
}

fn sorted(mut states: Vec<St>) -> Vec<St> {
    states.sort_by_key(|&s| s as u32);
    states
}

fn chain(states: &[St]) -> Vec<St> {
    states.to_vec()
}

fn sources(transitions: &[EnabledTransition<St, H>]) -> Vec<(St, usize)> {
    transitions
        .iter()
        .map(|t| (t.source, t.transition_index))
        .collect()
}

// ════════════════════════════════════════════════════════════════════════
// computeEntrySet
// ════════════════════════════════════════════════════════════════════════

#[test]
fn the_initial_configuration_enters_every_region_by_default() {
    let doc = Chart::default();
    let entry = compute_entry_set(&doc, &[entering(None, &[Target::State(St::S)], false)]);

    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::S, St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1]
    );
    // A `<parallel>` is not compound, so it has no initial transition to run.
    assert_eq!(
        sorted(chain(&entry.states_for_default_entry)),
        vec![St::S, St::R1, St::R2, St::R3]
    );
    assert!(entry.default_history_content.is_empty());
}

#[test]
fn a_target_set_enters_every_target_and_defaults_the_region_no_target_reaches() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(
            Some(St::T),
            &[Target::State(St::A2), Target::State(St::B2)],
            false,
        )],
    );

    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::S, St::P, St::R1, St::A2, St::R2, St::B2, St::R3, St::C1]
    );
    // `s`, `r1` and `r2` are entered as ANCESTORS of a named target, so their
    // initial transitions do not run; only `r3`, reached by nobody, defaults.
    assert_eq!(chain(&entry.states_for_default_entry), vec![St::R3]);
    assert!(!entry.is_default_entry(St::S));
    assert!(entry.is_default_entry(St::R3));
}

#[test]
fn an_unrecorded_shallow_history_takes_its_default_and_owes_its_content_to_its_parent() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::A1), &[Target::History(H::Hb)], false)],
    );

    // The domain is `s`: the history stands for `b2`, and the least compound
    // ancestor of `a1` and `b2` walks past the `<parallel>`.
    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::P, St::R1, St::A1, St::R2, St::B2, St::R3, St::C1]
    );
    assert_eq!(
        sorted(chain(&entry.states_for_default_entry)),
        vec![St::R1, St::R3]
    );
    assert_eq!(entry.default_history_content_of(St::R2), Some(H::Hb));
    assert_eq!(entry.default_history_content_of(St::P), None);
}

#[test]
fn a_recorded_shallow_history_restores_what_it_recorded_and_owes_no_content() {
    let doc = Chart {
        recorded_hb: Some(vec![St::B1]),
        ..Chart::default()
    };
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::A1), &[Target::History(H::Hb)], false)],
    );

    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1]
    );
    assert!(entry.default_history_content.is_empty());
}

#[test]
fn a_deep_history_that_recorded_every_region_restores_all_of_them() {
    let doc = Chart {
        recorded_hs: Some(vec![St::A2, St::B1, St::C2]),
        ..Chart::default()
    };
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::T), &[Target::History(H::Hs)], false)],
    );

    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::S, St::P, St::R1, St::A2, St::R2, St::B1, St::R3, St::C2]
    );
    // Nothing is entered by default: every compound state on the way is an
    // ancestor of a recorded state.
    assert!(entry.states_for_default_entry.is_empty());
    assert!(entry.default_history_content.is_empty());
}

#[test]
fn an_unrecorded_deep_history_with_a_multi_state_default_enters_the_whole_set() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::T), &[Target::History(H::Hs)], false)],
    );

    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::S, St::P, St::R1, St::A2, St::R2, St::B2, St::R3, St::C1]
    );
    assert_eq!(chain(&entry.states_for_default_entry), vec![St::R3]);
    assert_eq!(entry.default_history_content_of(St::S), Some(H::Hs));
}

#[test]
fn an_internal_transition_to_a_descendant_leaves_its_source_out_of_the_set() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::S), &[Target::State(St::Q1)], true)],
    );

    assert_eq!(chain(&entry.states_to_enter), vec![St::Q, St::Q1]);
    assert!(entry.states_for_default_entry.is_empty());
}

#[test]
fn an_external_transition_to_a_descendant_reenters_its_source() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::S), &[Target::State(St::Q1)], false)],
    );

    assert_eq!(chain(&entry.states_to_enter), vec![St::S, St::Q, St::Q1]);
    assert!(entry.states_for_default_entry.is_empty());
}

#[test]
fn an_external_transition_on_a_region_root_reenters_every_sibling_region() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::R1), &[Target::State(St::A2)], false)],
    );

    // A `<parallel>` is never a domain, so the domain is `s`, and the sibling
    // regions are exited and entered again at their defaults.
    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::P, St::R1, St::A2, St::R2, St::B1, St::R3, St::C1]
    );
    assert_eq!(
        sorted(chain(&entry.states_for_default_entry)),
        vec![St::R2, St::R3]
    );
}

#[test]
fn an_internal_transition_on_a_region_root_stays_inside_its_region() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::R1), &[Target::State(St::A2)], true)],
    );

    assert_eq!(chain(&entry.states_to_enter), vec![St::A2]);
}

#[test]
fn transitions_of_one_microstep_enter_in_document_order() {
    let doc = Chart::default();
    // Selected second-region first, as a set is free to be; entry order is
    // document order all the same.
    let entry = compute_entry_set(
        &doc,
        &[
            entering(Some(St::B1), &[Target::State(St::B2)], false),
            entering(Some(St::A1), &[Target::State(St::A2)], false),
        ],
    );

    assert_eq!(chain(&entry.states_to_enter), vec![St::A2, St::B2]);
}

#[test]
fn a_deep_multi_target_initial_defaults_only_the_state_that_names_it() {
    let doc = Chart::default();
    let entry = compute_entry_set(
        &doc,
        &[entering(Some(St::T), &[Target::State(St::M)], false)],
    );

    assert_eq!(
        chain(&entry.states_to_enter),
        vec![St::S, St::M, St::Mp, St::M1, St::M1a, St::M2, St::M2b]
    );
    // `m` is the target and defaults; `m1` and `m2` are ancestors of its
    // initial targets, and `s` an ancestor of the target itself.
    assert_eq!(chain(&entry.states_for_default_entry), vec![St::M]);
}

#[test]
fn a_targetless_transition_enters_nothing() {
    let doc = Chart::default();
    let entry = compute_entry_set(&doc, &[entering(Some(St::A1), &[], false)]);

    assert!(entry.states_to_enter.is_empty());
    assert!(entry.states_for_default_entry.is_empty());
}

#[test]
fn the_domain_is_asked_of_the_states_a_history_stands_for() {
    let mut doc = Chart::default();
    // Unrecorded, `hs` stands for "a2 b2", whose least compound ancestor with
    // `t` is the document itself.
    let unrecorded = effective_target_states(&doc, &[Target::History(H::Hs)]);
    assert_eq!(chain(&unrecorded), vec![St::A2, St::B2]);
    assert_eq!(
        transition_domain(&doc, Some(St::T), &unrecorded, false),
        None
    );

    // Recorded inside `q`, an internal transition on `q` keeps `q` as its
    // domain.
    doc.recorded_hs = Some(vec![St::Q1]);
    let recorded = effective_target_states(&doc, &[Target::History(H::Hs)]);
    assert_eq!(chain(&recorded), vec![St::Q1]);
    assert_eq!(
        transition_domain(&doc, Some(St::Q), &recorded, true),
        Some(St::Q)
    );
}

// ════════════════════════════════════════════════════════════════════════
// isDescendant
// ════════════════════════════════════════════════════════════════════════

#[test]
fn a_child_and_a_grandchild_are_descendants() {
    let doc = Chart::default();
    assert!(is_descendant(&doc, St::A1, St::R1));
    assert!(is_descendant(&doc, St::A1, St::P));
    assert!(is_descendant(&doc, St::M2b, St::S));
}

#[test]
fn a_state_is_not_its_own_descendant() {
    // Appendix D's isDescendant is strict.
    let doc = Chart::default();
    assert!(!is_descendant(&doc, St::R1, St::R1));
}

#[test]
fn states_on_unrelated_branches_are_not_descendants() {
    let doc = Chart::default();
    assert!(!is_descendant(&doc, St::A1, St::R2));
    assert!(!is_descendant(&doc, St::Q1, St::P));
    assert!(!is_descendant(&doc, St::T, St::S));
}

// ════════════════════════════════════════════════════════════════════════
// Domains and exit sets
// ════════════════════════════════════════════════════════════════════════

fn enabled(source: St, index: usize) -> EnabledTransition<St, H> {
    let t = &transitions_of(source)[index];
    EnabledTransition {
        source,
        targets: t.targets,
        transition_index: index,
        has_actions: t.content,
        is_internal: t.internal,
    }
}

#[test]
fn a_self_transition_exits_only_its_source() {
    let chart = Chart::at_initial_configuration();
    // §scxml-D-findLCCA chooses among the PROPER ancestors, so the domain of
    // `a1 -> a1` is `r1`, and the exit set is `a1` alone. A source that were
    // its own domain would exit nothing, and the self-transition would not
    // leave and re-enter its state at all.
    let self_transition = enabled(St::A1, 4);
    assert_eq!(
        chain(&compute_exit_set(
            &chart,
            &self_transition,
            &chart.configuration
        )),
        vec![St::A1]
    );
}

#[test]
fn an_external_transition_leaving_a_parallel_exits_every_region() {
    let chart = Chart::at_initial_configuration();
    // `p -> t`: the domain is the `<scxml>` element, so the whole
    // configuration goes — the sibling regions of `p` included, which no walk
    // up from the source alone could name.
    let leaving = enabled(St::P, 0);
    assert_eq!(
        sorted(chain(&compute_exit_set(
            &chart,
            &leaving,
            &chart.configuration
        ))),
        sorted(chart.configuration.clone())
    );
}

#[test]
fn an_internal_transition_from_a_region_root_exits_only_inside_the_region() {
    let chart = Chart::at_initial_configuration();
    static TO_A2: [Target; 1] = [Target::State(St::A2)];
    let internal = EnabledTransition {
        source: St::R1,
        targets: &TO_A2,
        transition_index: 0,
        has_actions: false,
        is_internal: true,
    };
    let external = EnabledTransition {
        is_internal: false,
        ..internal
    };
    assert_eq!(
        chain(&compute_exit_set(&chart, &internal, &chart.configuration)),
        vec![St::A1]
    );
    // Written external, the same transition's domain is `s` — a `<parallel>`
    // is never a domain — so every region goes with it.
    assert_eq!(
        sorted(chain(&compute_exit_set(
            &chart,
            &external,
            &chart.configuration
        ))),
        vec![St::P, St::R1, St::A1, St::R2, St::B1, St::R3, St::C1]
    );
}

#[test]
fn a_targetless_transition_exits_nothing() {
    let chart = Chart::at_initial_configuration();
    assert!(compute_exit_set(&chart, &enabled(St::A1, 2), &chart.configuration).is_empty());
}

#[test]
fn the_states_to_exit_come_out_in_reverse_document_order() {
    let chart = Chart::at_initial_configuration();
    let exited = compute_states_to_exit(&chart, &[enabled(St::P, 0)], &chart.configuration);
    assert_eq!(
        chain(&exited),
        vec![St::C1, St::R3, St::B1, St::R2, St::A1, St::R1, St::P, St::S]
    );
}

// ════════════════════════════════════════════════════════════════════════
// selectTransitions / removeConflictingTransitions
// ════════════════════════════════════════════════════════════════════════

#[test]
fn every_region_takes_its_own_transition() {
    let mut chart = Chart::at_initial_configuration();
    // §scxml-3.4: two regions' transitions have domains in disjoint subtrees,
    // so their exit sets are disjoint and both survive.
    assert_eq!(
        sources(&select_transitions(&mut chart, Ev::E)),
        vec![(St::A1, 0), (St::B1, 0)]
    );
}

#[test]
fn a_transition_selected_first_preempts_a_later_one_that_is_not_its_descendant() {
    let mut chart = Chart::at_initial_configuration();
    // `a1` selects `a1 -> a2`; `b1` walks up to `p -> t`, which exits `a1`
    // too, and `p` does not descend from `a1`: it is preempted.
    assert_eq!(
        sources(&select_transitions(&mut chart, Ev::F)),
        vec![(St::A1, 1)]
    );
}

#[test]
fn a_descendant_source_preempts_an_ancestor_selected_before_it() {
    let mut chart = Chart::at_initial_configuration();
    // `a1` walks up to `p -> t` first; `b1` then selects `b1 -> b2`, which
    // conflicts with it and descends from `p`, so it wins. `c1` reaches
    // `p -> t` again — one transition, already in the set.
    assert_eq!(
        sources(&select_transitions(&mut chart, Ev::G)),
        vec![(St::B1, 1)]
    );
}

#[test]
fn a_targetless_transition_is_never_preempted() {
    let mut chart = Chart::at_initial_configuration();
    // W3C test 403c: an empty exit set conflicts with nothing, so `a1`'s
    // targetless transition survives the `p -> t` that exits `a1` itself.
    assert_eq!(
        sources(&select_transitions(&mut chart, Ev::H)),
        vec![(St::A1, 2), (St::P, 2)]
    );
}

#[test]
fn a_transition_reached_from_two_regions_is_selected_once() {
    let mut chart = Chart::at_initial_configuration();
    // In the initial configuration only `c1` walks up to `p`'s targetless K —
    // `a1` and `b1` answer K themselves. Move the first two regions to `a2`
    // and `b2`, which answer nothing, and all three atomic states reach it: it
    // is still one element of the set.
    let taken = select_transitions(&mut chart, Ev::K);
    assert_eq!(sources(&taken), vec![(St::A1, 3), (St::B1, 2), (St::P, 3)]);
    chart.configuration = vec![St::S, St::P, St::R1, St::A2, St::R2, St::B2, St::R3, St::C1];
    assert_eq!(
        sources(&select_transitions(&mut chart, Ev::K)),
        vec![(St::P, 3)]
    );
}

#[test]
fn an_eventless_selection_with_nothing_enabled_selects_nothing() {
    let mut chart = Chart::at_initial_configuration();
    assert!(select_transitions(&mut chart, Ev::Null).is_empty());
}

// ════════════════════════════════════════════════════════════════════════
// The microstep
// ════════════════════════════════════════════════════════════════════════

#[test]
fn a_microstep_exits_then_runs_content_in_selection_order_then_enters() {
    let mut chart = Chart::at_initial_configuration();
    let taken = select_transitions(&mut chart, Ev::K);
    microstep(&mut chart, &taken);

    // §scxml-D-executeTransitionContent runs content in SELECTION order:
    // `p`'s K was reached last, from `c1`, though `p` comes first in document
    // order.
    assert_eq!(
        chart.log,
        vec![
            "exit B1",
            "exit A1",
            "content A1#3",
            "content B1#2",
            "content P#3",
            "enter A2",
            "enter B2",
        ]
    );
}

#[test]
fn every_exit_is_handed_the_configuration_before_the_first_exit() {
    let mut chart = Chart::at_initial_configuration();
    let before = chart.configuration.clone();
    let taken = select_transitions(&mut chart, Ev::F);
    microstep(&mut chart, &taken);

    assert_eq!(chart.exits_saw, vec![before]);
}

#[test]
fn a_self_transition_leaves_and_reenters_its_state() {
    let mut chart = Chart::at_initial_configuration();
    let taken = select_transitions(&mut chart, Ev::R);
    assert_eq!(sources(&taken), vec![(St::A1, 4), (St::B1, 3)]);
    microstep(&mut chart, &taken);

    assert_eq!(
        chart.log,
        vec![
            "exit B1",
            "exit A1",
            "content A1#4",
            "content B1#3",
            "enter A1",
            "enter B2",
        ]
    );
}

#[test]
fn a_history_default_content_runs_after_its_parent_is_entered() {
    let mut chart = Chart::at_initial_configuration();
    static TO_HB: [Target; 1] = [Target::History(H::Hb)];
    let to_history = EnabledTransition {
        source: St::A1,
        targets: &TO_HB,
        transition_index: 0,
        has_actions: false,
        is_internal: false,
    };
    microstep(&mut chart, &[to_history]);

    // The domain is `s`, so all of `p` leaves and comes back; `r2` is an
    // ancestor of the default target `b2`, not entered by default, and the
    // history's default content runs once `r2` has been entered.
    assert_eq!(
        chart.log,
        vec![
            "exit C1",
            "exit R3",
            "exit B1",
            "exit R2",
            "exit A1",
            "exit R1",
            "exit P",
            "enter P",
            "enter R1 (default)",
            "enter A1",
            "enter R2",
            "history default Hb",
            "enter B2",
            "enter R3 (default)",
            "enter C1",
        ]
    );
}

// ════════════════════════════════════════════════════════════════════════
// isInFinalState
// ════════════════════════════════════════════════════════════════════════
//
// run (parallel)
//   form > filling, formDone (final)
//   checks (parallel)
//     left  > leftPending,  leftDone (final)
//     right > rightPending, rightDone (final)
//
// `checks` is a `<parallel>` among the regions of a `<parallel>`: it has no
// `<final>` child of its own and is final only because both of its regions are.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum C {
    Run,
    Form,
    Filling,
    FormDone,
    Checks,
    Left,
    LeftPending,
    LeftDone,
    Right,
    RightPending,
    RightDone,
}

struct Completion;

impl Document for Completion {
    type State = C;
    type History = sce_rust_runtime::helpers::microstep::NoHistory;
    // `run` with `form` and a state of it, and `checks` with both its regions
    // and a state of each: 8 states, 3 of them atomic, 4 levels down.
    type Configuration = BoundedVec<C, 8>;
    type EnabledSet = BoundedVec<EnabledTransition<C, Self::History>, 3>;
    type HistoryDefaults = BoundedVec<(C, Self::History), 0>;
    // Three steps for each of the four levels, and three of headroom.
    type Walk = BoundedVec<WalkStep<C, Self::History>, 15>;
    const MAX_DEPTH: usize = 4;

    fn parent_of(&self, state: C) -> Option<C> {
        match state {
            C::Run => None,
            C::Form | C::Checks => Some(C::Run),
            C::Filling | C::FormDone => Some(C::Form),
            C::Left | C::Right => Some(C::Checks),
            C::LeftPending | C::LeftDone => Some(C::Left),
            C::RightPending | C::RightDone => Some(C::Right),
        }
    }

    fn is_compound(&self, state: C) -> bool {
        matches!(state, C::Form | C::Left | C::Right)
    }

    fn is_parallel(&self, state: C) -> bool {
        matches!(state, C::Run | C::Checks)
    }

    fn is_final(&self, state: C) -> bool {
        matches!(state, C::FormDone | C::LeftDone | C::RightDone)
    }

    fn child_states(&self, state: C) -> &'static [C] {
        match state {
            C::Run => &[C::Form, C::Checks],
            C::Form => &[C::Filling, C::FormDone],
            C::Checks => &[C::Left, C::Right],
            C::Left => &[C::LeftPending, C::LeftDone],
            C::Right => &[C::RightPending, C::RightDone],
            _ => &[],
        }
    }

    fn initial_targets(&self, _state: C) -> &'static [EntryTarget<C, Self::History>] {
        &[]
    }

    fn history_parent(&self, history: Self::History) -> C {
        match history {}
    }

    fn history_value(&self, history: Self::History) -> Option<&[C]> {
        match history {}
    }

    fn history_default_targets(
        &self,
        history: Self::History,
    ) -> &'static [EntryTarget<C, Self::History>] {
        match history {}
    }

    fn document_order(&self, state: C) -> u32 {
        state as u32
    }
}

#[test]
fn a_compound_state_is_final_when_a_final_child_is_active() {
    let doc = Completion;
    assert!(is_in_final_state(
        &doc,
        C::Form,
        &[
            C::Run,
            C::Form,
            C::FormDone,
            C::Checks,
            C::Left,
            C::LeftPending,
            C::Right,
            C::RightPending
        ]
    ));
    assert!(!is_in_final_state(
        &doc,
        C::Form,
        &[
            C::Run,
            C::Form,
            C::Filling,
            C::Checks,
            C::Left,
            C::LeftPending,
            C::Right,
            C::RightPending
        ]
    ));
}

#[test]
fn a_nested_parallel_is_final_only_when_every_region_is() {
    let doc = Completion;
    assert!(is_in_final_state(
        &doc,
        C::Checks,
        &[C::Checks, C::Left, C::LeftDone, C::Right, C::RightDone]
    ));
    assert!(
        !is_in_final_state(
            &doc,
            C::Checks,
            &[C::Checks, C::Left, C::LeftDone, C::Right, C::RightPending]
        ),
        "one region still pending leaves the <parallel> short of final"
    );
}

#[test]
fn a_parallel_region_of_a_parallel_counts() {
    let doc = Completion;
    assert!(
        is_in_final_state(
            &doc,
            C::Run,
            &[
                C::Run,
                C::Form,
                C::FormDone,
                C::Checks,
                C::Left,
                C::LeftDone,
                C::Right,
                C::RightDone
            ]
        ),
        "`checks` has no <final> child of its own; it is final because both of its regions are"
    );
    assert!(!is_in_final_state(
        &doc,
        C::Run,
        &[
            C::Run,
            C::Form,
            C::FormDone,
            C::Checks,
            C::Left,
            C::LeftDone,
            C::Right,
            C::RightPending
        ]
    ));
}

#[test]
fn neither_an_atomic_state_nor_a_final_element_is_in_a_final_state() {
    let doc = Completion;
    let configuration = [
        C::Run,
        C::Form,
        C::FormDone,
        C::Checks,
        C::Left,
        C::LeftPending,
        C::Right,
        C::RightPending,
    ];
    assert!(
        !is_in_final_state(&doc, C::LeftPending, &configuration),
        "an atomic state has no <final> child to be active"
    );
    assert!(
        !is_in_final_state(&doc, C::FormDone, &configuration),
        "being a <final> element is not being IN a final state — the predicate asks about children"
    );
}

// ════════════════════════════════════════════════════════════════════════
// The stack walks, held to the recursion they replace
// ════════════════════════════════════════════════════════════════════════
//
// Appendix D writes the entry set, the effective targets and the completion check
// as recursions, and the runtime runs them over a stack of steps so that its
// worst-case stack assumes no depth. The two must answer alike for EVERY
// document, not only the ones written out above, and what the recursion answers
// depends on the ORDER it visits things in: a `<parallel>` region is judged
// against what the entry set holds when it is reached, and a later `<history>` of
// a parent replaces an earlier one's default content. So the reference below is
// the recursion itself, as the transcription first wrote it, and random documents
// ask both. The same documents show the stack never needs more steps than the
// capacity the generator gives it (`sce_build::microstep_bounds`).

use sce_rust_runtime::helpers::hierarchy::Bounded;
use sce_rust_runtime::BoundedPush;

/// Appendix D's recursions, as the transcription first wrote them.
mod recursion {
    use std::fmt::Debug;

    use sce_rust_runtime::helpers::hierarchy::Bounded;
    use sce_rust_runtime::helpers::microstep::{
        is_descendant, transition_domain, Document, EntrySet, EntrySetOf, EntryTarget,
        EntryTransition,
    };
    use sce_rust_runtime::BoundedPush;

    fn add_once<S: Copy + PartialEq + Debug, B: Bounded<S>>(set: &mut B, state: S) {
        if !set.contains(&state) {
            set.push_bounded(state);
        }
    }

    pub fn effective_target_states<D: Document>(
        doc: &D,
        targets: &[EntryTarget<D::State, D::History>],
    ) -> D::Configuration {
        let mut effective = D::Configuration::default();
        add_effective_target_states(doc, targets, &mut effective);
        effective
    }

    fn add_effective_target_states<D: Document>(
        doc: &D,
        targets: &[EntryTarget<D::State, D::History>],
        effective: &mut D::Configuration,
    ) {
        for &target in targets {
            match target {
                EntryTarget::State(state) => add_once(effective, state),
                EntryTarget::History(history) => match doc.history_value(history) {
                    Some(recorded) if !recorded.is_empty() => {
                        for &state in recorded {
                            add_once(effective, state);
                        }
                    }
                    _ => add_effective_target_states(
                        doc,
                        doc.history_default_targets(history),
                        effective,
                    ),
                },
            }
        }
    }

    pub fn compute_entry_set<D: Document>(
        doc: &D,
        transitions: &[EntryTransition<D::State, D::History>],
    ) -> EntrySetOf<D> {
        let mut entry = EntrySet {
            states_to_enter: D::Configuration::default(),
            states_for_default_entry: D::Configuration::default(),
            default_history_content: D::HistoryDefaults::default(),
        };
        for transition in transitions {
            for &target in transition.targets {
                add_descendant_states_to_enter(doc, target, &mut entry);
            }
            let effective = effective_target_states(doc, transition.targets);
            if effective.is_empty() {
                continue;
            }
            let domain =
                transition_domain(doc, transition.source, &effective, transition.is_internal);
            for &state in effective.iter() {
                add_ancestor_states_to_enter(doc, EntryTarget::State(state), domain, &mut entry);
            }
        }
        entry
            .states_to_enter
            .sort_by_key(|&state| doc.document_order(state));
        entry
    }

    fn add_descendant_states_to_enter<D: Document>(
        doc: &D,
        target: EntryTarget<D::State, D::History>,
        entry: &mut EntrySetOf<D>,
    ) {
        let state = match target {
            EntryTarget::State(state) => state,
            EntryTarget::History(history) => {
                let parent = doc.history_parent(history);
                match doc.history_value(history) {
                    Some(recorded) if !recorded.is_empty() => {
                        for &state in recorded {
                            add_descendant_states_to_enter(doc, EntryTarget::State(state), entry);
                        }
                        for &state in recorded {
                            add_ancestor_states_to_enter(
                                doc,
                                EntryTarget::State(state),
                                Some(parent),
                                entry,
                            );
                        }
                    }
                    _ => {
                        match entry
                            .default_history_content
                            .iter_mut()
                            .find(|slot| slot.0 == parent)
                        {
                            Some(slot) => slot.1 = history,
                            None => entry
                                .default_history_content
                                .push_bounded((parent, history)),
                        }
                        let defaults = doc.history_default_targets(history);
                        for &default in defaults {
                            add_descendant_states_to_enter(doc, default, entry);
                        }
                        for &default in defaults {
                            add_ancestor_states_to_enter(doc, default, Some(parent), entry);
                        }
                    }
                }
                return;
            }
        };

        add_once(&mut entry.states_to_enter, state);
        if doc.is_compound(state) {
            add_once(&mut entry.states_for_default_entry, state);
            let initial = doc.initial_targets(state);
            for &child in initial {
                add_descendant_states_to_enter(doc, child, entry);
            }
            for &child in initial {
                add_ancestor_states_to_enter(doc, child, Some(state), entry);
            }
        } else if doc.is_parallel(state) {
            add_region_defaults(doc, state, entry);
        }
    }

    fn add_ancestor_states_to_enter<D: Document>(
        doc: &D,
        target: EntryTarget<D::State, D::History>,
        ancestor: Option<D::State>,
        entry: &mut EntrySetOf<D>,
    ) {
        for &anc in proper_ancestors(doc, target, ancestor).iter() {
            add_once(&mut entry.states_to_enter, anc);
            if doc.is_parallel(anc) {
                add_region_defaults(doc, anc, entry);
            }
        }
    }

    fn proper_ancestors<D: Document>(
        doc: &D,
        target: EntryTarget<D::State, D::History>,
        ancestor: Option<D::State>,
    ) -> D::Configuration {
        let mut chain = D::Configuration::default();
        let mut current = match target {
            EntryTarget::State(state) => doc.parent_of(state),
            EntryTarget::History(history) => Some(doc.history_parent(history)),
        };
        for _ in 0..D::MAX_DEPTH {
            let Some(state) = current else {
                if ancestor.is_some() {
                    chain.clear();
                }
                return chain;
            };
            if ancestor == Some(state) {
                return chain;
            }
            chain.push_bounded(state);
            current = doc.parent_of(state);
        }
        panic!("cyclic parent relationship walking from {target:?}");
    }

    fn add_region_defaults<D: Document>(doc: &D, parallel: D::State, entry: &mut EntrySetOf<D>) {
        for &child in doc.child_states(parallel) {
            let taken = entry
                .states_to_enter
                .iter()
                .any(|&state| is_descendant(doc, state, child));
            if !taken {
                add_descendant_states_to_enter(doc, EntryTarget::State(child), entry);
            }
        }
    }

    pub fn is_in_final_state<D: Document>(
        doc: &D,
        state: D::State,
        configuration: &[D::State],
    ) -> bool {
        if doc.is_compound(state) {
            doc.child_states(state)
                .iter()
                .any(|&child| doc.is_final(child) && configuration.contains(&child))
        } else if doc.is_parallel(state) {
            doc.child_states(state)
                .iter()
                .all(|&child| is_in_final_state(doc, child, configuration))
        } else {
            false
        }
    }
}

/// A small xorshift generator: the documents are a function of the seed alone, so
/// a failure names the seed that reproduces it.
struct Rng(u64);

impl Rng {
    fn draw(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.draw() % n as u64) as usize
    }

    fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rs(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rh(u8);

type RTarget = EntryTarget<Rs, Rh>;
type RStep = WalkStep<Rs, Rh>;

thread_local! {
    /// The most steps any walk of the document being asked has had on its stack.
    static HIGH_WATER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The walks' stack for a random document: a `Vec` that says how tall it has been.
#[derive(Debug, Clone, Default)]
struct Watched(Vec<RStep>);

impl BoundedPush<RStep> for Watched {
    fn push_bounded(&mut self, item: RStep) {
        self.0.push(item);
        HIGH_WATER.with(|high| high.set(high.get().max(self.0.len())));
    }
}

impl std::ops::Deref for Watched {
    type Target = [RStep];
    fn deref(&self) -> &[RStep] {
        &self.0
    }
}

impl std::ops::DerefMut for Watched {
    fn deref_mut(&mut self) -> &mut [RStep] {
        &mut self.0
    }
}

impl IntoIterator for Watched {
    type Item = RStep;
    type IntoIter = std::vec::IntoIter<RStep>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Bounded<RStep> for Watched {
    fn clear(&mut self) {
        self.0.clear();
    }
    fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }
    fn retain<F: FnMut(&RStep) -> bool>(&mut self, keep: F) {
        self.0.retain(keep);
    }
}

/// A document of up to twelve states drawn at random: a forest, each state with
/// children a `<parallel>` or a compound state, initial targets anywhere below a
/// compound state, and up to three `<history>` elements that recorded something,
/// recorded nothing or recorded an empty set, and whose default transitions may
/// name a later `<history>` below their parent.
struct Random {
    parent: Vec<Option<usize>>,
    kids: Vec<&'static [Rs]>,
    parallel: Vec<bool>,
    compound: Vec<bool>,
    is_final: Vec<bool>,
    initial: Vec<&'static [RTarget]>,
    history_parent: Vec<usize>,
    history_recorded: Vec<Option<Vec<Rs>>>,
    history_defaults: Vec<&'static [RTarget]>,
    /// Levels, a `<history>` counting as one below its parent.
    depth: usize,
}

/// Lent for the life of the test process: the [`Document`] hands out
/// `&'static` slices, as a generated machine's tables are.
fn leak<T>(items: Vec<T>) -> &'static [T] {
    Box::leak(items.into_boxed_slice())
}

fn random_document(rng: &mut Rng) -> Random {
    let n = 1 + rng.below(12);
    let mut parent: Vec<Option<usize>> = Vec::new();
    for i in 0..n {
        parent.push(if i == 0 || rng.chance(4) {
            None
        } else {
            Some(rng.below(i))
        });
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, p) in parent.iter().enumerate() {
        if let Some(p) = p {
            children[*p].push(i);
        }
    }
    let parallel: Vec<bool> = children
        .iter()
        .map(|c| !c.is_empty() && rng.chance(3))
        .collect();
    let compound: Vec<bool> = (0..n)
        .map(|i| !children[i].is_empty() && !parallel[i])
        .collect();
    let is_final: Vec<bool> = children
        .iter()
        .map(|c| c.is_empty() && rng.chance(3))
        .collect();
    let below = |state: usize| -> Vec<usize> {
        let mut found = Vec::new();
        let mut frontier = children[state].clone();
        while let Some(next) = frontier.pop() {
            found.push(next);
            frontier.extend(children[next].iter().copied());
        }
        found
    };
    let pick = |rng: &mut Rng, from: &[usize]| from[rng.below(from.len())];

    let initial: Vec<&'static [RTarget]> = (0..n)
        .map(|i| {
            if !compound[i] {
                return leak(Vec::new());
            }
            let inside = below(i);
            let count = 1 + rng.below(2);
            leak(
                (0..count)
                    .map(|_| RTarget::State(Rs(pick(rng, &inside) as u8)))
                    .collect(),
            )
        })
        .collect();

    let with_children: Vec<usize> = (0..n).filter(|&i| !children[i].is_empty()).collect();
    let histories = if with_children.is_empty() {
        0
    } else {
        rng.below(4)
    };
    let history_parent: Vec<usize> = (0..histories).map(|_| pick(rng, &with_children)).collect();
    let mut history_recorded = Vec::new();
    let mut history_defaults: Vec<&'static [RTarget]> = Vec::new();
    for h in 0..histories {
        let inside = below(history_parent[h]);
        history_recorded.push(match rng.below(3) {
            0 => None,
            1 => Some(Vec::new()),
            _ => Some(
                (0..1 + rng.below(3))
                    .map(|_| Rs(pick(rng, &inside) as u8))
                    .collect(),
            ),
        });
        // A default transition may name a later history whose parent is this
        // one's or lies below it: a later one only, so the names never loop.
        let named: Vec<usize> = (h + 1..histories)
            .filter(|&j| {
                history_parent[j] == history_parent[h] || inside.contains(&history_parent[j])
            })
            .collect();
        let count = 1 + rng.below(2);
        history_defaults.push(leak(
            (0..count)
                .map(|_| {
                    if !named.is_empty() && rng.chance(4) {
                        RTarget::History(Rh(pick(rng, &named) as u8))
                    } else {
                        RTarget::State(Rs(pick(rng, &inside) as u8))
                    }
                })
                .collect(),
        ));
    }

    let level = |mut state: usize| {
        let mut levels = 1;
        while let Some(up) = parent[state] {
            levels += 1;
            state = up;
        }
        levels
    };
    let mut depth = (0..n).map(level).max().unwrap_or(1);
    for &p in &history_parent {
        depth = depth.max(level(p) + 1);
    }

    Random {
        kids: children
            .iter()
            .map(|c| leak(c.iter().map(|&i| Rs(i as u8)).collect()))
            .collect(),
        parent,
        parallel,
        compound,
        is_final,
        initial,
        history_parent,
        history_recorded,
        history_defaults,
        depth,
    }
}

impl Random {
    fn histories(&self) -> usize {
        self.history_parent.len()
    }

    /// The capacity the generator gives the walks' stack for this document
    /// (`sce_build::microstep_bounds`).
    fn walk_bound(&self) -> usize {
        3 * self.depth + self.histories() + 3
    }
}

impl Document for Random {
    type State = Rs;
    type History = Rh;
    type Configuration = Vec<Rs>;
    type EnabledSet = Vec<EnabledTransition<Rs, Rh>>;
    type HistoryDefaults = Vec<(Rs, Rh)>;
    type Walk = Watched;
    const MAX_DEPTH: usize = 32;

    fn parent_of(&self, state: Rs) -> Option<Rs> {
        self.parent[usize::from(state.0)].map(|p| Rs(p as u8))
    }

    fn is_compound(&self, state: Rs) -> bool {
        self.compound[usize::from(state.0)]
    }

    fn is_parallel(&self, state: Rs) -> bool {
        self.parallel[usize::from(state.0)]
    }

    fn is_final(&self, state: Rs) -> bool {
        self.is_final[usize::from(state.0)]
    }

    fn child_states(&self, state: Rs) -> &'static [Rs] {
        self.kids[usize::from(state.0)]
    }

    fn initial_targets(&self, state: Rs) -> &'static [RTarget] {
        self.initial[usize::from(state.0)]
    }

    fn history_parent(&self, history: Rh) -> Rs {
        Rs(self.history_parent[usize::from(history.0)] as u8)
    }

    fn history_value(&self, history: Rh) -> Option<&[Rs]> {
        self.history_recorded[usize::from(history.0)].as_deref()
    }

    fn history_default_targets(&self, history: Rh) -> &'static [RTarget] {
        self.history_defaults[usize::from(history.0)]
    }

    fn document_order(&self, state: Rs) -> u32 {
        u32::from(state.0)
    }
}

fn random_target(rng: &mut Rng, doc: &Random) -> RTarget {
    if doc.histories() > 0 && rng.chance(3) {
        RTarget::History(Rh(rng.below(doc.histories()) as u8))
    } else {
        RTarget::State(Rs(rng.below(doc.parent.len()) as u8))
    }
}

fn random_transition(rng: &mut Rng, doc: &Random) -> EntryTransition<Rs, Rh> {
    let source = if rng.chance(6) {
        None
    } else {
        Some(Rs(rng.below(doc.parent.len()) as u8))
    };
    let count = 1 + rng.below(3);
    EntryTransition {
        source,
        targets: leak((0..count).map(|_| random_target(rng, doc)).collect()),
        is_internal: rng.chance(2),
    }
}

#[test]
fn the_stack_walks_answer_as_the_recursion_they_replace_for_any_document() {
    let mut deepest_walk = 0;
    for seed in 1..=4000u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let doc = random_document(&mut rng);
        HIGH_WATER.with(|high| high.set(0));

        for _ in 0..8 {
            let first = random_transition(&mut rng, &doc);
            let second = random_transition(&mut rng, &doc);

            // One transition, then two, so the walks also run one after another
            // over the stack the entry set lends them.
            for transitions in [&[first][..], &[first, second][..]] {
                let walked = compute_entry_set(&doc, transitions);
                let reference = recursion::compute_entry_set(&doc, transitions);
                assert_eq!(
                    walked.states_to_enter, reference.states_to_enter,
                    "seed {seed}: states to enter, for {transitions:?}"
                );
                let by_state = |states: &[Rs]| {
                    let mut states = states.to_vec();
                    states.sort_by_key(|state| state.0);
                    states
                };
                assert_eq!(
                    by_state(&walked.states_for_default_entry),
                    by_state(&reference.states_for_default_entry),
                    "seed {seed}: states entered by default, for {transitions:?}"
                );
                // Keyed by parent, a later history replacing an earlier one's, so
                // what is compared is the pair each parent ends with.
                let by_parent = |pairs: &[(Rs, Rh)]| {
                    let mut pairs = pairs.to_vec();
                    pairs.sort_by_key(|pair| (pair.0 .0, pair.1 .0));
                    pairs
                };
                assert_eq!(
                    by_parent(&walked.default_history_content),
                    by_parent(&reference.default_history_content),
                    "seed {seed}: history default content, for {transitions:?}"
                );
            }

            assert_eq!(
                effective_target_states(&doc, first.targets),
                recursion::effective_target_states(&doc, first.targets),
                "seed {seed}: effective targets of {:?}",
                first.targets
            );
        }

        // Completion, for every state against random configurations.
        for state in 0..doc.parent.len() {
            for _ in 0..6 {
                let configuration: Vec<Rs> = (0..doc.parent.len())
                    .filter(|_| rng.chance(2))
                    .map(|i| Rs(i as u8))
                    .collect();
                let state = Rs(state as u8);
                assert_eq!(
                    is_in_final_state(&doc, state, &configuration),
                    recursion::is_in_final_state(&doc, state, &configuration),
                    "seed {seed}: is_in_final_state({state:?}) over {configuration:?}"
                );
            }
        }

        let high = HIGH_WATER.with(std::cell::Cell::get);
        assert!(
            high <= doc.walk_bound(),
            "seed {seed}: the walks stacked {high} steps, and the generator gives a stack of {} \
             for a document {} levels deep with {} histories",
            doc.walk_bound(),
            doc.depth,
            doc.histories()
        );
        deepest_walk = deepest_walk.max(high);
    }
    // The documents do reach into the stack, so the bound above was asked a real
    // question and not an empty one.
    assert!(
        deepest_walk >= 6,
        "the deepest walk was {deepest_walk} steps"
    );
}
