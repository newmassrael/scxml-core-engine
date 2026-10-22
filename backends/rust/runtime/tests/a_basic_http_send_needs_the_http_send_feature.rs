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
use sce_rust_runtime::{EnabledTransition, Engine, EntryTarget, NoHistory, StatePolicy, WalkStep};

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
    type EventQueue = sce_rust_runtime::EventQueueManager<
        sce_rust_runtime::EventWithMetadata<Self::Event, Self::Payload>,
    >;
    type ScheduledSendId = sce_rust_runtime::SceString;
    type Configuration = BoundedVec<HttpState, 1>;
    type EnabledSet = BoundedVec<EnabledTransition<Self::State, Self::History>, 1>;
    type HistoryDefaults = BoundedVec<(Self::State, Self::History), 0>;
    type EntryWalk = BoundedVec<WalkStep<Self::State, Self::History>, 6>;
    const MAX_CONFIGURATION: usize = 1;
    const MAX_ENABLED: usize = 1;
    const MAX_DEPTH: usize = 1;

    fn initial_state() -> Self::State {
        HttpState::Idle
    }

    fn is_final_state(_state: Self::State) -> bool {
        false
    }

    fn get_parent(_state: Self::State) -> Option<Self::State> {
        None
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
        &[EntryTarget::State(HttpState::Idle)]
    }

    fn get_history_parent(history: Self::History) -> Self::State {
        match history {}
    }

    fn get_history_default_targets(
        history: Self::History,
    ) -> &'static [EntryTarget<Self::State, Self::History>] {
        match history {}
    }

    fn get_document_order(_state: Self::State) -> u32 {
        0
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

    fn history_value(&self, history: Self::History) -> Option<&[Self::State]> {
        match history {}
    }

    fn execute_entry_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
    }

    fn execute_exit_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _before: &[Self::State],
    ) {
    }

    fn first_enabled_transition(
        &mut self,
        _state: Self::State,
        _event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<EnabledTransition<Self::State, Self::History>> {
        None
    }

    fn execute_transition_content(
        &mut self,
        _source: Self::State,
        _index: usize,
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
