// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: Procedure kind runtime — shared types and the event-driven
// execution loop that generated `ProcedurePolicy` implementations plug into.
//
// This module is only compiled when the `alloc` feature is enabled because
// procedures inherently require heap-backed strings, byte buffers, and a
// key/value donedata map. The parent crate stays `#![no_std]`; embedded
// targets that cannot afford an allocator simply leave this feature off
// and keep using the pure-algorithm modules (filter, interpolation, ...).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ── Service types ───────────────────────────────────────────────

/// Request sent to a service handler during procedure execution.
///
/// Each field maps 1:1 to a `<send>` attribute in the SCXML source:
///
/// ```xml
/// <send sce:service="Diag" sce:subfunc="0x02"
///       sce:addr="ecuAddr" sce:payload="frame.encode()"/>
/// ```
///
/// The `service` field always carries the dispatch identifier. The other
/// three are `Option` to track attribute presence distinct from empty
/// strings. `payload` is typed as raw bytes (not stringified) because
/// that is its semantic role — a wire-format data blob whose content
/// originates from a codec `encode()` call or equivalent byte producer.
/// `subfunc` and `addr` remain textual representations since the user
/// may reference datamodel variables of any SCE type (u8, u16, u32,
/// u64, String) and the handler-side interpretation varies per protocol.
pub struct ProcedureServiceRequest {
    /// Dispatch identifier (sce:service attribute, always present).
    pub service: String,
    /// Sub-function identifier (sce:subfunc attribute). `None` if absent.
    pub subfunc: Option<String>,
    /// Address identifier (sce:addr attribute). `None` if absent.
    pub addr: Option<String>,
    /// Payload bytes (sce:payload attribute). `None` if absent.
    pub payload: Option<Vec<u8>>,
}

/// Response received from a service handler.
pub struct ProcedureServiceResponse {
    /// Whether the service call succeeded (raises "ok" or "fail" event).
    pub success: bool,
    /// Response data available as _event.data.
    pub data: String,
}

/// Result of running a procedure to completion.
///
/// `done_data` uses `BTreeMap` rather than `HashMap` so that the crate can
/// stay allocator-only (no `std`) and to give deterministic iteration
/// order across runs — which matters for numerical conformance oracles
/// that compare dispatched byte sequences across languages.
#[derive(Debug)]
pub struct ProcedureRunResult {
    /// Whether the procedure reached a <final> state.
    pub completed: bool,
    /// Name of the final state reached.
    pub final_state: String,
    /// Parameters from <donedata> in the final state.
    pub done_data: BTreeMap<String, String>,
}

// ── Procedure policy trait ──────────────────────────────────────

/// Trait that generated procedure code implements.
///
/// Each method corresponds to a section of the generated state machine.
/// The execution loop ([`run_procedure`]) calls these methods to drive
/// the state machine from initial to final state.
pub trait ProcedurePolicy {
    /// State enum type (generated per procedure).
    type State: Copy + PartialEq;
    /// Event enum type (generated per procedure).
    type Event: Copy + PartialEq;

    /// The Event value representing "no event" (eventless transitions).
    fn none_event() -> Self::Event;
    /// Initial state of the procedure.
    fn initial_state(&self) -> Self::State;
    /// Whether the given state is a <final> state.
    fn is_final(state: Self::State) -> bool;
    /// SCXML id of the final state (e.g., "done", "error").
    fn final_state_name(state: Self::State) -> &'static str;

    /// W3C SCXML 5.10: store _event.data for access in guards/actions.
    fn set_pending_event_data(&mut self, data: String);
    /// Collected <donedata> from final states.
    fn done_data(&self) -> &BTreeMap<String, String>;

    /// Execute entry actions for a state; returns (event, eventData).
    fn execute_entry_actions(&mut self, state: Self::State) -> (Self::Event, String);
    /// Process a transition; returns Some((nextState, trIndex, hasAssigns)) or None.
    fn process_transition(
        &self,
        state: Self::State,
        event: Self::Event,
    ) -> Option<(Self::State, usize, bool)>;
    /// Execute `<assign>` actions for a transition.
    ///
    /// Returns `None` for normal flow (transition completes and the loop
    /// advances to the target state). A `Some(Event)` return signals that
    /// an assign-time check (e.g. RFC `claudedocs/rfc-forge-bytes-bounded.md`
    /// §3 B4 bytes cap violation) raised an internal event; the loop
    /// re-processes the *source* state with that event so the fixture's
    /// `<transition event="error.execution">` (or any other internal-event
    /// handler) can pick it up. If the source has no matching transition,
    /// `process_transition` returns `None` and the procedure terminates
    /// uncompleted — W3C-correct for an unhandled internal event.
    fn execute_transition_actions(
        &mut self,
        source: Self::State,
        tr_index: usize,
    ) -> Option<Self::Event>;
}

// ── Execution engine ────────────────────────────────────────────

/// Safety limit for the event loop — prevents infinite loops from
/// misconfigured procedures.
const MAX_ITERATIONS: usize = 1000;

/// Run a procedure to completion using the given policy.
///
/// Drives the state machine from the initial state through service sends
/// until a <final> state is reached or no transition is possible.
pub fn run_procedure<P: ProcedurePolicy>(policy: &mut P) -> ProcedureRunResult {
    let mut current = policy.initial_state();
    let mut event;

    let (e, data) = policy.execute_entry_actions(current);
    event = e;
    if !data.is_empty() {
        policy.set_pending_event_data(data);
    }

    for _ in 0..MAX_ITERATIONS {
        if P::is_final(current) {
            break;
        }
        let transition = match policy.process_transition(current, event) {
            Some(t) => t,
            None => break,
        };
        if transition.2 {
            // Assign-time check may raise an internal event; re-process
            // the source with it instead of advancing to the target.
            if let Some(raised) = policy.execute_transition_actions(current, transition.1) {
                event = raised;
                continue;
            }
        }
        current = transition.0;
        let (e, data) = policy.execute_entry_actions(current);
        event = e;
        if !data.is_empty() {
            policy.set_pending_event_data(data);
        }
    }

    let completed = P::is_final(current);
    ProcedureRunResult {
        completed,
        final_state: if completed {
            String::from(P::final_state_name(current))
        } else {
            String::new()
        },
        done_data: if completed {
            policy.done_data().clone()
        } else {
            BTreeMap::new()
        },
    }
}
