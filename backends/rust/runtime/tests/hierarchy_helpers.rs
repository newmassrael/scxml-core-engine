// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Integration tests for the hierarchy helpers: the bounded buffer a
// configuration is handed to a host in (`StatePolicy::Configuration`, what
// `Engine::get_active_states` returns).
//
// The walks over the hierarchy — descendancy, domains, exit and entry sets —
// are `helpers::microstep`'s, over the document's tables and the slices of the
// machine's workspace, and are held to Appendix D in
// `tests/microstep_algorithms.rs`.

use sce_rust_runtime::helpers::hierarchy::{Bounded, BoundedVec};
use sce_rust_runtime::BoundedPush;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FakeState {
    Root,
    A,
    A1,
    A2,
    B1,
}

// ──────────────────────────────────────────────
// `Bounded` over the buffer a generated policy spells with `BoundedVec`, and
// over the `heapless::Vec` that is it without `std` — on both, whatever the
// profile this test is built in, since the trait is implemented for both.
// ──────────────────────────────────────────────

/// Everything a configuration buffer is asked to do, written once over the
/// trait.
fn builds_in_order<B: Bounded<FakeState>>() {
    let mut chain = B::default();
    assert!(chain.is_empty());
    chain.push_bounded(FakeState::A);
    chain.push_bounded(FakeState::A1);
    chain.push_bounded(FakeState::A2);
    assert_eq!(chain.len(), 3);
    assert_eq!(&chain[..], [FakeState::A, FakeState::A1, FakeState::A2]);
}

fn drops_what_it_is_told_to<B: Bounded<FakeState>>() {
    let mut chain = B::default();
    for state in [
        FakeState::Root,
        FakeState::A,
        FakeState::A1,
        FakeState::A2,
        FakeState::B1,
    ] {
        chain.push_bounded(state);
    }

    chain.retain(|&s| s != FakeState::A);
    assert_eq!(
        &chain[..],
        [FakeState::Root, FakeState::A1, FakeState::A2, FakeState::B1]
    );

    chain.truncate(2);
    assert_eq!(&chain[..], [FakeState::Root, FakeState::A1]);
    // Keeping more than there is does nothing.
    chain.truncate(10);
    assert_eq!(chain.len(), 2);

    chain.clear();
    assert!(chain.is_empty());
}

#[test]
fn a_bounded_vec_builds_in_order() {
    builds_in_order::<BoundedVec<FakeState, 3>>();
}

#[test]
fn a_heapless_vec_builds_in_order() {
    builds_in_order::<heapless::Vec<FakeState, 3>>();
}

#[test]
fn a_bounded_vec_drops_what_it_is_told_to() {
    drops_what_it_is_told_to::<BoundedVec<FakeState, 5>>();
}

#[test]
fn a_heapless_vec_drops_what_it_is_told_to() {
    drops_what_it_is_told_to::<heapless::Vec<FakeState, 5>>();
}

#[test]
#[should_panic(expected = "microstep buffer overflow (capacity 2)")]
fn a_heapless_vec_past_its_capacity_is_a_generator_bug_and_says_so() {
    let mut chain = heapless::Vec::<FakeState, 2>::default();
    chain.push_bounded(FakeState::A);
    chain.push_bounded(FakeState::A1);
    chain.push_bounded(FakeState::A2);
}
