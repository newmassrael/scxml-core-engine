// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Integration tests for hierarchy helpers (LCA, descendant, entry/exit chains).
//
// Uses a fake 4-level hierarchy policy to exercise the algorithms without
// pulling in a real generated state machine. Tests match C++ unit tests at
// `tests/ctest/helpers/HierarchicalStateHelperTest.cpp` conceptually.
//
//     Root
//      ├── A
//      │   ├── A1
//      │   └── A2
//      └── B
//          └── B1

use sce_rust_runtime::helpers::hierarchy::{
    build_entry_chain, build_entry_chain_from_ancestor, build_exit_chain, find_lca,
    is_descendant_of,
};
use sce_rust_runtime::{Engine, StatePolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FakeState {
    Root,
    A,
    A1,
    A2,
    B,
    B1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FakeEvent {
    Null,
    Tick,
}

struct FakePolicy;

impl FakePolicy {
    // Constructor kept for parity with the FakePolicy used in other tests in
    // this file even though the current test set instantiates the unit
    // struct directly. Dropping it would diverge from the convention.
    #[allow(dead_code)]
    fn new() -> Self {
        FakePolicy
    }
}

impl StatePolicy for FakePolicy {
    type State = FakeState;
    type Event = FakeEvent;

    fn initial_state() -> Self::State {
        FakeState::A1
    }

    fn is_final_state(_state: Self::State) -> bool {
        false
    }

    fn get_parent(state: Self::State) -> Option<Self::State> {
        match state {
            FakeState::Root => None,
            FakeState::A => Some(FakeState::Root),
            FakeState::A1 | FakeState::A2 => Some(FakeState::A),
            FakeState::B => Some(FakeState::Root),
            FakeState::B1 => Some(FakeState::B),
        }
    }

    fn is_compound_state(state: Self::State) -> bool {
        matches!(state, FakeState::Root | FakeState::A | FakeState::B)
    }

    fn is_descendant_of(desc: Self::State, anc: Self::State) -> bool {
        // For tests, use the helper (recursive walk)
        let mut current = desc;
        loop {
            match Self::get_parent(current) {
                None => return false,
                Some(parent) => {
                    if parent == anc {
                        return true;
                    }
                    current = parent;
                }
            }
        }
    }

    fn get_document_order(state: Self::State) -> u32 {
        match state {
            FakeState::Root => 0,
            FakeState::A => 1,
            FakeState::A1 => 2,
            FakeState::A2 => 3,
            FakeState::B => 4,
            FakeState::B1 => 5,
        }
    }

    fn get_event_name(_event: Self::Event) -> &'static str {
        "tick"
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        if name == "tick" {
            Some(FakeEvent::Tick)
        } else {
            None
        }
    }

    fn null_event() -> Self::Event {
        FakeEvent::Null
    }

    fn last_transition_is_internal(&self) -> bool {
        false
    }
    fn set_last_transition_is_internal(&mut self, _v: bool) {}
    fn last_transition_is_targetless(&self) -> bool {
        false
    }
    fn set_last_transition_is_targetless(&mut self, _v: bool) {}
    fn last_transition_source_state(&self) -> Self::State {
        FakeState::Root
    }
    fn set_last_transition_source_state(&mut self, _s: Self::State) {}

    fn execute_entry_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {}
    fn execute_exit_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _pre: &[Self::State],
    ) {
    }
    fn process_transition(
        &mut self,
        _cs: &mut Self::State,
        _e: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> bool {
        false
    }
    fn execute_transition_actions(&mut self, _engine: &mut Engine<Self>) {}
}

// ──────────────────────────────────────────────
// is_descendant_of
// ──────────────────────────────────────────────

#[test]
fn descendant_direct_child() {
    assert!(is_descendant_of::<FakePolicy>(FakeState::A1, FakeState::A));
    assert!(is_descendant_of::<FakePolicy>(FakeState::B1, FakeState::B));
}

#[test]
fn descendant_grandchild() {
    assert!(is_descendant_of::<FakePolicy>(
        FakeState::A1,
        FakeState::Root
    ));
    assert!(is_descendant_of::<FakePolicy>(
        FakeState::B1,
        FakeState::Root
    ));
}

#[test]
fn descendant_self_is_not_descendant() {
    // W3C Appendix D.2: strict descendancy — a state is not its own descendant
    assert!(!is_descendant_of::<FakePolicy>(FakeState::A, FakeState::A));
}

#[test]
fn descendant_unrelated_branches() {
    assert!(!is_descendant_of::<FakePolicy>(FakeState::A1, FakeState::B));
    assert!(!is_descendant_of::<FakePolicy>(FakeState::B1, FakeState::A));
}

// ──────────────────────────────────────────────
// find_lca
// ──────────────────────────────────────────────

#[test]
fn lca_of_equal_states_is_self() {
    assert_eq!(
        find_lca::<FakePolicy>(FakeState::A1, FakeState::A1),
        Some(FakeState::A1)
    );
}

#[test]
fn lca_of_siblings_is_parent() {
    assert_eq!(
        find_lca::<FakePolicy>(FakeState::A1, FakeState::A2),
        Some(FakeState::A)
    );
}

#[test]
fn lca_of_cross_branch_is_root() {
    assert_eq!(
        find_lca::<FakePolicy>(FakeState::A1, FakeState::B1),
        Some(FakeState::Root)
    );
}

#[test]
fn lca_is_asymmetric_for_ancestor_descendant_pairs() {
    // W3C SCXML 5.9.2: the LCA algorithm builds state1's ancestor chain from
    // `parent(state1)` (NOT from state1 itself), then walks state2's chain
    // from state2 itself. This matches C++ `findLCA` at HierarchicalStateHelper.h
    // and is INTENTIONALLY ASYMMETRIC.
    //
    // For find_lca(A, A1): ancestors1 = [Root] (A's parent chain). Walk from
    // A1: A1 → A → Root. Root matches → returns Root.
    assert_eq!(
        find_lca::<FakePolicy>(FakeState::A, FakeState::A1),
        Some(FakeState::Root)
    );

    // For find_lca(A1, A): ancestors1 = [A, Root] (A1's parent chain). Walk
    // from A: A matches in [A, Root] at index 0 → returns A.
    //
    // This asymmetry is required by W3C external-transition semantics where
    // the source state is state1 and target is state2 — the LCA is the
    // lowest state the transition needs to re-enter, which differs depending
    // on direction.
    assert_eq!(
        find_lca::<FakePolicy>(FakeState::A1, FakeState::A),
        Some(FakeState::A)
    );
}

// ──────────────────────────────────────────────
// build_exit_chain
// ──────────────────────────────────────────────

#[test]
fn exit_chain_leaf_to_ancestor() {
    // From A1 exiting up to Root (exclusive) should yield [A1, A]
    let chain = build_exit_chain::<FakePolicy>(FakeState::A1, FakeState::Root);
    assert_eq!(chain, vec![FakeState::A1, FakeState::A]);
}

#[test]
fn exit_chain_stop_at_self_is_empty() {
    let chain = build_exit_chain::<FakePolicy>(FakeState::A1, FakeState::A1);
    assert!(chain.is_empty());
}

// ──────────────────────────────────────────────
// build_entry_chain / build_entry_chain_from_ancestor
// ──────────────────────────────────────────────

#[test]
fn entry_chain_full_path_to_leaf() {
    // Root → A → A1
    let chain = build_entry_chain::<FakePolicy>(FakeState::A1);
    assert_eq!(chain, vec![FakeState::Root, FakeState::A, FakeState::A1]);
}

#[test]
fn entry_chain_from_ancestor_excludes_ancestor() {
    // From Root down to A2 → [A, A2]
    let chain = build_entry_chain_from_ancestor::<FakePolicy>(FakeState::A2, FakeState::Root);
    assert_eq!(chain, vec![FakeState::A, FakeState::A2]);
}
