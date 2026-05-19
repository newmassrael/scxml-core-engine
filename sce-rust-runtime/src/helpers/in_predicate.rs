// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.9.2: `In(stateId)` predicate implementation.
//!
//! 1:1 port of `sce/include/common/InPredicateHelper.h`. Provides the single
//! source of truth for checking whether a named state is currently active.
//!
//! Generic over `P: StatePolicy` -- uses `P::get_state_name` to convert
//! state enum values to their string names for comparison.

use crate::policy::StatePolicy;

/// W3C SCXML 5.9.2: Check if a state (by name) is in the active configuration.
///
/// Walks the `active_states` slice, comparing each state's name (via
/// `P::get_state_name`) against `state_id`.
///
/// Ports C++ `InPredicateHelper::isStateActive`.
///
/// # Arguments
///
/// * `active_states` - Currently active states
/// * `state_id` - State ID string to check for membership
pub fn is_state_active<P: StatePolicy>(active_states: &[P::State], state_id: &str) -> bool {
    active_states
        .iter()
        .any(|&state| P::get_state_name(state) == state_id)
}

/// W3C SCXML 5.9.2: Check if a state (by enum value) is in the active configuration.
///
/// Direct equality check without string comparison -- more efficient for
/// generated code that knows the state enum value at compile time.
pub fn is_state_active_by_value<P: StatePolicy>(
    active_states: &[P::State],
    state: P::State,
) -> bool {
    active_states.contains(&state)
}
