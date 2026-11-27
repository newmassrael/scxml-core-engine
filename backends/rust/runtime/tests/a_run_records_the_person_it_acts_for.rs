// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0116, D7: who did it is recorded by the platform. A fact a macrostep says into the
// event journal and a host-run invocation it starts are recorded as done for the person the
// external event that began the macrostep was said for, the internal events it raises included,
// and for nobody in a macrostep begun by an event said for nobody; never as a document wrote it.
// The machine is written by hand, so that what these cases read is the runtime.
//
// The machine: one state, `s`, whose targetless transitions take `say` (a fact, and the internal
// event `said`) and `said` (a fact).

// The journal is the `std` profile's.
#![cfg(feature = "std")]

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::journal::JournalEntry;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EngineState, EventId, EventMetadata,
    EventType, EventWithMetadata, HistoryId, NoHistory, Principal, RunStore, StateId, StateKind,
    StatePolicy, StateRow, Target, TransitionId, TransitionRow, Workspace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    S,
}

// Declared in the order of `EVENTS`, so a variant's discriminant is its `EventId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ev {
    Null,
    Say,
    Said,
}

/// `s` on `say`, targetless.
const S_SAY: TransitionId = TransitionId(0);
/// `s` on `said`, targetless.
const S_SAID: TransitionId = TransitionId(1);

/// The person a document would write if it could: what every fact the machine says names, and
/// what the engine must write over.
fn forged() -> Box<Principal> {
    Box::new(person("mallory", "m9"))
}

/// `subject` of the issuer these cases name, as the instance `instance` acts for them.
fn person(subject: &str, instance: &str) -> Principal {
    Principal {
        issuer: "https://id.example".to_string(),
        subject: subject.to_string(),
        proved_ms: 1_700_000_000_000,
        method: "test".to_string(),
        instance: instance.to_string(),
    }
}

/// A machine that says a fact on `say` and on the `said` it raises.
struct Teller;

impl StatePolicy for Teller {
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
    // No delayed send, no `<cancel>`, no `<donedata>`, and no deadline.
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // One state and no history: one transition is enabled at a time.
    type Workspace = Workspace<{ state_scratch(1) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(1) }, 0, 0>;
    type Configuration = BoundedVec<St, 1>;
    type StartFailure = core::convert::Infallible;

    // One child of the `<scxml>` element with two targetless transitions that have content;
    // the document's initial transition, last, enters it.
    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[StateRow::new(None, StateKind::Atomic, StateId(0), None)],
        histories: &[],
        transitions: &[
            // s: say, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // s: said, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // <scxml initial="s">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(2),
        default_state: StateId(St::S as u16),
        null_event: EventId(0),
        error_events: &[],
        // Appendix D's sets computed as the appendix writes them, which serves any document
        // (docs/adr/0125).
        walk: &sce_rust_runtime::GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
        candidates: None,
        selection: None,
    };
    const STATES: &'static [St] = &[St::S];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [Ev] = &[Ev::Null, Ev::Say, Ev::Said];

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
            Ev::Say => "say",
            Ev::Said => "said",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "say" => Some(Ev::Say),
            "said" => Some(Ev::Said),
            _ => None,
        }
    }

    fn get_state_name(_state: Self::State) -> &'static str {
        "s"
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "s" => Some(St::S),
            _ => None,
        }
    }

    fn event_data<'a>(
        _held: &'a sce_rust_runtime::CurrentEventData<'_, Self>,
    ) -> Self::EventData<'a> {
    }

    fn execute_entry_actions(
        &mut self,
        _state: StateId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
        _is_default_entry: bool,
    ) {
    }

    fn execute_exit_actions(
        &mut self,
        _state: StateId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
    }

    fn first_enabled_transition(
        &mut self,
        state: StateId,
        event: EventId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) -> Option<TransitionId> {
        match (Self::state_of(state), Self::event_of(event)) {
            (St::S, Ev::Say) => Some(S_SAY),
            (St::S, Ev::Said) => Some(S_SAID),
            _ => None,
        }
    }

    // The content a generated `<send type="sce:journal">` makes, naming a person the engine must
    // write over.
    fn execute_transition_content(
        &mut self,
        transition: TransitionId,
        _event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
    ) {
        let said = |schema: &'static str| JournalEntry {
            schema,
            revision: 1,
            data: Vec::new(),
            valid_ms: None,
            principal: Some(forged()),
        };
        if transition == S_SAY {
            engine.record_journal_entry(said("say"));
            engine.raise(EventWithMetadata::new(Ev::Said));
        } else if transition == S_SAID {
            engine.record_journal_entry(said("said"));
        }
    }
}

/// The metadata of an external event an instance sent, said for `person`.
fn said_for(person: Option<Principal>) -> EventMetadata {
    EventMetadata {
        event_type: EventType::External,
        principal: person.map(Box::new),
        ..EventMetadata::default()
    }
}

/// A running machine standing in `s`.
fn started() -> Engine<Teller> {
    let mut engine = Engine::new(Teller);
    let Ok(()) = engine.initialize();
    engine.step();
    engine
}

/// Raise `event` said for `person` and run it.
fn run(engine: &mut Engine<Teller>, event: &str, person: Option<Principal>) {
    engine.raise_external_by_name_with_meta(event, &said_for(person));
    engine.step();
}

/// The schema and the person of each entry the machine said since the last take.
fn said(engine: &mut Engine<Teller>) -> Vec<(&'static str, Option<String>)> {
    engine
        .take_journal()
        .into_iter()
        .map(|entry| {
            (
                entry.schema,
                entry.principal.map(|person| person.subject.clone()),
            )
        })
        .collect()
}

/// A fact said in a macrostep begun by an event said for a person is recorded as said for that
/// person, the one said on the internal event the macrostep raised among them, whatever the
/// document wrote; one begun by an event said for nobody records nobody, though a person acted
/// before it. Red if `record_journal_entry` keeps what the document wrote, if the person is read
/// from the event being processed (the internal `said` carries none), or if the person of an
/// earlier macrostep is kept past an external event said for nobody.
#[test]
fn a_fact_is_recorded_as_said_for_the_person_the_macrostep_acts_for() {
    let mut engine = started();
    run(&mut engine, "say", Some(person("alice", "a1")));
    assert_eq!(
        said(&mut engine),
        [
            ("say", Some("alice".to_string())),
            ("said", Some("alice".to_string()))
        ]
    );
    run(&mut engine, "say", None);
    assert_eq!(said(&mut engine), [("say", None), ("said", None)]);
    run(&mut engine, "say", Some(person("bob", "b1")));
    let taken = engine.take_journal();
    assert_eq!(
        taken
            .iter()
            .map(|entry| entry.principal.as_deref().cloned())
            .collect::<Vec<_>>(),
        [Some(person("bob", "b1")), Some(person("bob", "b1"))],
        "the whole person, the instance among it, is recorded"
    );
}
