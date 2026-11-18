// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A machine served as a pool of sessions, one per key (docs/adr/0107,
//! decision D3; SCE_MESH.md section 14.4, "Keyed session pool").
//!
//! deploy.yaml's `sessions:` says where the key of an envelope is read: a
//! field of its data, or the name of the client whose link it arrived on.
//! Every key names a session of the machine of its own: a session tree with
//! its own engine, its own Mesh [`Endpoint`] and its own inbox (docs/adr/0084).
//! [`Pool::receive`] places an envelope in the inbox of the session its key
//! names, and [`Pool::run`] runs that session: it takes what its inbox holds
//! one input at a time, in the order it arrived, and runs each to completion
//! before it takes the next. The requests of one key are therefore serialised
//! by the machine's own run to completion, so a rule one key owns (how many
//! members it admits) is a rule of the document and needs no lock; the
//! sessions of different keys run on as many threads as the host runs them on.
//!
//! A session is in memory while it is used. The first input for a key takes
//! the key's lease in the host's [`SessionStore`], then loads the session's
//! last durable checkpoint ([`checkpoint::load`]) and restores its machine
//! from the checkpoint's saved state and its endpoint from the checkpoint's
//! section ([`Endpoint::restore`]), or, when the store holds none, starts the
//! machine from its initial datamodel. A session is kept as its
//! [`Checkpoint`] (SCE_MESH.md section 10.11): its machine's saved state and
//! its endpoint's (the duplicate window, the replies a duplicate request is
//! answered with, what waits for a link) in one write. After every input that
//! took an envelope in or ran a macrostep the pool takes one in the three
//! steps of section 10.11: the machine's saved state and the endpoint sealed
//! with the next number ([`Endpoint::checkpoint`]), the write through a
//! [`StoreSink`] carrying the version it was read at and the lease's fence,
//! so a session another process took over is never written by this one, and
//! the endpoint told the checkpoint is durable ([`Endpoint::durable`]), which
//! releases what the input made: nothing a session sends leaves before the
//! checkpoint that holds it is durable. A session that had no event for
//! `idle_ms` and holds nothing pending is saved as a clean checkpoint,
//! dropped from memory, and its lease released ([`Pool::evict_idle`]), so its
//! restore tells no peer it may have lost an envelope. One whose earliest wait
//! comes due while it is out of memory is loaded again ([`Pool::due`]), and its
//! restore arms what came due.
//!
//! A process that loses a key's lease (another process took it over, or a
//! renewal was refused) stops serving the key: the session is dropped without
//! being saved, and every request for it that waits, or that arrives while
//! another process holds the key, is answered with `RpcStatus::Unavailable`,
//! which tells the client to try again.
//!
//! The pool decides nothing a document can see that a machine not pooled
//! would not see: what reaches a session goes through its own [`Endpoint`],
//! with the duplicate window, the reply matching and the `_event` fields every
//! machine has, and a reply carries the machine's name as its source and goes
//! back on the link its request came in on.
//!
//! Time is the host's, as for an [`Endpoint`]: the pool reads the wall clock
//! of its [`Environment`], and the host calls [`Pool::due`],
//! [`Pool::renew_leases`], [`Pool::evict_idle`] and [`Pool::tick`] on its own
//! schedule, sleeping until [`Pool::next_wake_ms`] in between.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};

use sce_forge_runtime::codec::SceCursor;
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_session_store::{
    AcquireError, Failure, Held, LeaseError, SessionStore, WriteError, LATEST_MOMENT,
};

use crate::checkpoint::{self, Checkpoint, CheckpointSink, LoadError, StoreSink};
use crate::endpoint::{
    Endpoint, EngineCall, Environment, SharedEndpoint, Transport, TransportFailure,
};
use crate::generated::envelope::Envelope;
use crate::generated::pattern_kind::PatternKind;
use crate::generated::payload_codec::PayloadCodec;
use crate::generated::rpc_status::RpcStatus;
use crate::inbound::ConfigError;
use crate::peers::{KeyFrom, Machine, Sessions};
use crate::router::{payload_text, within, RouterError};

/// The most saved sessions one call of [`Pool::due`] loads, and one call of
/// [`Pool::next_wake_ms`] reads, from the store's index of what comes due.
const WAKE_BATCH: usize = 256;

/// One session's machine, as the host built it from its generated machine.
///
/// The host implements it for its machine in a few lines: [`Self::run`] hands
/// the calls to the engine (`crate::endpoint::apply_to`) and ticks it, and
/// [`Self::save`] is the generated `save_at`.
pub trait SessionMachine: Send {
    /// Tell the machine `calls`, in order, and run it until it waits: every
    /// macrostep they cause and every delayed send due at `wall_now_ms`, in
    /// milliseconds since the Unix epoch. Answers whether a macrostep ran.
    fn run(&mut self, calls: Vec<EngineCall>, wall_now_ms: u64) -> bool;

    /// The machine's whole state at a macrostep boundary, its tree of sessions
    /// included, at `wall_now_ms`: what the session's [`Checkpoint`] holds of
    /// the machine. Refused, with the reason, for a machine that cannot be
    /// saved now.
    fn save(&self, wall_now_ms: u64) -> Result<SavedState, String>;

    /// Whether the machine reached a top-level final state: its session is
    /// over, and the next input for its key starts a new one.
    fn finished(&self) -> bool;
}

/// The endpoint a session's machine is registered with
/// (`crate::endpoint::register`): its own, on the pool's transport.
pub type SessionEndpoint<T, E> = SharedEndpoint<SessionTransport<T>, E>;

/// How the host builds the machine of a session.
pub trait SessionFactory<T, E>: Send + Sync {
    /// The machine of one session.
    type Machine: SessionMachine;

    /// A machine for `key` started from the document's initial datamodel at
    /// `wall_now_ms`, its Mesh sends and requests registered with `endpoint`.
    fn start(
        &self,
        key: &str,
        endpoint: &SessionEndpoint<T, E>,
        wall_now_ms: u64,
    ) -> Result<Self::Machine, String>;

    /// A machine for `key` standing where `state`, the saved state of the
    /// session's last durable [`Checkpoint`], left one, restored at
    /// `wall_now_ms` (so a send that came due while it was out of memory comes
    /// due now), its Mesh sends and requests registered with `endpoint`. The
    /// pool restores the endpoint from the same checkpoint once this answers.
    fn restore(
        &self,
        key: &str,
        state: &SavedState,
        endpoint: &SessionEndpoint<T, E>,
        wall_now_ms: u64,
    ) -> Result<Self::Machine, String>;
}

/// How a pool holds its keys' leases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolConfig {
    /// This process's name as the holder of a lease: one that no other
    /// process serving the same store uses.
    pub owner: String,
    /// How long a lease lasts unless it is renewed, in milliseconds.
    pub lease_ttl_ms: u64,
    /// How long after its last grant or renewal a lease is renewed: above
    /// zero and below [`Self::lease_ttl_ms`], so that a lease is renewed
    /// before it runs out.
    pub renew_every_ms: u64,
}

/// Why a pool could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolConfigError {
    /// The machine's deployment declares no `sessions:`.
    NotPooled { machine: String },
    /// The renewal period is zero, or not below the lease's length.
    Renewal {
        renew_every_ms: u64,
        lease_ttl_ms: u64,
    },
    /// A session's router could not be built from the machine.
    Router(ConfigError),
}

/// What went wrong that only the host can act on. None of it reaches a
/// document; a request it concerns is answered with a failure status.
#[derive(Debug)]
pub enum PoolError {
    /// Bytes from `peer` that are not an envelope, so no key can be read
    /// from them.
    Malformed { peer: String },
    /// A reply from `peer` to no request a session of the pool waits on: a
    /// late one, whose request was answered, cancelled or expired.
    UnroutedReply { peer: String },
    /// An envelope from `peer` that is not a request and names no session.
    Unkeyed {
        peer: String,
        event: String,
        reason: String,
    },
    /// An envelope for `key` that is not a request, dropped because this
    /// process does not serve the key now.
    Dropped {
        key: String,
        event: String,
        reason: String,
    },
    /// Another process holds the lease on `key`.
    LeaseHeld { key: String, held: Held },
    /// This process lost the lease on `key`, and stopped serving it.
    LeaseLost { key: String, reason: String },
    /// The store failed; `key` is the session it concerned, if one.
    Store {
        key: Option<String>,
        failure: Failure,
    },
    /// The host could not build the machine of `key`.
    NotBuilt { key: String, reason: String },
    /// The machine of `key` could not be saved; it stays in memory and is
    /// saved at its next chance.
    NotSaved { key: String, reason: String },
    /// A failure reply to `peer` could not be sent.
    NotAnswered { peer: String, reason: String },
    /// What the endpoint of the session `key` reported to its host.
    Endpoint { key: String, error: RouterError },
}

/// The transport of a session's endpoint: the pool's own, through which every
/// session sends, which notes the request each session sends so that its
/// reply reaches the session that waits for it.
pub struct SessionTransport<T> {
    key: String,
    inner: Arc<Mutex<T>>,
    replies: Replies,
}

impl<T: Transport> Transport for SessionTransport<T> {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if let Some(wire_id) = request_id(bytes) {
            lock(&self.replies).insert(wire_id, self.key.clone());
        }
        lock(&self.inner).transmit(peer, bytes)
    }
}

/// The session each request a session sent waits in, by the request's wire
/// `invoke_id`.
type Replies = Arc<Mutex<BTreeMap<[u8; 16], String>>>;

/// Every session the pool holds, in memory or about to be, by key.
type Table<M, T, E> = BTreeMap<String, Arc<Slot<M, T, E>>>;

/// One input of a session's inbox, applied by the session's owner-thread
/// operation of the same name.
enum Input {
    Envelope {
        peer: String,
        bytes: Vec<u8>,
    },
    Ready(String),
    NotReady(String),
    Tick,
    /// Load the session: one of its waits came due while it was out of memory.
    Wake,
}

/// A session's inbox. Closed once the session left the table: nothing is
/// posted to it after that, since posts and removals both hold the table.
#[derive(Default)]
struct Inbox {
    queue: VecDeque<Input>,
    closed: bool,
}

/// The lease a process holds on a session's key.
enum LeaseState {
    Unheld,
    Held {
        fence: u64,
        renewed_ms: u64,
    },
    /// Taken over, or not renewed: the session is dropped at its next run.
    Lost,
}

/// One key's session: its inbox, which any thread posts to, its machine,
/// which the thread that runs it owns for as long as it runs it, and its
/// lease, which the thread that renews leases reads too.
struct Slot<M, T, E> {
    key: String,
    inbox: Mutex<Inbox>,
    live: Mutex<Option<Live<M, T, E>>>,
    lease: Mutex<LeaseState>,
}

impl<M, T, E> Slot<M, T, E> {
    fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            inbox: Mutex::new(Inbox::default()),
            live: Mutex::new(None),
            lease: Mutex::new(LeaseState::Unheld),
        }
    }
}

/// A session in memory.
struct Live<M, T, E> {
    machine: M,
    endpoint: SessionEndpoint<T, E>,
    /// The version of the session the store holds, `0` for none yet.
    version: u64,
    /// The number of the last checkpoint the endpoint was sealed or restored
    /// with, `0` before the first: the next is sealed with one above it.
    sequence: u64,
    /// The earliest wait of the state last saved.
    earliest_due: Option<u64>,
    /// When it last had an event: an envelope, or a macrostep a wait ran.
    last_active_ms: u64,
    /// Whether a macrostep ran since the state was last saved.
    dirty: bool,
}

/// What the pool answers a request with when it cannot serve its session.
struct Refusal {
    status: RpcStatus,
    reason: String,
}

impl Refusal {
    fn unavailable(reason: String) -> Self {
        Self {
            status: RpcStatus::Unavailable,
            reason,
        }
    }
}

/// How a run of a session ended.
enum Served {
    /// Its inbox is empty and its machine waits.
    Waiting,
    /// Its machine reached a top-level final state.
    Finished,
    /// Its lease is no longer this process's, for this reason.
    Lost(String),
}

/// Which checkpoint of a session the pool takes (SCE_MESH.md section 10.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Take {
    /// One when an input changed the session since its last checkpoint.
    IfChanged,
    /// A clean one, whatever changed: the session is dropped from memory once
    /// it is durable, so its endpoint takes nothing in after it.
    Clean,
}

/// A request an envelope carries, as much of it as a reply needs.
struct Request {
    invoke_id: [u8; 16],
    event: String,
}

/// Why an envelope names no session.
enum Unplaced {
    /// A reply to no request a session waits on.
    Reply,
    /// No key could be read from it.
    Unkeyed {
        event: String,
        request: Option<Request>,
        reason: String,
    },
}

/// A machine served as a pool of sessions, one per key. See the module
/// documentation for what it does and when the host calls it.
pub struct Pool<F, T, E>
where
    F: SessionFactory<T, E>,
{
    machine: Machine,
    sessions: Sessions,
    config: PoolConfig,
    factory: F,
    store: Arc<dyn SessionStore>,
    transport: Arc<Mutex<T>>,
    environment: Mutex<E>,
    table: Mutex<Table<F::Machine, T, E>>,
    /// The peers whose links are ready, which a session's endpoint is told
    /// when it is built.
    ready: Mutex<BTreeSet<String>>,
    replies: Replies,
    /// The keys another process held when this one asked, until when.
    deferred: Mutex<BTreeMap<String, u64>>,
    errors: Mutex<Vec<PoolError>>,
}

impl<F, T, E> Pool<F, T, E>
where
    F: SessionFactory<T, E>,
    T: Transport + 'static,
    E: Environment + Clone + 'static,
{
    /// A pool of the sessions of `machine`, whose deployment declares
    /// `sessions:`, built by `factory`, kept in `store`, sending through
    /// `transport` and reading the clock and drawing ids from `environment`.
    /// Each session's checkpoint is written to `store` under the key's lease.
    pub fn new(
        machine: Machine,
        config: PoolConfig,
        factory: F,
        store: Arc<dyn SessionStore>,
        transport: T,
        environment: E,
    ) -> Result<Self, PoolConfigError> {
        let Some(sessions) = machine.sessions else {
            return Err(PoolConfigError::NotPooled {
                machine: machine.name.to_string(),
            });
        };
        if config.renew_every_ms == 0 || config.renew_every_ms >= config.lease_ttl_ms {
            return Err(PoolConfigError::Renewal {
                renew_every_ms: config.renew_every_ms,
                lease_ttl_ms: config.lease_ttl_ms,
            });
        }
        machine.router().map_err(PoolConfigError::Router)?;
        Ok(Self {
            machine,
            sessions,
            config,
            factory,
            store,
            transport: Arc::new(Mutex::new(transport)),
            environment: Mutex::new(environment),
            table: Mutex::new(BTreeMap::new()),
            ready: Mutex::new(BTreeSet::new()),
            replies: Arc::new(Mutex::new(BTreeMap::new())),
            deferred: Mutex::new(BTreeMap::new()),
            errors: Mutex::new(Vec::new()),
        })
    }

    /// The machine the pool serves.
    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    /// Whether the session `key` is in memory. Waits for a run of it that is
    /// going on.
    pub fn is_live(&self, key: &str) -> bool {
        let slot = lock(&self.table).get(key).cloned();
        let mut loaded = false;
        if let Some(slot) = slot {
            loaded = lock(&slot.live).is_some();
        }
        loaded
    }

    /// Envelope `bytes` the transport received from `peer`: placed in the
    /// inbox of the session its key names, whose key is answered so that the
    /// host runs it ([`Self::run`]).
    ///
    /// A reply goes to the session whose request it answers. An envelope
    /// whose key cannot be read is answered, when it is a request, with
    /// `RpcStatus::InvalidArgument`, and otherwise dropped and told to the
    /// host ([`Self::take_errors`]); either way no session is named.
    pub fn receive(&self, peer: &str, bytes: Vec<u8>) -> Option<String> {
        let placed = {
            let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&bytes)) else {
                self.error(PoolError::Malformed {
                    peer: peer.to_string(),
                });
                return None;
            };
            self.place(peer, &envelope)
        };
        match placed {
            Ok(key) => Some(self.post(
                key,
                Input::Envelope {
                    peer: peer.to_string(),
                    bytes,
                },
            )),
            Err(Unplaced::Reply) => {
                self.error(PoolError::UnroutedReply {
                    peer: peer.to_string(),
                });
                None
            }
            Err(Unplaced::Unkeyed {
                event,
                request,
                reason,
            }) => {
                match request {
                    Some(request) => self.refuse(
                        peer,
                        &request,
                        RpcStatus::InvalidArgument,
                        &alloc::format!("the request names no session: {reason}"),
                    ),
                    None => self.error(PoolError::Unkeyed {
                        peer: peer.to_string(),
                        event,
                        reason,
                    }),
                }
                None
            }
        }
    }

    /// `peer`'s link became ready. Every session is told, in order with what
    /// its inbox holds; the keys answered are the sessions to run.
    pub fn peer_ready(&self, peer: &str) -> Vec<String> {
        lock(&self.ready).insert(peer.to_string());
        self.broadcast(|| Input::Ready(peer.to_string()))
    }

    /// `peer`'s link stopped being ready. Every session is told, as
    /// [`Self::peer_ready`] tells it.
    pub fn peer_not_ready(&self, peer: &str) -> Vec<String> {
        lock(&self.ready).remove(peer);
        self.broadcast(|| Input::NotReady(peer.to_string()))
    }

    /// Let time pass for every session's endpoint: its ordering gaps, its
    /// transmissions waiting out their backoff and its requests' deadlines
    /// (see [`Endpoint::tick`]). The keys answered are the sessions to run.
    pub fn tick(&self) -> Vec<String> {
        self.broadcast(|| Input::Tick)
    }

    /// Run the session `key`: load it when it is not in memory, then apply
    /// what its inbox holds one input at a time, each run to completion and
    /// saved, and every delayed send of it that is due. Any thread may run any
    /// session; two runs of one key take turns, and the second finds the work
    /// done. A key the pool holds no session for is not run.
    pub fn run(&self, key: &str) {
        let Some(slot) = lock(&self.table).get(key).cloned() else {
            return;
        };
        let mut live = lock(&slot.live);
        loop {
            if live.is_none() {
                if self.retire_if_idle(&slot) {
                    return;
                }
                match self.load(&slot) {
                    Ok(loaded) => *live = Some(loaded),
                    Err(refusal) => {
                        self.abandon(&slot, &refusal);
                        return;
                    }
                }
            }
            let Some(session) = live.as_mut() else {
                return;
            };
            match self.serve(&slot, session) {
                Served::Waiting => return,
                Served::Finished => {
                    if let Some(finished) = live.take() {
                        self.finish(&slot, &finished);
                    }
                }
                Served::Lost(reason) => {
                    self.lose(&slot, &mut live, reason);
                    return;
                }
            }
        }
    }

    /// The sessions whose earliest wait is due now: those in memory, and
    /// those the store holds out of memory, which are loaded when they are
    /// run. A key another process holds is left to it.
    ///
    /// The store's index of earliest dues says which are due; each is read
    /// only once its key's lease is taken ([`Self::run`]). This is why the
    /// pool does not wake through `checkpoint::wake_due`, which reads every
    /// due session at once: a checkpoint read before the lease may be one
    /// another process has replaced since, and a session in memory here or
    /// leased elsewhere is not to be read at all.
    pub fn due(&self) -> Vec<String> {
        let now = self.wall_now();
        let mut keys = Vec::new();
        for slot in self.slots() {
            let Some(live) = try_lock(&slot.live) else {
                continue;
            };
            let due = live.as_ref().and_then(|session| session.earliest_due);
            if due.is_some_and(|due| due <= now) {
                keys.push(slot.key.clone());
            }
        }
        let listed = match self.store.due_before(now, WAKE_BATCH) {
            Ok(listed) => listed,
            Err(failure) => {
                self.error(PoolError::Store { key: None, failure });
                return keys;
            }
        };
        let deferred = lock(&self.deferred).clone();
        let mut table = lock(&self.table);
        for listed in listed {
            let held_elsewhere = deferred.get(&listed.key).is_some_and(|until| *until > now);
            if table.contains_key(&listed.key) || held_elsewhere {
                continue;
            }
            let slot = Arc::new(Slot::new(&listed.key));
            lock(&slot.inbox).queue.push_back(Input::Wake);
            table.insert(listed.key.clone(), slot);
            keys.push(listed.key);
        }
        keys
    }

    /// Renew every lease whose renewal is due. The keys answered are those
    /// whose lease was lost: the host runs each, which drops its session and
    /// answers what waits for it.
    pub fn renew_leases(&self) -> Vec<String> {
        let now = self.wall_now();
        let mut lost = Vec::new();
        for slot in self.slots() {
            let mut lease = lock(&slot.lease);
            let LeaseState::Held { fence, renewed_ms } = *lease else {
                continue;
            };
            if now < renewed_ms.saturating_add(self.config.renew_every_ms) {
                continue;
            }
            match self
                .store
                .renew(&slot.key, fence, self.config.lease_ttl_ms, now)
            {
                Ok(_) => {
                    *lease = LeaseState::Held {
                        fence,
                        renewed_ms: now,
                    }
                }
                Err(LeaseError::Lost(why)) => {
                    *lease = LeaseState::Lost;
                    self.error(PoolError::LeaseLost {
                        key: slot.key.clone(),
                        reason: why.to_string(),
                    });
                    lost.push(slot.key.clone());
                }
                // Kept: a renewal tried again before the lease runs out keeps
                // it, and one that comes too late is refused as lost.
                Err(LeaseError::Failed(failure)) => self.error(PoolError::Store {
                    key: Some(slot.key.clone()),
                    failure,
                }),
            }
        }
        lost
    }

    /// Save and drop every session that has had no event for `idle_ms`, holds
    /// nothing in its inbox and whose endpoint is quiet
    /// ([`Endpoint::is_quiet`]), and release its lease. Its last checkpoint is
    /// clean, since its endpoint takes nothing in after it, so its restore
    /// raises no `restored` row (SCE_MESH.md section 16.7, row 17). A session
    /// being run is not idle. The keys answered are those dropped.
    pub fn evict_idle(&self) -> Vec<String> {
        let now = self.wall_now();
        let mut evicted = Vec::new();
        for slot in self.slots() {
            let Some(mut live) = try_lock(&slot.live) else {
                continue;
            };
            let Some(session) = live.as_mut() else {
                continue;
            };
            if now < session.last_active_ms.saturating_add(self.sessions.idle_ms)
                || !lock(&slot.inbox).queue.is_empty()
                || !lock(&session.endpoint).is_quiet()
            {
                continue;
            }
            match self.checkpoint(&slot, session, now, Take::Clean) {
                Err(reason) => {
                    self.lose(&slot, &mut live, reason);
                    continue;
                }
                // Not saved: dropped, it would be lost.
                Ok(false) => continue,
                Ok(true) => {}
            }
            let removed = {
                let mut table = lock(&self.table);
                let mut inbox = lock(&slot.inbox);
                if inbox.queue.is_empty() {
                    inbox.closed = true;
                    remove_slot(&mut table, &slot);
                    true
                } else {
                    false
                }
            };
            if !removed {
                // An input arrived while the clean checkpoint was written, and
                // the endpoint takes it in after that checkpoint: one that is
                // not clean replaces it now, so that a restore after a crash
                // still tells the peers an envelope may have been lost.
                session.dirty = true;
                if let Err(reason) = self.checkpoint(&slot, session, now, Take::IfChanged) {
                    self.lose(&slot, &mut live, reason);
                }
                continue;
            }
            *live = None;
            self.forget_replies(&slot.key);
            self.release(&slot);
            evicted.push(slot.key.clone());
        }
        evicted
    }

    /// The earliest wall-clock moment, in milliseconds since the Unix epoch,
    /// at which [`Self::due`], [`Self::renew_leases`] or [`Self::evict_idle`]
    /// has work; `None` when none of them will. A session being run is left
    /// out: its run reports what it leaves. Of the sessions out of memory,
    /// one another process leases counts from when its lease runs out:
    /// `checkpoint::next_wake`, the first due of the whole store, would wake
    /// this process again and again for a session it may not serve.
    pub fn next_wake_ms(&self) -> Option<u64> {
        let mut next: Option<u64> = None;
        let mut at = |moment: u64| next = Some(next.map_or(moment, |n| n.min(moment)));
        for slot in self.slots() {
            if let LeaseState::Held { renewed_ms, .. } = *lock(&slot.lease) {
                at(renewed_ms.saturating_add(self.config.renew_every_ms));
            }
            let Some(live) = try_lock(&slot.live) else {
                continue;
            };
            let Some(session) = live.as_ref() else {
                continue;
            };
            if let Some(due) = session.earliest_due {
                at(due);
            }
            if lock(&session.endpoint).is_quiet() {
                at(session.last_active_ms.saturating_add(self.sessions.idle_ms));
            }
        }
        match self.store.due_before(LATEST_MOMENT, WAKE_BATCH) {
            Ok(listed) => {
                let table = lock(&self.table);
                let deferred = lock(&self.deferred);
                let first = listed
                    .iter()
                    .filter(|listed| !table.contains_key(&listed.key))
                    .map(|listed| {
                        deferred
                            .get(&listed.key)
                            .map_or(listed.due, |until| listed.due.max(*until))
                    })
                    .min();
                if let Some(first) = first {
                    at(first);
                }
            }
            Err(failure) => self.error(PoolError::Store { key: None, failure }),
        }
        next
    }

    /// What went wrong that only the host can act on, in the order it arose.
    pub fn take_errors(&self) -> Vec<PoolError> {
        core::mem::take(&mut *lock(&self.errors))
    }

    /// The session `envelope` is for, or why it names none.
    fn place(&self, peer: &str, envelope: &Envelope<'_>) -> Result<String, Unplaced> {
        if envelope.pattern == PatternKind::RpcReply {
            let wire_id: Option<[u8; 16]> = envelope.invoke_id.map(Into::into);
            return wire_id
                .and_then(|wire_id| lock(&self.replies).get(&wire_id).cloned())
                .ok_or(Unplaced::Reply);
        }
        self.key_of(peer, envelope)
            .map_err(|reason| Unplaced::Unkeyed {
                event: envelope.event_type.to_string(),
                request: request_of(envelope),
                reason,
            })
    }

    /// The key `envelope`, which arrived on `peer`'s link, names, or why it
    /// names none.
    fn key_of(&self, peer: &str, envelope: &Envelope<'_>) -> Result<String, String> {
        let key = match self.sessions.key_from {
            KeyFrom::Client => peer.to_string(),
            KeyFrom::Field(field) => field_key(envelope, field)?,
        };
        if key.is_empty() {
            return Err("its key is empty".to_string());
        }
        let most = usize::try_from(self.sessions.max_key_bytes).unwrap_or(usize::MAX);
        if key.len() > most {
            return Err(alloc::format!(
                "its key is {} bytes, more than the {most} a key of this pool holds",
                key.len()
            ));
        }
        Ok(key)
    }

    /// Put `input` in the inbox of the session `key`, which the table holds
    /// from now on if it did not.
    fn post(&self, key: String, input: Input) -> String {
        let mut table = lock(&self.table);
        let slot = table
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Slot::new(&key)));
        lock(&slot.inbox).queue.push_back(input);
        key
    }

    /// Put an input made by `input` in every session's inbox, and answer their
    /// keys.
    fn broadcast(&self, input: impl Fn() -> Input) -> Vec<String> {
        let table = lock(&self.table);
        for slot in table.values() {
            lock(&slot.inbox).queue.push_back(input());
        }
        table.keys().cloned().collect()
    }

    /// Every session the table holds now.
    fn slots(&self) -> Vec<Arc<Slot<F::Machine, T, E>>> {
        lock(&self.table).values().cloned().collect()
    }

    /// Take `slot` out of the table when its inbox holds nothing, so that it
    /// is not loaded for nothing. True when it is out of the table, taken out
    /// now or before.
    fn retire_if_idle(&self, slot: &Arc<Slot<F::Machine, T, E>>) -> bool {
        let mut table = lock(&self.table);
        let mut inbox = lock(&slot.inbox);
        if inbox.closed {
            return true;
        }
        if !inbox.queue.is_empty() {
            return false;
        }
        inbox.closed = true;
        remove_slot(&mut table, slot);
        true
    }

    /// Take `slot`'s key's lease and build its session: restored from the
    /// store when it holds one, started otherwise.
    fn load(&self, slot: &Slot<F::Machine, T, E>) -> Result<Live<F::Machine, T, E>, Refusal> {
        let key = slot.key.as_str();
        let now = self.wall_now();
        let lease = match self
            .store
            .acquire(key, &self.config.owner, self.config.lease_ttl_ms, now)
        {
            Ok(lease) => lease,
            Err(AcquireError::Held(held)) => {
                lock(&self.deferred).insert(key.to_string(), held.expires_ms);
                let reason = alloc::format!("another process serves the session: {held}");
                self.error(PoolError::LeaseHeld {
                    key: key.to_string(),
                    held,
                });
                return Err(Refusal::unavailable(reason));
            }
            Err(AcquireError::Failed(failure)) => {
                let reason = failure.to_string();
                self.error(PoolError::Store {
                    key: Some(key.to_string()),
                    failure,
                });
                return Err(Refusal::unavailable(reason));
            }
        };
        lock(&self.deferred).remove(key);
        *lock(&slot.lease) = LeaseState::Held {
            fence: lease.fence,
            renewed_ms: now,
        };
        let built = self.build(key, now);
        if built.is_err() {
            self.release(slot);
        }
        built
    }

    /// The session `key` as its last durable checkpoint holds it, or a new
    /// one, at `now`.
    fn build(&self, key: &str, now: u64) -> Result<Live<F::Machine, T, E>, Refusal> {
        let not_built = |reason: String| {
            self.error(PoolError::NotBuilt {
                key: key.to_string(),
                reason: reason.clone(),
            });
            Refusal {
                status: RpcStatus::InternalError,
                reason,
            }
        };
        let loaded = checkpoint::load(&*self.store, key).map_err(|error| match error {
            LoadError::Store(failure) => {
                let reason = failure.to_string();
                self.error(PoolError::Store {
                    key: Some(key.to_string()),
                    failure,
                });
                Refusal::unavailable(reason)
            }
            unreadable @ LoadError::Unreadable { .. } => not_built(unreadable.to_string()),
        })?;
        let endpoint = self.endpoint_for(key)?;
        let machine = match &loaded {
            Some(loaded) => self.restore(key, &loaded.checkpoint, &endpoint, now),
            None => self.factory.start(key, &endpoint, now),
        }
        .map_err(not_built)?;
        let (version, sequence, earliest_due) = loaded.as_ref().map_or((0, 0, None), |loaded| {
            (
                loaded.version,
                loaded.checkpoint.sequence,
                loaded.checkpoint.earliest_due(),
            )
        });
        Ok(Live {
            machine,
            endpoint,
            version,
            sequence,
            earliest_due,
            last_active_ms: now,
            // A machine just started ran its initial macrostep, which no
            // checkpoint holds yet.
            dirty: loaded.is_none(),
        })
    }

    /// The machine of the session `key` restored from `checkpoint` at `now`,
    /// and `endpoint`, the session's new endpoint, restored from the same
    /// checkpoint: what it held unreleased leaves, what it released for the
    /// machine and the machine had not applied is queued for it again, and,
    /// unless the checkpoint is clean, each peer it took envelopes from is
    /// told one may have been lost (SCE_MESH.md section 10.11).
    fn restore(
        &self,
        key: &str,
        checkpoint: &Checkpoint,
        endpoint: &SessionEndpoint<T, E>,
        now: u64,
    ) -> Result<F::Machine, String> {
        let machine = self
            .factory
            .restore(key, &checkpoint.state, endpoint, now)?;
        lock(endpoint).restore(checkpoint).map_err(|error| {
            alloc::format!("the session's endpoint cannot be restored: {error}")
        })?;
        Ok(machine)
    }

    /// A session's own endpoint, on the pool's transport, checkpointing, told
    /// which peers are ready.
    fn endpoint_for(&self, key: &str) -> Result<SessionEndpoint<T, E>, Refusal> {
        let mut router = self.machine.router().map_err(|error| Refusal {
            status: RpcStatus::InternalError,
            reason: alloc::format!("the session's router cannot be built: {error:?}"),
        })?;
        // A session is kept only as its checkpoint, so nothing it sends may
        // leave before a durable checkpoint holds it.
        router.enable_checkpoints();
        let transport = SessionTransport {
            key: key.to_string(),
            inner: Arc::clone(&self.transport),
            replies: Arc::clone(&self.replies),
        };
        let mut endpoint = Endpoint::new(router, transport, lock(&self.environment).clone());
        for peer in lock(&self.ready).iter() {
            endpoint.peer_ready(peer);
        }
        Ok(Arc::new(Mutex::new(endpoint)))
    }

    /// Apply what `slot`'s inbox holds to `session`, then what is due, taking
    /// a checkpoint after each input that took an envelope in or ran a
    /// macrostep.
    fn serve(&self, slot: &Slot<F::Machine, T, E>, session: &mut Live<F::Machine, T, E>) -> Served {
        let key = slot.key.as_str();
        loop {
            if matches!(*lock(&slot.lease), LeaseState::Lost) {
                return Served::Lost("its lease was not renewed".to_string());
            }
            let next = lock(&slot.inbox).queue.pop_front();
            let Some(input) = next else {
                break;
            };
            let now = self.wall_now();
            let event = matches!(input, Input::Envelope { .. });
            let calls = {
                let mut endpoint = lock(&session.endpoint);
                match input {
                    Input::Envelope { peer, bytes } => endpoint.receive(&peer, bytes),
                    Input::Ready(peer) => endpoint.peer_ready(&peer),
                    Input::NotReady(peer) => endpoint.peer_not_ready(&peer),
                    Input::Tick => endpoint.tick(),
                    Input::Wake => {}
                }
                endpoint.take_calls()
            };
            if self.drive(key, session, calls, now) || event {
                session.last_active_ms = now;
            }
            // An envelope taken in changed what the endpoint holds (its
            // duplicate window, its reply cache), which the checkpoint holds
            // too, even when it ran no macrostep.
            session.dirty |= event;
            if let Err(reason) = self.checkpoint(slot, session, now, Take::IfChanged) {
                return Served::Lost(reason);
            }
            if session.machine.finished() {
                return Served::Finished;
            }
        }
        let mut calls = lock(&session.endpoint).take_calls();
        loop {
            let now = self.wall_now();
            if self.drive(key, session, calls, now) {
                session.last_active_ms = now;
            }
            if let Err(reason) = self.checkpoint(slot, session, now, Take::IfChanged) {
                return Served::Lost(reason);
            }
            // What the checkpoint released and the transport refused at once
            // is a row the machine is told.
            calls = lock(&session.endpoint).take_calls();
            if calls.is_empty() || session.machine.finished() {
                break;
            }
        }
        self.prune_replies(key, session);
        if session.machine.finished() {
            Served::Finished
        } else {
            Served::Waiting
        }
    }

    /// Tell the machine `calls` and run it until it waits, again for as long
    /// as its endpoint has more for it. True when a macrostep ran.
    fn drive(
        &self,
        key: &str,
        session: &mut Live<F::Machine, T, E>,
        mut calls: Vec<EngineCall>,
        now: u64,
    ) -> bool {
        let mut stepped = false;
        loop {
            stepped |= session.machine.run(calls, now);
            let mut endpoint = lock(&session.endpoint);
            self.report(key, &mut endpoint);
            calls = endpoint.take_calls();
            drop(endpoint);
            if calls.is_empty() {
                break;
            }
        }
        session.dirty |= stepped;
        stepped
    }

    /// Take the checkpoint of `session` that `take` asks for, in the three
    /// steps of SCE_MESH.md section 10.11: the machine's saved state and the
    /// endpoint sealed with the next number ([`Endpoint::checkpoint`]), the
    /// write through a [`StoreSink`] over the version the session was read at
    /// and under the key's lease, and, once the write answered, the endpoint
    /// told the checkpoint is durable ([`Endpoint::durable`]), which releases
    /// what it holds. Answers whether a durable checkpoint holds the session
    /// as `take` asked. An error is the reason this process no longer holds
    /// the session's key; a save that failed otherwise leaves the session
    /// unsaved, and what it made held, to be saved at its next chance.
    fn checkpoint(
        &self,
        slot: &Slot<F::Machine, T, E>,
        session: &mut Live<F::Machine, T, E>,
        now: u64,
        take: Take,
    ) -> Result<bool, String> {
        if session.machine.finished() || (take == Take::IfChanged && !session.dirty) {
            return Ok(true);
        }
        let key = slot.key.as_str();
        let Some(fence) = fence_of(slot) else {
            return Err("its lease is no longer held".to_string());
        };
        let not_saved = |reason: String| -> Result<bool, String> {
            self.error(PoolError::NotSaved {
                key: key.to_string(),
                reason,
            });
            Ok(false)
        };
        let state = match session.machine.save(now) {
            Ok(state) => state,
            Err(reason) => return not_saved(reason),
        };
        let sequence = session.sequence.saturating_add(1);
        let sealed = lock(&session.endpoint).checkpoint(sequence, state, take == Take::Clean);
        let taken = match sealed {
            Ok(taken) => taken,
            Err(error) => return not_saved(error.to_string()),
        };
        // Sealed: the endpoint is never sealed with this number again, whether
        // or not the write lands, and a later checkpoint holds what this one
        // holds.
        session.sequence = sequence;
        let mut sink = StoreSink::new(&*self.store, key, session.version, fence);
        match sink.write(&taken) {
            Ok(()) => {
                session.version = sink.version();
                session.earliest_due = taken.earliest_due();
                session.dirty = false;
                let mut endpoint = lock(&session.endpoint);
                endpoint.durable(sequence);
                self.report(key, &mut endpoint);
                Ok(true)
            }
            Err(WriteError::Conflict(conflict)) => {
                *lock(&slot.lease) = LeaseState::Lost;
                let reason = alloc::format!("its save was refused: {conflict}");
                self.error(PoolError::LeaseLost {
                    key: key.to_string(),
                    reason: reason.clone(),
                });
                Err(reason)
            }
            Err(WriteError::Failed(failure)) => {
                self.error(PoolError::Store {
                    key: Some(key.to_string()),
                    failure,
                });
                Ok(false)
            }
        }
    }

    /// The machine of `slot` finished: its saved session is removed from the
    /// store, and its lease released. What the inputs since its last
    /// checkpoint made is sealed first and leaves once the removal answered,
    /// as a checkpoint's envelopes leave once it is durable: a process that
    /// fails before restores the session from that checkpoint, and its
    /// machine makes them again.
    fn finish(&self, slot: &Slot<F::Machine, T, E>, finished: &Live<F::Machine, T, E>) {
        let key = slot.key.as_str();
        if let Some(fence) = fence_of(slot) {
            let sequence = finished.sequence.saturating_add(1);
            let mut endpoint = lock(&finished.endpoint);
            let sealed = match endpoint.seal(sequence) {
                Ok(_) => true,
                Err(error) => {
                    self.error(PoolError::NotSaved {
                        key: key.to_string(),
                        reason: error.to_string(),
                    });
                    false
                }
            };
            match self.store.delete(key, finished.version, fence) {
                Ok(()) if sealed => {
                    endpoint.durable(sequence);
                    self.report(key, &mut endpoint);
                }
                Ok(()) => {}
                Err(WriteError::Conflict(conflict)) => self.error(PoolError::LeaseLost {
                    key: key.to_string(),
                    reason: alloc::format!("its end was refused: {conflict}"),
                }),
                Err(WriteError::Failed(failure)) => self.error(PoolError::Store {
                    key: Some(key.to_string()),
                    failure,
                }),
            }
        }
        self.forget_replies(key);
        self.release(slot);
    }

    /// Drop `slot`'s session, whose key this process no longer serves for
    /// `reason`, without saving it: what it held for a checkpoint is lost with
    /// it, since another process serves the key from its last durable one.
    /// Each request its inbox holds is answered `Unavailable`.
    fn lose(
        &self,
        slot: &Arc<Slot<F::Machine, T, E>>,
        live: &mut Option<Live<F::Machine, T, E>>,
        reason: String,
    ) {
        *live = None;
        *lock(&slot.lease) = LeaseState::Unheld;
        self.forget_replies(&slot.key);
        self.abandon(slot, &Refusal::unavailable(reason));
    }

    /// Hand the host what the endpoint of the session `key` reported to it.
    fn report(&self, key: &str, endpoint: &mut Endpoint<SessionTransport<T>, E>) {
        for error in endpoint.take_host_errors() {
            self.error(PoolError::Endpoint {
                key: key.to_string(),
                error,
            });
        }
    }

    /// Give up `slot`'s lease, if this process holds it.
    fn release(&self, slot: &Slot<F::Machine, T, E>) {
        let state = core::mem::replace(&mut *lock(&slot.lease), LeaseState::Unheld);
        let LeaseState::Held { fence, .. } = state else {
            return;
        };
        match self.store.release(&slot.key, fence) {
            // One that was taken over is not this process's to give up.
            Ok(()) | Err(LeaseError::Lost(_)) => {}
            Err(LeaseError::Failed(failure)) => self.error(PoolError::Store {
                key: Some(slot.key.clone()),
                failure,
            }),
        }
    }

    /// Stop serving `slot`'s key: take it out of the table and answer each
    /// request its inbox held with `refusal`; anything else it held is
    /// dropped and told to the host.
    fn abandon(&self, slot: &Arc<Slot<F::Machine, T, E>>, refusal: &Refusal) {
        let held = {
            let mut table = lock(&self.table);
            let mut inbox = lock(&slot.inbox);
            inbox.closed = true;
            remove_slot(&mut table, slot);
            core::mem::take(&mut inbox.queue)
        };
        for input in held {
            let Input::Envelope { peer, bytes } = input else {
                continue;
            };
            let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&bytes)) else {
                continue;
            };
            match request_of(&envelope) {
                Some(request) => self.refuse(&peer, &request, refusal.status, &refusal.reason),
                None => self.error(PoolError::Dropped {
                    key: slot.key.clone(),
                    event: envelope.event_type.to_string(),
                    reason: refusal.reason.clone(),
                }),
            }
        }
    }

    /// Answer `request`, which arrived on `peer`'s link, with `status` and
    /// `reason`: the reply a requester ends its invocation with, sent by the
    /// pool because no session of it answers.
    fn refuse(&self, peer: &str, request: &Request, status: RpcStatus, reason: &str) {
        let id = lock(&self.environment).envelope_id();
        let sent = refusal_envelope(self.machine.name, request, status, reason, id)
            .map_err(|error| alloc::format!("{error:?}"))
            .and_then(|bytes| {
                lock(&self.transport)
                    .transmit(peer, &bytes)
                    .map_err(|failure| alloc::format!("{failure:?}"))
            });
        if let Err(reason) = sent {
            self.error(PoolError::NotAnswered {
                peer: peer.to_string(),
                reason,
            });
        }
    }

    /// Forget every request the session `key` sent.
    fn forget_replies(&self, key: &str) {
        lock(&self.replies).retain(|_, owner| owner != key);
    }

    /// Forget the requests the session `key` sent that no longer wait.
    fn prune_replies(&self, key: &str, session: &Live<F::Machine, T, E>) {
        let endpoint = lock(&session.endpoint);
        lock(&self.replies).retain(|wire_id, owner| owner != key || endpoint.awaits_reply(wire_id));
    }

    fn error(&self, error: PoolError) {
        lock(&self.errors).push(error);
    }

    fn wall_now(&self) -> u64 {
        lock(&self.environment).now_unix_ms()
    }
}

/// The fence of the lease `slot` holds, if it holds one.
fn fence_of<M, T, E>(slot: &Slot<M, T, E>) -> Option<u64> {
    match *lock(&slot.lease) {
        LeaseState::Held { fence, .. } => Some(fence),
        LeaseState::Unheld | LeaseState::Lost => None,
    }
}

/// Take `slot` out of `table`, if it is still the session the table holds for
/// its key: a newer one for the key stays.
fn remove_slot<M, T, E>(table: &mut Table<M, T, E>, slot: &Arc<Slot<M, T, E>>) {
    if table
        .get(&slot.key)
        .is_some_and(|current| Arc::ptr_eq(current, slot))
    {
        table.remove(&slot.key);
    }
}

/// The key the field `field` of `envelope`'s data holds: a text as it is, an
/// integer as its digits.
fn field_key(envelope: &Envelope<'_>, field: &str) -> Result<String, String> {
    let text = payload_text(envelope).map_err(|codec| {
        alloc::format!("its data is in the {codec} codec, which the pool does not read")
    })?;
    let data = json::parse(&text).map_err(|error| alloc::format!("its data is {error}"))?;
    match data.member(field) {
        Some(Value::Text(text)) => Ok(text.clone()),
        Some(Value::Number(digits)) if is_integer(digits) => Ok(digits.clone()),
        Some(_) => Err(alloc::format!(
            "its field '{field}' is neither a text nor an integer"
        )),
        None => Err(alloc::format!("its data has no field '{field}'")),
    }
}

/// Whether a JSON number's spelling is an integer: digits, after a sign.
fn is_integer(digits: &str) -> bool {
    let unsigned = digits.strip_prefix('-').unwrap_or(digits);
    !unsigned.is_empty() && unsigned.bytes().all(|byte| byte.is_ascii_digit())
}

/// The request `envelope` is, if it is one.
fn request_of(envelope: &Envelope<'_>) -> Option<Request> {
    if envelope.pattern != PatternKind::RpcRequest {
        return None;
    }
    let invoke_id: [u8; 16] = envelope.invoke_id?.into();
    Some(Request {
        invoke_id,
        event: envelope.event_type.to_string(),
    })
}

/// The wire `invoke_id` of the request `bytes` are, if they are one.
fn request_id(bytes: &[u8]) -> Option<[u8; 16]> {
    let envelope = Envelope::decode(&mut SceCursor::new(bytes)).ok()?;
    request_of(&envelope).map(|request| request.invoke_id)
}

/// The reply that ends `request` with `status`: from `machine`, named as the
/// request's reply by the event-name convention (`service.request.X` is
/// answered by `service.response.X`), with `reason` as its message.
fn refusal_envelope(
    machine: &str,
    request: &Request,
    status: RpcStatus,
    reason: &str,
    id: [u8; 16],
) -> Result<Vec<u8>, RouterError> {
    let event = match request.event.strip_prefix("service.request.") {
        Some(suffix) => alloc::format!("service.response.{suffix}"),
        None => request.event.clone(),
    };
    Ok(Envelope {
        id: (&id).into(),
        source: within(machine)?,
        event_type: within(event.as_str())?,
        pattern: PatternKind::RpcReply,
        datacontenttype: PayloadCodec::None,
        invoke_id: Some((&request.invoke_id).into()),
        rpc_status: Some(status),
        rpc_error_message: Some(within(reason)?),
        ..Envelope::new()
    }
    .encode_to_vec())
}

/// `mutex`'s value, whatever a thread that panicked while it held it left:
/// every value here is changed in steps that leave it whole.
fn lock<V>(mutex: &Mutex<V>) -> MutexGuard<'_, V> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `mutex`'s value, or `None` while another thread holds it.
fn try_lock<V>(mutex: &Mutex<V>) -> Option<MutexGuard<'_, V>> {
    match mutex.try_lock() {
        Ok(guard) => Some(guard),
        Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::{MESH_PROCESSOR_TYPE, MESH_RPC_INVOKE_TYPE};
    use crate::inbound::Delivery;
    use crate::peers::{Peer, PeerLink};
    use crate::router::PeerConfig;
    use crate::rpc::MESH_EVENT_PARAM;
    use alloc::vec;
    use sce_rust_runtime::saved_state::{SavedAct, SavedHostSend, SavedInvokes, SavedSend};
    use sce_rust_runtime::{HostInvokeRequest, HostSendRequest};
    use sce_rust_session_store::{Due, Lease, MemoryStore, Stored};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Condvar;
    use std::time::{Duration, Instant};

    /// The wall clock every test starts at.
    const T0: u64 = 1_700_000_000_000;

    /// How many members a room admits: the rule the room's document owns.
    const LIMIT: u32 = 3;

    /// The wall clock every handle of one test reads, and envelope ids that
    /// never repeat across them.
    #[derive(Clone)]
    struct Clock {
        now: Arc<AtomicU64>,
        ids: Arc<AtomicU64>,
    }

    impl Clock {
        fn new() -> Self {
            Self {
                now: Arc::new(AtomicU64::new(T0)),
                ids: Arc::new(AtomicU64::new(0)),
            }
        }

        fn advance(&self, ms: u64) {
            self.now.fetch_add(ms, Ordering::SeqCst);
        }

        fn at(&self) -> u64 {
            self.now.load(Ordering::SeqCst)
        }
    }

    impl Environment for Clock {
        fn now_ms(&mut self) -> i64 {
            i64::try_from(self.at()).unwrap_or(i64::MAX)
        }
        fn now_unix_ms(&mut self) -> u64 {
            self.at()
        }
        fn envelope_id(&mut self) -> [u8; 16] {
            let n = self.ids.fetch_add(1, Ordering::SeqCst) + 1;
            let mut id = [0xA5; 16];
            id[8..].copy_from_slice(&n.to_be_bytes());
            id
        }
        fn jitter_draw(&mut self) -> i64 {
            0
        }
    }

    /// A transport that records what it was handed; its clones share the
    /// record.
    #[derive(Clone, Default)]
    struct Wire(Handed);

    /// What a test's transport was handed, by peer, in order, shared by its clones.
    type Handed = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

    impl Transport for Wire {
        fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
            lock(&self.0).push((peer.to_string(), bytes.to_vec()));
            Ok(())
        }
    }

    /// An envelope a test's transport was handed, as much of it as the tests
    /// read.
    #[derive(Debug)]
    struct Sent {
        /// The envelope's id: what a receiver's duplicate window knows a copy by.
        id: [u8; 16],
        peer: String,
        event: String,
        pattern: PatternKind,
        status: Option<RpcStatus>,
        wire_id: Option<[u8; 16]>,
        data: String,
    }

    impl Wire {
        /// What the transport was handed since the last call, in order.
        fn take(&self) -> Vec<Sent> {
            core::mem::take(&mut *lock(&self.0))
                .into_iter()
                .map(|(peer, bytes)| {
                    let envelope =
                        Envelope::decode(&mut SceCursor::new(&bytes)).expect("an envelope");
                    Sent {
                        id: envelope.id.into(),
                        peer,
                        event: envelope.event_type.to_string(),
                        pattern: envelope.pattern,
                        status: envelope.rpc_status,
                        wire_id: envelope.invoke_id.map(Into::into),
                        data: String::from_utf8_lossy(envelope.data.into_inner()).into_owned(),
                    }
                })
                .collect()
        }
    }

    /// What the room machines of one test report.
    #[derive(Default)]
    struct Probe {
        /// The key of each room machine inside `run` now.
        running: Mutex<Vec<String>>,
        /// Whether two runs of one key were ever inside `run` at once.
        overlapped: AtomicBool,
        /// How many runs reached the meeting point, and its signal.
        met: Mutex<u32>,
        arrived: Condvar,
        /// What each run that reached the meeting point saw there.
        meetings: Mutex<Vec<u32>>,
        started: AtomicU64,
        restored: AtomicU64,
        /// The answers of the requests the rooms sent, by room.
        answers: Mutex<Vec<(String, String)>>,
        /// The events a room was told and has no transition for.
        unhandled: Mutex<Vec<String>>,
    }

    impl Probe {
        fn enter(&self, key: &str) {
            let mut running = lock(&self.running);
            if running.iter().any(|running| running == key) {
                self.overlapped.store(true, Ordering::SeqCst);
            }
            running.push(key.to_string());
        }

        fn leave(&self, key: &str) {
            let mut running = lock(&self.running);
            if let Some(at) = running.iter().position(|running| running == key) {
                running.remove(at);
            }
        }

        /// Wait, at most five seconds, until two runs reached this point, and
        /// record how many had.
        fn meet(&self) {
            let mut met = lock(&self.met);
            *met += 1;
            self.arrived.notify_all();
            let deadline = Instant::now() + Duration::from_secs(5);
            while *met < 2 {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                met = self
                    .arrived
                    .wait_timeout(met, left)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
            let seen = *met;
            drop(met);
            lock(&self.meetings).push(seen);
        }
    }

    /// A room's machine, written by hand in the shape a generated one has to
    /// the pool: it reads what its endpoint tells it, answers requests through
    /// its endpoint, and keeps reminders that come due on the wall clock.
    struct Room {
        key: String,
        members: u32,
        /// When each reminder comes due, and the peer it is for.
        reminders: Vec<(u64, String)>,
        closed: bool,
        endpoint: SessionEndpoint<Wire, Clock>,
        probe: Arc<Probe>,
    }

    impl Room {
        fn send(&self, peer: &str, event: &str, data: &str, invoke_id: &str) {
            lock(&self.endpoint).send(&HostSendRequest {
                processor_type: MESH_PROCESSOR_TYPE.to_string(),
                event_name: event.to_string(),
                target: alloc::format!("#{peer}"),
                event_data: data.to_string(),
                invoke_id: invoke_id.to_string(),
                ..HostSendRequest::default()
            });
        }

        fn ask_other(&self) {
            let mut params = std::collections::HashMap::new();
            params.insert(
                MESH_EVENT_PARAM.to_string(),
                vec!["service.request.lookup".to_string()],
            );
            let started = lock(&self.endpoint).invoke(&HostInvokeRequest {
                processor_type: MESH_RPC_INVOKE_TYPE.to_string(),
                invoke_id: "lookup".to_string(),
                src: "#other".to_string(),
                params,
                token: 1,
                ..HostInvokeRequest::default()
            });
            assert!(started.refusal.is_none(), "{started:?}");
        }
    }

    impl SessionMachine for Room {
        fn run(&mut self, calls: Vec<EngineCall>, wall_now_ms: u64) -> bool {
            self.probe.enter(&self.key);
            let mut stepped = false;
            for call in calls {
                stepped = true;
                let event = match call {
                    EngineCall::Raise(event) => event,
                    EngineCall::EndInvoke { data, .. } => {
                        lock(&self.probe.answers).push((self.key.clone(), data));
                        continue;
                    }
                };
                let peer = event
                    .metadata
                    .origin_address()
                    .strip_prefix("mesh://")
                    .unwrap_or_default()
                    .to_string();
                let invoke_id = event.metadata.invoke_id.clone();
                match event.name.as_str() {
                    "service.request.join" => {
                        // Read, wait, write: a second run of the room between
                        // the two would admit past the limit.
                        let members = self.members;
                        std::thread::sleep(Duration::from_millis(2));
                        let admitted = members < LIMIT;
                        if admitted {
                            self.members = members + 1;
                        }
                        let answer = alloc::format!(
                            r#"{{"admitted":{admitted},"members":{}}}"#,
                            self.members
                        );
                        self.send(&peer, "service.response.join", &answer, &invoke_id);
                    }
                    "service.request.meet" => {
                        self.probe.meet();
                        self.send(&peer, "service.response.meet", "{}", &invoke_id);
                    }
                    "remind" => {
                        let after = json::parse(&event.metadata.data)
                            .ok()
                            .and_then(|data| match data.member("after_ms") {
                                Some(Value::Number(ms)) => ms.parse::<u64>().ok(),
                                _ => None,
                            })
                            .unwrap_or(0);
                        self.reminders.push((wall_now_ms + after, peer));
                    }
                    "ask" => self.ask_other(),
                    "close" => self.closed = true,
                    other => lock(&self.probe.unhandled).push(other.to_string()),
                }
            }
            let (due, waiting): (Vec<_>, Vec<_>) = core::mem::take(&mut self.reminders)
                .into_iter()
                .partition(|(due, _)| *due <= wall_now_ms);
            self.reminders = waiting;
            for (_, peer) in due {
                stepped = true;
                let data = alloc::format!(r#"{{"room":{}}}"#, json::quote(&self.key));
                self.send(&peer, "reminded", &data, "");
            }
            self.probe.leave(&self.key);
            stepped
        }

        /// The room as a generated `save_at` gives a machine: its members as
        /// a variable, and each reminder as a delayed send waiting, earliest
        /// first, so that the checkpoint's earliest due is the first
        /// reminder's.
        fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
            let mut pending: Vec<SavedSend> = self
                .reminders
                .iter()
                .map(|(due, peer)| SavedSend {
                    due: *due,
                    act: SavedAct::Host(SavedHostSend {
                        processor_type: MESH_PROCESSOR_TYPE.to_string(),
                        event: "reminded".to_string(),
                        target: alloc::format!("#{peer}"),
                        send_id: alloc::format!("reminder-{due}"),
                        ..SavedHostSend::default()
                    }),
                })
                .collect();
            pending.sort_by_key(|send| send.due);
            Ok(SavedState {
                shape: "room".to_string(),
                configuration: vec!["open".to_string()],
                current: "open".to_string(),
                variables: vec![(
                    "members".to_string(),
                    Value::Number(self.members.to_string()),
                )],
                history: Vec::new(),
                pending,
                invokes: SavedInvokes::Resumed(Vec::new()),
                session_id: alloc::format!("room-{}", self.key),
                invoke_starts: Vec::new(),
                host_invokes: Vec::new(),
                host_invoke_starts: Vec::new(),
                host_invoke_token: 0,
                auto_send_seq: 0,
                external: Vec::new(),
                revisions: Vec::new(),
                features: None,
            })
        }

        fn finished(&self) -> bool {
            self.closed
        }
    }

    /// The members and reminders of a saved room.
    fn room_of(state: &SavedState) -> Result<(u32, Vec<(u64, String)>), String> {
        let members = match state.variable("members") {
            Ok(Value::Number(digits)) => {
                digits.parse::<u32>().map_err(|error| error.to_string())?
            }
            other => return Err(alloc::format!("its members are {other:?}")),
        };
        let reminders = state
            .pending
            .iter()
            .map(|send| match &send.act {
                SavedAct::Host(host) => host
                    .target
                    .strip_prefix('#')
                    .map(|peer| (send.due, peer.to_string()))
                    .ok_or_else(|| "a reminder for no peer".to_string()),
                other => Err(alloc::format!("a reminder that is {other:?}")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((members, reminders))
    }

    /// The members of the room `store` holds under `key`, read from its last
    /// durable checkpoint.
    fn stored_members(store: &MemoryStore, key: &str) -> u32 {
        let loaded = checkpoint::load(store, key).expect("read").expect("saved");
        room_of(&loaded.checkpoint.state).expect("a room").0
    }

    struct Rooms {
        probe: Arc<Probe>,
    }

    impl SessionFactory<Wire, Clock> for Rooms {
        type Machine = Room;

        fn start(
            &self,
            key: &str,
            endpoint: &SessionEndpoint<Wire, Clock>,
            _wall_now_ms: u64,
        ) -> Result<Room, String> {
            self.probe.started.fetch_add(1, Ordering::SeqCst);
            Ok(Room {
                key: key.to_string(),
                members: 0,
                reminders: Vec::new(),
                closed: false,
                endpoint: Arc::clone(endpoint),
                probe: Arc::clone(&self.probe),
            })
        }

        fn restore(
            &self,
            key: &str,
            state: &SavedState,
            endpoint: &SessionEndpoint<Wire, Clock>,
            _wall_now_ms: u64,
        ) -> Result<Room, String> {
            let (members, reminders) = room_of(state)?;
            self.probe.restored.fetch_add(1, Ordering::SeqCst);
            Ok(Room {
                key: key.to_string(),
                members,
                reminders,
                closed: false,
                endpoint: Arc::clone(endpoint),
                probe: Arc::clone(&self.probe),
            })
        }
    }

    const CLIENT: PeerConfig = PeerConfig {
        transport: "wss",
        buffer: None,
        retry: None,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders: &["client"],
        deadline_ms: None,
        reply_events: &["service.response.join", "service.response.meet"],
        causal: None,
    };

    /// The rooms server: one session per room, keyed by the `room` field of
    /// what a client sends, idle after a second.
    const ROOMS: Machine = Machine {
        name: "rooms",
        dedup_window: 64,
        gap_timeout_ms: 50,
        peers: &[
            Peer {
                name: "client",
                config: CLIENT,
                link: PeerLink::WssAccept { keepalive_ms: None },
            },
            Peer {
                name: "other",
                config: PeerConfig {
                    responders: &["other"],
                    reply_events: &[],
                    ..CLIENT
                },
                link: PeerLink::WssDial {
                    url: "wss://other.example",
                    keepalive_ms: None,
                },
            },
        ],
        sessions: Some(Sessions {
            key_from: KeyFrom::Field("room"),
            max_key_bytes: 16,
            idle_ms: 1_000,
        }),
    };

    const LEASE_TTL_MS: u64 = 10_000;

    /// A pool of [`ROOMS`] as the process `owner` serves it over `store`, its
    /// links up, and the transport it sends through.
    fn pool(
        owner: &str,
        store: &MemoryStore,
        clock: &Clock,
        probe: &Arc<Probe>,
    ) -> (Pool<Rooms, Wire, Clock>, Wire) {
        pool_over(owner, Arc::new(store.clone()), clock, probe)
    }

    /// A pool of [`ROOMS`] as [`pool`] makes one, kept in `store`.
    fn pool_over(
        owner: &str,
        store: Arc<dyn SessionStore>,
        clock: &Clock,
        probe: &Arc<Probe>,
    ) -> (Pool<Rooms, Wire, Clock>, Wire) {
        let wire = Wire::default();
        let pool = Pool::new(
            ROOMS,
            PoolConfig {
                owner: owner.to_string(),
                lease_ttl_ms: LEASE_TTL_MS,
                renew_every_ms: 3_000,
            },
            Rooms {
                probe: Arc::clone(probe),
            },
            store,
            wire.clone(),
            clock.clone(),
        )
        .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
        pool.peer_ready("client");
        pool.peer_ready("other");
        (pool, wire)
    }

    /// Sixteen bytes that differ in their last four by `n`.
    fn id(tag: u8, n: u32) -> [u8; 16] {
        let mut id = [tag; 16];
        id[12..].copy_from_slice(&n.to_be_bytes());
        id
    }

    /// The `n`th envelope from the client: `event` with `data`, a request
    /// under the wire invoke id `id(0x22, n)` when `request`.
    fn from_client(event: &str, data: &str, n: u32, request: bool) -> Vec<u8> {
        let envelope_id = id(0x11, n);
        let wire_id = id(0x22, n);
        Envelope {
            id: (&envelope_id).into(),
            source: within("client").unwrap(),
            event_type: within(event).unwrap(),
            pattern: if request {
                PatternKind::RpcRequest
            } else {
                PatternKind::FireForget
            },
            datacontenttype: PayloadCodec::Json,
            data: within(data.as_bytes()).unwrap(),
            invoke_id: request.then(|| (&wire_id).into()),
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    fn join(room: &str, n: u32) -> Vec<u8> {
        from_client(
            "service.request.join",
            &alloc::format!(r#"{{"room":"{room}"}}"#),
            n,
            true,
        )
    }

    /// Place `bytes` from the client and run the session they name.
    fn deliver(pool: &Pool<Rooms, Wire, Clock>, bytes: Vec<u8>) -> String {
        let key = pool.receive("client", bytes).expect("placed in a session");
        pool.run(&key);
        key
    }

    /// The requests of one key are taken one at a time, however many threads
    /// deliver and run them: the room never admits more than its limit, one
    /// session holds the key, and no two runs of it overlap.
    #[test]
    fn one_key_takes_its_requests_one_at_a_time_and_its_limit_holds() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        let pool = Arc::new(pool);
        let threads: Vec<_> = (0..12u32)
            .map(|n| {
                let pool = Arc::clone(&pool);
                std::thread::spawn(move || {
                    deliver(&pool, join("r1", n));
                })
            })
            .collect();
        for thread in threads {
            thread.join().expect("no run panicked");
        }
        let replies = wire.take();
        assert_eq!(replies.len(), 12, "{replies:?}");
        let admitted = replies
            .iter()
            .filter(|reply| reply.data.contains(r#""admitted":true"#))
            .count();
        assert_eq!(admitted, LIMIT as usize, "{replies:?}");
        for reply in &replies {
            assert_eq!(reply.pattern, PatternKind::RpcReply);
            assert_eq!(reply.status, Some(RpcStatus::Ok));
            assert_eq!(reply.peer, "client");
        }
        let mut answered: Vec<[u8; 16]> = replies.iter().filter_map(|r| r.wire_id).collect();
        answered.sort_unstable();
        let mut asked: Vec<[u8; 16]> = (0..12u32).map(|n| id(0x22, n)).collect();
        asked.sort_unstable();
        assert_eq!(answered, asked, "each request is answered once");
        assert!(!probe.overlapped.load(Ordering::SeqCst));
        assert_eq!(probe.started.load(Ordering::SeqCst), 1, "one session");
        assert!(pool.take_errors().is_empty());
        assert_eq!(stored_members(&store, "r1"), LIMIT);
    }

    /// Sessions of different keys run beside each other: a run of one key
    /// waits at a meeting point until a run of the other reaches it too, which
    /// it never would if the pool ran one session at a time.
    #[test]
    fn sessions_of_different_keys_run_beside_each_other() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        let pool = Arc::new(pool);
        let threads: Vec<_> = ["r1", "r2"]
            .into_iter()
            .enumerate()
            .map(|(n, room)| {
                let pool = Arc::clone(&pool);
                let n = u32::try_from(n).expect("small");
                std::thread::spawn(move || {
                    let meet = alloc::format!(r#"{{"room":"{room}"}}"#);
                    deliver(&pool, from_client("service.request.meet", &meet, n, true));
                })
            })
            .collect();
        for thread in threads {
            thread.join().expect("no run panicked");
        }
        assert_eq!(*lock(&probe.meetings), vec![2, 2], "both runs met");
        assert_eq!(wire.take().len(), 2);
    }

    /// While one process holds a key's lease, another does not serve it: its
    /// request is answered `Unavailable`, which tells the client to try again,
    /// it writes nothing, and the holder keeps serving the key.
    #[test]
    fn a_key_leased_to_another_process_is_not_served_here() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (first, first_wire) = pool("p1", &store, &clock, &probe);
        let (second, second_wire) = pool("p2", &store, &clock, &probe);
        deliver(&first, join("r1", 1));
        assert_eq!(first_wire.take().len(), 1);
        let version = store.get("r1").expect("read").expect("saved").version;

        deliver(&second, join("r1", 2));
        let replies = second_wire.take();
        let [reply] = replies.as_slice() else {
            panic!("one reply, got {replies:?}");
        };
        assert_eq!(reply.pattern, PatternKind::RpcReply);
        assert_eq!(reply.status, Some(RpcStatus::Unavailable));
        assert_eq!(reply.wire_id, Some(id(0x22, 2)));
        assert_eq!(reply.event, "service.response.join");
        assert_eq!(reply.peer, "client");
        assert!(!second.is_live("r1"));
        assert_eq!(
            store.get("r1").expect("read").expect("saved").version,
            version,
            "the second process wrote nothing"
        );
        let errors = second.take_errors();
        assert!(
            matches!(errors.as_slice(), [PoolError::LeaseHeld { key, held }] if key == "r1" && held.owner == "p1"),
            "{errors:?}"
        );

        deliver(&first, join("r1", 3));
        let replies = first_wire.take();
        assert!(
            replies[0].data.contains(r#""members":2"#),
            "the holder still serves the room: {replies:?}"
        );
    }

    /// A process whose lease ran out and was taken over stops serving the key
    /// when its renewal is refused: its session is dropped, and a request it
    /// is handed for the key is answered `Unavailable`. The process that took
    /// the key over restored what the first one saved, and released again
    /// what that checkpoint held (SCE_MESH.md section 10.11): the first
    /// process may have failed between writing it and sending its reply, so
    /// the reply goes again, the same envelope under the same id, which the
    /// client's duplicate window drops.
    #[test]
    fn a_process_whose_lease_was_taken_over_stops_serving_the_key() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (first, first_wire) = pool("p1", &store, &clock, &probe);
        let (second, second_wire) = pool("p2", &store, &clock, &probe);
        deliver(&first, join("r1", 1));
        let answered = first_wire.take();
        assert_eq!(answered.len(), 1, "{answered:?}");

        clock.advance(LEASE_TTL_MS + 1);
        deliver(&second, join("r1", 2));
        let replies = second_wire.take();
        assert_eq!(replies.len(), 2, "{replies:?}");
        assert_eq!(
            (replies[0].id, replies[0].wire_id, &replies[0].data),
            (answered[0].id, answered[0].wire_id, &answered[0].data),
            "the restore releases again the reply its checkpoint held"
        );
        assert!(replies[1].data.contains(r#""members":2"#), "{replies:?}");
        assert_eq!(replies[1].wire_id, Some(id(0x22, 2)));
        assert_eq!(probe.restored.load(Ordering::SeqCst), 1);

        assert_eq!(first.renew_leases(), vec!["r1".to_string()]);
        first.run("r1");
        assert!(!first.is_live("r1"));
        deliver(&first, join("r1", 3));
        let replies = first_wire.take();
        assert_eq!(replies.len(), 1, "{replies:?}");
        assert_eq!(replies[0].status, Some(RpcStatus::Unavailable));
        assert!(second.is_live("r1"));
    }

    /// A process that did not learn of a takeover by renewing learns it when
    /// it saves: the save is refused by the fence, and the session is dropped
    /// without its write landing.
    #[test]
    fn a_save_under_a_lease_taken_over_is_refused_and_the_session_dropped() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (first, first_wire) = pool("p1", &store, &clock, &probe);
        let (second, _second_wire) = pool("p2", &store, &clock, &probe);
        deliver(&first, join("r1", 1));
        clock.advance(LEASE_TTL_MS + 1);
        deliver(&second, join("r1", 2));
        let taken_over = store.get("r1").expect("read").expect("saved");

        deliver(&first, join("r1", 3));
        assert!(!first.is_live("r1"));
        assert_eq!(
            store.get("r1").expect("read").expect("saved"),
            taken_over,
            "the first process's write did not land"
        );
        let errors = first.take_errors();
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, PoolError::LeaseLost { key, .. } if key == "r1")),
            "{errors:?}"
        );
        first_wire.take();
    }

    /// A session with no event for its idle time is saved, dropped and its
    /// lease released; the next request for its key restores it where it
    /// stood. One that waits for an answer is not idle.
    #[test]
    fn an_idle_session_is_saved_dropped_and_restored_by_its_next_request() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        deliver(&pool, join("r1", 1));
        deliver(&pool, join("r1", 2));
        wire.take();

        clock.advance(999);
        assert!(pool.evict_idle().is_empty(), "idle for less than a second");
        clock.advance(1);
        assert_eq!(pool.evict_idle(), vec!["r1".to_string()]);
        assert!(!pool.is_live("r1"));
        // Its lease was released: another process may take the key at once.
        let lease = store
            .acquire("r1", "someone", 1, clock.at())
            .expect("released");
        store.release("r1", lease.fence).expect("given back");

        deliver(&pool, join("r1", 3));
        let replies = wire.take();
        assert!(replies[0].data.contains(r#""members":3"#), "{replies:?}");
        assert_eq!(probe.started.load(Ordering::SeqCst), 1);
        assert_eq!(probe.restored.load(Ordering::SeqCst), 1);

        // A session whose request to another machine waits for its answer
        // holds something pending: it stays.
        deliver(&pool, from_client("ask", r#"{"room":"r1"}"#, 4, false));
        let asked = wire.take();
        let [request] = asked.as_slice() else {
            panic!("one request, got {asked:?}");
        };
        assert_eq!(request.peer, "other");
        clock.advance(1_000);
        assert!(pool.evict_idle().is_empty(), "it waits for an answer");
        assert!(pool.is_live("r1"));
    }

    /// A reply reaches the session whose request it answers, though it
    /// carries no key of its own; a reply to no request of the pool reaches
    /// none.
    #[test]
    fn a_reply_reaches_the_session_whose_request_it_answers() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        deliver(&pool, from_client("ask", r#"{"room":"r2"}"#, 1, false));
        let asked = wire.take();
        let wire_id = asked[0].wire_id.expect("a request carries its invoke id");

        let answer = |wire_id: [u8; 16], n: u32| {
            let envelope_id = id(0x33, n);
            Envelope {
                id: (&envelope_id).into(),
                source: within("other").unwrap(),
                event_type: within("service.response.lookup").unwrap(),
                pattern: PatternKind::RpcReply,
                datacontenttype: PayloadCodec::Json,
                data: within(&b"{\"x\":1}"[..]).unwrap(),
                invoke_id: Some((&wire_id).into()),
                rpc_status: Some(RpcStatus::Ok),
                ..Envelope::new()
            }
            .encode_to_vec()
        };
        assert_eq!(
            pool.receive("other", answer(wire_id, 1)),
            Some("r2".to_string())
        );
        pool.run("r2");
        assert_eq!(
            *lock(&probe.answers),
            vec![("r2".to_string(), r#"{"x":1}"#.to_string())]
        );
        assert!(
            lock(&pool.replies).is_empty(),
            "an answered request is forgotten"
        );

        assert_eq!(pool.receive("other", answer([0x44; 16], 2)), None);
        assert!(matches!(
            pool.take_errors().as_slice(),
            [PoolError::UnroutedReply { peer }] if peer == "other"
        ));
    }

    /// A session out of memory is loaded when its first delayed send comes
    /// due, and not before; its restore arms the send, which is delivered
    /// once.
    #[test]
    fn a_session_out_of_memory_is_loaded_when_its_delayed_send_comes_due() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        deliver(
            &pool,
            from_client("remind", r#"{"room":"r1","after_ms":5000}"#, 1, false),
        );
        assert_eq!(
            store.get("r1").expect("read").expect("saved").earliest_due,
            Some(T0 + 5_000)
        );
        clock.advance(1_000);
        assert_eq!(pool.evict_idle(), vec!["r1".to_string()]);
        assert_eq!(pool.next_wake_ms(), Some(T0 + 5_000));

        clock.advance(3_999);
        assert!(pool.due().is_empty(), "not due yet");
        assert!(wire.take().is_empty());

        clock.advance(1);
        assert_eq!(pool.due(), vec!["r1".to_string()]);
        pool.run("r1");
        let sent = wire.take();
        let [reminded] = sent.as_slice() else {
            panic!("one reminder, got {sent:?}");
        };
        assert_eq!(reminded.event, "reminded");
        assert_eq!(reminded.peer, "client");
        assert_eq!(reminded.pattern, PatternKind::FireForget);
        assert_eq!(reminded.data, r#"{"room":"r1"}"#);
        assert_eq!(probe.restored.load(Ordering::SeqCst), 1);
        assert_eq!(
            store.get("r1").expect("read").expect("saved").earliest_due,
            None,
            "the reminder left the saved state as it was delivered"
        );
        assert!(pool.due().is_empty());
        pool.run("r1");
        assert!(wire.take().is_empty(), "delivered once");
    }

    /// An envelope whose key cannot be read names no session: a request is
    /// answered `InvalidArgument`, anything else is dropped and told to the
    /// host. A key may be an integer, and is its digits.
    #[test]
    fn an_envelope_whose_key_cannot_be_read_names_no_session() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        for (n, data) in [
            r#"{"who":"x"}"#,
            r#"{"room":"a room name far over sixteen bytes"}"#,
            r#"{"room":true}"#,
            r#"{"room":""}"#,
        ]
        .into_iter()
        .enumerate()
        {
            let n = u32::try_from(n).expect("small");
            assert_eq!(
                pool.receive("client", from_client("service.request.join", data, n, true)),
                None,
                "{data}"
            );
            let replies = wire.take();
            assert_eq!(replies.len(), 1, "{data}: {replies:?}");
            assert_eq!(
                replies[0].status,
                Some(RpcStatus::InvalidArgument),
                "{data}"
            );
            assert_eq!(replies[0].wire_id, Some(id(0x22, n)));
        }
        assert!(pool.take_errors().is_empty());

        assert_eq!(
            pool.receive("client", from_client("remind", "{}", 9, false)),
            None
        );
        assert!(matches!(
            pool.take_errors().as_slice(),
            [PoolError::Unkeyed { event, .. }] if event == "remind"
        ));
        assert!(wire.take().is_empty());

        assert_eq!(
            pool.receive(
                "client",
                from_client("service.request.join", r#"{"room":7}"#, 10, true)
            ),
            Some("7".to_string())
        );
        assert_eq!(pool.receive("client", vec![0xFF]), None);
        assert!(matches!(
            pool.take_errors().as_slice(),
            [PoolError::Malformed { peer }] if peer == "client"
        ));
    }

    /// A session whose machine finished is removed from the store and from
    /// memory, and the next request for its key starts a new one.
    #[test]
    fn a_finished_session_is_removed_and_its_key_starts_again() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        deliver(&pool, join("r1", 1));
        deliver(&pool, from_client("close", r#"{"room":"r1"}"#, 2, false));
        assert!(!pool.is_live("r1"));
        assert_eq!(store.get("r1").expect("read"), None);
        deliver(&pool, join("r1", 3));
        let replies = wire.take();
        assert!(replies[1].data.contains(r#""members":1"#), "{replies:?}");
        assert_eq!(probe.started.load(Ordering::SeqCst), 2);
    }

    /// A link's readiness reaches every session: one that is lost is told to
    /// each as the row a machine is told.
    #[test]
    fn a_link_lost_is_told_to_every_session() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, _wire) = pool("p1", &store, &clock, &probe);
        deliver(&pool, join("r1", 1));
        deliver(&pool, join("r2", 2));
        let keys = pool.peer_not_ready("client");
        assert_eq!(keys, vec!["r1".to_string(), "r2".to_string()]);
        for key in &keys {
            pool.run(key);
        }
        assert_eq!(
            *lock(&probe.unhandled),
            vec![
                "error.communication".to_string(),
                "error.communication".to_string()
            ]
        );
        assert_eq!(pool.tick(), keys);
    }

    /// A store whose writes fail while `failing` is set, as a medium that is
    /// down fails them: nothing is written.
    struct Flaky {
        inner: MemoryStore,
        failing: AtomicBool,
    }

    impl SessionStore for Flaky {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(key)
        }

        fn put(
            &self,
            key: &str,
            state: &[u8],
            earliest_due: Option<u64>,
            expected_version: u64,
            fence: u64,
        ) -> Result<u64, WriteError> {
            if self.failing.load(Ordering::SeqCst) {
                return Err(WriteError::Failed(Failure::new("the medium is down")));
            }
            self.inner
                .put(key, state, earliest_due, expected_version, fence)
        }

        fn delete(&self, key: &str, expected_version: u64, fence: u64) -> Result<(), WriteError> {
            self.inner.delete(key, expected_version, fence)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn acquire(
            &self,
            key: &str,
            owner: &str,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, AcquireError> {
            self.inner.acquire(key, owner, ttl_ms, now_ms)
        }

        fn renew(
            &self,
            key: &str,
            fence: u64,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, LeaseError> {
            self.inner.renew(key, fence, ttl_ms, now_ms)
        }

        fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
            self.inner.release(key, fence)
        }
    }

    /// Nothing a session sends leaves before a checkpoint that holds it is
    /// durable (SCE_MESH.md section 10.11): a reply whose checkpoint the store
    /// failed to write is held, and leaves, before the next reply, with the
    /// next checkpoint the store writes.
    #[test]
    fn a_reply_leaves_only_once_a_checkpoint_that_holds_it_is_durable() {
        let (clock, probe) = (Clock::new(), Arc::new(Probe::default()));
        let store = Arc::new(Flaky {
            inner: MemoryStore::new(),
            failing: AtomicBool::new(true),
        });
        let shared: Arc<dyn SessionStore> = Arc::clone(&store) as Arc<dyn SessionStore>;
        let (pool, wire) = pool_over("p1", shared, &clock, &probe);

        deliver(&pool, join("r1", 1));
        assert!(
            wire.take().is_empty(),
            "the reply left before a checkpoint holding it was written"
        );
        let errors = pool.take_errors();
        assert!(
            !errors.is_empty()
                && errors.iter().all(
                    |error| matches!(error, PoolError::Store { key: Some(key), .. } if key == "r1")
                ),
            "each failed write is told to the host: {errors:?}"
        );
        assert!(pool.is_live("r1"), "an unsaved session stays in memory");
        clock.advance(1_000);
        assert!(
            pool.evict_idle().is_empty(),
            "a session holding a reply for its checkpoint is not dropped"
        );

        store.failing.store(false, Ordering::SeqCst);
        deliver(&pool, join("r1", 2));
        let replies = wire.take();
        assert_eq!(replies.len(), 2, "{replies:?}");
        assert_eq!(replies[0].wire_id, Some(id(0x22, 1)));
        assert!(replies[0].data.contains(r#""members":1"#), "{replies:?}");
        assert_eq!(replies[1].wire_id, Some(id(0x22, 2)));
        assert!(replies[1].data.contains(r#""members":2"#), "{replies:?}");
        assert_eq!(stored_members(&store.inner, "r1"), 2);
    }

    /// A session is kept with its endpoint: a duplicate of a request it
    /// answered before it was dropped from memory is answered, once it is
    /// restored, with the first reply, and its machine does not take the
    /// request in again. A session dropped while idle is kept as a clean
    /// checkpoint, so its restore tells no peer an envelope may have been lost.
    #[test]
    fn a_restored_session_answers_a_duplicate_request_with_its_first_reply() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        deliver(&pool, join("r1", 1));
        let first = wire.take();
        assert_eq!(first.len(), 1, "{first:?}");

        clock.advance(1_000);
        assert_eq!(pool.evict_idle(), vec!["r1".to_string()]);
        deliver(&pool, join("r1", 1));
        let again = wire.take();
        assert_eq!(again.len(), 1, "{again:?}");
        assert_eq!(again[0].wire_id, first[0].wire_id);
        assert_eq!(again[0].data, first[0].data);
        assert_eq!(probe.restored.load(Ordering::SeqCst), 1);
        assert_eq!(
            stored_members(&store, "r1"),
            1,
            "the machine took the request in once"
        );
        assert!(
            lock(&probe.unhandled).is_empty(),
            "a clean checkpoint's restore raises no restored row: {:?}",
            lock(&probe.unhandled)
        );
    }

    #[test]
    fn a_machine_not_pooled_or_a_renewal_not_below_the_lease_is_refused() {
        let store: Arc<dyn SessionStore> = Arc::new(MemoryStore::new());
        let config = PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: 1_000,
            renew_every_ms: 999,
        };
        let build = |machine: Machine, config: PoolConfig| {
            Pool::new(
                machine,
                config,
                Rooms {
                    probe: Arc::new(Probe::default()),
                },
                Arc::clone(&store),
                Wire::default(),
                Clock::new(),
            )
            .err()
        };
        assert_eq!(
            build(
                Machine {
                    sessions: None,
                    ..ROOMS
                },
                config.clone()
            ),
            Some(PoolConfigError::NotPooled {
                machine: "rooms".to_string()
            })
        );
        for renew_every_ms in [0, 1_000] {
            assert_eq!(
                build(
                    ROOMS,
                    PoolConfig {
                        renew_every_ms,
                        ..config.clone()
                    }
                ),
                Some(PoolConfigError::Renewal {
                    renew_every_ms,
                    lease_ttl_ms: 1_000
                })
            );
        }
        assert!(build(ROOMS, config).is_none());
    }
}
