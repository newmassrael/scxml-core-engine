// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML Appendix D.2: conflict resolution algorithms.
//!
//! 1:1 port of `sce/include/core/ConflictResolutionHelper.h`. Provides unified
//! conflict resolution logic shared by both AOT and (future) interpreter engines.
//!
//! All functions are generic over `P: StatePolicy`, matching the C++ pattern of
//! `template <typename StatePolicy>`.

use crate::helpers::hierarchy;
use crate::policy::StatePolicy;

/// W3C SCXML Appendix D.2: Transition descriptor for conflict resolution.
///
/// Ports C++ `ConflictResolutionAlgorithms::TransitionDescriptor<StateType>`.
#[derive(Debug, Clone)]
pub struct TransitionDescriptor<S> {
    /// Source state of the transition.
    pub source: S,
    /// Target state of the transition.
    pub target: S,
    /// States exited by this transition (computed by [`compute_exit_set`]).
    pub exit_set: Vec<S>,
    /// Index of the transition in generated code (for `execute_transition_actions`).
    pub transition_index: i32,
    /// Whether the transition has executable content.
    pub has_actions: bool,
    /// W3C SCXML 3.13: Whether the transition is `type="internal"`.
    pub is_internal: bool,
    /// W3C SCXML 5.9.2: Whether the transition has no `target` attribute.
    pub is_targetless: bool,
    /// W3C SCXML 3.13: Whether the transition exits a parallel state.
    pub is_external: bool,
}

impl<S: Default> Default for TransitionDescriptor<S> {
    fn default() -> Self {
        Self {
            source: S::default(),
            target: S::default(),
            exit_set: Vec::new(),
            transition_index: 0,
            has_actions: false,
            is_internal: false,
            is_targetless: false,
            is_external: false,
        }
    }
}

impl<S> TransitionDescriptor<S> {
    /// Construct a transition descriptor with source, target, and metadata.
    pub fn new(
        source: S,
        target: S,
        transition_index: i32,
        has_actions: bool,
        is_internal: bool,
        is_targetless: bool,
    ) -> Self {
        Self {
            source,
            target,
            exit_set: Vec::new(),
            transition_index,
            has_actions,
            is_internal,
            is_targetless,
            is_external: false,
        }
    }
}

/// W3C SCXML Appendix D.2: Check if two exit sets have non-empty intersection.
///
/// Two transitions conflict if their exit sets share any common state.
///
/// Ports C++ `ConflictResolutionAlgorithms::hasIntersection`.
pub fn has_intersection<S: Eq>(set1: &[S], set2: &[S]) -> bool {
    for s1 in set1 {
        for s2 in set2 {
            if s1 == s2 {
                return true;
            }
        }
    }
    false
}

/// W3C SCXML Appendix D.2: Compute exit set for a single transition.
///
/// Exit set = states from source up to (but not including) LCA with target.
/// Delegates to hierarchy helpers for LCA computation.
///
/// Ports C++ `ConflictResolutionHelper<StatePolicy>::computeExitSet`.
pub fn compute_exit_set<P: StatePolicy>(
    source: P::State,
    target: P::State,
    is_internal: bool,
    is_targetless: bool,
) -> Vec<P::State> {
    // W3C SCXML 5.9.2: Targetless transitions have empty exit set
    if is_targetless {
        return Vec::new();
    }

    // W3C SCXML 3.13: Internal transition to compound descendant -- source stays active
    if is_internal {
        let source_is_compound =
            P::is_compound_state(source) && !P::is_parallel_state(source);
        if source_is_compound && P::is_descendant_of(target, source) && target != source {
            return Vec::new();
        }
    }

    // External transition: compute exit set from source up to LCA
    let lca = hierarchy::find_lca::<P>(source, target);

    let mut exit_set = Vec::new();
    let mut current = source;
    loop {
        exit_set.push(current);
        match P::get_parent(current) {
            Some(parent) => {
                if let Some(lca_state) = lca {
                    if parent == lca_state {
                        break;
                    }
                }
                current = parent;
            }
            None => break,
        }
    }

    exit_set
}

/// W3C SCXML Appendix D.2: Remove conflicting transitions.
///
/// From all enabled transitions (in document order), produce a filtered
/// non-conflicting set using W3C preemption rules.
///
/// Ports C++ `ConflictResolutionAlgorithms::removeConflictingTransitions`.
pub fn remove_conflicting_transitions<P: StatePolicy>(
    enabled_transitions: &[TransitionDescriptor<P::State>],
) -> Vec<TransitionDescriptor<P::State>> {
    let mut filtered: Vec<TransitionDescriptor<P::State>> = Vec::new();

    for t1 in enabled_transitions {
        let mut t1_preempted = false;
        let mut to_remove: Vec<usize> = Vec::new();

        for (i, t2) in filtered.iter().enumerate() {
            let mut has_conflict = false;

            // W3C SCXML Appendix D.2: Check if exit sets intersect
            if has_intersection(&t1.exit_set, &t2.exit_set) {
                has_conflict = true;
            }

            // W3C SCXML Appendix D.2: Target/source conflict detection
            if !has_conflict && (t1.target == t2.source || t2.target == t1.source) {
                has_conflict = true;
            }

            // W3C SCXML 3.13: Parallel state conflict detection
            if !(has_conflict || t2.is_internal && t2.source == t2.target) {
                for exit_state in &t1.exit_set {
                    if P::is_parallel_state(*exit_state)
                        && P::is_descendant_of(t2.source, *exit_state)
                    {
                        has_conflict = true;
                        break;
                    }
                }
            }

            // Check reverse: t2 exits parallel state that is ancestor of t1's source
            if !(has_conflict || t1.is_internal && t1.source == t1.target) {
                for exit_state in &t2.exit_set {
                    if P::is_parallel_state(*exit_state)
                        && P::is_descendant_of(t1.source, *exit_state)
                    {
                        has_conflict = true;
                        break;
                    }
                }
            }

            if has_conflict {
                // W3C SCXML Appendix D.2: Preemption rules
                if t1.target == t2.source {
                    to_remove.push(i);
                } else if t2.target == t1.source {
                    t1_preempted = true;
                } else if P::is_descendant_of(t1.source, t2.source) {
                    to_remove.push(i);
                } else {
                    t1_preempted = true;
                }
            }
        }

        if !t1_preempted {
            // Remove in reverse order to preserve indices
            for &idx in to_remove.iter().rev() {
                filtered.remove(idx);
            }
            filtered.push(t1.clone());
        }
    }

    filtered
}
