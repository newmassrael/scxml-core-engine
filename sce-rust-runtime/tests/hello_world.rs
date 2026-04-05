// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Hand-crafted minimal state machine exercising the Engine lifecycle.
//
// Topology:
//     Stopped ──Play──▶ Running
//     Running ──Stop──▶ Stopped
//     Running ──End──▶ Done (final)
//
// This is the Phase 1 acceptance gate #1: if Engine + StatePolicy can be
// implemented by hand for a trivial SM and drive transitions correctly, the
// trait shape is viable for template-driven generation.

use sce_rust_runtime::{Engine, StatePolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HwState {
    Stopped,
    Running,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HwEvent {
    Null,
    Play,
    Stop,
    End,
}

struct HwPolicy {
    last_transition_is_internal: bool,
    last_transition_is_targetless: bool,
    last_transition_source_state: HwState,
}

impl HwPolicy {
    fn new() -> Self {
        Self {
            last_transition_is_internal: false,
            last_transition_is_targetless: false,
            last_transition_source_state: HwState::Stopped,
        }
    }
}

impl StatePolicy for HwPolicy {
    type State = HwState;
    type Event = HwEvent;

    fn initial_state() -> Self::State {
        HwState::Stopped
    }

    fn is_final_state(state: Self::State) -> bool {
        matches!(state, HwState::Done)
    }

    fn get_parent(_state: Self::State) -> Option<Self::State> {
        None // All states are flat (no hierarchy)
    }

    fn is_compound_state(_state: Self::State) -> bool {
        false
    }

    fn is_descendant_of(_desc: Self::State, _anc: Self::State) -> bool {
        false
    }

    fn get_document_order(state: Self::State) -> u32 {
        match state {
            HwState::Stopped => 0,
            HwState::Running => 1,
            HwState::Done => 2,
        }
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        match event {
            HwEvent::Null => "",
            HwEvent::Play => "play",
            HwEvent::Stop => "stop",
            HwEvent::End => "end",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "play" => Some(HwEvent::Play),
            "stop" => Some(HwEvent::Stop),
            "end" => Some(HwEvent::End),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        HwEvent::Null
    }

    fn last_transition_is_internal(&self) -> bool {
        self.last_transition_is_internal
    }
    fn set_last_transition_is_internal(&mut self, v: bool) {
        self.last_transition_is_internal = v;
    }
    fn last_transition_is_targetless(&self) -> bool {
        self.last_transition_is_targetless
    }
    fn set_last_transition_is_targetless(&mut self, v: bool) {
        self.last_transition_is_targetless = v;
    }
    fn last_transition_source_state(&self) -> Self::State {
        self.last_transition_source_state
    }
    fn set_last_transition_source_state(&mut self, s: Self::State) {
        self.last_transition_source_state = s;
    }

    fn execute_entry_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {
        // No entry actions in this minimal test
    }

    fn execute_exit_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _pre: &[Self::State],
    ) {
        // No exit actions
    }

    fn process_transition(
        &mut self,
        current_state: &mut Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> bool {
        // Record the source state before any change
        self.last_transition_source_state = *current_state;
        self.last_transition_is_internal = false;
        self.last_transition_is_targetless = false;

        match (*current_state, event) {
            (HwState::Stopped, HwEvent::Play) => {
                *current_state = HwState::Running;
                true
            }
            (HwState::Running, HwEvent::Stop) => {
                *current_state = HwState::Stopped;
                true
            }
            (HwState::Running, HwEvent::End) => {
                *current_state = HwState::Done;
                true
            }
            _ => false,
        }
    }

    fn execute_transition_actions(&mut self, _engine: &mut Engine<Self>) {
        // No transition actions
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[test]
fn initial_state_is_stopped() {
    let engine = Engine::<HwPolicy>::new(HwPolicy::new());
    assert_eq!(engine.get_current_state(), HwState::Stopped);
    assert!(!engine.is_in_final_state());
}

#[test]
fn initialize_enters_initial_state() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    assert_eq!(engine.get_current_state(), HwState::Stopped);
    assert!(engine.is_running());
}

#[test]
fn play_transitions_to_running() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    engine.process_event(HwEvent::Play);
    assert_eq!(engine.get_current_state(), HwState::Running);
    assert!(!engine.is_in_final_state());
}

#[test]
fn full_lifecycle_stopped_running_done() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    assert_eq!(engine.get_current_state(), HwState::Stopped);

    engine.process_event(HwEvent::Play);
    assert_eq!(engine.get_current_state(), HwState::Running);

    engine.process_event(HwEvent::End);
    assert_eq!(engine.get_current_state(), HwState::Done);
    assert!(engine.is_in_final_state());
}

#[test]
fn stop_returns_to_stopped_from_running() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    engine.process_event(HwEvent::Play);
    engine.process_event(HwEvent::Stop);
    assert_eq!(engine.get_current_state(), HwState::Stopped);
}

#[test]
fn unmatched_event_is_ignored() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    // Stopped has no handler for Stop
    engine.process_event(HwEvent::Stop);
    assert_eq!(engine.get_current_state(), HwState::Stopped);
}

#[test]
fn stop_method_halts_engine() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    engine.stop();
    assert!(!engine.is_running());
    // After stop, process_event should be a no-op
    engine.process_event(HwEvent::Play);
    assert_eq!(engine.get_current_state(), HwState::Stopped);
}
