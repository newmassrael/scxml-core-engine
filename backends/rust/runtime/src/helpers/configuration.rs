// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-3.11: is a state chain a configuration THIS document can hold?
//!
//! [`Engine::get_active_states`](crate::Engine::get_active_states) publishes the
//! configuration a machine is in, and [`Engine::enter_at`](crate::Engine::enter_at)
//! is the door that takes one back. This module owns the question between them,
//! and it is the only place that asks it.
//!
//! ## Why this rejects instead of panicking
//!
//! Every other input to this engine is authored: the generator wrote the policy,
//! and a hierarchy that does not walk is a generator defect, which is why the
//! chain builders in [`hierarchy`](super::hierarchy) panic on one. A restored
//! configuration is the single exception — it arrives from OUTSIDE the process,
//! read back by a host from wherever it persisted it, recorded against a
//! document revision that may since have moved. A host holding a stale record
//! has to be able to handle that answer, so this returns
//! [`ConfigurationRejection`] and the engine hands it on.
//!
//! ## What cannot be wrong, and why it is not checked
//!
//! A chain member is a `P::State` — a generated enum, one variant per state in
//! this document. A state this document does not have therefore cannot be
//! constructed, so "no such state" needs no rejection variant: the type refuses
//! it before this function is reached.
//!
//! ## What is checked
//!
//! - the chain is not empty and names nothing twice;
//! - it is ancestor-closed, and closes on exactly one root;
//! - §scxml-3.11: a compound member holds exactly ONE active child — this is
//!   what refuses two siblings of one region;
//! - §scxml-3.11: a `<parallel>` member holds ALL of its regions, because they
//!   are "simultaneously active when the parent element is active" — this is
//!   what refuses a chain with a region missing;
//! - an atomic member holds no children;
//! - the claimed current state is an atomic member of the chain.
//!
//! Together these admit exactly the chains some run of this document could have
//! published, and refuse every chain that would leave the engine in a
//! configuration it could not have reached on its own.

use super::hierarchy::StateChain;
use crate::policy::StatePolicy;

/// Why a chain was refused as a configuration of this document.
///
/// Each variant names the state it tripped on so a host can say which one.
/// `Debug` prints the GENERATED Rust identifier for a state, which is not the
/// word the document uses — pass the state to
/// [`StatePolicy::get_state_name`]
/// to render it in the document's own vocabulary, which is the vocabulary a host
/// persisted it under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationRejection<S> {
    /// The chain holds nothing. A configuration always holds at least a root.
    Empty,
    /// A state appears more than once.
    Duplicate {
        /// The state named twice.
        state: S,
    },
    /// A state's parent is absent, so the chain is not ancestor-closed.
    AncestorMissing {
        /// The state whose parent is missing.
        state: S,
        /// The parent the hierarchy declares and the chain does not hold.
        parent: S,
    },
    /// The chain closes on the wrong number of roots. Exactly one is a
    /// configuration, more than one is two disjoint trees, and zero is
    /// unreachable while the chain is ancestor-closed and non-empty.
    RootCount {
        /// How many chain members have no parent.
        found: usize,
    },
    /// §scxml-3.11: a compound state holds the wrong number of active children.
    /// Exactly one is a configuration; more than one is the "two siblings of one
    /// region" shape.
    CompoundChildCount {
        /// The compound state.
        parent: S,
        /// How many of its children the chain holds.
        found: usize,
    },
    /// §scxml-3.11: a `<parallel>` state's region is absent, so the chain is not
    /// a configuration in which every region is simultaneously active.
    ParallelRegionMissing {
        /// The parallel state.
        parallel: S,
        /// The region it declares and the chain does not hold.
        region: S,
    },
    /// §scxml-3.11: a `<parallel>` state holds a number of children that is not
    /// its region count. A parallel state is entered with all of its regions and
    /// nothing else.
    ParallelChildCount {
        /// The parallel state.
        parallel: S,
        /// How many of its children the chain holds.
        found: usize,
        /// How many regions it declares.
        regions: usize,
    },
    /// A state is atomic yet the chain gives it children.
    AtomicHasChildren {
        /// The atomic state the chain gave children to.
        state: S,
    },
    /// The claimed current state is not in the chain at all.
    CurrentNotActive {
        /// The state the caller claimed the machine is at.
        current: S,
    },
    /// The claimed current state is in the chain but is not atomic, so it is not
    /// a state a settled machine can be "at".
    CurrentNotAtomic {
        /// The state the caller claimed the machine is at.
        current: S,
    },
}

/// Whether `configuration` is a configuration of `P`'s document, with `current`
/// as its current state.
///
/// Pure: reads the policy's static hierarchy and nothing else, allocates
/// nothing, and is therefore the same function under std and `no_std`.
///
/// Cost is quadratic in the chain length, which is bounded by
/// [`MAX_HIERARCHY_DEPTH`](super::hierarchy::MAX_HIERARCHY_DEPTH) under `no_std`
/// and by the document's depth under std. A configuration is a handful of states
/// and this runs once per restore, so the shape is chosen for being obviously
/// right rather than for being fast.
pub fn validate<P: StatePolicy>(
    configuration: &StateChain<P::State>,
    current: P::State,
) -> Result<(), ConfigurationRejection<P::State>> {
    if configuration.is_empty() {
        return Err(ConfigurationRejection::Empty);
    }

    // Nothing twice. Checked first because every count below would otherwise
    // read a duplicate as a second child and blame the wrong rule.
    for (i, &state) in configuration.iter().enumerate() {
        if configuration[..i].contains(&state) {
            return Err(ConfigurationRejection::Duplicate { state });
        }
    }

    // Ancestor-closed, closing on exactly one root.
    let mut roots = 0usize;
    for &state in configuration.iter() {
        match P::get_parent(state) {
            None => roots += 1,
            Some(parent) => {
                if !configuration.contains(&parent) {
                    return Err(ConfigurationRejection::AncestorMissing { state, parent });
                }
            }
        }
    }
    if roots != 1 {
        return Err(ConfigurationRejection::RootCount { found: roots });
    }

    // Per-member child arity: the rule that separates a configuration from a
    // set of states that merely all exist.
    for &state in configuration.iter() {
        let children = configuration
            .iter()
            .filter(|&&s| P::get_parent(s) == Some(state))
            .count();

        if P::is_parallel_state(state) {
            // §scxml-3.4: every region, simultaneously.
            let regions = P::get_parallel_regions(state);
            for &region in regions {
                if !configuration.contains(&region) {
                    return Err(ConfigurationRejection::ParallelRegionMissing {
                        parallel: state,
                        region,
                    });
                }
            }
            if children != regions.len() {
                return Err(ConfigurationRejection::ParallelChildCount {
                    parallel: state,
                    found: children,
                    regions: regions.len(),
                });
            }
        } else if P::is_compound_state(state) {
            // §scxml-3.11: exactly one.
            if children != 1 {
                return Err(ConfigurationRejection::CompoundChildCount {
                    parent: state,
                    found: children,
                });
            }
        } else if children != 0 {
            return Err(ConfigurationRejection::AtomicHasChildren { state });
        }
    }

    // The current state, which is the other half of what the engine publishes.
    if !configuration.contains(&current) {
        return Err(ConfigurationRejection::CurrentNotActive { current });
    }
    if P::is_compound_state(current) || P::is_parallel_state(current) {
        return Err(ConfigurationRejection::CurrentNotAtomic { current });
    }

    Ok(())
}
