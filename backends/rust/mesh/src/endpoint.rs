// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The host half of a Rust Mesh endpoint: the [`Router`] core put to work
//! with a transport, a clock and a source of randomness, and joined to an
//! engine.
//!
//! The router decides and this module performs. A transmission that fails
//! is retried on the binding's schedule (§mesh-10.10) or given up as its
//! §16.7 row; an event the router releases, and every row it raises, becomes
//! an [`EngineEvent`] carrying the `_event` fields the C++ core gives the
//! same event (sce/include/mesh/MeshDispatch.h), so a document cannot tell
//! which core it runs beside.
//!
//! Engine-bound events are queued rather than handed back from the send
//! handler. The engine raises a handler's answer with no origin, where the
//! C++ core raises a row as an envelope from its own machine; one queue,
//! drained by [`raise_into`], is what keeps the two cores' `_event`
//! identical for a row observed while sending and one observed while
//! receiving.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::sync::{Arc, Mutex};

use sce_rust_runtime::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE;
use sce_rust_runtime::{Engine, EventMetadata, EventType, HostSendRequest, StatePolicy};

use crate::outbound::{AfterFailure, Attempts};
use crate::router::{Effect, Router, RouterError};
use crate::signal::Signal;

/// The `<send type>` a Mesh send is lowered to.
pub const MESH_PROCESSOR_TYPE: &str = "sce:mesh";

/// `_event.origin` of a Mesh-delivered event is this scheme and the sending
/// machine's name (§mesh-10.7) — the C++ core's `kMeshOriginScheme`.
pub const MESH_ORIGIN_SCHEME: &str = "mesh://";

/// Why a transport declined a transmission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportFailure {
    /// Whether sending the same bytes again could succeed — the transport's
    /// reading of its own failure. A terminal one is given up at once.
    pub retryable: bool,
    /// The transport's own words, carried as the row's `transport_error`.
    pub message: Option<String>,
}

/// Moves envelope bytes to a peer. Bytes that arrive go to
/// [`Endpoint::receive`]; how they arrive is the transport's.
pub trait Transport: Send {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure>;
}

/// What the core leaves to the host: the time, and the randomness an
/// envelope id and a retry's jitter need.
pub trait Environment: Send {
    /// A monotonic clock, in milliseconds.
    fn now_ms(&mut self) -> i64;
    /// A fresh envelope id (§mesh-7.5: a UUID v7).
    fn envelope_id(&mut self) -> [u8; 16];
    /// A non-negative number drawn uniformly, for a retry's jitter.
    fn jitter_draw(&mut self) -> i64;
}

/// An event for the engine's external queue, by name, with its `_event`.
#[derive(Debug, Clone)]
pub struct EngineEvent {
    pub name: String,
    pub metadata: EventMetadata,
}

/// A transmission waiting out its backoff.
struct Retry {
    peer: String,
    bytes: Vec<u8>,
    attempts: Attempts,
    due_ms: i64,
}

/// One machine's Mesh endpoint on a host.
pub struct Endpoint<T, E> {
    router: Router,
    transport: T,
    environment: E,
    retries: Vec<Retry>,
    to_engine: Vec<EngineEvent>,
    host_errors: Vec<RouterError>,
}

impl<T: Transport, E: Environment> Endpoint<T, E> {
    pub fn new(router: Router, transport: T, environment: E) -> Self {
        Self {
            router,
            transport,
            environment,
            retries: Vec::new(),
            to_engine: Vec::new(),
            host_errors: Vec::new(),
        }
    }

    /// Perform a `<send type="sce:mesh">` the engine handed its host.
    pub fn send(&mut self, request: &HostSendRequest) {
        let id = self.environment.envelope_id();
        let now = self.environment.now_ms();
        let effects = self.router.send(request, id, now);
        self.apply(effects);
    }

    /// Envelope `bytes` the transport received from `peer`.
    pub fn receive(&mut self, peer: &str, bytes: Vec<u8>) {
        let now = self.environment.now_ms();
        let effects = self.router.receive(peer, bytes, now);
        self.apply(effects);
    }

    /// `peer`'s transport became ready.
    pub fn peer_ready(&mut self, peer: &str) {
        let now = self.environment.now_ms();
        let effects = self.router.peer_ready(peer, now);
        self.apply(effects);
    }

    /// `peer`'s transport stopped being ready.
    pub fn peer_not_ready(&mut self, peer: &str) {
        let effects = self.router.peer_not_ready(peer);
        self.apply(effects);
    }

    /// End the ordering gaps that have waited out their timeout, and send
    /// again what has waited out its backoff. The host calls this on its own
    /// schedule; nothing else makes time pass for the core.
    pub fn tick(&mut self) {
        let now = self.environment.now_ms();
        let effects = self.router.tick(now);
        self.apply(effects);
        let (due, waiting) = core::mem::take(&mut self.retries)
            .into_iter()
            .partition::<Vec<_>, _>(|retry| retry.due_ms <= now);
        self.retries = waiting;
        for retry in due {
            self.transmit(retry.peer, retry.bytes, retry.attempts);
        }
    }

    /// A transmission the transport accepted and then could not complete —
    /// an envelope still queued on a link that closed (SCE_MESH.md
    /// §mesh-18.3). It is that envelope's first failed send: sent again on
    /// the binding's schedule, or given up as its row.
    pub fn transmit_failed(&mut self, peer: &str, bytes: Vec<u8>, failure: TransportFailure) {
        self.after_failure(peer.to_string(), bytes, Attempts::default(), failure);
    }

    /// The events the engine must now raise, in the order they arose.
    pub fn take_events(&mut self) -> Vec<EngineEvent> {
        core::mem::take(&mut self.to_engine)
    }

    /// What went wrong that only the host can act on — a peer it never
    /// bound, a limit of this receiver. None of these reaches the document
    /// (§mesh-16.7, "Scope of synthesis").
    pub fn take_host_errors(&mut self) -> Vec<RouterError> {
        core::mem::take(&mut self.host_errors)
    }

    fn apply(&mut self, effects: Result<Vec<Effect>, RouterError>) {
        let effects = match effects {
            Ok(effects) => effects,
            Err(error) => return self.host_errors.push(error),
        };
        for effect in effects {
            match effect {
                Effect::Transmit { peer, bytes } => self.transmit(peer, bytes, Attempts::default()),
                Effect::Deliver {
                    event,
                    data,
                    source,
                    send_id,
                } => self.to_engine.push(EngineEvent {
                    name: event,
                    metadata: mesh_metadata(data, &source, send_id.unwrap_or_default()),
                }),
                Effect::Raise { peer, signal } => self.raise(peer.as_deref(), &signal),
            }
        }
    }

    /// Transmit `bytes` to `peer`, the attempts before this one being
    /// `attempts`. A failure waits out the binding's backoff or is given up
    /// as its row.
    fn transmit(&mut self, peer: String, bytes: Vec<u8>, attempts: Attempts) {
        if let Err(failure) = self.transport.transmit(&peer, &bytes) {
            self.after_failure(peer, bytes, attempts, failure);
        }
    }

    /// A send to `peer` failed after `attempts`: wait out the binding's
    /// backoff, or give it up as its row.
    fn after_failure(
        &mut self,
        peer: String,
        bytes: Vec<u8>,
        mut attempts: Attempts,
        failure: TransportFailure,
    ) {
        let policy = self.router.retry_policy(&peer);
        let draw = self.environment.jitter_draw();
        match attempts.after_failure(policy.as_ref(), failure.retryable, failure.message, draw) {
            Ok(AfterFailure::RetryIn { wait_ms }) => {
                let due_ms = self.environment.now_ms().saturating_add(wait_ms);
                self.retries.push(Retry {
                    peer,
                    bytes,
                    attempts,
                    due_ms,
                });
            }
            Ok(AfterFailure::GiveUp(signal)) => self.raise(Some(&peer), &signal),
            Err(error) => self.host_errors.push(RouterError::Rule(error)),
        }
    }

    /// Queue the `error.communication` a row raises. The C++ core raises it
    /// as an envelope from its own machine, so it carries that origin.
    fn raise(&mut self, peer: Option<&str>, signal: &Signal) {
        let data = signal.event_data(self.router.binding(peer));
        let machine = self.router.machine().to_string();
        self.to_engine.push(EngineEvent {
            name: "error.communication".to_string(),
            metadata: mesh_metadata(data, &machine, String::new()),
        });
    }
}

/// The `_event` of an event that arrived over Mesh from `source`, as
/// §mesh-10.7 tabulates it and the C++ core fills it: external, origin
/// `mesh://<source>`, the SCXML processor as origin type (every envelope is
/// SCE's own, so the traffic is inter-SCXML by construction), and the send
/// id the envelope carried.
fn mesh_metadata(data: String, source: &str, send_id: String) -> EventMetadata {
    EventMetadata {
        data,
        event_type: EventType::External,
        send_id,
        origin: alloc::format!("{MESH_ORIGIN_SCHEME}{source}"),
        origin_type: SCXML_EVENT_PROCESSOR_TYPE.to_string(),
        ..EventMetadata::default()
    }
}

/// Raise `events` on `engine`'s external queue, in order. A name the
/// machine does not declare is dropped, as the engine drops any such event.
pub fn raise_into<P: StatePolicy>(engine: &mut Engine<P>, events: Vec<EngineEvent>) {
    for event in events {
        engine.raise_external_by_name_with_meta(&event.name, &event.metadata);
    }
}

/// An endpoint shared by the engine's send handler and the host loop that
/// feeds it receipts and ticks.
pub type SharedEndpoint<T, E> = Arc<Mutex<Endpoint<T, E>>>;

/// Serve `<send type="sce:mesh">` on `engine` with `endpoint`.
///
/// The handler answers the engine with nothing: what a send produces for the
/// document is queued on the endpoint, and the host raises it with
/// [`raise_into`] after the step that sent it.
pub fn register<P, T, E>(engine: &mut Engine<P>, endpoint: &SharedEndpoint<T, E>)
where
    P: StatePolicy,
    T: Transport + 'static,
    E: Environment + 'static,
{
    let endpoint = Arc::clone(endpoint);
    engine.register_event_processor(MESH_PROCESSOR_TYPE, move |request| {
        endpoint
            .lock()
            .expect("a Mesh endpoint is never left mid-update: nothing in it panics while locked")
            .send(&request);
        Vec::new()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbound::Delivery;
    use crate::outbound::RetryPolicy;
    use crate::router::PeerConfig;
    use alloc::collections::VecDeque;
    use alloc::vec;

    /// A transport that records what it sent and fails as told.
    #[derive(Default)]
    struct Recorder {
        sent: Vec<(String, Vec<u8>)>,
        failures: VecDeque<TransportFailure>,
    }

    impl Transport for Recorder {
        fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
            if let Some(failure) = self.failures.pop_front() {
                return Err(failure);
            }
            self.sent.push((peer.to_string(), bytes.to_vec()));
            Ok(())
        }
    }

    /// A clock the test moves, ids that count, and no jitter.
    #[derive(Default)]
    struct Fixed {
        now: i64,
        ids: u8,
    }

    impl Environment for Fixed {
        fn now_ms(&mut self) -> i64 {
            self.now
        }
        fn envelope_id(&mut self) -> [u8; 16] {
            self.ids += 1;
            [self.ids; 16]
        }
        fn jitter_draw(&mut self) -> i64 {
            0
        }
    }

    fn config(retry: Option<RetryPolicy>) -> PeerConfig {
        PeerConfig {
            transport: "wss",
            max_pending: 4,
            max_age_ms: 0,
            retry,
            stamp_sequence: false,
            delivery: Delivery {
                dedup: true,
                ordered: false,
            },
        }
    }

    fn endpoint(
        machine: &str,
        peer: &str,
        retry: Option<RetryPolicy>,
    ) -> Endpoint<Recorder, Fixed> {
        let mut router = Router::new(machine, 8, 50).unwrap();
        router.add_peer(peer, config(retry));
        let mut endpoint = Endpoint::new(router, Recorder::default(), Fixed::default());
        endpoint.peer_ready(peer);
        endpoint
    }

    fn send(event: &str, send_id: &str) -> HostSendRequest {
        HostSendRequest {
            processor_type: MESH_PROCESSOR_TYPE.to_string(),
            event_name: event.to_string(),
            target: "#hmi".to_string(),
            send_id: send_id.to_string(),
            event_data: r#"{"a":1}"#.to_string(),
            ..HostSendRequest::default()
        }
    }

    fn failure(retryable: bool) -> TransportFailure {
        TransportFailure {
            retryable,
            message: Some("reset".to_string()),
        }
    }

    #[test]
    fn a_delivered_event_carries_the_fields_the_cpp_core_gives_it() {
        let mut ecu = endpoint("ecu", "hmi", None);
        let mut hmi = endpoint("hmi", "ecu", None);
        ecu.send(&send("go", "s-7"));
        let (peer, bytes) = ecu.transport.sent.pop().unwrap();
        assert_eq!(peer, "hmi");
        hmi.receive("ecu", bytes);

        let events = hmi.take_events();
        let [event] = events.as_slice() else {
            panic!("expected one event, got {events:?}");
        };
        assert_eq!(event.name, "go");
        assert_eq!(event.metadata.data, r#"{"a":1}"#);
        assert_eq!(event.metadata.event_type, EventType::External);
        assert_eq!(event.metadata.origin, "mesh://ecu");
        assert_eq!(event.metadata.origin_type, SCXML_EVENT_PROCESSOR_TYPE);
        assert_eq!(event.metadata.send_id, "s-7");
    }

    #[test]
    fn a_send_without_an_id_arrives_without_one() {
        let mut ecu = endpoint("ecu", "hmi", None);
        let mut hmi = endpoint("hmi", "ecu", None);
        ecu.send(&send("go", ""));
        let (_, bytes) = ecu.transport.sent.pop().unwrap();
        hmi.receive("ecu", bytes);
        assert_eq!(hmi.take_events()[0].metadata.send_id, "");
    }

    #[test]
    fn a_failed_send_with_no_retry_policy_is_lost_as_send_failed() {
        let mut ecu = endpoint("ecu", "hmi", None);
        ecu.transport.failures.push_back(failure(true));
        ecu.send(&send("go", "s-1"));

        let events = ecu.take_events();
        let [event] = events.as_slice() else {
            panic!("expected one row, got {events:?}");
        };
        assert_eq!(event.name, "error.communication");
        assert_eq!(
            event.metadata.data,
            r#"{"errorName":"communication","reason":"SEND_FAILED","target":"hmi","transport":"wss","transport_error":"reset"}"#
        );
        // Raised as the C++ core raises it: an envelope from its own machine.
        assert_eq!(event.metadata.origin, "mesh://ecu");
        assert_eq!(event.metadata.send_id, "");
    }

    #[test]
    fn a_retryable_failure_is_sent_again_after_its_backoff() {
        let policy = RetryPolicy {
            max_retries: 2,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1000,
            jitter_pct: 0,
        };
        let mut ecu = endpoint("ecu", "hmi", Some(policy));
        ecu.transport.failures.push_back(failure(true));
        ecu.send(&send("go", "s-1"));
        assert!(ecu.transport.sent.is_empty());
        assert!(ecu.take_events().is_empty());

        ecu.environment.now = 99;
        ecu.tick();
        assert!(ecu.transport.sent.is_empty(), "sent before its backoff");

        ecu.environment.now = 100;
        ecu.tick();
        assert_eq!(ecu.transport.sent.len(), 1);
        assert!(ecu.take_events().is_empty());
    }

    #[test]
    fn retries_that_run_out_are_delivery_exhausted() {
        let policy = RetryPolicy {
            max_retries: 1,
            initial_backoff_ms: 10,
            backoff_multiplier: 1.0,
            max_backoff_ms: 10,
            jitter_pct: 0,
        };
        let mut ecu = endpoint("ecu", "hmi", Some(policy));
        ecu.transport
            .failures
            .extend([failure(true), failure(true)]);
        ecu.send(&send("go", "s-1"));
        ecu.environment.now = 10;
        ecu.tick();

        let events = ecu.take_events();
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(
            events[0].metadata.data,
            r#"{"errorName":"communication","reason":"DELIVERY_EXHAUSTED","target":"hmi","transport":"wss","transport_error":"reset","attempts":2}"#
        );
    }

    #[test]
    fn a_terminal_failure_is_not_retried() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_backoff_ms: 10,
            backoff_multiplier: 1.0,
            max_backoff_ms: 10,
            jitter_pct: 0,
        };
        let mut ecu = endpoint("ecu", "hmi", Some(policy));
        ecu.transport.failures.push_back(failure(false));
        ecu.send(&send("go", "s-1"));
        let events = ecu.take_events();
        assert_eq!(events.len(), 1, "{events:?}");
        assert!(events[0].metadata.data.contains(r#""attempts":1"#));
        ecu.environment.now = 1000;
        ecu.tick();
        assert!(ecu.transport.sent.is_empty());
    }

    #[test]
    fn what_only_the_host_can_act_on_never_reaches_the_document() {
        let mut ecu = endpoint("ecu", "hmi", None);
        ecu.receive("stranger", vec![0xFF]);
        assert!(ecu.take_events().is_empty());
        assert_eq!(
            ecu.take_host_errors(),
            vec![RouterError::UnknownPeer("stranger".to_string())]
        );
    }
}
