// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0131: a policy's hook is handed the policy as `&mut self` and the
// engine's state as `&mut EngineState<Self>`, two fields of the engine borrowed
// apart. The hooks of this machine write the policy and raise through the state
// in the same call, and read the state while the policy is borrowed, so a run
// of it under Miri (`scripts/gates/forge-rust.sh`) holds the dispatch to the
// aliasing rules: the earlier dispatch handed a `&mut P` into the engine beside
// a `&mut` of the whole engine, and Miri refused the start of every machine.
//
// The machine:
//
//     idle  --go-->  busy
//     idle  --echo-->  (targetless)
//     busy  --echo-->  (targetless)
//
// Entering `idle` raises `echo`; the content of `go` raises `echo` again.

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EngineState, EventId, EventWithMetadata,
    HistoryId, NoHistory, RunStore, StateId, StateKind, StatePolicy, StateRow, Target,
    TransitionId, TransitionRow, Workspace,
};

// Declared in document order, so a variant's discriminant is its `StateId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    Idle,
    Busy,
}

// Declared in the order of `EVENTS`, so a variant's discriminant is its
// `EventId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ev {
    Null,
    Go,
    Echo,
}

/// The written transitions, by `TransitionId`: grouped by source in state
/// order, as the tables number them.
const IDLE_GO: TransitionId = TransitionId(0);
const IDLE_ECHO: TransitionId = TransitionId(1);
const BUSY_ECHO: TransitionId = TransitionId(2);

/// What the hooks wrote into the policy while the state was lent beside it.
#[derive(Default)]
struct SplitPolicy {
    /// Each state entered, in order.
    entered: Vec<St>,
    /// How many states were exited.
    exits: usize,
    /// How many `echo` contents ran.
    echoes: usize,
    /// How many `go` contents ran.
    goes: usize,
    /// For each selection asked, whether the state it was asked for was active
    /// in the configuration the state holds.
    asked_active: Vec<bool>,
}

impl StatePolicy for SplitPolicy {
    type State = St;
    type Event = Ev;
    type History = NoHistory;
    type Payload = ();
    // docs/adr/0124: its events carry no data.
    type EventData<'e> = ();
    type Hal = sce_rust_runtime::StdHal;
    type InternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ExternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // No delayed send, no `<cancel>` and no `<donedata>`.
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // A flat document of two states and no history: one transition is enabled
    // at a time.
    type Workspace = Workspace<{ state_scratch(2) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(2) }, 0, 0>;
    type Configuration = BoundedVec<St, 1>;
    type StartFailure = core::convert::Infallible;

    // Two children of the `<scxml>` element; the document's initial transition,
    // last, enters `idle`.
    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[
            StateRow::new(None, StateKind::Atomic, StateId(0), None),
            StateRow::new(None, StateKind::Atomic, StateId(1), None),
        ],
        histories: &[],
        transitions: &[
            // idle: go -> busy, with content
            TransitionRow::new(Some(StateId(0)), Target::state(StateId(1)), false, true),
            // idle: echo, targetless, with content
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // busy: echo, targetless, with content
            TransitionRow::new(Some(StateId(1)), Target::NONE, false, true),
            // <scxml initial="idle">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(3),
        default_state: StateId(St::Idle as u16),
        null_event: EventId(0),
        error_events: &[],
        // Appendix D's sets computed as the appendix writes them, which serves
        // any document (docs/adr/0125).
        walk: &sce_rust_runtime::GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
    };
    const STATES: &'static [St] = &[St::Idle, St::Busy];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [Ev] = &[Ev::Null, Ev::Go, Ev::Echo];

    fn state_id(state: Self::State) -> StateId {
        StateId(state as u16)
    }

    fn history_id(history: Self::History) -> HistoryId {
        match history {}
    }

    fn event_id(event: Self::Event) -> EventId {
        EventId(event as u16)
    }

    fn null_event() -> Self::Event {
        Ev::Null
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        match event {
            Ev::Null => "",
            Ev::Go => "go",
            Ev::Echo => "echo",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "go" => Some(Ev::Go),
            "echo" => Some(Ev::Echo),
            _ => None,
        }
    }

    fn get_state_name(state: Self::State) -> &'static str {
        match state {
            St::Idle => "idle",
            St::Busy => "busy",
        }
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "idle" => Some(St::Idle),
            "busy" => Some(St::Busy),
            _ => None,
        }
    }

    fn event_data<'a>(
        _held: &'a sce_rust_runtime::CurrentEventData<'_, Self>,
    ) -> Self::EventData<'a> {
    }

    // Writes the policy and raises through the state in one call.
    fn execute_entry_actions(
        &mut self,
        state: StateId,
        _event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
        _is_default_entry: bool,
    ) {
        let state = Self::state_of(state);
        self.entered.push(state);
        if state == St::Idle {
            engine.raise(EventWithMetadata::new(Ev::Echo));
        }
    }

    fn execute_exit_actions(
        &mut self,
        _state: StateId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
        self.exits += 1;
    }

    // Reads the state while the policy is borrowed mutably, and writes what it
    // read into the policy.
    fn first_enabled_transition(
        &mut self,
        state: StateId,
        event: EventId,
        _event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
    ) -> Option<TransitionId> {
        let state = Self::state_of(state);
        self.asked_active.push(engine.is_active(state));
        match (state, Self::event_of(event)) {
            (St::Idle, Ev::Go) => Some(IDLE_GO),
            (St::Idle, Ev::Echo) => Some(IDLE_ECHO),
            (St::Busy, Ev::Echo) => Some(BUSY_ECHO),
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        transition: TransitionId,
        _event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
    ) {
        match transition {
            IDLE_GO => {
                self.goes += 1;
                engine.raise(EventWithMetadata::new(Ev::Echo));
            }
            IDLE_ECHO | BUSY_ECHO => self.echoes += 1,
            _ => {}
        }
    }
}

/// A machine started: its initial configuration entered and the `echo` its
/// entry raised taken.
fn started() -> Engine<SplitPolicy> {
    let mut engine = Engine::new(SplitPolicy::default());
    engine.initialize().unwrap_or_else(|never| match never {});
    engine
}

#[test]
fn the_hand_written_tables_keep_the_rules_the_procedures_assume() {
    assert_eq!(SplitPolicy::TABLES.check(), Ok(()));
}

#[test]
fn a_start_runs_hooks_that_write_the_policy_and_raise_through_the_state() {
    let engine = started();
    let policy = engine.policy();
    assert_eq!(
        policy.entered,
        vec![St::Idle],
        "the entry hook did not record the state it entered"
    );
    assert_eq!(
        policy.echoes, 1,
        "the echo the entry hook raised through the state was not taken"
    );
    assert_eq!(policy.goes, 0, "a go ran that nothing raised");
    assert_eq!(
        engine.get_active_states().as_slice(),
        &[St::Idle],
        "the targetless echo moved the machine"
    );
}

#[test]
fn an_event_runs_hooks_that_write_the_policy_and_raise_through_the_state() {
    let mut engine = started();
    engine.process_event(Ev::Go);
    let policy = engine.policy();
    assert_eq!(policy.exits, 1, "idle was not exited once");
    assert_eq!(policy.goes, 1, "the content of go did not run once");
    assert_eq!(
        policy.entered,
        vec![St::Idle, St::Busy],
        "busy was not entered after idle"
    );
    assert_eq!(
        policy.echoes, 2,
        "the echo go's content raised through the state was not taken in busy"
    );
    assert_eq!(engine.get_active_states().as_slice(), &[St::Busy]);
}

#[test]
fn a_hook_reads_the_configuration_the_state_holds() {
    let mut engine = started();
    engine.process_event(Ev::Go);
    let asked = &engine.policy().asked_active;
    assert!(!asked.is_empty(), "no selection was asked of the policy");
    assert!(
        asked.iter().all(|&active| active),
        "a selection was asked for a state the state did not hold active: {asked:?}"
    );
}
