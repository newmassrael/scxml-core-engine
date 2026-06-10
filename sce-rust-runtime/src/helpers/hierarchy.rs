// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 3.3/3.12: hierarchical state algorithms — LCA, entry/exit chains, descendant checks.
//!
//! 1:1 port of `sce/include/core/HierarchicalStateHelper.h`. Uses `P: StatePolicy`
//! as the generic parameter instead of C++'s `template <typename StatePolicy>`.
//! All functions are pure — no side effects, no state.
//!
//! ## Algorithms
//!
//! - [`find_lca`]: Least Common Ancestor of two states (W3C SCXML 3.12)
//! - [`is_descendant_of`]: proper/improper descendant check (W3C Appendix D.2)
//! - [`build_exit_chain`]: ordered exit list from state up to (excluding) LCA
//! - [`build_entry_chain_from_ancestor`]: ordered entry list from LCA down to target
//! - [`build_entry_chain`]: full entry chain from root to leaf, including initial children
//!
//! ## Safety (cycle detection)
//!
//! All chain-building functions bound iteration at `MAX_HIERARCHY_DEPTH` (16 levels) to
//! detect cyclic parent relationships. A cycle indicates a generator bug or corrupted
//! SCXML; the functions panic with a diagnostic message. Matches C++ `throw std::runtime_error`
//! semantics — both are fatal, unrecoverable errors.
//!
//! ## no_std variant (Watching-zenoh RFC §5.J.2)
//!
//! Under `--features=no_std`, the chain return type [`StateChain`] is a stack-allocated
//! `heapless::Vec` capped at [`MAX_HIERARCHY_DEPTH`] (= 16). The same depth-guard panic
//! used under std bounds capacity, so the no_std push paths are infallible by construction
//! — they `.expect()` only as a generator-bug tripwire. No new capacity constant is
//! introduced; the std `Vec<P::State>` and no_std `heapless::Vec<P::State, 16>` share the
//! single existing `MAX_HIERARCHY_DEPTH` invariant.

use crate::policy::StatePolicy;

/// Maximum supported state hierarchy depth. Prevents infinite loops from cyclic
/// parent relationships and bounds stack/heap allocation. Matches C++ `MAX_DEPTH = 16`.
///
/// W3C SCXML has no normative depth limit; 16 covers every real-world document we've
/// encountered (typical: 1-5, complex: up to 10).
pub const MAX_HIERARCHY_DEPTH: usize = 16;

/// Compile-time bounded state chain returned by the hierarchy chain-builders.
///
/// - **std build**: [`Vec<S>`] — unbounded heap allocation, capacity hint only.
/// - **no_std build**: `heapless::Vec<S, MAX_HIERARCHY_DEPTH>` — stack-allocated,
///   compile-time-capped at 16 elements. The capacity is bounded by the existing
///   [`MAX_HIERARCHY_DEPTH`] depth-guard panic in each chain-builder, so push under
///   no_std is infallible-by-construction; a heapless push failure indicates a
///   generator bug (cyclic parent relationship the depth guard missed).
///
/// Watching-zenoh RFC §5.J.2 (lines 1989-1994): reuses [`MAX_HIERARCHY_DEPTH`] rather
/// than introducing a new capacity constant — the same depth invariant bounds both
/// the iteration count and the heapless allocation.
#[cfg(not(feature = "no_std"))]
pub type StateChain<S> = ::std::vec::Vec<S>;
/// no_std variant of [`StateChain`]: stack-allocated `heapless::Vec` capped
/// at [`MAX_HIERARCHY_DEPTH`]. See the std-variant doc-comment above for the
/// full contract.
#[cfg(feature = "no_std")]
pub type StateChain<S> = ::heapless::Vec<S, MAX_HIERARCHY_DEPTH>;

/// Push into a [`StateChain`] uniformly under std and no_std.
///
/// Under std this is `Vec::push`. Under no_std this is `heapless::Vec::push` with
/// an `.expect()` tripwire — the surrounding [`MAX_HIERARCHY_DEPTH`] depth guard
/// in each caller bounds the chain length to the heapless capacity, so the push
/// failure path is unreachable under valid input.
///
/// `pub` so generated state machines (`tools/codegen/templates/rust/`) can call
/// the cfg-branched push body without each fixture inlining the branch. Single
/// source of truth shared between in-crate state walkers (`Engine::get_active_states`)
/// and template-emitted `execute_entry_actions`.
#[inline]
pub fn push_chain<S: core::fmt::Debug>(chain: &mut StateChain<S>, item: S) {
    #[cfg(not(feature = "no_std"))]
    {
        chain.push(item);
    }
    #[cfg(feature = "no_std")]
    {
        chain.push(item).expect(
            "hierarchy: chain capacity exhausted — depth check at MAX_HIERARCHY_DEPTH should have fired first (generator bug or corrupted hierarchy)",
        );
    }
}

/// Construct an empty [`StateChain`].
///
/// Under std this is `Vec::with_capacity(8)` (preserves the existing pre-allocation
/// hint for typical depths 1-5). Under no_std this is `heapless::Vec::new()` — the
/// capacity is fixed at compile time so the hint is a no-op.
///
/// `pub` so generated state machines (`tools/codegen/templates/rust/`) can construct
/// empty chains via the cfg-branched path that resolves identically across std and
/// no_std builds.
#[inline]
pub fn new_chain<S>() -> StateChain<S> {
    #[cfg(not(feature = "no_std"))]
    {
        ::std::vec::Vec::with_capacity(8)
    }
    #[cfg(feature = "no_std")]
    {
        ::heapless::Vec::new()
    }
}

/// Construct a [`StateChain`] from a compile-time-sized array of items.
///
/// Replaces the `vec![a, b, c]` macro for template emission — `vec!` is a std-only
/// macro (`alloc` doesn't re-export it without the `alloc` feature, and heapless
/// has no equivalent). Generated state machines use this at every site that
/// previously emitted `vec![Self::State::A, Self::State::B]` for initial-children
/// lists and similar fixed-size state collections.
///
/// `N` must be `<= MAX_HIERARCHY_DEPTH` (= 16); larger arrays trigger the same
/// capacity-exhausted panic as [`push_chain`]. Initial-children lists are derived
/// at codegen time from the SCXML document, so generator-bug detection is the
/// only path that fires the panic.
#[inline]
pub fn state_chain_from_slice<S: core::fmt::Debug, const N: usize>(items: [S; N]) -> StateChain<S> {
    let mut chain = new_chain::<S>();
    for item in items {
        push_chain(&mut chain, item);
    }
    chain
}

/// W3C SCXML 3.12: Find the Least Common Ancestor of two states.
///
/// Returns `Some(lca)` if the states share a common ancestor (or are equal),
/// `None` if they belong to disjoint hierarchies (only possible if the SCXML
/// has multiple root states, which is invalid — the function still handles it
/// gracefully).
///
/// 1:1 port of C++ `HierarchicalAlgorithms::findLCA`.
///
/// # Algorithm
///
/// 1. If `state1 == state2`, return it directly.
/// 2. Walk up from `state1`'s parent, collecting ancestors into a list.
/// 3. Walk up from `state2`, checking each ancestor against the list; first
///    match is the LCA.
///
/// # Panics
///
/// Panics if either state's ancestor chain exceeds [`MAX_HIERARCHY_DEPTH`] (indicates
/// a cycle).
pub fn find_lca<P: StatePolicy>(state1: P::State, state2: P::State) -> Option<P::State> {
    if state1 == state2 {
        return Some(state1);
    }

    // Build state1's ancestor chain (starting from state1's parent, per W3C 3.13 test 504)
    let mut ancestors1: StateChain<P::State> = new_chain();
    if let Some(mut current) = P::get_parent(state1) {
        let mut depth = 0;
        loop {
            if depth >= MAX_HIERARCHY_DEPTH {
                panic!(
                    "find_lca: cyclic parent relationship detected while walking ancestors of {:?}",
                    state1
                );
            }
            push_chain(&mut ancestors1, current);
            match P::get_parent(current) {
                Some(parent) => current = parent,
                None => break,
            }
            depth += 1;
        }
    }

    // Walk up from state2, looking for intersection
    let mut current = state2;
    let mut depth = 0;
    loop {
        if depth >= MAX_HIERARCHY_DEPTH {
            panic!(
                "find_lca: cyclic parent relationship detected while walking ancestors of {:?}",
                state2
            );
        }
        if ancestors1.contains(&current) {
            return Some(current);
        }
        match P::get_parent(current) {
            Some(parent) => current = parent,
            None => return None,
        }
        depth += 1;
    }
}

/// W3C SCXML Appendix D.2: check whether `descendant` is a strict descendant of `ancestor`.
///
/// Returns `true` if walking up from `descendant` via `get_parent` eventually reaches
/// `ancestor`. Returns `false` if the walk terminates at the root without finding it,
/// or if `descendant == ancestor` (strict descendancy — self is not a descendant).
///
/// 1:1 port of C++ `HierarchicalAlgorithms::isDescendantOf`. Note: the C++ version
/// delegates to `StatePolicy::isDescendantOf` when generated code provides a fast
/// baked-in table. This wrapper always walks the hierarchy — for the baked table
/// path, call `P::is_descendant_of` directly.
///
/// # Panics
///
/// Panics if the ancestor walk exceeds [`MAX_HIERARCHY_DEPTH`].
pub fn is_descendant_of<P: StatePolicy>(descendant: P::State, ancestor: P::State) -> bool {
    let mut current = descendant;
    let mut depth = 0;
    loop {
        if depth >= MAX_HIERARCHY_DEPTH {
            panic!(
                "is_descendant_of: cyclic parent relationship detected walking from {:?}",
                descendant
            );
        }
        match P::get_parent(current) {
            None => return false,
            Some(parent) => {
                if parent == ancestor {
                    return true;
                }
                current = parent;
            }
        }
        depth += 1;
    }
}

/// W3C SCXML 3.12: Build an exit chain from `from_state` up to (but excluding) `stop_before_state`.
///
/// Returns states in leaf → ancestor order, suitable for calling `execute_exit_actions`
/// in sequence. The stop state is the LCA — it is not exited because the transition
/// stays within its subtree.
///
/// 1:1 port of C++ `HierarchicalAlgorithms::buildExitChain`.
///
/// # Panics
///
/// Panics if the chain exceeds [`MAX_HIERARCHY_DEPTH`].
pub fn build_exit_chain<P: StatePolicy>(
    from_state: P::State,
    stop_before_state: P::State,
) -> StateChain<P::State> {
    let mut chain: StateChain<P::State> = new_chain();
    let mut current = from_state;
    let mut depth = 0;

    while current != stop_before_state {
        if depth >= MAX_HIERARCHY_DEPTH {
            panic!(
                "build_exit_chain: cyclic parent relationship detected walking from {:?}",
                from_state
            );
        }
        push_chain(&mut chain, current);
        match P::get_parent(current) {
            Some(parent) => current = parent,
            None => break, // Reached root without finding stop state — return what we have
        }
        depth += 1;
    }

    chain
}

/// W3C SCXML 3.12: Build an entry chain from `ancestor` down to `target` (exclusive of ancestor).
///
/// Returns states in ancestor → descendant order, suitable for calling `execute_entry_actions`
/// in sequence. The ancestor itself is not included because it is already active at the
/// point where the chain is applied.
///
/// 1:1 port of C++ `HierarchicalAlgorithms::buildEntryChainFromAncestor`.
///
/// # Panics
///
/// Panics if the chain exceeds [`MAX_HIERARCHY_DEPTH`].
pub fn build_entry_chain_from_ancestor<P: StatePolicy>(
    target: P::State,
    ancestor: P::State,
) -> StateChain<P::State> {
    let mut chain: StateChain<P::State> = new_chain();
    let mut current = target;
    let mut depth = 0;

    while current != ancestor {
        if depth >= MAX_HIERARCHY_DEPTH {
            panic!(
                "build_entry_chain_from_ancestor: cyclic parent relationship detected walking from {:?}",
                target
            );
        }
        push_chain(&mut chain, current);
        match P::get_parent(current) {
            Some(parent) => current = parent,
            None => break, // Reached root without finding ancestor
        }
        depth += 1;
    }

    chain.reverse();
    chain
}

/// W3C SCXML 3.3: Build the complete entry chain from root down to `leaf_state`.
///
/// Returns states in root → leaf order. Parallel region children are NOT added here —
/// that responsibility belongs to `execute_entry_actions` (matches C++ comment at
/// `HierarchicalStateHelper.h:337-339`: "Do NOT add parallel regions here").
///
/// 1:1 port of C++ `HierarchicalStateHelper<StatePolicy>::buildEntryChain(State)`. The
/// initial-child expansion for compound states is a *follow-up* pass after the main
/// root-to-leaf walk — it's required for cases like "initial state is a compound
/// state with its own initial child" (e.g., initial="s1" where s1 is compound).
///
/// # Panics
///
/// Panics if the chain exceeds [`MAX_HIERARCHY_DEPTH`].
pub fn build_entry_chain<P: StatePolicy>(leaf_state: P::State) -> StateChain<P::State> {
    let mut chain: StateChain<P::State> = new_chain();
    let mut current = leaf_state;
    let mut depth = 0;

    // Walk from leaf to root
    loop {
        if depth >= MAX_HIERARCHY_DEPTH {
            panic!(
                "build_entry_chain: cyclic parent relationship detected walking from {:?}",
                leaf_state
            );
        }
        push_chain(&mut chain, current);
        match P::get_parent(current) {
            Some(parent) => current = parent,
            None => break, // Reached root
        }
        depth += 1;
    }

    // Reverse to root → leaf order
    chain.reverse();

    // W3C SCXML 3.3: the chain deliberately stops at `leaf_state` without
    // descending into a compound target's initial children. Document-initial
    // descent is resolved at codegen time (sce-build's `resolve_deep_initial`
    // rewrites `model.initial` to the deepest leaf before `initial_state()` is
    // emitted), and runtime descent for compound transition targets is owned
    // by `Engine::resolve_current_state_to_leaf` via
    // `StatePolicy::get_initial_or_history_child`, which also enters the
    // descended children. Descending here too would double-enter them.

    chain
}

// ──────────────────────────────────────────────
// Unit tests use a fake policy — see tests/hierarchy_helpers.rs for
// integration tests. Inline tests here only cover edge cases of the
// generic algorithms.
// ──────────────────────────────────────────────
