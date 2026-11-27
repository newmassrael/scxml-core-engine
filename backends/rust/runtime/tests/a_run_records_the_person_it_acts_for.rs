// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0116, D7: who did it is recorded by the platform. A fact a macrostep says into the
// event journal and a host-run invocation it starts are recorded as done for the person the
// external event that began the macrostep was said for, the internal events it raises included,
// and for nobody in a macrostep begun by an event said for nobody; never as a document wrote it.
// A saved state holds no person, so a machine running an invocation started for one refuses the
// save. The machine is written by hand, so that what these cases read is the runtime.
//
// The machine: one state, `s`, whose targetless transitions take `say` (a fact, and the internal
// event `said`), `said` (a fact) and `ask` (a host-run invocation).

// The journal, host-run invocations and a saved state are the `std` profile's.
#![cfg(feature = "std")]

use std::sync::{Arc, Mutex, PoisonError};

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::journal::JournalEntry;
use sce_rust_runtime::saved_state::{self, SavedShape};
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EngineState, EventId, EventMetadata,
    EventType, EventWithMetadata, HistoryId, HostInvokeEvent, HostInvokeRequest, NoHistory,
    Principal, RunStore, StateId, StateKind, StatePolicy, StateRow, Target, TransitionId,
    TransitionRow, Workspace,
};

/// The type the machine's invocation names, which the host serves.
const TYPE: &str = "x-sce-host";
/// The invocation's id.
const JOB: &str = "job";

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
    Ask,
}

/// `s` on `say`, targetless.
const S_SAY: TransitionId = TransitionId(0);
/// `s` on `said`, targetless.
const S_SAID: TransitionId = TransitionId(1);
/// `s` on `ask`, targetless.
const S_ASK: TransitionId = TransitionId(2);

/// The person a document would write if it could: what every fact and every request the machine
/// makes names, and what the engine must write over.
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

/// A machine that says a fact on `say` and on the `said` it raises, and starts `JOB` on `ask`.
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

    const HAS_HOST_INVOKES: bool = true;

    // One child of the `<scxml>` element with three targetless transitions that have content;
    // the document's initial transition, last, enters it.
    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[StateRow::new(None, StateKind::Atomic, StateId(0), None)],
        histories: &[],
        transitions: &[
            // s: say, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // s: said, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // s: ask, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // <scxml initial="s">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(3),
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
    const EVENTS: &'static [Ev] = &[Ev::Null, Ev::Say, Ev::Said, Ev::Ask];

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
            Ev::Ask => "ask",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "say" => Some(Ev::Say),
            "said" => Some(Ev::Said),
            "ask" => Some(Ev::Ask),
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

    // A session id of its own, which a save names.
    fn own_session_id(&self) -> &str {
        "teller"
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
            (St::S, Ev::Ask) => Some(S_ASK),
            _ => None,
        }
    }

    // The content a generated `<send type="sce:journal">` and a generated invoke site make,
    // each naming a person the engine must write over.
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
        } else if transition == S_ASK {
            let started = Engine::perform_host_invoke(
                self,
                engine,
                HostInvokeRequest {
                    processor_type: TYPE.to_string(),
                    invoke_id: JOB.to_string(),
                    principal: Some(forged()),
                    ..HostInvokeRequest::default()
                },
            );
            assert!(started, "the host serves the type the machine invokes");
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

/// A running machine standing in `s`, and the person each start its host's invoker was handed
/// names, in order. The host holds every start: none ends until a case ends it.
fn started() -> (Engine<Teller>, Arc<Mutex<Vec<Option<Principal>>>>) {
    let mut engine = Engine::new(Teller);
    let starts = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&starts);
    engine.register_invoker(TYPE, move |event| {
        if let HostInvokeEvent::Start(request) = event {
            seen.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(request.principal.map(|person| *person));
        }
        None
    });
    let Ok(()) = engine.initialize();
    engine.step();
    (engine, starts)
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
    let (mut engine, _) = started();
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

/// A host-run invocation started in a macrostep is handed the person the macrostep acts for,
/// whatever the document wrote, and none when it acts for nobody; and while one started for a
/// person runs, the machine refuses to be saved, since a restore would start it again for nobody.
/// Red if `start_host_invoke` leaves the request's person as the document wrote it, or if the
/// save holds a machine whose running invocation was started for a person.
#[test]
fn an_invocation_is_started_for_the_person_and_holds_the_save_while_it_runs() {
    let shape = SavedShape {
        first: "shape",
        revised: "shape",
        format2_first: "shape",
        format2_revised: "shape",
        legacy: &[],
        records: &[],
        enums: &[],
        features: None,
        migrations: &[],
    };
    let refused_for_a_person = |engine: &Engine<Teller>| {
        matches!(
            saved_state::save(engine, &shape, Vec::new(), Vec::new(), Vec::new(), 0),
            Err(refusal) if refusal.reason().contains("was started for a person")
        )
    };

    let (mut engine, starts) = started();
    run(&mut engine, "ask", None);
    assert!(
        !refused_for_a_person(&engine),
        "an invocation started for nobody held the save"
    );

    let (mut engine, starts_for_alice) = started();
    run(&mut engine, "ask", Some(person("alice", "a1")));
    assert!(
        refused_for_a_person(&engine),
        "a machine running an invocation started for a person was saved"
    );
    let handed = |starts: &Arc<Mutex<Vec<Option<Principal>>>>| {
        starts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    };
    assert_eq!(handed(&starts), [None]);
    assert_eq!(handed(&starts_for_alice), [Some(person("alice", "a1"))]);
}
