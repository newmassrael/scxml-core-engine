// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The data of the event being processed reaches every hook as an argument,
// one value whether a queue owned it or the host lent it (docs/adr/0124),
// held on a hand-written policy so the engine's half is measured without a
// generator.
//
// Topology:
//     idle ──frame [it carries bytes]──▶ open   (content raises `next`)
//     idle ──other──▶ idle
//     open ──next──▶ after
//
// Every hook records the data it was handed, so each test reads which calls
// saw which bytes: the guard, the exit, the content and the entry of the
// event's microstep see its data; the initial entry, an event queued ahead,
// the internal event the content raised and an event that carries none see
// none.

use std::num::NonZeroU32;

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, CurrentEventData, DocumentTables, Engine, EngineState, EventId,
    EventWithMetadata, HistoryId, NoHistory, RunStore, StateId, StateKind, StatePolicy, StateRow,
    Target, TransitionId, TransitionRow, Workspace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LtState {
    Idle,
    Open,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LtEvent {
    Null,
    Frame,
    Other,
    Next,
}

/// One hook call: what ran, and the bytes it was handed as `_event.data`.
type Call = (&'static str, Option<Vec<u8>>);

#[derive(Default)]
struct LtPolicy {
    calls: Vec<Call>,
}

impl LtPolicy {
    fn record(&mut self, what: &'static str, event_data: Option<&[u8]>) {
        self.calls.push((what, event_data.map(<[u8]>::to_vec)));
    }
}

const IDLE_OTHER: TransitionId = TransitionId(0);
const IDLE_FRAME: TransitionId = TransitionId(1);
const OPEN_NEXT: TransitionId = TransitionId(2);

impl StatePolicy for LtPolicy {
    type State = LtState;
    type Event = LtEvent;
    type History = NoHistory;
    // A queue owns the bytes an event carries.
    type Payload = Option<Vec<u8>>;
    // Every reader reads them borrowed, queued or lent alike.
    type EventData<'e> = Option<&'e [u8]>;
    type Hal = sce_rust_runtime::StdHal;
    type InternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ExternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    // It reads no person, and holds none (docs/adr/0116).
    type Principal = sce_rust_runtime::NoPrincipal;
    type ScheduledSendId = sce_rust_runtime::SceString;
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    type Workspace = Workspace<{ state_scratch(3) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(3) }, 0, 0>;
    type Configuration = BoundedVec<LtState, 1>;
    type StartFailure = core::convert::Infallible;

    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[
            StateRow::new(None, StateKind::Atomic, StateId(0), None),
            StateRow::new(None, StateKind::Atomic, StateId(1), None),
            StateRow::new(None, StateKind::Atomic, StateId(2), None),
        ],
        histories: &[],
        transitions: &[
            // idle: other -> idle
            TransitionRow::new(Some(StateId(0)), Target::state(StateId(0)), false, true),
            // idle: frame -> open
            TransitionRow::new(Some(StateId(0)), Target::state(StateId(1)), false, true),
            // open: next -> after
            TransitionRow::new(Some(StateId(1)), Target::state(StateId(2)), false, true),
            // <scxml initial="idle">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(3),
        default_state: StateId(LtState::Idle as u16),
        null_event: EventId(0),
        error_events: &[],
        // Appendix D's sets computed as the appendix writes them, which serves
        // any document (docs/adr/0125).
        walk: &sce_rust_runtime::GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
        candidates: None,
        selection: None,
    };
    const STATES: &'static [LtState] = &[LtState::Idle, LtState::Open, LtState::After];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [LtEvent] =
        &[LtEvent::Null, LtEvent::Frame, LtEvent::Other, LtEvent::Next];

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
            LtEvent::Null => "",
            LtEvent::Frame => "frame",
            LtEvent::Other => "other",
            LtEvent::Next => "next",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "frame" => Some(LtEvent::Frame),
            "other" => Some(LtEvent::Other),
            "next" => Some(LtEvent::Next),
            _ => None,
        }
    }

    fn get_state_name(state: Self::State) -> &'static str {
        match state {
            LtState::Idle => "idle",
            LtState::Open => "open",
            LtState::After => "after",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "idle" => Some(LtState::Idle),
            "open" => Some(LtState::Open),
            "after" => Some(LtState::After),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        LtEvent::Null
    }

    fn event_data<'a>(held: &'a CurrentEventData<'_, Self>) -> Self::EventData<'a> {
        match held {
            CurrentEventData::Queued(payload) => payload.as_deref(),
            CurrentEventData::Lent(event_data) => *event_data,
        }
    }

    // The door holds what the host lends to the bound its schema would state:
    // four bytes (docs/adr/0124, D2).
    fn admit_event_data(
        event_data: Self::EventData<'_>,
    ) -> Result<(), sce_rust_runtime::LentBreach> {
        sce_rust_runtime::LentBreach::within("bytes", event_data.map_or(0, <[u8]>::len), 4)
    }

    fn first_enabled_transition(
        &mut self,
        state: StateId,
        event: EventId,
        event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) -> Option<TransitionId> {
        match (Self::state_of(state), Self::event_of(event)) {
            (LtState::Idle, LtEvent::Other) => Some(IDLE_OTHER),
            // The guard reads the event's data: a `frame` that carries no
            // bytes enables nothing.
            (LtState::Idle, LtEvent::Frame) => {
                self.record("guard frame", event_data);
                event_data.map(|_| IDLE_FRAME)
            }
            (LtState::Open, LtEvent::Next) => Some(OPEN_NEXT),
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        transition: TransitionId,
        event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
    ) {
        match transition {
            IDLE_OTHER => self.record("other", event_data),
            IDLE_FRAME => {
                self.record("frame", event_data);
                engine.raise(EventWithMetadata::new(LtEvent::Next));
            }
            OPEN_NEXT => self.record("next", event_data),
            _ => {}
        }
    }

    fn execute_entry_actions(
        &mut self,
        state: StateId,
        event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
        _is_default_entry: bool,
    ) {
        let what = match Self::state_of(state) {
            LtState::Idle => "enter idle",
            LtState::Open => "enter open",
            LtState::After => "enter after",
        };
        self.record(what, event_data);
    }

    fn execute_exit_actions(
        &mut self,
        state: StateId,
        event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
        if Self::state_of(state) == LtState::Idle {
            self.record("exit idle", event_data);
        }
    }
}

fn started() -> Engine<LtPolicy> {
    let mut engine = Engine::new(LtPolicy::default());
    let Ok(()) = engine.initialize();
    engine
}

fn calls(engine: &Engine<LtPolicy>) -> Vec<Call> {
    engine.policy().calls.clone()
}

fn with(what: &'static str, bytes: &[u8]) -> Call {
    (what, Some(bytes.to_vec()))
}

fn bare(what: &'static str) -> Call {
    (what, None)
}

/// What one `frame` carrying `bytes` makes the machine run, from `idle`.
fn frame_microstep(bytes: &[u8]) -> Vec<Call> {
    vec![
        with("guard frame", bytes),
        with("exit idle", bytes),
        with("frame", bytes),
        with("enter open", bytes),
        // `next` is the next `_event`: what `frame` carried is not its data.
        bare("next"),
        bare("enter after"),
    ]
}

#[test]
fn the_hand_written_tables_keep_the_rules_the_procedures_assume() {
    assert_eq!(LtPolicy::TABLES.check(), Ok(()));
}

#[test]
fn an_event_lends_its_data_while_it_is_processed_and_the_event_it_raised_reads_none() {
    let mut engine = started();
    // The bytes live in this frame only: the borrow ends with the call, which
    // the borrow checker holds, since they are dropped right after it.
    let received = b"SYN".to_vec();
    engine.process_event_with(LtEvent::Frame, Some(received.as_slice()));
    drop(received);

    assert_eq!(engine.get_current_state(), LtState::After);
    let mut expected = vec![bare("enter idle")];
    expected.extend(frame_microstep(b"SYN"));
    assert_eq!(calls(&engine), expected);
}

#[test]
fn a_queued_event_hands_its_owned_payload_to_the_same_readers() {
    let mut engine = started();
    engine.raise_external_typed(LtEvent::Frame, Some(b"SYN".to_vec()));
    engine.step();

    // Owned by the queue or lent by the host, the same readers read the same
    // bytes in the same calls.
    assert_eq!(engine.get_current_state(), LtState::After);
    let mut expected = vec![bare("enter idle")];
    expected.extend(frame_microstep(b"SYN"));
    assert_eq!(calls(&engine), expected);
}

#[test]
fn an_event_that_carries_no_data_hands_none() {
    let mut engine = started();
    // The guard holds only with bytes, so the plain door leaves the machine
    // where it was, and the guard was asked with no data.
    engine.process_event(LtEvent::Frame);
    assert_eq!(engine.get_current_state(), LtState::Idle);
    assert_eq!(
        calls(&engine),
        vec![bare("enter idle"), bare("guard frame")]
    );

    // The same machine takes the same event when its data is lent.
    engine.process_event_with(LtEvent::Frame, Some(&b"x"[..]));
    assert_eq!(engine.get_current_state(), LtState::After);
}

#[test]
fn an_event_queued_ahead_of_the_lent_one_is_handed_its_own_data() {
    let mut engine = started();
    // Queued, not taken: `raise_external_typed` runs no macrostep.
    engine.raise_external_typed(LtEvent::Other, Some(b"q".to_vec()));
    engine.process_event_with(LtEvent::Frame, Some(&b"ack"[..]));

    assert_eq!(engine.get_current_state(), LtState::After);
    let mut expected = vec![
        bare("enter idle"),
        // `other` is taken first, with the payload it was queued with.
        with("exit idle", b"q"),
        with("other", b"q"),
        with("enter idle", b"q"),
    ];
    // Then `frame`, with what was lent.
    expected.extend(frame_microstep(b"ack"));
    assert_eq!(calls(&engine), expected);
}

#[test]
fn a_lent_event_the_budget_does_not_reach_is_taken_later_with_no_data() {
    let mut engine = started();
    engine.set_max_external_events_per_call(NonZeroU32::new(1).expect("one"));
    engine.raise_external(LtEvent::Other, "");
    // The call takes `other` and stops: `frame` stays queued, and its lend
    // ends with the call.
    engine.process_event_with(LtEvent::Frame, Some(&b"late"[..]));
    assert_eq!(engine.get_current_state(), LtState::Idle);
    engine.step();

    assert_eq!(engine.get_current_state(), LtState::Idle);
    assert_eq!(
        calls(&engine),
        vec![
            bare("enter idle"),
            bare("exit idle"),
            bare("other"),
            bare("enter idle"),
            // Taken by the next call, as a delivery that carries no data.
            bare("guard frame"),
        ]
    );
}

/// docs/adr/0124, D2: lent data past what its door holds is refused whole, as
/// a wire delivery past it is. This document declares no `error.execution`, so
/// the delivery is dropped and counted as a discarded external event, no hook
/// is asked of it, and `_event` stays what it was.
#[test]
fn a_lent_value_its_door_refuses_is_dropped_and_counted_as_a_queued_one_is() {
    let mut engine = started();
    engine.process_event_with(LtEvent::Frame, Some(&b"toolong"[..]));

    assert_eq!(engine.get_current_state(), LtState::Idle);
    assert_eq!(calls(&engine), vec![bare("enter idle")]);
    assert_eq!(engine.discarded_external_events(), 1);

    // Within its bound the same event is taken.
    engine.process_event_with(LtEvent::Frame, Some(&b"ack"[..]));
    assert_eq!(engine.get_current_state(), LtState::After);
}

#[test]
fn a_machine_that_is_not_running_refuses_the_lent_event_as_the_plain_door_does() {
    let mut engine = Engine::new(LtPolicy::default());
    engine.process_event_with(LtEvent::Frame, Some(&b"early"[..]));

    assert!(
        calls(&engine).is_empty(),
        "no hook runs before the machine starts"
    );
    assert_eq!(engine.unseen_external_events(), 1);
}
