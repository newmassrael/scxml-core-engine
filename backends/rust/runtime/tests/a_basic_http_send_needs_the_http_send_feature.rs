// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//! The BasicHTTP Event I/O Processor is the runtime's `http-send` feature
//! (docs/adr/0053, amendment of 2026-10-21).
//!
//! A build with the feature hands a BasicHTTP send to the host's transport. A
//! `std` build without it has no such processor, so the send names a type the
//! platform does not support and W3C SCXML 6.2 has the processor place
//! `error.execution` on the internal queue: the engine raises it, carrying the
//! send id, and posts nothing, delayed or not.
//!
//! The default build runs the first module; `scripts/gates/nostd-mcu.sh` runs
//! this crate's tests with `--no-default-features --features std`, which runs
//! the second.
#![cfg(feature = "std")]

use std::collections::HashMap;
use std::time::Duration;

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EventId, HistoryId, NoHistory, RunStore,
    StateId, StateKind, StatePolicy, StateRow, Target, TransitionId, TransitionRow, Workspace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HttpState {
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HttpEvent {
    Null,
    ErrorExecution,
}

/// A machine with one state and no transition, so an `error.execution` the
/// engine raises is left unhandled and counted, and the send id of every
/// event the engine takes is kept for the test to read.
#[derive(Default)]
struct HttpPolicy {
    send_ids: Vec<String>,
}

impl StatePolicy for HttpPolicy {
    type State = HttpState;
    type Event = HttpEvent;
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type EventQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    // A delayed BasicHTTP send waits in the delayed-send queue, so this policy
    // keeps one, keyed by the send ids its sends are made under.
    type Scheduler = sce_rust_runtime::PullScheduler<Self::ScheduledSendId>;
    type DoneData = sce_rust_runtime::NoDoneData;
    type Workspace = Workspace<{ state_scratch(1) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(1) }, 0, 0>;
    type Configuration = BoundedVec<HttpState, 1>;

    // One child of the `<scxml>` element and no transition of its own: the
    // only row of the transitions table is the document's initial transition.
    const TABLES: &'static DocumentTables = &DocumentTables {
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
        default_state: StateId(HttpState::Idle as u16),
        null_event: EventId(0),
        // `error.execution`, the one event the engine raises here.
        error_events: &[EventId(1)],
        history: None,
        parallel: None,
    };
    const STATES: &'static [HttpState] = &[HttpState::Idle];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [HttpEvent] = &[HttpEvent::Null, HttpEvent::ErrorExecution];

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
            HttpEvent::Null => "",
            HttpEvent::ErrorExecution => "error.execution",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "error.execution" => Some(HttpEvent::ErrorExecution),
            _ => None,
        }
    }

    fn get_state_name(_state: Self::State) -> &'static str {
        "idle"
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "idle" => Some(HttpState::Idle),
            _ => None,
        }
    }

    fn null_event() -> Self::Event {
        HttpEvent::Null
    }

    fn execute_entry_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
    }

    fn execute_exit_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {}

    fn first_enabled_transition(
        &mut self,
        _state: Self::State,
        _event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<TransitionId> {
        None
    }

    fn execute_transition_content(
        &mut self,
        _transition: TransitionId,
        _engine: &mut Engine<Self>,
    ) {
    }

    fn populate_event_metadata(&mut self, metadata: &sce_rust_runtime::EventMetadata) {
        self.send_ids.push(metadata.send_id.clone());
    }
}

fn running_engine() -> Engine<HttpPolicy> {
    let mut engine = Engine::new(HttpPolicy::default());
    engine.initialize();
    engine
}

/// The send's target. No server is contacted: the transport is the test's own
/// callback, so the host is one RFC 2606 reserves as never resolving, not the
/// W3C fixture endpoint.
const TARGET: &str = "http://transport.invalid/sce";

#[cfg(feature = "http-send")]
mod with_the_processor {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// The send reaches the host's transport with its target and send id, and
    /// nothing is refused.
    #[test]
    fn an_immediate_send_is_handed_to_the_transport() {
        let posted: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
        let seen = Arc::clone(&posted);
        let mut engine = running_engine();
        engine.set_http_send_callback(move |request| {
            seen.lock()
                .expect("the transport's record")
                .push((request.target, request.send_id));
            None
        });
        engine.perform_http_send(
            TARGET.to_string(),
            "ping".to_string(),
            String::new(),
            HashMap::new(),
            "http-1".to_string(),
        );
        engine.step();
        assert_eq!(
            *posted.lock().expect("the transport's record"),
            vec![(TARGET.to_string(), "http-1".to_string())]
        );
        assert_eq!(engine.unhandled_error_events(), 0);
    }

    /// A delayed send is armed for its deadline, and nothing is refused.
    #[test]
    fn a_delayed_send_is_armed() {
        let mut engine = running_engine();
        engine.schedule_http_send(
            TARGET.to_string(),
            "ping".to_string(),
            String::new(),
            HashMap::new(),
            Duration::from_millis(500),
            "http-2".to_string(),
        );
        assert!(engine.time_until_next_scheduled_ms().is_some());
        engine.step();
        assert_eq!(engine.unhandled_error_events(), 0);
    }
}

#[cfg(not(feature = "http-send"))]
mod without_the_processor {
    use super::*;

    /// W3C SCXML 6.2: the type is not supported, so `error.execution` goes on
    /// the internal queue carrying the send id, and nothing is posted.
    #[test]
    fn an_immediate_send_is_refused_with_error_execution() {
        let mut engine = running_engine();
        engine.perform_http_send(
            TARGET.to_string(),
            "ping".to_string(),
            String::new(),
            HashMap::new(),
            "http-1".to_string(),
        );
        engine.step();
        assert_eq!(engine.unhandled_error_events(), 1);
        assert_eq!(
            engine.last_unhandled_error(),
            Some(HttpEvent::ErrorExecution)
        );
        assert_eq!(engine.policy().send_ids, vec!["http-1".to_string()]);
    }

    /// A delay does not make the type supported: the send is refused when it
    /// runs, and no deadline is armed that nothing could perform.
    #[test]
    fn a_delayed_send_is_refused_now_and_nothing_is_armed() {
        let mut engine = running_engine();
        engine.schedule_http_send(
            TARGET.to_string(),
            "ping".to_string(),
            String::new(),
            HashMap::new(),
            Duration::from_millis(500),
            "http-2".to_string(),
        );
        assert_eq!(engine.time_until_next_scheduled_ms(), None);
        engine.step();
        assert_eq!(engine.unhandled_error_events(), 1);
        assert_eq!(engine.policy().send_ids, vec!["http-2".to_string()]);
    }
}
