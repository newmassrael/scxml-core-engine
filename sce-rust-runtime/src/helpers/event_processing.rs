// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML macrostep event processing algorithms.
//!
//! 1:1 port of `sce/include/core/EventProcessingAlgorithms.h`. Provides the
//! shared event processing logic used by both AOT and interpreter engines.
//!
//! Design principles (matching C++):
//! 1. Algorithm sharing only -- data structures remain engine-specific
//! 2. Generic over adapters -- zero overhead via monomorphization
//! 3. Clear interface contracts via trait bounds

use crate::sce_log_debug;

/// Maximum iterations for eventless transition loops (W3C SCXML 3.13).
///
/// Prevents infinite loops from cyclic eventless transition chains.
pub const MAX_EVENTLESS_ITERATIONS: usize = 100;

/// Adapter trait for event queues used by the processing algorithms.
///
/// Ports the C++ `EventQueueAdapter` concept.
pub trait EventQueueAdapter {
    /// Event type stored in the queue.
    type Event;

    /// Whether the queue has any events.
    fn has_events(&self) -> bool;

    /// Pop the next event (FIFO). Returns `None` if empty.
    fn pop_next(&mut self) -> Option<Self::Event>;
}

/// Adapter trait for state machine instances used by the processing algorithms.
///
/// Ports the implicit C++ template parameter contracts for `StateMachine`.
pub trait StateMachineAdapter {
    /// State type.
    type State: Copy + Eq;
    /// Event type.
    type Event;

    /// Get the current active state.
    fn get_current_state(&self) -> Self::State;

    /// Attempt an eventless transition. Returns `true` if a transition was taken.
    fn process_eventless_transition(&mut self) -> bool;

    /// Execute on-exit actions for a state.
    fn execute_on_exit(&mut self, state: Self::State);

    /// Execute on-entry actions for a state.
    fn execute_on_entry(&mut self, state: Self::State);

    /// Process a transition with an event. Returns `true` if a transition was taken.
    fn process_transition(&mut self, event: &Self::Event) -> bool;
}

/// W3C SCXML 3.12.1: Process internal event queue (FIFO).
///
/// Exhausts all internal events in FIFO order. The `handler` closure processes
/// each event and returns `true` to continue or `false` to stop.
///
/// Ports C++ `EventProcessingAlgorithms::processInternalEventQueue`.
pub fn process_internal_event_queue<Q, F>(queue: &mut Q, mut handler: F)
where
    Q: EventQueueAdapter,
    F: FnMut(Q::Event) -> bool,
{
    while queue.has_events() {
        if let Some(event) = queue.pop_next() {
            if !handler(event) {
                sce_log_debug!(
                    "EventProcessingAlgorithms: Event handler returned false, stopping queue processing"
                );
                break;
            }
        }
    }
}

/// W3C SCXML 3.13: Check eventless transitions until stable.
///
/// Includes maximum iteration limit to prevent infinite loops. Returns `true`
/// if any eventless transition was taken.
///
/// Ports C++ `EventProcessingAlgorithms::checkEventlessTransitions`.
pub fn check_eventless_transitions<SM, Q, F>(
    sm: &mut SM,
    queue: &mut Q,
    process_internal_event: F,
    max_iterations: usize,
) -> bool
where
    SM: StateMachineAdapter,
    Q: EventQueueAdapter,
    F: FnMut(Q::Event) -> bool + Clone,
{
    let mut any_transition = false;
    let mut iterations = 0;

    while iterations < max_iterations {
        let old_state = sm.get_current_state();

        if sm.process_eventless_transition() {
            let new_state = sm.get_current_state();

            if old_state != new_state {
                any_transition = true;
                sm.execute_on_exit(old_state);
                sm.execute_on_entry(new_state);

                // Process internal events after entering new state
                process_internal_event_queue(queue, process_internal_event.clone());
            } else {
                break; // No state change
            }
        } else {
            break; // No eventless transition
        }

        iterations += 1;
    }

    if iterations >= max_iterations {
        log::error!(
            target: "sce",
            "EventProcessingAlgorithms: Eventless transition loop detected after {} iterations",
            max_iterations
        );
        return false;
    }

    any_transition
}
