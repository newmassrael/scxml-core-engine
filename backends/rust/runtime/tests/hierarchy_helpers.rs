// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Integration tests for the hierarchy helpers: the bounded `StateChain` every
// configuration is held in, and the three constructors the public surface
// offers for it.
//
// The walks over the hierarchy — descendancy, domains, exit and entry sets —
// are `helpers::microstep`'s, and are held to Appendix D in
// `tests/microstep_algorithms.rs`.

use sce_rust_runtime::helpers::hierarchy::{
    new_chain, push_chain, state_chain_from_slice, StateChain,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FakeState {
    Root,
    A,
    A1,
    A2,
    B1,
}

// ──────────────────────────────────────────────
// new_chain / push_chain / state_chain_from_slice (C3 follow-up #1)
// Public so a host can build the `StateChain` `Engine::enter_at` takes on
// either runtime profile — these tests pin that surface.
// ──────────────────────────────────────────────

#[test]
fn new_chain_starts_empty() {
    let chain: StateChain<FakeState> = new_chain();
    assert!(chain.is_empty());
}

#[test]
fn push_chain_appends_in_order() {
    let mut chain: StateChain<FakeState> = new_chain();
    push_chain(&mut chain, FakeState::A);
    push_chain(&mut chain, FakeState::A1);
    push_chain(&mut chain, FakeState::A2);
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0], FakeState::A);
    assert_eq!(chain[1], FakeState::A1);
    assert_eq!(chain[2], FakeState::A2);
}

#[test]
fn state_chain_from_slice_preserves_order() {
    let chain = state_chain_from_slice([FakeState::A1, FakeState::A2, FakeState::B1]);
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0], FakeState::A1);
    assert_eq!(chain[1], FakeState::A2);
    assert_eq!(chain[2], FakeState::B1);
}

#[test]
fn state_chain_from_slice_empty_array() {
    let chain: StateChain<FakeState> = state_chain_from_slice([]);
    assert!(chain.is_empty());
}

#[test]
fn state_chain_from_slice_single_item() {
    let chain = state_chain_from_slice([FakeState::Root]);
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0], FakeState::Root);
}
