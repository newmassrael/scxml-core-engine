// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

use crate::policy::StatePolicy;

/// Maximum supported state hierarchy depth. Prevents infinite loops from cyclic
/// parent relationships and bounds stack/heap allocation. Matches C++ `MAX_DEPTH = 16`.
///
/// W3C SCXML has no normative depth limit; 16 covers every real-world document we've
/// encountered (typical: 1-5, complex: up to 10).
pub const MAX_HIERARCHY_DEPTH: usize = 16;

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
    let mut ancestors1: Vec<P::State> = Vec::with_capacity(8);
    if let Some(mut current) = P::get_parent(state1) {
        let mut depth = 0;
        loop {
            if depth >= MAX_HIERARCHY_DEPTH {
                panic!(
                    "find_lca: cyclic parent relationship detected while walking ancestors of {:?}",
                    state1
                );
            }
            ancestors1.push(current);
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
) -> Vec<P::State> {
    let mut chain: Vec<P::State> = Vec::with_capacity(8);
    let mut current = from_state;
    let mut depth = 0;

    while current != stop_before_state {
        if depth >= MAX_HIERARCHY_DEPTH {
            panic!(
                "build_exit_chain: cyclic parent relationship detected walking from {:?}",
                from_state
            );
        }
        chain.push(current);
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
) -> Vec<P::State> {
    let mut chain: Vec<P::State> = Vec::with_capacity(8);
    let mut current = target;
    let mut depth = 0;

    while current != ancestor {
        if depth >= MAX_HIERARCHY_DEPTH {
            panic!(
                "build_entry_chain_from_ancestor: cyclic parent relationship detected walking from {:?}",
                target
            );
        }
        chain.push(current);
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
pub fn build_entry_chain<P: StatePolicy>(leaf_state: P::State) -> Vec<P::State> {
    let mut chain: Vec<P::State> = Vec::with_capacity(8);
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
        chain.push(current);
        match P::get_parent(current) {
            Some(parent) => current = parent,
            None => break, // Reached root
        }
        depth += 1;
    }

    // Reverse to root → leaf order
    chain.reverse();

    // W3C SCXML 3.3: If the leaf is a compound state, extend the chain with its
    // initial child transitively. This is required when `initial_state()` itself
    // returns a compound state — the engine must enter its initial child too.
    //
    // Phase 1 note: `StatePolicy` does not yet expose `get_initial_child`.
    // The generator (Phase 2) will emit `initial_state()` pointing at the already-
    // resolved deepest leaf, so this loop is a no-op for Phase 1. Kept here as a
    // placeholder matching the C++ structure for Phase 2 port clarity.
    //
    // TODO(phase-2): add `get_initial_child(state)` to StatePolicy and extend here.

    chain
}

// ──────────────────────────────────────────────
// Unit tests use a fake policy — see tests/hierarchy_helpers.rs for
// integration tests. Inline tests here only cover edge cases of the
// generic algorithms.
// ──────────────────────────────────────────────
