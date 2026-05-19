// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 3.11: History state recording and restoration.
//!
//! 1:1 port of `sce/include/core/HistoryHelper.h`. Provides shallow and deep
//! history filtering, ancestor chain construction for restoration, and active
//! hierarchy queries.
//!
//! All functions are generic over `P: StatePolicy`.
//!
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): whole-module gated to `!no_std`.
//! The history-state helpers use `Vec<P::State>` + `HashSet<P::State>`
//! accumulators and have zero template consumer — the Rust AOT generator emits
//! history-state restoration inline.

#![cfg(not(feature = "no_std"))]

use std::collections::HashSet;

use crate::policy::StatePolicy;

/// History type classification (ports C++ `SCE::HistoryType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryType {
    /// Not a history state.
    None,
    /// Shallow history: records only immediate child states.
    Shallow,
    /// Deep history: records all leaf (atomic) descendant states.
    Deep,
}

/// W3C SCXML 3.11: Filter states for shallow history recording.
///
/// Shallow history records only immediate child states of the parent compound state.
///
/// Ports C++ `HistoryHelper::filterShallowHistory`.
pub fn filter_shallow_history<P: StatePolicy>(
    active_states: &[P::State],
    parent_state: P::State,
) -> Vec<P::State> {
    active_states
        .iter()
        .filter(|&&state| P::get_parent(state) == Some(parent_state))
        .copied()
        .collect()
}

/// Check if `state` is a descendant of `parent_state`.
///
/// Ports C++ `HistoryHelper::isDescendant`.
fn is_descendant<P: StatePolicy>(state: P::State, parent_state: P::State) -> bool {
    if state == parent_state {
        return false;
    }

    let mut current = P::get_parent(state);
    while let Some(cur) = current {
        if cur == parent_state {
            return true;
        }
        current = P::get_parent(cur);
    }

    false
}

/// W3C SCXML 3.11: Filter states for deep history recording.
///
/// Deep history records all leaf (atomic) descendant states of the parent compound
/// state. A leaf state is one that has no active child states in the current
/// configuration.
///
/// Ports C++ `HistoryHelper::filterDeepHistory`.
pub fn filter_deep_history<P: StatePolicy>(
    active_states: &[P::State],
    parent_state: P::State,
) -> Vec<P::State> {
    let active_set: HashSet<P::State> = active_states.iter().copied().collect();
    let mut filtered = Vec::new();

    for &state in active_states {
        // Must be a descendant of parent
        if !is_descendant::<P>(state, parent_state) {
            continue;
        }

        // Check if this is a leaf state (no active children)
        let is_leaf = !active_states
            .iter()
            .any(|&other| other != state && P::get_parent(other) == Some(state));

        // Only suppress for leaf check -- use active_set for completeness
        let _ = &active_set;

        if is_leaf {
            filtered.push(state);
        }
    }

    filtered
}

/// W3C SCXML 3.11: Record history for a compound state.
///
/// Dispatches to shallow or deep filtering based on `history_type`.
///
/// Ports C++ `HistoryHelper::recordHistory`.
pub fn record_history<P: StatePolicy>(
    active_states: &[P::State],
    parent_state: P::State,
    history_type: HistoryType,
) -> Vec<P::State> {
    match history_type {
        HistoryType::Shallow => filter_shallow_history::<P>(active_states, parent_state),
        HistoryType::Deep => filter_deep_history::<P>(active_states, parent_state),
        HistoryType::None => Vec::new(),
    }
}

/// W3C SCXML 3.11: Get ancestor chain for entering a history target state.
///
/// Builds the ancestor chain from `target` up to (but not including)
/// `stop_at_parent`, then returns in parent-before-child order.
///
/// Ports C++ `HistoryHelper::getAncestorsToEnter`.
pub fn get_ancestors_to_enter<P: StatePolicy>(
    target: P::State,
    stop_at_parent: Option<P::State>,
) -> Vec<P::State> {
    let mut ancestors = Vec::new();
    let mut current = target;

    loop {
        ancestors.push(current);

        if let Some(stop) = stop_at_parent {
            if current == stop {
                ancestors.pop(); // Don't include stop_at_parent itself
                break;
            }
        }

        match P::get_parent(current) {
            None => break,
            Some(parent) => current = parent,
        }
    }

    // W3C SCXML 3.11: Reverse to parent-before-child order
    ancestors.reverse();
    ancestors
}

/// W3C SCXML 3.11: Get active state hierarchy (current state + all ancestors).
///
/// Returns the complete hierarchy from leaf to root.
///
/// Ports C++ `HistoryHelper::getActiveHierarchy`.
pub fn get_active_hierarchy<P: StatePolicy>(current_state: P::State) -> Vec<P::State> {
    let mut active_states = Vec::new();
    let mut current = current_state;

    loop {
        active_states.push(current);
        match P::get_parent(current) {
            None => break,
            Some(parent) => current = parent,
        }
    }

    active_states
}
