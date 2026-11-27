// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{
    Engine, EventMetadata, HostInvokeEvent, HostInvokeRequest, HostInvokeResponse, HostSendRequest,
    MeshCall, Principal, StatePolicy,
};

use crate::causal::{CausalLayer, Dot, Refusal, Start};
use crate::checkpoint::{Checkpoint, CheckpointError, Clocks, EndpointSection, RetryState};
use crate::outbound::{AfterFailure, Attempts};
use crate::router::{Effect, Invoked, Router, RouterError, Standing};
use crate::signal::Signal;

/// The `<send type>` a Mesh send is lowered to — the runtime's constant, so
/// the type this crate serves is the one the engine's router door registers.
pub use sce_rust_runtime::MESH_PROCESSOR_TYPE;

/// The `<invoke type>` a Mesh request is lowered to, and the
/// `_event.origintype` of the event that ends it (§mesh-9.5).
pub use sce_rust_runtime::MESH_RPC_INVOKE_TYPE;

/// `_event.origin` of a Mesh-delivered event is this scheme and the sending
/// machine's name (§mesh-10.7) — the C++ core's `kMeshOriginScheme`. The
/// runtime's constant, so the origin this crate writes is the one the engine
/// writes when it ends a Mesh request.
pub use sce_rust_runtime::MESH_ORIGIN_SCHEME;

/// Why a transport declined a transmission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportFailure {
    /// Whether sending the same bytes again could succeed — the transport's
    /// reading of its own failure. A terminal one is given up at once.
    pub retryable: bool,
    /// The transport's own words, carried as the row's `transport_error`.
    pub message: Option<String>,
    /// `Some(depth)` when the transport's own queue for the peer already held `depth` envelopes
    /// and took no more (the HTTPS binding's mailbox, SCE_MESH.md section 22.3): §mesh-16.7 row 9,
    /// `backpressureDrop`, raised at once and never retried, since sending again only meets the
    /// same full queue. `None` for every other failure.
    pub queue_full: Option<u32>,
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

/// What the runtime applies: the one body that tells an engine what its
/// endpoint has for it ([`Engine::apply_mesh_call`]), for a top-level
/// machine ([`apply_to`]) and for a child session alike.
impl From<EngineCall> for MeshCall {
    fn from(call: EngineCall) -> Self {
        match call {
            EngineCall::Raise(EngineEvent { name, metadata }) => MeshCall::Raise { name, metadata },
            EngineCall::EndInvoke {
                invoke_id,
                token,
                failed,
                data,
                source,
            } => MeshCall::EndInvoke {
                invoke_id,
                token,
                failed,
                data,
                source,
            },
        }
    }
}

/// What the endpoint has to tell its engine, kept as what makes the call, so
/// that a checkpoint can hold it (SCE_MESH.md section 10.11); the call is built
/// when the host takes it ([`Endpoint::take_calls`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Told {
    /// An event that arrived from `source`, or a row this machine raises as
    /// `source`, with the `_event` fields a Mesh event carries.
    Event {
        name: String,
        data: String,
        source: String,
        send_id: String,
        invoke_id: String,
        /// The person the instance `source` acted for when its edge took the event in
        /// (`_x.principal`, docs/adr/0116, D3; [`Endpoint::receive_member_for`]), `None` for
        /// every other event. A checkpoint holds none ([`Endpoint::seal`]). Boxed as
        /// [`EventMetadata::principal`] is, into which it moves.
        principal: Option<Box<Principal>>,
    },
    /// The end of the start `token` of the Mesh request `invoke_id`.
    End {
        invoke_id: String,
        token: u64,
        failed: bool,
        data: String,
        source: Option<String>,
    },
}

impl Told {
    fn call(self) -> EngineCall {
        match self {
            Told::Event {
                name,
                data,
                source,
                send_id,
                invoke_id,
                principal,
            } => EngineCall::Raise(EngineEvent {
                name,
                metadata: EventMetadata {
                    // §mesh-10.7: an inbound request's invokeid, which a
                    // reply sent while it is handled carries back.
                    invoke_id,
                    principal,
                    ..mesh_metadata(data, &source, send_id)
                },
            }),
            Told::End {
                invoke_id,
                token,
                failed,
                data,
                source,
            } => EngineCall::EndInvoke {
                invoke_id,
                token,
                failed,
                data,
                source,
            },
        }
    }
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
    to_engine: Vec<Told>,
    host_errors: Vec<RouterError>,
    /// The events a link may not deliver to the machine, by link
    /// ([`Self::forbid_events_from`]).
    forbidden: std::collections::BTreeMap<String, Vec<String>>,
    /// What decides, event by event, whether what arrived may reach the machine
    /// ([`Self::set_admission`]).
    admission: Option<Box<dyn Admission>>,
    /// What the host writes into an event as it leaves ([`Self::set_sealer`]).
    sealer: Option<Box<dyn Sealer>>,
    /// The verdicts of the causal layers on the changes the machine sent, in
    /// the order they were given ([`Self::take_verdicts`]).
    verdicts: Vec<CausalVerdict>,
}

/// The verdict of a causal peer's layer on a change the machine sent to it
/// (docs/adr/0085, section 3, "offer and verdict").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CausalVerdict {
    /// Tagged with `dot`. The host records it with its dot in the copy's
    /// journal and then calls [`Endpoint::committed`], which releases it.
    Accepted { peer: String, dot: Dot },
    /// Refused: it took no number and no copy takes it in.
    Refused { peer: String, refusal: Refusal },
}

/// What the host asks of an event that arrived before the machine is told of it. It sees the
/// link the event came on (which the host authenticated), the event's name, and its data as
/// the sender wrote it, and answers with the reason when the event may not reach the machine.
///
/// An admission is where a rule that needs the event's content is kept, which a machine of the
/// statically typed data model cannot do for itself: it reads no `_event` but the payload, so
/// it cannot tell who sent an event, or check a signature the payload carries.
pub trait Admission: Send {
    /// `Ok(None)` when `event`, which `peer`'s link delivered with `data` for its payload, may
    /// reach the machine as it is; `Ok(Some(data))` when it may, with `data` in place of what
    /// arrived (the admission took out what was written for the host and not for the machine);
    /// `Err` with the reason when it may not.
    fn admit(&mut self, peer: &str, event: &str, data: &str) -> Result<Option<String>, String>;
}

/// What the host writes into an event as it leaves, after the machine sent it and before it is
/// put in an envelope: the half of an [`Admission`] that is on the sending side. A machine
/// that does not carry a proof of its own sends (a signature, a token) leaves the host to add
/// it, and the receiving host's admission takes it out again.
pub trait Sealer: Send {
    /// `Ok(Some(data))` when `event`, which the machine sent to `target` with `data`, leaves with
    /// other data; `Ok(None)` when it leaves as it is; and `Err` with the reason when it must not
    /// leave at all (a write the host cannot sign or seal as the list requires): it is dropped,
    /// told to the host as [`RouterError::EventNotSealed`], and nothing else is sent in its place.
    fn seal(&mut self, target: &str, event: &str, data: &str) -> Result<Option<String>, String>;
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
            forbidden: std::collections::BTreeMap::new(),
            admission: None,
            sealer: None,
            verdicts: Vec::new(),
        }
    }

    /// Start the causal layer of the causal peer `peer` (see
    /// [`Router::start_causal`]).
    pub fn start_causal(
        &mut self,
        peer: &str,
        start: Start,
        room: usize,
        announce_every_ms: i64,
    ) -> Result<(), RouterError> {
        self.router
            .start_causal(peer, start, room, announce_every_ms)
    }

    /// The verdicts of the causal layers on what the machine sent, in order.
    /// The host records each accepted change in the copy's journal, calls
    /// [`Self::committed`] once the record is committed, and tells the copy of
    /// each refusal.
    pub fn take_verdicts(&mut self) -> Vec<CausalVerdict> {
        core::mem::take(&mut self.verdicts)
    }

    /// The causal layer of the causal peer `peer`, once the host started it
    /// (see [`Router::causal_layer`]): its replica, its delivered vector, what
    /// it holds and what its log keeps.
    pub fn causal_layer(&self, peer: &str) -> Option<&CausalLayer<Vec<u8>>> {
        self.router.causal_layer(peer)
    }

    /// When a causal layer with work is next owed its `causalVector` (see
    /// [`Router::next_announce_due_ms`]): with [`Self::next_retry_due_ms`],
    /// the time a host that owns time sleeps until before its next
    /// [`tick`](Self::tick).
    pub fn next_announce_due_ms(&self) -> Option<i64> {
        self.router.next_announce_due_ms()
    }

    /// The host committed the journal record of `dot`, a change sent to the
    /// causal peer `peer`: it, and every accepted change before it, leaves.
    pub fn committed(&mut self, peer: &str, dot: Dot) {
        let now = self.environment.now_ms();
        let effects = self.router.committed(peer, dot, now);
        self.apply(effects);
    }

    /// Put `sealer` behind the machine: what the machine sends is shown to it, and leaves with
    /// the data it answers with. Replaces the one set before.
    pub fn set_sealer(&mut self, sealer: Box<dyn Sealer>) {
        self.sealer = Some(sealer);
    }

    /// Put `admission` in front of the machine: every event that arrives and is not already
    /// forbidden by its link ([`Self::forbid_events_from`]) is shown to it, and one it does
    /// not admit is dropped and told to the host as [`RouterError::EventNotAdmitted`]. Replaces
    /// the one set before.
    pub fn set_admission(&mut self, admission: Box<dyn Admission>) {
        self.admission = Some(admission);
    }

    /// The events the link `peer` may not deliver to the machine: what arrives on it under
    /// one of these names is dropped before the machine is told, and the host is told
    /// instead ([`RouterError::EventForbidden`]). An empty list lifts it.
    ///
    /// The link is what the host authenticated when it accepted the connection (the token it
    /// checked, the certificate it verified), and the envelope is whatever the peer wrote,
    /// its `source` among it: so what a link may say is decided here, where the link is known,
    /// and not by a machine that is told who sent an event by the event. A hub that lets a
    /// member write by its role forbids the writes on the links of the members that may not.
    pub fn forbid_events_from(&mut self, peer: &str, events: &[&str]) {
        if events.is_empty() {
            self.forbidden.remove(peer);
        } else {
            self.forbidden.insert(
                peer.to_string(),
                events.iter().map(|event| (*event).to_string()).collect(),
            );
        }
    }

    /// Perform a `<send type="sce:mesh">` the engine handed its host.
    pub fn send(&mut self, request: &HostSendRequest) {
        let id = self.environment.envelope_id();
        let now = self.environment.now_ms();
        let sealed = match self.sealer.as_mut() {
            Some(sealer) => {
                match sealer.seal(&request.target, &request.event_name, &request.event_data) {
                    Ok(sealed) => sealed,
                    Err(reason) => {
                        self.host_errors.push(RouterError::EventNotSealed {
                            target: request.target.clone(),
                            event: request.event_name.clone(),
                            reason,
                        });
                        return;
                    }
                }
            }
            None => None,
        };
        let effects = match sealed {
            Some(event_data) => self.router.send(
                &HostSendRequest {
                    event_data,
                    ..request.clone()
                },
                id,
                now,
            ),
            None => self.router.send(request, id, now),
        };
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
        self.take_in(peer, effects);
    }

    /// Envelope `bytes` the transport received from `peer`, an instance of an open machine
    /// whose binding receives durably, which its edge took in under the incarnation `made_under`
    /// while the store held its member as `standing` (see [`Router::receive_member`]).
    ///
    /// Two refusals are the host's to answer the instance with, and are returned rather than
    /// reported (docs/adr/0113, D4 and D6): [`RouterError::Fenced`], whose exchange is answered
    /// `412`, and [`RouterError::InstancesFull`], whose exchange is shed (`503`). Nothing of
    /// the envelope was taken in by either.
    pub fn receive_member(
        &mut self,
        peer: &str,
        made_under: u64,
        standing: Standing,
        bytes: Vec<u8>,
    ) -> Result<(), RouterError> {
        let now = self.environment.now_ms();
        match self
            .router
            .receive_member(peer, made_under, standing, bytes, now)
        {
            Err(refused @ (RouterError::Fenced { .. } | RouterError::InstancesFull { .. })) => {
                Err(refused)
            }
            effects => {
                self.take_in(peer, effects);
                Ok(())
            }
        }
    }

    /// Whether what arrives on `peer`'s link is judged by its instance's completion records (see
    /// [`Router::judges_by_records`]).
    pub fn judges_by_records(&self, peer: &str) -> bool {
        self.router.judges_by_records(peer)
    }

    /// The instances of open machines the endpoint's router bound since the host last asked (see
    /// [`Router::take_bound`]).
    pub fn take_bound(&mut self) -> Vec<String> {
        self.router.take_bound()
    }

    /// As [`Self::receive_member`], for an instance whose incarnation `made_under` acts for a
    /// person (docs/adr/0116, D1 and D3): `acting` is the person of the delegation the server's
    /// store holds for that incarnation, which the edge read where it took the envelope in, or
    /// `None` for an incarnation that acts for nobody. Each event the envelope raises on the
    /// instance's behalf carries it, and a document reads it as `_x.principal`; a row the
    /// endpoint raises as its own machine does not (docs/adr/0116, D1, invariant 12 of SCE_MESH.md
    /// section 10.9: what a machine reads of a person comes from the delegation its own server
    /// holds for the link the event came on, and from no field of an envelope). A refusal is
    /// [`Self::receive_member`]'s, and an envelope refused raises nothing to carry the person.
    pub fn receive_member_for(
        &mut self,
        peer: &str,
        made_under: u64,
        standing: Standing,
        acting: Option<Principal>,
        bytes: Vec<u8>,
    ) -> Result<(), RouterError> {
        let from = self.to_engine.len();
        self.receive_member(peer, made_under, standing, bytes)?;
        let Some(person) = acting.map(Box::new) else {
            return Ok(());
        };
        for told in &mut self.to_engine[from..] {
            if let Told::Event {
                source, principal, ..
            } = told
            {
                if source.as_str() == peer {
                    *principal = Some(person.clone());
                }
            }
        }
        Ok(())
    }

    /// Take `standing`, the store's latest answer for the member whose instance is `peer` (see
    /// [`Router::stand`]).
    pub fn stand(&mut self, peer: &str, standing: Standing) {
        self.router.stand(peer, standing);
    }

    /// The incarnations the run the last checkpoint sealed keeps was made under (see
    /// [`Router::sealed_under`]).
    pub fn sealed_under(&self) -> &[(String, u64)] {
        self.router.sealed_under()
    }

    /// Apply what the router made of what arrived on `peer`'s link, past the events the link may
    /// not deliver ([`Self::forbid_events_from`]) and the host's admission
    /// ([`Self::set_admission`]).
    fn take_in(&mut self, peer: &str, mut effects: Result<Vec<Effect>, RouterError>) {
        let mut refused: Vec<RouterError> = Vec::new();
        if let Ok(list) = effects.as_mut() {
            let forbidden = self.forbidden.get(peer);
            let admission = &mut self.admission;
            list.retain_mut(|effect| {
                let Effect::Deliver { event, data, .. } = effect else {
                    return true;
                };
                if forbidden.is_some_and(|events| events.contains(event)) {
                    refused.push(RouterError::EventForbidden {
                        peer: peer.to_string(),
                        event: event.clone(),
                    });
                    return false;
                }
                match admission.as_mut().map(|a| a.admit(peer, event, data)) {
                    Some(Err(reason)) => {
                        refused.push(RouterError::EventNotAdmitted {
                            peer: peer.to_string(),
                            event: event.clone(),
                            reason,
                        });
                        false
                    }
                    Some(Ok(Some(admitted))) => {
                        *data = admitted;
                        true
                    }
                    _ => true,
                }
            });
        }
        self.host_errors.append(&mut refused);
        self.apply(effects);
    }

    /// `peer`'s transport became ready.
    pub fn peer_ready(&mut self, peer: &str) {
        let now = self.environment.now_ms();
        let effects = self.router.peer_ready(peer, now);
        self.apply(effects);
    }

    /// `peer`'s transport stopped being ready, and the link is lost: each
    /// request waiting on it is forgotten ([`Router::peer_not_ready`]).
    pub fn peer_not_ready(&mut self, peer: &str) {
        let effects = self.router.peer_not_ready(peer);
        self.apply(effects);
    }

    /// `peer`'s transport stopped being ready while `peer` keeps what it holds
    /// for this machine: a request waiting on it stays waiting
    /// ([`Router::peer_interrupted`]).
    pub fn peer_interrupted(&mut self, peer: &str) {
        let effects = self.router.peer_interrupted(peer);
        self.apply(effects);
    }

    /// `peer` started again without what it held for this machine: each
    /// request waiting on it is sent again ([`Router::peer_restarted`]).
    pub fn peer_restarted(&mut self, peer: &str) {
        let now = self.environment.now_ms();
        let effects = self.router.peer_restarted(peer, now);
        self.apply(effects);
    }

    /// An answer that arrived as `peer`'s did not prove it came from `peer`
    /// (SCE_MESH.md section 22.9), and the link took nothing of it: the
    /// machine is told, as section 16.7 row 19. Readiness, what waits and
    /// what is queued are the link's to keep as they were, since nothing
    /// of the answer is believed.
    pub fn peer_unauthenticated(&mut self, peer: &str) {
        self.raise(Some(peer), &Signal::ResponseUnauthenticated);
    }

    /// Bind `peer` while the endpoint runs: a device a host learned of after it built its router (a
    /// person who joined a list). Replaces the binding when `peer` is bound already, and with it
    /// what that binding held.
    pub fn add_peer(&mut self, peer: &str, config: crate::router::PeerConfig) {
        self.router.add_peer(peer, config);
    }

    /// Bind the instances of the open machine `machine` as they arrive, at most `max_instances` at
    /// once (see [`Router::open_machine`]).
    pub fn open_machine(
        &mut self,
        machine: &str,
        config: crate::router::PeerConfig,
        max_instances: usize,
    ) -> Result<(), RouterError> {
        self.router.open_machine(machine, config, max_instances)
    }

    /// Unbind `peer`, which has left for good: its link's state, what waited for it, what it
    /// forbids and what is queued to retry to it are let go, and the machine is told what a lost
    /// link tells it. A peer that is not bound is left as it is.
    pub fn remove_peer(&mut self, peer: &str) {
        let effects = self.router.remove_peer(peer);
        self.retries.retain(|retry| retry.peer != peer);
        self.forbidden.remove(peer);
        self.apply(Ok(effects));
    }

    /// Whether `peer` is bound.
    pub fn has_peer(&self, peer: &str) -> bool {
        self.router.has_peer(peer)
    }

    /// Whether the endpoint holds nothing that a later call would still act
    /// on: nothing for the engine, no transmission waiting out its backoff, no
    /// request waiting for its answer, nothing buffered for a peer that is
    /// not ready, and nothing held for a checkpoint not yet durable or for a
    /// durable receiver's acknowledgement ([`Router::holds_for_durability`]).
    /// An endpoint dropped while it holds one of these loses it, so a host
    /// that drops idle machines asks this first.
    pub fn is_quiet(&self) -> bool {
        self.to_engine.is_empty()
            && self.retries.is_empty()
            && !self.router.awaits_replies()
            && !self.router.holds_outbound()
            && !self.router.holds_for_durability()
    }

    /// Whether the request the machine sent under the wire `invoke_id`
    /// `wire_id` still waits for its answer (see [`Router::awaits_reply`]).
    pub fn awaits_reply(&self, wire_id: &[u8; 16]) -> bool {
        self.router.awaits_reply(wire_id)
    }

    /// An exchange `peer` answered carried `envelopes`, which it answers only
    /// once a durable checkpoint of its own holds them: under `receipt:
    /// durable` on the `https` binding, each is acknowledged and held no more
    /// ([`Router::carried`]; docs/adr/0109, D10).
    pub fn peer_carried(&mut self, peer: &str, envelopes: &[Vec<u8>]) {
        self.router.carried(peer, envelopes);
    }

    /// Every envelope to `peer` takes a place in `window` first (see
    /// [`Router::bound_window`]): an `https` server's outbox (docs/adr/0109,
    /// D10).
    pub fn bound_window(
        &mut self,
        peer: &str,
        window: Arc<dyn crate::router::Window>,
    ) -> Result<(), RouterError> {
        self.router.bound_window(peer, window)
    }

    /// How many envelopes the machine produced for `peer` that no durable
    /// checkpoint released yet ([`Router::held_for`]).
    pub fn held_for(&self, peer: &str) -> usize {
        self.router.held_for(peer)
    }

    /// An envelope the machine sent `peer` was kept in `peer`'s outbox for
    /// `age_ms`, longer than the binding's `retain_ms`, without `peer`
    /// acknowledging it, and the outbox let it go (docs/adr/0109, D10): the
    /// machine is told, as SCE_MESH.md section 16.7 row 15, with `retain_ms`
    /// as the bound it outlived.
    pub fn outbox_expired(&mut self, peer: &str, age_ms: u64, retain_ms: u64) {
        let bound = |ms: u64| i64::try_from(ms).unwrap_or(i64::MAX);
        self.raise(
            Some(peer),
            &Signal::OutboundStaleDrop {
                age_ms: bound(age_ms),
                max_age_ms: bound(retain_ms),
            },
        );
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

    /// When the earliest transmission waiting out its backoff is due, in the
    /// environment's milliseconds: the time at which the host's next
    /// [`tick`](Self::tick) has something to send. `None` when nothing waits,
    /// which is what lets a host that owns time (a simulation) sleep until the
    /// next thing happens and not poll.
    pub fn next_retry_due_ms(&self) -> Option<i64> {
        self.retries
            .iter()
            .map(|retry| retry.due_ms)
            .chain(self.router.next_resend_due_ms())
            .min()
    }

    /// Seal the checkpoint `sequence` of the machine this endpoint serves
    /// (SCE_MESH.md section 10.11): what the machine produced since the last
    /// seal belongs to it, and is released when the host reports it durable
    /// ([`Self::durable`]). Called between two drives of the machine, when the
    /// host saved it; `sequence` is above every number this endpoint was
    /// sealed or restored with. Answers what the checkpoint holds of this
    /// endpoint: its router's state, what waits out a backoff, and what was
    /// released for the machine that the host has not yet applied, every
    /// moment on the environment's wall clock.
    pub fn seal(&mut self, sequence: u64) -> Result<EndpointSection, CheckpointError> {
        // docs/adr/0116, D6: an event said for a person is never run as nobody's, and a
        // checkpoint holds no person. Refused before the router is sealed, so the same number
        // seals once the host has applied and run it.
        if let Some(Told::Event { name, source, .. }) = self.to_engine.iter().find(|told| {
            matches!(
                told,
                Told::Event {
                    principal: Some(_),
                    ..
                }
            )
        }) {
            return Err(CheckpointError::Person(alloc::format!(
                "the event '{name}' of the instance '{source}' was said for a person and the host \
                 has not applied it"
            )));
        }
        let clocks = self.clocks();
        let router = self
            .router
            .seal(sequence, clocks)
            .map_err(CheckpointError::Router)?;
        Ok(EndpointSection {
            machine: self.router.machine().to_string(),
            router,
            retries: self
                .retries
                .iter()
                .map(|retry| RetryState {
                    peer: retry.peer.clone(),
                    envelope: retry.bytes.clone(),
                    made: retry.attempts.made(),
                    backoff: retry.attempts.last_backoff_ms(),
                    due: clocks.to_wall(retry.due_ms),
                })
                .collect(),
            calls: self.to_engine.clone(),
        })
    }

    /// The checkpoint `sequence` of a machine whose tree has this one
    /// endpoint: `state`, the machine's saved state, and this endpoint sealed
    /// with `sequence` ([`Self::seal`]). `clean` when the host stopped
    /// admitting before it (an orderly shutdown, its links closed first).
    pub fn checkpoint(
        &mut self,
        sequence: u64,
        state: SavedState,
        clean: bool,
    ) -> Result<Checkpoint, CheckpointError> {
        Ok(Checkpoint {
            sequence,
            clean,
            state,
            endpoints: alloc::vec![self.seal(sequence)?],
            followed: alloc::collections::BTreeMap::new(),
        })
    }

    /// The host wrote the checkpoint `sequence` durably: release what it, and
    /// every checkpoint before it, holds (SCE_MESH.md section 10.11).
    pub fn durable(&mut self, sequence: u64) {
        let now = self.environment.now_ms();
        let effects = self.router.durable(sequence, now);
        self.apply(effects);
    }

    /// Take this endpoint back to what `checkpoint`, the last durable
    /// checkpoint of its machine, holds of it (SCE_MESH.md section 10.11): the
    /// endpoint is the one the host built from the deployment, checkpointing
    /// and with its durable receipt declared, before anything was sent or
    /// received through it. What the checkpoint held unreleased leaves; what
    /// was released for the machine and not applied is queued for it again,
    /// followed, unless the checkpoint is clean, by a `restored` row (§mesh-16.7
    /// row 17) for each peer source the endpoint holds state for whose binding
    /// is not durable. A refused restore leaves the endpoint in no state to
    /// use.
    pub fn restore(&mut self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
        let clocks = self.clocks();
        let section = checkpoint
            .endpoint(self.router.machine())
            .ok_or_else(|| CheckpointError::NoSection(self.router.machine().to_string()))?;
        let effects = self.router.restore(
            &section.router,
            checkpoint.sequence,
            checkpoint.clean,
            clocks,
        )?;
        self.retries = section
            .retries
            .iter()
            .map(|retry| Retry {
                peer: retry.peer.clone(),
                bytes: retry.envelope.clone(),
                attempts: Attempts::resumed(retry.made, retry.backoff),
                due_ms: clocks.to_monotonic(retry.due),
            })
            .collect();
        self.to_engine = section.calls.clone();
        self.apply(Ok(effects));
        Ok(())
    }

    /// The last checkpoint this endpoint was sealed with ([`Router::sealed_through`]).
    pub fn sealed_through(&self) -> u64 {
        self.router.sealed_through()
    }

    /// The last checkpoint reported durable ([`Router::durable_through`]).
    pub fn durable_through(&self) -> u64 {
        self.router.durable_through()
    }

    /// The environment's two clocks now.
    fn clocks(&mut self) -> Clocks {
        Clocks {
            monotonic_ms: self.environment.now_ms(),
            wall_ms: self.environment.now_unix_ms(),
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
            .into_iter()
            .map(Told::call)
            .collect()
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
                } => self.to_engine.push(Told::Event {
                    name: event,
                    data,
                    source,
                    send_id: send_id.unwrap_or_default(),
                    invoke_id: invoke_id.unwrap_or_default(),
                    // The person an instance acts for is the edge's to tell
                    // ([`Self::receive_member_for`]), never the envelope's.
                    principal: None,
                }),
                Effect::Raise { peer, signal } => self.raise(peer.as_deref(), &signal),
                Effect::Complete {
                    invoke_id,
                    token,
                    data,
                    source,
                } => self.to_engine.push(Told::End {
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
                } => self.to_engine.push(Told::End {
                    invoke_id,
                    token,
                    failed: true,
                    data,
                    source,
                }),
                Effect::AcknowledgeDue { peer } => {
                    let id = self.environment.envelope_id();
                    match self.router.acknowledge(&peer, id) {
                        // A lost acknowledgement is repaired by the next one
                        // (SCE_MESH.md section 10.11), so a failed send of one
                        // is neither retried nor raised.
                        Ok(bytes) => {
                            let _ = self.transport.transmit(&peer, &bytes);
                        }
                        Err(error) => self.host_errors.push(error),
                    }
                }
                Effect::Accepted { peer, dot } => {
                    self.verdicts.push(CausalVerdict::Accepted { peer, dot })
                }
                Effect::Refused { peer, refusal } => {
                    self.verdicts.push(CausalVerdict::Refused { peer, refusal })
                }
                Effect::AnnounceDue { peer } => {
                    let id = self.environment.envelope_id();
                    let now = self.environment.now_ms();
                    let announced = self.router.announce(&peer, id, now);
                    self.apply(announced);
                }
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
        // SCE_MESH.md section 10.11: what a binding with durable receipt sends
        // is held until acknowledged and sent again by the router, so a failed
        // send of it is neither retried here nor given up, nor dropped when the
        // transport's queue is full.
        if self.router.holds_until_acknowledged(&peer) {
            return;
        }
        if let Some(depth) = failure.queue_full {
            return self.raise(Some(&peer), &Signal::BackpressureDrop { depth });
        }
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
        self.to_engine.push(Told::Event {
            name: "error.communication".to_string(),
            data,
            source: machine,
            send_id: String::new(),
            invoke_id: String::new(),
            principal: None,
        });
    }
}

/// The `_event` of an event that arrived over Mesh from `source`, as
/// §mesh-10.7 tabulates it and the C++ core fills it: external, origin
/// `mesh://<source>`, the SCXML processor as origin type (every envelope is
/// SCE's own, so the traffic is inter-SCXML by construction), and the send
/// id the envelope carried. Built by the runtime's one function for it, which
/// is also what a child session's event is built by.
fn mesh_metadata(data: String, source: &str, send_id: String) -> EventMetadata {
    sce_rust_runtime::mesh_event_metadata(data, source, send_id)
}

#[cfg(test)]
impl<T, E> Endpoint<T, E> {
    /// The transport, for a test that reads what it was handed.
    pub(crate) fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// The environment, for a test that moves its clock.
    pub(crate) fn environment_mut(&mut self) -> &mut E {
        &mut self.environment
    }
}

/// Tell `engine` what `calls` say, in order. An event name the machine does
/// not declare is dropped, as the engine drops any such event; an
/// invocation's end the engine no longer waits for — its state exited, or
/// it was started again — is refused by the engine's own token check.
///
/// An answer carries `_event.origin` `mesh://<source>` and origin type
/// `sce:mesh-rpc` (§mesh-9.5); a deadline the requester reached itself has
/// no peer to name, and carries neither. Applied by the engine's own
/// [`Engine::apply_mesh_call`], the body a child session's calls go through.
pub fn apply_to<P: StatePolicy>(engine: &mut Engine<P>, calls: Vec<EngineCall>) {
    for call in calls {
        engine.apply_mesh_call(call.into());
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
    use crate::router::{PeerConfig, Responders};
    use crate::testing::{Fixed, Recorder};
    use alloc::vec;
    use sce_rust_runtime::helpers::scxml_constants::SCXML_EVENT_PROCESSOR_TYPE;
    use sce_rust_runtime::EventType;

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
            responders: Responders::Named(Box::leak(Box::new([peer]))),
            deadline_ms: None,
            reply_events: &[],
            causal: None,
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
            queue_full: None,
        }
    }

    /// SCE_MESH.md section 22.9: an answer that did not verify is told to
    /// the machine as row 19, naming the binding it arrived on, and leaves
    /// the link as ready as it was. Red if `deliver` drops the event, or if
    /// it ends readiness as row 1 would.
    #[test]
    fn an_answer_that_did_not_verify_is_row_19_and_leaves_the_link_ready() {
        use crate::link::{deliver, LinkEvent};

        let mut hmi = endpoint("ecu", "hmi", None);
        deliver(
            &mut hmi,
            LinkEvent::Unauthenticated("hmi".to_string(), "no signature".to_string()),
        );
        let events = raised(&mut hmi);
        let [event] = events.as_slice() else {
            panic!("expected one error event, got {events:?}");
        };
        assert_eq!(event.name, "error.communication");
        assert_eq!(
            event.metadata.data,
            r#"{"errorName":"communication","reason":"responseUnauthenticated","target":"hmi","transport":"wss"}"#
        );
        hmi.send(&send("note", "after"));
        assert_eq!(
            hmi.transport.sent.len(),
            1,
            "the link is still ready, so a send leaves at once"
        );
    }

    /// What the machine is told when it sends to `target` and no binding for it is held: one
    /// `error.communication`, row 1 (`transportUnavailable`), that names the target and no
    /// transport, because the deployment bound none.
    fn assert_unbound(hub: &mut Endpoint<Recorder, Fixed>, target: &str) {
        hub.send(&HostSendRequest {
            target: format!("#{target}"),
            ..send("note", "unbound")
        });
        let events = raised(hub);
        let [event] = events.as_slice() else {
            panic!("expected one error event, got {events:?}");
        };
        assert_eq!(event.name, "error.communication");
        assert!(
            event.metadata.data.contains("transportUnavailable")
                && event
                    .metadata
                    .data
                    .contains(&format!("\"target\":\"{target}\""))
                && !event.metadata.data.contains("\"transport\""),
            "{}",
            event.metadata.data
        );
        assert!(hub.transport.sent.is_empty(), "nothing was transmitted");
        assert!(hub.take_host_errors().is_empty());
    }

    /// A peer bound after the endpoint was built is one the machine can send to and hear from, and a
    /// peer that was never bound is neither: a send to it is `error.communication`.
    #[test]
    fn a_peer_bound_while_the_endpoint_runs_is_one_it_can_send_to() {
        let mut hub = endpoint("hub", "dev-0", None);
        assert!(!hub.has_peer("dev-1"));
        assert_unbound(&mut hub, "dev-1");

        hub.add_peer("dev-1", config("dev-1", None));
        assert!(hub.has_peer("dev-1"));
        hub.peer_ready("dev-1");
        hub.send(&HostSendRequest {
            target: "#dev-1".to_string(),
            ..send("note", "n-2")
        });
        assert!(hub.take_host_errors().is_empty());
        assert!(raised(&mut hub).is_empty(), "nothing was raised");
        let sent = hub.transport.sent.pop().unwrap();
        assert_eq!(sent.0, "dev-1");
    }

    /// A peer that left is unbound: a send to it is `error.communication`, what it had sent is not
    /// remembered (so the same envelope is delivered again if a peer of that name comes back),
    /// and what the endpoint held to retry to it is dropped. Another peer is not touched.
    #[test]
    fn a_peer_that_left_is_forgotten_and_the_others_are_not() {
        let mut ecu = endpoint("ecu", "hub", None);
        ecu.send(&HostSendRequest {
            target: "#hub".to_string(),
            ..send("go", "s-1")
        });
        let (_, bytes) = ecu.transport.sent.pop().unwrap();

        let mut hub = endpoint("hub", "ecu", None);
        hub.add_peer("other", config("other", None));
        hub.peer_ready("other");
        hub.receive("ecu", bytes.clone());
        hub.receive("ecu", bytes.clone());
        assert_eq!(raised(&mut hub).len(), 1, "the second is a duplicate");

        hub.remove_peer("ecu");
        assert!(!hub.has_peer("ecu"));
        assert!(hub.has_peer("other"));
        // A link that was ready and is gone is the one edge a binding raises (§mesh-18.3), and the
        // machine is told it once, as when the link is lost.
        let edge = raised(&mut hub);
        assert_eq!(edge.len(), 1, "{edge:?}");
        assert_eq!(edge[0].name, "error.communication");
        assert_unbound(&mut hub, "ecu");

        // The peer is bound again by the host: what it sent before is new to the window.
        hub.add_peer("ecu", config("ecu", None));
        hub.receive("ecu", bytes);
        assert_eq!(raised(&mut hub).len(), 1, "its window did not outlive it");

        hub.send(&HostSendRequest {
            target: "#other".to_string(),
            ..send("note", "n-2")
        });
        assert!(hub.take_host_errors().is_empty(), "the other peer is bound");
        assert!(raised(&mut hub).is_empty());
        assert_eq!(hub.transport.sent.pop().unwrap().0, "other");
    }

    /// An envelope a link delivers in another machine's name is told to the host and to no
    /// engine, and leaves that machine's stream as it was (SCE_MESH.md section 10.9, invariant
    /// 10): ecu's envelope, played again on hud's link, is refused, and ecu's own copy is then
    /// delivered once, with ecu's origin. Red if the router takes `source` over the link: the
    /// copy on hud's link is then delivered as ecu's, and ecu's own is dropped as a duplicate.
    #[test]
    fn an_envelope_on_another_machines_link_reaches_the_host_and_not_the_machine() {
        let mut ecu = endpoint("ecu", "hmi", None);
        ecu.send(&send("go", "s-1"));
        let (_, bytes) = ecu.transport.sent.pop().unwrap();

        let mut hmi = endpoint("hmi", "ecu", None);
        hmi.add_peer("hud", config("hud", None));
        hmi.receive("hud", bytes.clone());
        assert_eq!(
            hmi.take_host_errors(),
            vec![RouterError::SourceNotLink {
                peer: "hud".to_string(),
                source: "ecu".to_string(),
            }]
        );
        assert!(hmi.take_calls().is_empty(), "no engine is told");

        hmi.receive("ecu", bytes);
        let events = raised(&mut hmi);
        let [event] = events.as_slice() else {
            panic!("expected ecu's event once, got {events:?}");
        };
        assert_eq!(event.name, "go");
        assert_eq!(event.metadata.origin_address(), "mesh://ecu");
        assert!(hmi.take_host_errors().is_empty());
    }

    /// An endpoint is quiet only while it holds nothing a later call acts on: a request
    /// waiting for its answer, a send waiting out its backoff and a send buffered for a peer
    /// that is not ready each keep it from being quiet, and each, once it ends, lets it be.
    #[test]
    fn an_endpoint_is_quiet_only_while_it_holds_nothing_pending() {
        let mut ecu = endpoint("ecu", "hmi", None);
        assert!(ecu.is_quiet());

        // A request waits for its answer until its deadline ends it.
        let mut params = std::collections::HashMap::new();
        params.insert(
            crate::rpc::MESH_EVENT_PARAM.to_string(),
            vec!["service.request.force".to_string()],
        );
        params.insert(
            crate::rpc::MESH_DEADLINE_PARAM.to_string(),
            vec!["50".to_string()],
        );
        let started = ecu.invoke(&HostInvokeRequest {
            processor_type: MESH_RPC_INVOKE_TYPE.to_string(),
            invoke_id: "ask".to_string(),
            src: "#hmi".to_string(),
            params,
            token: 1,
            ..HostInvokeRequest::default()
        });
        assert!(started.refusal.is_none(), "{started:?}");
        let (_, request) = ecu.transport.sent.pop().unwrap();
        let envelope = crate::generated::envelope::Envelope::decode(
            &mut sce_forge_runtime::codec::SceCursor::new(&request),
        )
        .unwrap();
        let wire: [u8; 16] = envelope.invoke_id.unwrap().into();
        assert!(ecu.awaits_reply(&wire));
        assert!(
            !ecu.awaits_reply(&[0xEE; 16]),
            "no request went out under it"
        );
        assert!(!ecu.is_quiet(), "a request waits");
        ecu.environment.now = 50;
        ecu.tick();
        assert!(!ecu.awaits_reply(&wire));
        assert!(!ecu.is_quiet(), "its end is still for the engine");
        ecu.take_calls();
        assert!(ecu.is_quiet());

        // A send waits out its backoff.
        let policy = RetryPolicy {
            max_retries: 1,
            initial_backoff_ms: 10,
            backoff_multiplier: 1.0,
            max_backoff_ms: 10,
            jitter_pct: 0,
        };
        let mut retrying = endpoint("ecu", "hmi", Some(policy));
        retrying.transport.failures.push_back(failure(true));
        retrying.send(&send("go", "s-1"));
        assert!(!retrying.is_quiet(), "a send waits out its backoff");
        retrying.environment.now = 10;
        retrying.tick();
        assert!(retrying.is_quiet());

        // A send is buffered for a peer that is not ready.
        let mut buffered = endpoint("ecu", "hmi", None);
        buffered.peer_not_ready("hmi");
        buffered.take_calls();
        buffered.send(&send("go", "s-2"));
        assert!(!buffered.is_quiet(), "a send is buffered");
        buffered.peer_ready("hmi");
        assert!(buffered.is_quiet());
    }

    /// The order an HTTPS exchange that lost its response hands the client its events in
    /// (SCE_MESH.md section 22.4): the exchange that carried a request fails after the server
    /// took it, so the request comes back unsent and the link is interrupted, and only then does
    /// the reply arrive, from the mailbox or answering the copy sent again. Handed to the
    /// endpoint as the binding hands them, the request stays waiting through the failure, its
    /// copy goes again under the retry policy, and the reply ends the invocation once. Red if an
    /// interrupted link is applied as a lost one: the reply then finds nothing waiting, and the
    /// invocation never ends.
    #[test]
    fn a_reply_after_an_interrupted_exchange_ends_the_request_once() {
        use crate::link::{deliver, LinkEvent};

        let policy = RetryPolicy {
            max_retries: 3,
            initial_backoff_ms: 10,
            backoff_multiplier: 1.0,
            max_backoff_ms: 10,
            jitter_pct: 0,
        };
        let mut ecu = endpoint("ecu", "hmi", Some(policy));
        let mut params = std::collections::HashMap::new();
        params.insert(
            crate::rpc::MESH_EVENT_PARAM.to_string(),
            vec!["service.request.force".to_string()],
        );
        let started = ecu.invoke(&HostInvokeRequest {
            processor_type: MESH_RPC_INVOKE_TYPE.to_string(),
            invoke_id: "ask".to_string(),
            src: "#hmi".to_string(),
            params,
            token: 1,
            ..HostInvokeRequest::default()
        });
        assert!(started.refusal.is_none(), "{started:?}");
        let (_, request) = ecu.transport.sent.pop().unwrap();
        let envelope = crate::generated::envelope::Envelope::decode(
            &mut sce_forge_runtime::codec::SceCursor::new(&request),
        )
        .unwrap();
        let wire: [u8; 16] = envelope.invoke_id.unwrap().into();

        // The server's machine took the request and answered it.
        let mut hmi = Endpoint::new(
            Router::new("hmi", 8, 50).unwrap(),
            Recorder::default(),
            Fixed::default(),
        );
        hmi.add_peer(
            "ecu",
            PeerConfig {
                reply_events: &["service.response.force"],
                ..config("ecu", None)
            },
        );
        hmi.peer_ready("ecu");
        hmi.receive("ecu", request.clone());
        assert_eq!(raised(&mut hmi).len(), 1, "the request reached the machine");
        hmi.send(&HostSendRequest {
            target: "#ecu".to_string(),
            invoke_id: crate::rpc::uuid_text(&wire),
            ..send("service.response.force", "")
        });
        let (_, answer) = hmi.transport.sent.pop().unwrap();

        // The response that would have carried it was lost.
        deliver(
            &mut ecu,
            LinkEvent::Unsent("hmi".to_string(), request.clone()),
        );
        deliver(
            &mut ecu,
            LinkEvent::Interrupted("hmi".to_string(), "the connection closed".to_string()),
        );
        let edge = raised(&mut ecu);
        assert_eq!(edge.len(), 1, "{edge:?}");
        assert!(
            edge[0].metadata.data.contains("transportUnavailable"),
            "row 1, and no row 5: {}",
            edge[0].metadata.data
        );
        assert!(ecu.awaits_reply(&wire), "the request still waits");

        ecu.environment.now = 10;
        ecu.tick();
        assert_eq!(
            ecu.transport.sent.pop(),
            Some(("hmi".to_string(), request)),
            "the copy goes again"
        );
        deliver(&mut ecu, LinkEvent::Ready("hmi".to_string()));
        deliver(
            &mut ecu,
            LinkEvent::Received("hmi".to_string(), answer.clone()),
        );
        deliver(&mut ecu, LinkEvent::Received("hmi".to_string(), answer));
        let ended: Vec<EngineCall> = ecu.take_calls();
        assert!(
            matches!(
                ended.as_slice(),
                [EngineCall::EndInvoke { invoke_id, failed: false, .. }] if invoke_id == "ask"
            ),
            "{ended:?}"
        );
        assert!(!ecu.awaits_reply(&wire));
    }

    #[test]
    fn removing_a_peer_that_is_not_bound_changes_nothing() {
        let mut hub = endpoint("hub", "ecu", None);
        hub.remove_peer("nobody");
        assert!(hub.has_peer("ecu"));
        assert!(hub.take_host_errors().is_empty());
        assert!(hub.take_calls().is_empty());
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
        assert_eq!(event.metadata.origin_address(), "mesh://ecu");
        assert_eq!(
            event.metadata.origin_processor(),
            SCXML_EVENT_PROCESSOR_TYPE
        );
        assert!(event.metadata.sender.is_empty());
        assert_eq!(event.metadata.send_id, "s-7");
    }

    /// docs/adr/0113, D2: an event from an instance of an open machine carries the instance's own
    /// origin, `mesh://<machine>/<instance>`, and a send to that origin reaches the instance's
    /// link. Red if the origin names the machine alone, which every instance shares.
    #[test]
    fn an_instances_event_carries_its_origin_and_a_send_to_it_reaches_it() {
        let mut phone = endpoint("phone", "cloud", None);
        let mut cloud = Endpoint::new(
            Router::new("cloud", 8, 50).unwrap(),
            Recorder::default(),
            Fixed::default(),
        );
        cloud
            .open_machine(
                "phone",
                PeerConfig {
                    responders: Responders::Instance,
                    ..config("phone", None)
                },
                4,
            )
            .unwrap();
        phone.send(&HostSendRequest {
            target: "#cloud".to_string(),
            ..send("go", "s-1")
        });
        let (_, sent) = phone.transport.sent.pop().unwrap();
        // The instance's host names it in the envelope (docs/adr/0113, D1).
        let mut envelope = crate::generated::envelope::Envelope::decode(
            &mut sce_forge_runtime::codec::SceCursor::new(&sent),
        )
        .unwrap();
        envelope.instance = Some(crate::router::within("a1").unwrap());
        cloud.receive("phone/a1", envelope.encode_to_vec());
        let events = raised(&mut cloud);
        let [event] = events.as_slice() else {
            panic!("expected one event, got {events:?}");
        };
        assert_eq!(event.metadata.origin_address(), "mesh://phone/a1");

        cloud.peer_ready("phone/a1");
        cloud.send(&HostSendRequest {
            target: event.metadata.origin_address().to_string(),
            ..send("back", "s-2")
        });
        let (peer, _) = cloud.transport.sent.pop().unwrap();
        assert_eq!(peer, "phone/a1");
    }

    /// A link the host authenticated may be forbidden some events: they are dropped before the
    /// machine is told, whatever source the envelope names, and the host is told instead; the
    /// others go through, and a link that is not forbidden is untouched.
    #[test]
    fn a_link_may_be_forbidden_events() {
        let mut ecu = endpoint("ecu", "hmi", None);
        ecu.send(&send("go", "s-1"));
        let (_, go) = ecu.transport.sent.pop().unwrap();
        ecu.send(&send("stop", "s-2"));
        let (_, stop) = ecu.transport.sent.pop().unwrap();

        let mut hmi = endpoint("hmi", "ecu", None);
        hmi.forbid_events_from("ecu", &["go"]);
        hmi.receive("ecu", go.clone());
        hmi.receive("ecu", stop.clone());
        let events = raised(&mut hmi);
        assert_eq!(
            events.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["stop"],
            "what the link may not deliver never reaches the machine"
        );
        assert_eq!(
            hmi.take_host_errors(),
            vec![RouterError::EventForbidden {
                peer: "ecu".to_string(),
                event: "go".to_string()
            }]
        );

        // Lifted, it is delivered; and a link nothing was forbidden on is not touched.
        hmi.forbid_events_from("ecu", &[]);
        let mut again = endpoint("hmi", "ecu", None);
        again.receive("ecu", go);
        assert_eq!(raised(&mut again).len(), 1);
        assert!(again.take_host_errors().is_empty());
    }

    /// What decides an event's admission is shown the link, the name and the data the sender
    /// wrote; an event it does not admit never reaches the machine and the host is told why,
    /// and one it admits goes through.
    #[test]
    fn an_admission_decides_what_reaches_the_machine() {
        struct OnlyShort;
        impl Admission for OnlyShort {
            fn admit(
                &mut self,
                peer: &str,
                event: &str,
                data: &str,
            ) -> Result<Option<String>, String> {
                assert_eq!(peer, "ecu");
                if event == "go" {
                    // What the sealer wrote for the host is taken out before the machine
                    // is told.
                    Ok(data.strip_suffix("+seal").map(str::to_string))
                } else {
                    Err(alloc::format!("{event} is not for this link"))
                }
            }
        }
        struct Seal;
        impl Sealer for Seal {
            fn seal(
                &mut self,
                target: &str,
                event: &str,
                data: &str,
            ) -> Result<Option<String>, String> {
                assert_eq!(target, "#hmi");
                if event == "never" {
                    return Err("this one does not leave".to_string());
                }
                Ok(Some(alloc::format!("{data}+seal")))
            }
        }
        let mut ecu = endpoint("ecu", "hmi", None);
        ecu.set_sealer(Box::new(Seal));
        ecu.send(&send("go", "s-1"));
        let (_, go) = ecu.transport.sent.pop().unwrap();
        ecu.send(&send("stop", "s-2"));
        let (_, stop) = ecu.transport.sent.pop().unwrap();

        let mut hmi = endpoint("hmi", "ecu", None);
        hmi.set_admission(Box::new(OnlyShort));
        hmi.receive("ecu", go);
        hmi.receive("ecu", stop);
        let events = raised(&mut hmi);
        assert_eq!(
            events.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["go"]
        );
        assert_eq!(
            events[0].metadata.data, r#"{"a":1}"#,
            "the machine is told what it sent, without what the host wrote"
        );
        assert_eq!(
            hmi.take_host_errors(),
            vec![RouterError::EventNotAdmitted {
                peer: "ecu".to_string(),
                event: "stop".to_string(),
                reason: "stop is not for this link".to_string(),
            }]
        );

        // A send the sealer says must not leave is dropped, told to the host, and nothing goes
        // in its place.
        ecu.send(&send("never", "s-3"));
        assert!(ecu.transport.sent.is_empty());
        assert_eq!(
            ecu.take_host_errors(),
            vec![RouterError::EventNotSealed {
                target: "#hmi".to_string(),
                event: "never".to_string(),
                reason: "this one does not leave".to_string(),
            }]
        );
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
            r#"{"errorName":"communication","reason":"sendFailed","target":"hmi","transport":"wss","transport_error":"reset"}"#
        );
        // Raised as the C++ core raises it: an envelope from its own machine.
        assert_eq!(event.metadata.origin_address(), "mesh://ecu");
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
            r#"{"errorName":"communication","reason":"deliveryExhausted","target":"hmi","transport":"wss","transport_error":"reset","attempts":2}"#
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

    /// A transport whose own queue for the peer is full (the HTTPS binding's mailbox) drops the
    /// envelope as `backpressureDrop`, row 9, with the depth it found: not `sendFailed`, and not
    /// retried, even under a policy that would retry any other failure.
    #[test]
    fn a_full_transport_queue_is_backpressure_drop_and_never_retried() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_backoff_ms: 10,
            backoff_multiplier: 1.0,
            max_backoff_ms: 10,
            jitter_pct: 0,
        };
        let mut ecu = endpoint("ecu", "hmi", Some(policy));
        ecu.transport.failures.push_back(TransportFailure {
            retryable: true,
            message: Some("the mailbox is full".to_string()),
            queue_full: Some(3),
        });
        ecu.send(&send("go", "s-1"));
        let events = raised(&mut ecu);
        let [event] = events.as_slice() else {
            panic!("expected one row, got {events:?}");
        };
        assert_eq!(
            event.metadata.data,
            r#"{"errorName":"communication","reason":"backpressureDrop","target":"hmi","transport":"wss","queue_depth":3}"#
        );
        assert_eq!(
            ecu.next_retry_due_ms(),
            None,
            "nothing waits to be sent again"
        );
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

    /// `cloud`, checkpointing, whose instances of `phone` send to it with durable receipt
    /// (docs/adr/0113, D5): the binding a server whose document reads a person is held to.
    fn open_cloud() -> Endpoint<Recorder, Fixed> {
        let mut router = Router::new("cloud", 8, 50).unwrap();
        router
            .open_machine(
                "phone",
                PeerConfig {
                    transport: "https",
                    buffer: None,
                    retry: None,
                    stamp_sequence: false,
                    delivery: Delivery {
                        dedup: true,
                        ordered: false,
                    },
                    responders: Responders::Instance,
                    deadline_ms: None,
                    reply_events: &[],
                    causal: None,
                },
                4,
            )
            .unwrap();
        router.enable_checkpoints();
        router.receive_durably("phone").unwrap();
        Endpoint::new(router, Recorder::default(), Fixed::default())
    }

    /// `ping`, which the instance `a1` of `phone` numbered `number` in its incarnation.
    fn ping_of_a1(number: u8) -> Vec<u8> {
        use crate::generated::envelope::Envelope;
        use crate::generated::pattern_kind::PatternKind;
        use crate::generated::payload_codec::PayloadCodec;
        use crate::router::within;
        Envelope {
            id: (&[0x40 + number; 16]).into(),
            source: within("phone").unwrap(),
            instance: Some(within("a1").unwrap()),
            event_type: within("ping").unwrap(),
            pattern: PatternKind::FireForget,
            datacontenttype: PayloadCodec::None,
            sequence_no: Some(u64::from(number)),
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    fn alice() -> Principal {
        Principal {
            issuer: "https://id.example".to_string(),
            subject: "alice".to_string(),
            proved_ms: 1_000,
            method: "test".to_string(),
            instance: "a1".to_string(),
        }
    }

    /// docs/adr/0116, D1 and D3: an event an instance
    /// sent while its incarnation acts for a person carries the person the edge read from the
    /// store, and one it sent under an incarnation that acts for nobody carries none. Red if
    /// `receive_member_for` does not stamp the events the envelope raised, or if `Told::call`
    /// drops the person on the way to the engine.
    #[test]
    fn an_instances_event_carries_the_person_its_incarnation_acts_for() {
        let mut cloud = open_cloud();
        let standing = Standing {
            incarnation: 7,
            mark: 1,
        };
        cloud
            .receive_member_for("phone/a1", 7, standing, Some(alice()), ping_of_a1(1))
            .expect("an incarnation that stands is taken in");
        cloud
            .receive_member("phone/a1", 7, standing, ping_of_a1(2))
            .expect("an incarnation that stands is taken in");
        let events = raised(&mut cloud);
        let people: Vec<(&str, Option<&Principal>)> = events
            .iter()
            .map(|event| (event.name.as_str(), event.metadata.principal.as_deref()))
            .collect();
        assert_eq!(
            people,
            [("ping", Some(&alice())), ("ping", None)],
            "{:?}",
            cloud.take_host_errors()
        );
    }

    /// docs/adr/0116, D6: a checkpoint holds no person, so an endpoint that holds an event said
    /// for one and not yet applied refuses to seal, and seals the same number once the host has
    /// taken it. Red if `seal` writes the event without its person, which a restore would then
    /// run as nobody's.
    #[test]
    fn an_endpoint_that_holds_an_event_of_a_person_seals_only_once_it_is_applied() {
        let mut cloud = open_cloud();
        let standing = Standing {
            incarnation: 7,
            mark: 1,
        };
        cloud
            .receive_member_for("phone/a1", 7, standing, Some(alice()), ping_of_a1(1))
            .expect("an incarnation that stands is taken in");
        assert!(
            matches!(cloud.seal(1), Err(CheckpointError::Person(_))),
            "an unapplied event of a person is not sealed"
        );
        assert_eq!(raised(&mut cloud).len(), 1);
        assert!(
            cloud.seal(1).is_ok(),
            "the same number seals once it is taken"
        );
    }
}
