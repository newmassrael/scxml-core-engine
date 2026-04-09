// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SCE Forge: Procedure types and execution engine for Level 2 procedures.
//
// Generated code implements ProcedurePolicy; this module provides the
// shared types and the event-driven execution loop.

use std::collections::HashMap;

// ── Service types ───────────────────────────────────────────────

/// Request sent to a service handler during procedure execution.
pub struct ProcedureServiceRequest {
    /// Service name from sce:service attribute.
    pub service: String,
    /// Sub-function from sce:subfunc attribute.
    pub subfunc: String,
    /// Additional parameters (addr, payload, etc.).
    pub params: HashMap<String, String>,
}

/// Response received from a service handler.
pub struct ProcedureServiceResponse {
    /// Whether the service call succeeded (raises "ok" or "fail" event).
    pub success: bool,
    /// Response data available as _event.data.
    pub data: String,
}

/// Result of running a procedure to completion.
#[derive(Debug)]
pub struct ProcedureRunResult {
    /// Whether the procedure reached a <final> state.
    pub completed: bool,
    /// Name of the final state reached.
    pub final_state: String,
    /// Parameters from <donedata> in the final state.
    pub done_data: HashMap<String, String>,
}

// ── Procedure policy trait ──────────────────────────────────────

/// Trait that generated Level 2 procedure code implements.
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
    fn done_data(&self) -> &HashMap<String, String>;

    /// Execute entry actions for a state; returns (event, eventData).
    fn execute_entry_actions(&mut self, state: Self::State) -> (Self::Event, String);
    /// Process a transition; returns Some((nextState, trIndex, hasAssigns)) or None.
    fn process_transition(&self, state: Self::State, event: Self::Event) -> Option<(Self::State, usize, bool)>;
    /// Execute <assign> actions for a transition.
    fn execute_transition_actions(&mut self, source: Self::State, tr_index: usize);
}

// ── Execution engine ────────────────────────────────────────────

/// Safety limit for the event loop — prevents infinite loops from misconfigured procedures.
const MAX_ITERATIONS: usize = 1000;

/// Run a procedure to completion using the given policy.
///
/// Drives the state machine from the initial state through service sends
/// until a <final> state is reached or no transition is possible.
pub fn run_procedure<P: ProcedurePolicy>(policy: &mut P) -> ProcedureRunResult {
    let mut current = policy.initial_state();
    let mut event = P::none_event();

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
            policy.execute_transition_actions(current, transition.1);
        }
        current = transition.0;
        event = P::none_event();
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
            P::final_state_name(current).to_string()
        } else {
            String::new()
        },
        done_data: if completed {
            policy.done_data().clone()
        } else {
            HashMap::new()
        },
    }
}
