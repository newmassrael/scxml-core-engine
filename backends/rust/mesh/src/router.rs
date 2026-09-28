// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The router core: one machine's Mesh endpoint, joining the send half
//! ([`crate::outbound`]) and the receive half ([`crate::inbound`]) to the
//! engine's host-processor surface.
//!
//! A `<send target="#peer">` reaches a host as a `HostSendRequest` whose
//! `event_data` is the text a local delivery would have carried in
//! `_event.data`; [`Router::send`] turns it into an envelope for that peer.
//! Envelope bytes a transport received go through [`Router::receive`], which
//! answers with the events the engine may now raise, in order.
//!
//! Sans-IO, like both halves: every call returns [`Effect`]s and the host
//! performs them — it owns the transport, the clock and the randomness an
//! envelope id needs. That is what lets the same core run under a test, a
//! WebSocket or an in-process loopback, and it is the shape SCE_MESH.md
//! §mesh-6.4 gives a transport: the transport moves bytes, the core decides.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use sce_forge_runtime::algorithm::AlgorithmError;
use sce_forge_runtime::codec::{CodecError, SceCursor};
use sce_rust_runtime::{HostInvokeRequest, HostSendRequest};

use crate::generated::envelope::Envelope;
use crate::generated::pattern_kind::PatternKind;
use crate::generated::payload_codec::PayloadCodec;
use crate::generated::rpc_status::RpcStatus;
use crate::inbound::{AdmitError, ConfigError, Delivery, Inbound, Outcome};
use crate::outbound::{Admitted, Outbound, OutboundBuffer, RetryPolicy};
use crate::rpc::{self, Answer, Correlation, Pending, MESH_DEADLINE_PARAM, MESH_EVENT_PARAM};
use crate::signal::{Binding, Signal};

/// What deployment says about one peer this machine talks to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeerConfig {
    /// The binding's transport kind (`"wss"`, `"custom_tcp"`, ...): the
    /// `transport` column of the §mesh-16.7 rows observed on it.
    pub transport: &'static str,
    /// deploy.yaml's `outbound_buffer:` section, or `None` when the machine
    /// declares none — then every envelope is dispatched directly (§mesh-10.10).
    pub buffer: Option<OutboundBuffer>,
    /// deploy.yaml's `retry` block, if the binding has one (§mesh-10.10).
    pub retry: Option<RetryPolicy>,
    /// Whether envelopes TO this peer carry a `sequence_no`: the binding
    /// declares `ordering: required` on a transport that does not itself
    /// order (§mesh-10.6.3).
    pub stamp_sequence: bool,
    /// How envelopes FROM this peer are delivered (§mesh-10.5, §mesh-10.6).
    pub delivery: Delivery,
    /// The machines whose reply may answer a request sent to this peer —
    /// the binding's `reply_from:`, or the peer alone (§mesh-14.6). Never
    /// empty.
    pub responders: &'static [&'static str],
    /// deploy.yaml's binding-level request deadline, which a request to this
    /// peer takes when it carries no `_mesh_deadline_ms`; `None` lets it wait
    /// for its reply indefinitely (§mesh-9.5).
    pub deadline_ms: Option<u64>,
    /// The events this machine sends to the peer as replies
    /// (`service.response.*`, §mesh-8.1). One goes out as `RpcReply` with the
    /// invokeid of the request being handled, which is how the requester
    /// matches it (§mesh-10.7); every other send is `FireForget`.
    pub reply_events: &'static [&'static str],
}

/// One thing the host must do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Hand `bytes` to the transport bound to `peer`.
    Transmit { peer: String, bytes: Vec<u8> },
    /// Raise `event` on the engine's external queue with `data` as
    /// `_event.data`; `source` is the machine that sent it, `send_id` the
    /// id its `<send>` carried (§mesh-10.7: the envelope's `subject`), and
    /// `invoke_id` the request's wire invokeid as hex when the envelope is an
    /// `RpcRequest` — the one field that comes back on the reply
    /// (§mesh-10.7).
    Deliver {
        event: String,
        data: String,
        source: String,
        send_id: Option<String>,
        invoke_id: Option<String>,
    },
    /// Raise `error.communication` with this §mesh-16.7 row. `peer` names the
    /// binding the row is about, when there is one.
    Raise {
        peer: Option<String>,
        signal: Signal,
    },
    /// End the start `token` of the SCXML invocation `invoke_id` with
    /// `done.invoke.<invoke_id>`: `data` is the reply's payload text and
    /// `source` the machine that answered, whose `mesh://<source>` is the
    /// event's `_event.origin` (§mesh-9.5).
    Complete {
        invoke_id: String,
        token: u64,
        data: String,
        source: String,
    },
    /// End it with `error.invoke.<invoke_id>` instead: `data` is the
    /// §mesh-10.7.1 `errorName: "invoke"` object, and `source` the machine
    /// that answered — `None` when the requester's own deadline ended it.
    Fail {
        invoke_id: String,
        token: u64,
        data: String,
        source: Option<String>,
    },
}

/// What became of a request [`Router::invoke`] was handed.
#[derive(Debug, Clone, PartialEq)]
pub enum Invoked {
    /// It is on its way (or queued for its peer); these are the effects of
    /// sending it, and its answer arrives later as [`Effect::Complete`] or
    /// [`Effect::Fail`].
    Started(Vec<Effect>),
    /// It could not reach the wire, so the invocation never starts: the host
    /// refuses it with this `_event.data`, and the engine raises
    /// `error.execution` (§mesh-9.5's pre-envelope tier).
    Refused(String),
}

struct Peer {
    outbound: Outbound,
    config: PeerConfig,
    next_sequence: u64,
}

/// One machine's Mesh endpoint.
pub struct Router {
    machine: String,
    inbound: Inbound,
    peers: BTreeMap<String, Peer>,
    requests: Correlation,
}

impl Router {
    /// A router for `machine`, with the receive side's dedup window and gap
    /// timeout (deploy.yaml `window_size`, `gap_timeout_ms`).
    pub fn new(machine: &str, dedup_window: u32, gap_timeout_ms: i64) -> Result<Self, ConfigError> {
        Ok(Self {
            machine: machine.to_string(),
            inbound: Inbound::new(dedup_window, gap_timeout_ms)?,
            peers: BTreeMap::new(),
            requests: Correlation::default(),
        })
    }

    /// Bind `peer` — the name a document writes as `#peer`.
    pub fn add_peer(&mut self, peer: &str, config: PeerConfig) {
        self.peers.insert(
            peer.to_string(),
            Peer {
                outbound: Outbound::new(config.buffer),
                config,
                next_sequence: 1,
            },
        );
    }

    /// Perform a `<send target="#peer">` the engine handed the host. `id` is
    /// the envelope id the host drew (a UUID v7), `now_ms` its monotonic
    /// clock.
    ///
    /// A Mesh target this router has no binding for is unreachable, and says
    /// so as §mesh-16.7 row 1: the router is registered, so this is not the
    /// `error.execution` of a processor nobody serves.
    pub fn send(
        &mut self,
        request: &HostSendRequest,
        id: [u8; 16],
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        let peer_name = mesh_peer(&request.target)
            .ok_or_else(|| RouterError::NotMeshTarget(request.target.clone()))?;
        let Some(peer) = self.peers.get_mut(peer_name) else {
            return Ok(alloc::vec![Effect::Raise {
                peer: Some(peer_name.to_string()),
                signal: Signal::TransportUnavailable,
            }]);
        };
        // §mesh-8.1, §mesh-10.7: a reply the deployment names for this peer
        // goes out as `RpcReply`, carrying the invokeid of the request the
        // document was handling when it sent it — the one field that makes
        // the round trip. A reply sent with no request current carries none,
        // and the requester delivers it as the event it names.
        let reply = peer
            .config
            .reply_events
            .contains(&request.event_name.as_str());
        let answering = if reply {
            rpc::unhex(&request.invoke_id)
        } else {
            None
        };
        let envelope = Envelope {
            id: &id,
            source: &self.machine,
            event_type: &request.event_name,
            pattern: if reply {
                PatternKind::RpcReply
            } else {
                PatternKind::FireForget
            },
            datacontenttype: payload_codec(&request.event_data),
            data: request.event_data.as_bytes(),
            // §mesh-10.7: the receiver's `_event.sendid` is the envelope's
            // `subject`, so a `<send>`'s id travels there.
            subject: (!request.send_id.is_empty()).then_some(request.send_id.as_str()),
            invoke_id: answering.as_ref().map(|id| &id[..]),
            rpc_status: reply.then_some(RpcStatus::Ok),
            ..Envelope::new()
        };
        transmit(peer_name, peer, envelope, now_ms)
    }

    /// Start an `<invoke type="sce:mesh-rpc">` the engine handed the host
    /// (§mesh-9.5): write its `RpcRequest` and keep it until its reply, its
    /// deadline or its cancellation ends it.
    ///
    /// `wire_id` is the `invoke_id` the host minted for this invocation and
    /// `id` the envelope's own id — two UUID v7s, so correlation and dedup
    /// never share a key (§mesh-9.5). `now_ms` is the host's monotonic clock,
    /// which the deadline is kept against, and `now_unix_ms` its wall clock,
    /// which the wire's `deadline_unix_ms` is written in.
    ///
    /// A target that names no Mesh peer, or one this router has no binding
    /// for, cannot reach the wire: the invocation is [`Invoked::Refused`].
    pub fn invoke(
        &mut self,
        request: &HostInvokeRequest,
        wire_id: [u8; 16],
        id: [u8; 16],
        now_ms: i64,
        now_unix_ms: u64,
    ) -> Result<Invoked, RouterError> {
        let Some(peer_name) = mesh_peer(&request.src) else {
            return Ok(Invoked::Refused(rpc::src_not_found_data(&alloc::format!(
                "'{}' names no Mesh peer",
                request.src
            ))));
        };
        let Some(peer) = self.peers.get_mut(peer_name) else {
            return Ok(Invoked::Refused(rpc::src_not_found_data(&alloc::format!(
                "no binding for '#{peer_name}'"
            ))));
        };
        let event =
            param(request, MESH_EVENT_PARAM).ok_or(RouterError::MissingParam(MESH_EVENT_PARAM))?;
        // §mesh-9.5 deadline precedence: the invoke's own param, else the
        // binding's, else none.
        let deadline_ms = match param(request, MESH_DEADLINE_PARAM) {
            Some(text) => Some(text.parse::<u64>().map_err(|_| RouterError::BadParam {
                name: MESH_DEADLINE_PARAM,
                value: text.to_string(),
            })?),
            None => peer.config.deadline_ms,
        };
        let envelope = Envelope {
            id: &id,
            source: &self.machine,
            event_type: event,
            pattern: PatternKind::RpcRequest,
            datacontenttype: payload_codec(&request.event_data),
            data: request.event_data.as_bytes(),
            invoke_id: Some(&wire_id),
            deadline_unix_ms: deadline_ms.map(|ms| now_unix_ms.saturating_add(ms)),
            ..Envelope::new()
        };
        let responders = peer.config.responders;
        let effects = transmit(peer_name, peer, envelope, now_ms)?;
        self.requests.register(
            wire_id,
            Pending {
                invoke_id: request.invoke_id.clone(),
                token: request.token,
                responders,
                expires_ms: deadline_ms
                    .map(|ms| now_ms.saturating_add(i64::try_from(ms).unwrap_or(i64::MAX))),
            },
        );
        Ok(Invoked::Started(effects))
    }

    /// The state that started the start `token` of `invoke_id` exited: stop
    /// waiting for its answer. Nothing goes on the wire, and an answer that
    /// arrives later is dropped (§mesh-9.5, `<cancel>`).
    pub fn cancel_invoke(&mut self, invoke_id: &str, token: u64) {
        self.requests.cancel(invoke_id, token);
    }

    /// `peer`'s transport became ready: release what waited for it.
    pub fn peer_ready(&mut self, peer: &str, now_ms: i64) -> Result<Vec<Effect>, RouterError> {
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        let drained = bound
            .outbound
            .mark_ready(now_ms)
            .map_err(RouterError::Rule)?;
        let mut effects: Vec<Effect> = drained
            .send
            .into_iter()
            .map(|bytes| Effect::Transmit {
                peer: peer.to_string(),
                bytes,
            })
            .collect();
        effects.extend(drained.signals.into_iter().map(|signal| Effect::Raise {
            peer: Some(peer.to_string()),
            signal,
        }));
        Ok(effects)
    }

    /// `peer`'s transport stopped being ready.
    pub fn peer_not_ready(&mut self, peer: &str) -> Result<Vec<Effect>, RouterError> {
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        Ok(bound
            .outbound
            .mark_not_ready()
            .map(|signal| Effect::Raise {
                peer: Some(peer.to_string()),
                signal,
            })
            .into_iter()
            .collect())
    }

    /// The retry policy the host applies when a transmission to `peer`
    /// fails (see [`crate::outbound::Attempts`]).
    pub fn retry_policy(&self, peer: &str) -> Option<RetryPolicy> {
        self.peers.get(peer).and_then(|bound| bound.config.retry)
    }

    /// The machine this router speaks for — the `source` of what it sends.
    pub fn machine(&self) -> &str {
        &self.machine
    }

    /// The binding a raised row names: the peer an [`Effect::Raise`]
    /// carries, with the transport the deployment bound it to, so the
    /// row's `target` and `transport` columns are the deployment's. A peer
    /// the deployment never bound is named with no transport.
    pub fn binding<'a>(&'a self, peer: Option<&'a str>) -> Option<Binding<'a>> {
        peer.map(|peer| Binding {
            peer,
            transport: self.peers.get(peer).map(|bound| bound.config.transport),
        })
    }

    /// Envelope `bytes` a transport received from `peer`, at `now_ms`.
    ///
    /// What the document is told about is an effect; what only the host can
    /// act on is an error. An envelope that cannot be read is §mesh-16.7 row 4
    /// and one an ordered binding cannot place is row 11, so both are
    /// effects. A hold that is full or a sequence at the `u64` maximum is a
    /// limit of this receiver, which no row names and distribution may not
    /// invent one for (§mesh-16.7, "Scope of synthesis"), so it is returned.
    pub fn receive(
        &mut self,
        peer: &str,
        bytes: Vec<u8>,
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        let delivery = self
            .peers
            .get(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?
            .config
            .delivery;
        let raise = |signal| {
            Ok(alloc::vec![Effect::Raise {
                peer: Some(peer.to_string()),
                signal,
            }])
        };
        // §mesh-14.6: a reply is checked against its request's responder set
        // on arrival, while the binding it came in on is still known — an
        // ordered binding may hold it and release it on a later tick. One
        // from outside the set is refused before the dedup window or the
        // ordering hold sees it, so it takes no slot in either.
        if let Some(signal) = self.undeclared_reply(&bytes, peer) {
            return raise(signal);
        }
        match self.inbound.admit(bytes, delivery, now_ms) {
            Ok(outcome) => Ok(self.effects_of(outcome, Some(peer))),
            // The bytes are the envelope's CBOR (§mesh-7.5); nothing in
            // them can be trusted to name a source.
            Err(AdmitError::Malformed(_)) => raise(Signal::EnvelopeCorrupt {
                source: None,
                codec: "cbor",
            }),
            Err(AdmitError::Unstamped) => raise(Signal::MissingSequence {
                source: peer.to_string(),
            }),
            Err(AdmitError::Rule(error)) => Err(RouterError::Rule(error)),
            Err(AdmitError::HoldFull) => Err(RouterError::HoldFull {
                peer: Some(peer.to_string()),
            }),
        }
    }

    /// End every ordering gap that has waited out its timeout by `now_ms`,
    /// then every request whose deadline has passed: each ends in
    /// `error.invoke` with `deadlineExceeded`, the same shape a peer's
    /// `DeadlineExceeded` reply takes (§mesh-10.7.1).
    pub fn tick(&mut self, now_ms: i64) -> Result<Vec<Effect>, RouterError> {
        let mut effects = match self.inbound.tick(now_ms) {
            Ok(outcome) => self.effects_of(outcome, None),
            Err(AdmitError::Rule(error)) => return Err(RouterError::Rule(error)),
            Err(AdmitError::HoldFull) => return Err(RouterError::HoldFull { peer: None }),
            // A tick reads no bytes and stamps nothing.
            Err(AdmitError::Malformed(_)) | Err(AdmitError::Unstamped) => {
                unreachable!("Inbound::tick decodes no envelope")
            }
        };
        effects.extend(
            self.requests
                .expire(now_ms)
                .into_iter()
                .map(|(wire_id, pending)| Effect::Fail {
                    invoke_id: pending.invoke_id,
                    token: pending.token,
                    data: rpc::invoke_error_data(RpcStatus::DeadlineExceeded, None, None, &wire_id),
                    source: None,
                }),
        );
        Ok(effects)
    }

    /// Row 14 for `bytes`, if they are a reply to a live request that
    /// arrived on a binding outside its responder set. Bytes that do not
    /// decode are left for the receive half to report.
    fn undeclared_reply(&self, bytes: &[u8], peer: &str) -> Option<Signal> {
        let envelope = Envelope::decode(&mut SceCursor::new(bytes)).ok()?;
        if envelope.pattern != PatternKind::RpcReply {
            return None;
        }
        let wire_id: [u8; 16] = envelope.invoke_id?.try_into().ok()?;
        match self.requests.check(&wire_id, peer) {
            Answer::Undeclared => Some(Signal::RpcReplyFromUndeclaredPeer {
                source: envelope.source.to_string(),
                invoke_id: rpc::hex(&wire_id),
            }),
            Answer::Admitted | Answer::Unknown => None,
        }
    }

    /// Turn what the receive half released into effects, in release order,
    /// followed by the rows it observed.
    fn effects_of(&mut self, outcome: Outcome, peer: Option<&str>) -> Vec<Effect> {
        let mut effects: Vec<Effect> = outcome
            .released
            .iter()
            .filter_map(|received| {
                let envelope = received.envelope();
                match reply_to(&envelope) {
                    // Its responder was checked on arrival.
                    Some(wire_id) => self.answer(&envelope, &wire_id, peer),
                    None => Some(deliver(&envelope, peer)),
                }
            })
            .collect();
        effects.extend(outcome.signals.into_iter().map(|signal| Effect::Raise {
            peer: peer.map(str::to_string),
            signal,
        }));
        effects
    }
}

/// The wire id of the request `envelope` answers, when it is a reply that
/// carries one. A reply to a request this router never sent, or one already
/// retired, is then dropped rather than delivered: this core sends every
/// request with an `invoke_id`, so nothing else can be waiting for it.
fn reply_to(envelope: &Envelope<'_>) -> Option<[u8; 16]> {
    if envelope.pattern != PatternKind::RpcReply {
        return None;
    }
    envelope.invoke_id?.try_into().ok()
}

impl Router {
    /// How a released reply ends the request `wire_id` (§mesh-9.5), or
    /// `None` when no request is waiting on it any more — it was answered,
    /// cancelled or expired, and a late answer is dropped.
    ///
    /// `Ok` — or no status at all, which the requester reads as `Ok` as the
    /// C++ core does — completes it with the reply's payload; any other
    /// status fails it. An `Ok` whose payload the engine cannot be handed is
    /// §mesh-16.7 row 4 and ends nothing: the request stays waiting for an
    /// answer it can use, or for its deadline.
    fn answer(
        &mut self,
        envelope: &Envelope<'_>,
        wire_id: &[u8; 16],
        peer: Option<&str>,
    ) -> Option<Effect> {
        let source = envelope.source.to_string();
        let status = envelope.rpc_status.unwrap_or(RpcStatus::Ok);
        let completion = match status {
            RpcStatus::Ok => match payload_text(envelope) {
                Ok(data) => Ok(data),
                Err(codec) => {
                    return self.requests.is_waiting(wire_id).then(|| Effect::Raise {
                        peer: peer.map(str::to_string),
                        signal: Signal::EnvelopeCorrupt {
                            source: Some(source),
                            codec,
                        },
                    });
                }
            },
            status => Err(status),
        };
        let pending = self.requests.retire(wire_id)?;
        Some(match completion {
            Ok(data) => Effect::Complete {
                invoke_id: pending.invoke_id,
                token: pending.token,
                data,
                source,
            },
            Err(status) => Effect::Fail {
                invoke_id: pending.invoke_id,
                token: pending.token,
                data: rpc::invoke_error_data(
                    status,
                    envelope.rpc_error_message,
                    Some(&source),
                    wire_id,
                ),
                source: Some(source),
            },
        })
    }
}

/// Why the router could not do what the host asked. None of these is a
/// §mesh-16.7 row: each is the host's to act on, not the document's.
#[derive(Debug, Clone, PartialEq)]
pub enum RouterError {
    /// The host handed [`Router::send`] a target that is not a Mesh peer
    /// reference (see [`mesh_peer`]).
    NotMeshTarget(String),
    /// The host named a peer it never bound with [`Router::add_peer`].
    UnknownPeer(String),
    /// A standard rule refused its input: a sequence at the `u64` maximum,
    /// or a clock reading that cannot be subtracted.
    Rule(AlgorithmError),
    /// A sender has more envelopes held ahead of sequence than the ordering
    /// rule's list carries. `peer` is the binding that delivered the one
    /// that overflowed; a tick, which delivers nothing, cannot name one.
    HoldFull { peer: Option<String> },
    /// The envelope could not be written within the bounds its document
    /// declares — an event whose data is longer than an envelope carries.
    Encode(CodecError),
    /// A Mesh request reached [`Router::invoke`] without a reserved param
    /// the build always gives it — a request the lowering did not make.
    MissingParam(&'static str),
    /// A reserved param's value is not what the build writes there.
    BadParam { name: &'static str, value: String },
}

/// The peer a `<send target>` names, when it names one — the runtime's
/// predicate, so the router and a generated send site's choice to reach it
/// are one answer rather than two copies of it.
pub use sce_rust_runtime::helpers::send::mesh_peer;

/// Write `envelope` for `peer` and hand it to that peer's send half: what
/// [`Router::send`] and [`Router::invoke`] share once they know what to say.
fn transmit(
    peer_name: &str,
    peer: &mut Peer,
    mut envelope: Envelope<'_>,
    now_ms: i64,
) -> Result<Vec<Effect>, RouterError> {
    envelope.sequence_no = peer.config.stamp_sequence.then_some(peer.next_sequence);
    let bytes = envelope.encode_to_vec().map_err(RouterError::Encode)?;
    // §mesh-10.6.3: the counter advances once per envelope written, so one
    // that could not be written leaves no hole; and it has no wrap guard,
    // 2^64 sends being beyond any deployment's lifetime.
    if peer.config.stamp_sequence {
        peer.next_sequence = peer.next_sequence.wrapping_add(1);
    }
    Ok(match peer.outbound.admit(bytes, now_ms) {
        Admitted::Send(bytes) => alloc::vec![Effect::Transmit {
            peer: peer_name.to_string(),
            bytes,
        }],
        Admitted::Queued => Vec::new(),
        Admitted::Dropped(signal) => alloc::vec![Effect::Raise {
            peer: Some(peer_name.to_string()),
            signal,
        }],
    })
}

/// The codec a payload of the sending engine's own `_event.data` text
/// travels in: JSON, or nothing when there is none — the rule the C++ core
/// applies to a child's donedata (sce/include/mesh/ChildSessionAdapter.h).
fn payload_codec(event_data: &str) -> PayloadCodec {
    if event_data.is_empty() {
        PayloadCodec::None
    } else {
        PayloadCodec::Json
    }
}

/// An envelope's payload as the text the engine is handed. A JSON or empty
/// payload is the `_event.data` text the sender's engine wrote; a payload in
/// another codec is bytes the engine's text surface cannot be handed, named
/// by that codec so §mesh-16.7 row 4 can say which.
fn payload_text(envelope: &Envelope<'_>) -> Result<String, &'static str> {
    match envelope.datacontenttype {
        PayloadCodec::None => Ok(String::new()),
        PayloadCodec::Json => core::str::from_utf8(envelope.data)
            .map(str::to_string)
            .map_err(|_| "json"),
        PayloadCodec::Cbor => Err("cbor"),
        PayloadCodec::Typed => Err("typed"),
        PayloadCodec::Raw => Err("raw"),
    }
}

/// The first value the request carries for the `<param>` `name`.
fn param<'a>(request: &'a HostInvokeRequest, name: &str) -> Option<&'a str> {
    request.params.get(name)?.first().map(String::as_str)
}

/// The event an admitted envelope raises, or §mesh-16.7 row 4 when its
/// payload is not text the engine can be handed (see [`payload_text`]).
/// `peer` is the binding it arrived on, when a receipt rather than a tick
/// released it.
fn deliver(envelope: &Envelope<'_>, peer: Option<&str>) -> Effect {
    match payload_text(envelope) {
        Ok(data) => Effect::Deliver {
            event: envelope.event_type.to_string(),
            data,
            source: envelope.source.to_string(),
            send_id: envelope.subject.map(str::to_string),
            // §mesh-10.7: `_event.invokeid` of an inbound request is its
            // wire invokeid, as hex.
            invoke_id: (envelope.pattern == PatternKind::RpcRequest)
                .then_some(envelope.invoke_id)
                .flatten()
                .map(rpc::hex),
        },
        Err(codec) => Effect::Raise {
            peer: peer.map(str::to_string),
            signal: Signal::EnvelopeCorrupt {
                source: Some(envelope.source.to_string()),
                codec,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A binding with ordering required on a transport that does not order:
    /// the sender stamps and the receiver orders.
    const ORDERED: PeerConfig = PeerConfig {
        transport: "wss",
        buffer: Some(OutboundBuffer {
            max_pending: 4,
            max_age_ms: 0,
        }),
        retry: None,
        stamp_sequence: true,
        delivery: Delivery {
            dedup: true,
            ordered: true,
        },
        // The peer these tests bind is `hmi`; `pair` gives each side its own.
        responders: &["hmi"],
        deadline_ms: None,
        reply_events: &[],
    };

    const UNORDERED: PeerConfig = PeerConfig {
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        ..ORDERED
    };

    /// §mesh-10.10: a binding on a machine with no `outbound_buffer:` section
    /// is dispatched directly — a send to a peer nothing has called ready is
    /// transmitted at once rather than held, and the transport reports what it
    /// could not deliver.
    #[test]
    fn without_a_buffer_a_send_is_transmitted_before_the_peer_is_ready() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        ecu.add_peer(
            "hmi",
            PeerConfig {
                buffer: None,
                ..UNORDERED
            },
        );
        let effects = ecu.send(&request("#hmi", "ping", ""), [7; 16], 0).unwrap();
        assert!(
            matches!(effects.as_slice(), [Effect::Transmit { peer, .. }] if peer == "hmi"),
            "{effects:?}"
        );
    }

    /// Two routers bound to each other, the sender's transport ready.
    fn pair(config: PeerConfig) -> (Router, Router) {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        ecu.add_peer(
            "hmi",
            PeerConfig {
                responders: &["hmi"],
                ..config
            },
        );
        hmi.add_peer(
            "ecu",
            PeerConfig {
                responders: &["ecu"],
                ..config
            },
        );
        assert!(ecu.peer_ready("hmi", 0).unwrap().is_empty());
        (ecu, hmi)
    }

    fn events(effects: &[Effect]) -> Vec<&str> {
        effects
            .iter()
            .map(|effect| match effect {
                Effect::Deliver { event, .. } => event.as_str(),
                other => panic!("expected only deliveries, got {other:?}"),
            })
            .collect()
    }

    fn request(target: &str, event: &str, data: &str) -> HostSendRequest {
        HostSendRequest {
            processor_type: "sce:mesh".to_string(),
            event_name: event.to_string(),
            target: target.to_string(),
            send_id: "send.1".to_string(),
            event_data: data.to_string(),
            ..HostSendRequest::default()
        }
    }

    fn transmitted(effects: Result<Vec<Effect>, RouterError>) -> Vec<u8> {
        match effects.unwrap().as_slice() {
            [Effect::Transmit { peer, bytes }] if peer == "hmi" => bytes.clone(),
            other => panic!("expected one transmission to hmi, got {other:?}"),
        }
    }

    #[test]
    fn a_send_to_a_ready_peer_crosses_as_the_engines_event_data() {
        let (mut ecu, mut hmi) = pair(ORDERED);
        let bytes = transmitted(ecu.send(
            &request("#hmi", "speed.changed", r#"{"kph":42}"#),
            [1; 16],
            0,
        ));
        assert_eq!(
            hmi.receive("ecu", bytes, 0).unwrap(),
            alloc::vec![Effect::Deliver {
                event: "speed.changed".to_string(),
                data: r#"{"kph":42}"#.to_string(),
                source: "ecu".to_string(),
                send_id: Some("send.1".to_string()),
                invoke_id: None,
            }]
        );
    }

    #[test]
    fn a_send_with_no_data_travels_with_no_payload() {
        let (mut ecu, mut hmi) = pair(ORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "ping", ""), [2; 16], 0));
        let envelope =
            Envelope::decode(&mut sce_forge_runtime::codec::SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.datacontenttype, PayloadCodec::None);
        assert_eq!(
            hmi.receive("ecu", bytes.clone(), 0).unwrap(),
            alloc::vec![Effect::Deliver {
                event: "ping".to_string(),
                data: String::new(),
                source: "ecu".to_string(),
                send_id: Some("send.1".to_string()),
                invoke_id: None,
            }]
        );
    }

    #[test]
    fn an_ordered_binding_stamps_from_one_so_the_receiver_restores_send_order() {
        let (mut ecu, mut hmi) = pair(ORDERED);
        let sent: Vec<_> = ["a", "b", "c"]
            .iter()
            .zip(1u8..)
            .map(|(event, id)| transmitted(ecu.send(&request("#hmi", event, ""), [id; 16], 0)))
            .collect();
        let stamps: Vec<_> = sent
            .iter()
            .map(|bytes| {
                Envelope::decode(&mut sce_forge_runtime::codec::SceCursor::new(bytes))
                    .unwrap()
                    .sequence_no
            })
            .collect();
        assert_eq!(stamps, alloc::vec![Some(1), Some(2), Some(3)]);

        assert_eq!(
            events(&hmi.receive("ecu", sent[0].clone(), 0).unwrap()),
            ["a"]
        );
        assert!(hmi.receive("ecu", sent[2].clone(), 0).unwrap().is_empty());
        assert_eq!(
            events(&hmi.receive("ecu", sent[1].clone(), 0).unwrap()),
            ["b", "c"]
        );
    }

    #[test]
    fn an_unordered_binding_does_not_stamp() {
        let (mut ecu, _) = pair(UNORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "a", ""), [1; 16], 0));
        let envelope =
            Envelope::decode(&mut sce_forge_runtime::codec::SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.sequence_no, None);
    }

    #[test]
    fn an_unstamped_envelope_on_an_ordered_binding_is_missing_its_sequence() {
        let (mut ecu, _) = pair(UNORDERED);
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer("ecu", ORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "a", ""), [1; 16], 0));
        assert_eq!(
            hmi.receive("ecu", bytes, 0).unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("ecu".to_string()),
                signal: Signal::MissingSequence {
                    source: "ecu".to_string(),
                },
            }]
        );
    }

    #[test]
    fn a_resent_envelope_is_delivered_once() {
        let (mut ecu, mut hmi) = pair(UNORDERED);
        let bytes = transmitted(ecu.send(&request("#hmi", "a", ""), [7; 16], 0));
        assert_eq!(
            events(&hmi.receive("ecu", bytes.clone(), 0).unwrap()),
            ["a"]
        );
        assert!(hmi.receive("ecu", bytes, 1).unwrap().is_empty());
    }

    #[test]
    fn a_peer_not_yet_ready_holds_the_send_until_it_is() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        ecu.add_peer("hmi", ORDERED);
        assert!(ecu
            .send(&request("#hmi", "a", ""), [1; 16], 0)
            .unwrap()
            .is_empty());
        transmitted(ecu.peer_ready("hmi", 10));
    }

    #[test]
    fn a_mesh_target_with_no_binding_is_unreachable_not_unserved() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        assert_eq!(
            ecu.send(&request("#nowhere", "a", ""), [1; 16], 0).unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("nowhere".to_string()),
                signal: Signal::TransportUnavailable,
            }]
        );
    }

    #[test]
    fn a_target_that_is_not_a_mesh_peer_is_refused_to_the_host() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        for target in ["hmi", "#", "#_internal", "#_parent", ""] {
            assert_eq!(
                ecu.send(&request(target, "a", ""), [1; 16], 0),
                Err(RouterError::NotMeshTarget(target.to_string()))
            );
        }
    }

    #[test]
    fn bytes_that_are_not_an_envelope_are_reported_as_corrupt_cbor() {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer("ecu", ORDERED);
        assert_eq!(
            hmi.receive("ecu", alloc::vec![0xFF], 0).unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("ecu".to_string()),
                signal: Signal::EnvelopeCorrupt {
                    source: None,
                    codec: "cbor",
                },
            }]
        );
    }

    #[test]
    fn a_payload_the_engine_cannot_be_handed_is_corrupt_in_its_own_codec() {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer("ecu", UNORDERED);
        for (id, codec, name) in [
            (1u8, PayloadCodec::Cbor, "cbor"),
            (2, PayloadCodec::Typed, "typed"),
            (3, PayloadCodec::Raw, "raw"),
        ] {
            let bytes = Envelope {
                id: &[id; 16],
                source: "ecu",
                event_type: "a",
                pattern: PatternKind::FireForget,
                datacontenttype: codec,
                data: &[0x01],
                ..Envelope::new()
            }
            .encode_to_vec()
            .unwrap();
            assert_eq!(
                hmi.receive("ecu", bytes, 0).unwrap(),
                alloc::vec![Effect::Raise {
                    peer: Some("ecu".to_string()),
                    signal: Signal::EnvelopeCorrupt {
                        source: Some("ecu".to_string()),
                        codec: name,
                    },
                }]
            );
        }
    }

    #[test]
    fn a_raised_row_carries_the_binding_it_was_observed_on() {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        ecu.add_peer("hmi", ORDERED);
        let raised = ecu.send(&request("#nowhere", "a", ""), [1; 16], 0).unwrap();
        let [Effect::Raise { peer, signal }] = raised.as_slice() else {
            panic!("expected one raised row, got {raised:?}");
        };
        // An unbound target has no transport to name: its row says which
        // peer it was, and nothing a deployment did not declare.
        assert_eq!(
            signal.event_data(ecu.binding(peer.as_deref())),
            r#"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"nowhere"}"#
        );

        ecu.peer_ready("hmi", 0).unwrap();
        let lost = ecu.peer_not_ready("hmi").unwrap();
        let [Effect::Raise { peer, signal }] = lost.as_slice() else {
            panic!("expected one raised row, got {lost:?}");
        };
        assert_eq!(
            signal.event_data(ecu.binding(peer.as_deref())),
            r#"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"hmi","transport":"wss"}"#
        );
    }

    #[test]
    fn a_peer_the_host_never_bound_is_the_hosts_error() {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        assert_eq!(
            hmi.receive("ecu", alloc::vec![0xFF], 0),
            Err(RouterError::UnknownPeer("ecu".to_string()))
        );
        assert_eq!(
            hmi.peer_ready("ecu", 0),
            Err(RouterError::UnknownPeer("ecu".to_string()))
        );
    }

    // ── §mesh-9.5: `<invoke type="sce:mesh-rpc">`, the requester's half ──

    const WIRE: [u8; 16] = [9; 16];

    /// `ecu` bound to `hmi` with a binding-level deadline, and to `mallory`,
    /// which is not in `hmi`'s responder set.
    fn requester(deadline_ms: Option<u64>) -> Router {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        let config = PeerConfig {
            buffer: None,
            responders: &["hmi"],
            deadline_ms,
            ..UNORDERED
        };
        ecu.add_peer("hmi", config);
        ecu.add_peer(
            "mallory",
            PeerConfig {
                responders: &["mallory"],
                ..config
            },
        );
        ecu
    }

    fn ask(src: &str, deadline_ms: Option<&str>) -> HostInvokeRequest {
        let mut params = std::collections::HashMap::new();
        params.insert(
            MESH_EVENT_PARAM.to_string(),
            alloc::vec!["service.request.force".to_string()],
        );
        if let Some(ms) = deadline_ms {
            params.insert(MESH_DEADLINE_PARAM.to_string(), alloc::vec![ms.to_string()]);
        }
        HostInvokeRequest {
            processor_type: "sce:mesh-rpc".to_string(),
            invoke_id: "ask".to_string(),
            src: src.to_string(),
            params,
            event_data: r#"{"n":3}"#.to_string(),
            token: 7,
            ..HostInvokeRequest::default()
        }
    }

    /// Start `ask("#hmi")` at monotonic 10, wall 1000, returning the request
    /// envelope's bytes.
    fn started(ecu: &mut Router, deadline_ms: Option<&str>) -> Vec<u8> {
        match ecu.invoke(&ask("#hmi", deadline_ms), WIRE, [1; 16], 10, 1000) {
            Ok(Invoked::Started(effects)) => transmitted(Ok(effects)),
            other => panic!("expected the request to start, got {other:?}"),
        }
    }

    fn reply(id: u8, status: Option<RpcStatus>, message: Option<&str>, data: &str) -> Vec<u8> {
        Envelope {
            id: &[id; 16],
            source: "hmi",
            event_type: "service.response.force",
            pattern: PatternKind::RpcReply,
            datacontenttype: payload_codec(data),
            data: data.as_bytes(),
            invoke_id: Some(&WIRE),
            rpc_status: status,
            rpc_error_message: message,
            ..Envelope::new()
        }
        .encode_to_vec()
        .unwrap()
    }

    #[test]
    fn a_request_carries_its_event_payload_and_wire_ids() {
        let mut ecu = requester(Some(500));
        let bytes = started(&mut ecu, None);
        let envelope = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.pattern, PatternKind::RpcRequest);
        assert_eq!(envelope.event_type, "service.request.force");
        assert_eq!(envelope.data, br#"{"n":3}"#);
        assert_eq!(envelope.datacontenttype, PayloadCodec::Json);
        assert_eq!(envelope.invoke_id, Some(&WIRE[..]));
        assert_eq!(envelope.id, &[1; 16]);
        // §mesh-9.5 precedence: the binding's deadline when the invoke gives none.
        assert_eq!(envelope.deadline_unix_ms, Some(1500));
    }

    #[test]
    fn the_invokes_own_deadline_wins_over_the_bindings() {
        let mut ecu = requester(Some(500));
        let bytes = started(&mut ecu, Some("200"));
        let envelope = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.deadline_unix_ms, Some(1200));
    }

    #[test]
    fn a_request_with_no_deadline_carries_none() {
        let mut ecu = requester(None);
        let bytes = started(&mut ecu, None);
        let envelope = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.deadline_unix_ms, None);
        assert!(
            ecu.tick(i64::MAX).unwrap().is_empty(),
            "no deadline, no expiry"
        );
    }

    #[test]
    fn an_ok_reply_completes_the_invocation_once() {
        let mut ecu = requester(None);
        started(&mut ecu, None);
        let answer = reply(2, Some(RpcStatus::Ok), None, r#"{"force":12}"#);
        assert_eq!(
            ecu.receive("hmi", answer, 20).unwrap(),
            alloc::vec![Effect::Complete {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"force":12}"#.to_string(),
                source: "hmi".to_string(),
            }]
        );
        // A second answer finds nothing waiting.
        assert!(ecu
            .receive("hmi", reply(3, Some(RpcStatus::Ok), None, "1"), 21)
            .unwrap()
            .is_empty());
    }

    /// The key is optional on the wire; the requester reads its absence as
    /// `Ok`, as the C++ core does.
    #[test]
    fn a_reply_with_no_status_is_ok() {
        let mut ecu = requester(None);
        started(&mut ecu, None);
        assert!(matches!(
            ecu.receive("hmi", reply(2, None, None, ""), 20).unwrap().as_slice(),
            [Effect::Complete { data, .. }] if data.is_empty()
        ));
    }

    #[test]
    fn a_failed_reply_fails_the_invocation_with_the_status_by_name() {
        let mut ecu = requester(None);
        started(&mut ecu, None);
        let answer = reply(2, Some(RpcStatus::Unavailable), Some("busy"), "");
        assert_eq!(
            ecu.receive("hmi", answer, 20).unwrap(),
            alloc::vec![Effect::Fail {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"errorName":"invoke","reason":"unavailable","detail":"busy","source":"hmi","invoke_id":"09090909090909090909090909090909"}"#.to_string(),
                source: Some("hmi".to_string()),
            }]
        );
    }

    /// §mesh-14.6: a reply from outside the responder set is row 14 and
    /// leaves the request answerable by a declared responder.
    #[test]
    fn a_reply_from_an_undeclared_peer_is_refused_and_the_request_stays() {
        let mut ecu = requester(None);
        started(&mut ecu, None);
        assert_eq!(
            ecu.receive("mallory", reply(2, Some(RpcStatus::Ok), None, "1"), 20)
                .unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("mallory".to_string()),
                signal: Signal::RpcReplyFromUndeclaredPeer {
                    // The envelope's word, reported as written; what refused
                    // it was the binding it came in on.
                    source: "hmi".to_string(),
                    invoke_id: "09090909090909090909090909090909".to_string(),
                },
            }]
        );
        assert!(matches!(
            ecu.receive("hmi", reply(3, Some(RpcStatus::Ok), None, "2"), 21)
                .unwrap()
                .as_slice(),
            [Effect::Complete { data, .. }] if data == "2"
        ));
    }

    /// §mesh-9.5 `<cancel>`: nothing on the wire, and a later answer is
    /// dropped rather than ending an invocation that is gone.
    #[test]
    fn a_cancelled_request_drops_its_answer() {
        let mut ecu = requester(Some(100));
        started(&mut ecu, None);
        ecu.cancel_invoke("ask", 7);
        assert!(ecu
            .receive("hmi", reply(2, Some(RpcStatus::Ok), None, "1"), 20)
            .unwrap()
            .is_empty());
        assert!(
            ecu.tick(1000).unwrap().is_empty(),
            "a cancelled request has no deadline"
        );
    }

    #[test]
    fn a_deadline_fails_the_invocation_as_deadline_exceeded_with_no_source() {
        let mut ecu = requester(Some(100));
        started(&mut ecu, None);
        // Kept against the monotonic clock the request started at (10).
        assert!(ecu.tick(109).unwrap().is_empty());
        assert_eq!(
            ecu.tick(110).unwrap(),
            alloc::vec![Effect::Fail {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"errorName":"invoke","reason":"deadlineExceeded","invoke_id":"09090909090909090909090909090909"}"#.to_string(),
                source: None,
            }]
        );
        assert!(ecu
            .receive("hmi", reply(2, Some(RpcStatus::Ok), None, "1"), 120)
            .unwrap()
            .is_empty());
    }

    /// An `Ok` the engine cannot be handed is row 4 and ends nothing: the
    /// request still waits for an answer it can use.
    #[test]
    fn an_unreadable_ok_reply_leaves_the_request_waiting() {
        let mut ecu = requester(None);
        started(&mut ecu, None);
        let raw = Envelope {
            id: &[2; 16],
            source: "hmi",
            event_type: "service.response.force",
            pattern: PatternKind::RpcReply,
            datacontenttype: PayloadCodec::Raw,
            data: &[0xFF],
            invoke_id: Some(&WIRE),
            ..Envelope::new()
        }
        .encode_to_vec()
        .unwrap();
        assert!(matches!(
            ecu.receive("hmi", raw, 20).unwrap().as_slice(),
            [Effect::Raise {
                signal: Signal::EnvelopeCorrupt { codec: "raw", .. },
                ..
            }]
        ));
        assert!(matches!(
            ecu.receive("hmi", reply(3, Some(RpcStatus::Ok), None, "1"), 21)
                .unwrap()
                .as_slice(),
            [Effect::Complete { .. }]
        ));
    }

    /// §mesh-9.5's pre-envelope tier: a target that cannot reach the wire
    /// refuses the invocation, which the engine raises as `error.execution`.
    #[test]
    fn a_request_that_cannot_reach_the_wire_is_refused() {
        let mut ecu = requester(None);
        assert_eq!(
            ecu.invoke(&ask("#nobody", None), WIRE, [1; 16], 0, 0),
            Ok(Invoked::Refused(
                r##"{"errorName":"execution","reason":"INVOKE_SRC_NOT_FOUND","detail":"no binding for '#nobody'"}"##
                    .to_string()
            ))
        );
        assert!(matches!(
            ecu.invoke(&ask("hmi", None), WIRE, [1; 16], 0, 0),
            Ok(Invoked::Refused(data)) if data.contains("names no Mesh peer")
        ));
    }

    // ── §mesh-10.7: the responder's half — the request in, its reply out ──

    const WIRE_HEX: &str = "09090909090909090909090909090909";

    /// `hmi`, answering `ecu`: `service.response.force` is a reply to it.
    fn responder() -> Router {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer(
            "ecu",
            PeerConfig {
                buffer: None,
                responders: &["ecu"],
                reply_events: &["service.response.force"],
                ..UNORDERED
            },
        );
        hmi
    }

    fn answer(event: &str, invoke_id: &str) -> HostSendRequest {
        HostSendRequest {
            invoke_id: invoke_id.to_string(),
            ..request("#ecu", event, r#"{"force":12}"#)
        }
    }

    fn sent_to_ecu(effects: Result<Vec<Effect>, RouterError>) -> Vec<u8> {
        match effects.unwrap().as_slice() {
            [Effect::Transmit { peer, bytes }] if peer == "ecu" => bytes.clone(),
            other => panic!("expected one transmission to ecu, got {other:?}"),
        }
    }

    /// The request arrives as the event it names, with its wire invokeid as
    /// `_event.invokeid` — the field the reply carries back.
    #[test]
    fn a_request_is_delivered_with_its_wire_invokeid() {
        let bytes = started(&mut requester(None), None);
        assert_eq!(
            responder().receive("ecu", bytes, 0).unwrap(),
            alloc::vec![Effect::Deliver {
                event: "service.request.force".to_string(),
                data: r#"{"n":3}"#.to_string(),
                source: "ecu".to_string(),
                send_id: None,
                invoke_id: Some(WIRE_HEX.to_string()),
            }]
        );
    }

    /// The whole round trip: the reply the document sends while handling the
    /// request goes out as `RpcReply` carrying that invokeid, and completes the
    /// requester's invocation.
    #[test]
    fn a_reply_sent_while_handling_the_request_answers_it() {
        let mut ecu = requester(None);
        let mut hmi = responder();
        hmi.receive("ecu", started(&mut ecu, None), 0).unwrap();

        let bytes = sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_HEX), [5; 16], 0));
        let reply = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(reply.pattern, PatternKind::RpcReply);
        assert_eq!(reply.invoke_id, Some(&WIRE[..]));
        assert_eq!(reply.rpc_status, Some(RpcStatus::Ok));

        assert_eq!(
            ecu.receive("hmi", bytes, 20).unwrap(),
            alloc::vec![Effect::Complete {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"force":12}"#.to_string(),
                source: "hmi".to_string(),
            }]
        );
    }

    /// Only the events the deployment names as replies are stamped: anything
    /// else the document sends while handling the request stays fire-and-forget,
    /// so a notification cannot retire the requester's invocation.
    #[test]
    fn a_send_that_is_not_a_reply_is_not_stamped_as_one() {
        let bytes = sent_to_ecu(responder().send(&answer("status.changed", WIRE_HEX), [5; 16], 0));
        let envelope = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.pattern, PatternKind::FireForget);
        assert_eq!(envelope.invoke_id, None);
        assert_eq!(envelope.rpc_status, None);
    }

    /// A reply sent with no request current has no invokeid to carry; the
    /// requester then delivers it as the event it names.
    #[test]
    fn a_reply_with_no_request_current_carries_no_invokeid() {
        let bytes =
            sent_to_ecu(responder().send(&answer("service.response.force", ""), [5; 16], 0));
        let envelope = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(envelope.pattern, PatternKind::RpcReply);
        assert_eq!(envelope.invoke_id, None);
    }

    #[test]
    fn unhex_reads_back_what_hex_wrote_and_nothing_else() {
        assert_eq!(rpc::unhex(&rpc::hex(&WIRE)), Some(WIRE));
        assert_eq!(rpc::unhex(""), None);
        assert_eq!(rpc::unhex("ask"), None);
        assert_eq!(rpc::unhex(&"zz".repeat(16)), None);
    }

    #[test]
    fn a_request_without_its_event_param_is_the_hosts_error() {
        let mut ecu = requester(None);
        let mut request = ask("#hmi", None);
        request.params.remove(MESH_EVENT_PARAM);
        assert_eq!(
            ecu.invoke(&request, WIRE, [1; 16], 0, 0),
            Err(RouterError::MissingParam(MESH_EVENT_PARAM))
        );
    }
}
