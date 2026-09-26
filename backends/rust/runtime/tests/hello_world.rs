// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Hand-crafted minimal state machine exercising the Engine lifecycle.
//
// Topology:
//     Stopped ──Play──▶ Running
//     Running ──Stop──▶ Stopped
//     Running ──End──▶ Done (final)
//
// Smoke test: if Engine + StatePolicy can be implemented by hand for a
// trivial SM and drive transitions correctly, the trait shape is viable for
// template-driven generation. Pins that baseline.

use sce_rust_runtime::{EnabledTransition, Engine, EntryTarget, NoHistory, StatePolicy};

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

struct HwPolicy;

impl HwPolicy {
    fn new() -> Self {
        Self
    }
}

/// One targeted transition of `source`, as a policy reports it.
fn to(
    source: HwState,
    target: &'static [EntryTarget<HwState, NoHistory>],
) -> EnabledTransition<HwState, NoHistory> {
    EnabledTransition {
        source,
        targets: target,
        transition_index: 0,
        has_actions: false,
        is_internal: false,
    }
}

impl StatePolicy for HwPolicy {
    type State = HwState;
    type Event = HwEvent;
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;

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

    fn get_child_states(_state: Self::State) -> &'static [Self::State] {
        &[]
    }

    fn get_initial_targets(
        _state: Self::State,
    ) -> &'static [EntryTarget<Self::State, Self::History>] {
        &[]
    }

    fn get_document_initial_targets() -> &'static [EntryTarget<Self::State, Self::History>] {
        &[EntryTarget::State(HwState::Stopped)]
    }

    fn get_history_parent(history: Self::History) -> Self::State {
        match history {}
    }

    fn get_history_default_targets(
        history: Self::History,
    ) -> &'static [EntryTarget<Self::State, Self::History>] {
        match history {}
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

    fn get_state_name(state: Self::State) -> &'static str {
        match state {
            HwState::Stopped => "stopped",
            HwState::Running => "running",
            HwState::Done => "done",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "stopped" => Some(HwState::Stopped),
            "running" => Some(HwState::Running),
            "done" => Some(HwState::Done),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        HwEvent::Null
    }

    fn history_value(&self, history: Self::History) -> Option<&[Self::State]> {
        match history {}
    }

    // `_is_default_entry` is unused: this policy has no compound states, so no
    // state has an initial transition whose content could run.
    fn execute_entry_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
        // No entry actions in this minimal test
    }

    fn execute_exit_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _before: &[Self::State],
    ) {
        // No exit actions
    }

    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<EnabledTransition<Self::State, Self::History>> {
        match (state, event) {
            (HwState::Stopped, HwEvent::Play) => {
                Some(to(state, &[EntryTarget::State(HwState::Running)]))
            }
            (HwState::Running, HwEvent::Stop) => {
                Some(to(state, &[EntryTarget::State(HwState::Stopped)]))
            }
            (HwState::Running, HwEvent::End) => {
                Some(to(state, &[EntryTarget::State(HwState::Done)]))
            }
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        _source: Self::State,
        _index: usize,
        _engine: &mut Engine<Self>,
    ) {
        // No transition actions
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

// Compile-time regression guard: `Engine<P>` must stay `Send` whenever
// `P: Send`, so a host can move an engine across a thread boundary (e.g. into a
// worker spawned with `thread::spawn(move || ...)`). The boxed completion /
// HTTP-send callbacks carry a `+ Send` bound for exactly this reason; dropping
// it would make `Engine<P>` unconditionally `!Send` and fail this assertion at
// compile time. `HwPolicy` is `Send` (plain enum/bool fields), so the only way
// this stops compiling is an engine field losing `Send`.
#[test]
fn engine_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<HwPolicy>();
    assert_send::<Engine<HwPolicy>>();
}

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
    assert_eq!(engine.terminal_state(), Some(HwState::Done));
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
