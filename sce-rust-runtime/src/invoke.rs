// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 6.4: Invoke lifecycle management — Single Source of Truth
//!
//! 1:1 port of C++ `sce/include/core/InvokeHelper.h`.
//!
//! # Defer / Cancel / Execute Pattern
//!
//! W3C SCXML 6.4 requires that invoke elements in states entered-but-not-exited
//! during a macrostep are executed at the **end** of that macrostep:
//!
//! 1. **Entry** → [`defer_invoke`]: add to pending list
//! 2. **Exit**  → [`cancel_invokes_for_state`]: remove from pending list
//! 3. **Macrostep end** → [`execute_pending_invokes`]: execute all remaining
//!
//! This prevents invoking in states that are immediately exited (test 422).

use std::fmt::Debug;

/// W3C SCXML 6.4: Pending invoke structure for defer/cancel/execute pattern.
///
/// Matches C++ `PendingInvoke` struct in generated code.
#[derive(Debug, Clone)]
pub struct PendingInvoke<S: Copy + PartialEq + Debug> {
    /// W3C SCXML 6.4.1: Runtime-generated invoke identifier
    /// Format: "stateid.platformid.index" (e.g., "s01.140234567890._invoke_0")
    pub invoke_id: String,
    /// State containing the invoke element
    pub state: S,
}

/// W3C SCXML 6.4: Active child session tracking.
///
/// Matches C++ `ChildSession` struct in generated code.
#[derive(Debug, Clone)]
pub struct ChildSession {
    /// Child's session ID (format: "parentSessionId.invokeId")
    pub session_id: String,
    /// Invoke element ID (e.g., "_invoke_0")
    pub invoke_id: String,
    /// Parent's session ID
    pub parent_session_id: String,
    /// W3C SCXML 6.4.6: Autoforward events to child
    pub autoforward: bool,
    /// W3C SCXML 6.5: Finalize handler script content
    pub finalize_script: String,
}

/// W3C SCXML 6.4: Defer invoke execution until macrostep end.
///
/// 1:1 port of C++ `InvokeHelper::deferInvoke`.
pub fn defer_invoke<S: Copy + PartialEq + Debug>(
    pending: &mut Vec<PendingInvoke<S>>,
    invoke: PendingInvoke<S>,
) {
    log::debug!("InvokeHelper: Deferred invoke {}", invoke.invoke_id);
    pending.push(invoke);
}

/// W3C SCXML 6.4: Cancel pending invokes for exited state.
///
/// 1:1 port of C++ `InvokeHelper::cancelInvokesForState`.
/// Uses retain (inverse of C++ remove_if + erase).
pub fn cancel_invokes_for_state<S: Copy + PartialEq + Debug>(
    pending: &mut Vec<PendingInvoke<S>>,
    state: S,
) {
    let before = pending.len();
    pending.retain(|p| {
        if p.state == state {
            log::debug!("InvokeHelper: Cancelled pending invoke {}", p.invoke_id);
            false
        } else {
            true
        }
    });
    if pending.len() < before {
        log::debug!(
            "InvokeHelper: Cancelled {} pending invokes for state",
            before - pending.len()
        );
    }
}

/// W3C SCXML 6.4: Execute all pending invokes at macrostep end.
///
/// 1:1 port of C++ `InvokeHelper::executePendingInvokes`.
/// Copy-and-clear pattern prevents iterator invalidation during execution
/// (executing an invoke may trigger events that modify the pending list).
pub fn execute_pending_invokes<S: Copy + PartialEq + Debug, F>(
    pending: &mut Vec<PendingInvoke<S>>,
    mut executor: F,
) where
    F: FnMut(&PendingInvoke<S>),
{
    if pending.is_empty() {
        return;
    }

    log::debug!("InvokeHelper: Executing {} pending invokes", pending.len());

    // W3C SCXML 6.4: Take pending list to prevent iterator invalidation
    let invokes_to_execute = std::mem::take(pending);

    for invoke_info in &invokes_to_execute {
        log::debug!("InvokeHelper: Starting invoke {}", invoke_info.invoke_id);
        executor(invoke_info);
    }
}

/// W3C SCXML 6.3.1: Create done.invoke event name.
///
/// 1:1 port of C++ `InvokeHelper::createDoneInvokeEventName`.
pub fn create_done_invoke_event_name(invoke_id: &str) -> String {
    format!("done.invoke.{}", invoke_id)
}

/// W3C SCXML 3.12.1: Validate invoke ID format.
///
/// 1:1 port of C++ `InvokeHelper::isValidInvokeId`.
pub fn is_valid_invoke_id(invoke_id: &str) -> bool {
    !invoke_id.is_empty()
}

/// W3C SCXML 6.4: Get count of pending invokes.
pub fn get_pending_count<S: Copy + PartialEq + Debug>(pending: &[PendingInvoke<S>]) -> usize {
    pending.len()
}

/// W3C SCXML 6.4: Check if specific invoke is pending.
pub fn is_invoke_pending<S: Copy + PartialEq + Debug>(
    pending: &[PendingInvoke<S>],
    invoke_id: &str,
) -> bool {
    pending.iter().any(|p| p.invoke_id == invoke_id)
}
