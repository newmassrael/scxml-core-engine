// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//!
//! A router whose deployment checkpoints it ([`Router::enable_checkpoints`],
//! SCE_MESH.md section 10.11) holds every envelope it produces until the host
//! reports durable a checkpoint that holds it ([`Router::durable`]), and a
//! binding with durable receipt ([`Router::send_durably`],
//! [`Router::receive_durably`]) keeps what it sent until its receiver
//! acknowledges it, and acknowledges what it received once a durable checkpoint
//! holds it.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use sce_forge_runtime::algorithm::AlgorithmError;
use sce_forge_runtime::codec::{Bounded, CodecError, Entries, SceCursor};
use sce_rust_runtime::{HostInvokeRequest, HostSendRequest};

use crate::causal::{
    envelope_tag, stamp, CausalGroup, CausalLayer, Dot, OfferError, Refusal, Rejected, Replica,
    Start, Verdict,
};
use crate::checkpoint::{
    CheckpointError, Clocks, PeerState, QueuedState, ReceivedState, RouterState, UnackedState,
    Unreleased,
};
use crate::generated::envelope::Envelope;
use crate::generated::pattern_kind::PatternKind;
use crate::generated::payload_codec::PayloadCodec;
use crate::generated::rpc_status::RpcStatus;
use crate::inbound::{sequenced, AdmitError, ConfigError, Delivery, Inbound, Judged, Outcome};

pub use crate::inbound::Standing;
use crate::outbound::{Admitted, Outbound, OutboundBuffer, RetryPolicy};
use crate::peers::{instance_of, instance_peer};
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
    /// order (§mesh-10.6.3). Only the envelopes the receiver orders take
    /// one; a reply goes unnumbered (SCE_MESH.md section 10.6.4).
    pub stamp_sequence: bool,
    /// How envelopes FROM this peer are delivered (§mesh-10.5, §mesh-10.6).
    pub delivery: Delivery,
    /// Whose reply may answer a request sent to this peer.
    pub responders: Responders,
    /// deploy.yaml's binding-level request deadline, which a request to this
    /// peer takes when it carries no `_mesh_deadline_ms`; `None` lets it wait
    /// for its reply indefinitely (§mesh-9.5).
    pub deadline_ms: Option<u64>,
    /// The events this machine sends to the peer as replies
    /// (`service.response.*`, §mesh-8.1). One goes out as `RpcReply` with the
    /// invokeid of the request being handled, which is how the requester
    /// matches it (§mesh-10.7); every other send is `FireForget`.
    pub reply_events: &'static [&'static str],
    /// The causal group this binding is the route of (docs/adr/0085, section
    /// 3.2), or `None` for the per-source rules. A causal peer's changes go
    /// through its causal layer ([`Router::start_causal`]), which then
    /// replaces the duplicate window and the sequence buffer for it.
    pub causal: Option<CausalGroup>,
}

/// Whose reply may answer a request sent to a peer
/// ([`PeerConfig::responders`], §mesh-14.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Responders {
    /// The machines named: the binding's `reply_from:`, or the peer alone.
    /// Never empty in a table the build writes.
    Named(&'static [&'static str]),
    /// The instance the request was sent to, and no other: the peer is an
    /// instance of an open machine (docs/adr/0113, D3), and the list of its
    /// names is one no deployment can write.
    Instance,
}

impl Responders {
    /// Whether a reply that arrived on the link of `peer` may answer a
    /// request sent to `target`. Under [`Self::Instance`] only the link of
    /// the instance it was sent to does: a machine's name is no instance's, and
    /// one instance answers for no other.
    pub(crate) fn admits(self, peer: &str, target: &str) -> bool {
        match self {
            Self::Named(names) => names.contains(&peer),
            Self::Instance => peer == target,
        }
    }
}

impl PeerConfig {
    /// Whether the link of this binding names the machine that sent what arrives on it, so
    /// that an envelope whose `source` names another machine is refused
    /// ([`RouterError::SourceNotLink`], SCE_MESH.md section 10.9, invariant 10).
    ///
    /// Every transport a host core serves joins this machine to one peer per link and tells
    /// the core that peer's name, which the host authenticated: `wss` by the path the client
    /// dialed (section 18.1), `zenoh_inbox` by the last chunk of the key it was put under
    /// (section 20.1), `https` by the path and the authenticator (section 22.1). So does
    /// every binding but a causal route, which is a group and not a link to one machine: a
    /// member's change reaches a copy through any member or relay as its author wrote it
    /// (§mesh-10.12), and its author is the replica its tag names (section 10.9, invariant
    /// 9), not the link it came in on.
    pub fn link_names_sender(&self) -> bool {
        self.causal.is_none()
    }
}

/// One thing the host must do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Hand `bytes` to the transport bound to `peer`.
    Transmit { peer: String, bytes: Vec<u8> },
    /// Raise `event` on the engine's external queue with `data` as
    /// `_event.data`; `source` is the machine that sent it, or the instance
    /// of an open machine that did as its name `<machine>/<instance>`, whose
    /// `mesh://<source>` is the event's `_event.origin` (docs/adr/0113,
    /// D2); `send_id` the
    /// id its `<send>` carried (§mesh-10.7: the envelope's `subject`), and
    /// `invoke_id` the request's wire invokeid as RFC 4122 text when the envelope is an
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
    /// A change the document sent to the causal peer `peer` was accepted and
    /// tagged with `dot` (docs/adr/0085, section 3, "offer and verdict"). The
    /// host records it with its dot in the copy's journal, then calls
    /// [`Router::committed`]; nothing of it leaves before.
    Accepted { peer: String, dot: Dot },
    /// A change the document sent to the causal peer `peer` was refused: it
    /// took no number and no copy takes it in. The host tells the copy, which
    /// tells `noRoomForHistory`.
    Refused { peer: String, refusal: Refusal },
    /// The causal peer `peer` is owed this copy's `causalVector`, its link
    /// having come back or its period having passed. The host draws an
    /// envelope id and calls [`Router::announce`].
    AnnounceDue { peer: String },
    /// The peer `peer`, which sends to this machine with durable receipt, is
    /// owed this machine's acknowledgement (SCE_MESH.md section 10.11): a
    /// checkpoint that holds more of its stream became durable, or it sent
    /// again what one already holds. The host draws an envelope id and calls
    /// [`Router::acknowledge`], and sends what it answers once: a lost
    /// acknowledgement is repaired by the next.
    AcknowledgeDue { peer: String },
}

/// How a binding with durable receipt sends (SCE_MESH.md section 10.11): the
/// deployment's `receipt:` block of the sending machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurableSend {
    /// `window`: the envelopes held unacknowledged for the peer, at least 1. A
    /// send while it is full is refused as `backpressureDrop`.
    pub window: usize,
    /// `retry_ms`: how long a sent envelope waits for its acknowledgement
    /// before it is sent again, at least 1.
    pub retry_ms: i64,
}

impl Default for DurableSend {
    /// The deployment's defaults when the block is absent: 256 and 1000.
    fn default() -> Self {
        Self {
            window: 256,
            retry_ms: 1000,
        }
    }
}

/// The room a binding keeps for what is sent to one peer, shared by every router that sends to it:
/// the durable window of an `https` server's outbox (docs/adr/0109, D10; SCE_MESH.md section
/// 10.11), which counts the peer's outbox and what the routers produced for it and no commit
/// appended yet. A router reserves one place for each envelope it writes to the peer; the
/// window's owner gives the place back once the envelope is acknowledged, or once it is dropped
/// unwritten.
pub trait Window: Send + Sync {
    /// Reserve a place for one more envelope, or `Err(depth)`, the envelopes the window holds,
    /// when it holds its most: the send is refused as SCE_MESH.md section 16.7 row 9, and nothing
    /// accepted is dropped.
    fn reserve(&self) -> Result<(), u32>;
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
    /// The causal layer of a causal peer, once the host started it.
    causal: Option<Causal>,
    /// What this machine holds for the peer when it sends to it with durable
    /// receipt ([`Router::send_durably`]).
    durable_out: Option<DurableOut>,
    /// What it keeps of the peer's stream when the peer sends to it with
    /// durable receipt ([`Router::receive_durably`]).
    durable_in: Option<DurableIn>,
    /// The window every envelope to the peer takes a place in, when its binding keeps one
    /// ([`Router::bound_window`]).
    window: Option<Arc<dyn Window>>,
}

/// A sender's side of durable receipt for one peer.
struct DurableOut {
    send: DurableSend,
    /// Produced for the peer and held for the checkpoint that holds them.
    waiting: usize,
    /// Released and not yet acknowledged, by sequence number.
    unacked: BTreeMap<u64, Unacked>,
}

impl DurableOut {
    /// What counts against the window: produced and not yet acknowledged.
    fn held(&self) -> usize {
        self.waiting + self.unacked.len()
    }
}

/// An envelope sent with durable receipt, kept until its acknowledgement.
struct Unacked {
    bytes: Vec<u8>,
    /// When it was last sent, on the host's monotonic clock; `None` while it
    /// has not been, its link not being ready.
    sent_ms: Option<i64>,
}

/// A receiver's side of durable receipt for one peer's stream.
#[derive(Default)]
struct DurableIn {
    /// Every envelope numbered at most this was admitted.
    held: u64,
    /// Admitted past a gap above `held`.
    above: BTreeSet<u64>,
    /// `held` as of the last durable checkpoint: what an acknowledgement
    /// reports.
    durable: u64,
}

impl DurableIn {
    /// Whether the envelope numbered `sequence` was admitted already.
    fn holds(&self, sequence: u64) -> bool {
        sequence <= self.held || self.above.contains(&sequence)
    }

    /// The envelope numbered `sequence` was admitted.
    fn admitted(&mut self, sequence: u64) {
        if self.holds(sequence) {
            return;
        }
        self.above.insert(sequence);
        while let Some(next) = self
            .held
            .checked_add(1)
            .filter(|next| self.above.remove(next))
        {
            self.held = next;
        }
    }
}

/// What a checkpointing router holds back (SCE_MESH.md section 10.11).
#[derive(Default)]
struct Persist {
    /// Produced since the last seal, in order.
    open: Vec<Unreleased>,
    /// Sealed into a checkpoint not yet durable, by its number.
    sealed: VecDeque<(u64, Vec<Unreleased>)>,
    /// Each durable receiver's `held` when each checkpoint not yet durable
    /// was sealed, by its number.
    marks: VecDeque<(u64, Vec<(String, u64)>)>,
    /// The last checkpoint sealed, and the last reported durable.
    sealed_through: u64,
    durable_through: u64,
    /// The incarnations the run the last checkpoint sealed keeps was made under, each with its
    /// instance's link (docs/adr/0113, D5).
    under: Vec<(String, u64)>,
}

impl Persist {
    /// Whether `envelope`, written for `peer`, was produced and is not yet
    /// released: open, or sealed into a checkpoint not yet durable.
    fn holds(&self, peer: &str, envelope: &[u8]) -> bool {
        self.open
            .iter()
            .chain(self.sealed.iter().flat_map(|(_, batch)| batch.iter()))
            .any(|unreleased| unreleased.peer == peer && unreleased.envelope.as_slice() == envelope)
    }
}

/// A causal peer's layer and when it last announced its vector.
struct Causal {
    layer: CausalLayer<Vec<u8>>,
    announce_every_ms: i64,
    announced_ms: Option<i64>,
}

/// An open machine whose instances a router binds as they arrive
/// ([`Router::open_machine`], docs/adr/0113, D3 and D6).
#[derive(Clone, Copy)]
struct OpenMachine {
    /// The one configuration every instance of it is bound under.
    config: PeerConfig,
    /// The most of its instances bound at once.
    max_instances: usize,
    /// Whether its instances send to this machine with durable receipt: each is then judged by
    /// its completion records (docs/adr/0113, D5; [`Router::receive_member`]).
    durable: bool,
}

/// One machine's Mesh endpoint.
pub struct Router {
    machine: String,
    inbound: Inbound,
    peers: BTreeMap<String, Peer>,
    /// The open machines this router binds the instances of, by name.
    open: BTreeMap<String, OpenMachine>,
    requests: Correlation,
    /// `Some` when the router checkpoints ([`Router::enable_checkpoints`]).
    persist: Option<Persist>,
}

impl Router {
    /// A router for `machine`, with the receive side's dedup window and gap
    /// timeout (deploy.yaml `window_size`, `gap_timeout_ms`).
    pub fn new(machine: &str, dedup_window: u32, gap_timeout_ms: i64) -> Result<Self, ConfigError> {
        Ok(Self {
            machine: machine.to_string(),
            inbound: Inbound::new(dedup_window, gap_timeout_ms)?,
            peers: BTreeMap::new(),
            open: BTreeMap::new(),
            requests: Correlation::default(),
            persist: None,
        })
    }

    /// Bind the instances of the open machine `machine` (deploy.yaml
    /// `instances: open`, docs/adr/0113): no peer is bound by its name, and an
    /// instance is bound under `config`, as `<machine>/<instance>`
    /// ([`instance_peer`]), the first time it is named: an envelope that
    /// arrives on its link, a send to its origin, its readiness, a checkpoint
    /// that holds it (D3). At most `max_instances` are bound at once, the
    /// deployment's `max_instances`: a new one past it is refused
    /// ([`RouterError::InstancesFull`]) and none that is bound is put out for
    /// it (D6); the host unbinds an instance whose membership ended
    /// ([`Self::remove_peer`]).
    ///
    /// Refused for a bound of none, and for a causal binding, whose sender is
    /// the replica its tag names and not the link (SCE_MESH.md section 10.9,
    /// invariant 9; D7).
    pub fn open_machine(
        &mut self,
        machine: &str,
        config: PeerConfig,
        max_instances: usize,
    ) -> Result<(), RouterError> {
        let refuse = |reason| {
            Err(RouterError::BadOpenMachine {
                machine: machine.to_string(),
                reason,
            })
        };
        if max_instances == 0 {
            return refuse("its bound on instances is none");
        }
        if !config.link_names_sender() {
            return refuse("its binding is a causal route, whose sender is not its link");
        }
        if !crate::link::is_path_segment(machine) {
            return refuse("its name is not a segment of the path its instances post to");
        }
        self.open.insert(
            machine.to_string(),
            OpenMachine {
                config,
                max_instances,
                durable: false,
            },
        );
        Ok(())
    }

    /// The open machine `peer` names an instance of, when it names one.
    fn open_of(&self, peer: &str) -> Option<&OpenMachine> {
        let (machine, _) = instance_of(peer)?;
        self.open.get(machine)
    }

    /// What the router holds of `peer`: the configuration it is bound under,
    /// or, for an instance of an open machine not yet bound, its machine's.
    fn config_of(&self, peer: &str) -> Option<PeerConfig> {
        self.peers
            .get(peer)
            .map(|bound| bound.config)
            .or_else(|| self.open_of(peer).map(|open| open.config))
    }

    /// The one lookup that binds (docs/adr/0113, D3): a peer bound by its
    /// name; else an instance of an open machine, bound now under its
    /// machine's configuration while fewer than its bound are; else unknown.
    /// Receiving, sending, readiness and a restore all ask it, so a name is
    /// bound by one rule.
    fn bind(&mut self, peer: &str) -> Result<(), RouterError> {
        if self.peers.contains_key(peer) {
            return Ok(());
        }
        let Some(open) = self.open_of(peer).copied() else {
            return Err(RouterError::UnknownPeer(peer.to_string()));
        };
        let machine = instance_of(peer).map_or(peer, |(machine, _)| machine);
        if self.instances_of(machine) >= open.max_instances {
            return Err(RouterError::InstancesFull {
                machine: machine.to_string(),
                max_instances: open.max_instances,
            });
        }
        self.add_peer(peer, open.config);
        Ok(())
    }

    /// How many instances of the open machine `machine` are bound.
    fn instances_of(&self, machine: &str) -> usize {
        self.peers
            .keys()
            .filter(|name| instance_of(name).is_some_and(|(of, _)| of == machine))
            .count()
    }

    /// Bind `peer` — the name a document writes as `#peer`. A binding that
    /// replaces one has none of its durable receipt: the host declares it
    /// again ([`Self::send_durably`], [`Self::receive_durably`]).
    pub fn add_peer(&mut self, peer: &str, config: PeerConfig) {
        self.peers.insert(
            peer.to_string(),
            Peer {
                outbound: Outbound::new(config.buffer),
                config,
                next_sequence: 1,
                causal: None,
                durable_out: None,
                durable_in: None,
                window: None,
            },
        );
    }

    /// Every envelope written to `peer` takes a place in `window` first, and one it has no room
    /// for is refused as `backpressureDrop` (SCE_MESH.md section 16.7 row 9) and never written:
    /// the durable window of an `https` server's outbox (docs/adr/0109, D10), which the sessions
    /// of a pool share. A send that is never accepted is told, and nothing accepted is dropped.
    pub fn bound_window(&mut self, peer: &str, window: Arc<dyn Window>) -> Result<(), RouterError> {
        self.bind(peer)?;
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        bound.window = Some(window);
        Ok(())
    }

    /// The binding to `peer` acknowledges by the exchange that carried an envelope, and one
    /// carried `envelopes`: the `https` binding's `200`, which its server answers only once a
    /// durable checkpoint holds what the exchange carried (docs/adr/0109, D10; SCE_MESH.md
    /// section 10.11). Each of them this router holds until acknowledged, under durable receipt
    /// ([`Self::send_durably`]), is acknowledged by its sequence number and held no more; so no
    /// `durableReceipt` envelope is sent on such a binding. Anything else is passed over.
    pub fn carried(&mut self, peer: &str, envelopes: &[Vec<u8>]) {
        let Some(out) = self
            .peers
            .get_mut(peer)
            .and_then(|bound| bound.durable_out.as_mut())
        else {
            return;
        };
        for bytes in envelopes {
            let Ok(envelope) = Envelope::decode(&mut SceCursor::new(bytes)) else {
                continue;
            };
            if let Some(sequence) = envelope.sequence_no {
                out.unacked.remove(&sequence);
            }
        }
    }

    /// How many envelopes the machine produced for `peer` that no durable checkpoint released
    /// yet (SCE_MESH.md section 10.11): those a host drops with the router, losing them.
    pub fn held_for(&self, peer: &str) -> usize {
        self.persist.as_ref().map_or(0, |persist| {
            persist
                .open
                .iter()
                .chain(persist.sealed.iter().flat_map(|(_, batch)| batch.iter()))
                .filter(|unreleased| unreleased.peer == peer)
                .count()
        })
    }

    /// Checkpoint this machine: its deployment says `persistence: checkpoint`
    /// (SCE_MESH.md section 10.11). From now on every envelope the machine
    /// produces, a reply included, is held until the host reports durable a
    /// checkpoint that holds it ([`Self::durable`]), so a peer never holds an
    /// envelope a restored machine does not know it sent. Envelopes the router
    /// sends again (a resend, a retry, a reply a duplicate request is answered
    /// with, a causal repair) left once already and are not held again.
    pub fn enable_checkpoints(&mut self) {
        if self.persist.is_none() {
            self.persist = Some(Persist::default());
        }
    }

    /// Whether the router checkpoints.
    pub fn is_checkpointed(&self) -> bool {
        self.persist.is_some()
    }

    /// The binding to `peer` sends with durable receipt (SCE_MESH.md section
    /// 10.11): every envelope to it is stamped with a sequence number, kept
    /// once released until `peer` acknowledges a durable checkpoint holds it,
    /// and sent again after `send.retry_ms` without one and at once on each
    /// readiness edge of the link. A failed send of one is neither retried by
    /// the binding's retry policy nor given up: it is held. Needs the router to
    /// checkpoint, since a sender that forgets what it holds when its process
    /// dies keeps no promise; refused for a causal peer, whose route repairs
    /// what it loses by its own vectors.
    pub fn send_durably(&mut self, peer: &str, send: DurableSend) -> Result<(), RouterError> {
        if send.window == 0 || send.retry_ms < 1 {
            return Err(RouterError::BadReceipt {
                peer: peer.to_string(),
                window: send.window,
                retry_ms: send.retry_ms,
            });
        }
        self.durable_peer(peer)?.durable_out = Some(DurableOut {
            send,
            waiting: 0,
            unacked: BTreeMap::new(),
        });
        Ok(())
    }

    /// The binding from `peer` receives with durable receipt (SCE_MESH.md
    /// section 10.11): every envelope from it carries a sequence number, a
    /// repeat of one admitted is dropped by that number and answered with this
    /// machine's acknowledgement, a gap is never timed out (the envelope is
    /// still held by its sender), and each checkpoint that becomes durable
    /// acknowledges what it holds. Needs the router to checkpoint; refused for
    /// a causal peer.
    ///
    /// For an open machine (docs/adr/0113, D5), each of its instances: what
    /// one instance sends goes to several sessions, so no receiver sees its
    /// numbers one after another, and each envelope is judged by the
    /// completion records of its instance's incarnation instead
    /// ([`Self::receive_member`]).
    pub fn receive_durably(&mut self, peer: &str) -> Result<(), RouterError> {
        if let Some(open) = self.open.get_mut(peer) {
            if self.persist.is_none() {
                return Err(RouterError::NotCheckpointed);
            }
            open.durable = true;
            return Ok(());
        }
        self.durable_peer(peer)?.durable_in = Some(DurableIn::default());
        Ok(())
    }

    /// The bound peer `peer`, which may take durable receipt.
    fn durable_peer(&mut self, peer: &str) -> Result<&mut Peer, RouterError> {
        if self.persist.is_none() {
            return Err(RouterError::NotCheckpointed);
        }
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        if bound.config.causal.is_some() {
            return Err(RouterError::DurableOnCausal(peer.to_string()));
        }
        Ok(bound)
    }

    /// Whether what this machine sends to `peer` is held until acknowledged,
    /// so a failed send of it is sent again by the router and not given up.
    pub fn holds_until_acknowledged(&self, peer: &str) -> bool {
        self.peers
            .get(peer)
            .is_some_and(|bound| bound.durable_out.is_some())
    }

    /// The last checkpoint the router was sealed with, `0` before the first.
    /// What a binding that answers only after a durable checkpoint (the `https`
    /// binding's `receipt: durable`) waits past: an envelope admitted now is
    /// held by the next checkpoint sealed, and by none before.
    pub fn sealed_through(&self) -> u64 {
        self.persist
            .as_ref()
            .map_or(0, |persist| persist.sealed_through)
    }

    /// The incarnations the run the last checkpoint sealed keeps was made
    /// under, each with the link of its instance (docs/adr/0113, D5): every
    /// one an envelope taken in since the seal before was judged under
    /// ([`Self::receive_member`]). The write that keeps the checkpoint names
    /// each, and keeps nothing once the store no longer holds one
    /// (`sce_rust_session_store::Write::under`), so nothing of a run is kept
    /// after its incarnation ended.
    pub fn sealed_under(&self) -> &[(String, u64)] {
        self.persist
            .as_ref()
            .map_or(&[], |persist| persist.under.as_slice())
    }

    /// The last checkpoint the host reported durable, `0` before the first.
    pub fn durable_through(&self) -> u64 {
        self.persist
            .as_ref()
            .map_or(0, |persist| persist.durable_through)
    }

    /// Seal the checkpoint `sequence`: what was produced since the last seal
    /// belongs to it, and so do the replies made and each durable receiver's
    /// mark. Answers the router's part of the checkpoint, every moment written
    /// on the wall clock `clocks` read. `sequence` is above every number the
    /// router was sealed or restored with.
    pub(crate) fn seal(
        &mut self,
        sequence: u64,
        clocks: Clocks,
    ) -> Result<RouterState, RouterError> {
        let marks: Vec<(String, u64)> = self
            .peers
            .iter()
            .filter_map(|(name, bound)| {
                bound
                    .durable_in
                    .as_ref()
                    .map(|input| (name.clone(), input.held))
            })
            .collect();
        let persist = self.persist.as_mut().ok_or(RouterError::NotCheckpointed)?;
        if sequence <= persist.sealed_through {
            return Err(RouterError::CheckpointOutOfOrder {
                sealed: persist.sealed_through,
                given: sequence,
            });
        }
        let open = core::mem::take(&mut persist.open);
        persist.sealed.push_back((sequence, open));
        persist.marks.push_back((sequence, marks));
        persist.sealed_through = sequence;
        // Every envelope a checkpoint not yet durable holds is this one's too:
        // a restore from it releases them all.
        let unreleased = persist
            .sealed
            .iter()
            .flat_map(|(_, batch)| batch.iter().cloned())
            .collect();
        let under = self.inbound.seal(sequence);
        if let Some(persist) = self.persist.as_mut() {
            persist.under = under;
        }
        Ok(RouterState {
            peers: self
                .peers
                .iter()
                .map(|(name, bound)| peer_state(name, bound, clocks))
                .collect(),
            senders: self.inbound.states(clocks),
            unreleased,
        })
    }

    /// The host wrote the checkpoint `sequence` durably: release what it and
    /// every checkpoint before it hold, at `now_ms`. The envelopes go to their
    /// peers in the order they were produced, the replies they hold now answer
    /// a duplicate request, and each peer whose durable stream a released
    /// checkpoint holds more of is owed an acknowledgement
    /// ([`Effect::AcknowledgeDue`]).
    pub fn durable(&mut self, sequence: u64, now_ms: i64) -> Result<Vec<Effect>, RouterError> {
        let persist = self.persist.as_mut().ok_or(RouterError::NotCheckpointed)?;
        if sequence > persist.sealed_through {
            return Err(RouterError::NotSealed {
                sequence,
                sealed: persist.sealed_through,
            });
        }
        let covered = persist
            .sealed
            .iter()
            .take_while(|(at, _)| *at <= sequence)
            .count();
        let released: Vec<Unreleased> = persist
            .sealed
            .drain(..covered)
            .flat_map(|(_, batch)| batch)
            .collect();
        // The latest mark a released checkpoint holds is the one to report.
        let marked = persist
            .marks
            .iter()
            .take_while(|(at, _)| *at <= sequence)
            .count();
        let marks = persist
            .marks
            .drain(..marked)
            .next_back()
            .map(|(_, marks)| marks)
            .unwrap_or_default();
        persist.durable_through = persist.durable_through.max(sequence);
        self.inbound.released_through(sequence);

        let mut effects = Vec::new();
        for unreleased in released {
            // A peer unbound since took what was produced for it.
            let Some(bound) = self.peers.get_mut(&unreleased.peer) else {
                continue;
            };
            if let Some(out) = bound.durable_out.as_mut() {
                out.waiting = out.waiting.saturating_sub(1);
            }
            effects.extend(release(
                &unreleased.peer,
                bound,
                unreleased.envelope,
                unreleased.sequence,
                now_ms,
            ));
        }
        for (name, mark) in marks {
            let Some(input) = self
                .peers
                .get_mut(&name)
                .and_then(|bound| bound.durable_in.as_mut())
            else {
                continue;
            };
            if mark > input.durable {
                input.durable = mark;
                effects.push(Effect::AcknowledgeDue { peer: name });
            }
        }
        Ok(effects)
    }

    /// Take back the router's part of the checkpoint `sequence`, which the
    /// host read as the last durable one, at `clocks`. What it holds unreleased
    /// is released, since it is durable; a binding with durable receipt keeps
    /// what it holds unacknowledged and sends it again when its link is ready.
    /// Unless the checkpoint is `clean`, each source this machine holds
    /// receive state for, and whose binding does not receive durably, is told
    /// that what it sent after the checkpoint may be lost (`restored`,
    /// SCE_MESH.md section 16.7, row 17).
    ///
    /// The router is the one the host built from its deployment, checkpointing
    /// and with its durable receipt declared, before anything was sent or
    /// received through it. A refused restore leaves it in no state to use.
    pub(crate) fn restore(
        &mut self,
        state: &RouterState,
        sequence: u64,
        clean: bool,
        clocks: Clocks,
    ) -> Result<Vec<Effect>, CheckpointError> {
        let persist = self
            .persist
            .as_mut()
            .ok_or(CheckpointError::Router(RouterError::NotCheckpointed))?;
        *persist = Persist {
            sealed_through: sequence,
            durable_through: sequence,
            ..Persist::default()
        };
        for saved in &state.peers {
            // docs/adr/0113, D3: an instance the checkpoint holds is bound by
            // the one lookup, under its open machine's configuration.
            self.bind(&saved.peer).map_err(|_| {
                CheckpointError::Unbound(format!(
                    "it holds state for the peer '{}', which this router does not bind",
                    saved.peer
                ))
            })?;
            let bound = self.peers.get_mut(&saved.peer).ok_or_else(|| {
                CheckpointError::Unbound(format!(
                    "it holds state for the peer '{}', which this router does not bind",
                    saved.peer
                ))
            })?;
            bound.next_sequence = saved.next_sequence;
            for queued in &saved.queued {
                bound
                    .outbound
                    .requeue(queued.envelope.clone(), clocks.to_monotonic(queued.since));
            }
            match bound.durable_out.as_mut() {
                Some(out) => {
                    out.waiting = 0;
                    out.unacked = saved
                        .unacked
                        .iter()
                        .map(|held| {
                            (
                                held.sequence,
                                Unacked {
                                    bytes: held.envelope.clone(),
                                    sent_ms: None,
                                },
                            )
                        })
                        .collect();
                }
                None if saved.unacked.is_empty() => {}
                None => {
                    return Err(CheckpointError::Unbound(format!(
                        "it holds envelopes for '{}' until they are acknowledged, and this \
                         router does not send to it durably",
                        saved.peer
                    )))
                }
            }
            // A receiver that is no longer durable has nothing to keep of the
            // marks: they promise nothing it holds.
            if let (Some(input), Some(received)) = (bound.durable_in.as_mut(), &saved.received) {
                input.held = received.held;
                input.durable = received.held;
                input.above = received.above.iter().copied().collect();
            }
        }
        self.inbound.restore(&state.senders, clocks)?;

        let mut effects = Vec::new();
        if !clean {
            for saved in &state.senders {
                // docs/adr/0113, D5: an instance judged by its completion
                // records holds what it sent until it has its answer.
                let durable = self
                    .peers
                    .get(&saved.source)
                    .is_some_and(|bound| bound.durable_in.is_some())
                    || self.open_of(&saved.source).is_some_and(|open| open.durable);
                if !durable {
                    effects.push(Effect::Raise {
                        peer: None,
                        signal: Signal::Restored {
                            source: saved.source.clone(),
                        },
                    });
                }
            }
        }
        for unreleased in &state.unreleased {
            self.bind(&unreleased.peer).map_err(|_| {
                CheckpointError::Unbound(format!(
                    "it holds an envelope for the peer '{}', which this router does not bind",
                    unreleased.peer
                ))
            })?;
            let bound = self.peers.get_mut(&unreleased.peer).ok_or_else(|| {
                CheckpointError::Unbound(format!(
                    "it holds an envelope for the peer '{}', which this router does not bind",
                    unreleased.peer
                ))
            })?;
            effects.extend(release(
                &unreleased.peer,
                bound,
                unreleased.envelope.clone(),
                unreleased.sequence,
                clocks.monotonic_ms,
            ));
        }
        Ok(effects)
    }

    /// The acknowledgement this machine owes `peer`, which sends to it with
    /// durable receipt (SCE_MESH.md section 10.11): pattern `durableReceipt`,
    /// `id` the envelope id the host drew, and `sequence_no` the mark of the
    /// last durable checkpoint, every envelope of `peer`'s stream numbered at
    /// most it being held by one. Cumulative and idempotent, so it needs no
    /// acknowledgement, sequence number or duplicate suppression of its own.
    pub fn acknowledge(&mut self, peer: &str, id: [u8; 16]) -> Result<Vec<u8>, RouterError> {
        let machine = within(self.machine.as_str())?;
        let mark = self
            .peers
            .get(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?
            .durable_in
            .as_ref()
            .ok_or_else(|| RouterError::NotDurable(peer.to_string()))?
            .durable;
        Ok(Envelope {
            id: (&id).into(),
            source: machine,
            event_type: within("")?,
            pattern: PatternKind::DurableReceipt,
            datacontenttype: PayloadCodec::None,
            sequence_no: Some(mark),
            ..Envelope::new()
        }
        .encode_to_vec())
    }

    /// When the next envelope held for an acknowledgement is due to be sent
    /// again, on the host's monotonic clock: with the endpoint's retries, the
    /// time a host that owns time sleeps until before its next
    /// [`tick`](Self::tick). Only a peer whose link is ready counts; one that
    /// is not is sent to at its readiness edge.
    pub fn next_resend_due_ms(&self) -> Option<i64> {
        self.peers
            .values()
            .filter(|bound| bound.outbound.is_ready())
            .filter_map(|bound| bound.durable_out.as_ref())
            .flat_map(|out| {
                out.unacked.values().map(move |held| {
                    held.sent_ms
                        .map_or(i64::MIN, |at| at.saturating_add(out.send.retry_ms))
                })
            })
            .min()
    }

    /// Start the causal layer of the causal peer `peer` (docs/adr/0085,
    /// sections 3.2 and 3.3): `start` is the incarnation the host read from
    /// the copy's journal, or a new one; `room` the most changes the layer
    /// may hold not yet stable; `announce_every_ms` the period of its
    /// `causalVector`. A send to a causal peer before this is refused.
    pub fn start_causal(
        &mut self,
        peer: &str,
        start: Start,
        room: usize,
        announce_every_ms: i64,
    ) -> Result<(), RouterError> {
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        let group = bound
            .config
            .causal
            .ok_or_else(|| RouterError::NotCausal(peer.to_string()))?;
        // §mesh-10.12, what a host does: the peer's layer begins from the
        // incarnation the host read from the copy's journal, or a new one.
        bound.causal = Some(Causal {
            layer: CausalLayer::new(group, start, room),
            announce_every_ms,
            announced_ms: None,
        });
        Ok(())
    }

    /// The causal layer of `peer`, for the host to read (its replica, its
    /// delivered vector, what it holds).
    pub fn causal_layer(&self, peer: &str) -> Option<&CausalLayer<Vec<u8>>> {
        self.peers
            .get(peer)
            .and_then(|bound| bound.causal.as_ref())
            .map(|causal| &causal.layer)
    }

    /// When a causal peer whose layer still has work is next owed this copy's
    /// `causalVector`: the time at which [`Router::tick`] announces it. A layer
    /// has work while its log keeps a change not yet stable or it holds one it
    /// cannot yet deliver; a vector is how either ends, by a repair or by the
    /// mark that moves the stable cut. `None` when no started layer has work,
    /// which lets a host that owns time (a simulation) sleep until something
    /// else happens; a layer never announced is owed its vector at once.
    pub fn next_announce_due_ms(&self) -> Option<i64> {
        self.peers
            .values()
            .filter_map(|bound| {
                let causal = bound.causal.as_ref()?;
                if causal.layer.log_len() == 0 && causal.layer.held_len() == 0 {
                    return None;
                }
                Some(match causal.announced_ms {
                    None => i64::MIN,
                    Some(at) => at.saturating_add(causal.announce_every_ms),
                })
            })
            .min()
    }

    /// The host committed the journal record of `dot`, a change of the causal
    /// peer `peer`: what it releases goes to that peer's send half.
    pub fn committed(
        &mut self,
        peer: &str,
        dot: Dot,
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        let released = causal_of(peer, bound)?.layer.committed(dot);
        let mut effects = Vec::new();
        for bytes in released {
            effects.extend(dispatch(peer, bound, bytes, now_ms));
        }
        Ok(effects)
    }

    /// Send this copy's `causalVector` to the causal peer `peer` (pattern 23:
    /// the epoch and its delivered vector, keys 19 and 20). `id` is the
    /// envelope id the host drew.
    pub fn announce(
        &mut self,
        peer: &str,
        id: [u8; 16],
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        let machine = within(self.machine.as_str())?;
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        let causal = causal_of(peer, bound)?;
        let tag = causal.layer.summary();
        causal.announced_ms = Some(now_ms);
        let envelope = Envelope {
            id: (&id).into(),
            source: machine,
            event_type: within("")?,
            pattern: PatternKind::CausalVector,
            datacontenttype: PayloadCodec::None,
            ..Envelope::new()
        };
        let bytes = stamp(envelope, &tag)
            .map_err(|_| RouterError::Encode(CodecError::CborOutOfRange))?
            .encode_to_vec();
        Ok(dispatch(peer, bound, bytes, now_ms))
    }

    /// Perform a `<send target="#peer">` the engine handed the host. `id` is
    /// the envelope id the host drew (a UUID v7), `now_ms` its monotonic
    /// clock. A router that checkpoints holds the envelope for the next
    /// checkpoint ([`Self::enable_checkpoints`]).
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
        let peer_name = addressed_peer(&request.target)
            .ok_or_else(|| RouterError::NotMeshTarget(request.target.clone()))?;
        // docs/adr/0113, D3: a peer the one lookup cannot bind, unknown or past
        // its open machine's bound, is unreachable.
        let bound = self.bind(peer_name);
        let Some(peer) = self.peers.get_mut(peer_name).filter(|_| bound.is_ok()) else {
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
            rpc::parse_uuid_text(&request.invoke_id)
        } else {
            None
        };
        let envelope = Envelope {
            id: (&id).into(),
            source: within(self.machine.as_str())?,
            event_type: within(request.event_name.as_str())?,
            pattern: if reply {
                PatternKind::RpcReply
            } else {
                PatternKind::FireForget
            },
            datacontenttype: payload_codec(&request.event_data),
            data: within(request.event_data.as_bytes())?,
            // §mesh-10.7: the receiver's `_event.sendid` is the envelope's
            // `subject`, so a `<send>`'s id travels there.
            subject: (!request.send_id.is_empty())
                .then(|| within(request.send_id.as_str()))
                .transpose()?,
            invoke_id: answering.as_ref().map(Into::into),
            rpc_status: reply.then_some(RpcStatus::Ok),
            ..Envelope::new()
        };
        if peer.config.causal.is_some() {
            return offer(peer_name, peer, envelope);
        }
        let (bytes, sequence) = match write_envelope(peer, envelope) {
            Ok(written) => written,
            Err(signal) => {
                return Ok(alloc::vec![Effect::Raise {
                    peer: Some(peer_name.to_string()),
                    signal,
                }])
            }
        };
        // §mesh-10.5: the request's window entry keeps its first reply, which
        // answers a duplicate of the request.
        if let Some(invoke) = answering {
            self.inbound
                .answered(&invoke, &bytes, self.persist.is_some());
        }
        Ok(hand_over(
            self.persist.as_mut(),
            peer_name,
            peer,
            bytes,
            sequence,
            now_ms,
        ))
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
        let Some(peer_name) = addressed_peer(&request.src) else {
            return Ok(Invoked::Refused(rpc::src_not_found_data(&alloc::format!(
                "'{}' names no Mesh peer",
                request.src
            ))));
        };
        let bound = self.bind(peer_name);
        let Some(peer) = self.peers.get_mut(peer_name).filter(|_| bound.is_ok()) else {
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
            id: (&id).into(),
            source: within(self.machine.as_str())?,
            event_type: within(event)?,
            pattern: PatternKind::RpcRequest,
            datacontenttype: payload_codec(&request.event_data),
            data: within(request.event_data.as_bytes())?,
            invoke_id: Some((&wire_id).into()),
            deadline_unix_ms: deadline_ms.map(|ms| now_unix_ms.saturating_add(ms)),
            ..Envelope::new()
        };
        let responders = peer.config.responders;
        let (bytes, sequence) = match write_envelope(peer, envelope) {
            Ok(written) => written,
            // A request a binding with durable receipt has no room for never
            // leaves, and its invocation ends at once, told why.
            Err(signal) => {
                return Ok(Invoked::Started(alloc::vec![
                    Effect::Raise {
                        peer: Some(peer_name.to_string()),
                        signal,
                    },
                    Effect::Fail {
                        invoke_id: request.invoke_id.clone(),
                        token: request.token,
                        data: rpc::invoke_error_data(
                            RpcStatus::Unavailable,
                            Some("backpressureDrop"),
                            None,
                            &wire_id,
                        ),
                        source: None,
                    },
                ]))
            }
        };
        let effects = hand_over(
            self.persist.as_mut(),
            peer_name,
            peer,
            bytes.clone(),
            sequence,
            now_ms,
        );
        self.requests.register(
            wire_id,
            Pending {
                invoke_id: request.invoke_id.clone(),
                token: request.token,
                target: peer_name.to_string(),
                responders,
                expires_ms: deadline_ms
                    .map(|ms| now_ms.saturating_add(i64::try_from(ms).unwrap_or(i64::MAX))),
                request: bytes,
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
        self.bind(peer)?;
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
        // SCE_MESH.md section 10.11: what a binding with durable receipt holds
        // unacknowledged is sent again at once on each readiness edge.
        if let Some(out) = bound.durable_out.as_mut() {
            for held in out.unacked.values_mut() {
                held.sent_ms = Some(now_ms);
                effects.push(Effect::Transmit {
                    peer: peer.to_string(),
                    bytes: held.bytes.clone(),
                });
            }
        }
        // docs/adr/0085, section 3.2: a link that comes back carries this
        // copy's vector at once, so what was lost while it was down is
        // repaired without waiting for the period.
        if bound.causal.is_some() {
            effects.push(Effect::AnnounceDue {
                peer: peer.to_string(),
            });
        }
        Ok(effects)
    }

    /// `peer`'s transport stopped being ready, and the link is lost: what
    /// `peer` held for this machine cannot reach it any more.
    ///
    /// A request waiting on the link can no longer be answered over it: it is
    /// forgotten, as a cancel forgets it, and each is §mesh-16.7 row 5 — the
    /// peer-down path the C++ core takes for the same requests.
    pub fn peer_not_ready(&mut self, peer: &str) -> Result<Vec<Effect>, RouterError> {
        let mut effects = self.peer_interrupted(peer)?;
        effects.extend(
            self.requests
                .forget_sent_to(peer)
                .into_iter()
                .map(|wire_id| Effect::Raise {
                    peer: Some(peer.to_string()),
                    signal: Signal::InvokeChildLost {
                        invoke_id: rpc::uuid_text(&wire_id),
                        target: peer.to_string(),
                    },
                }),
        );
        Ok(effects)
    }

    /// `peer`'s transport stopped being ready while `peer` keeps what it holds
    /// for this machine, which reaches this machine once the link is ready
    /// again: an exchange of the HTTPS binding that failed, its server keeping
    /// the client's mailbox (SCE_MESH.md section 22.4).
    ///
    /// The readiness edge is all it is: what the machine sends waits as it
    /// waits for any peer that is not ready (§mesh-10.10), and the edge is
    /// §mesh-16.7 row 1 once. A request waiting on `peer` stays waiting, since
    /// its reply still reaches this machine; its reply, its deadline or its
    /// cancel ends it.
    pub fn peer_interrupted(&mut self, peer: &str) -> Result<Vec<Effect>, RouterError> {
        self.bind(peer)?;
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

    /// `peer` started again without what it held for this machine: the
    /// mailbox of an HTTPS server now numbers in another incarnation
    /// (SCE_MESH.md section 22.3), so a reply it held is gone, at `now_ms`.
    ///
    /// Each request still waiting on `peer` is sent again, its envelope as it
    /// was written and so under its own ids: the peer's §mesh-10.5 window
    /// answers a copy of a request it took with the reply the first copy
    /// produced, drops one it is still handling, and takes one it never saw as
    /// new, so the request is answered once whichever it was. A copy of one
    /// this machine's transport still carries is dropped the same way. A
    /// request a checkpoint not yet durable still holds has not left, and
    /// leaves when that checkpoint is durable (SCE_MESH.md section 10.11).
    pub fn peer_restarted(&mut self, peer: &str, now_ms: i64) -> Result<Vec<Effect>, RouterError> {
        self.bind(peer)?;
        let persist = self.persist.as_ref();
        let again: Vec<Vec<u8>> = self
            .requests
            .sent_to(peer)
            .filter(|request| !persist.is_some_and(|persist| persist.holds(peer, request)))
            .map(<[u8]>::to_vec)
            .collect();
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        let mut effects = Vec::new();
        for request in again {
            effects.extend(dispatch(peer, bound, request, now_ms));
        }
        Ok(effects)
    }

    /// Whether `peer` is bound.
    pub fn has_peer(&self, peer: &str) -> bool {
        self.peers.contains_key(peer)
    }

    /// Whether a request this router sent still waits for its answer: no
    /// reply, deadline or cancel has ended it yet.
    pub fn awaits_replies(&self) -> bool {
        !self.requests.is_empty()
    }

    /// Whether the request this router sent under the wire `invoke_id`
    /// `wire_id` still waits for its answer.
    pub fn awaits_reply(&self, wire_id: &[u8; 16]) -> bool {
        self.requests.is_waiting(wire_id)
    }

    /// Whether a peer's send half holds an envelope it has not yet handed to
    /// the transport, one buffered until the peer is ready (§mesh-10.10).
    pub fn holds_outbound(&self) -> bool {
        self.peers.values().any(|bound| bound.outbound.depth() > 0)
    }

    /// Whether the router holds an envelope back for durability (SCE_MESH.md
    /// section 10.11): one the machine produced that no durable checkpoint has
    /// released yet, or one sent with durable receipt that its receiver has
    /// not acknowledged. Either is lost, or never sent again, by a host that
    /// drops the endpoint before it is released or acknowledged.
    pub fn holds_for_durability(&self) -> bool {
        let unreleased = self.persist.as_ref().is_some_and(|persist| {
            !persist.open.is_empty() || persist.sealed.iter().any(|(_, batch)| !batch.is_empty())
        });
        unreleased
            || self.peers.values().any(|bound| {
                bound
                    .durable_out
                    .as_ref()
                    .is_some_and(|out| out.waiting > 0 || !out.unacked.is_empty())
            })
    }

    /// Unbind `peer`, which has left for good. What waited for its link is let go as when its link
    /// is lost (each request waiting on it is §mesh-16.7 row 5, and what the outbound buffer held
    /// is dropped), its receive state is forgotten, and a send to it afterwards is to a peer that
    /// is not bound. The effects are what a link lost raises; none when `peer` was not bound.
    pub fn remove_peer(&mut self, peer: &str) -> Vec<Effect> {
        let effects = self.peer_not_ready(peer).unwrap_or_default();
        self.peers.remove(peer);
        self.inbound.forget(peer);
        effects
    }

    /// The retry policy the host applies when a transmission to `peer`
    /// fails (see [`crate::outbound::Attempts`]).
    pub fn retry_policy(&self, peer: &str) -> Option<RetryPolicy> {
        self.config_of(peer).and_then(|config| config.retry)
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
            transport: self.config_of(peer).map(|config| config.transport),
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
    /// So is an envelope that names another sender than the machine `peer`'s
    /// link names ([`RouterError::SourceNotLink`]): what the link delivered is
    /// the host's to judge, and nothing the router keeps by sender sees it.
    pub fn receive(
        &mut self,
        peer: &str,
        bytes: Vec<u8>,
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        self.receive_from(peer, None, bytes, now_ms)
    }

    /// Envelope `bytes` the transport received from `peer`, an instance of
    /// an open machine whose binding receives durably (docs/adr/0113, D5;
    /// [`Self::receive_durably`]), at `now_ms`: its edge took it in under
    /// the incarnation `made_under`, and `standing` is the store's latest
    /// answer for its member, as late as that one at least.
    ///
    /// As [`Self::receive`], but judged by the instance's completion records
    /// in place of a window: one made under an incarnation its member no
    /// longer stands under is fenced ([`RouterError::Fenced`]), one below the
    /// mark is one whose answer the instance has, and a repeat of one taken in
    /// is answered from its record. The judgement is made again here, as the
    /// envelope runs, and the last one is the store's: the write that keeps
    /// the checkpoint names the incarnation ([`Self::sealed_under`]).
    pub fn receive_member(
        &mut self,
        peer: &str,
        made_under: u64,
        standing: Standing,
        bytes: Vec<u8>,
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        self.receive_from(peer, Some((made_under, standing)), bytes, now_ms)
    }

    /// Take `standing`, the store's latest answer for the member whose instance
    /// is `peer` (docs/adr/0113, D5 and D6): the completion records below its
    /// mark are dropped, since the instance has their answers, and all of them
    /// when it stands under a later incarnation. What the router keeps of an
    /// instance is then at most what it has in flight.
    pub fn stand(&mut self, peer: &str, standing: Standing) {
        self.inbound.stand(peer, standing);
    }

    /// Envelope `bytes` from `peer`, with the incarnation it was taken in
    /// under and the standing of its member when its instance is judged by
    /// its completion records.
    fn receive_from(
        &mut self,
        peer: &str,
        judged: Option<(u64, Standing)>,
        bytes: Vec<u8>,
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        self.bind(peer)?;
        let config = self
            .peers
            .get(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?
            .config;
        if config.causal.is_some() {
            return self.receive_causal(peer, bytes, now_ms);
        }
        let delivery = config.delivery;
        let raise = |signal| {
            Ok(alloc::vec![Effect::Raise {
                peer: Some(peer.to_string()),
                signal,
            }])
        };
        let decoded = Envelope::decode(&mut SceCursor::new(&bytes)).ok();
        // SCE_MESH.md section 10.9, invariant 10: the link names the machine
        // at its other end, and `source` is only what the envelope says. One
        // that names another machine is refused before anything kept by
        // sender sees it (the duplicate window and its reply cache, the
        // ordering hold, a durable stream, a request's correlation) and
        // before the machine is told, so no link speaks for a machine it is
        // not.
        //
        // Invariant 11 (docs/adr/0113, D1): the link of an instance of an open
        // machine names the pair, and an envelope on it names both its machine
        // and its instance; one on any other link names no instance.
        if let Some(source) = decoded
            .as_ref()
            .filter(|envelope| config.link_names_sender() && !names_link(envelope, peer))
            .map(sender_of)
        {
            return Err(RouterError::SourceNotLink {
                peer: peer.to_string(),
                source,
            });
        }
        // §mesh-14.6: a reply is checked against its request's responder set
        // on arrival, while the binding it came in on is still known — an
        // ordered binding may hold it and release it on a later tick. One
        // from outside the set is refused before the dedup window or the
        // ordering hold sees it, so it takes no slot in either.
        let head = decoded.map(|envelope| (envelope.pattern, envelope.sequence_no));
        // SCE_MESH.md section 10.11: an acknowledgement releases what this
        // machine held for `peer` up to its mark, and reaches no engine. One
        // without a mark acknowledges nothing.
        if let Some((PatternKind::DurableReceipt, mark)) = head {
            if let (Some(mark), Some(out)) = (
                mark,
                self.peers
                    .get_mut(peer)
                    .and_then(|bound| bound.durable_out.as_mut()),
            ) {
                out.unacked = out.unacked.split_off(&mark.saturating_add(1));
            }
            return Ok(Vec::new());
        }
        if let Some(signal) = self.undeclared_reply(&bytes, peer) {
            return raise(signal);
        }
        // docs/adr/0113, D5: an instance of an open machine whose binding
        // receives durably is judged by its completion records, under the
        // incarnation its edge took the envelope in under and the standing the
        // store holds its member at.
        let by_records = self.open_of(peer).is_some_and(|open| open.durable);
        match (by_records, judged) {
            (true, Some((made_under, standing))) => {
                return match self.inbound.admit_member(peer, bytes, made_under, standing) {
                    Ok(Judged::Admitted(outcome)) => {
                        Ok(self.effects_of(outcome, Some(peer), now_ms))
                    }
                    Ok(Judged::Fenced { current }) => Err(RouterError::Fenced {
                        peer: peer.to_string(),
                        incarnation: made_under,
                        current,
                    }),
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
                };
            }
            (true, None) => return Err(RouterError::Unjudged(peer.to_string())),
            (false, Some(_)) => return Err(RouterError::NotDurable(peer.to_string())),
            (false, None) => {}
        }
        // SCE_MESH.md section 10.11: on a durable stream a repeat is told by
        // its sequence number, exactly, and is answered with the
        // acknowledgement this machine can give, so a sender whose
        // acknowledgement was lost is answered.
        let durable = match (
            self.peers
                .get(peer)
                .and_then(|bound| bound.durable_in.as_ref()),
            head,
        ) {
            (Some(input), Some((_, Some(sequence)))) => {
                if input.holds(sequence) {
                    return Ok(if input.durable > 0 {
                        alloc::vec![Effect::AcknowledgeDue {
                            peer: peer.to_string(),
                        }]
                    } else {
                        Vec::new()
                    });
                }
                Some((sequence, input.held.saturating_add(1)))
            }
            (Some(_), Some((_, None))) => {
                return raise(Signal::MissingSequence {
                    source: peer.to_string(),
                })
            }
            _ => None,
        };
        // SCE_MESH.md section 10.5: the window is the link's, so a router keeps
        // one per peer it binds and no envelope makes one of its own.
        match self
            .inbound
            .admit_from(peer, bytes, delivery, durable.map(|(_, from)| from), now_ms)
        {
            Ok(outcome) => {
                if let Some((sequence, _)) = durable {
                    if let Some(input) = self
                        .peers
                        .get_mut(peer)
                        .and_then(|bound| bound.durable_in.as_mut())
                    {
                        input.admitted(sequence);
                    }
                }
                Ok(self.effects_of(outcome, Some(peer), now_ms))
            }
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
    /// `DeadlineExceeded` reply takes (§mesh-10.7.1). Then send again what a
    /// binding with durable receipt holds past its `retry_ms`.
    pub fn tick(&mut self, now_ms: i64) -> Result<Vec<Effect>, RouterError> {
        let mut effects = match self.inbound.tick(now_ms) {
            Ok(outcome) => self.effects_of(outcome, None, now_ms),
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
        // SCE_MESH.md section 10.11: an envelope held for an acknowledgement
        // that has not come within `retry_ms` is sent again, with its id,
        // sequence number and data, while its link is ready.
        for (name, bound) in &mut self.peers {
            if !bound.outbound.is_ready() {
                continue;
            }
            let Some(out) = bound.durable_out.as_mut() else {
                continue;
            };
            let retry_ms = out.send.retry_ms;
            for held in out.unacked.values_mut() {
                if held
                    .sent_ms
                    .is_none_or(|at| now_ms.saturating_sub(at) >= retry_ms)
                {
                    held.sent_ms = Some(now_ms);
                    effects.push(Effect::Transmit {
                        peer: name.clone(),
                        bytes: held.bytes.clone(),
                    });
                }
            }
        }
        // docs/adr/0085, section 3.2: each causal peer is owed this copy's
        // vector on its period, which repairs a loss no link edge reports.
        for (name, bound) in &self.peers {
            if let Some(causal) = &bound.causal {
                let due = match causal.announced_ms {
                    None => true,
                    Some(at) => now_ms.saturating_sub(at) >= causal.announce_every_ms,
                };
                if due {
                    effects.push(Effect::AnnounceDue { peer: name.clone() });
                }
            }
        }
        Ok(effects)
    }

    /// Envelope `bytes` the causal peer `peer` delivered: a change goes
    /// through the causal layer, which delivers it with whatever it releases,
    /// holds it or drops it; a `causalVector` is recorded as its sender's mark
    /// and answered with every change of the log it lacks. An envelope of
    /// another epoch, or by a replica the view does not hold, is raised as
    /// `causalViewMismatch` and is not delivered (docs/adr/0085, section
    /// 3.2).
    fn receive_causal(
        &mut self,
        peer: &str,
        bytes: Vec<u8>,
        now_ms: i64,
    ) -> Result<Vec<Effect>, RouterError> {
        // §mesh-10.12: every envelope of a causal peer reaches its layer
        // before the engine; a `causalVector` is a repair and a mark, and
        // never reaches an engine.
        let raise = |signal| {
            Ok(alloc::vec![Effect::Raise {
                peer: Some(peer.to_string()),
                signal,
            }])
        };
        let (source, pattern, tag) = match Envelope::decode(&mut SceCursor::new(&bytes)) {
            Ok(envelope) => (
                envelope.source.to_string(),
                envelope.pattern,
                envelope_tag(&envelope),
            ),
            Err(_) => {
                return raise(Signal::EnvelopeCorrupt {
                    source: None,
                    codec: "cbor",
                })
            }
        };
        let tag = match tag {
            Ok(Some(tag)) => tag,
            // An envelope on a causal route carries its tag; one without it,
            // or with half of it, cannot be placed, and is not delivered.
            Ok(None) | Err(_) => {
                return raise(Signal::EnvelopeCorrupt {
                    source: Some(source),
                    codec: "cbor",
                })
            }
        };
        let bound = self
            .peers
            .get_mut(peer)
            .ok_or_else(|| RouterError::UnknownPeer(peer.to_string()))?;
        let layer = &mut causal_of(peer, bound)?.layer;
        let taken = if pattern == PatternKind::CausalVector {
            layer
                .receive_summary(&tag)
                .map(|answer| (Vec::new(), answer))
        } else {
            layer
                .receive(&tag, bytes)
                .map(|delivered| (delivered, Vec::new()))
        };
        let (delivered, answer) = match taken {
            Ok(taken) => taken,
            Err(Rejected::OtherEpoch { epoch }) => {
                let replica = tag.vector.first().map(Replica::of);
                return raise(view_mismatch(source, epoch, replica));
            }
            Err(Rejected::NotInView { replica }) => {
                return raise(view_mismatch(source, tag.epoch, Some(replica)));
            }
            Err(Rejected::Malformed) => {
                return raise(Signal::EnvelopeCorrupt {
                    source: Some(source),
                    codec: "cbor",
                })
            }
            Err(Rejected::Rule(error)) => return Err(RouterError::Rule(error)),
        };
        let mut effects = Vec::new();
        for bytes in answer {
            effects.extend(dispatch(peer, bound, bytes, now_ms));
        }
        for bytes in delivered {
            // These bytes decoded on arrival, and decoding is a function of
            // the bytes alone.
            if let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&bytes)) {
                effects.push(deliver(&envelope, Some(peer)));
            }
        }
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
        let wire_id: [u8; 16] = envelope.invoke_id?.into();
        match self.requests.check(&wire_id, peer) {
            Answer::Undeclared => Some(Signal::RpcReplyFromUndeclaredPeer {
                source: envelope.source.to_string(),
                invoke_id: rpc::uuid_text(&wire_id),
            }),
            Answer::Admitted | Answer::Unknown => None,
        }
    }

    /// Turn what the receive half released into effects, in release order,
    /// followed by the rows it observed and the replies it answers duplicates
    /// with.
    fn effects_of(&mut self, outcome: Outcome, peer: Option<&str>, now_ms: i64) -> Vec<Effect> {
        let mut effects: Vec<Effect> = outcome
            .released
            .iter()
            .filter_map(|received| {
                let envelope = received.envelope();
                // A causal route's control message never reaches an engine
                // (docs/adr/0085, section 3.2); one on a binding that is not
                // causal is no message of its route, and is dropped.
                if envelope.pattern == PatternKind::CausalVector {
                    return None;
                }
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
        // §mesh-10.5: the reply a duplicate request is answered with goes back
        // on the binding the duplicate came in on. A tick releases no replay.
        if let Some(name) = peer {
            if let Some(bound) = self.peers.get_mut(name) {
                for reply in outcome.replays {
                    effects.extend(dispatch(name, bound, reply, now_ms));
                }
            }
        }
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
    envelope.invoke_id.map(Into::into)
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
        let source = sender_name(envelope, peer);
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
                    envelope
                        .rpc_error_message
                        .map(|message| message.into_inner()),
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
    /// The host named a peer it never bound with [`Router::add_peer`], and no
    /// instance of an open machine ([`Router::open_machine`]).
    UnknownPeer(String),
    /// An instance of the open machine `machine` was named for the first
    /// time while `max_instances` of it are bound (docs/adr/0113, D6): it is
    /// not bound, what arrived on its link is not taken in, and no bound
    /// instance is put out for it. The host sheds it (`503`, docs/adr/0109,
    /// D1) and unbinds the instances whose membership ended.
    InstancesFull {
        machine: String,
        max_instances: usize,
    },
    /// An envelope of the instance `peer`, taken in under the incarnation
    /// `incarnation`, was not run: its member stands under `current` now
    /// (docs/adr/0113, D4 and D5). Nothing of it was taken in; the host
    /// answers its instance that its incarnation is fenced (`412`).
    Fenced {
        peer: String,
        incarnation: u64,
        current: u64,
    },
    /// An envelope of the instance `peer`, whose binding receives durably,
    /// was handed to [`Router::receive`] without the incarnation it was taken
    /// in under: only [`Router::receive_member`] judges it (docs/adr/0113,
    /// D5). Nothing of it was taken in.
    Unjudged(String),
    /// An open machine whose instances cannot be told apart was asked to be
    /// bound ([`Router::open_machine`]); `reason` says why.
    BadOpenMachine {
        machine: String,
        reason: &'static str,
    },
    /// An envelope arrived on the link of `peer` naming another machine, `source`, as its
    /// sender, and was dropped: the link names the machine the host authenticated at its other
    /// end, and an envelope's `source` is only what its writer wrote (SCE_MESH.md section
    /// 10.9, invariant 10). Nothing the router keeps by sender saw it, and the machine was not
    /// told. Never raised on a causal route ([`PeerConfig::link_names_sender`]).
    SourceNotLink { peer: String, source: String },
    /// A standard rule refused its input: a sequence at the `u64` maximum,
    /// or a clock reading that cannot be subtracted.
    Rule(AlgorithmError),
    /// A sender has more envelopes held ahead of sequence than the ordering
    /// rule's list carries. `peer` is the binding that delivered the one
    /// that overflowed; a tick, which delivers nothing, cannot name one.
    HoldFull { peer: Option<String> },
    /// The host asked for the causal layer of a peer whose binding is not
    /// causal.
    NotCausal(String),
    /// A causal peer was used before the host started its causal layer
    /// ([`Router::start_causal`]).
    CausalNotStarted(String),
    /// The envelope could not be written within the bounds its document
    /// declares — an event whose data is longer than an envelope carries.
    Encode(CodecError),
    /// A Mesh request reached [`Router::invoke`] without a reserved param
    /// the build always gives it — a request the lowering did not make.
    MissingParam(&'static str),
    /// A reserved param's value is not what the build writes there.
    BadParam { name: &'static str, value: String },
    /// An event arrived on a link that may not deliver it, and was dropped before the machine
    /// was told ([`crate::endpoint::Endpoint::forbid_events_from`]).
    EventForbidden { peer: String, event: String },
    /// An event arrived that the host's [`crate::endpoint::Admission`] did not admit, and was
    /// dropped before the machine was told. `reason` is the admission's own word for it.
    EventNotAdmitted {
        peer: String,
        event: String,
        reason: String,
    },
    /// An event the machine sent was not sent: the host's [`crate::endpoint::Sealer`] said it
    /// must not leave, and `reason` is its word for it.
    EventNotSealed {
        target: String,
        event: String,
        reason: String,
    },
    /// The host asked for what only a router that checkpoints does (a seal, a durable report,
    /// durable receipt) of one that does not ([`Router::enable_checkpoints`]).
    NotCheckpointed,
    /// The host sealed a checkpoint whose number `given` is not above `sealed`, the last the
    /// router was sealed or restored with.
    CheckpointOutOfOrder { sealed: u64, given: u64 },
    /// The host reported durable the checkpoint `sequence`, which the router was never sealed
    /// with: the last it was is `sealed`.
    NotSealed { sequence: u64, sealed: u64 },
    /// Durable receipt was asked of a causal peer's binding, whose route repairs what it loses
    /// by its own vectors (SCE_MESH.md section 10.12).
    DurableOnCausal(String),
    /// An acknowledgement was asked for a peer this machine does not receive from durably.
    NotDurable(String),
    /// A `receipt:` block with a field below 1.
    BadReceipt {
        peer: String,
        window: usize,
        retry_ms: i64,
    },
}

/// The peer a `<send target>` names, when it names one — the runtime's
/// predicate, so the router and a generated send site's choice to reach it
/// are one answer rather than two copies of it.
pub use sce_rust_runtime::helpers::send::mesh_peer;

/// The peer a `<send>` target or an `<invoke>` `src` names: `#peer`
/// ([`mesh_peer`]), or the origin of an instance of an open machine,
/// `mesh://<machine>/<instance>` (docs/adr/0113, D2), the `_event.origin` of
/// its events, which a document keeps and sends to as `targetexpr`. Both
/// segments are unreserved characters alone, so the origin's percent-encoding
/// is the name itself, and a target that encodes one is not this form.
pub fn addressed_peer(target: &str) -> Option<&str> {
    mesh_peer(target).or_else(|| {
        target
            .strip_prefix(sce_rust_runtime::MESH_ORIGIN_SCHEME)
            .filter(|name| instance_of(name).is_some())
    })
}

/// Whether `envelope` names as its sender the one the link of `peer` names
/// (SCE_MESH.md section 10.9, invariants 10 and 11): the machine `peer` and
/// no instance, or, on the link of an instance of an open machine, that
/// machine and that instance (docs/adr/0113, D1).
fn names_link(envelope: &Envelope<'_>, peer: &str) -> bool {
    let instance = envelope.instance.map(|instance| instance.into_inner());
    match instance_of(peer) {
        Some((machine, of)) => envelope.source == machine && instance == Some(of),
        None => envelope.source == peer && instance.is_none(),
    }
}

/// The sender `envelope` names, as a link of it would be named: its machine,
/// or the pair `<machine>/<instance>` when it names an instance.
fn sender_of(envelope: &Envelope<'_>) -> String {
    match envelope.instance {
        Some(instance) => instance_peer(envelope.source.into_inner(), instance.into_inner()),
        None => envelope.source.to_string(),
    }
}

/// The sender a machine is told an event from `envelope` came from: the
/// machine its `source` names, or, on the link of an instance of an open
/// machine, the instance itself (`<machine>/<instance>`), whose
/// `mesh://<machine>/<instance>` is the event's `_event.origin` and the
/// address a reply is sent to (docs/adr/0113, D2). The link was held to the
/// envelope on arrival ([`names_link`]).
fn sender_name(envelope: &Envelope<'_>, peer: Option<&str>) -> String {
    match peer.filter(|peer| instance_of(peer).is_some()) {
        Some(instance) => instance.to_string(),
        None => envelope.source.to_string(),
    }
}

/// `envelope` as the bytes that go to `peer`, with the sequence number it is
/// stamped with: the peer's next, when its binding stamps one (§mesh-10.6.3)
/// and the envelope is one the receiver orders (`sequenced`: a reply takes
/// none), or when the binding sends with durable receipt, which numbers every
/// envelope (SCE_MESH.md section 10.11). What [`Router::send`] and
/// [`Router::invoke`] share once they know what to say, so a request and a
/// send to one peer draw from one counter.
///
/// Refused, as `backpressureDrop` (§mesh-16.7 row 9), when a binding with
/// durable receipt already holds its window of envelopes: a send that was
/// never accepted is told, and nothing accepted is dropped.
fn write_envelope(
    peer: &mut Peer,
    mut envelope: Envelope<'_>,
) -> Result<(Vec<u8>, Option<u64>), Signal> {
    if let Some(out) = &peer.durable_out {
        let held = out.held();
        if held >= out.send.window {
            return Err(Signal::BackpressureDrop {
                depth: u32::try_from(held).unwrap_or(u32::MAX),
            });
        }
    }
    // Reserved last, so that a place is taken only for an envelope that is written: nothing
    // below refuses one.
    if let Some(window) = &peer.window {
        window
            .reserve()
            .map_err(|depth| Signal::BackpressureDrop { depth })?;
    }
    let durable = peer.durable_out.is_some();
    let stamps = (peer.config.stamp_sequence || durable) && sequenced(envelope.pattern, durable);
    let sequence = stamps.then_some(peer.next_sequence);
    envelope.sequence_no = sequence;
    let bytes = envelope.encode_to_vec();
    // §mesh-10.6.3: the counter advances once per envelope written, so one
    // that could not be written leaves no hole; and it has no wrap guard,
    // 2^64 sends being beyond any deployment's lifetime.
    if stamps {
        peer.next_sequence = peer.next_sequence.wrapping_add(1);
    }
    Ok((bytes, sequence))
}

/// Hand `bytes`, written for `peer` with `sequence`, on: held for the next
/// checkpoint when the router checkpoints (`persist`, SCE_MESH.md section
/// 10.11), released at once otherwise.
fn hand_over(
    persist: Option<&mut Persist>,
    peer_name: &str,
    peer: &mut Peer,
    bytes: Vec<u8>,
    sequence: Option<u64>,
    now_ms: i64,
) -> Vec<Effect> {
    let Some(persist) = persist else {
        return release(peer_name, peer, bytes, sequence, now_ms);
    };
    if let Some(out) = peer.durable_out.as_mut() {
        out.waiting += 1;
    }
    persist.open.push(Unreleased {
        peer: peer_name.to_string(),
        sequence,
        envelope: bytes,
    });
    Vec::new()
}

/// Release `bytes`, written for `peer` with `sequence`: a binding with durable
/// receipt keeps it until it is acknowledged and sends it now if its link is
/// ready, at its readiness edge otherwise (SCE_MESH.md section 10.11); any
/// other hands it to the peer's send half (§mesh-10.10).
fn release(
    peer_name: &str,
    peer: &mut Peer,
    bytes: Vec<u8>,
    sequence: Option<u64>,
    now_ms: i64,
) -> Vec<Effect> {
    if let (Some(out), Some(sequence)) = (peer.durable_out.as_mut(), sequence) {
        if !peer.outbound.is_ready() {
            out.unacked.insert(
                sequence,
                Unacked {
                    bytes,
                    sent_ms: None,
                },
            );
            return Vec::new();
        }
        out.unacked.insert(
            sequence,
            Unacked {
                bytes: bytes.clone(),
                sent_ms: Some(now_ms),
            },
        );
        return alloc::vec![Effect::Transmit {
            peer: peer_name.to_string(),
            bytes,
        }];
    }
    dispatch(peer_name, peer, bytes, now_ms)
}

/// What a checkpoint holds of the bound peer `name`, its moments on the wall
/// clock `clocks` read.
fn peer_state(name: &str, bound: &Peer, clocks: Clocks) -> PeerState {
    PeerState {
        peer: name.to_string(),
        next_sequence: bound.next_sequence,
        queued: bound
            .outbound
            .queued()
            .map(|(bytes, at)| QueuedState {
                envelope: bytes.to_vec(),
                since: clocks.to_wall(at),
            })
            .collect(),
        unacked: bound
            .durable_out
            .as_ref()
            .map(|out| {
                out.unacked
                    .iter()
                    .map(|(sequence, held)| UnackedState {
                        sequence: *sequence,
                        envelope: held.bytes.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        received: bound.durable_in.as_ref().map(|input| ReceivedState {
            held: input.held,
            above: input.above.iter().copied().collect(),
        }),
    }
}

/// Hand written envelope `bytes` to `peer`'s send half.
fn dispatch(peer_name: &str, peer: &mut Peer, bytes: Vec<u8>, now_ms: i64) -> Vec<Effect> {
    match peer.outbound.admit(bytes, now_ms) {
        Admitted::Send(bytes) => alloc::vec![Effect::Transmit {
            peer: peer_name.to_string(),
            bytes,
        }],
        Admitted::Queued => Vec::new(),
        Admitted::Dropped(signal) => alloc::vec![Effect::Raise {
            peer: Some(peer_name.to_string()),
            signal,
        }],
    }
}

/// Offer `envelope` to the causal layer of `peer` (docs/adr/0085, section 3,
/// "offer and verdict"): the verdict is the host's to act on, and nothing
/// leaves before the host commits an accepted change's journal record.
fn offer(
    peer_name: &str,
    peer: &mut Peer,
    envelope: Envelope<'_>,
) -> Result<Vec<Effect>, RouterError> {
    let layer = &mut causal_of(peer_name, peer)?.layer;
    let verdict = layer
        .offer(|tag| {
            stamp(envelope, tag)
                .map(|stamped| stamped.encode_to_vec())
                .map_err(|_| RouterError::Encode(CodecError::CborOutOfRange))
        })
        .map_err(|error| match error {
            OfferError::Build(error) => error,
            OfferError::Rule(error) => RouterError::Rule(error),
        })?;
    Ok(alloc::vec![match verdict {
        Verdict::Accepted(dot) => Effect::Accepted {
            peer: peer_name.to_string(),
            dot,
        },
        Verdict::Refused(refusal) => Effect::Refused {
            peer: peer_name.to_string(),
            refusal,
        },
    }])
}

/// The started causal layer of the causal peer `peer_name`.
fn causal_of<'a>(peer_name: &str, peer: &'a mut Peer) -> Result<&'a mut Causal, RouterError> {
    if peer.config.causal.is_none() {
        return Err(RouterError::NotCausal(peer_name.to_string()));
    }
    peer.causal
        .as_mut()
        .ok_or_else(|| RouterError::CausalNotStarted(peer_name.to_string()))
}

/// The row an envelope of another epoch, or of a replica the view does not
/// hold, raises.
fn view_mismatch(source: String, epoch: u64, replica: Option<Replica>) -> Signal {
    Signal::CausalViewMismatch {
        source,
        epoch,
        replica: replica
            .map(|replica| rpc::uuid_text(&replica.0))
            .unwrap_or_default(),
    }
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
pub(crate) fn payload_text(envelope: &Envelope<'_>) -> Result<String, &'static str> {
    match envelope.datacontenttype {
        PayloadCodec::None => Ok(String::new()),
        PayloadCodec::Json => core::str::from_utf8(envelope.data.into_inner())
            .map(str::to_string)
            .map_err(|_| "json"),
        PayloadCodec::Cbor => Err("cbor"),
        PayloadCodec::Typed => Err("typed"),
        PayloadCodec::Raw => Err("raw"),
    }
}

/// `value` as an envelope's text or byte string of at most `MAX` bytes. The
/// envelope holds each within its bound (docs/adr/0077 §3), so a value past it
/// has no envelope and is refused here, with the error its encode gave it
/// before the type held the bound (`CodecError::CborOutOfRange`).
pub(crate) fn within<L: Entries, const MAX: usize>(
    value: L,
) -> Result<Bounded<L, MAX>, RouterError> {
    Bounded::new(value).ok_or(RouterError::Encode(CodecError::CborOutOfRange))
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
            source: sender_name(envelope, peer),
            send_id: envelope.subject.map(|subject| subject.to_string()),
            // §mesh-10.7: `_event.invokeid` of an inbound request is its
            // wire invokeid, as RFC 4122 text.
            invoke_id: (envelope.pattern == PatternKind::RpcRequest)
                .then_some(envelope.invoke_id)
                .flatten()
                .map(|id| rpc::uuid_text(id.into())),
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
        responders: Responders::Named(&["hmi"]),
        deadline_ms: None,
        reply_events: &[],
        causal: None,
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
                responders: Responders::Named(&["hmi"]),
                ..config
            },
        );
        hmi.add_peer(
            "ecu",
            PeerConfig {
                responders: Responders::Named(&["ecu"]),
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
                id: (&[id; 16]).into(),
                source: within("ecu").unwrap(),
                event_type: within("a").unwrap(),
                pattern: PatternKind::FireForget,
                datacontenttype: codec,
                data: (&[0x01]).into(),
                ..Envelope::new()
            }
            .encode_to_vec();
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

    /// docs/adr/0085, section 3.2: a `causalVector` on a binding that is not
    /// causal is dropped, not delivered as an event.
    #[test]
    fn a_causal_routes_control_message_never_reaches_the_engine() {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer("ecu", UNORDERED);
        let bytes = Envelope {
            id: (&[1u8; 16]).into(),
            source: within("ecu").unwrap(),
            event_type: within("").unwrap(),
            pattern: PatternKind::CausalVector,
            datacontenttype: PayloadCodec::None,
            ..Envelope::new()
        }
        .encode_to_vec();
        let effects = hmi.receive("ecu", bytes, 0).unwrap();
        assert!(effects.is_empty(), "{effects:?}");
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
            r#"{"errorName":"communication","reason":"transportUnavailable","target":"nowhere"}"#
        );

        ecu.peer_ready("hmi", 0).unwrap();
        let lost = ecu.peer_not_ready("hmi").unwrap();
        let [Effect::Raise { peer, signal }] = lost.as_slice() else {
            panic!("expected one raised row, got {lost:?}");
        };
        assert_eq!(
            signal.event_data(ecu.binding(peer.as_deref())),
            r#"{"errorName":"communication","reason":"transportUnavailable","target":"hmi","transport":"wss"}"#
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
            responders: Responders::Named(&["hmi"]),
            deadline_ms,
            ..UNORDERED
        };
        ecu.add_peer("hmi", config);
        ecu.add_peer(
            "mallory",
            PeerConfig {
                responders: Responders::Named(&["mallory"]),
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
        reply_by("hmi", id, status, message, data)
    }

    /// [`reply`], written by `source`.
    fn reply_by(
        source: &str,
        id: u8,
        status: Option<RpcStatus>,
        message: Option<&str>,
        data: &str,
    ) -> Vec<u8> {
        Envelope {
            id: (&[id; 16]).into(),
            source: within(source).unwrap(),
            event_type: within("service.response.force").unwrap(),
            pattern: PatternKind::RpcReply,
            datacontenttype: payload_codec(data),
            data: within(data.as_bytes()).unwrap(),
            invoke_id: Some((&WIRE).into()),
            rpc_status: status,
            rpc_error_message: message.map(|m| within(m).unwrap()),
            ..Envelope::new()
        }
        .encode_to_vec()
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
        assert_eq!(envelope.invoke_id.map(<[u8; 16]>::from), Some(WIRE));
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
                data: r#"{"errorName":"invoke","reason":"unavailable","detail":"busy","source":"hmi","invoke_id":"09090909-0909-0909-0909-090909090909"}"#.to_string(),
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
            ecu.receive(
                "mallory",
                reply_by("mallory", 2, Some(RpcStatus::Ok), None, "1"),
                20
            )
            .unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("mallory".to_string()),
                signal: Signal::RpcReplyFromUndeclaredPeer {
                    // What refused it was the binding it came in on, whose
                    // machine the envelope names.
                    source: "mallory".to_string(),
                    invoke_id: "09090909-0909-0909-0909-090909090909".to_string(),
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

    /// SCE_MESH.md section 10.9, invariant 10, before §mesh-14.6 is asked: a reply that
    /// arrives on mallory's link naming hmi, the declared responder, is
    /// refused to the host, ends nothing and raises nothing, and the request
    /// is still answered by hmi. Red if the router takes the envelope's
    /// `source` over its link: the reply is then row 14 under hmi's name.
    #[test]
    fn a_reply_naming_the_responder_on_another_link_is_refused_to_the_host() {
        let mut ecu = requester(None);
        started(&mut ecu, None);
        assert_eq!(
            ecu.receive("mallory", reply(2, Some(RpcStatus::Ok), None, "1"), 20),
            Err(RouterError::SourceNotLink {
                peer: "mallory".to_string(),
                source: "hmi".to_string(),
            })
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
                data: r#"{"errorName":"invoke","reason":"deadlineExceeded","invoke_id":"09090909-0909-0909-0909-090909090909"}"#.to_string(),
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
            id: (&[2; 16]).into(),
            source: within("hmi").unwrap(),
            event_type: within("service.response.force").unwrap(),
            pattern: PatternKind::RpcReply,
            datacontenttype: PayloadCodec::Raw,
            data: (&[0xFF]).into(),
            invoke_id: Some((&WIRE).into()),
            ..Envelope::new()
        }
        .encode_to_vec();
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
                r##"{"errorName":"execution","reason":"invokeSrcNotFound","detail":"no binding for '#nobody'"}"##
                    .to_string()
            ))
        );
        assert!(matches!(
            ecu.invoke(&ask("hmi", None), WIRE, [1; 16], 0, 0),
            Ok(Invoked::Refused(data)) if data.contains("names no Mesh peer")
        ));
    }

    // ── §mesh-10.7: the responder's half — the request in, its reply out ──

    const WIRE_TEXT: &str = "09090909-0909-0909-0909-090909090909";

    /// `hmi`, answering `ecu`: `service.response.force` is a reply to it.
    fn responder() -> Router {
        responder_with_window(8)
    }

    /// [`responder`] with a duplicate window of `window` ids.
    fn responder_with_window(window: u32) -> Router {
        let mut hmi = Router::new("hmi", window, 50).unwrap();
        hmi.add_peer(
            "ecu",
            PeerConfig {
                buffer: None,
                responders: Responders::Named(&["ecu"]),
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
                invoke_id: Some(WIRE_TEXT.to_string()),
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

        let bytes = sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 0));
        let reply = Envelope::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(reply.pattern, PatternKind::RpcReply);
        assert_eq!(reply.invoke_id.map(<[u8; 16]>::from), Some(WIRE));
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
        let bytes = sent_to_ecu(responder().send(&answer("status.changed", WIRE_TEXT), [5; 16], 0));
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

    // ── §mesh-10.5: the reply cache ──────────────────────────────────────

    /// A duplicate of a request the responder answered is answered with the
    /// reply the first copy produced, byte for byte, and the machine is not
    /// told the request again.
    #[test]
    fn a_duplicate_of_an_answered_request_is_answered_with_the_first_reply() {
        let mut ecu = requester(None);
        let mut hmi = responder();
        let asked = started(&mut ecu, None);
        assert_eq!(hmi.receive("ecu", asked.clone(), 0).unwrap().len(), 1);
        let first = sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 0));
        assert_eq!(sent_to_ecu(hmi.receive("ecu", asked, 10)), first);
    }

    /// A duplicate of a request still being handled is dropped, and the reply
    /// goes once, when it is made. The first reply is the one kept: a second
    /// reply to the same request leaves, and a later duplicate still gets the
    /// first.
    #[test]
    fn a_duplicate_of_a_request_still_handled_is_dropped() {
        let mut ecu = requester(None);
        let mut hmi = responder();
        let asked = started(&mut ecu, None);
        assert_eq!(hmi.receive("ecu", asked.clone(), 0).unwrap().len(), 1);
        let early = hmi.receive("ecu", asked.clone(), 5).unwrap();
        assert!(early.is_empty(), "{early:?}");

        let first = sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 6));
        let second = sent_to_ecu(hmi.send(
            &HostSendRequest {
                event_data: r#"{"force":99}"#.to_string(),
                ..answer("service.response.force", WIRE_TEXT)
            },
            [6; 16],
            7,
        ));
        assert_ne!(first, second);
        assert_eq!(sent_to_ecu(hmi.receive("ecu", asked, 8)), first);
    }

    /// The cache is bounded by the window: once the request's id leaves the
    /// window, its reply goes with it, and the same request is taken as new.
    #[test]
    fn a_reply_leaves_the_cache_with_its_requests_window_entry() {
        let mut ecu = requester(None);
        let mut hmi = responder_with_window(1);
        let asked = started(&mut ecu, None);
        hmi.receive("ecu", asked.clone(), 0).unwrap();
        sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 0));

        // Another envelope from `ecu` takes the window's one place.
        let other = transmitted(ecu.send(&request("#hmi", "status.changed", ""), [2; 16], 0));
        hmi.receive("ecu", other, 1).unwrap();

        let again = hmi.receive("ecu", asked, 2).unwrap();
        assert!(
            matches!(again.first(), Some(Effect::Deliver { event, .. }) if event == "service.request.force"),
            "{again:?}"
        );
        assert!(
            !again
                .iter()
                .any(|effect| matches!(effect, Effect::Transmit { .. })),
            "no reply is sent again for a request the window let go: {again:?}"
        );
    }

    // ── SCE_MESH.md section 10.9, invariant 10: the link names the sender ──

    /// An HTTPS server, `cloud`, that keeps the mailboxes of two clients,
    /// `alice` and `bob`, each the name its exchange path and its
    /// authenticator prove (SCE_MESH.md sections 22.1 and 22.5), and answers
    /// `service.request.force` with `service.response.force`.
    fn https_server() -> Router {
        let mut cloud = Router::new("cloud", 8, 50).unwrap();
        let accepting = PeerConfig {
            transport: "https",
            buffer: None,
            reply_events: &["service.response.force"],
            ..UNORDERED
        };
        cloud.add_peer(
            "alice",
            PeerConfig {
                responders: Responders::Named(&["alice"]),
                ..accepting
            },
        );
        cloud.add_peer(
            "bob",
            PeerConfig {
                responders: Responders::Named(&["bob"]),
                ..accepting
            },
        );
        cloud
    }

    /// The request `service.request.force` with the wire invokeid
    /// [`WIRE`], written as `source` wrote it.
    fn force_request_by(source: &str) -> Vec<u8> {
        Envelope {
            id: (&[0xB0; 16]).into(),
            source: within(source).unwrap(),
            event_type: within("service.request.force").unwrap(),
            pattern: PatternKind::RpcRequest,
            datacontenttype: PayloadCodec::Json,
            data: within(r#"{"n":3}"#.as_bytes()).unwrap(),
            invoke_id: Some((&WIRE).into()),
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    /// An authenticated client that writes another client's name as its
    /// `source` is refused to the host, and takes nothing of that client's:
    /// not the id in its duplicate window, so the other's own request is
    /// delivered, and not the reply its cache keeps, so a repeat drew no reply
    /// onto the forger's link. Red if the router takes `source` over the
    /// link: alice's copy is then delivered as bob's, bob's own is dropped as
    /// a duplicate, and alice's repeat is answered with bob's reply.
    #[test]
    fn a_client_that_names_another_as_its_source_is_refused_and_takes_nothing_of_its() {
        let mut cloud = https_server();
        let forged = force_request_by("bob");
        let refused = Err(RouterError::SourceNotLink {
            peer: "alice".to_string(),
            source: "bob".to_string(),
        });
        assert_eq!(cloud.receive("alice", forged.clone(), 0), refused);

        // The same bytes on bob's link are bob's: his window did not hold the id.
        assert_eq!(
            cloud.receive("bob", forged.clone(), 1).unwrap(),
            alloc::vec![Effect::Deliver {
                event: "service.request.force".to_string(),
                data: r#"{"n":3}"#.to_string(),
                source: "bob".to_string(),
                send_id: None,
                invoke_id: Some(WIRE_TEXT.to_string()),
            }]
        );
        let answered = cloud
            .send(
                &HostSendRequest {
                    invoke_id: WIRE_TEXT.to_string(),
                    ..request("#bob", "service.response.force", r#"{"force":12}"#)
                },
                [5; 16],
                2,
            )
            .unwrap();
        let [Effect::Transmit { peer, bytes: reply }] = answered.as_slice() else {
            panic!("expected the reply to bob, got {answered:?}");
        };
        assert_eq!(peer, "bob");

        // A repeat on alice's link draws nothing from bob's reply cache; a
        // repeat on bob's is answered with what he was answered.
        assert_eq!(cloud.receive("alice", forged.clone(), 3), refused);
        assert_eq!(
            cloud.receive("bob", forged, 4).unwrap(),
            alloc::vec![Effect::Transmit {
                peer: "bob".to_string(),
                bytes: reply.clone(),
            }]
        );
    }

    /// SCE_MESH.md section 10.5: an endpoint keeps one window per link. Ten thousand envelopes
    /// on alice's link, each claiming a source of its own, are each refused to the host and make
    /// no window; alice's own envelope then makes the one window her link has. Red if the
    /// router lets an envelope name its sender: each claim then makes a window of its own.
    #[test]
    fn claims_of_ten_thousand_sources_on_one_link_make_no_window_of_their_own() {
        let mut cloud = https_server();
        for n in 0..10_000u32 {
            let claimed = alloc::format!("s{n}");
            assert_eq!(
                cloud.receive("alice", force_request_by(&claimed), i64::from(n)),
                Err(RouterError::SourceNotLink {
                    peer: "alice".to_string(),
                    source: claimed,
                })
            );
        }
        assert_eq!(cloud.inbound.senders(), 0, "no claim made a window");
        assert!(cloud
            .receive("alice", force_request_by("alice"), 10_000)
            .is_ok());
        assert_eq!(cloud.inbound.senders(), 1, "alice's link has its one");
    }

    /// An HTTPS server, `cloud`, bound to the open machine `phone` as well as to
    /// `alice` and `bob` (docs/adr/0113): no instance of `phone` is listed,
    /// and at most `max_instances` of them are bound at once.
    fn open_server(max_instances: usize) -> Router {
        let mut cloud = https_server();
        cloud
            .open_machine(
                "phone",
                PeerConfig {
                    transport: "https",
                    buffer: None,
                    responders: Responders::Instance,
                    reply_events: &["service.response.force"],
                    ..UNORDERED
                },
                max_instances,
            )
            .unwrap();
        cloud
    }

    /// The request [`force_request_by`] writes, from the instance `instance`
    /// of `source`, under the envelope id `[id; 16]`.
    fn force_request_from(source: &str, instance: &str, id: u8) -> Vec<u8> {
        Envelope {
            id: (&[id; 16]).into(),
            source: within(source).unwrap(),
            instance: Some(within(instance).unwrap()),
            event_type: within("service.request.force").unwrap(),
            pattern: PatternKind::RpcRequest,
            datacontenttype: PayloadCodec::Json,
            data: within(r#"{"n":3}"#.as_bytes()).unwrap(),
            invoke_id: Some((&WIRE).into()),
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    /// docs/adr/0113, D3: an instance of an open machine is bound by the one
    /// lookup the first time it is named, under its machine's configuration
    /// and as its pair, and its event is told as the instance's, whose origin
    /// is `mesh://phone/a1`. Neither the machine's bare name nor a name that
    /// holds no instance id binds anything. Red if the router binds only the
    /// peers it was given, or tells the event as the machine's.
    #[test]
    fn an_instance_of_an_open_machine_is_bound_as_it_arrives() {
        let mut cloud = open_server(4);
        assert!(!cloud.has_peer("phone/a1"));
        assert_eq!(
            cloud
                .receive("phone/a1", force_request_from("phone", "a1", 1), 0)
                .unwrap(),
            alloc::vec![Effect::Deliver {
                event: "service.request.force".to_string(),
                data: r#"{"n":3}"#.to_string(),
                source: "phone/a1".to_string(),
                send_id: None,
                invoke_id: Some(WIRE_TEXT.to_string()),
            }]
        );
        assert!(cloud.has_peer("phone/a1"));
        assert_eq!(
            cloud
                .binding(Some("phone/b2"))
                .and_then(|binding| binding.transport),
            Some("https"),
            "an instance not yet bound is named with its machine's transport"
        );
        for name in ["phone", "phone/", "phone/a 1", "elsewhere/a1"] {
            assert_eq!(
                cloud.receive(name, force_request_by("phone"), 1),
                Err(RouterError::UnknownPeer(name.to_string())),
                "{name}"
            );
        }
    }

    /// SCE_MESH.md section 10.9, invariant 11 (docs/adr/0113, D1): the link of
    /// an instance names the pair, so an envelope on it that names another
    /// instance, no instance or another machine is refused to the host and
    /// makes no window; and a link of no instance carries none. Red if the
    /// instance is not held to the link, or if `source` alone is.
    #[test]
    fn an_envelope_on_an_instances_link_names_its_machine_and_its_instance() {
        let mut cloud = open_server(4);
        for (bytes, claimed) in [
            (force_request_from("phone", "b2", 1), "phone/b2"),
            (force_request_by("phone"), "phone"),
            (force_request_from("tablet", "a1", 2), "tablet/a1"),
        ] {
            assert_eq!(
                cloud.receive("phone/a1", bytes, 0),
                Err(RouterError::SourceNotLink {
                    peer: "phone/a1".to_string(),
                    source: claimed.to_string(),
                })
            );
        }
        assert_eq!(cloud.inbound.senders(), 0, "no claim made a window");
        assert_eq!(
            cloud.receive("alice", force_request_from("alice", "a1", 3), 1),
            Err(RouterError::SourceNotLink {
                peer: "alice".to_string(),
                source: "alice/a1".to_string(),
            }),
            "a link of no instance carries none"
        );
    }

    /// docs/adr/0113, D6: no instance is bound past the open machine's bound,
    /// and none that is bound is put out for a new one; once the host unbinds
    /// one, a new one is bound. A declared peer is no instance and is not
    /// counted. Red without the bound, or with an eviction.
    #[test]
    fn no_instance_is_bound_past_the_bound_and_none_is_put_out_for_one() {
        let mut cloud = open_server(1);
        assert!(cloud
            .receive("phone/a1", force_request_from("phone", "a1", 1), 0)
            .is_ok());
        assert_eq!(
            cloud.receive("phone/b2", force_request_from("phone", "b2", 2), 1),
            Err(RouterError::InstancesFull {
                machine: "phone".to_string(),
                max_instances: 1,
            })
        );
        assert!(!cloud.has_peer("phone/b2"));
        assert!(
            cloud.has_peer("phone/a1"),
            "the bound instance is not put out"
        );
        assert!(
            cloud.receive("alice", force_request_by("alice"), 2).is_ok(),
            "a declared peer is no instance"
        );
        cloud.remove_peer("phone/a1");
        assert!(cloud
            .receive("phone/b2", force_request_from("phone", "b2", 2), 3)
            .is_ok());
    }

    /// docs/adr/0113, D2: a document answers an instance at the origin it was
    /// told, `mesh://phone/a1`, which reaches the instance's link; a target of
    /// another shape names no peer. Red if a send names only `#peer`.
    #[test]
    fn a_document_answers_an_instance_at_its_origin() {
        let mut cloud = open_server(4);
        cloud
            .receive("phone/a1", force_request_from("phone", "a1", 1), 0)
            .unwrap();
        let answered = cloud
            .send(
                &HostSendRequest {
                    invoke_id: WIRE_TEXT.to_string(),
                    ..request(
                        "mesh://phone/a1",
                        "service.response.force",
                        r#"{"force":12}"#,
                    )
                },
                [5; 16],
                1,
            )
            .unwrap();
        let [Effect::Transmit { peer, bytes }] = answered.as_slice() else {
            panic!("expected the reply to the instance, got {answered:?}");
        };
        assert_eq!(peer, "phone/a1");
        let reply = Envelope::decode(&mut SceCursor::new(bytes)).unwrap();
        assert_eq!(reply.pattern, PatternKind::RpcReply);
        assert_eq!(reply.source, "cloud");
        for target in ["mesh://phone", "mesh://phone/a%31", "mesh://phone/a1/x"] {
            assert_eq!(
                cloud.send(&request(target, "service.response.force", "{}"), [6; 16], 2),
                Err(RouterError::NotMeshTarget(target.to_string())),
                "{target}"
            );
        }
    }

    /// The wall and monotonic clocks the checkpoints of these tests are taken against.
    const CLOCKS: Clocks = Clocks {
        monotonic_ms: 0,
        wall_ms: 1_000_000,
    };

    /// `cloud`, checkpointing, whose instances of `phone` send to it with durable receipt and
    /// are judged by their completion records (docs/adr/0113, D5).
    fn durable_open_server() -> Router {
        let mut cloud = open_server(4);
        cloud.enable_checkpoints();
        cloud.receive_durably("phone").unwrap();
        cloud
    }

    /// The request `service.request.force` the instance `a1` of `phone` numbered `number` in
    /// its incarnation, with the wire invokeid `[number; 16]`.
    fn numbered(number: u8) -> Vec<u8> {
        Envelope {
            id: (&[0x40 + number; 16]).into(),
            source: within("phone").unwrap(),
            instance: Some(within("a1").unwrap()),
            event_type: within("service.request.force").unwrap(),
            pattern: PatternKind::RpcRequest,
            datacontenttype: PayloadCodec::Json,
            data: within(r#"{"n":3}"#.as_bytes()).unwrap(),
            invoke_id: Some((&[number; 16]).into()),
            sequence_no: Some(u64::from(number)),
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    /// docs/formal/OpenInstances.tla, `OpenInstances_without_records`: a repeat of a request is
    /// answered from its completion record however many requests came between, where a window
    /// of ids (eight here) would have let it go and run it again. Red with a window in place of
    /// the records: the repeat is delivered a second time.
    #[test]
    fn a_repeat_is_answered_from_its_record_however_many_came_between() {
        let mut cloud = durable_open_server();
        let standing = Standing {
            incarnation: 7,
            mark: 1,
        };
        let first = cloud
            .receive_member("phone/a1", 7, standing, numbered(1), 0)
            .unwrap();
        assert!(
            matches!(first.as_slice(), [Effect::Deliver { .. }]),
            "{first:?}"
        );
        let held = cloud
            .send(
                &HostSendRequest {
                    invoke_id: rpc::uuid_text(&[1; 16]),
                    ..request("mesh://phone/a1", "service.response.force", "{}")
                },
                [5; 16],
                1,
            )
            .unwrap();
        assert!(held.is_empty(), "a reply waits for its checkpoint");
        cloud.seal(1, CLOCKS).unwrap();
        let released = cloud.durable(1, 2).unwrap();
        let [Effect::Transmit { peer, bytes: reply }] = released.as_slice() else {
            panic!("expected the reply to leave, got {released:?}");
        };
        assert_eq!(peer, "phone/a1");
        for number in 2..=20 {
            let taken = cloud
                .receive_member("phone/a1", 7, standing, numbered(number), 3)
                .unwrap();
            assert!(
                matches!(taken.as_slice(), [Effect::Deliver { .. }]),
                "{number}: {taken:?}"
            );
        }
        assert_eq!(
            cloud
                .receive_member("phone/a1", 7, standing, numbered(1), 4)
                .unwrap(),
            alloc::vec![Effect::Transmit {
                peer: "phone/a1".to_string(),
                bytes: reply.clone(),
            }],
            "the repeat is answered from its record, nineteen requests later"
        );
    }

    /// docs/formal/OpenInstances.tla, `OpenInstances_without_recheck`: the incarnation and the
    /// mark are judged again as a request runs, since either may have moved while it waited. One
    /// taken in under an incarnation its member no longer stands under is fenced; one whose
    /// number the mark passed is one whose answer the instance has, and is not run. Red without
    /// the second judgement: a request that takes effect twice.
    #[test]
    fn a_request_is_judged_again_as_it_runs() {
        let mut cloud = durable_open_server();
        assert_eq!(
            cloud.receive_member(
                "phone/a1",
                7,
                Standing {
                    incarnation: 9,
                    mark: 1
                },
                numbered(1),
                0
            ),
            Err(RouterError::Fenced {
                peer: "phone/a1".to_string(),
                incarnation: 7,
                current: 9,
            })
        );
        let now = Standing {
            incarnation: 9,
            mark: 1,
        };
        assert!(matches!(
            cloud
                .receive_member("phone/a1", 9, now, numbered(1), 1)
                .unwrap()
                .as_slice(),
            [Effect::Deliver { .. }]
        ));
        assert_eq!(
            cloud
                .receive_member(
                    "phone/a1",
                    9,
                    Standing {
                        incarnation: 9,
                        mark: 3
                    },
                    numbered(2),
                    2
                )
                .unwrap(),
            Vec::new(),
            "the mark passed it while it waited"
        );
        assert_eq!(
            cloud.receive_member(
                "phone/a1",
                7,
                Standing {
                    incarnation: 7,
                    mark: 1
                },
                numbered(3),
                3
            ),
            Err(RouterError::Fenced {
                peer: "phone/a1".to_string(),
                incarnation: 7,
                current: 9,
            }),
            "a standing of the past changes nothing"
        );
    }

    /// docs/formal/OpenInstances.tla, `OpenInstances_without_durable_mark` and `SCrash`: the
    /// records are in the checkpoint with the mark the store held, so a process that restores it
    /// answers a repeat from them and does not run it again; a record is dropped only once the
    /// store's mark passes it, never by what a process saw. The checkpoint names the incarnation
    /// its run was made under, and one that took nothing in names none. Red if the records are
    /// not checkpointed, or are dropped by another mark.
    #[test]
    fn the_records_are_checkpointed_and_dropped_by_the_stores_mark() {
        let mut cloud = durable_open_server();
        let standing = Standing {
            incarnation: 7,
            mark: 1,
        };
        cloud
            .receive_member("phone/a1", 7, standing, numbered(3), 0)
            .unwrap();
        let state = cloud.seal(1, CLOCKS).unwrap();
        assert_eq!(cloud.sealed_under(), [("phone/a1".to_string(), 7)]);
        cloud.seal(2, CLOCKS).unwrap();
        assert!(
            cloud.sealed_under().is_empty(),
            "a run that took nothing in"
        );

        let mut restored = durable_open_server();
        assert_eq!(
            restored.restore(&state, 1, false, CLOCKS),
            Ok(Vec::new()),
            "an instance judged by its records holds what it sent: no restored row"
        );
        assert_eq!(
            restored
                .receive_member("phone/a1", 7, standing, numbered(3), 1)
                .unwrap(),
            Vec::new(),
            "a repeat after a restore is not run again"
        );
        let records = |router: &mut Router, sequence: u64| {
            router
                .seal(sequence, CLOCKS)
                .unwrap()
                .senders
                .into_iter()
                .find(|sender| sender.source == "phone/a1")
                .and_then(|sender| sender.records)
                .expect("the instance's records")
        };
        let kept = records(&mut restored, 2);
        assert_eq!((kept.mark, kept.kept.len()), (1, 1));
        restored.stand(
            "phone/a1",
            Standing {
                incarnation: 7,
                mark: 4,
            },
        );
        let kept = records(&mut restored, 3);
        assert_eq!(
            (kept.mark, kept.kept.len()),
            (4, 0),
            "the store's mark passed it"
        );
        restored.stand(
            "phone/a1",
            Standing {
                incarnation: 7,
                mark: 2,
            },
        );
        assert_eq!(records(&mut restored, 4).mark, 4, "a mark never falls");
    }

    /// docs/adr/0113, D5: an instance whose binding receives durably is judged only with the
    /// incarnation it was taken in under, and a peer judged by no records is given none; an open
    /// machine's instances receive durably only on a router that checkpoints.
    #[test]
    fn only_an_instance_that_receives_durably_is_judged_by_records() {
        let mut cloud = durable_open_server();
        assert_eq!(
            cloud.receive("phone/a1", numbered(1), 0),
            Err(RouterError::Unjudged("phone/a1".to_string()))
        );
        assert_eq!(
            cloud.receive_member(
                "alice",
                7,
                Standing {
                    incarnation: 7,
                    mark: 1
                },
                force_request_by("alice"),
                0
            ),
            Err(RouterError::NotDurable("alice".to_string()))
        );
        assert_eq!(
            open_server(4).receive_durably("phone"),
            Err(RouterError::NotCheckpointed)
        );
    }

    /// docs/adr/0113, D6 and D7: an open machine is bound only with room for
    /// one instance at least, on a binding whose link names its sender, and
    /// under a name a path carries. Red if any of the three is taken.
    #[test]
    fn an_open_machine_is_bound_only_where_its_instances_can_be_told_apart() {
        let mut cloud = https_server();
        let config = PeerConfig {
            responders: Responders::Instance,
            ..UNORDERED
        };
        for (machine, config, max_instances) in [
            ("phone", config, 0),
            (
                "phone",
                PeerConfig {
                    causal: PEOPLE.causal,
                    ..config
                },
                4,
            ),
            ("ph/one", config, 4),
        ] {
            assert!(matches!(
                cloud.open_machine(machine, config, max_instances),
                Err(RouterError::BadOpenMachine { .. })
            ));
        }
        assert!(cloud.open_machine("phone", config, 4).is_ok());
    }

    /// §mesh-16.7 row 5: the link a request waits on is lost. The request is
    /// forgotten and named; a reply or a deadline that comes afterwards ends
    /// nothing.
    #[test]
    fn a_lost_link_is_row_5_for_each_request_waiting_on_it() {
        let mut ecu = requester(Some(100));
        started(&mut ecu, None);
        assert_eq!(
            ecu.peer_not_ready("hmi").unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("hmi".to_string()),
                signal: Signal::InvokeChildLost {
                    invoke_id: WIRE_TEXT.to_string(),
                    target: "hmi".to_string(),
                },
            }]
        );
        assert!(ecu
            .receive("hmi", reply(2, Some(RpcStatus::Ok), None, "1"), 20)
            .unwrap()
            .is_empty());
        assert!(ecu.tick(1000).unwrap().is_empty());
    }

    /// SCE_MESH.md section 22.4: an HTTPS exchange that failed interrupts the
    /// link and loses nothing, since the server keeps the reply in the
    /// client's mailbox. The edge is row 1 and not row 5, the request stays
    /// waiting, and the reply that reaches the client afterwards ends it
    /// once. Red if an interruption forgets the request as a lost link does:
    /// the reply then finds nothing waiting and the invocation never ends.
    #[test]
    fn an_interrupted_link_keeps_the_request_and_its_later_reply_ends_it_once() {
        let mut ecu = requester(None);
        assert!(ecu.peer_ready("hmi", 0).unwrap().is_empty());
        started(&mut ecu, None);
        assert_eq!(
            ecu.peer_interrupted("hmi").unwrap(),
            alloc::vec![Effect::Raise {
                peer: Some("hmi".to_string()),
                signal: Signal::TransportUnavailable,
            }]
        );
        assert!(ecu.awaits_reply(&WIRE));
        assert!(
            ecu.peer_interrupted("hmi").unwrap().is_empty(),
            "row 1 once per edge"
        );
        let answer = reply(2, Some(RpcStatus::Ok), None, r#"{"force":12}"#);
        assert_eq!(
            ecu.receive("hmi", answer.clone(), 20).unwrap(),
            alloc::vec![Effect::Complete {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"force":12}"#.to_string(),
                source: "hmi".to_string(),
            }]
        );
        assert!(
            ecu.receive("hmi", answer, 30).unwrap().is_empty(),
            "the mailbox's copy of the same reply ends nothing"
        );
        assert!(!ecu.awaits_reply(&WIRE));
    }

    /// SCE_MESH.md section 22.3: the server started again with an empty
    /// mailbox, and the reply it held for the request is gone. The request is
    /// sent again as it was written; the responder's §mesh-10.5 window answers
    /// the copy with the reply its first copy produced, and that reply ends
    /// the request once. Red if a restart sends nothing again (the request
    /// then waits for a reply that no longer exists) or forgets the request
    /// (the cached reply then ends nothing).
    #[test]
    fn a_restarted_peer_is_asked_again_and_answers_from_its_reply_cache_once() {
        let mut ecu = requester(None);
        let mut hmi = responder();
        assert!(ecu.peer_ready("hmi", 0).unwrap().is_empty());
        let asked = started(&mut ecu, None);
        assert_eq!(hmi.receive("ecu", asked.clone(), 0).unwrap().len(), 1);
        let first = sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 0));

        // The reply went to the mailbox the restart emptied.
        let again = transmitted(ecu.peer_restarted("hmi", 40));
        assert_eq!(
            again, asked,
            "the request as it was written, under its own ids"
        );
        let cached = sent_to_ecu(hmi.receive("ecu", again, 50));
        assert_eq!(cached, first);
        assert_eq!(
            ecu.receive("hmi", cached.clone(), 60).unwrap(),
            alloc::vec![Effect::Complete {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"force":12}"#.to_string(),
                source: "hmi".to_string(),
            }]
        );
        assert!(ecu.receive("hmi", cached, 70).unwrap().is_empty());
        assert!(
            ecu.peer_restarted("hmi", 80).unwrap().is_empty(),
            "nothing waits to be asked again"
        );
    }

    /// SCE_MESH.md section 10.11: a request a checkpoint not yet durable
    /// holds has not left, and a restart of its peer does not send it ahead
    /// of that checkpoint; once released it is sent again as any is. Red if
    /// the restart ignores what the checkpoint holds.
    #[test]
    fn a_request_a_checkpoint_still_holds_is_not_sent_again_before_it_leaves() {
        let mut ecu = requester(None);
        ecu.enable_checkpoints();
        assert!(ecu.peer_ready("hmi", 0).unwrap().is_empty());
        match ecu.invoke(&ask("#hmi", None), WIRE, [1; 16], 10, 1000) {
            Ok(Invoked::Started(effects)) => assert!(effects.is_empty(), "{effects:?}"),
            other => panic!("expected the request to start, got {other:?}"),
        }
        assert!(ecu.peer_restarted("hmi", 20).unwrap().is_empty());
        ecu.seal(
            1,
            Clocks {
                monotonic_ms: 20,
                wall_ms: 2000,
            },
        )
        .unwrap();
        assert!(ecu.peer_restarted("hmi", 25).unwrap().is_empty());
        let asked = transmitted(ecu.durable(1, 30));
        assert_eq!(transmitted(ecu.peer_restarted("hmi", 40)), asked);
    }

    /// A restart or an interruption of a peer the host never bound is the
    /// host's error, as a lost link of one is.
    #[test]
    fn an_unbound_peer_is_neither_interrupted_nor_restarted() {
        let mut ecu = requester(None);
        let unknown = Err(RouterError::UnknownPeer("nobody".to_string()));
        assert_eq!(ecu.peer_interrupted("nobody"), unknown);
        assert_eq!(ecu.peer_restarted("nobody", 0), unknown);
    }

    /// `ecu` asking `hmi` over an ordered binding (§mesh-10.6): every
    /// envelope stamped, every arrival ordered, nothing held for readiness.
    fn ordered_requester() -> Router {
        let mut ecu = Router::new("ecu", 8, 50).unwrap();
        ecu.add_peer(
            "hmi",
            PeerConfig {
                buffer: None,
                ..ORDERED
            },
        );
        ecu
    }

    /// `hmi` answering `ecu` over the same ordered binding.
    fn ordered_responder() -> Router {
        let mut hmi = Router::new("hmi", 8, 50).unwrap();
        hmi.add_peer(
            "ecu",
            PeerConfig {
                buffer: None,
                responders: Responders::Named(&["ecu"]),
                reply_events: &["service.response.force"],
                ..ORDERED
            },
        );
        hmi
    }

    /// SCE_MESH.md section 10.6.4 after a restart: the copy of a request sent again carries
    /// its first sequence number, below what the responder now expects. The
    /// responder's duplicate window sees it before its ordering does
    /// (§mesh-10.5), so the copy is answered from the reply cache and never
    /// reaches the ordering buffer: it is neither a stale envelope dropped
    /// unanswered nor a gap that holds back the requester's next envelope,
    /// which is delivered at once. Red if ordering ran before the duplicate
    /// window: the copy would then be dropped as stale and never answered.
    #[test]
    fn a_request_sent_again_on_an_ordered_binding_is_answered_once_and_holds_nothing_back() {
        let mut ecu = ordered_requester();
        let mut hmi = ordered_responder();
        assert!(ecu.peer_ready("hmi", 0).unwrap().is_empty());
        let asked = started(&mut ecu, None);
        let stamped = Envelope::decode(&mut SceCursor::new(&asked))
            .unwrap()
            .sequence_no;
        assert!(stamped.is_some(), "an ordered binding stamps the request");
        assert_eq!(
            events(&hmi.receive("ecu", asked.clone(), 0).unwrap()),
            ["service.request.force"]
        );
        let first = sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 0));

        // The reply went to the mailbox the restart emptied.
        let again = transmitted(ecu.peer_restarted("hmi", 10));
        assert_eq!(
            Envelope::decode(&mut SceCursor::new(&again))
                .unwrap()
                .sequence_no,
            stamped,
            "the copy keeps its first sequence number"
        );
        assert_eq!(sent_to_ecu(hmi.receive("ecu", again, 20)), first);

        let next = transmitted(ecu.send(&request("#hmi", "status.changed", ""), [2; 16], 30));
        assert_eq!(
            events(&hmi.receive("ecu", next, 30).unwrap()),
            ["status.changed"],
            "the next envelope is not held behind the copy"
        );
        assert_eq!(
            ecu.receive("hmi", first.clone(), 40).unwrap(),
            alloc::vec![Effect::Complete {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"force":12}"#.to_string(),
                source: "hmi".to_string(),
            }]
        );
        assert!(ecu.receive("hmi", first, 50).unwrap().is_empty());
    }

    /// The same copy reaching a responder that started again with no state
    /// for `ecu` (no checkpoint restored its window): it is the first envelope
    /// that responder hears from `ecu`, so it anchors the ordering there
    /// (SCE_MESH.md section 10.6.4) and is taken as new, and the requester's next envelope
    /// follows on from it without waiting.
    #[test]
    fn a_request_sent_again_to_a_responder_without_state_anchors_its_ordering() {
        let mut ecu = ordered_requester();
        assert!(ecu.peer_ready("hmi", 0).unwrap().is_empty());
        started(&mut ecu, None);
        let again = transmitted(ecu.peer_restarted("hmi", 10));

        let mut fresh = ordered_responder();
        assert_eq!(
            events(&fresh.receive("ecu", again, 20).unwrap()),
            ["service.request.force"]
        );
        let next = transmitted(ecu.send(&request("#hmi", "status.changed", ""), [2; 16], 30));
        assert_eq!(
            events(&fresh.receive("ecu", next, 30).unwrap()),
            ["status.changed"]
        );
        let reply =
            sent_to_ecu(fresh.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 40));
        assert!(matches!(
            ecu.receive("hmi", reply, 50).unwrap().as_slice(),
            [Effect::Complete { invoke_id, .. }] if invoke_id == "ask"
        ));
    }

    /// The sequence number each of the envelopes `sent` carries, in order.
    fn numbers(sent: &[&[u8]]) -> Vec<Option<u64>> {
        sent.iter()
            .map(|bytes| {
                Envelope::decode(&mut SceCursor::new(bytes))
                    .unwrap()
                    .sequence_no
            })
            .collect()
    }

    /// SCE_MESH.md section 10.6.4: a reply is not in its sender's sequence on
    /// an ordered binding, so it is written with no `sequence_no` and the
    /// counter does not move: the sends around it are numbered 1 and 2. Red if
    /// the send half stamps every envelope: the reply takes 2 and the next
    /// send 3, a number the receiver waits for and never gets.
    #[test]
    fn a_reply_on_an_ordered_binding_takes_no_number() {
        let mut hmi = ordered_responder();
        let before = sent_to_ecu(hmi.send(&request("#ecu", "status.changed", ""), [2; 16], 0));
        let reply = sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 0));
        let after = sent_to_ecu(hmi.send(&request("#ecu", "status.changed", ""), [3; 16], 0));
        assert_eq!(
            Envelope::decode(&mut SceCursor::new(&reply))
                .unwrap()
                .pattern,
            PatternKind::RpcReply
        );
        assert_eq!(
            numbers(&[&before, &reply, &after]),
            alloc::vec![Some(1), None, Some(2)]
        );
    }

    /// SCE_MESH.md section 10.6.4: a reply with no `sequence_no`, as the C++
    /// router's server answers, completes its request on an ordered binding.
    /// Red if the receive half asks a reply for a number: it is then refused
    /// as `missingSequence` (§mesh-16.7 row 11) and the request waits for
    /// ever.
    #[test]
    fn an_unnumbered_reply_completes_its_request_on_an_ordered_binding() {
        let mut ecu = ordered_requester();
        started(&mut ecu, None);
        assert_eq!(
            ecu.receive("hmi", reply(5, None, None, r#"{"force":12}"#), 20)
                .unwrap(),
            alloc::vec![Effect::Complete {
                invoke_id: "ask".to_string(),
                token: 7,
                data: r#"{"force":12}"#.to_string(),
                source: "hmi".to_string(),
            }]
        );
    }

    /// The whole ordered round trip: a reply sent between two sends leaves no
    /// gap in the stream, so the send after it is delivered at once and no
    /// `orderingGap` (§mesh-16.7 row 12) is raised once the gap timeout has
    /// passed. Red if the reply took a number: the send after it then waits
    /// for that number and ends in a timed-out gap.
    #[test]
    fn a_reply_between_two_sends_leaves_no_gap() {
        let mut ecu = ordered_requester();
        let mut hmi = ordered_responder();
        let asked = started(&mut ecu, None);
        assert_eq!(
            events(&hmi.receive("ecu", asked, 0).unwrap()),
            ["service.request.force"]
        );
        let first = sent_to_ecu(hmi.send(&request("#ecu", "status.changed", ""), [2; 16], 10));
        let answered =
            sent_to_ecu(hmi.send(&answer("service.response.force", WIRE_TEXT), [5; 16], 10));
        let second = sent_to_ecu(hmi.send(&request("#ecu", "status.done", ""), [3; 16], 10));

        assert_eq!(
            events(&ecu.receive("hmi", first, 20).unwrap()),
            ["status.changed"]
        );
        assert!(matches!(
            ecu.receive("hmi", answered, 20).unwrap().as_slice(),
            [Effect::Complete { invoke_id, .. }] if invoke_id == "ask"
        ));
        assert_eq!(
            events(&ecu.receive("hmi", second, 20).unwrap()),
            ["status.done"],
            "the send after the reply waits for no number"
        );
        let later = ecu.tick(10_000).unwrap();
        assert!(later.is_empty(), "no gap times out: {later:?}");
    }

    /// SCE_MESH.md section 10.11: on a binding with durable receipt the
    /// number is also what the receiver acknowledges, so a reply is numbered
    /// there like every envelope. Red if the exemption of section 10.6.4
    /// reached a durable binding: the reply would leave with no number, and could be
    /// neither acknowledged nor sent again.
    #[test]
    fn a_reply_on_a_durable_binding_is_numbered() {
        let mut hmi = ordered_responder();
        hmi.enable_checkpoints();
        hmi.send_durably("ecu", DurableSend::default()).unwrap();
        assert!(hmi.peer_ready("ecu", 0).unwrap().is_empty());
        assert!(hmi
            .send(&answer("service.response.force", WIRE_TEXT), [5; 16], 0)
            .unwrap()
            .is_empty());
        hmi.seal(
            1,
            Clocks {
                monotonic_ms: 0,
                wall_ms: 1000,
            },
        )
        .unwrap();
        let reply = sent_to_ecu(hmi.durable(1, 10));
        assert_eq!(numbers(&[&reply]), alloc::vec![Some(1)]);
    }

    #[test]
    fn uuid_text_reads_back_what_it_wrote_in_either_case_and_nothing_else() {
        assert_eq!(rpc::parse_uuid_text(&rpc::uuid_text(&WIRE)), Some(WIRE));
        assert_eq!(
            rpc::parse_uuid_text(&rpc::uuid_text(&WIRE).to_uppercase()),
            Some(WIRE)
        );
        assert_eq!(rpc::parse_uuid_text(""), None);
        assert_eq!(rpc::parse_uuid_text("ask"), None);
        // The 32 digits without the dashes are not the canonical text.
        assert_eq!(rpc::parse_uuid_text(&"09".repeat(16)), None);
        assert_eq!(
            rpc::parse_uuid_text("zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz"),
            None
        );
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

    // ── The causal route (docs/adr/0085, sections 3.2 and 9.2) ─────────

    /// A causal group of three, every copy one machine `list` on its own
    /// device; the binding `#people` is the group's route. Unbuffered, so a
    /// release is transmitted at once.
    const PEOPLE: PeerConfig = PeerConfig {
        transport: "wss",
        buffer: None,
        retry: None,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: false,
            ordered: false,
        },
        responders: Responders::Named(&["people"]),
        deadline_ms: None,
        reply_events: &[],
        causal: Some(CausalGroup {
            name: "people",
            epoch: 0,
            view: &[
                "phone/phone_list",
                "laptop/laptop_list",
                "tablet/tablet_list",
            ],
        }),
    };

    /// The copy of the list whose replica is `[n; 16]`, started.
    fn copy(n: u8) -> Router {
        let mut router = Router::new("list", 8, 50).unwrap();
        router.add_peer("people", PEOPLE);
        router
            .start_causal("people", Start::NewIncarnation(Replica([n; 16])), 64, 100)
            .expect("a causal peer starts");
        router
    }

    /// The single dot an offer was accepted with.
    fn accepted(effects: Result<Vec<Effect>, RouterError>) -> Dot {
        match effects.unwrap().as_slice() {
            [Effect::Accepted { peer, dot }] if peer == "people" => *dot,
            other => panic!("expected one accepted change, got {other:?}"),
        }
    }

    fn sent(effects: Result<Vec<Effect>, RouterError>) -> Vec<Vec<u8>> {
        effects
            .unwrap()
            .into_iter()
            .map(|effect| match effect {
                Effect::Transmit { peer, bytes } if peer == "people" => bytes,
                other => panic!("expected only transmissions, got {other:?}"),
            })
            .collect()
    }

    /// Write-ahead. Red if the router transmits a causal change when the
    /// layer tags it: the send then leaves before the journal commit.
    #[test]
    fn a_causal_send_leaves_only_once_its_journal_record_is_committed() {
        let mut phone = copy(1);
        let mut laptop = copy(2);
        let dot = accepted(phone.send(&request("#people", "edit", r#"{"n":1}"#), [9; 16], 0));
        assert_eq!(dot.seq, 1);
        let released = sent(phone.committed("people", dot, 0));
        assert_eq!(released.len(), 1);
        assert_eq!(
            events(&laptop.receive("people", released[0].clone(), 0).unwrap()),
            ["edit"]
        );
        // The same change again is dropped by its dot, not delivered twice.
        assert!(laptop
            .receive("people", released[0].clone(), 0)
            .unwrap()
            .is_empty());
    }

    /// A causal route is a group, not a link to one machine: a member's
    /// change reaches a copy through any member or relay as its author wrote
    /// it (§mesh-10.12), so the binding `people` names no sender, and the
    /// change of `phone_list` is delivered under its author's name.
    /// Red if the router holds a causal route to its link's name: the change
    /// is then refused as a source the link does not name.
    #[test]
    fn a_causal_route_delivers_a_change_under_its_authors_name_from_any_link() {
        assert!(!PEOPLE.link_names_sender());
        assert!(UNORDERED.link_names_sender());
        let mut phone = Router::new("phone_list", 8, 50).unwrap();
        phone.add_peer("people", PEOPLE);
        phone
            .start_causal("people", Start::NewIncarnation(Replica([1; 16])), 64, 100)
            .expect("a causal peer starts");
        let mut laptop = copy(2);
        let dot = accepted(phone.send(&request("#people", "edit", r#"{"n":1}"#), [9; 16], 0));
        let released = sent(phone.committed("people", dot, 0));
        let delivered = laptop.receive("people", released[0].clone(), 0).unwrap();
        assert!(
            matches!(
                delivered.as_slice(),
                [Effect::Deliver { event, source, .. }] if event == "edit" && source == "phone_list"
            ),
            "{delivered:?}"
        );
    }

    /// A causal peer the host has not started is the host's error, and so is
    /// starting a peer that is not causal.
    #[test]
    fn a_causal_peer_is_used_only_once_started() {
        let mut router = Router::new("list", 8, 50).unwrap();
        router.add_peer("people", PEOPLE);
        router.add_peer("hmi", UNORDERED);
        assert_eq!(
            router.send(&request("#people", "edit", ""), [1; 16], 0),
            Err(RouterError::CausalNotStarted("people".to_string()))
        );
        assert_eq!(
            router.start_causal("hmi", Start::NewIncarnation(Replica([1; 16])), 8, 100),
            Err(RouterError::NotCausal("hmi".to_string()))
        );
    }

    /// Durable receipt is a promise only a router that checkpoints can keep,
    /// on a binding that is not a causal route, with a window and a resend
    /// period of at least 1; an acknowledgement is owed only on a durable
    /// stream; and a durable report names a checkpoint that was sealed.
    #[test]
    fn durable_receipt_is_refused_where_it_cannot_be_kept() {
        let send = DurableSend::default();
        assert_eq!(
            send,
            DurableSend {
                window: 256,
                retry_ms: 1000
            }
        );
        let mut router = Router::new("list", 8, 50).unwrap();
        router.add_peer("people", PEOPLE);
        router.add_peer("hmi", UNORDERED);
        assert_eq!(
            router.send_durably("hmi", send),
            Err(RouterError::NotCheckpointed)
        );
        assert_eq!(
            router.receive_durably("hmi"),
            Err(RouterError::NotCheckpointed)
        );
        assert_eq!(router.durable(1, 0), Err(RouterError::NotCheckpointed));

        router.enable_checkpoints();
        assert_eq!(
            router.send_durably("nobody", send),
            Err(RouterError::UnknownPeer("nobody".to_string()))
        );
        assert_eq!(
            router.receive_durably("people"),
            Err(RouterError::DurableOnCausal("people".to_string()))
        );
        for (window, retry_ms) in [(0, 100), (1, 0)] {
            assert_eq!(
                router.send_durably("hmi", DurableSend { window, retry_ms }),
                Err(RouterError::BadReceipt {
                    peer: "hmi".to_string(),
                    window,
                    retry_ms
                })
            );
        }
        assert_eq!(
            router.acknowledge("hmi", [1; 16]),
            Err(RouterError::NotDurable("hmi".to_string()))
        );
        assert_eq!(
            router.durable(1, 0),
            Err(RouterError::NotSealed {
                sequence: 1,
                sealed: 0
            })
        );
        assert!(!router.holds_until_acknowledged("hmi"));
        router.send_durably("hmi", send).unwrap();
        assert!(router.holds_until_acknowledged("hmi"));
    }

    /// Repair, on a link that comes back. Red if a peer that becomes ready
    /// asks for no vector: the lost change then waits for the period.
    #[test]
    fn a_link_that_comes_back_sends_a_vector_and_repairs_what_was_lost() {
        let mut phone = copy(1);
        let mut laptop = copy(2);
        let dot = accepted(phone.send(&request("#people", "edit", ""), [9; 16], 0));
        let _lost = sent(phone.committed("people", dot, 0));
        let effects = laptop.peer_ready("people", 10).unwrap();
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::AnnounceDue { peer } if peer == "people")),
            "{effects:?}"
        );
        let vector = sent(laptop.announce("people", [3; 16], 10));
        assert_eq!(vector.len(), 1);
        let answer = sent(phone.receive("people", vector[0].clone(), 10));
        assert_eq!(answer.len(), 1);
        assert_eq!(
            events(&laptop.receive("people", answer[0].clone(), 10).unwrap()),
            ["edit"]
        );
    }

    /// The period. Red if the tick never asks for a vector: a member that
    /// lost a change and whose links never drop is then never repaired.
    #[test]
    fn a_causal_peer_is_owed_a_vector_on_its_period() {
        let mut phone = copy(1);
        let due = |effects: Vec<Effect>| {
            effects
                .iter()
                .any(|e| matches!(e, Effect::AnnounceDue { peer } if peer == "people"))
        };
        assert!(due(phone.tick(0).unwrap()));
        let _ = phone.announce("people", [3; 16], 0).unwrap();
        assert!(!due(phone.tick(99).unwrap()));
        assert!(due(phone.tick(100).unwrap()));
    }

    /// When a host that owns time wakes for a vector: never while the layer
    /// has no work, and a period after its last vector once its log keeps a
    /// change not yet stable. Red if a layer with an empty log asks to be
    /// woken (a simulated world then never comes to rest), or if one whose
    /// log keeps a change does not (a loss no link edge reports is then never
    /// repaired while nothing else happens).
    #[test]
    fn a_layer_with_work_is_woken_for_its_vector_and_one_without_is_not() {
        let mut phone = copy(1);
        assert_eq!(phone.next_announce_due_ms(), None);
        let _ = phone.announce("people", [3; 16], 40).unwrap();
        let dot = accepted(phone.send(&request("#people", "edit", ""), [9; 16], 50));
        let _ = sent(phone.committed("people", dot, 50));
        assert_eq!(phone.next_announce_due_ms(), Some(140));
    }

    /// The view. Red if the epoch check is removed: a change tagged with
    /// another epoch is then delivered rather than raised as row 18.
    #[test]
    fn a_change_of_another_view_is_raised_and_not_delivered() {
        let mut laptop = copy(2);
        let envelope = Envelope {
            id: (&[5u8; 16]).into(),
            source: within("list").unwrap(),
            event_type: within("edit").unwrap(),
            pattern: PatternKind::FireForget,
            datacontenttype: PayloadCodec::None,
            ..Envelope::new()
        };
        let tag = crate::causal::Tag {
            epoch: 1,
            vector: alloc::vec![Replica([1; 16]).entry(1)],
        };
        let bytes = stamp(envelope, &tag).unwrap().encode_to_vec();
        let effects = laptop.receive("people", bytes, 0).unwrap();
        assert_eq!(
            effects,
            alloc::vec![Effect::Raise {
                peer: Some("people".to_string()),
                signal: Signal::CausalViewMismatch {
                    source: "list".to_string(),
                    epoch: 1,
                    replica: rpc::uuid_text(&[1; 16]),
                },
            }]
        );
    }

    /// A small deterministic generator for the world below.
    struct Rng(u64);

    impl Rng {
        fn step(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.step() % n as u64) as usize
        }

        fn chance(&mut self, percent: u64) -> bool {
            self.step() % 100 < percent
        }
    }

    /// Three copies of the list, the bytes in flight to each, and what each
    /// has taken in: per author, how many of its changes.
    struct World {
        copies: Vec<Router>,
        wire: Vec<(usize, Vec<u8>)>,
        seen: [[u32; 3]; 3],
        rng: Rng,
        now: i64,
        lossy: bool,
    }

    impl World {
        /// Act on what copy `at` was told to do, as its host would.
        fn act(&mut self, at: usize, effects: Result<Vec<Effect>, RouterError>) {
            for effect in effects.expect("the router accepts what the world gives it") {
                match effect {
                    Effect::Transmit { bytes, .. } => {
                        for to in (0..3).filter(|&to| to != at) {
                            if self.lossy && self.rng.chance(30) {
                                continue;
                            }
                            if self.lossy && self.rng.chance(10) {
                                self.wire.push((to, bytes.clone()));
                            }
                            self.wire.push((to, bytes.clone()));
                        }
                    }
                    Effect::Accepted { peer, dot } => {
                        let released = self.copies[at].committed(&peer, dot, self.now);
                        self.act(at, released);
                    }
                    Effect::AnnounceDue { peer } => {
                        let id = [self.rng.step() as u8; 16];
                        let announced = self.copies[at].announce(&peer, id, self.now);
                        self.act(at, announced);
                    }
                    Effect::Deliver { data, .. } => self.take(at, &data),
                    other => panic!("copy {at}: unexpected {other:?}"),
                }
            }
        }

        /// Copy `at` takes in the change `data` names: `<author> <n> <seen
        /// of author 0> <of 1> <of 2>`, what its author had taken in when it
        /// made it. It must be the author's next change, and everything its
        /// author had taken in must be taken in here already.
        fn take(&mut self, at: usize, data: &str) {
            let fields: Vec<u32> = data
                .split(' ')
                .map(|field| field.parse().expect("a number"))
                .collect();
            let author = fields[0] as usize;
            assert_eq!(
                fields[1],
                self.seen[at][author] + 1,
                "copy {at} takes in {data} once and skips no gap"
            );
            for other in (0..3).filter(|&other| other != author) {
                assert!(
                    self.seen[at][other] >= fields[2 + other],
                    "copy {at} takes in {data} before a change it depends on"
                );
            }
            self.seen[at][author] = fields[1];
        }

        /// Copy `at` makes its next change.
        fn make(&mut self, at: usize) {
            let n = self.seen[at][at] + 1;
            let [a, b, c] = self.seen[at];
            let data = alloc::format!("{at} {n} {a} {b} {c}");
            self.seen[at][at] = n;
            let id = [self.rng.step() as u8; 16];
            let effects = self.copies[at].send(&request("#people", "edit", &data), id, self.now);
            self.act(at, effects);
        }

        fn deliver_one(&mut self) {
            let index = self.rng.below(self.wire.len());
            let (to, bytes) = self.wire.remove(index);
            let effects = self.copies[to].receive("people", bytes, self.now);
            self.act(to, effects);
        }
    }

    /// The world. Three copies on links that lose, duplicate and reorder take
    /// in every change exactly once, in an order that is a linear extension
    /// of what each change's author had taken in, with no gap skipped, and
    /// their logs drop to the stable cut once the links are quiet. Red if the
    /// hold branch delivers, or if repair is removed (a lost change then
    /// never arrives).
    #[test]
    fn three_copies_through_their_routers_take_in_every_change_once_in_causal_order() {
        const CHANGES_EACH: u32 = 15;
        for seed in [0x0BAD_5EED_u64, 0x0CA5_CADE, 0x00C0_FFEE] {
            let mut world = World {
                copies: (1..=3u8).map(copy).collect(),
                wire: Vec::new(),
                seen: [[0; 3]; 3],
                rng: Rng(seed),
                now: 0,
                lossy: true,
            };
            while world
                .seen
                .iter()
                .enumerate()
                .any(|(at, s)| s[at] < CHANGES_EACH)
                || !world.wire.is_empty()
            {
                world.now += 1;
                match world.rng.below(10) {
                    0..=2 => {
                        let at = world.rng.below(3);
                        if world.seen[at][at] < CHANGES_EACH {
                            world.make(at);
                        }
                    }
                    3 => {
                        let at = world.rng.below(3);
                        let now = world.now;
                        let ticked = world.copies[at].tick(now);
                        world.act(at, ticked);
                    }
                    _ => {
                        if !world.wire.is_empty() {
                            world.deliver_one();
                        }
                    }
                }
            }
            world.lossy = false;
            for _ in 0..20 {
                world.now += 1;
                for at in [0, 1, 2] {
                    let id = [world.rng.step() as u8; 16];
                    let now = world.now;
                    let announced = world.copies[at].announce("people", id, now);
                    world.act(at, announced);
                }
                while !world.wire.is_empty() {
                    world.deliver_one();
                }
            }
            for (at, (seen, router)) in world.seen.iter().zip(&world.copies).enumerate() {
                assert_eq!(
                    *seen, [CHANGES_EACH; 3],
                    "seed {seed:#x}: copy {at} takes in every change"
                );
                let layer = router.causal_layer("people").unwrap();
                assert_eq!(layer.held_len(), 0, "seed {seed:#x}: nothing held");
                assert_eq!(layer.log_len(), 0, "seed {seed:#x}: the log is at the cut");
            }
        }
    }
}
