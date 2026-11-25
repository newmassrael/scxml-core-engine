// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0132: a machine's delayed-send queue is sized by what its document
// can leave waiting. The queue names its capacity in its type, a build with
// debug assertions records the most entries it held, a run whose policy carries
// a proven capacity is held to it as each turn closes, and a new run starts with
// nothing waiting, which the proof that a state entered once arms once relies on.

use core::time::Duration;

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EngineState, EventId, HistoryId, NoHistory,
    PullScheduler, RunStore, SceString, StateId, StateKind, StatePolicy, StateRow, Target,
    TransitionId, TransitionRow, Workspace, MAX_SCHEDULED_EVENTS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    S0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ev {
    Null,
    Delayed,
}

/// One state and no transition of its own; the tests arm delayed events on its
/// queue by hand, as a host may, and the policy states the capacity a generator
/// would have proven for a document that leaves one waiting.
struct OneTimerPolicy;

impl StatePolicy for OneTimerPolicy {
    type State = St;
    type Event = Ev;
    type History = NoHistory;
    type Payload = ();
    // docs/adr/0124: its events carry no data.
    type EventData<'e> = ();
    type Hal = sce_rust_runtime::StdHal;
    type InternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ExternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ScheduledSendId = SceString;
    // The capacity of one, as the generated policy writes a proven one.
    type Scheduler = PullScheduler<Self::ScheduledSendId, 1>;
    const SCHEDULER_BOUND: Option<usize> = Some(1);
    type DoneData = sce_rust_runtime::NoDoneData;
    type Workspace = Workspace<{ state_scratch(1) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(1) }, 0, 0>;
    type Configuration = BoundedVec<St, 1>;
    type StartFailure = core::convert::Infallible;

    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[StateRow::new(None, StateKind::Atomic, StateId(0), None)],
        histories: &[],
        transitions: &[TransitionRow::new(
            None,
            Target::state(StateId(0)),
            false,
            false,
        )],
        targets: &[],
        initial: TransitionId(0),
        default_state: StateId(St::S0 as u16),
        null_event: EventId(0),
        error_events: &[],
        walk: &sce_rust_runtime::GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
        selection: None,
    };
    const STATES: &'static [St] = &[St::S0];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [Ev] = &[Ev::Null, Ev::Delayed];

    fn state_id(s: Self::State) -> StateId {
        StateId(s as u16)
    }
    fn history_id(h: Self::History) -> HistoryId {
        match h {}
    }
    fn event_id(e: Self::Event) -> EventId {
        EventId(e as u16)
    }

    fn null_event() -> Self::Event {
        Ev::Null
    }

    fn get_event_name(e: Self::Event) -> &'static str {
        match e {
            Ev::Null => "",
            Ev::Delayed => "delayed",
        }
    }
    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "delayed" => Some(Ev::Delayed),
            _ => None,
        }
    }
    fn get_state_name(_s: Self::State) -> &'static str {
        "s0"
    }
    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "s0" => Some(St::S0),
            _ => None,
        }
    }

    fn event_data<'a>(
        _held: &'a sce_rust_runtime::CurrentEventData<'_, Self>,
    ) -> Self::EventData<'a> {
    }

    fn execute_entry_actions(
        &mut self,
        _s: StateId,
        _event_data: Self::EventData<'_>,
        _eng: &mut EngineState<Self>,
        _is_default_entry: bool,
    ) {
    }
    fn execute_exit_actions(
        &mut self,
        _s: StateId,
        _event_data: Self::EventData<'_>,
        _eng: &mut EngineState<Self>,
    ) {
    }
    fn first_enabled_transition(
        &mut self,
        _s: StateId,
        _e: EventId,
        _event_data: Self::EventData<'_>,
        _eng: &mut EngineState<Self>,
    ) -> Option<TransitionId> {
        None
    }
    fn execute_transition_content(
        &mut self,
        _t: TransitionId,
        _event_data: Self::EventData<'_>,
        _eng: &mut EngineState<Self>,
    ) {
    }
}

fn started() -> Engine<OneTimerPolicy> {
    let mut engine = Engine::<OneTimerPolicy>::new(OneTimerPolicy);
    engine.initialize().unwrap_or_else(|never| match never {});
    engine
}

/// A queue's capacity is part of its type: the one a policy names, and the
/// runtime's default for a queue written by hand with none.
#[test]
fn a_queue_names_its_capacity_in_its_type() {
    assert_eq!(PullScheduler::<SceString, 2>::CAPACITY, 2);
    assert_eq!(
        PullScheduler::<SceString>::CAPACITY,
        MAX_SCHEDULED_EVENTS,
        "a queue that names no capacity keeps the runtime's default"
    );
    assert_eq!(
        <<OneTimerPolicy as StatePolicy>::Scheduler>::CAPACITY,
        1,
        "the policy's queue is of the capacity it names"
    );
}

/// The most entries a queue held at once is what a run left waiting at its
/// peak: an entry cancelled or delivered frees its slot, and the record keeps
/// the peak.
#[cfg(debug_assertions)]
#[test]
fn a_queue_records_the_most_entries_it_held_at_once() {
    let mut queue = PullScheduler::<SceString, 2>::new();
    assert_eq!(queue.high_water(), 0);
    let _ = queue.schedule_event_at(EventId(1), 10, "a", "", "");
    let _ = queue.schedule_event_at(EventId(1), 20, "b", "", "");
    assert_eq!(queue.high_water(), 2);
    assert!(queue.cancel_event("a"));
    let _ = queue.schedule_event_at(EventId(1), 30, "c", "", "");
    assert_eq!(queue.high_water(), 2, "two waited at once, never three");
    assert!(queue.cancel_event("b"));
    assert!(queue.cancel_event("c"));
    assert_eq!(queue.high_water(), 2, "the record keeps the peak");
}

/// A run that never leaves more waiting than the proven capacity closes each
/// turn: one armed, cancelled, and armed again is one at a time.
#[cfg(debug_assertions)]
#[test]
fn a_run_within_its_proven_capacity_closes_every_turn() {
    let mut engine = started();
    engine.schedule_event_as(Ev::Delayed, Duration::from_secs(60), "t", "", "");
    assert!(engine.cancel_event("t"));
    engine.schedule_event_as(Ev::Delayed, Duration::from_secs(60), "t", "", "");
    engine.step();
    assert_eq!(engine.scheduled_high_water(), 1);
}

/// A run that left more waiting than the proven capacity stops a test build
/// as the turn closes, where a std queue would have grown in silence.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "more than the capacity the generator proved")]
fn a_turn_that_closes_past_the_proven_capacity_stops_a_test_build() {
    let mut engine = started();
    engine.schedule_event_as(Ev::Delayed, Duration::from_secs(60), "t1", "", "");
    engine.schedule_event_as(Ev::Delayed, Duration::from_secs(60), "t2", "", "");
    engine.step();
}

/// A run starts with nothing waiting: what an ended run left is not delivered
/// to the next, and is not counted against its capacity.
#[test]
fn a_new_run_starts_with_nothing_waiting() {
    let mut engine = started();
    engine.schedule_event_as(Ev::Delayed, Duration::ZERO, "t", "", "");
    assert!(engine.has_ready_events());
    engine.stop();
    engine.initialize().unwrap_or_else(|never| match never {});
    assert!(
        !engine.has_ready_events(),
        "the event the ended run armed waits in the new run"
    );
    #[cfg(debug_assertions)]
    assert_eq!(engine.scheduled_high_water(), 0);
}
