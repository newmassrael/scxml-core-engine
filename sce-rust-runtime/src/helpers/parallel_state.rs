// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 3.4: Parallel state processing algorithms.
//!
//! Merged port of six C++ headers into a single Rust module:
//! - `sce/include/core/ParallelStateHelper.h` (state queries, document order)
//! - `sce/include/core/ParallelExitEntryHelper.h` (exit/entry order computation)
//! - `sce/include/core/ParallelTransitionHelper.h` (transition conflict detection)
//! - `sce/include/core/ParallelCompletionHelper.h` (all-regions-final check)
//! - `sce/include/core/ParallelConfigurationHelper.h` (configuration tracking)
//! - `sce/include/core/ParallelProcessingAlgorithms.h` (region enter/exit/broadcast)
//!
//! All functions are generic over `P: StatePolicy`.

use std::collections::{HashMap, HashSet};

use crate::helpers::hierarchy;
use crate::policy::StatePolicy;

// ═══════════════════════════════════════════════════════════════════════════════
// ParallelStateHelper: state queries and document order
// ═══════════════════════════════════════════════════════════════════════════════

/// W3C SCXML 3.4: Get all child regions of a parallel state.
///
/// Ports C++ `ParallelStateHelper::getParallelRegions`.
pub fn get_parallel_regions<P: StatePolicy>(parallel_state: P::State) -> &'static [P::State] {
    P::get_parallel_regions(parallel_state)
}

/// W3C SCXML 3.13: Compare two states by document order.
///
/// Returns `true` if `state1` appears before `state2` in document order.
///
/// Ports C++ `ParallelStateHelper::compareDocumentOrder`.
pub fn compare_document_order<P: StatePolicy>(state1: P::State, state2: P::State) -> bool {
    P::get_document_order(state1) < P::get_document_order(state2)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ParallelExitEntryHelper: exit/entry order computation
// ═══════════════════════════════════════════════════════════════════════════════

/// Check if `ancestor` is an ancestor of `descendant`.
///
/// Walks up from `descendant` via `get_parent` until `ancestor` is found or
/// root is reached.
fn is_ancestor<P: StatePolicy>(ancestor: P::State, descendant: P::State) -> bool {
    let mut current = descendant;
    loop {
        match P::get_parent(current) {
            None => return false,
            Some(parent) => {
                if parent == ancestor {
                    return true;
                }
                current = parent;
            }
        }
    }
}

/// W3C SCXML 3.13: Compute exit order for active states.
///
/// States are exited in exit order:
/// 1. Children before parents
/// 2. Reverse document order for tie-breaking
///
/// Ports C++ `ParallelExitEntryHelper::computeExitOrder`.
pub fn compute_exit_order<P: StatePolicy>(
    active_states: &[P::State],
    target_states: &[P::State],
) -> Vec<P::State> {
    let mut exit_set: Vec<P::State> = Vec::new();

    // Collect all states to exit (those not ancestors of any target)
    for &active_state in active_states {
        let should_exit = !target_states
            .iter()
            .any(|&target| is_ancestor::<P>(active_state, target));

        if should_exit {
            let mut current = active_state;
            loop {
                if !exit_set.contains(&current) {
                    exit_set.push(current);
                }
                match P::get_parent(current) {
                    None => break,
                    Some(parent) => {
                        let is_target_ancestor = target_states
                            .iter()
                            .any(|&target| is_ancestor::<P>(parent, target));
                        if is_target_ancestor {
                            break;
                        }
                        current = parent;
                    }
                }
            }
        }
    }

    // Sort by exit order: children before parents, reverse document order for ties
    exit_set.sort_by(|&a, &b| {
        if is_ancestor::<P>(a, b) {
            std::cmp::Ordering::Greater // a is ancestor of b, b exits first
        } else if is_ancestor::<P>(b, a) {
            std::cmp::Ordering::Less // b is ancestor of a, a exits first
        } else {
            // Neither is ancestor of the other -- reverse document order
            P::get_document_order(b).cmp(&P::get_document_order(a))
        }
    });

    exit_set
}

/// W3C SCXML 3.13: Compute entry order for target states.
///
/// States are entered in entry order:
/// 1. Parents before children
/// 2. Document order for tie-breaking
///
/// Ports C++ `ParallelExitEntryHelper::computeEntryOrder`.
pub fn compute_entry_order<P: StatePolicy>(
    target_states: &[P::State],
    current_states: &[P::State],
) -> Vec<P::State> {
    let current_set: HashSet<P::State> = current_states.iter().copied().collect();
    let mut entry_set: Vec<P::State> = Vec::new();

    for &target in target_states {
        let mut path_to_root: Vec<P::State> = Vec::new();
        let mut current = target;
        loop {
            if !current_set.contains(&current) {
                path_to_root.push(current);
            }
            match P::get_parent(current) {
                None => break,
                Some(parent) => current = parent,
            }
        }

        // Add in reverse order (root to leaf)
        for &state in path_to_root.iter().rev() {
            if !entry_set.contains(&state) {
                entry_set.push(state);
            }
        }
    }

    // Sort by entry order: parents before children, document order for ties
    entry_set.sort_by(|&a, &b| {
        if is_ancestor::<P>(a, b) {
            std::cmp::Ordering::Less // a is ancestor of b, a enters first
        } else if is_ancestor::<P>(b, a) {
            std::cmp::Ordering::Greater // b is ancestor of a, b enters first
        } else {
            P::get_document_order(a).cmp(&P::get_document_order(b))
        }
    });

    entry_set
}

/// W3C SCXML 3.13 + 3.4: Compute exit order for parallel state children.
///
/// Sorts active region states in reverse document order (later states exit first).
///
/// Ports C++ `ParallelExitEntryHelper::computeParallelExitOrder`.
pub fn compute_parallel_exit_order<P: StatePolicy>(
    _parallel_state: P::State,
    active_region_states: &[P::State],
) -> Vec<P::State> {
    let mut exit_order: Vec<P::State> = active_region_states.to_vec();
    exit_order.sort_by(|&a, &b| P::get_document_order(b).cmp(&P::get_document_order(a)));
    exit_order
}

// ═══════════════════════════════════════════════════════════════════════════════
// ParallelTransitionHelper: transition conflict detection
// ═══════════════════════════════════════════════════════════════════════════════

/// W3C SCXML C.1: Transition descriptor for parallel conflict detection.
///
/// Ports C++ `ParallelTransitionHelper::Transition<StateType>`.
#[derive(Debug, Clone)]
pub struct Transition<S> {
    /// Source state.
    pub source: S,
    /// Target states.
    pub targets: Vec<S>,
    /// States exited by this transition.
    pub exit_set: HashSet<S>,
    /// Transition index for `execute_transition_actions`.
    pub transition_index: i32,
    /// Whether the transition has executable content.
    pub has_actions: bool,
    /// W3C SCXML 3.13: Whether transition is `type="internal"`.
    pub is_internal: bool,
    /// W3C SCXML 5.9.2: Whether transition has no target attribute.
    pub is_targetless: bool,
}

impl<S: Default> Default for Transition<S> {
    fn default() -> Self {
        Self {
            source: S::default(),
            targets: Vec::new(),
            exit_set: HashSet::new(),
            transition_index: 0,
            has_actions: false,
            is_internal: false,
            is_targetless: false,
        }
    }
}

/// W3C SCXML 3.13: Compute exit set for a transition.
///
/// Ports C++ `ParallelTransitionHelper::computeExitSet`.
pub fn compute_transition_exit_set<P: StatePolicy>(
    transition: &Transition<P::State>,
) -> HashSet<P::State> {
    let mut exit_set = HashSet::new();

    // W3C SCXML 5.9.2: Targetless transitions have empty exit set
    if transition.is_targetless {
        return exit_set;
    }

    // W3C SCXML 3.13: Internal transition to compound descendant
    if transition.is_internal {
        let all_descendants = transition.targets.iter().all(|&target| {
            let source_compound =
                P::is_compound_state(transition.source) && !P::is_parallel_state(transition.source);
            source_compound && P::is_descendant_of(target, transition.source) && target != transition.source
        });

        if all_descendants {
            return exit_set; // Empty -- source stays active
        }
    }

    // External transition: find LCA of source and all targets
    let mut lca: Option<P::State> = None;
    for &target in &transition.targets {
        let current_lca = hierarchy::find_lca::<P>(transition.source, target);
        match (lca, current_lca) {
            (None, cl) => lca = cl,
            (Some(existing), Some(cl)) => {
                lca = hierarchy::find_lca::<P>(existing, cl);
            }
            _ => {}
        }
    }

    // Collect states from source up to (but not including) LCA
    let mut current = transition.source;
    loop {
        exit_set.insert(current);
        match P::get_parent(current) {
            None => break,
            Some(parent) => {
                if let Some(lca_state) = lca {
                    if parent == lca_state {
                        break;
                    }
                }
                current = parent;
            }
        }
    }

    exit_set
}

/// W3C SCXML C.1: Check if two transitions conflict.
///
/// Ports C++ `ParallelTransitionHelper::hasConflict`.
pub fn has_conflict<P: StatePolicy>(
    t1: &Transition<P::State>,
    t2: &Transition<P::State>,
) -> bool {
    // Check if exit sets intersect
    for state in &t1.exit_set {
        if t2.exit_set.contains(state) {
            return true;
        }
    }

    // W3C SCXML 3.13: t1 exits parallel ancestor of t2's source
    for exit_state in &t1.exit_set {
        if P::is_parallel_state(*exit_state) && P::is_descendant_of(t2.source, *exit_state) {
            return true;
        }
    }

    // Reverse: t2 exits parallel ancestor of t1's source
    for exit_state in &t2.exit_set {
        if P::is_parallel_state(*exit_state) && P::is_descendant_of(t1.source, *exit_state) {
            return true;
        }
    }

    false
}

/// W3C SCXML C.1: Select optimal non-conflicting transition set.
///
/// Ports C++ `ParallelTransitionHelper::selectOptimalTransitions`.
pub fn select_optimal_transitions<P: StatePolicy>(
    enabled_transitions: &mut [Transition<P::State>],
) -> Vec<Transition<P::State>> {
    // Compute exit sets
    for transition in enabled_transitions.iter_mut() {
        transition.exit_set = compute_transition_exit_set::<P>(transition);
    }

    // Sort by depth (deeper first -- preemption)
    enabled_transitions.sort_by(|a, b| {
        get_depth::<P>(b.source).cmp(&get_depth::<P>(a.source))
    });

    // Greedy selection
    let mut selected: Vec<Transition<P::State>> = Vec::new();
    for transition in enabled_transitions.iter() {
        let conflicts = selected
            .iter()
            .any(|sel| has_conflict::<P>(transition, sel));
        if !conflicts {
            selected.push(transition.clone());
        }
    }

    selected
}

/// Get hierarchy depth of a state (0 = root).
///
/// Ports C++ `ParallelTransitionHelper::getDepth`.
pub fn get_depth<P: StatePolicy>(state: P::State) -> usize {
    let mut depth = 0;
    let mut current = state;
    while let Some(parent) = P::get_parent(current) {
        depth += 1;
        current = parent;
    }
    depth
}

/// W3C SCXML 3.13: Compute effective LCA considering internal transition semantics.
///
/// Ports C++ `ParallelTransitionHelper::computeEffectiveLCA`.
pub fn compute_effective_lca<P: StatePolicy>(
    source: P::State,
    target: P::State,
    is_internal: bool,
) -> Option<P::State> {
    if is_internal {
        let source_compound =
            P::is_compound_state(source) && !P::is_parallel_state(source);
        if source_compound && P::is_descendant_of(target, source) && target != source {
            return Some(source);
        }
    }
    hierarchy::find_lca::<P>(source, target)
}

/// W3C SCXML Appendix D.2: Compute and sort states to exit for microstep execution.
///
/// Ports C++ `ParallelTransitionHelper::computeStatesToExit`.
pub fn compute_states_to_exit<P: StatePolicy>(
    transitions: &[Transition<P::State>],
    active_states: &[P::State],
) -> Vec<P::State> {
    let mut states_to_exit: Vec<P::State> = Vec::new();

    for trans in transitions {
        if trans.is_targetless || trans.targets.is_empty() {
            continue;
        }

        for &target in &trans.targets {
            let lca = compute_effective_lca::<P>(trans.source, target, trans.is_internal);

            match lca {
                None => {
                    // No LCA: exit from source up to root
                    let mut current = trans.source;
                    loop {
                        let is_active = active_states.contains(&current);
                        if is_active && !states_to_exit.contains(&current) {
                            states_to_exit.push(current);
                        }
                        match P::get_parent(current) {
                            None => break,
                            Some(parent) => current = parent,
                        }
                    }
                }
                Some(lca_state) => {
                    let should_exit_source = !trans.is_internal && trans.source == lca_state;

                    for &active_state in active_states {
                        if active_state == lca_state
                            && !should_exit_source
                        {
                            continue;
                        }

                        let mut should_exit = false;

                        if active_state == lca_state && should_exit_source {
                            should_exit = true;
                        } else {
                            let mut current = active_state;
                            loop {
                                match P::get_parent(current) {
                                    None => break,
                                    Some(parent) => {
                                        if parent == lca_state {
                                            should_exit = true;
                                            break;
                                        }
                                        current = parent;
                                    }
                                }
                            }
                        }

                        if should_exit && !states_to_exit.contains(&active_state) {
                            states_to_exit.push(active_state);
                        }
                    }
                }
            }
        }
    }

    // W3C SCXML 3.13: Sort by REVERSE document order
    states_to_exit.sort_by(|&a, &b| P::get_document_order(b).cmp(&P::get_document_order(a)));

    states_to_exit
}

/// W3C SCXML Appendix D.2: Sort transitions by source state document order.
///
/// Ports C++ `ParallelTransitionHelper::sortTransitionsBySource`.
pub fn sort_transitions_by_source<P: StatePolicy>(
    transitions: &mut [Transition<P::State>],
) {
    transitions.sort_by(|a, b| {
        P::get_document_order(a.source).cmp(&P::get_document_order(b.source))
    });
}

/// W3C SCXML Appendix D.2: Sort transitions by target state document order.
///
/// Ports C++ `ParallelTransitionHelper::sortTransitionsByTarget`.
pub fn sort_transitions_by_target<P: StatePolicy>(
    transitions: &mut [Transition<P::State>],
) {
    transitions.sort_by(|a, b| {
        let target_a = a.targets.first().copied().unwrap_or(a.source);
        let target_b = b.targets.first().copied().unwrap_or(b.source);
        P::get_document_order(target_a).cmp(&P::get_document_order(target_b))
    });
}

/// W3C SCXML 3.13: Sort states for exit by depth then reverse document order.
///
/// Ports C++ `ParallelTransitionHelper::sortStatesForExit`.
pub fn sort_states_for_exit<P: StatePolicy>(states: &mut [P::State]) {
    states.sort_by(|&a, &b| {
        let depth_a = get_depth::<P>(a);
        let depth_b = get_depth::<P>(b);
        if depth_a != depth_b {
            depth_b.cmp(&depth_a) // Deeper states exit first
        } else {
            P::get_document_order(b).cmp(&P::get_document_order(a)) // Later states exit first
        }
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
// ParallelCompletionHelper: all-regions-final check
// ═══════════════════════════════════════════════════════════════════════════════

/// W3C SCXML 3.4: Check if all child regions of a parallel state are in final states.
///
/// A parallel state is complete when ALL child regions have at least one active
/// final state.
///
/// Ports C++ `ParallelCompletionHelper::areAllRegionsInFinal`.
pub fn are_all_regions_in_final<P: StatePolicy>(
    parallel_state: P::State,
    active_states: &[P::State],
) -> bool {
    let regions = P::get_parallel_regions(parallel_state);
    if regions.is_empty() {
        return false;
    }

    for &region in regions {
        let region_has_final = active_states.iter().any(|&active| {
            P::get_parent(active) == Some(region) && P::is_final_state(active)
        });

        if !region_has_final {
            return false;
        }
    }

    true
}

// ═══════════════════════════════════════════════════════════════════════════════
// ParallelConfigurationHelper: configuration tracking
// ═══════════════════════════════════════════════════════════════════════════════

/// W3C SCXML 3.4: Configuration for tracking active states in parallel regions.
///
/// Ports C++ `ParallelConfigurationHelper::Configuration<StateType>`.
#[derive(Debug, Clone)]
pub struct Configuration<S: Copy + Eq + std::hash::Hash> {
    /// Map from region ID to active state in that region.
    pub region_states: HashMap<S, S>,
}

impl<S: Copy + Eq + std::hash::Hash> Configuration<S> {
    /// Construct an empty configuration.
    pub fn new() -> Self {
        Self {
            region_states: HashMap::new(),
        }
    }

    /// Check if a state is in the configuration.
    pub fn contains(&self, state: S) -> bool {
        self.region_states.values().any(|&v| v == state)
    }

    /// Set the active state for a region.
    pub fn set_region_state(&mut self, region: S, state: S) {
        self.region_states.insert(region, state);
    }

    /// Get the active state in a region.
    pub fn get_region_state(&self, region: S) -> Option<S> {
        self.region_states.get(&region).copied()
    }

    /// Remove a region from the configuration.
    pub fn remove_region(&mut self, region: S) {
        self.region_states.remove(&region);
    }

    /// Get all active states across all regions.
    pub fn get_all_active_states(&self) -> Vec<S> {
        self.region_states.values().copied().collect()
    }

    /// Clear all regions.
    pub fn clear(&mut self) {
        self.region_states.clear();
    }

    /// Number of active regions.
    pub fn len(&self) -> usize {
        self.region_states.len()
    }

    /// Whether the configuration is empty.
    pub fn is_empty(&self) -> bool {
        self.region_states.is_empty()
    }
}

impl<S: Copy + Eq + std::hash::Hash> Default for Configuration<S> {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ParallelProcessingAlgorithms: region enter/exit/broadcast
// ═══════════════════════════════════════════════════════════════════════════════

/// Trait for parallel state management adapters.
///
/// Ports the C++ template parameter contracts used by
/// `ParallelProcessingAlgorithms`. Implementors handle region-level operations
/// (enter, exit, event broadcast, final-state checks).
pub trait ParallelStateManager<S, E> {
    /// Enter a region to its initial state.
    fn enter_region(&mut self, region: S);
    /// Exit a region.
    fn exit_region(&mut self, region: S);
    /// Process an event in a specific region. Returns `true` if a transition was taken.
    fn process_region_event(&mut self, region: S, event: &E) -> bool;
    /// Check if a region is in a final state.
    fn is_region_in_final_state(&self, region: S) -> bool;
}

/// W3C SCXML 3.4: Enter all regions of a parallel state.
///
/// Ports C++ `ParallelProcessingAlgorithms::enterAllRegions`.
pub fn enter_all_regions<S, E>(
    manager: &mut dyn ParallelStateManager<S, E>,
    regions: &[S],
) where
    S: Copy,
{
    for &region in regions {
        manager.enter_region(region);
    }
}

/// W3C SCXML D.1: Broadcast event to all active parallel regions.
///
/// Returns `true` if any region took a transition.
///
/// Ports C++ `ParallelProcessingAlgorithms::broadcastEventToRegions`.
pub fn broadcast_event_to_regions<S, E>(
    manager: &mut dyn ParallelStateManager<S, E>,
    event: &E,
    active_regions: &[S],
) -> bool
where
    S: Copy,
{
    let mut any_transition = false;
    for &region in active_regions {
        if manager.process_region_event(region, event) {
            any_transition = true;
        }
    }
    any_transition
}

/// W3C SCXML 3.4: Check if all regions are in final states.
///
/// Ports C++ `ParallelProcessingAlgorithms::areAllRegionsInFinalState`.
pub fn are_all_regions_in_final_state<S, E>(
    manager: &dyn ParallelStateManager<S, E>,
    regions: &[S],
) -> bool
where
    S: Copy,
{
    for &region in regions {
        if !manager.is_region_in_final_state(region) {
            return false;
        }
    }
    true
}

/// W3C SCXML 3.4: Exit all regions of a parallel state.
///
/// Exits in reverse order (matching C++ reverse iterator pattern).
///
/// Ports C++ `ParallelProcessingAlgorithms::exitAllRegions`.
pub fn exit_all_regions<S, E>(
    manager: &mut dyn ParallelStateManager<S, E>,
    regions: &[S],
) where
    S: Copy,
{
    // W3C SCXML: Exit in reverse document order
    for &region in regions.iter().rev() {
        manager.exit_region(region);
    }
}
