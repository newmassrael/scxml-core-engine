// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.9: Guard condition evaluation (static guard variant).
//!
//! 1:1 port of `sce/include/common/GuardHelper.h`. The C++ version evaluates
//! guard expressions via `IScriptEngine`. This Rust port provides:
//!
//! 1. The `In()` predicate-based guard (no script engine needed)
//! 2. A placeholder for script-engine-based guards (deferred to script phase)
//!
//! W3C SCXML 5.9: "If a conditional expression cannot be evaluated as a boolean
//! value or if its evaluation causes an error, the SCXML processor MUST treat
//! the expression as if it evaluated to 'false' AND place error.execution in
//! the internal event queue."

use crate::helpers::in_predicate;
use crate::policy::StatePolicy;

/// W3C SCXML 5.9.2: Evaluate a static guard based on the `In()` predicate.
///
/// Checks if the named state is in the active configuration. This is the
/// only guard type that can be evaluated without a script engine.
///
/// Returns `true` if the state is active, `false` otherwise.
pub fn evaluate_in_predicate<P: StatePolicy>(
    active_states: &[P::State],
    state_id: &str,
) -> bool {
    in_predicate::is_state_active::<P>(active_states, state_id)
}

/// W3C SCXML 5.9.2: Evaluate a negated `In()` predicate guard.
///
/// Returns `true` if the state is NOT active.
pub fn evaluate_not_in_predicate<P: StatePolicy>(
    active_states: &[P::State],
    state_id: &str,
) -> bool {
    !in_predicate::is_state_active::<P>(active_states, state_id)
}

/// W3C SCXML 5.9: Result of a guard evaluation.
///
/// - `Some(true)`: guard passed, transition should be taken
/// - `Some(false)`: guard failed, transition should not be taken
/// - `None`: evaluation error -- caller must raise `error.execution` and
///   treat as `false`
pub type GuardResult = Option<bool>;

/// W3C SCXML 5.9: Combine multiple guard results (logical AND).
///
/// All guards must pass for the combined result to be `true`. If any guard
/// returns `None` (evaluation error), the combined result is `None`.
pub fn combine_guards(results: &[GuardResult]) -> GuardResult {
    let mut combined = true;
    for result in results {
        match result {
            None => return None, // Evaluation error propagates
            Some(false) => combined = false,
            Some(true) => {}
        }
    }
    Some(combined)
}
