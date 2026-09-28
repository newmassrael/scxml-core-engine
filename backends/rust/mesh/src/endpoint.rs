// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The host half of a Rust Mesh endpoint: the [`Router`] core put to work
//! with a transport, a clock and a source of randomness, and joined to an
//! engine.
//!
//! The router decides and this module performs. A transmission that fails
//! is retried on the binding's schedule (§mesh-10.10) or given up as its
//! §mesh-16.7 row; an event the router releases, and every row it raises, becomes
//! an [`EngineEvent`] carrying the `_event` fields the C++ core gives the
//! same event (sce/include/mesh/MeshDispatch.h), so a document cannot tell
//! which core it runs beside. A Mesh request the router ends becomes the
//! call that ends its invocation (§mesh-9.5).
//!
//! What the engine must be told is queued rather than handed back from the
//! send handler. The engine raises a handler's answer with no origin, where
//! the C++ core raises a row as an envelope from its own machine; one queue,
//! drained by [`apply_to`], is what keeps the two cores' `_event` identical
//! for a row observed while sending and one observed while receiving, and
//! keeps an invocation's end in order with the events around it.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::sync::{Arc, Mutex};

use sce_rust_runtime::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE;
use sce_rust_runtime::{
    Engine, EventMetadata, EventType, HostInvokeEvent, HostInvokeRequest, HostInvokeResponse,
    HostSendRequest, StatePolicy,
};

use crate::outbound::{AfterFailure, Attempts};
use crate::router::{Effect, Invoked, Router, RouterError};
use crate::signal::Signal;

/// The `<send type>` a Mesh send is lowered to — the runtime's constant, so
/// the type this crate serves is the one the engine's router door registers.
pub use sce_rust_runtime::MESH_PROCESSOR_TYPE;

/// The `<invoke type>` a Mesh request is lowered to, and the
/// `_event.origintype` of the event that ends it (§mesh-9.5).
pub use sce_rust_runtime::MESH_RPC_INVOKE_TYPE;

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
    /// The wall clock, in milliseconds since the Unix epoch — what a
    /// request's `deadline_unix_ms` is written in (§mesh-9.5).
    fn now_unix_ms(&mut self) -> u64;
    /// A fresh UUID v7 (§mesh-7.5): an envelope's id, or the wire
    /// `invoke_id` a request is correlated by.
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

/// One thing the host must tell its engine, in the order the endpoint came
/// to know it.
#[derive(Debug, Clone)]
pub enum EngineCall {
    /// Raise this event on the external queue.
    Raise(EngineEvent),
    /// End the start `token` of the Mesh request `invoke_id` — with
    /// `done.invoke`, or with `error.invoke` when `failed` — carrying `data`
    /// as `_event.data`. `source` is the machine that answered, `None` for a
    /// deadline the requester reached itself.
    EndInvoke {
        invoke_id: String,
        token: u64,
        failed: bool,
        data: String,
        source: Option<String>,
    },
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
    to_engine: Vec<EngineCall>,
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

    /// Start an `<invoke type="sce:mesh-rpc">` the engine handed its host,
    /// answering the engine as a host invoker does: with nothing when the
    /// request is on its way — its end is queued later — or with a refusal
    /// when it cannot reach the wire (§mesh-9.5's pre-envelope tier).
    pub fn invoke(&mut self, request: &HostInvokeRequest) -> HostInvokeResponse {
        let wire_id = self.environment.envelope_id();
        let id = self.environment.envelope_id();
        let now = self.environment.now_ms();
        let now_unix = self.environment.now_unix_ms();
        match self.router.invoke(request, wire_id, id, now, now_unix) {
            Ok(Invoked::Started(effects)) => {
                self.apply(Ok(effects));
                HostInvokeResponse::default()
            }
            Ok(Invoked::Refused(data)) => HostInvokeResponse {
                refusal: Some(data),
                ..HostInvokeResponse::default()
            },
            // Only the host can act on it; the document is told the request
            // could not start rather than left waiting on one that never
            // left.
            Err(error) => {
                let refusal = alloc::format!("{error:?}");
                self.host_errors.push(error);
                HostInvokeResponse {
                    refusal: Some(refusal),
                    ..HostInvokeResponse::default()
                }
            }
        }
    }

    /// The state that started the start `token` of the Mesh request
    /// `invoke_id` exited.
    pub fn cancel_invoke(&mut self, invoke_id: &str, token: u64) {
        self.router.cancel_invoke(invoke_id, token);
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

    /// What the engine must now be told, in the order it arose.
    pub fn take_calls(&mut self) -> Vec<EngineCall> {
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
                    invoke_id,
                } => self.to_engine.push(EngineCall::Raise(EngineEvent {
                    name: event,
                    metadata: EventMetadata {
                        // §mesh-10.7: an inbound request's invokeid, which a
                        // reply sent while it is handled carries back.
                        invoke_id: invoke_id.unwrap_or_default(),
                        ..mesh_metadata(data, &source, send_id.unwrap_or_default())
                    },
                })),
                Effect::Raise { peer, signal } => self.raise(peer.as_deref(), &signal),
                Effect::Complete {
                    invoke_id,
                    token,
                    data,
                    source,
                } => self.to_engine.push(EngineCall::EndInvoke {
                    invoke_id,
                    token,
                    failed: false,
                    data,
                    source: Some(source),
                }),
                Effect::Fail {
                    invoke_id,
                    token,
                    data,
                    source,
                } => self.to_engine.push(EngineCall::EndInvoke {
                    invoke_id,
                    token,
                    failed: true,
                    data,
                    source,
                }),
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
        self.to_engine.push(EngineCall::Raise(EngineEvent {
            name: "error.communication".to_string(),
            metadata: mesh_metadata(data, &machine, String::new()),
        }));
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

/// Tell `engine` what `calls` say, in order. An event name the machine does
/// not declare is dropped, as the engine drops any such event; an
/// invocation's end the engine no longer waits for — its state exited, or
/// it was started again — is refused by the engine's own token check.
///
/// An answer carries `_event.origin` `mesh://<source>` and origin type
/// `sce:mesh-rpc` (§mesh-9.5); a deadline the requester reached itself has
/// no peer to name, and carries neither.
pub fn apply_to<P: StatePolicy>(engine: &mut Engine<P>, calls: Vec<EngineCall>) {
    for call in calls {
        match call {
            EngineCall::Raise(event) => {
                engine.raise_external_by_name_with_meta(&event.name, &event.metadata);
            }
            EngineCall::EndInvoke {
                invoke_id,
                token,
                failed,
                data,
                source,
            } => {
                let (origin, origin_type) = match &source {
                    Some(source) => (
                        alloc::format!("{MESH_ORIGIN_SCHEME}{source}"),
                        MESH_RPC_INVOKE_TYPE,
                    ),
                    None => (String::new(), ""),
                };
                let (id, kind) = (invoke_id.as_str(), MESH_RPC_INVOKE_TYPE);
                if failed {
                    engine.fail_host_invoke(kind, id, token, &data, &origin, origin_type);
                } else {
                    engine.complete_host_invoke_from(kind, id, token, &data, &origin, origin_type);
                }
            }
        }
    }
}

/// An endpoint shared by the engine's send handler and the host loop that
/// feeds it receipts and ticks.
pub type SharedEndpoint<T, E> = Arc<Mutex<Endpoint<T, E>>>;

/// Serve `<send type="sce:mesh">` and `<invoke type="sce:mesh-rpc">` on
/// `engine` with `endpoint`.
///
/// The send handler answers the engine with nothing, and a request that
/// starts is answered with nothing too: what either produces for the
/// document is queued on the endpoint, and the host applies it with
/// [`apply_to`] after the step that sent it.
pub fn register<P, T, E>(engine: &mut Engine<P>, endpoint: &SharedEndpoint<T, E>)
where
    P: StatePolicy,
    T: Transport + 'static,
    E: Environment + 'static,
{
    const HELD: &str =
        "a Mesh endpoint is never left mid-update: nothing in it panics while locked";
    let sender = Arc::clone(endpoint);
    engine.register_mesh_router(move |request| {
        sender.lock().expect(HELD).send(&request);
        Vec::new()
    });
    let invoker = Arc::clone(endpoint);
    engine.register_mesh_rpc_invoker(move |event| {
        let mut endpoint = invoker.lock().expect(HELD);
        match event {
            HostInvokeEvent::Start(request) => Some(endpoint.invoke(&request)),
            HostInvokeEvent::Cancel(cancel) => {
                endpoint.cancel_invoke(&cancel.invoke_id, cancel.token);
                None
            }
        }
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
        /// The wall clock a fixed distance ahead of the monotonic one, so a
        /// wire deadline and a kept one can be told apart.
        fn now_unix_ms(&mut self) -> u64 {
            1_000_000 + self.now as u64
        }
        fn envelope_id(&mut self) -> [u8; 16] {
            self.ids += 1;
            [self.ids; 16]
        }
        fn jitter_draw(&mut self) -> i64 {
            0
        }
    }

    /// The events `endpoint` queued, which in these tests are all it queued.
    fn raised(endpoint: &mut Endpoint<Recorder, Fixed>) -> Vec<EngineEvent> {
        endpoint
            .take_calls()
            .into_iter()
            .map(|call| match call {
                EngineCall::Raise(event) => event,
                other => panic!("expected only events, got {other:?}"),
            })
            .collect()
    }

    fn config(peer: &'static str, retry: Option<RetryPolicy>) -> PeerConfig {
        PeerConfig {
            transport: "wss",
            buffer: Some(crate::outbound::OutboundBuffer {
                max_pending: 4,
                max_age_ms: 0,
            }),
            retry,
            stamp_sequence: false,
            delivery: Delivery {
                dedup: true,
                ordered: false,
            },
            // A generated peer table holds this as a `const`; a test names
            // its peer at run time, so the one-element set is leaked.
            responders: Box::leak(Box::new([peer])),
            deadline_ms: None,
            reply_events: &[],
        }
    }

    fn endpoint(
        machine: &str,
        peer: &'static str,
        retry: Option<RetryPolicy>,
    ) -> Endpoint<Recorder, Fixed> {
        let mut router = Router::new(machine, 8, 50).unwrap();
        router.add_peer(peer, config(peer, retry));
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

        let events = raised(&mut hmi);
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
        assert_eq!(raised(&mut hmi)[0].metadata.send_id, "");
    }

    #[test]
    fn a_failed_send_with_no_retry_policy_is_lost_as_send_failed() {
        let mut ecu = endpoint("ecu", "hmi", None);
        ecu.transport.failures.push_back(failure(true));
        ecu.send(&send("go", "s-1"));

        let events = raised(&mut ecu);
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
        assert!(raised(&mut ecu).is_empty());

        ecu.environment.now = 99;
        ecu.tick();
        assert!(ecu.transport.sent.is_empty(), "sent before its backoff");

        ecu.environment.now = 100;
        ecu.tick();
        assert_eq!(ecu.transport.sent.len(), 1);
        assert!(raised(&mut ecu).is_empty());
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

        let events = raised(&mut ecu);
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
        let events = raised(&mut ecu);
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
        assert!(raised(&mut ecu).is_empty());
        assert_eq!(
            ecu.take_host_errors(),
            vec![RouterError::UnknownPeer("stranger".to_string())]
        );
    }
}
