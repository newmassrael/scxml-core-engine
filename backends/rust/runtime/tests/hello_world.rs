// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EventId, HistoryId, NoHistory, RunStore,
    StateId, StateKind, StatePolicy, StateRow, Target, TransitionId, TransitionRow, Workspace,
};

// Declared in document order, so a variant's discriminant is its `StateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HwState {
    Stopped,
    Running,
    Done,
}

// Declared in the order of `EVENTS`, so a variant's discriminant is its
// `EventId`.
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

/// The written transitions, by `TransitionId`: grouped by source in state
/// order, as the tables number them.
const STOPPED_PLAY: TransitionId = TransitionId(0);
const RUNNING_STOP: TransitionId = TransitionId(1);
const RUNNING_END: TransitionId = TransitionId(2);

impl StatePolicy for HwPolicy {
    type State = HwState;
    type Event = HwEvent;
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // What the generator gives this document: it has no delayed send, no
    // `<cancel>` and no `<donedata>`, so its machine keeps neither a
    // delayed-send queue nor a donedata slot. The tests at the end of this file
    // hold what the engine does with the two.
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // A flat document of three states and no history: the sets of states are
    // bit vectors of three states, one transition is enabled, and the walks
    // owe three steps for its one level and three of headroom.
    type Workspace = Workspace<{ state_scratch(3) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(3) }, 0, 0>;
    type Configuration = BoundedVec<HwState, 1>;

    // Three children of the `<scxml>` element; the document's initial
    // transition, last, enters `stopped`.
    const TABLES: &'static DocumentTables = &DocumentTables {
        states: &[
            StateRow::new(None, StateKind::Atomic, StateId(0), None),
            StateRow::new(None, StateKind::Atomic, StateId(1), None),
            StateRow::new(None, StateKind::Final, StateId(2), None),
        ],
        histories: &[],
        transitions: &[
            // stopped: play -> running
            TransitionRow::new(Some(StateId(0)), Target::state(StateId(1)), false, false),
            // running: stop -> stopped
            TransitionRow::new(Some(StateId(1)), Target::state(StateId(0)), false, false),
            // running: end -> done
            TransitionRow::new(Some(StateId(1)), Target::state(StateId(2)), false, false),
            // <scxml initial="stopped">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        // Every transition has one target, which its row holds.
        targets: &[],
        initial: TransitionId(3),
        default_state: StateId(HwState::Stopped as u16),
        null_event: EventId(0),
        error_events: &[],
        history: None,
        parallel: None,
    };
    const STATES: &'static [HwState] = &[HwState::Stopped, HwState::Running, HwState::Done];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [HwEvent] = &[HwEvent::Null, HwEvent::Play, HwEvent::Stop, HwEvent::End];

    fn state_id(state: Self::State) -> StateId {
        StateId(state as u16)
    }

    fn history_id(history: Self::History) -> HistoryId {
        match history {}
    }

    fn event_id(event: Self::Event) -> EventId {
        EventId(event as u16)
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

    fn execute_exit_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {
        // No exit actions
    }

    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<TransitionId> {
        match (state, event) {
            (HwState::Stopped, HwEvent::Play) => Some(STOPPED_PLAY),
            (HwState::Running, HwEvent::Stop) => Some(RUNNING_STOP),
            (HwState::Running, HwEvent::End) => Some(RUNNING_END),
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        _transition: TransitionId,
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
fn the_hand_written_tables_keep_the_rules_the_procedures_assume() {
    assert_eq!(HwPolicy::TABLES.check(), Ok(()));
}

#[test]
fn initial_state_is_stopped() {
    let engine = Engine::<HwPolicy>::new(HwPolicy::new());
    assert_eq!(engine.get_current_state(), HwState::Stopped);
    assert!(!engine.is_in_final_state());
}

#[test]
fn an_engine_built_in_its_slot_runs_as_one_built_by_value() {
    let mut slot = std::mem::MaybeUninit::<Engine<HwPolicy>>::uninit();
    // A slot that holds something else comes out the same: every field is
    // written and none is read, so what was there is not an input.
    // SAFETY: the slot is writable storage for one engine, and this writes bytes
    // into it and reads nothing.
    unsafe { slot.as_mut_ptr().write_bytes(0xA5, 1) };

    let engine = Engine::new_in_place(&mut slot, HwPolicy::new());
    assert_eq!(engine.get_current_state(), HwState::Stopped);
    assert!(!engine.is_running());
    assert!(!engine.has_ready_events());

    engine.initialize();
    engine.process_event(HwEvent::Play);
    engine.process_event(HwEvent::End);
    assert_eq!(engine.terminal_state(), Some(HwState::Done));
    assert!(engine.is_in_final_state());

    // SAFETY: `new_in_place` initialised the slot, and the engine in it is not
    // used again.
    unsafe { slot.assume_init_drop() };
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

// ──────────────────────────────────────────────
// A machine that keeps no delayed-send queue and no donedata slot
//
// This document has no delayed send, no `<cancel>` and no `<donedata>`, and
// `HwPolicy` names what the generator names for such a document. The engine
// answers every question about the queue as one that holds nothing, and stops
// the program, rather than drop it, when something is armed on it anyway.
// ──────────────────────────────────────────────

#[test]
fn the_queue_and_the_slot_a_document_does_not_use_take_no_space() {
    assert_eq!(std::mem::size_of::<sce_rust_runtime::NoDelayedSends>(), 0);
    assert_eq!(std::mem::size_of::<sce_rust_runtime::NoDoneData>(), 0);
}

#[test]
fn a_machine_with_no_delayed_send_answers_that_nothing_is_pending() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    assert!(!engine.has_ready_events());
    assert_eq!(engine.time_until_next_scheduled_ms(), None);
    assert!(
        !engine.cancel_event("anything"),
        "a cancel on a machine that keeps no queue finds nothing"
    );
    assert_eq!(
        engine.donedata_at_final(),
        "",
        "a machine that keeps no slot reads its donedata as the empty string"
    );

    // A tick drains a queue that holds nothing and runs the macrostep as
    // `step` does.
    engine.tick();
    engine.process_event(HwEvent::Play);
    engine.tick();
    assert_eq!(engine.get_current_state(), HwState::Running);
    assert!(engine.is_running());
}

#[test]
#[should_panic(expected = "keeps no delayed-send queue")]
fn arming_a_delayed_send_on_a_machine_with_no_queue_stops_the_program() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.initialize();
    let _ = engine.schedule_event(
        HwEvent::End,
        std::time::Duration::from_millis(1),
        "",
        "",
        "",
    );
}

#[test]
#[should_panic(expected = "keeps no slot")]
fn keeping_donedata_on_a_machine_with_no_slot_stops_the_program() {
    let mut engine = Engine::<HwPolicy>::new(HwPolicy::new());
    engine.stash_donedata_at_final("{\"r\":1}".to_string());
}

/// A saved state of this document standing in `running`, with `pending` and
/// `hostinvokes` holding what is given.
fn saved_state_with(pending: &str, hostinvokes: &str) -> sce_rust_runtime::saved_state::SavedState {
    let text = format!(
        r#"{{"format":1,"shape":"d","configuration":["running"],"current":"running","variables":{{}},"history":{{}},"pending":[{pending}],"invokes":[],"hostinvokes":[{hostinvokes}],"hostinvoketoken":"0","sendseq":"0","external":[]}}"#
    );
    sce_rust_runtime::saved_state::SavedState::from_json(&text)
        .unwrap_or_else(|refusal| panic!("the saved state reads: {refusal}"))
}

#[test]
fn a_restore_with_nothing_waiting_is_a_state_of_this_document() {
    // The control for the two refusals below: the same state with nothing
    // waiting is accepted, so what they refuse is the waiting entry.
    assert_eq!(
        sce_rust_runtime::saved_state::judge::<HwPolicy>(&saved_state_with("", "")),
        Ok(())
    );
}

#[test]
fn a_restore_that_lists_a_waiting_send_is_refused_by_a_machine_with_no_queue() {
    let saved = saved_state_with(
        r#"{"due":"1","act":"raise","event":"end","data":"","sendid":"","origin":""}"#,
        "",
    );
    let refusal = sce_rust_runtime::saved_state::judge::<HwPolicy>(&saved)
        .expect_err("a machine with no delayed-send queue has no send waiting");
    assert!(
        refusal.reason().contains("pending[0]")
            && refusal.reason().contains("keeps no delayed-send queue"),
        "{refusal}"
    );
}

#[test]
fn a_restore_that_lists_a_deadline_is_refused_by_a_machine_with_no_queue() {
    let saved = saved_state_with(
        "",
        r#"{"type":"x-host","id":"h","src":"","params":{},"data":"","content":"","due":"1"}"#,
    );
    let refusal = sce_rust_runtime::saved_state::judge::<HwPolicy>(&saved)
        .expect_err("a machine with no delayed-send queue has no deadline armed");
    assert!(
        refusal.reason().contains("hostinvokes[0]")
            && refusal.reason().contains("keeps no delayed-send queue"),
        "{refusal}"
    );
}
