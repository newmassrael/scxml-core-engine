// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 3.6: Deep initial state entry logic.
//!
//! 1:1 port of `sce/include/core/StateEntryHelper.h`. Provides ancestor path
//! calculation and optimized entry order for deep initial targets.
//!
//! All functions are generic over `P: StatePolicy`.

use std::collections::HashSet;
use std::hash::Hash;

use crate::policy::StatePolicy;

/// Maximum depth for hierarchy traversal. Prevents infinite loops from cyclic
/// parent relationships in malformed SCXML documents.
const MAX_DEPTH: usize = 100;

/// W3C SCXML 3.6: Calculate ancestor path for a deep initial target.
///
/// Returns ancestor path from `parent + 1` to `target` (inclusive) in
/// top-to-bottom (document) order.
///
/// Ports C++ `StateEntryHelper::calculateAncestorPath`.
///
/// # Panics
///
/// Panics if a cycle is detected or if hierarchy depth exceeds [`MAX_DEPTH`].
pub fn calculate_ancestor_path<P: StatePolicy>(
    target: P::State,
    parent: P::State,
) -> Vec<P::State> {
    let mut ancestors: Vec<P::State> = Vec::new();
    let mut current = target;
    let mut visited: HashSet<P::State> = HashSet::new();
    visited.insert(target);
    visited.insert(parent);

    let mut depth = 0;

    while depth < MAX_DEPTH {
        match P::get_parent(current) {
            None => break, // Reached root without finding parent
            Some(parent_state) => {
                if parent_state == parent {
                    break; // Reached desired parent
                }

                // W3C SCXML: Detect cycles (invalid SCXML)
                if visited.contains(&parent_state) {
                    panic!(
                        "W3C SCXML 3.6: Cycle detected in state hierarchy. \
                         State has circular ancestor path."
                    );
                }
                visited.insert(parent_state);

                ancestors.push(parent_state);
                current = parent_state;
                depth += 1;
            }
        }
    }

    if depth >= MAX_DEPTH {
        panic!(
            "W3C SCXML 3.6: Maximum hierarchy depth exceeded. \
             State has ancestor chain > {} levels.",
            MAX_DEPTH
        );
    }

    // W3C SCXML 3.13: Reverse to get document order (top-to-bottom)
    ancestors.reverse();
    ancestors.push(target);

    ancestors
}

/// W3C SCXML 3.6: Optimize entry order across multiple deep targets.
///
/// Deduplicates common ancestors while preserving document order.
///
/// Ports C++ `StateEntryHelper::optimizeEntryOrder`.
pub fn optimize_entry_order<S: Copy + Eq + Hash>(paths: &[Vec<S>]) -> Vec<S> {
    let mut optimized: Vec<S> = Vec::new();
    let mut seen: HashSet<S> = HashSet::new();

    // W3C SCXML 3.13: Preserve document order
    for path in paths {
        for &state in path {
            if seen.insert(state) {
                optimized.push(state);
            }
        }
    }

    optimized
}

/// W3C SCXML 3.6: Enter deep initial targets with optimized traversal.
///
/// Single Source of Truth for deep state entry logic.
///
/// Algorithm:
/// 1. Calculate ancestor paths for all targets
/// 2. Optimize entry order (deduplicate common ancestors)
/// 3. Execute entry actions in optimized order
///
/// Ports C++ `StateEntryHelper::enterDeepTargets`.
pub fn enter_deep_targets<P: StatePolicy>(
    parent: P::State,
    targets: &[P::State],
    mut execute_entry: impl FnMut(P::State),
) {
    if targets.is_empty() {
        return;
    }

    // W3C SCXML 3.6: Calculate ancestor paths for all targets
    let all_paths: Vec<Vec<P::State>> = targets
        .iter()
        .map(|&target| calculate_ancestor_path::<P>(target, parent))
        .collect();

    // W3C SCXML 3.6: Optimize entry order (deduplicate ancestors)
    let optimized_order = optimize_entry_order(&all_paths);

    // W3C SCXML 3.8: Execute entry actions in optimized order
    for state in optimized_order {
        execute_entry(state);
    }
}
