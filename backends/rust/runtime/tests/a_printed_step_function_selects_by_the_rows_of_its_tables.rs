// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0127 and docs/adr/0129: a document printed as its own step function
// reads the same selection as the shared interpreter. Its arms name the states
// a selection asks, the atomic state first and each ancestor outward, and each
// state is answered by `DocumentTables::first_enabled` over the selection rows
// of its tables, which asks the policy's `guard_holds` for each transition the
// event enables. So a printed policy writes no `first_enabled_transition`,
// exactly as an interpreted one does not.
//
// One hand-written policy is run in both forms: `Doc<false>` for the shared
// interpreter, over tables that carry the general walk, and `Doc<true>` as its
// own step function, over the same tables carrying `NO_WALK`. Both carry the
// same selection rows, and neither writes `first_enabled_transition`, whose
// default stops the program. Every hook records its call, so the two forms are
// held to each other call for call.
//
// Topology:
//     session (compound, initial idle)
//         idle ──go [never]──▶ done
//         idle ──go──▶ busy            (content)
//         busy ──stop──▶ idle
//         busy ──(eventless) [never]──▶ done
//     session ──finish──▶ done
//     done (final)

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EngineState, Entered, EventId, HistoryId,
    NoHistory, RunStore, SelectionRows, StateId, StateKind, StatePolicy, StateRow, StepFunction,
    StepFunctionForm, Steps, Target, TransitionId, TransitionRow, TriggerEvent, TriggerRow,
    WalkProcedures, Workspace, GENERAL_WALK, NO_WALK,
};

// Declared in document order, so a variant's discriminant is its `StateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DocState {
    Session,
    Idle,
    Busy,
    Done,
}

// Declared in the order of `EVENTS`, the "no event" last, as a generated
// machine numbers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DocEvent {
    Go,
    Stop,
    Finish,
    Null,
}

const SESSION: StateId = StateId(0);
const IDLE: StateId = StateId(1);
const BUSY: StateId = StateId(2);
const DONE: StateId = StateId(3);

const GO: EventId = EventId(0);
const STOP: EventId = EventId(1);
const FINISH: EventId = EventId(2);
const NULL: EventId = EventId(3);

// The written transitions, grouped by source in state order, then the initial
// transitions, as the generator numbers them.
const SESSION_FINISH: TransitionId = TransitionId(0);
const IDLE_GO_NEVER: TransitionId = TransitionId(1);
const IDLE_GO: TransitionId = TransitionId(2);
const BUSY_STOP: TransitionId = TransitionId(3);
const BUSY_EVENTLESS_NEVER: TransitionId = TransitionId(4);

const STATE_ROWS: &[StateRow] = &[
    StateRow::new(None, StateKind::Compound, BUSY, Some(TransitionId(5))),
    StateRow::new(Some(SESSION), StateKind::Atomic, IDLE, None),
    StateRow::new(Some(SESSION), StateKind::Atomic, BUSY, None),
    StateRow::new(None, StateKind::Final, DONE, None),
];

const TRANSITION_ROWS: &[TransitionRow] = &[
    // session: finish -> done
    TransitionRow::new(Some(SESSION), Target::state(DONE), false, false),
    // idle: go [never] -> done
    TransitionRow::new(Some(IDLE), Target::state(DONE), false, false),
    // idle: go -> busy, with content
    TransitionRow::new(Some(IDLE), Target::state(BUSY), false, true),
    // busy: stop -> idle
    TransitionRow::new(Some(BUSY), Target::state(IDLE), false, false),
    // busy: eventless [never] -> done
    TransitionRow::new(Some(BUSY), Target::state(DONE), false, false),
    // session's initial: idle
    TransitionRow::new(Some(SESSION), Target::state(IDLE), false, false),
    // <scxml initial="session">
    TransitionRow::new(None, Target::state(SESSION), false, false),
];

// Each state's first written transition, and one past the last state.
const FIRST_WRITTEN: &[TransitionId] = &[
    TransitionId(0),
    TransitionId(1),
    TransitionId(3),
    TransitionId(5),
    TransitionId(5),
];

const TRIGGERS: &[TriggerRow] = &[
    TriggerRow::at(0),
    TriggerRow::at(2),
    TriggerRow::at(2),
    TriggerRow::at(4),
    TriggerRow::at(6),
    TriggerRow::NONE,
    TriggerRow::NONE,
];

const TRIGGER_EVENTS: &[TriggerEvent] = &[
    TriggerEvent::count(1),
    TriggerEvent::new(FINISH),
    TriggerEvent::count(1),
    TriggerEvent::new(GO),
    TriggerEvent::count(1),
    TriggerEvent::new(STOP),
    TriggerEvent::count(1),
    TriggerEvent::new(NULL),
];

/// The document's tables, walked by `walk`: the same rows and the same
/// selection in both forms.
const fn tables(walk: &'static WalkProcedures) -> DocumentTables<'static> {
    DocumentTables {
        states: STATE_ROWS,
        histories: &[],
        transitions: TRANSITION_ROWS,
        targets: &[],
        initial: TransitionId(6),
        default_state: SESSION,
        null_event: NULL,
        error_events: &[],
        walk,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
        selection: Some(SelectionRows {
            first_written: FIRST_WRITTEN,
            triggers: TRIGGERS,
            trigger_events: TRIGGER_EVENTS,
        }),
    }
}

const INTERPRETED_TABLES: DocumentTables<'static> = tables(&GENERAL_WALK);
const PRINTED_TABLES: DocumentTables<'static> = tables(&NO_WALK);

/// The document, printed as its own step function when `PRINTED` and for the
/// shared interpreter otherwise. Every hook writes its call to `calls`.
struct Doc<const PRINTED: bool> {
    calls: Vec<String>,
}

impl<const PRINTED: bool> Doc<PRINTED> {
    fn new() -> Self {
        Self { calls: Vec::new() }
    }

    fn record(&mut self, call: String) {
        self.calls.push(call);
    }
}

impl<const PRINTED: bool> StatePolicy for Doc<PRINTED> {
    type State = DocState;
    type Event = DocEvent;
    type History = NoHistory;
    type Payload = ();
    // docs/adr/0124: its events carry no data.
    type EventData<'e> = ();
    type Hal = sce_rust_runtime::StdHal;
    type InternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ExternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ScheduledSendId = sce_rust_runtime::SceString;
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // Sized for the shared interpreter's walks over four states and two
    // levels, with room to spare; the printed form borrows only the enabled
    // transition of it.
    type Workspace = Workspace<{ state_scratch(4) }, 1, 12, 0>;
    type RunStore = RunStore<{ state_words(4) }, 0, 0>;
    type Configuration = BoundedVec<DocState, 2>;
    type StartFailure = core::convert::Infallible;

    const TABLES: &'static DocumentTables<'static> = if PRINTED {
        &PRINTED_TABLES
    } else {
        &INTERPRETED_TABLES
    };
    const STATES: &'static [DocState] = &[
        DocState::Session,
        DocState::Idle,
        DocState::Busy,
        DocState::Done,
    ];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [DocEvent] = &[
        DocEvent::Go,
        DocEvent::Stop,
        DocEvent::Finish,
        DocEvent::Null,
    ];
    const STEP_FUNCTION: Option<StepFunctionForm<Self>> = if PRINTED {
        Some(StepFunctionForm::of())
    } else {
        None
    };

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
            DocEvent::Go => "go",
            DocEvent::Stop => "stop",
            DocEvent::Finish => "finish",
            DocEvent::Null => "",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "go" => Some(DocEvent::Go),
            "stop" => Some(DocEvent::Stop),
            "finish" => Some(DocEvent::Finish),
            _ => None,
        }
    }

    fn get_state_name(state: Self::State) -> &'static str {
        match state {
            DocState::Session => "session",
            DocState::Idle => "idle",
            DocState::Busy => "busy",
            DocState::Done => "done",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "session" => Some(DocState::Session),
            "idle" => Some(DocState::Idle),
            "busy" => Some(DocState::Busy),
            "done" => Some(DocState::Done),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        DocEvent::Null
    }

    fn event_data<'a>(
        _held: &'a sce_rust_runtime::CurrentEventData<'_, Self>,
    ) -> Self::EventData<'a> {
    }

    fn execute_entry_actions(
        &mut self,
        state: StateId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
        is_default_entry: bool,
    ) {
        let name = Self::get_state_name(Self::state_of(state));
        self.record(format!("enter {name} by default {is_default_entry}"));
    }

    fn execute_exit_actions(
        &mut self,
        state: StateId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
        let name = Self::get_state_name(Self::state_of(state));
        self.record(format!("exit {name}"));
    }

    // The only judge of a guard, in both forms: asked by the selection rows
    // for each transition the event enables, until one holds.
    fn guard_holds(
        &mut self,
        transition: TransitionId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) -> bool {
        self.record(format!("guard {}", transition.index()));
        transition != IDLE_GO_NEVER && transition != BUSY_EVENTLESS_NEVER
    }

    fn execute_transition_content(
        &mut self,
        transition: TransitionId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
        self.record(format!("content {}", transition.index()));
    }
}

/// The step function the generator would print for this document: the arms
/// of `sce_build::step_function::StepFunction::of`, written out. Its selection
/// names the states asked and asks each of them `Steps::enabled`.
impl<const PRINTED: bool> StepFunction for Doc<PRINTED> {
    fn enter_initial_configuration<M: Steps>(machine: &mut M) -> Entered {
        machine.enter(SESSION, true);
        machine.enter(IDLE, false);
        Entered {
            states: 2,
            current: Some(IDLE),
            ended_in: None,
        }
    }

    fn select_transition<M: Steps>(
        machine: &mut M,
        atomic: StateId,
        event: EventId,
    ) -> Option<TransitionId> {
        match atomic {
            IDLE => {
                if let Some(transition) = machine.enabled(IDLE, event) {
                    return Some(transition);
                }
                machine.enabled(SESSION, event)
            }
            BUSY => {
                if let Some(transition) = machine.enabled(BUSY, event) {
                    return Some(transition);
                }
                machine.enabled(SESSION, event)
            }
            _ => None,
        }
    }

    fn microstep<M: Steps>(machine: &mut M, atomic: StateId, transition: TransitionId) -> Entered {
        let ended = Entered {
            states: 1,
            current: Some(DONE),
            ended_in: Some(DONE),
        };
        match (transition, atomic) {
            (SESSION_FINISH | IDLE_GO_NEVER, IDLE) => {
                machine.exit(IDLE);
                machine.exit(SESSION);
                machine.enter(DONE, false);
                ended
            }
            (SESSION_FINISH | BUSY_EVENTLESS_NEVER, BUSY) => {
                machine.exit(BUSY);
                machine.exit(SESSION);
                machine.enter(DONE, false);
                ended
            }
            (IDLE_GO, IDLE) => {
                machine.exit(IDLE);
                machine.content(IDLE_GO);
                machine.enter(BUSY, false);
                Entered {
                    states: 1,
                    current: Some(BUSY),
                    ended_in: None,
                }
            }
            (BUSY_STOP, BUSY) => {
                machine.exit(BUSY);
                machine.enter(IDLE, false);
                Entered {
                    states: 1,
                    current: Some(IDLE),
                    ended_in: None,
                }
            }
            _ => Entered {
                states: 0,
                current: None,
                ended_in: None,
            },
        }
    }

    fn exit_configuration<M: Steps>(machine: &mut M, atomic: StateId) {
        match atomic {
            IDLE | BUSY => {
                machine.exit(atomic);
                machine.exit(SESSION);
            }
            DONE => machine.exit(DONE),
            _ => {}
        }
    }
}

/// What one form observed: every hook call, and after each event the
/// configuration.
fn run<const PRINTED: bool>(events: &[DocEvent]) -> (Vec<String>, Option<DocState>) {
    let mut engine = Engine::new(Doc::<PRINTED>::new());
    engine.initialize().unwrap_or_else(|never| match never {});
    let mut observed = Vec::new();
    for &event in events {
        engine.process_event(event);
        let configuration: Vec<&str> = engine
            .get_active_states()
            .iter()
            .map(|&state| Doc::<PRINTED>::get_state_name(state))
            .collect();
        observed.push(format!("after {event:?}: {configuration:?}"));
    }
    let mut calls = engine.policy().calls.clone();
    calls.extend(observed);
    (calls, engine.terminal_state())
}

/// Where `call` stands in `calls`.
fn position(calls: &[String], call: &str) -> usize {
    calls
        .iter()
        .position(|made| made == call)
        .unwrap_or_else(|| panic!("{call:?} was made: {calls:?}"))
}

const DRIVE: &[DocEvent] = &[DocEvent::Stop, DocEvent::Go, DocEvent::Finish];

#[test]
fn both_forms_carry_tables_that_keep_the_rules() {
    assert_eq!(Doc::<false>::TABLES.check(), Ok(()));
    assert_eq!(Doc::<true>::TABLES.check(), Ok(()));
    assert!(Doc::<false>::STEP_FUNCTION.is_none());
    assert!(Doc::<true>::STEP_FUNCTION.is_some());
}

#[test]
fn the_printed_form_selects_by_the_rows_and_asks_the_guards() {
    let (calls, ended) = run::<true>(DRIVE);
    assert_eq!(ended, Some(DocState::Done), "{calls:?}");
    // `go` enables both of idle's transitions: the guarded one is asked first,
    // in the order written, fails, and the next is asked and taken.
    assert_eq!(
        calls.iter().filter(|call| *call == "guard 1").count(),
        1,
        "{calls:?}"
    );
    assert!(position(&calls, "guard 1") < position(&calls, "guard 2"));
    assert!(position(&calls, "guard 2") < position(&calls, "content 2"));
    // In busy the eventless transition is asked with the "no event".
    assert!(position(&calls, "enter busy by default false") < position(&calls, "guard 4"));
    // `finish` enables nothing of busy's own, so the selection goes on to its
    // ancestor, whose transition ends the run.
    assert!(position(&calls, "guard 4") < position(&calls, "guard 0"));
    assert!(position(&calls, "exit busy") < position(&calls, "exit session"));
    assert!(position(&calls, "exit session") < position(&calls, "enter done by default false"));
    // `stop` was sent only in idle, where nothing it enables is written, so
    // busy's `stop` transition was never asked.
    assert!(!calls.iter().any(|call| call == "guard 3"), "{calls:?}");
}

#[test]
fn the_printed_form_runs_as_the_shared_interpreter_does() {
    let (interpreted, interpreted_end) = run::<false>(DRIVE);
    let (printed, printed_end) = run::<true>(DRIVE);
    assert_eq!(printed, interpreted);
    assert_eq!(printed_end, interpreted_end);
}
