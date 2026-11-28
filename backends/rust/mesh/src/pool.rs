// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A machine served as a pool of sessions, one per key (docs/adr/0107,
//! decision D3; SCE_MESH.md section 14.4, "Keyed session pool").
//!
//! deploy.yaml's `sessions:` says where the key of an envelope is read: a
//! field of its data, or the name of the client whose link it arrived on; or
//! that none is read, and the pool keeps one session, whose key is the empty
//! key, which every input is for ([`KeyFrom::One`], docs/adr/0114, D4).
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
//! with the next number ([`Endpoint::checkpoint`]), the write
//! ([`SessionStore::write`]) carrying the version it was read at and the
//! lease's fence, so a session another process took over is never written by
//! this one, and
//! the endpoint told the checkpoint is durable ([`Endpoint::durable`]), which
//! releases what the input made: nothing a session sends leaves before the
//! checkpoint that holds it is durable. The write is handed to the pool's
//! committer ([`Committer`], docs/adr/0109, D15), which commits the writes of
//! every session handed to it since its last commit in one transaction of the
//! store and answers each once that transaction is durable: the sessions of
//! different keys, run on different threads, share the cost of a commit, and
//! each still releases nothing before the commit that holds it. A session
//! that had no event for `idle_ms` and holds nothing pending is saved as a
//! clean checkpoint, dropped from memory, and its lease released
//! ([`Pool::evict_idle`]), so its
//! restore tells no peer it may have lost an envelope. One whose earliest wait
//! comes due while it is out of memory is loaded again ([`Pool::due`]), and its
//! restore arms what came due.
//!
//! What a client can make the pool hold is bounded (docs/adr/0109, D1;
//! SCE_MESH.md section 14.4). At most `sessions.max_live` sessions are in
//! memory: at the bound the session idle longest that holds nothing pending
//! is saved and dropped to make room, and when none can be, an envelope for a
//! key not in memory is shed. At most `sessions.max_inbox` envelopes wait in
//! one session's inbox, and one past it is shed the same way. A session
//! started for a key the store holds nothing for is provisional until its
//! first run ends: if its machine finished in that run it is never written,
//! so a document that answers and finishes on a key it does not know costs
//! the store nothing.
//!
//! A host whose binding acknowledges what it carries (the HTTPS binding's
//! `200`, SCE_MESH.md section 22) acknowledges nothing a crash would lose: it
//! hands the pool each envelope with [`Pool::receive_with_receipt`] and reads
//! the envelope's [`Receipt`] once the session it names has run. The run that
//! takes the envelope in settles it when a durable checkpoint holds what the
//! session made of it, leaves it pending while the checkpoint's write failed
//! (a later checkpoint, which the envelope carried again drives, holds it),
//! and loses it when the session is dropped before one does ([`Fate`]). The
//! receipt is the session's own: a session's checkpoint numbers start again
//! when it ends or is taken over, so a number says nothing across sessions.
//!
//! A client whose `https` accepting binding declares `receipt: durable` has an
//! outbox in the store (docs/adr/0109, D10; [`crate::outbox`]): what a
//! checkpoint holds for it is appended to the outbox by the write that keeps
//! the checkpoint, so the session holds nothing for it once the write answered
//! and the server answers the client's exchanges from the outbox
//! ([`Pool::outboxes`]). The binding's `mailbox` is a window every session
//! shares: an envelope written while the client's outbox and what the sessions
//! produced for it hold `mailbox` is refused at admit as SCE_MESH.md section
//! 16.7 row 9. An entry older than `retain_ms` expires ([`Pool::expire`]), and
//! the session that sent it is told at its next run, as row 15.
//!
//! A session whose machine says facts into the event journal (docs/adr/0114,
//! D1; [`SessionMachine::take_journal`]) has them appended to its own stream,
//! the machine and the key (D2), by the write of the checkpoint that holds the
//! state that said them, so an entry exists exactly when that state does: a
//! write that fails keeps them for the next, and a session that ends appends
//! them by the write that removes it, one the store never held too. The pool
//! keeps its sessions through a handle of its own machine
//! (`SessionStore::machine`), or of a store that keeps one machine.
//!
//! A pool whose machine follows journals (deploy.yaml's `follows:`, docs/adr/0114,
//! D4) reads them from the store it is given ([`Pool::follow_from`]) when the
//! host calls [`Pool::follow`], and hands each entry to the session its key
//! field names; each session keeps the position it took in of each journal in
//! its checkpoint, so an entry handed again takes effect once (module
//! `follow`). What such a session decides is made on what it read
//! (docs/adr/0116, D5): a fact it says records how far it had read each journal
//! it follows, and a request that carries a position of them, in the field
//! deploy.yaml's `sessions.position_from` names, waits in its session's inbox
//! until the reader's mark has passed the position; a position above the
//! journal's horizon is refused where it is received.
//!
//! A machine bound to an open machine (docs/adr/0113), whose instances no
//! deployment lists, is pooled with the store that keeps their members
//! ([`Pool::with_members`]). Its edge judges each exchange of an instance by
//! the member's incarnation and low-water mark and hands the envelope on with
//! that judgement ([`Pool::receive_member`]); the session that runs it judges
//! it again by its completion records, and the write that keeps its checkpoint
//! names every incarnation the run was made under, so the store keeps nothing
//! of a run once one of them ended. Such a write refused is not a lost lease:
//! the session is let go and restored from what the store keeps, and an
//! envelope found fenced is [`Fate::Fenced`], which the edge answers `412`.
//! What concerns one instance reaches only the sessions whose routers bind it
//! (D6): its readiness, the store's latest answer for its member
//! ([`Pool::stand`]), and the end of its membership ([`Pool::end_members`]),
//! which unbinds it and drops what each session kept of it.
//!
//! [`Pool::stop`] stops the pool in order: it admits nothing, runs what each
//! inbox holds, and saves every session as a clean checkpoint, so a start
//! after it raises no `restored`.
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
//!
//! The pool reports what it does through the metrics port its committer
//! reports to (docs/adr/0109, D16; [`Pool::observed`]): the sessions it holds
//! in memory, each one it loads, restores and evicts, each envelope it sheds
//! and why, and each lease renewal the store refused. [`Pool::health`] says
//! whether it takes work in, whether a stop is going on, and whether its store
//! answers, for the program's readiness and liveness checks.
//!
//! What the pool decides of one session's slot under the slot's locks
//! (whether an envelope is admitted or meets a bound, how an inbox counts what
//! it holds and when its session may leave the table, the lease the process
//! holds on the key and when it comes due) is the module `slot`'s, a core
//! proved by Verus (docs/adr/0115, D13). What is left here is who holds which
//! lock and when, judged by the model `SessionPool`
//! (docs/formal/SessionPool.tla).

mod slot;

/// The locks and the flags that decide who holds a session's slot, and when
/// (the model `SessionPool`): the table, the `Arc` it shares each slot by,
/// each slot's inbox, `live` and lease locks, the lock of the journals'
/// reader, which holds the table while it hands an entry, and the stop's
/// flags. A slot is told apart from another by its serial, never by its
/// `Arc`, so what the loom models explore of a slot's sharing is the text that
/// ships: who clones it out of the table, and when its last holder lets it go.
///
/// A build gets `std`'s and `core`'s. The loom models (`loom_models` in the
/// tests below) build the crate's tests under `--cfg loom` and get loom's,
/// which is how they explore every interleaving of the text that ships; the
/// library is never built with loom's. The pool's other locks (a session's
/// endpoint, the transport, the replies awaited, the clock, the errors) stay
/// `std`'s: each is held under a slot's `live` lock or for a step that takes
/// none of these, so no schedule loom explores leaves a thread waiting on one.
mod sync {
    #[cfg(all(loom, test))]
    pub(super) use loom::sync::atomic::AtomicBool;
    #[cfg(all(loom, test))]
    pub(super) use loom::sync::{Arc, Mutex, MutexGuard};

    #[cfg(not(all(loom, test)))]
    pub(super) use core::sync::atomic::AtomicBool;
    #[cfg(not(all(loom, test)))]
    pub(super) use std::sync::{Arc, Mutex, MutexGuard};

    /// Let another thread go on while this one waits for a lock it holds: a
    /// sleep of a millisecond, and under loom a yield, which is how loom lets
    /// the holder run.
    #[cfg(all(loom, test))]
    pub(super) fn pause() {
        loom::thread::yield_now();
    }

    /// Let another thread go on while this one waits for a lock it holds: a
    /// sleep of a millisecond.
    #[cfg(not(all(loom, test)))]
    pub(super) fn pause() {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::time::{Duration, Instant};

use sce_forge_runtime::codec::SceCursor;
use sce_rust_metrics::{Counter, Eviction, Gauge, Metrics, PoolShed, RestoredFrom};
use sce_rust_runtime::journal::JournalEntry;
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::saved_state::{SavedInvokes, SavedState};
use sce_rust_runtime::Principal;
use sce_rust_session_store::{
    AcquireError, Actor, Append, Change, Conflict, Expiry, Fact, FactData, Failure, Held, Journal,
    JournalStore, LeaseError, MembershipStore, Read, SessionStore, Under, Write, WriteError,
    LATEST_MOMENT,
};

use crate::checkpoint::{self, Checkpoint, EndpointSection, LoadError, Unreleased};
use crate::commit::Committer;
use crate::endpoint::{
    Endpoint, EngineCall, Environment, SharedEndpoint, Transport, TransportFailure,
};
use crate::generated::envelope::Envelope;
use crate::generated::pattern_kind::PatternKind;
use crate::generated::payload_codec::PayloadCodec;
use crate::generated::rpc_status::RpcStatus;
use crate::health::Health;
use crate::inbound::ConfigError;
use crate::outbox::Outboxes;
use crate::peers::{instance_of, KeyFrom, Machine, Sessions};
use crate::router::{payload_text, within, RouterError, Standing};
use slot::{Door, Full, KeyLease};

mod follow;

pub use follow::{FOLLOW_PAGE, FOLLOW_WINDOW, JOURNAL_ORIGIN_TYPE};

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

    /// The entries of the event journal its runs made since the last take, in
    /// the order made, leaving none (docs/adr/0114, D1): the generated
    /// machine's `EngineState::take_journal`. The pool appends them by the
    /// write of the checkpoint that holds the state that made them, so an entry
    /// exists exactly when that state does. A machine that says no fact makes
    /// none.
    fn take_journal(&mut self) -> Vec<JournalEntry> {
        Vec::new()
    }
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
    /// The store keeps the sessions of another machine
    /// (`SessionStore::machine`, docs/adr/0114, D2): a session of this pool is
    /// this machine and its key.
    StoreOfAnotherMachine { machine: String, store: String },
    /// The renewal period is zero, or not below the lease's length.
    Renewal {
        renew_every_ms: u64,
        lease_ttl_ms: u64,
    },
    /// A session's router could not be built from the machine.
    Router(ConfigError),
    /// The machine is bound to an open machine (docs/adr/0113), whose members the pool keeps in
    /// its store and lets go when their leases end: it is pooled by [`Pool::with_members`].
    NoMembers { machine: String },
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
    /// An envelope for `key` shed at a bound of what the pool holds in memory
    /// (`sessions.max_live` or `sessions.max_inbox`, docs/adr/0109, D1): no
    /// session took it in. A request the host does not acknowledge by
    /// receipt is answered `Unavailable` instead of told here.
    Shed {
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
    /// The entry at `position` of the journal `journal`, which the machine follows
    /// (docs/adr/0114, D4), can be handed to no session, for `reason`: its schema is no kind of
    /// entry the build described, its data no value of its record, or its key unreadable. The
    /// reader passes it.
    Unfollowed {
        journal: String,
        position: u64,
        reason: String,
    },
    /// A checkpoint of the session `key` was not kept: the run it holds was made under the
    /// incarnation `incarnation` of a member that stands under `current` now, or under none
    /// (docs/adr/0113, D5). The session is restored from what the store keeps; what it took in
    /// since its last durable checkpoint is lost, and its senders carry it again.
    Fenced {
        key: String,
        incarnation: u64,
        current: Option<u64>,
    },
    /// An envelope for `key` that is not a request, dropped for the position of the journals the
    /// machine follows it carries in the field `sessions.position_from` names (docs/adr/0116,
    /// D5), for `reason`: it is no position, it is above the journal's horizon, which no answer
    /// told, or the pool reads no journal. A request is answered with a failure status instead.
    Unpositioned {
        key: String,
        event: String,
        reason: String,
    },
    /// The envelope numbered `number` in `client`'s outbox, appended at `at_ms`, was kept longer
    /// than the binding's `retain_ms` without `client` acknowledging it, and the store let it go
    /// (docs/adr/0109, D10). `sender`, the session that sent it, is told at its next run, as
    /// SCE_MESH.md section 16.7 row 15; `None` for an answer the pool made in a session's place.
    OutboxExpired {
        client: String,
        number: u64,
        at_ms: u64,
        sender: Option<String>,
    },
}

/// What [`Pool::stop`] did with each session it held in memory.
#[derive(Debug, Default)]
pub struct StopReport {
    /// The sessions saved as a clean checkpoint and dropped, their leases released: their restore
    /// raises no `restored` (SCE_MESH.md section 16.7 row 17).
    pub saved: Vec<String>,
    /// The sessions not saved by the stop, each with the reason: each restores from its last
    /// durable checkpoint, which is not clean, and raises `restored` as after a crash.
    pub not_saved: Vec<(String, String)>,
}

/// The transport of a session's endpoint: the pool's own, through which every
/// session sends, which notes the request each session sends so that its
/// reply reaches the session that waits for it.
pub struct SessionTransport<T> {
    key: String,
    inner: Arc<Mutex<T>>,
    replies: Replies,
    /// The outboxes of the clients whose binding is durable (docs/adr/0109, D10).
    outboxes: Option<Arc<Outboxes>>,
}

impl<T: Transport> Transport for SessionTransport<T> {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if let Some(wire_id) = request_id(bytes) {
            lock(&self.replies).insert(wire_id, self.key.clone());
        }
        // A client with a durable outbox has every envelope a checkpoint released in its outbox
        // already, appended by the write that made that checkpoint durable. What else the
        // endpoint sends it (a reply a duplicate request is answered with) is a copy of one its
        // outbox holds until it acknowledges it, or one it took in before it did.
        if self
            .outboxes
            .as_ref()
            .is_some_and(|outboxes| outboxes.keeps(peer))
        {
            return Ok(());
        }
        lock(&self.inner).transmit(peer, bytes)
    }
}

/// What became of an envelope handed to the pool, as a host that acknowledges
/// what its binding carried reads it: SCE_MESH.md section 10.11 acknowledges
/// nothing before a checkpoint that holds it is durable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    /// Not settled: its session has not taken it in yet, or took it in and
    /// holds it in memory, the checkpoint that holds it not written.
    Pending,
    /// Settled, and the host may acknowledge it: a durable checkpoint holds
    /// what its session made of it, or the pool answered or dropped it for
    /// good without a session (a request answered with a failure status, bytes
    /// that are not an envelope, a reply to no request, an envelope no key can
    /// be read from), which receiving it again would not change.
    Settled,
    /// Lost: the session that took it in was dropped before a checkpoint
    /// holding it was durable (its lease was lost, or its end was not
    /// written), or it was dropped unanswered while this process did not serve
    /// its key. The host does not acknowledge it, and its sender carries it
    /// again.
    Lost,
    /// Fenced: it was taken in by its edge under an incarnation of its instance's membership that
    /// the store no longer holds, the member standing under `current` (docs/adr/0113, D4 and D5),
    /// and nothing of it was taken in. The host answers its exchange `412`, and the instance
    /// learns that what it waited on has an unknown outcome.
    Fenced { current: u64 },
}

/// The receipt of one envelope handed to the pool
/// ([`Pool::receive_with_receipt`]): what became of it, settled by the run of
/// the session it names. Its clones read the same fate.
#[derive(Debug, Clone)]
pub struct Receipt(Arc<Mutex<Fate>>);

impl Receipt {
    fn new(fate: Fate) -> Self {
        Self(Arc::new(Mutex::new(fate)))
    }

    /// What became of the envelope so far. Once it is no longer
    /// [`Fate::Pending`] it does not change.
    pub fn fate(&self) -> Fate {
        *lock(&self.0)
    }

    /// Settle a pending envelope as `fate`; a settled or lost one stays as it
    /// is.
    fn settle(&self, fate: Fate) {
        let mut held = lock(&self.0);
        if *held == Fate::Pending {
            *held = fate;
        }
    }
}

/// The session each request a session sent waits in, by the request's wire
/// `invoke_id`.
type Replies = Arc<Mutex<BTreeMap<[u8; 16], String>>>;

/// A session's slot as the table and each call that acts on it share it.
type SharedSlot<M, T, E> = sync::Arc<Slot<M, T, E>>;

/// Every session the pool holds, in memory or about to be, by key.
type Table<M, T, E> = BTreeMap<String, SharedSlot<M, T, E>>;

/// One input of a session's inbox, applied by the session's owner-thread
/// operation of the same name.
enum Input {
    Envelope {
        peer: String,
        bytes: Vec<u8>,
        receipt: Receipt,
        /// For an instance of an open machine judged by its completion records, the incarnation
        /// its edge took it in under and the store's answer for its member then (docs/adr/0113,
        /// D5).
        judged: Option<(u64, Standing)>,
        /// For an instance whose incarnation acts for a person, the person, as the edge read the
        /// delegation its store holds (docs/adr/0116, D1 and D3); boxed, as an event's person is,
        /// so that an envelope said for nobody costs its inbox one pointer.
        acting: Option<Box<Principal>>,
        /// The position of the journals the machine follows that the envelope waits for the
        /// pool's reader to pass before its session takes it (docs/adr/0116, D5); `None` for one
        /// that carries none, or one the reader had passed when it was received.
        after: Option<u64>,
    },
    /// The store's latest answer for the member whose instance is the peer named
    /// (docs/adr/0113, D5).
    Stand(String, Standing),
    /// The membership of the instance named ended (docs/adr/0113, D4): its binding and what the
    /// router keeps of it are let go.
    EndMember(String),
    Ready(String),
    NotReady(String),
    Tick,
    /// Load the session: one of its waits came due while it was out of memory.
    Wake,
    /// An expiry noted an envelope the session sent that its client never acknowledged
    /// (docs/adr/0109, D10): read the session's notes and tell its machine.
    Notes,
    /// An entry of a journal the machine follows (docs/adr/0114, D4; module `follow`); boxed, so
    /// that an inbox's inputs stay the size of an envelope.
    Followed(Box<follow::Followed>),
}

impl Input {
    /// Whether the input counts toward the inbox's bound as an envelope does: an envelope, or an
    /// entry of a followed journal, which waits as an envelope does so that a follower that cannot
    /// keep up stops the reader at the bound (docs/adr/0114, D4), unless it was told past the
    /// bound to a session whose envelopes wait for the reader (docs/adr/0116, D5;
    /// `Followed::counted`). What the inbox's [`Door`] counts as `bounded` is decided here.
    fn waits_as_an_envelope(&self) -> bool {
        match self {
            Input::Envelope { .. } => true,
            Input::Followed(entry) => entry.counted,
            _ => false,
        }
    }

    /// Whether the input is an envelope that waits for the pool's reader to pass a position above
    /// `mark` (docs/adr/0116, D5).
    fn waits_on_reader(&self, mark: u64) -> bool {
        matches!(self, Input::Envelope { after: Some(after), .. } if *after > mark)
    }
}

/// A session's inbox: what it holds, in the order it took it, and its
/// [`Door`], which decides what it takes and counts it, the two moved
/// together here alone. Closed once the session left the table: nothing is
/// posted to it after that, since posts and removals both hold the table.
struct Inbox {
    queue: VecDeque<Input>,
    door: Door,
}

impl Inbox {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            door: Door::open(),
        }
    }

    /// Put `input`, the envelope or followed journal entry [`slot::admission`]
    /// admitted, `door` being the inbox's door after it.
    fn admitted(&mut self, door: Door, input: Input) {
        self.door = door;
        self.queue.push_back(input);
    }

    /// Put `input`, an input the inbox's bound does not count ([`Door::tell`]), in
    /// the inbox: one the pool makes itself, or an entry of a followed journal
    /// handed past the bound (docs/adr/0116, D5).
    fn tell(&mut self, input: Input) {
        self.door.tell();
        self.queue.push_back(input);
    }

    /// Take the oldest input the session's owner may apply while the pool's reader has passed
    /// `mark` (docs/adr/0116, D5): an envelope waits while it, or an envelope before it, carries a
    /// position above `mark`, so that a client's envelopes keep their order; every other input,
    /// an entry of a followed journal among them, is taken past it, so that the session takes in
    /// what the reader must hand it for the mark to rise.
    fn take(&mut self, mark: u64) -> Option<Input> {
        let index = self.takeable(mark)?;
        let input = self.queue.remove(index)?;
        self.door.took(input.waits_as_an_envelope());
        Some(input)
    }

    /// Where the oldest input [`Self::take`] takes at `mark` is in the queue.
    fn takeable(&self, mark: u64) -> Option<usize> {
        let mut waiting = false;
        self.queue.iter().position(|input| match input {
            Input::Envelope { .. } => {
                waiting |= input.waits_on_reader(mark);
                !waiting
            }
            _ => true,
        })
    }

    /// Whether an envelope it holds waits for the reader to pass a position above `mark`.
    fn waits_on_reader(&self, mark: u64) -> bool {
        self.queue.iter().any(|input| input.waits_on_reader(mark))
    }

    fn is_empty(&self) -> bool {
        self.door.is_empty()
    }

    /// Close the inbox for its session to leave the table, when it holds
    /// nothing ([`Door::retire`]): whether it is closed, now or before.
    fn retire(&mut self) -> bool {
        self.door.retire()
    }

    /// Close the inbox whatever it holds, and hand back what it held for the
    /// pool to answer ([`Door::close`]).
    fn close(&mut self) -> VecDeque<Input> {
        self.door.close();
        core::mem::take(&mut self.queue)
    }
}

/// One key's session: its inbox, which any thread posts to, its machine,
/// which the thread that runs it owns for as long as it runs it, and its
/// lease, which the thread that renews leases reads too.
struct Slot<M, T, E> {
    key: String,
    /// What names this slot apart from every other the pool made, before or since
    /// (`Pool::new_slot`): a removal takes the slot the table holds for the key out only when
    /// it is this one, and the reader of the journals hands an entry of a key into the slot an
    /// earlier one went to only while the table still holds that slot (module `follow`).
    serial: u64,
    /// The entry the store indexes the key under ([`SessionStore::indexed`]):
    /// what its listing of dues names the session by.
    indexed: String,
    inbox: sync::Mutex<Inbox>,
    live: sync::Mutex<Option<Live<M, T, E>>>,
    lease: sync::Mutex<KeyLease>,
}

impl<M, T, E> Slot<M, T, E> {
    fn new(key: &str, serial: u64, indexed: String) -> Self {
        Self {
            key: key.to_string(),
            serial,
            indexed,
            inbox: sync::Mutex::new(Inbox::new()),
            live: sync::Mutex::new(None),
            lease: sync::Mutex::new(KeyLease::unheld()),
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
    /// The earliest wait of the machine's state last saved. Not the checkpoint's due, which the
    /// store keeps: what the endpoint held when it was sealed (a send not yet released) is due at
    /// once for a process that restores it, and this one released it when the write answered.
    earliest_due: Option<u64>,
    /// When it last had an event: an envelope, or a macrostep a wait ran.
    last_active_ms: u64,
    /// Whether a macrostep ran since the state was last saved.
    dirty: bool,
    /// The receipts of the envelopes taken in since the last durable
    /// checkpoint, settled by the next one.
    unsaved: Vec<Receipt>,
    /// Started for a key the store held nothing for, and in its first run: it
    /// is written only if that run does not end in a top-level final state
    /// (docs/adr/0109, D1), so a key a document refuses costs the store
    /// nothing.
    provisional: bool,
    /// The last of the session's notes its machine was told (docs/adr/0109,
    /// D10): the next write takes every note up to it, in the transaction that
    /// keeps what the machine made of them.
    notes_through: u64,
    /// The entries of the event journal its runs made that no durable write
    /// appended yet (docs/adr/0114, D1), each with what it was decided having
    /// read (docs/adr/0116, D5): the next write of the session, which keeps the
    /// state that made them, appends them.
    facts: Vec<Said>,
    /// For each journal the session follows, the position of the last entry it
    /// took in (docs/adr/0114, D4): written with what it did, in the checkpoint.
    followed: BTreeMap<String, u64>,
    /// The incarnations the run since the last durable checkpoint was made under, each with its
    /// member's name (docs/adr/0113, D5): every checkpoint sealed since names them, and the write
    /// that keeps one keeps nothing once the store no longer holds one of them.
    under: BTreeSet<(String, u64)>,
    /// A write of it was refused for an incarnation the store no longer holds: the session is
    /// let go and restored from what the store keeps (docs/adr/0113, D5).
    fenced: bool,
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
    /// A write of it was refused for an incarnation the store no longer holds (docs/adr/0113,
    /// D5): it is restored from what the store keeps.
    Fenced,
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

impl Full {
    /// Why the envelope was shed, as the pool reports it (docs/adr/0109, D16).
    fn shed(self) -> PoolShed {
        match self {
            Full::Inbox => PoolShed::MaxInbox,
            Full::Live => PoolShed::MaxLive,
            Full::Stopping => PoolShed::Stopping,
        }
    }

    /// The reason a shed envelope is told or answered with.
    fn reason(self, sessions: &Sessions) -> String {
        match self {
            Full::Stopping => "the pool is stopping, and admits nothing".to_string(),
            Full::Inbox => alloc::format!(
                "its session's inbox holds {} envelopes, the most it takes",
                sessions.max_inbox
            ),
            Full::Live => alloc::format!(
                "the pool holds {} sessions in memory, the most it holds, and none is \
                 quiet enough to save and drop",
                sessions.max_live
            ),
        }
    }
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

/// The instances of open machines the pool's sessions hold (docs/adr/0113, D6): the sessions
/// whose routers bind each, which its readiness and the end of its membership reach and no
/// other, and the entry the store keeps each member under, by which an ended one is named.
#[derive(Default)]
struct Known {
    /// The sessions whose routers bind each instance, by the instance's name.
    holders: BTreeMap<String, BTreeSet<String>>,
    /// The instances each session's router binds, by the session's key.
    held: BTreeMap<String, BTreeSet<String>>,
    /// Each instance's name, by the entry the store keeps its member under
    /// ([`MembershipStore::member_entry`]).
    entries: BTreeMap<String, String>,
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
    /// The committer every write of a session goes through (docs/adr/0109,
    /// D15), and every acknowledgement of a durable client's outbox.
    committer: Arc<Committer>,
    transport: Arc<Mutex<T>>,
    environment: Mutex<E>,
    table: sync::Mutex<Table<F::Machine, T, E>>,
    /// The peers whose links are ready, which a session's endpoint is told
    /// when it is built.
    ready: Mutex<BTreeSet<String>>,
    replies: Replies,
    /// The keys another process held when this one asked, until when, each
    /// by the entry the store indexes it under.
    deferred: Mutex<BTreeMap<String, u64>>,
    errors: Mutex<Vec<PoolError>>,
    /// The outboxes of the clients whose `https` accepting binding is durable
    /// (docs/adr/0109, D10), which the server answers their exchanges from.
    outboxes: Option<Arc<Outboxes>>,
    /// The members of the open machines the machine is bound to, kept in the pool's own store
    /// (docs/adr/0113, D4); `None` when it is bound to none.
    members: Option<Arc<dyn MembershipStore>>,
    /// The instances the sessions hold (docs/adr/0113, D6).
    known: Mutex<Known>,
    /// Set by [`Pool::stop`] with the table held, and read with it held by
    /// every admit: nothing is admitted from then on. Read without it only by
    /// [`Pool::health`], which reports it.
    stopping: sync::AtomicBool,
    /// Set once [`Pool::stop`] ended.
    stopped: sync::AtomicBool,
    /// The serial of the last slot the pool made (`Pool::new_slot`).
    slots_made: AtomicU64,
    /// The store the journals the machine follows are read from
    /// ([`Pool::follow_from`], docs/adr/0114, D4), and what its reader holds.
    journal: Option<Arc<dyn JournalStore>>,
    reader: sync::Mutex<follow::Reader>,
    /// The reader's mark as the store last kept it (docs/adr/0116, D5): every entry at or below it
    /// was taken in by a durable checkpoint of its session. An envelope that carries a position
    /// at or below it is taken at once; one above it waits. Only rises.
    read_mark: AtomicU64,
    /// The envelopes that wait for the reader, each by the position it carries and the key of
    /// its session, which [`Pool::follow`] answers once the mark passed the position.
    awaiting: Mutex<BTreeSet<(u64, String)>>,
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
    /// Each session's checkpoint is written to `store` under the key's lease,
    /// through a committer of the pool's own over `store` (docs/adr/0109,
    /// D15), whose leader waits `sessions.commit_delay_us` before it takes a
    /// batch.
    pub fn new(
        machine: Machine,
        config: PoolConfig,
        factory: F,
        store: Arc<dyn SessionStore>,
        transport: T,
        environment: E,
    ) -> Result<Self, PoolConfigError> {
        Self::observed(
            machine,
            config,
            factory,
            store,
            transport,
            environment,
            sce_rust_metrics::none(),
        )
    }

    /// [`Self::new`], reporting what the pool, its committer and its
    /// durable clients' outboxes do to `metrics` (docs/adr/0109, D16).
    ///
    /// Refused for a machine bound to an open machine, whose members the pool keeps:
    /// [`Self::with_members`] pools it.
    pub fn observed(
        machine: Machine,
        config: PoolConfig,
        factory: F,
        store: Arc<dyn SessionStore>,
        transport: T,
        environment: E,
        metrics: Arc<dyn Metrics>,
    ) -> Result<Self, PoolConfigError> {
        if !machine.open_peers.is_empty() {
            return Err(PoolConfigError::NoMembers {
                machine: machine.name.to_string(),
            });
        }
        Self::assemble(
            machine,
            config,
            factory,
            (store, None),
            transport,
            environment,
            metrics,
        )
    }

    /// [`Self::new`] for a machine that may be bound to open machines (docs/adr/0113): their
    /// members are kept in `store`, the one the sessions are kept in, so that a write made under
    /// an incarnation is judged in its own transaction (D5), and the pool lets go of what its
    /// sessions hold of a member whose lease ended ([`Self::end_members`]).
    pub fn with_members(
        machine: Machine,
        config: PoolConfig,
        factory: F,
        store: Arc<dyn MembershipStore>,
        transport: T,
        environment: E,
    ) -> Result<Self, PoolConfigError> {
        Self::observed_with_members(
            machine,
            config,
            factory,
            store,
            transport,
            environment,
            sce_rust_metrics::none(),
        )
    }

    /// [`Self::with_members`], reporting to `metrics` as [`Self::observed`] does.
    pub fn observed_with_members(
        machine: Machine,
        config: PoolConfig,
        factory: F,
        store: Arc<dyn MembershipStore>,
        transport: T,
        environment: E,
        metrics: Arc<dyn Metrics>,
    ) -> Result<Self, PoolConfigError> {
        let sessions: Arc<dyn SessionStore> = store.clone();
        Self::assemble(
            machine,
            config,
            factory,
            (sessions, Some(store)),
            transport,
            environment,
            metrics,
        )
    }

    /// The pool of `machine` over `stores`: the store its sessions are kept in, and the same
    /// store as the one that keeps members, when it does.
    fn assemble(
        machine: Machine,
        config: PoolConfig,
        factory: F,
        stores: (Arc<dyn SessionStore>, Option<Arc<dyn MembershipStore>>),
        transport: T,
        environment: E,
        metrics: Arc<dyn Metrics>,
    ) -> Result<Self, PoolConfigError> {
        let (store, members) = stores;
        let Some(sessions) = machine.sessions else {
            return Err(PoolConfigError::NotPooled {
                machine: machine.name.to_string(),
            });
        };
        // docs/adr/0114, D2: a store whose handle keeps a named machine's
        // sessions keeps this machine's, or another's would be written.
        if !store.machine().is_empty() && store.machine() != machine.name {
            return Err(PoolConfigError::StoreOfAnotherMachine {
                machine: machine.name.to_string(),
                store: store.machine().to_string(),
            });
        }
        if config.renew_every_ms == 0 || config.renew_every_ms >= config.lease_ttl_ms {
            return Err(PoolConfigError::Renewal {
                renew_every_ms: config.renew_every_ms,
                lease_ttl_ms: config.lease_ttl_ms,
            });
        }
        machine.router().map_err(PoolConfigError::Router)?;
        let committer = Arc::new(Committer::observed(
            Arc::clone(&store),
            Duration::from_micros(sessions.commit_delay_us),
            metrics,
        ));
        let outboxes = Outboxes::of_machine(&machine, Arc::clone(&committer));
        Ok(Self {
            machine,
            sessions,
            config,
            factory,
            store,
            committer,
            transport: Arc::new(Mutex::new(transport)),
            environment: Mutex::new(environment),
            table: sync::Mutex::new(BTreeMap::new()),
            ready: Mutex::new(BTreeSet::new()),
            replies: Arc::new(Mutex::new(BTreeMap::new())),
            deferred: Mutex::new(BTreeMap::new()),
            errors: Mutex::new(Vec::new()),
            outboxes,
            members,
            known: Mutex::new(Known::default()),
            stopping: sync::AtomicBool::new(false),
            stopped: sync::AtomicBool::new(false),
            slots_made: AtomicU64::new(0),
            journal: None,
            reader: sync::Mutex::new(follow::Reader::new()),
            read_mark: AtomicU64::new(0),
            awaiting: Mutex::new(BTreeSet::new()),
        })
    }

    /// What the pool says of itself to the program's readiness and liveness
    /// checks (docs/adr/0109, D16): it accepts until [`Self::stop`] begins, is
    /// stopping until that call ends, and its store is reachable when it
    /// answers a read now ([`Committer::store_answers`]). The read blocks for
    /// as long as the store does, so a program on an asynchronous runtime
    /// calls this from a thread that may block.
    pub fn health(&self) -> Health {
        let stopping = self.stopping.load(Ordering::SeqCst);
        Health {
            accepting: !stopping,
            stopping: stopping && !self.stopped.load(Ordering::SeqCst),
            store_reachable: self.committer.store_answers(),
        }
    }

    /// What the pool reports to.
    fn metrics(&self) -> &dyn Metrics {
        self.committer.metrics().as_ref()
    }

    /// The machine the pool serves.
    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    /// The outboxes of the clients whose `https` accepting binding declares `receipt: durable`
    /// (docs/adr/0109, D10; SCE_MESH.md section 10.11), kept in the pool's store; `None` when it
    /// has none. The server answers those clients' exchanges from them
    /// (`HttpsServer::keep_outboxes`).
    pub fn outboxes(&self) -> Option<&Arc<Outboxes>> {
        self.outboxes.as_ref()
    }

    /// The committer the pool writes its sessions and its clients'
    /// acknowledgements through (docs/adr/0109, D15): what a host reads the
    /// writes waiting for the next commit from ([`Committer::waiting`]).
    pub fn committer(&self) -> &Arc<Committer> {
        &self.committer
    }

    /// Whether the session `key` is in memory. Waits for a run of it that is
    /// going on.
    pub fn is_live(&self, key: &str) -> bool {
        let slot = hold(&self.table).get(key).cloned();
        let mut loaded = false;
        if let Some(slot) = slot {
            loaded = hold(&slot.live).is_some();
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
    ///
    /// The pool holds at most `sessions.max_live` sessions in memory and
    /// `sessions.max_inbox` envelopes in one session's inbox (docs/adr/0109,
    /// D1). An envelope for a key not in memory while the pool holds its
    /// most, and none of its sessions can be saved and dropped to make room,
    /// or one past its session's inbox, is shed: a request is answered
    /// `RpcStatus::Unavailable`, which tells its client to try again, and
    /// anything else is dropped and told to the host ([`PoolError::Shed`]).
    pub fn receive(&self, peer: &str, bytes: Vec<u8>) -> Option<String> {
        self.receive_as(peer, bytes, false, None, None).0
    }

    /// [`Self::receive`], with the envelope's [`Receipt`]: the host reads it
    /// once it has run the session the key names, and acknowledges the
    /// envelope to its sender only when it is [`Fate::Settled`]. An envelope
    /// that names no session is settled at once, unless the failure reply to
    /// its request could not be sent.
    ///
    /// An envelope shed at a bound of what the pool holds is answered by
    /// nothing and is [`Fate::Lost`]: the binding refuses it to its sender,
    /// who carries it again (the `https` binding's `503`), so no reply tells
    /// a client of a request it will send again and the pool then serves.
    pub fn receive_with_receipt(&self, peer: &str, bytes: Vec<u8>) -> (Option<String>, Receipt) {
        self.receive_as(peer, bytes, true, None, None)
    }

    /// [`Self::receive_with_receipt`] for envelope `bytes` of `peer`, an instance of an open
    /// machine whose binding receives durably (docs/adr/0113, D5): its edge took it in under the
    /// incarnation `made_under`, while the store held its member as `standing`
    /// (`MembershipStore::admit`). The session that runs it judges both again
    /// ([`crate::router::Router::receive_member`]), and the write that keeps what it made of it
    /// names the incarnation, so nothing of it is kept once the incarnation ended. One the
    /// session finds fenced is [`Fate::Fenced`]; one whose session binds as many instances as
    /// the deployment's `max_instances` is shed, [`Fate::Lost`] (D6).
    pub fn receive_member(
        &self,
        peer: &str,
        made_under: u64,
        standing: Standing,
        bytes: Vec<u8>,
    ) -> (Option<String>, Receipt) {
        self.receive_member_for(peer, made_under, standing, None, bytes)
    }

    /// [`Self::receive_member`] for an instance whose incarnation `made_under` acts for a person
    /// (docs/adr/0116, D1 and D3): `acting` is the person of the delegation the server's store
    /// holds for that incarnation, which the edge read where it took the envelope in
    /// (`Judgement::acting` of the `https` server), or `None` for one that acts for nobody. Each
    /// event the envelope raises in its session carries it, and a document reads it as
    /// `_x.principal` (`Endpoint::receive_member_for`).
    pub fn receive_member_for(
        &self,
        peer: &str,
        made_under: u64,
        standing: Standing,
        acting: Option<Principal>,
        bytes: Vec<u8>,
    ) -> (Option<String>, Receipt) {
        self.know(peer);
        self.receive_as(peer, bytes, true, Some((made_under, standing)), acting)
    }

    /// [`Self::receive_with_receipt`], where `acknowledged` says whether the
    /// host reads the receipt: a shed envelope is then lost rather than
    /// answered; `judged` and `acting` are what [`Self::receive_member_for`]
    /// was given.
    fn receive_as(
        &self,
        peer: &str,
        bytes: Vec<u8>,
        acknowledged: bool,
        judged: Option<(u64, Standing)>,
        acting: Option<Principal>,
    ) -> (Option<String>, Receipt) {
        let placed = {
            let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&bytes)) else {
                self.error(PoolError::Malformed {
                    peer: peer.to_string(),
                });
                return (None, Receipt::new(Fate::Settled));
            };
            self.place(peer, &envelope).map(|key| {
                // docs/adr/0116, D5: the position of the journals the machine follows that the
                // envelope carries, which it waits for the reader to pass.
                let after = self
                    .position_of(&envelope)
                    .map_err(|reason| Refusal {
                        status: RpcStatus::InvalidArgument,
                        reason,
                    })
                    .and_then(|position| self.waits_for(position))
                    .map_err(|refusal| {
                        (
                            request_of(&envelope),
                            envelope.event_type.to_string(),
                            refusal,
                        )
                    });
                (key, after)
            })
        };
        match placed {
            Ok((key, Err((request, event, refusal)))) => {
                // Refused as unavailable (the store did not answer the horizon), it is carried
                // again: lost to a host that reads the receipt, as a shed envelope is.
                let carried_again = refusal.status == RpcStatus::Unavailable;
                let fate = match request.filter(|_| !(carried_again && acknowledged)) {
                    Some(request) => self.refuse(peer, &request, refusal.status, &refusal.reason),
                    None => {
                        self.error(PoolError::Unpositioned {
                            key,
                            event,
                            reason: refusal.reason,
                        });
                        if carried_again {
                            Fate::Lost
                        } else {
                            Fate::Settled
                        }
                    }
                };
                (None, Receipt::new(fate))
            }
            Ok((key, Ok(after))) => {
                let receipt = Receipt::new(Fate::Pending);
                let input = Input::Envelope {
                    peer: peer.to_string(),
                    bytes,
                    receipt: receipt.clone(),
                    judged,
                    acting: acting.map(Box::new),
                    after,
                };
                match self.post_bounded(&key, input) {
                    Ok(()) => {
                        // Noted once it waits in its inbox, before the host runs its session: a
                        // mark that rose since it was read is read again by the run, and one that
                        // rises later answers the key (`Pool::follow`).
                        if let Some(after) = after {
                            lock(&self.awaiting).insert((after, key.clone()));
                        }
                        (Some(key), receipt)
                    }
                    Err((input, full)) => {
                        self.shed(&key, input, full, acknowledged);
                        (None, receipt)
                    }
                }
            }
            Err(Unplaced::Reply) => {
                self.error(PoolError::UnroutedReply {
                    peer: peer.to_string(),
                });
                (None, Receipt::new(Fate::Settled))
            }
            Err(Unplaced::Unkeyed {
                event,
                request,
                reason,
            }) => {
                let fate = match request {
                    Some(request) => self.refuse(
                        peer,
                        &request,
                        RpcStatus::InvalidArgument,
                        &alloc::format!("the request names no session: {reason}"),
                    ),
                    None => {
                        self.error(PoolError::Unkeyed {
                            peer: peer.to_string(),
                            event,
                            reason,
                        });
                        Fate::Settled
                    }
                };
                (None, Receipt::new(fate))
            }
        }
    }

    /// `peer`'s link became ready. Every session is told, in order with what
    /// its inbox holds; the keys answered are the sessions to run. Of an
    /// instance of an open machine, only the sessions whose routers bind it are
    /// told (docs/adr/0113, D6), and one that binds it later is told then.
    pub fn peer_ready(&self, peer: &str) -> Vec<String> {
        lock(&self.ready).insert(peer.to_string());
        if self.is_instance(peer) {
            self.know(peer);
            return self.tell_holders(peer, || Input::Ready(peer.to_string()));
        }
        self.broadcast(|| Input::Ready(peer.to_string()))
    }

    /// `peer`'s link stopped being ready. Every session is told, as
    /// [`Self::peer_ready`] tells it.
    pub fn peer_not_ready(&self, peer: &str) -> Vec<String> {
        let mut ready = lock(&self.ready);
        ready.remove(peer);
        if !self.is_instance(peer) {
            drop(ready);
            return self.broadcast(|| Input::NotReady(peer.to_string()));
        }
        {
            // An instance no session holds and whose link is not ready is noted no more: the end
            // of its membership, here or in another process, has nothing here to let go.
            let mut known = lock(&self.known);
            if known.holders.get(peer).is_none_or(BTreeSet::is_empty) {
                self.unnote(&mut known, peer);
            }
        }
        drop(ready);
        self.tell_holders(peer, || Input::NotReady(peer.to_string()))
    }

    /// `standing`, the store's latest answer for the member whose instance is `peer`
    /// (`MembershipStore::admit`), told to the sessions whose routers bind it (docs/adr/0113,
    /// D5 and D6): each drops the completion records below its mark, and all of them when it
    /// stands under a later incarnation. The keys answered are the sessions to run.
    pub fn stand(&self, peer: &str, standing: Standing) -> Vec<String> {
        self.tell_holders(peer, || Input::Stand(peer.to_string(), standing))
    }

    /// Let go of the members whose leases ended (docs/adr/0113, D4): the store removes at most a
    /// batch of them ([`MembershipStore::end_expired`]), and each session whose router binds one
    /// unbinds it, dropping what it kept of it (its completion records, its place among the
    /// deployment's `max_instances`). Its outbox ends with it, what it still owes let go as
    /// `retain_ms` lets an entry go and told to the session that sent it (SCE_MESH.md section
    /// 16.7 row 15). The keys answered are the sessions to run. The host calls this on its clock
    /// loop's schedule, as it calls [`Self::expire`]. A stopping pool ends no member: the store
    /// keeps each, and its outbox, for a process that serves the sessions binding it.
    pub fn end_members(&self) -> Vec<String> {
        let Some(members) = &self.members else {
            return Vec::new();
        };
        // Read before the store ends anything: a stopping pool tells no session, so an end it
        // made would reach none of the sessions it saves.
        if self.stopping.load(Ordering::SeqCst) {
            return Vec::new();
        }
        let ended = match members.end_expired(self.wall_now(), WAKE_BATCH) {
            Ok(ended) => ended,
            Err(failure) => {
                self.error(PoolError::Store { key: None, failure });
                return Vec::new();
            }
        };
        let mut keys = BTreeSet::new();
        let mut senders = BTreeSet::new();
        for ended in ended {
            let (instance, holders) = {
                let mut known = lock(&self.known);
                let Some(instance) = known.entries.remove(&ended.member) else {
                    // No session here holds it: its outbox, named by the entry a store that keeps
                    // names as they are given keeps it under, ends all the same.
                    drop(known);
                    self.end_outbox(&ended.member, &mut senders);
                    continue;
                };
                let holders = known.holders.remove(&instance).unwrap_or_default();
                for key in &holders {
                    if let Some(held) = known.held.get_mut(key) {
                        held.remove(&instance);
                    }
                }
                (instance, holders)
            };
            lock(&self.ready).remove(&instance);
            {
                let table = hold(&self.table);
                for key in holders {
                    if let Some(slot) = table.get(&key) {
                        hold(&slot.inbox).tell(Input::EndMember(instance.clone()));
                        keys.insert(key);
                    }
                }
            }
            self.end_outbox(&instance, &mut senders);
        }
        keys.extend(self.tell_notes(senders));
        keys.into_iter().collect()
    }

    /// Whether `peer` names an instance of an open machine the machine is bound to.
    fn is_instance(&self, peer: &str) -> bool {
        instance_of(peer).is_some_and(|(machine, _)| self.machine.open_peer(machine).is_some())
    }

    /// Note the instance `peer` by the entry the store keeps its member under, so that the end
    /// of its membership finds it ([`Self::end_members`]).
    fn know(&self, peer: &str) {
        let Some(members) = &self.members else {
            return;
        };
        if !self.is_instance(peer) {
            return;
        }
        let entry = members.member_entry(peer);
        lock(&self.known)
            .entries
            .entry(entry)
            .or_insert_with(|| peer.to_string());
    }

    /// Put an input made by `input` in the inbox of every session in the table whose router binds
    /// the instance `peer` (docs/adr/0113, D6), and answer their keys.
    fn tell_holders(&self, peer: &str, input: impl Fn() -> Input) -> Vec<String> {
        let holders: Vec<String> = lock(&self.known)
            .holders
            .get(peer)
            .map(|keys| keys.iter().cloned().collect())
            .unwrap_or_default();
        let table = hold(&self.table);
        // A stopping pool admits no tell, as it admits no envelope. Read with the table held, as
        // `Self::stop` sets it: a tell that found the pool running is in its session's inbox
        // before the stop drains it, so it is applied before the session is saved.
        if self.stopping.load(Ordering::SeqCst) {
            return Vec::new();
        }
        let mut keys = Vec::new();
        for key in holders {
            if let Some(slot) = table.get(&key) {
                hold(&slot.inbox).tell(input());
                keys.push(key);
            }
        }
        keys
    }

    /// The session `key`'s router bound the instances `bound` (docs/adr/0113, D6): it holds
    /// them from now on, and is told the readiness of each that is ready.
    fn learn(&self, key: &str, endpoint: &mut Endpoint<SessionTransport<T>, E>) {
        let bound = endpoint.take_bound();
        if bound.is_empty() {
            return;
        }
        for instance in &bound {
            self.know(instance);
        }
        {
            let mut known = lock(&self.known);
            for instance in &bound {
                known
                    .holders
                    .entry(instance.clone())
                    .or_default()
                    .insert(key.to_string());
                known
                    .held
                    .entry(key.to_string())
                    .or_default()
                    .insert(instance.clone());
            }
        }
        let ready: Vec<String> = {
            let ready = lock(&self.ready);
            bound
                .into_iter()
                .filter(|instance| ready.contains(instance))
                .collect()
        };
        for instance in ready {
            endpoint.peer_ready(&instance);
        }
    }

    /// The session `key` left memory: it holds no instance until it is built again
    /// (docs/adr/0113, D6), and no request it sent waits.
    fn forget_session(&self, key: &str) {
        self.forget_replies(key);
        let ready = lock(&self.ready);
        let mut known = lock(&self.known);
        let Some(held) = known.held.remove(key) else {
            return;
        };
        for instance in held {
            let unheld = known.holders.get_mut(&instance).is_some_and(|holders| {
                holders.remove(key);
                holders.is_empty()
            });
            // One no session holds and whose link is not ready is noted no more, as
            // `Self::peer_not_ready` lets it go.
            if unheld && !ready.contains(&instance) {
                self.unnote(&mut known, &instance);
            }
        }
    }

    /// Note the instance `peer` no more in `known`.
    fn unnote(&self, known: &mut Known, peer: &str) {
        known.holders.remove(peer);
        if let Some(members) = &self.members {
            known.entries.remove(&members.member_entry(peer));
        }
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
        let Some(slot) = hold(&self.table).get(key).cloned() else {
            return;
        };
        let mut live = hold(&slot.live);
        loop {
            if live.is_none() {
                if self.retire_if_idle(&slot) {
                    return;
                }
                match self.load(&slot) {
                    Ok(loaded) => {
                        *live = Some(loaded);
                        self.metrics().adjust(Gauge::LiveSessions, 1);
                    }
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
                    if let Some(mut finished) = live.take() {
                        self.metrics().adjust(Gauge::LiveSessions, -1);
                        self.finish(&slot, &mut finished);
                    }
                }
                Served::Lost(reason) => {
                    self.lose(&slot, &mut live, reason);
                    return;
                }
                // docs/adr/0113, D5: nothing of the run since the last durable checkpoint is
                // kept, and the session goes on from what the store keeps: it is let go with
                // its lease, and loaded again for what its inbox holds.
                Served::Fenced => {
                    if let Some(mut fenced) = live.take() {
                        self.metrics().adjust(Gauge::LiveSessions, -1);
                        self.let_go(&mut fenced);
                    }
                    self.forget_session(&slot.key);
                    self.release(&slot);
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
        if self.stopping.load(Ordering::SeqCst) {
            return Vec::new();
        }
        let now = self.wall_now();
        let mut keys = Vec::new();
        for slot in self.slots() {
            let Some(live) = try_hold(&slot.live) else {
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
        let (in_memory, room) = {
            let table = hold(&self.table);
            let in_memory: BTreeSet<String> =
                table.values().map(|slot| slot.indexed.clone()).collect();
            // The pool holds at most `max_live` sessions (docs/adr/0109, D1):
            // the rest come due again at a later call, once room is made.
            let room = slot::room(table.len(), self.sessions.max_live);
            (in_memory, room)
        };
        // The store lists each session by the entry it indexes it under; a
        // store that keeps each key as a keyed digest reads the key from the
        // session it holds (docs/adr/0109, D8). Named outside the table's
        // lock, since naming one reads the store.
        let mut named = Vec::new();
        for listed in listed {
            if named.len() >= room {
                break;
            }
            let held_elsewhere = deferred.get(&listed.key).is_some_and(|until| *until > now);
            if in_memory.contains(&listed.key) || held_elsewhere {
                continue;
            }
            match self.store.key_of(&listed.key) {
                Ok(Some(key)) => named.push((key, listed.key)),
                // Removed since it was listed: nothing comes due.
                Ok(None) => {}
                Err(failure) => self.error(PoolError::Store { key: None, failure }),
            }
        }
        let mut table = hold(&self.table);
        // Read again with the table held, as `Self::stop` sets it: a stop
        // decided while the store was read admits no session named here.
        if self.stopping.load(Ordering::SeqCst) {
            return Vec::new();
        }
        for (key, indexed) in named {
            if table.contains_key(&key) {
                continue;
            }
            if slot::room(table.len(), self.sessions.max_live) == 0 {
                break;
            }
            let slot = self.new_slot(&key, indexed);
            hold(&slot.inbox).tell(Input::Wake);
            table.insert(key.clone(), slot);
            keys.push(key);
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
            let mut lease = hold(&slot.lease);
            let Some(fence) = lease.renewal_due(now, self.config.renew_every_ms) else {
                continue;
            };
            match self
                .store
                .renew(&slot.key, fence, self.config.lease_ttl_ms, now)
            {
                Ok(_) => lease.renewed(now),
                Err(LeaseError::Lost(why)) => {
                    lease.lose();
                    self.metrics().count(Counter::LeaseRenewalsRefused, 1);
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
            let Some(mut live) = try_hold(&slot.live) else {
                continue;
            };
            let idle = live.as_ref().is_some_and(|session| {
                now >= session.last_active_ms.saturating_add(self.sessions.idle_ms)
                    && self.is_quiet(&slot, session)
            });
            if idle && self.evict(&slot, &mut live, now, Eviction::Idle) {
                evicted.push(slot.key.clone());
            }
        }
        evicted
    }

    /// Save `slot`'s session, which holds nothing pending, as a clean
    /// checkpoint, drop it from memory and release its lease, reporting it
    /// evicted for `why` (docs/adr/0109, D16). Whether it was dropped: one
    /// whose save failed stays, and so does one an input reached while it was
    /// saved.
    fn evict(
        &self,
        slot: &SharedSlot<F::Machine, T, E>,
        live: &mut sync::MutexGuard<'_, Option<Live<F::Machine, T, E>>>,
        now: u64,
        why: Eviction,
    ) -> bool {
        let Some(session) = live.as_mut() else {
            return false;
        };
        match self.checkpoint(slot, session, now, Take::Clean) {
            Err(reason) => {
                self.lose(slot, live, reason);
                return false;
            }
            // Not saved: dropped, it would be lost.
            Ok(false) => return false,
            Ok(true) => {}
        }
        let removed = {
            let mut table = hold(&self.table);
            let retired = hold(&slot.inbox).retire();
            if retired {
                remove_slot(&mut table, slot);
            }
            retired
        };
        if !removed {
            // An input arrived while the clean checkpoint was written, and
            // the endpoint takes it in after that checkpoint: one that is
            // not clean replaces it now, so that a restore after a crash
            // still tells the peers an envelope may have been lost.
            session.dirty = true;
            if let Err(reason) = self.checkpoint(slot, session, now, Take::IfChanged) {
                self.lose(slot, live, reason);
            }
            return false;
        }
        **live = None;
        self.metrics().adjust(Gauge::LiveSessions, -1);
        self.metrics().count(Counter::Evictions { reason: why }, 1);
        self.forget_session(&slot.key);
        self.release(slot);
        true
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
            if let Some(renewal) = hold(&slot.lease).next_renewal_ms(self.config.renew_every_ms) {
                at(renewal);
            }
            let Some(live) = try_hold(&slot.live) else {
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
                let table = hold(&self.table);
                let in_memory: BTreeSet<&str> =
                    table.values().map(|slot| slot.indexed.as_str()).collect();
                let deferred = lock(&self.deferred);
                let first = listed
                    .iter()
                    .filter(|listed| !in_memory.contains(listed.key.as_str()))
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
        // docs/adr/0109, D10: an outbox entry read in this process expires at
        // its moment, which [`Self::expire`] then acts on.
        if let Some(expiry) = self
            .outboxes
            .as_ref()
            .and_then(|outboxes| outboxes.next_expiry_ms())
        {
            at(expiry);
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
        match self.sessions.key_from {
            KeyFrom::Client => self.bounded_key(peer.to_string()),
            KeyFrom::Field(field) => self.bounded_key(field_key(envelope, field)?),
            // docs/adr/0114, D4: the one session of the pool, whatever the envelope carries.
            KeyFrom::One => Ok(String::new()),
        }
    }

    /// The position of the journals the machine follows that `envelope` carries in the field
    /// `sessions.position_from` names (docs/adr/0116, D5): `None` for a pool that names none, or
    /// an envelope whose data has no such field or holds `null` in it; or why it carries none the
    /// pool can read.
    fn position_of(&self, envelope: &Envelope<'_>) -> Result<Option<u64>, String> {
        let Some(field) = self.sessions.position_from else {
            return Ok(None);
        };
        let text = payload_text(envelope).map_err(|codec| {
            alloc::format!("its data is in the {codec} codec, which the pool does not read")
        })?;
        if text.is_empty() {
            return Ok(None);
        }
        position_in(&text, field)
    }

    /// What an envelope that carries `position` of the journals the machine follows waits for
    /// (docs/adr/0116, D5): nothing when it carries none, or one the reader's mark has passed; the
    /// position, when it is at or below the journal's horizon, which the reader reaches. Refused
    /// when it is above the horizon: nothing at or below the horizon can still become visible, so
    /// a position above it was told by no answer, and the reader would never pass it; and when the
    /// pool reads no journal (`Pool::follow_from`), or the store did not answer its horizon.
    fn waits_for(&self, position: Option<u64>) -> Result<Option<u64>, Refusal> {
        let Some(position) = position.filter(|position| *position > self.read_mark()) else {
            return Ok(None);
        };
        let Some(journal) = &self.journal else {
            return Err(Refusal {
                status: RpcStatus::InternalError,
                reason:
                    "the pool reads no journal (`Pool::follow_from`), so it passes no position \
                         of one"
                        .to_string(),
            });
        };
        // The horizon, which every read answers: past the last position there is, no entry.
        match journal.after(u64::MAX, 0) {
            Ok(page) if position <= page.horizon => Ok(Some(position)),
            Ok(page) => Err(Refusal {
                status: RpcStatus::InvalidArgument,
                reason: alloc::format!(
                    "it carries the position {position} of the journals the machine follows, \
                     above the journal's horizon, {}, which no answer told",
                    page.horizon
                ),
            }),
            Err(failure) => {
                self.error(PoolError::Store { key: None, failure });
                Err(Refusal::unavailable(
                    "the journal's horizon could not be read".to_string(),
                ))
            }
        }
    }

    /// The reader's mark as the store last kept it (docs/adr/0116, D5).
    fn read_mark(&self) -> u64 {
        self.read_mark.load(Ordering::SeqCst)
    }

    /// The reader's mark kept in the store is `mark` (docs/adr/0116, D5): the keys of the sessions
    /// whose envelopes waited for a position at or below it, which the host runs.
    fn passed(&self, mark: u64) -> Vec<String> {
        self.read_mark.fetch_max(mark, Ordering::SeqCst);
        let mut awaiting = lock(&self.awaiting);
        let above = awaiting.split_off(&(mark.saturating_add(1), String::new()));
        core::mem::replace(&mut *awaiting, above)
            .into_iter()
            .map(|(_, key)| key)
            .collect()
    }

    /// `key`, read from an envelope or a followed entry, or why it names no session of this pool:
    /// it is empty, or longer than a key of the pool holds. The empty key is the one session's of
    /// a pool that keeps one ([`KeyFrom::One`]), which no key read is taken for.
    fn bounded_key(&self, key: String) -> Result<String, String> {
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

    /// Put the envelope `input` in the inbox of the session `key`, which the
    /// table holds from now on if it did not, within the pool's bounds
    /// (docs/adr/0109, D1): past `max_inbox` envelopes in its inbox it is
    /// refused, and a key the table does not hold while it holds `max_live`
    /// is refused unless a session can be saved and dropped to make room. A
    /// refused input comes back with the bound it met.
    fn post_bounded(&self, key: &str, input: Input) -> Result<(), (Input, Full)> {
        match self.try_post(key, input) {
            Err((input, Full::Live)) if self.make_room() => self.try_post(key, input),
            other => other,
        }
    }

    /// [`Self::post_bounded`] without making room.
    fn try_post(&self, key: &str, input: Input) -> Result<(), (Input, Full)> {
        self.post_in(&mut hold(&self.table), key, input).map(|_| ())
    }

    /// Put `input`, an envelope or an entry of a followed journal, in the inbox of the session
    /// `key` in `table`, the pool's table locked, which holds the session from now on if it did
    /// not, as [`slot::admission`] admits it; answer the slot it was put in, or the bound it met with
    /// the input.
    fn post_in(
        &self,
        table: &mut Table<F::Machine, T, E>,
        key: &str,
        input: Input,
    ) -> Result<SharedSlot<F::Machine, T, E>, (Input, Full)> {
        // Read with the table held, as `Self::stop` sets it.
        let stopping = self.stopping.load(Ordering::SeqCst);
        let (live, max_live, max_inbox) =
            (table.len(), self.sessions.max_live, self.sessions.max_inbox);
        if let Some(slot) = table.get(key) {
            let mut inbox = hold(&slot.inbox);
            return match slot::admission(stopping, Some(inbox.door), live, max_live, max_inbox) {
                Ok(door) => {
                    inbox.admitted(door, input);
                    Ok(sync::Arc::clone(slot))
                }
                Err(full) => Err((input, full)),
            };
        }
        match slot::admission(stopping, None, live, max_live, max_inbox) {
            Ok(door) => {
                let slot = self.new_slot(key, self.store.indexed(key));
                hold(&slot.inbox).admitted(door, input);
                table.insert(key.to_string(), sync::Arc::clone(&slot));
                Ok(slot)
            }
            Err(full) => Err((input, full)),
        }
    }

    /// A slot for the session `key`, which the store indexes as `indexed`, under a serial no
    /// other slot of the pool is given: the count of the slots made, raised by one in a single
    /// atomic step, so that two slots made at once never share it. It only rises, and a `u64`
    /// does not wrap at any rate a pool makes slots, so a serial is never given again, and a slot
    /// made for a key that left the table is never taken for the one it left.
    fn new_slot(&self, key: &str, indexed: String) -> SharedSlot<F::Machine, T, E> {
        let serial = self.slots_made.fetch_add(1, Ordering::SeqCst) + 1;
        sync::Arc::new(Slot::new(key, serial, indexed))
    }

    /// Save and drop the session that has gone longest without an event and
    /// holds nothing pending, to make room for another (docs/adr/0109, D1).
    /// Whether one was dropped; a session being run is not a candidate.
    fn make_room(&self) -> bool {
        let now = self.wall_now();
        // Each quiet session's slot, with when it last had an event.
        let mut quiet = Vec::new();
        for slot in self.slots() {
            let Some(live) = try_hold(&slot.live) else {
                continue;
            };
            if let Some(session) = live
                .as_ref()
                .filter(|session| self.is_quiet(&slot, session))
            {
                quiet.push((session.last_active_ms, sync::Arc::clone(&slot)));
            }
        }
        quiet.sort_by_key(|(last_active_ms, _)| *last_active_ms);
        for (_, slot) in quiet {
            let Some(mut live) = try_hold(&slot.live) else {
                continue;
            };
            let still_quiet = live
                .as_ref()
                .is_some_and(|session| self.is_quiet(&slot, session));
            if still_quiet && self.evict(&slot, &mut live, now, Eviction::MaxLive) {
                return true;
            }
        }
        false
    }

    /// Whether `session` holds nothing a later call acts on: its inbox is
    /// empty and its endpoint is quiet ([`Endpoint::is_quiet`]).
    fn is_quiet(&self, slot: &Slot<F::Machine, T, E>, session: &Live<F::Machine, T, E>) -> bool {
        hold(&slot.inbox).is_empty() && lock(&session.endpoint).is_quiet()
    }

    /// Tell what became of `input`, an envelope for `key` that no session
    /// took in at the bound `full` (docs/adr/0109, D1). For a host that reads
    /// the receipt (`acknowledged`) it is lost, and its binding refuses it to
    /// its sender, who carries it again; otherwise a request is answered
    /// `Unavailable`, and anything else is dropped and told to the host. Either
    /// way it is reported as shed at its bound (docs/adr/0109, D16).
    fn shed(&self, key: &str, input: Input, full: Full, acknowledged: bool) {
        let Input::Envelope {
            peer,
            bytes,
            receipt,
            ..
        } = input
        else {
            return;
        };
        self.metrics().count(
            Counter::PoolSheds {
                reason: full.shed(),
            },
            1,
        );
        let reason = full.reason(&self.sessions);
        let reason = reason.as_str();
        let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&bytes)) else {
            receipt.settle(Fate::Lost);
            return;
        };
        let fate = match request_of(&envelope).filter(|_| !acknowledged) {
            Some(request) => self.refuse(&peer, &request, RpcStatus::Unavailable, reason),
            None => {
                self.error(PoolError::Shed {
                    key: key.to_string(),
                    event: envelope.event_type.to_string(),
                    reason: reason.to_string(),
                });
                Fate::Lost
            }
        };
        receipt.settle(fate);
    }

    /// Put an input made by `input` in every session's inbox, and answer their
    /// keys.
    fn broadcast(&self, input: impl Fn() -> Input) -> Vec<String> {
        let table = hold(&self.table);
        // A stopping pool admits nothing: its sessions are being saved as they
        // stand. Read with the table held, as `Self::stop` sets it.
        if self.stopping.load(Ordering::SeqCst) {
            return Vec::new();
        }
        for slot in table.values() {
            hold(&slot.inbox).tell(input());
        }
        table.keys().cloned().collect()
    }

    /// Expire the outboxes of the pool's durable clients (docs/adr/0109, D10): each entry older
    /// than its binding's `retain_ms` is deleted by the store, which notes it, in the same
    /// transaction, for the session that sent it; each is told to the host
    /// ([`PoolError::OutboxExpired`]). A sender in memory reads its notes at its next run, and the
    /// keys answered are those sessions, to run; one out of memory was made due at once by the
    /// store, and is loaded by [`Self::due`]. The host calls this on its clock loop's schedule
    /// (`Self::next_wake_ms` counts the expiries of the outboxes read in this process).
    pub fn expire(&self) -> Vec<String> {
        let Some(outboxes) = self.outboxes.as_ref() else {
            return Vec::new();
        };
        let now = self.wall_now();
        let mut senders = BTreeSet::new();
        for client in outboxes.clients() {
            match outboxes.expire(&client, now, WAKE_BATCH) {
                Ok(expiry) => self.expired(expiry, &mut senders),
                Err(failure) => self.error(PoolError::Store { key: None, failure }),
            }
        }
        self.tell_notes(senders)
    }

    /// Tell the host each entry `expiry` let go of an outbox ([`PoolError::OutboxExpired`]), and
    /// note the session that sent it among `senders`, which read their notes at their next run.
    fn expired(&self, expiry: Expiry, senders: &mut BTreeSet<String>) {
        for expired in expiry.expired {
            if let Some(sender) = &expired.entry.sender {
                senders.insert(sender.clone());
            }
            self.error(PoolError::OutboxExpired {
                client: expired.client,
                number: expired.entry.number,
                at_ms: expired.entry.at_ms,
                sender: expired.entry.sender,
            });
        }
    }

    /// Queue the reading of their notes for each of `senders` in memory, and answer their keys,
    /// the sessions to run; one out of memory was made due by the store.
    fn tell_notes(&self, senders: BTreeSet<String>) -> Vec<String> {
        let table = hold(&self.table);
        let mut keys = Vec::new();
        for sender in senders {
            if let Some(slot) = table.get(&sender) {
                hold(&slot.inbox).tell(Input::Notes);
                keys.push(sender);
            }
        }
        keys
    }

    /// End the outbox of the instance `client`, whose membership ended (docs/adr/0113, D4;
    /// [`Outboxes::end`]): what it still owes is let go, each entry told the host and noted for
    /// the session that sent it, among `senders`, and the outbox is closed.
    fn end_outbox(&self, client: &str, senders: &mut BTreeSet<String>) {
        let Some(outboxes) = self.outboxes.as_ref() else {
            return;
        };
        loop {
            match outboxes.end(client, WAKE_BATCH) {
                Ok(expiry) => {
                    let done = expiry.closed || expiry.expired.is_empty();
                    self.expired(expiry, senders);
                    if done {
                        return;
                    }
                }
                Err(failure) => {
                    self.error(PoolError::Store { key: None, failure });
                    return;
                }
            }
        }
    }

    /// Stop the pool in order (docs/adr/0109, D10): admit nothing from now on (what arrives is
    /// shed, [`Fate::Lost`] for a host that reads receipts, `RpcStatus::Unavailable` for a
    /// request it does not), run what each session's inbox holds to completion, wait for a run
    /// of a session another thread is in the middle of, then save every session in memory as a
    /// clean checkpoint (SCE_MESH.md section 10.11: its endpoint takes nothing in after it, so
    /// its restore raises no `restored`), drop it and release its lease. What is not done by
    /// `deadline` is not done: the report names each session not saved and why, and each such
    /// session restores from its last durable checkpoint with `restored`, as after a crash.
    ///
    /// The host stops its binding's server first (`HttpsServer::stop`), so that no exchange is
    /// taken in while the sessions are saved, and catches the signal that asks it to stop
    /// (docs/SCE_SERVER_DEPLOY.md, section 4).
    pub fn stop(&self, deadline: Instant) -> StopReport {
        {
            // Set with the table held, as every admit reads it: an admit that
            // found the pool running put its input in an inbox before the
            // inboxes below are read, and one after it is refused.
            let _table = hold(&self.table);
            self.stopping.store(true, Ordering::SeqCst);
        }
        // What the inboxes hold, run to completion; a run made on another thread meanwhile is
        // waited for by `run`, which takes the session's lock.
        loop {
            let waiting: Vec<String> = self
                .slots()
                .into_iter()
                .filter(|slot| {
                    // docs/adr/0116, D5: an envelope that waits for a reader a stopping pool no
                    // longer runs is not run for; it is answered below.
                    let inbox = hold(&slot.inbox);
                    inbox.takeable(self.read_mark()).is_some()
                })
                .map(|slot| slot.key.clone())
                .collect();
            if waiting.is_empty() || Instant::now() >= deadline {
                break;
            }
            for key in waiting {
                self.run(&key);
            }
        }
        let mut report = StopReport::default();
        for slot in self.slots() {
            let mut live = loop {
                if let Some(live) = try_hold(&slot.live) {
                    break Some(live);
                }
                if Instant::now() >= deadline {
                    break None;
                }
                sync::pause();
            };
            let Some(live) = live.as_mut() else {
                report.not_saved.push((
                    slot.key.clone(),
                    "a run of it was still going at the deadline".to_string(),
                ));
                continue;
            };
            if live.is_none() {
                // docs/adr/0116, D5: a session never loaded whose envelopes all wait for the
                // reader: what its inbox holds is answered, and its senders carry it again.
                if !hold(&slot.inbox).is_empty() {
                    self.abandon(&slot, &Refusal::unavailable("the pool stopped".to_string()));
                }
                continue;
            }
            let now = self.wall_now();
            let saved = match live.as_mut() {
                Some(session) => self.checkpoint(&slot, session, now, Take::Clean),
                None => continue,
            };
            match saved {
                Ok(true) => {
                    **live = None;
                    self.metrics().adjust(Gauge::LiveSessions, -1);
                    self.forget_session(&slot.key);
                    self.release(&slot);
                    // Out of the table; what its inbox still holds, when the deadline came
                    // before its run, was never admitted, and its sender carries it again.
                    self.abandon(&slot, &Refusal::unavailable("the pool stopped".to_string()));
                    report.saved.push(slot.key.clone());
                }
                Ok(false) => {
                    self.lose(&slot, live, "the pool stopped".to_string());
                    report.not_saved.push((
                        slot.key.clone(),
                        "its clean checkpoint could not be written".to_string(),
                    ));
                }
                Err(reason) => {
                    self.lose(&slot, live, reason.clone());
                    report.not_saved.push((slot.key.clone(), reason));
                }
            }
        }
        // docs/adr/0109, D16: the stop has ended, which `health` reports as stopped.
        self.stopped.store(true, Ordering::SeqCst);
        report
    }

    /// Every session the table holds now.
    fn slots(&self) -> Vec<SharedSlot<F::Machine, T, E>> {
        hold(&self.table).values().cloned().collect()
    }

    /// Take `slot` out of the table when its inbox holds nothing, so that it
    /// is not loaded for nothing. True when it is out of the table, taken out
    /// now or before.
    fn retire_if_idle(&self, slot: &SharedSlot<F::Machine, T, E>) -> bool {
        let mut table = hold(&self.table);
        let retired = hold(&slot.inbox).retire();
        if retired {
            remove_slot(&mut table, slot);
        }
        retired
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
                lock(&self.deferred).insert(slot.indexed.clone(), held.expires_ms);
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
        lock(&self.deferred).remove(&slot.indexed);
        hold(&slot.lease).granted(lease.fence, now);
        let built = self.build(key, now);
        match built {
            Ok(_) => self.metrics().count(Counter::Loads, 1),
            Err(_) => self.release(slot),
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
        if let Some(loaded) = &loaded {
            self.append_held(key, &loaded.checkpoint, now)?;
        }
        let machine = match &loaded {
            Some(loaded) => self.restore(key, &loaded.checkpoint, &endpoint, now),
            None => self.factory.start(key, &endpoint, now),
        }
        .map_err(not_built)?;
        // docs/adr/0109, D16: a restore from a checkpoint that is not clean is the one that
        // raises `restored` (SCE_MESH.md section 16.7 row 17).
        if let Some(loaded) = &loaded {
            let checkpoint = if loaded.checkpoint.clean {
                RestoredFrom::Clean
            } else {
                RestoredFrom::NotClean
            };
            self.metrics().count(Counter::Restores { checkpoint }, 1);
        }
        let (version, sequence, earliest_due) = loaded.as_ref().map_or((0, 0, None), |loaded| {
            (
                loaded.version,
                loaded.checkpoint.sequence,
                // The restore just released what the endpoint held.
                loaded.checkpoint.state.earliest_due(),
            )
        });
        // docs/adr/0109, D10: what an expiry noted for the session is told
        // its machine now, and taken by the write that keeps what it made of it.
        let notes_through = self.raise_notes(key, &mut lock(&endpoint), 0);
        Ok(Live {
            machine,
            endpoint,
            version,
            sequence,
            earliest_due,
            last_active_ms: now,
            // A machine just started ran its initial macrostep, which no
            // checkpoint holds yet; one told of notes is to be saved with them
            // taken.
            dirty: loaded.is_none() || notes_through > 0,
            unsaved: Vec::new(),
            provisional: loaded.is_none(),
            notes_through,
            facts: Vec::new(),
            followed: loaded
                .as_ref()
                .map(|loaded| loaded.checkpoint.followed.clone())
                .unwrap_or_default(),
            under: BTreeSet::new(),
            fenced: false,
        })
    }

    /// Append to the durable clients' outboxes what `checkpoint`, the last durable checkpoint of
    /// the session `key`, holds for them unreleased (docs/adr/0109, D10): one a build that kept
    /// no outbox for the client wrote, which the restore releases. The pool writes no such
    /// checkpoint, since the write that makes one durable appends what it holds for a durable
    /// client; so a restore after a binding became durable loses nothing the checkpoint held.
    fn append_held(&self, key: &str, checkpoint: &Checkpoint, now: u64) -> Result<(), Refusal> {
        let Some(outboxes) = &self.outboxes else {
            return Ok(());
        };
        let held: Vec<&Unreleased> = checkpoint
            .endpoints
            .iter()
            .flat_map(|section| section.router.unreleased.iter())
            .filter(|unreleased| outboxes.keeps(&unreleased.peer))
            .collect();
        if held.is_empty() {
            return Ok(());
        }
        let incarnation = self.incarnation();
        let appends: Vec<Append<'_>> = held
            .iter()
            .map(|unreleased| Append {
                client: &unreleased.peer,
                incarnation: &incarnation,
                sender: Some(key),
                envelope: &unreleased.envelope,
            })
            .collect();
        match self.store.append(&appends, now) {
            Ok(appended) => {
                let envelopes: Vec<Vec<u8>> = held
                    .iter()
                    .map(|unreleased| unreleased.envelope.clone())
                    .collect();
                outboxes.committed(&appended, &envelopes, now, false);
                Ok(())
            }
            Err(failure) => {
                let reason = failure.to_string();
                self.error(PoolError::Store {
                    key: Some(key.to_string()),
                    failure,
                });
                Err(Refusal::unavailable(reason))
            }
        }
    }

    /// Tell the machine of the session `key`, through `endpoint`, each note an expiry wrote for
    /// it above `told` (docs/adr/0109, D10): an envelope it sent a client that the client never
    /// acknowledged within the binding's `retain_ms`, raised as SCE_MESH.md section 16.7 row 15.
    /// Answers the last note told, which the session's next write takes.
    fn raise_notes(
        &self,
        key: &str,
        endpoint: &mut Endpoint<SessionTransport<T>, E>,
        told: u64,
    ) -> u64 {
        let Some(outboxes) = &self.outboxes else {
            return told;
        };
        let notes = match self.store.notes(key) {
            Ok(notes) => notes,
            Err(failure) => {
                self.error(PoolError::Store {
                    key: Some(key.to_string()),
                    failure,
                });
                return told;
            }
        };
        let now = self.wall_now();
        let mut last = told;
        for note in notes.into_iter().filter(|note| note.id > told) {
            let retain_ms = outboxes.retain_ms(&note.client).unwrap_or(0);
            endpoint.outbox_expired(&note.client, now.saturating_sub(note.at_ms), retain_ms);
            last = last.max(note.id);
        }
        last
    }

    /// The incarnation a client's outbox numbers in when this pool's write is the one that opens
    /// it (SCE_MESH.md section 22.3): a fresh UUID of the environment, as 32 lowercase
    /// hexadecimal digits.
    fn incarnation(&self) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let drawn = lock(&self.environment).envelope_id();
        let mut text = String::with_capacity(32);
        for byte in drawn {
            text.push(char::from(DIGITS[usize::from(byte >> 4)]));
            text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        text
    }

    /// Take out of `sections`, the endpoint sections of a checkpoint about to be written, what
    /// they hold unreleased for a durable client (docs/adr/0109, D10): the write appends it to
    /// the client's outbox, so the checkpoint holds nothing for it, and its restore releases
    /// nothing of it again.
    fn outbound(&self, sections: &mut [EndpointSection]) -> Vec<Unreleased> {
        let Some(outboxes) = &self.outboxes else {
            return Vec::new();
        };
        let mut outbound = Vec::new();
        for section in sections {
            let (held, kept): (Vec<Unreleased>, Vec<Unreleased>) =
                core::mem::take(&mut section.router.unreleased)
                    .into_iter()
                    .partition(|unreleased| outboxes.keeps(&unreleased.peer));
            section.router.unreleased = kept;
            outbound.extend(held);
        }
        outbound
    }

    /// Give back the places `outbound` took in their clients' outbox windows: envelopes no write
    /// will append, their session dropped before one held them.
    fn unreserve(&self, outbound: &[Unreleased]) {
        if let Some(outboxes) = &self.outboxes {
            for unreleased in outbound {
                outboxes.unreserve(&unreleased.peer, 1);
            }
        }
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
        // docs/adr/0109, D10: what the session sends a durable client takes a
        // place in the client's outbox window, which every session shares.
        if let Some(outboxes) = &self.outboxes {
            let clients: Vec<String> = outboxes
                .bindings()
                .map(|(client, _)| client.to_string())
                .collect();
            for client in clients {
                if let Some(window) = outboxes.window(&client) {
                    router
                        .bound_window(&client, window)
                        .map_err(|error| Refusal {
                            status: RpcStatus::InternalError,
                            reason: alloc::format!(
                                "the session's router cannot keep {client}'s outbox window: \
                                 {error:?}"
                            ),
                        })?;
                }
            }
            // docs/adr/0113, D4: each instance of an open machine whose binding is durable has an
            // outbox of its own, whose window it takes when the router binds it.
            for open in self.machine.open_peers {
                let machine = open.peer.name;
                if !outboxes.keeps_instances_of(machine) {
                    continue;
                }
                let outboxes = Arc::clone(outboxes);
                router
                    .open_windows(
                        machine,
                        Arc::new(move |instance: &str| outboxes.window(instance)),
                    )
                    .map_err(|error| Refusal {
                        status: RpcStatus::InternalError,
                        reason: alloc::format!(
                            "the session's router cannot keep the outbox windows of {machine}'s \
                             instances: {error:?}"
                        ),
                    })?;
            }
        }
        let transport = SessionTransport {
            key: key.to_string(),
            inner: Arc::clone(&self.transport),
            replies: Arc::clone(&self.replies),
            outboxes: self.outboxes.clone(),
        };
        let mut endpoint = Endpoint::new(router, transport, lock(&self.environment).clone());
        // An instance of an open machine is told only to a session whose router binds it
        // (docs/adr/0113, D6), once it does (`Self::learn`).
        for peer in lock(&self.ready).iter() {
            if !self.is_instance(peer) {
                endpoint.peer_ready(peer);
            }
        }
        Ok(Arc::new(Mutex::new(endpoint)))
    }

    /// Apply what `slot`'s inbox holds to `session`, then what is due, taking
    /// a checkpoint after each input that took an envelope in or ran a
    /// macrostep.
    fn serve(&self, slot: &Slot<F::Machine, T, E>, session: &mut Live<F::Machine, T, E>) -> Served {
        let key = slot.key.as_str();
        loop {
            if hold(&slot.lease).is_lost() {
                return Served::Lost("its lease was not renewed".to_string());
            }
            let next = hold(&slot.inbox).take(self.read_mark());
            let Some(input) = next else {
                break;
            };
            let now = self.wall_now();
            let mut event = matches!(input, Input::Envelope { .. });
            // docs/adr/0113, D5 and D4: the store's answer for a member drops completion
            // records, and the end of a membership unbinds its instance, each of which the
            // checkpoint holds.
            let member = matches!(input, Input::Stand(..) | Input::EndMember(..));
            let told = session.notes_through;
            let calls = {
                let mut endpoint = lock(&session.endpoint);
                let mut followed = None;
                match input {
                    // docs/adr/0113, D5: an instance whose binding receives durably is judged by
                    // its completion records under the incarnation its edge took it in under;
                    // every other envelope, an instance's under a `transport` receipt among them,
                    // by the section 10.5 window.
                    Input::Envelope {
                        peer,
                        bytes,
                        receipt,
                        judged: Some((made_under, standing)),
                        acting,
                        ..
                    } if endpoint.judges_by_records(&peer) => {
                        match endpoint.receive_member_for(
                            &peer,
                            made_under,
                            standing,
                            acting.map(|person| *person),
                            bytes,
                        ) {
                            Ok(()) => session.unsaved.push(receipt),
                            // docs/adr/0113, D5: judged again as it runs, it names an incarnation
                            // its member no longer stands under, and nothing of it was taken in.
                            Err(RouterError::Fenced { current, .. }) => {
                                receipt.settle(Fate::Fenced { current });
                            }
                            // D6: the session binds as many instances as the deployment allows, and
                            // none is put out for this one: it is shed, and carried again.
                            Err(refused) => {
                                receipt.settle(Fate::Lost);
                                self.error(PoolError::Endpoint {
                                    key: key.to_string(),
                                    error: refused,
                                });
                            }
                        }
                    }
                    // D6: an instance under a `transport` receipt is held to the bound as one judged
                    // by its records is, and shed past it.
                    Input::Envelope {
                        peer,
                        bytes,
                        receipt,
                        acting,
                        ..
                    } => match endpoint.receive_within_bound(
                        &peer,
                        acting.map(|person| *person),
                        bytes,
                    ) {
                        Ok(()) => session.unsaved.push(receipt),
                        Err(refused) => {
                            receipt.settle(Fate::Lost);
                            self.error(PoolError::Endpoint {
                                key: key.to_string(),
                                error: refused,
                            });
                        }
                    },
                    Input::Stand(peer, standing) => endpoint.stand(&peer, standing),
                    Input::EndMember(peer) => endpoint.remove_peer(&peer),
                    Input::Ready(peer) => endpoint.peer_ready(&peer),
                    Input::NotReady(peer) => endpoint.peer_not_ready(&peer),
                    Input::Tick => endpoint.tick(),
                    Input::Wake => {}
                    Input::Notes => {
                        session.notes_through = self.raise_notes(key, &mut endpoint, told);
                    }
                    // docs/adr/0114, D4: an entry above the position the session took in of its
                    // journal is told the machine, and the position written with what it did; one
                    // at or below it is a repeat, settled once what holds its position is durable.
                    Input::Followed(entry) => {
                        let taken = session.followed.get(&entry.journal).copied();
                        if taken.is_some_and(|taken| entry.position <= taken) {
                            if session.dirty {
                                session.unsaved.push(entry.receipt);
                            } else {
                                entry.receipt.settle(Fate::Settled);
                            }
                        } else {
                            event = true;
                            followed = Some(entry.call());
                            session.followed.insert(entry.journal, entry.position);
                            session.unsaved.push(entry.receipt);
                        }
                    }
                }
                let mut calls = endpoint.take_calls();
                calls.extend(followed);
                calls
            };
            if self.drive(key, session, calls, now) || event {
                session.last_active_ms = now;
            }
            // An envelope taken in changed what the endpoint holds (its
            // duplicate window, its reply cache), which the checkpoint holds
            // too, even when it ran no macrostep; so did a note told, which
            // the next write takes; so did what concerns a member.
            session.dirty |= event || member || session.notes_through > told;
            // A provisional session is written once its first run ends, and
            // only if its machine is still running then (docs/adr/0109, D1).
            if !session.provisional {
                if let Err(reason) = self.checkpoint(slot, session, now, Take::IfChanged) {
                    return Served::Lost(reason);
                }
                if session.fenced {
                    return Served::Fenced;
                }
            }
            if session.machine.finished() {
                return Served::Finished;
            }
        }
        // The first run of a provisional session ends here with its machine
        // running: the document keeps the key, and its checkpoint is taken.
        session.provisional = false;
        let mut calls = lock(&session.endpoint).take_calls();
        loop {
            let now = self.wall_now();
            if self.drive(key, session, calls, now) {
                session.last_active_ms = now;
            }
            if let Err(reason) = self.checkpoint(slot, session, now, Take::IfChanged) {
                return Served::Lost(reason);
            }
            if session.fenced {
                return Served::Fenced;
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
            // docs/adr/0114, D1: what the run said is appended with the state
            // it left, by the session's next write; docs/adr/0116, D5: with how
            // far the session had read each journal it follows as it said it.
            let said = session.machine.take_journal();
            if !said.is_empty() {
                let read = reads_of(&session.followed);
                session.facts.extend(said.into_iter().map(|entry| Said {
                    entry,
                    read: read.clone(),
                }));
            }
            let mut endpoint = lock(&session.endpoint);
            self.report(key, &mut endpoint);
            self.learn(key, &mut endpoint);
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
    /// write ([`SessionStore::write`], through the pool's [`Committer`], with
    /// what other sessions handed it meanwhile) over the version the session
    /// was read at and under the key's lease, appending what the checkpoint
    /// holds for a durable client to its outbox, and, once the write answered,
    /// the endpoint
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
        let sealed = {
            let mut endpoint = lock(&session.endpoint);
            let sealed = endpoint.checkpoint(sequence, state, take == Take::Clean);
            // docs/adr/0113, D5: the checkpoint holds the run since the last durable one, so its
            // write names every incarnation that run was made under, those an earlier seal named
            // whose write was not kept included.
            if sealed.is_ok() {
                session
                    .under
                    .extend(endpoint.sealed_under().iter().cloned());
            }
            sealed
        };
        let mut taken = match sealed {
            Ok(taken) => taken,
            Err(error) => return not_saved(error.to_string()),
        };
        // docs/adr/0114, D4: the position of each followed journal is written with what its
        // entries did.
        taken.followed.clone_from(&session.followed);
        // Sealed: the endpoint is never sealed with this number again, whether
        // or not the write lands, and a later checkpoint holds what this one
        // holds.
        session.sequence = sequence;
        // docs/adr/0109, D10: what the checkpoint holds for a durable client is
        // appended to the client's outbox by the write that keeps it, the
        // transactional outbox: both are durable or neither is.
        let outbound = self.outbound(&mut taken.endpoints);
        let written = {
            let incarnation = if outbound.is_empty() {
                String::new()
            } else {
                self.incarnation()
            };
            let appends: Vec<Append<'_>> = outbound
                .iter()
                .map(|unreleased| Append {
                    client: &unreleased.peer,
                    incarnation: &incarnation,
                    sender: Some(key),
                    envelope: &unreleased.envelope,
                })
                .collect();
            let document = taken.to_json();
            // docs/adr/0114, D1: the facts the state said are appended by the
            // write that keeps it, so both are durable or neither is.
            let facts = facts_of(&session.facts);
            let under = unders(&session.under);
            // docs/adr/0109, D15: handed to the committer, which writes it with
            // what other sessions handed meanwhile and answers once the
            // transaction that holds it is durable (step 2); step 3 follows.
            self.committer.write(
                key,
                &Write {
                    change: Change::Keep {
                        state: document.as_bytes(),
                        earliest_due: taken.earliest_due(),
                    },
                    expected_version: session.version,
                    fence,
                    outbox: &appends,
                    journal: self.journal_of(&facts),
                    notes_through: session.notes_through,
                    now_ms: now,
                    sessions: &[],
                    under: &under,
                },
            )
        };
        match written {
            Ok(written) => {
                session.version = written.version;
                session.facts.clear();
                session.under.clear();
                if let Some(outboxes) = &self.outboxes {
                    let envelopes: Vec<Vec<u8>> = outbound
                        .into_iter()
                        .map(|unreleased| unreleased.envelope)
                        .collect();
                    outboxes.committed(&written.appended, &envelopes, now, true);
                }
                session.earliest_due = taken.state.earliest_due();
                session.dirty = false;
                // Every envelope taken in so far was taken in before the seal:
                // this checkpoint holds what the session made of each.
                settle_all(&mut session.unsaved, Fate::Settled);
                let mut endpoint = lock(&session.endpoint);
                endpoint.durable(sequence);
                self.report(key, &mut endpoint);
                Ok(true)
            }
            // docs/adr/0113, D5: the run was made under an incarnation that ended, and nothing of
            // it is kept; the session is restored from what the store keeps (`Served::Fenced`).
            Err(WriteError::Conflict(Conflict::Incarnation { presented, current })) => {
                session.fenced = true;
                self.error(PoolError::Fenced {
                    key: key.to_string(),
                    incarnation: presented,
                    current,
                });
                Ok(false)
            }
            Err(WriteError::Conflict(conflict)) => {
                hold(&slot.lease).lose();
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
    /// store, when the store holds one, and its lease released. What the inputs since its last
    /// checkpoint made is sealed first and leaves once the removal answered,
    /// as a checkpoint's envelopes leave once it is durable: a process that
    /// fails before restores the session from that checkpoint, and its
    /// machine makes them again. The envelopes taken in since that checkpoint
    /// are settled once the removal answered, and lost when it did not.
    fn finish(&self, slot: &Slot<F::Machine, T, E>, finished: &mut Live<F::Machine, T, E>) {
        let key = slot.key.as_str();
        let mut ended = false;
        if let Some(fence) = fence_of(slot) {
            let sequence = finished.sequence.saturating_add(1);
            let mut endpoint = lock(&finished.endpoint);
            let mut sealed = match endpoint.seal(sequence) {
                Ok(section) => Some(section),
                Err(error) => {
                    self.error(PoolError::NotSaved {
                        key: key.to_string(),
                        reason: error.to_string(),
                    });
                    None
                }
            };
            // docs/adr/0113, D5: the removal keeps what the last run made, and names every
            // incarnation the run since the last durable checkpoint was made under.
            if sealed.is_some() {
                finished
                    .under
                    .extend(endpoint.sealed_under().iter().cloned());
            }
            let under = unders(&finished.under);
            // docs/adr/0109, D10: what its last inputs sent a durable client is
            // appended to the client's outbox by the write that removes it.
            let outbound = match sealed.as_mut() {
                Some(section) => self.outbound(core::slice::from_mut(section)),
                None => Vec::new(),
            };
            let now = self.wall_now();
            // docs/adr/0109, D13: the sessions of the machine's tree that
            // started a host-run invoke, whose first results of the key-value
            // feature a store keeps under their identities and deletes with
            // the removal.
            let host_sessions = sessions_with_host_starts(&finished.machine, now);
            let host_session_ids: Vec<&str> = host_sessions.iter().map(String::as_str).collect();
            // A session the store never held (a provisional one whose first
            // run finished, docs/adr/0109, D1) has nothing to remove, and what
            // it sent leaves at once: nothing durable changed, unless a
            // durable client's outbox is appended to, a host operation of it
            // kept a first result, or it said a fact (docs/adr/0114, D1),
            // which the removal appends.
            let facts = facts_of(&finished.facts);
            let nothing_kept_by_the_removal = host_sessions.is_empty() && facts.is_empty();
            let removed =
                if finished.version == 0 && outbound.is_empty() && nothing_kept_by_the_removal {
                    Ok(None)
                } else {
                    let incarnation = if outbound.is_empty() {
                        String::new()
                    } else {
                        self.incarnation()
                    };
                    let appends: Vec<Append<'_>> = outbound
                        .iter()
                        .map(|unreleased| Append {
                            client: &unreleased.peer,
                            incarnation: &incarnation,
                            sender: Some(key),
                            envelope: &unreleased.envelope,
                        })
                        .collect();
                    self.committer
                        .write(
                            key,
                            &Write {
                                outbox: &appends,
                                journal: self.journal_of(&facts),
                                now_ms: now,
                                sessions: &host_session_ids,
                                under: &under,
                                ..Write::delete(finished.version, fence)
                            },
                        )
                        .map(Some)
                };
            match removed {
                Ok(written) if sealed.is_some() => {
                    if let (Some(written), Some(outboxes)) = (written, &self.outboxes) {
                        let envelopes: Vec<Vec<u8>> = outbound
                            .iter()
                            .map(|unreleased| unreleased.envelope.clone())
                            .collect();
                        outboxes.committed(&written.appended, &envelopes, now, true);
                    }
                    endpoint.durable(sequence);
                    self.report(key, &mut endpoint);
                    ended = true;
                }
                Ok(_) => {}
                Err(WriteError::Conflict(conflict)) => {
                    self.unreserve(&outbound);
                    self.error(PoolError::LeaseLost {
                        key: key.to_string(),
                        reason: alloc::format!("its end was refused: {conflict}"),
                    })
                }
                Err(WriteError::Failed(failure)) => {
                    self.unreserve(&outbound);
                    self.error(PoolError::Store {
                        key: Some(key.to_string()),
                        failure,
                    })
                }
            }
        } else if let Some(outboxes) = &self.outboxes {
            // No lease, no write: what it held for a durable client is let go.
            let endpoint = lock(&finished.endpoint);
            for client in outboxes.clients() {
                outboxes.unreserve(&client, endpoint.held_for(&client));
            }
        }
        let fate = if ended { Fate::Settled } else { Fate::Lost };
        settle_all(&mut finished.unsaved, fate);
        self.forget_session(key);
        self.release(slot);
    }

    /// Drop `slot`'s session, whose key this process no longer serves for
    /// `reason`, without saving it: what it held for a checkpoint is lost with
    /// it, since another process serves the key from its last durable one, and
    /// so are the envelopes it took in since. Each request its inbox holds is
    /// answered `Unavailable`.
    fn lose(
        &self,
        slot: &SharedSlot<F::Machine, T, E>,
        live: &mut Option<Live<F::Machine, T, E>>,
        reason: String,
    ) {
        if let Some(mut dropped) = live.take() {
            self.metrics().adjust(Gauge::LiveSessions, -1);
            self.let_go(&mut dropped);
        }
        hold(&slot.lease).forget();
        self.forget_session(&slot.key);
        self.abandon(slot, &Refusal::unavailable(reason));
    }

    /// What `dropped`, a session let go unsaved, held since its last durable checkpoint is lost
    /// with it: the envelopes and journal entries it took in, which their senders and readers hand
    /// again; what it produced for a durable client that no write appended, which gives its window
    /// places back (docs/adr/0109, D10); and the facts its runs said that no write appended
    /// (docs/adr/0114, D1). Those facts are not kept for a next write, as a failed write keeps
    /// them: the session is restored from its last durable checkpoint, whose state said none of
    /// them, and an entry exists exactly when the state that said it does. The run that said them
    /// runs again from what is handed again, and says them again.
    fn let_go(&self, dropped: &mut Live<F::Machine, T, E>) {
        settle_all(&mut dropped.unsaved, Fate::Lost);
        if let Some(outboxes) = &self.outboxes {
            let endpoint = lock(&dropped.endpoint);
            for client in outboxes.clients() {
                outboxes.unreserve(&client, endpoint.held_for(&client));
            }
        }
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
        let Some(fence) = hold(&slot.lease).give_up() else {
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
    /// request its inbox held with `refusal`, which settles it; anything else
    /// it held is dropped, told to the host and lost.
    fn abandon(&self, slot: &SharedSlot<F::Machine, T, E>, refusal: &Refusal) {
        let held = {
            let mut table = hold(&self.table);
            let held = hold(&slot.inbox).close();
            remove_slot(&mut table, slot);
            held
        };
        for input in held {
            let (peer, bytes, receipt) = match input {
                Input::Envelope {
                    peer,
                    bytes,
                    receipt,
                    ..
                } => (peer, bytes, receipt),
                // docs/adr/0114, D4: an entry no session took in is handed again by the reader.
                Input::Followed(entry) => {
                    entry.receipt.settle(Fate::Lost);
                    continue;
                }
                _ => continue,
            };
            let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&bytes)) else {
                receipt.settle(Fate::Lost);
                continue;
            };
            let fate = match request_of(&envelope) {
                Some(request) => self.refuse(&peer, &request, refusal.status, &refusal.reason),
                None => {
                    self.error(PoolError::Dropped {
                        key: slot.key.clone(),
                        event: envelope.event_type.to_string(),
                        reason: refusal.reason.clone(),
                    });
                    Fate::Lost
                }
            };
            receipt.settle(fate);
        }
    }

    /// Answer `request`, which arrived on `peer`'s link, with `status` and
    /// `reason`: the reply a requester ends its invocation with, sent by the
    /// pool because no session of it answers. The request is settled when the
    /// reply was handed to the transport, and lost when it could not be.
    fn refuse(&self, peer: &str, request: &Request, status: RpcStatus, reason: &str) -> Fate {
        let id = lock(&self.environment).envelope_id();
        let sent = refusal_envelope(self.machine.name, request, status, reason, id)
            .map_err(|error| alloc::format!("{error:?}"))
            .and_then(|bytes| self.answer(peer, bytes));
        match sent {
            Ok(()) => Fate::Settled,
            Err(reason) => {
                self.error(PoolError::NotAnswered {
                    peer: peer.to_string(),
                    reason,
                });
                Fate::Lost
            }
        }
    }

    /// Send `bytes`, an answer the pool makes in a session's place, to `peer`: appended to its
    /// outbox when its binding keeps a durable one (docs/adr/0109, D10), so that what settles the
    /// request it answers is durable too, and handed to the transport otherwise.
    fn answer(&self, peer: &str, bytes: Vec<u8>) -> Result<(), String> {
        let Some(outboxes) = self
            .outboxes
            .as_ref()
            .filter(|outboxes| outboxes.keeps(peer))
        else {
            return lock(&self.transport)
                .transmit(peer, &bytes)
                .map_err(|failure| alloc::format!("{failure:?}"));
        };
        let now = self.wall_now();
        let incarnation = self.incarnation();
        let appended = self
            .store
            .append(
                &[Append {
                    client: peer,
                    incarnation: &incarnation,
                    sender: None,
                    envelope: &bytes,
                }],
                now,
            )
            .map_err(|failure| failure.to_string())?;
        outboxes.committed(&appended, &[bytes], now, false);
        Ok(())
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

    /// The journal a write of a session of this machine appends `facts` to
    /// (docs/adr/0114, D1 and D2): the session's own stream, its machine and
    /// its key; none when it says no fact.
    fn journal_of<'a>(&'a self, facts: &'a [Fact<'a>]) -> Option<Journal<'a>> {
        (!facts.is_empty()).then_some(Journal {
            machine: self.machine.name,
            facts,
        })
    }

    fn wall_now(&self) -> u64 {
        lock(&self.environment).now_unix_ms()
    }
}

/// A fact a session's run said that no durable write appended yet: the engine's entry, and how
/// far the session had read each journal it follows when the run said it (docs/adr/0116, D5).
struct Said {
    entry: JournalEntry,
    read: Vec<Read>,
}

/// How far a session whose checkpoint holds `followed` has read each journal it follows
/// (docs/adr/0114, D4), as a fact it says records it (docs/adr/0116, D5): in the order of the
/// journals' names, each once.
fn reads_of(followed: &BTreeMap<String, u64>) -> Vec<Read> {
    followed
        .iter()
        .map(|(journal, through)| Read {
            journal: journal.clone(),
            through: *through,
        })
        .collect()
}

/// The facts `said` holds, in their order, as a write appends them.
fn facts_of(said: &[Said]) -> Vec<Fact<'_>> {
    said.iter()
        .map(|Said { entry, read }| Fact {
            schema: entry.schema,
            revision: entry.revision,
            data: FactData::Bytes(&entry.data),
            valid_ms: entry.valid_ms,
            // docs/adr/0116, D7: who the fact was said for, as the engine recorded it from the
            // person the edge stamped on the event that began the macrostep.
            actor: entry.principal.as_deref().map(|person| Actor {
                instance: &person.instance,
                issuer: &person.issuer,
                subject: &person.subject,
                proved_ms: person.proved_ms,
                method: &person.method,
            }),
            // docs/adr/0116, D5: what the decision had read of the journals the session follows.
            read: read.as_slice(),
        })
        .collect()
}

/// The incarnations `under` holds, each with its member's name, as a write names them
/// (docs/adr/0113, D5).
fn unders(under: &BTreeSet<(String, u64)>) -> Vec<Under<'_>> {
    under
        .iter()
        .map(|(member, incarnation)| Under {
            member: member.as_str(),
            incarnation: *incarnation,
        })
        .collect()
}

/// Settle every receipt of `receipts` as `fate`, and forget them.
fn settle_all(receipts: &mut Vec<Receipt>, fate: Fate) {
    for receipt in receipts.drain(..) {
        receipt.settle(fate);
    }
}

/// The ids of the sessions of `machine`'s tree, its root and every running
/// child its state holds at `wall_now_ms`, that started a host-run `<invoke>`:
/// the only ones a host keeps first results under (docs/adr/0109, D13), and
/// which the write that removes the session deletes them for. Empty when the
/// machine cannot be saved now.
fn sessions_with_host_starts<M: SessionMachine>(machine: &M, wall_now_ms: u64) -> Vec<String> {
    fn gather(state: &SavedState, into: &mut Vec<String>) {
        if !state.host_invoke_starts.is_empty() && !state.session_id.is_empty() {
            into.push(state.session_id.clone());
        }
        if let SavedInvokes::Resumed(children) = &state.invokes {
            for child in children {
                gather(&child.session, into);
            }
        }
    }
    let mut ids = Vec::new();
    if let Ok(state) = machine.save(wall_now_ms) {
        gather(&state, &mut ids);
    }
    ids
}

/// The fence of the lease `slot` holds, if it holds one.
fn fence_of<M, T, E>(slot: &Slot<M, T, E>) -> Option<u64> {
    hold(&slot.lease).fence()
}

/// Take `slot` out of `table`, if it is still the session the table holds for
/// its key, the slot of its serial: a newer one for the key stays.
fn remove_slot<M, T, E>(table: &mut Table<M, T, E>, slot: &Slot<M, T, E>) {
    if table
        .get(&slot.key)
        .is_some_and(|current| current.serial == slot.serial)
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
    key_in(&text, field)
}

/// The key the field `field` of `text`, a JSON object, holds: a text as it is,
/// an integer as its digits. What an envelope's data and an entry of a followed
/// journal (docs/adr/0114, D4) are keyed by alike.
fn key_in(text: &str, field: &str) -> Result<String, String> {
    let data = json::parse(text).map_err(|error| alloc::format!("its data is {error}"))?;
    match data.member(field) {
        Some(Value::Text(text)) => Ok(text.clone()),
        Some(Value::Number(digits)) if is_integer(digits) => Ok(digits.clone()),
        Some(_) => Err(alloc::format!(
            "its field '{field}' is neither a text nor an integer"
        )),
        None => Err(alloc::format!("its data has no field '{field}'")),
    }
}

/// The position of a journal the field `field` of `text`, a JSON object, holds
/// (docs/adr/0116, D5): `None` when it has no such field or holds `null` in it; refused when it
/// holds anything but an unsigned integer that fits a position.
fn position_in(text: &str, field: &str) -> Result<Option<u64>, String> {
    let data = json::parse(text).map_err(|error| alloc::format!("its data is {error}"))?;
    match data.member(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(digits)) => digits.parse::<u64>().map(Some).map_err(|_| {
            alloc::format!("its field '{field}' is {digits}, which is no position of a journal")
        }),
        Some(_) => Err(alloc::format!(
            "its field '{field}' is no position of a journal"
        )),
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

/// [`lock`], for a lock of [`sync`]: one the loom models explore.
fn hold<V>(mutex: &sync::Mutex<V>) -> sync::MutexGuard<'_, V> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The value of `mutex`, a lock of [`sync`], or `None` while another
/// thread holds it.
fn try_hold<V>(mutex: &sync::Mutex<V>) -> Option<sync::MutexGuard<'_, V>> {
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
    use crate::peers::{Instances, OpenPeer, Peer, PeerLink};
    use crate::router::{PeerConfig, Responders};
    use crate::rpc::MESH_EVENT_PARAM;
    use alloc::vec;
    use sce_rust_metrics::recording::Recording;
    use sce_rust_runtime::saved_state::{SavedAct, SavedHostSend, SavedInvokes, SavedSend};
    use sce_rust_runtime::{HostInvokeRequest, HostSendRequest};
    use sce_rust_session_store::{Change, Due, Lease, MemoryStore, Stored, Write, Written};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Condvar;
    use std::time::{Duration, Instant};

    mod following;
    mod journal;
    mod reading;

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
        responders: Responders::Named(&["client"]),
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
                    responders: Responders::Named(&["other"]),
                    reply_events: &[],
                    ..CLIENT
                },
                link: PeerLink::WssDial {
                    url: "wss://other.example",
                    keepalive_ms: None,
                },
            },
        ],
        open_peers: &[],
        instances: Instances::One,
        sessions: Some(ROOMS_SESSIONS),
    };

    /// How [`ROOMS`] is pooled: by its `room` field, idle after a second, with
    /// deploy.yaml's default bounds.
    const ROOMS_SESSIONS: Sessions = Sessions {
        key_from: KeyFrom::Field("room"),
        max_key_bytes: 16,
        idle_ms: 1_000,
        max_live: 100_000,
        max_inbox: 64,
        commit_delay_us: 0,
        follows: crate::peers::Follows::NONE,
        position_from: None,
    };

    /// [`ROOMS`] holding at most `max_live` sessions in memory and
    /// `max_inbox` envelopes in one session's inbox.
    fn rooms_bounded(max_live: u32, max_inbox: u32) -> Machine {
        Machine {
            sessions: Some(Sessions {
                max_live,
                max_inbox,
                ..ROOMS_SESSIONS
            }),
            ..ROOMS
        }
    }

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
        pool_of(ROOMS, owner, store, clock, probe)
    }

    /// A pool of `machine`, a deployment of the rooms, as [`pool`] makes one,
    /// kept in `store`.
    fn pool_of(
        machine: Machine,
        owner: &str,
        store: Arc<dyn SessionStore>,
        clock: &Clock,
        probe: &Arc<Probe>,
    ) -> (Pool<Rooms, Wire, Clock>, Wire) {
        pool_with(machine, owner, store, clock, probe, Wire::default())
    }

    /// A pool of `machine` as [`pool_of`] makes one, sending through `wire`.
    fn pool_with(
        machine: Machine,
        owner: &str,
        store: Arc<dyn SessionStore>,
        clock: &Clock,
        probe: &Arc<Probe>,
        wire: Wire,
    ) -> (Pool<Rooms, Wire, Clock>, Wire) {
        let pool = Pool::new(
            machine,
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

    /// A store in memory that counts its transactions (docs/adr/0109, D15):
    /// it holds its first commit until the test lets it end, as a slow medium
    /// would, notes the keys each commit kept, and notes for each key how many
    /// envelopes the wire had been handed when the commit that kept its write
    /// ended.
    struct Committing {
        inner: MemoryStore,
        handed: Handed,
        open: Mutex<bool>,
        opened: Condvar,
        /// How many items the commits that began hold.
        entered: AtomicU64,
        /// The keys each commit kept, in the order the commits ended.
        commits: Mutex<Vec<Vec<String>>>,
        /// For each key, the envelopes the wire held when its write was durable.
        durable_at: Mutex<BTreeMap<String, usize>>,
    }

    impl Committing {
        fn over(inner: MemoryStore, wire: &Wire) -> Self {
            Committing {
                inner,
                handed: Arc::clone(&wire.0),
                open: Mutex::new(false),
                opened: Condvar::new(),
                entered: AtomicU64::new(0),
                commits: Mutex::new(Vec::new()),
                durable_at: Mutex::new(BTreeMap::new()),
            }
        }

        fn open(&self) {
            *lock(&self.open) = true;
            self.opened.notify_all();
        }

        fn entered(&self) -> usize {
            usize::try_from(self.entered.load(Ordering::SeqCst)).unwrap_or(usize::MAX)
        }

        /// A commit that kept `keys` ended.
        fn ended(&self, keys: Vec<String>) {
            let handed = lock(&self.handed).len();
            let mut durable_at = lock(&self.durable_at);
            for key in &keys {
                durable_at.insert(key.clone(), handed);
            }
            lock(&self.commits).push(keys);
        }
    }

    impl SessionStore for Committing {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(key)
        }

        /// A transaction of one write.
        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            self.entered.fetch_add(1, Ordering::SeqCst);
            let written = self.inner.write(key, write)?;
            self.ended(vec![key.to_string()]);
            Ok(written)
        }

        fn write_many(
            &self,
            batch: &[sce_rust_session_store::Batched<'_>],
        ) -> Result<Vec<sce_rust_session_store::Decided>, Failure> {
            let before = self.entered.fetch_add(batch.len() as u64, Ordering::SeqCst);
            if before == 0 {
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut open = lock(&self.open);
                while !*open {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        break;
                    }
                    open = self
                        .opened
                        .wait_timeout(open, left)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
            }
            let decided = self.inner.write_many(batch)?;
            let kept = batch
                .iter()
                .zip(&decided)
                .filter_map(|(item, decided)| match (item, decided) {
                    (sce_rust_session_store::Batched::Write { key, .. }, Ok(_)) => {
                        Some(key.to_string())
                    }
                    _ => None,
                })
                .collect();
            self.ended(kept);
            Ok(decided)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
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

    /// docs/adr/0109, D15: the checkpoints of sessions of many keys, run on
    /// threads of their own as a server runs its clients' exchanges, are
    /// committed in fewer transactions than inputs: while the first commit is
    /// written, every other session's write waits, and the next commit, which
    /// starts as soon as the first ended, takes them all. No reply leaves
    /// before the commit that holds its session's checkpoint ended. Red if the
    /// pool writes each checkpoint in a transaction of its own (eight commits,
    /// and no write waits for the held one), or tells the endpoint its
    /// checkpoint is durable before the commit answered (a reply handed before
    /// the commit that holds it).
    #[test]
    fn many_sessions_share_a_commit_and_each_reply_waits_for_its_own() {
        const KEYS: u32 = 8;
        let (clock, probe) = (Clock::new(), Arc::new(Probe::default()));
        let wire = Wire::default();
        let store = Arc::new(Committing::over(MemoryStore::new(), &wire));
        let (pool, wire) = pool_with(
            ROOMS,
            "p1",
            Arc::clone(&store) as Arc<dyn SessionStore>,
            &clock,
            &probe,
            wire,
        );
        let pool = Arc::new(pool);
        let runs: Vec<_> = (0..KEYS)
            .map(|n| {
                let pool = Arc::clone(&pool);
                std::thread::spawn(move || {
                    deliver(&pool, join(&alloc::format!("r{n}"), n));
                })
            })
            .collect();
        // Every session's write handed: one in the commit the store holds, the
        // rest waiting for the next.
        let until = Instant::now() + Duration::from_secs(5);
        while store.entered() + pool.committer().waiting() < KEYS as usize && Instant::now() < until
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            store.entered() + pool.committer().waiting(),
            KEYS as usize,
            "every session handed its checkpoint while the first commit was held"
        );
        assert!(
            lock(&wire.0).is_empty(),
            "a reply left before the commit that holds its session's checkpoint ended"
        );
        store.open();
        for run in runs {
            run.join().expect("no run panicked");
        }

        let commits = lock(&store.commits).clone();
        let kept: usize = commits.iter().map(Vec::len).sum();
        assert_eq!(kept, KEYS as usize, "one checkpoint a session: {commits:?}");
        assert!(
            commits.len() < KEYS as usize,
            "{} transactions for {KEYS} inputs: {commits:?}",
            commits.len()
        );
        assert_eq!(
            commits.len(),
            2,
            "the held commit, and one of everything handed while it was written: {commits:?}"
        );
        let durable_at = lock(&store.durable_at).clone();
        let replies = wire.take();
        assert_eq!(replies.len(), KEYS as usize, "{replies:?}");
        for (at, reply) in replies.iter().enumerate() {
            let n = (0..KEYS)
                .find(|&n| reply.wire_id == Some(id(0x22, n)))
                .unwrap_or_else(|| panic!("a reply to no join: {reply:?}"));
            let key = alloc::format!("r{n}");
            let committed = durable_at
                .get(&key)
                .copied()
                .unwrap_or_else(|| panic!("no commit kept {key}'s checkpoint"));
            assert!(
                at >= committed,
                "the reply to {key} was handed as envelope {at}, before the commit holding its \
                 checkpoint ended with {committed} handed"
            );
        }
        assert!(pool.take_errors().is_empty());
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
        let loaded = checkpoint::load(&store, "r1")
            .expect("read")
            .expect("saved");
        assert_eq!(
            loaded.checkpoint.state.earliest_due(),
            None,
            "the reminder left the saved state as it was delivered"
        );
        assert!(pool.due().is_empty(), "the session in memory released it");
        pool.run("r1");
        assert!(wire.take().is_empty(), "delivered once");

        // Saved and dropped once idle, the session waits for nothing in the store either.
        clock.advance(1_000);
        assert_eq!(pool.evict_idle(), vec!["r1".to_string()]);
        assert_eq!(
            store.get("r1").expect("read").expect("saved").earliest_due,
            None
        );
        assert!(pool.due().is_empty());
    }

    /// A store that indexes each key apart from it, as one that keeps keyed digests does
    /// (docs/adr/0109, D8): the session `k` is kept under the entry `#k`, which its listing of
    /// dues names, and the key is read back from the store.
    struct IndexedApart {
        inner: MemoryStore,
    }

    impl IndexedApart {
        fn entry(key: &str) -> String {
            alloc::format!("#{key}")
        }
    }

    impl SessionStore for IndexedApart {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(&Self::entry(key))
        }

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            self.inner.write(&Self::entry(key), write)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn indexed(&self, key: &str) -> String {
            Self::entry(key)
        }

        fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
            Ok(self
                .inner
                .get(indexed)?
                .and(indexed.strip_prefix('#'))
                .map(ToString::to_string))
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
        }

        fn acquire(
            &self,
            key: &str,
            owner: &str,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, AcquireError> {
            self.inner.acquire(&Self::entry(key), owner, ttl_ms, now_ms)
        }

        fn renew(
            &self,
            key: &str,
            fence: u64,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, LeaseError> {
            self.inner.renew(&Self::entry(key), fence, ttl_ms, now_ms)
        }

        fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
            self.inner.release(&Self::entry(key), fence)
        }
    }

    /// docs/adr/0109, D8: a store that lists what comes due by an entry other than the key, a
    /// keyed digest of it, wakes the session by the key it names for that entry; a session in
    /// memory or held elsewhere is told apart by its entry, so it is neither loaded twice nor
    /// woken early.
    #[test]
    fn a_session_listed_by_an_entry_apart_from_its_key_is_woken_by_its_key() {
        let (inner, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let store: Arc<dyn SessionStore> = Arc::new(IndexedApart {
            inner: inner.clone(),
        });
        let (pool, wire) = pool_over("p1", store, &clock, &probe);
        deliver(
            &pool,
            from_client("remind", r#"{"room":"r1","after_ms":5000}"#, 1, false),
        );
        assert!(inner.get("r1").expect("read").is_none());
        assert_eq!(
            inner
                .due_before(LATEST_MOMENT, 10)
                .expect("the listing")
                .into_iter()
                .map(|due| due.key)
                .collect::<Vec<_>>(),
            ["#r1"],
            "the store lists the session by its entry"
        );
        clock.advance(1_000);
        assert_eq!(pool.evict_idle(), vec!["r1".to_string()]);
        assert_eq!(pool.next_wake_ms(), Some(T0 + 5_000));
        clock.advance(4_000);
        assert_eq!(
            pool.due(),
            vec!["r1".to_string()],
            "the session is woken by the key its entry names"
        );
        assert!(
            pool.due().is_empty(),
            "the entry the store still lists is told apart as the session the pool holds"
        );
        pool.run("r1");
        let sent = wire.take();
        let [reminded] = sent.as_slice() else {
            panic!("one reminder, got {sent:?}");
        };
        assert_eq!(reminded.data, r#"{"room":"r1"}"#);
        assert_eq!(probe.restored.load(Ordering::SeqCst), 1);
    }

    /// A store that, once armed, takes one write and then fails its process: the write is
    /// durable, and the pool never hears it answered, so it releases nothing the write held.
    struct DiesAfterWrite {
        inner: MemoryStore,
        armed: AtomicBool,
    }

    impl SessionStore for DiesAfterWrite {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(key)
        }

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            let written = self.inner.write(key, write);
            let kept = matches!(write.change, Change::Keep { .. });
            if kept && written.is_ok() && self.armed.swap(false, Ordering::SeqCst) {
                panic!("the process fails after the write landed");
            }
            written
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn indexed(&self, key: &str) -> String {
            self.inner.indexed(key)
        }

        fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
            self.inner.key_of(indexed)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
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

    /// A process that fails after the checkpoint that delivers a session's last delayed send is
    /// durable, and before the send left, leaves a session whose machine waits for nothing and
    /// whose checkpoint holds the send. The store lists it as due at once, so the next process
    /// loads it at its first look at what came due, and the restore delivers the send, once.
    #[test]
    fn a_send_held_by_a_durable_checkpoint_leaves_after_its_process_failed() {
        let (clock, probe) = (Clock::new(), Arc::new(Probe::default()));
        let store = Arc::new(DiesAfterWrite {
            inner: MemoryStore::new(),
            armed: AtomicBool::new(false),
        });
        let shared: Arc<dyn SessionStore> = Arc::clone(&store) as Arc<dyn SessionStore>;
        let (first, first_wire) = pool_over("p1", Arc::clone(&shared), &clock, &probe);
        deliver(
            &first,
            from_client("remind", r#"{"room":"r1","after_ms":5000}"#, 1, false),
        );
        clock.advance(5_000);
        assert_eq!(first.due(), vec!["r1".to_string()]);
        store.armed.store(true, Ordering::SeqCst);
        let failed =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| first.run("r1"))).is_err();
        assert!(failed, "the store did not fail the process");
        assert!(first_wire.take().is_empty(), "the reminder never left");
        drop(first);
        assert_eq!(
            store
                .inner
                .get("r1")
                .expect("read")
                .expect("saved")
                .earliest_due,
            Some(checkpoint::DUE_AT_ONCE),
            "the checkpoint holding the reminder is due at once"
        );

        let (second, second_wire) = pool_over("p1", shared, &clock, &probe);
        assert_eq!(second.due(), vec!["r1".to_string()]);
        second.run("r1");
        let sent = second_wire.take();
        let [reminded] = sent.as_slice() else {
            panic!("one reminder, got {sent:?}");
        };
        assert_eq!(reminded.event, "reminded");
        assert_eq!(reminded.peer, "client");
        assert!(second.due().is_empty());
        second.run("r1");
        assert!(second_wire.take().is_empty(), "delivered once");
        clock.advance(1_000);
        assert_eq!(second.evict_idle(), vec!["r1".to_string()]);
        assert_eq!(
            store
                .inner
                .get("r1")
                .expect("read")
                .expect("saved")
                .earliest_due,
            None,
            "saved once it released the reminder, the session waits for nothing"
        );
        assert!(second.due().is_empty());
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

    /// A keyed pool takes no client for another (SCE_MESH.md sections 10.9, invariant 10, and
    /// 22.1): `other`, writing the client's name as its source, reaches the session its key
    /// names and is refused there, before the session's machine or the client's duplicate
    /// window sees it; the client's own copy is then taken in and answered once. Red if the
    /// router takes `source` over the link: the forgery then joins the room as the client and
    /// its reply goes to `other`, and the client's own request is dropped as a duplicate.
    #[test]
    fn a_keyed_pool_takes_no_client_for_another() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool("p1", &store, &clock, &probe);
        let key = pool
            .receive("other", join("r1", 1))
            .expect("placed in the session its key names");
        pool.run(&key);
        assert!(wire.take().is_empty(), "nothing answers the forgery");
        let errors = pool.take_errors();
        assert!(
            matches!(
                errors.as_slice(),
                [PoolError::Endpoint {
                    key,
                    error: RouterError::SourceNotLink { peer, source },
                }] if key == "r1" && peer == "other" && source == "client"
            ),
            "{errors:?}"
        );

        deliver(&pool, join("r1", 1));
        let replies = wire.take();
        assert_eq!(replies.len(), 1, "{replies:?}");
        assert_eq!(replies[0].wire_id, Some(id(0x22, 1)));
        assert_eq!(stored_members(&store, "r1"), 1, "the client joined once");
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

    /// A store that counts the checkpoints written to it.
    struct Counting {
        inner: MemoryStore,
        puts: AtomicU64,
    }

    impl SessionStore for Counting {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(key)
        }

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            if matches!(write.change, Change::Keep { .. }) {
                self.puts.fetch_add(1, Ordering::SeqCst);
            }
            self.inner.write(key, write)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn indexed(&self, key: &str) -> String {
            self.inner.indexed(key)
        }

        fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
            self.inner.key_of(indexed)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
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

    /// docs/adr/0109, D1: a session started for a key the store holds nothing
    /// for is written only if its first run does not end in a final state. A
    /// request for an unknown key that the document answers and finishes on
    /// is answered at once and writes nothing; a key the document keeps is
    /// written once its first run ends. Red if the pool checkpoints a
    /// provisional session inside its first run: the finished room is then
    /// written before it is removed.
    #[test]
    fn a_key_whose_document_answers_and_finishes_leaves_the_store_unwritten() {
        let (clock, probe) = (Clock::new(), Arc::new(Probe::default()));
        let store = Arc::new(Counting {
            inner: MemoryStore::new(),
            puts: AtomicU64::new(0),
        });
        let shared: Arc<dyn SessionStore> = Arc::clone(&store) as Arc<dyn SessionStore>;
        let (pool, wire) = pool_over("p1", shared, &clock, &probe);
        let key = pool.receive("client", join("r9", 1)).expect("placed");
        assert_eq!(
            pool.receive("client", from_client("close", r#"{"room":"r9"}"#, 2, false)),
            Some(key.clone())
        );
        pool.run(&key);
        let replies = wire.take();
        assert_eq!(replies.len(), 1, "the answer left: {replies:?}");
        assert_eq!(replies[0].status, Some(RpcStatus::Ok));
        assert_eq!(store.puts.load(Ordering::SeqCst), 0, "nothing was written");
        assert_eq!(store.inner.get("r9").expect("read"), None);
        assert!(!pool.is_live("r9"));
        let lease = store
            .inner
            .acquire("r9", "someone", 1, clock.at())
            .expect("its lease was released");
        store.inner.release("r9", lease.fence).expect("given back");

        deliver(&pool, join("r1", 3));
        assert_eq!(
            store.puts.load(Ordering::SeqCst),
            1,
            "a kept key is written"
        );
        assert_eq!(stored_members(&store.inner, "r1"), 1);
        assert!(pool.take_errors().is_empty());
    }

    /// docs/adr/0109, D1: at `max_live`, the session that has gone longest
    /// without an event and holds nothing pending is saved and dropped to make
    /// room for a key not in memory. Red if the pool admits a key past its
    /// bound: three sessions are then in memory.
    #[test]
    fn a_pool_at_max_live_saves_and_drops_its_longest_idle_quiet_session() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool_of(
            rooms_bounded(2, 64),
            "p1",
            Arc::new(store.clone()),
            &clock,
            &probe,
        );
        deliver(&pool, join("r1", 1));
        clock.advance(10);
        deliver(&pool, join("r2", 2));
        clock.advance(10);
        deliver(&pool, join("r3", 3));
        assert_eq!(wire.take().len(), 3);
        assert!(!pool.is_live("r1"), "the longest idle made room");
        assert!(pool.is_live("r2"));
        assert!(pool.is_live("r3"));
        assert_eq!(
            stored_members(&store, "r1"),
            1,
            "saved before it was dropped"
        );
        assert!(pool.take_errors().is_empty());
    }

    /// docs/adr/0109, D1: at `max_live` with no session quiet enough to drop,
    /// an envelope for a key not in memory is shed: a request a host does not
    /// acknowledge by receipt is answered `Unavailable`, and one it does is
    /// lost unanswered, so its binding refuses it and its client carries it
    /// again. Red if the pool admits a key past its bound.
    #[test]
    fn a_pool_at_max_live_with_no_quiet_session_sheds_a_new_key() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool_of(
            rooms_bounded(1, 64),
            "p1",
            Arc::new(store.clone()),
            &clock,
            &probe,
        );
        // r1 waits for an answer from another machine: it is not quiet.
        deliver(&pool, from_client("ask", r#"{"room":"r1"}"#, 1, false));
        assert_eq!(wire.take().len(), 1, "its request left");

        assert_eq!(pool.receive("client", join("r2", 2)), None);
        let replies = wire.take();
        let [reply] = replies.as_slice() else {
            panic!("one reply, got {replies:?}");
        };
        assert_eq!(reply.status, Some(RpcStatus::Unavailable));
        assert_eq!(reply.wire_id, Some(id(0x22, 2)));

        let (key, receipt) = pool.receive_with_receipt("client", join("r3", 3));
        assert_eq!(key, None);
        assert_eq!(receipt.fate(), Fate::Lost);
        assert!(
            wire.take().is_empty(),
            "a shed the binding refuses is not answered"
        );
        assert!(matches!(
            pool.take_errors().as_slice(),
            [PoolError::Shed { key, .. }] if key == "r3"
        ));
        assert!(pool.is_live("r1"));
        assert!(!pool.is_live("r2") && !pool.is_live("r3"));
        assert_eq!(store.get("r3").expect("read"), None);
    }

    /// docs/adr/0109, D1: a session's inbox takes at most `max_inbox`
    /// envelopes; one past it is shed as a key past `max_live` is, and what
    /// the inbox took is served. Red if the inbox takes past its bound: four
    /// requests are then answered `Ok`.
    #[test]
    fn an_envelope_past_max_inbox_is_shed() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool_of(rooms_bounded(16, 2), "p1", Arc::new(store), &clock, &probe);
        assert_eq!(
            pool.receive("client", join("r1", 1)),
            Some("r1".to_string())
        );
        assert_eq!(
            pool.receive("client", join("r1", 2)),
            Some("r1".to_string())
        );
        assert_eq!(pool.receive("client", join("r1", 3)), None);
        let shed = wire.take();
        assert_eq!(shed.len(), 1, "{shed:?}");
        assert_eq!(shed[0].status, Some(RpcStatus::Unavailable));
        assert_eq!(shed[0].wire_id, Some(id(0x22, 3)));
        let (key, receipt) = pool.receive_with_receipt("client", join("r1", 4));
        assert_eq!((key, receipt.fate()), (None, Fate::Lost));

        pool.run("r1");
        let served = wire.take();
        assert_eq!(served.len(), 2, "{served:?}");
        assert!(served
            .iter()
            .all(|reply| reply.status == Some(RpcStatus::Ok)));
        assert!(matches!(
            pool.take_errors().as_slice(),
            [PoolError::Shed { key, .. }] if key == "r1"
        ));
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

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            let kept = matches!(write.change, Change::Keep { .. });
            if kept && self.failing.load(Ordering::SeqCst) {
                return Err(WriteError::Failed(Failure::new("the medium is down")));
            }
            self.inner.write(key, write)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn indexed(&self, key: &str) -> String {
            self.inner.indexed(key)
        }

        fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
            self.inner.key_of(indexed)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
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

    /// An envelope's receipt is settled once a durable checkpoint holds what
    /// its session made of it (SCE_MESH.md section 10.11), and stays pending
    /// while the store fails the write: a host that acknowledges only settled
    /// envelopes acknowledges nothing a crash would lose. The copy its sender
    /// carries again, which the duplicate window drops, drives the checkpoint
    /// that settles both, and the reply leaves once. One the pool answers
    /// without a session, and the one that ends its session, are settled too.
    #[test]
    fn an_envelope_is_settled_only_once_a_durable_checkpoint_holds_it() {
        let (clock, probe) = (Clock::new(), Arc::new(Probe::default()));
        let store = Arc::new(Flaky {
            inner: MemoryStore::new(),
            failing: AtomicBool::new(true),
        });
        let shared: Arc<dyn SessionStore> = Arc::clone(&store) as Arc<dyn SessionStore>;
        let (pool, wire) = pool_over("p1", shared, &clock, &probe);

        let (key, first) = pool.receive_with_receipt("client", join("r1", 1));
        assert_eq!(key.as_deref(), Some("r1"));
        assert_eq!(first.fate(), Fate::Pending, "its session has not run");
        pool.run("r1");
        assert_eq!(
            first.fate(),
            Fate::Pending,
            "settled though no checkpoint holding it was written"
        );
        assert!(wire.take().is_empty());

        store.failing.store(false, Ordering::SeqCst);
        let (_, again) = pool.receive_with_receipt("client", join("r1", 1));
        pool.run("r1");
        assert_eq!(first.fate(), Fate::Settled);
        assert_eq!(again.fate(), Fate::Settled);
        let replies = wire.take();
        assert_eq!(replies.len(), 1, "the reply leaves once: {replies:?}");
        assert_eq!(replies[0].wire_id, Some(id(0x22, 1)));
        assert_eq!(stored_members(&store.inner, "r1"), 1, "taken in once");
        pool.take_errors();

        let (key, unkeyed) = pool.receive_with_receipt(
            "client",
            from_client("service.request.join", r#"{"who":"x"}"#, 2, true),
        );
        assert_eq!(key, None);
        assert_eq!(
            unkeyed.fate(),
            Fate::Settled,
            "answered InvalidArgument, which receiving it again would not change"
        );
        assert_eq!(wire.take()[0].status, Some(RpcStatus::InvalidArgument));
        let (_, malformed) = pool.receive_with_receipt("client", vec![0xFF]);
        assert_eq!(malformed.fate(), Fate::Settled);

        let (_, closing) =
            pool.receive_with_receipt("client", from_client("close", r#"{"room":"r1"}"#, 3, false));
        pool.run("r1");
        assert_eq!(
            closing.fate(),
            Fate::Settled,
            "its session's end was written"
        );
        assert_eq!(store.get("r1").expect("read"), None);
    }

    /// An envelope a session took in is lost when the session is dropped
    /// before a checkpoint holding it is durable: here its write was refused
    /// by the fence of the process that took the key over. While that process
    /// holds the key, a request is answered `Unavailable`, which settles it,
    /// and an envelope that is no request is dropped and lost, so that a host
    /// acknowledges neither one its machine never took in.
    #[test]
    fn an_envelope_a_dropped_session_took_in_is_lost() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (first, first_wire) = pool("p1", &store, &clock, &probe);
        let (second, _second_wire) = pool("p2", &store, &clock, &probe);
        let (_, kept) = first.receive_with_receipt("client", join("r1", 1));
        first.run("r1");
        assert_eq!(kept.fate(), Fate::Settled);
        first_wire.take();
        clock.advance(LEASE_TTL_MS + 1);
        deliver(&second, join("r1", 2));

        let (_, refused) = first.receive_with_receipt("client", join("r1", 3));
        first.run("r1");
        assert_eq!(
            refused.fate(),
            Fate::Lost,
            "its checkpoint's write was refused by the fence"
        );
        assert!(!first.is_live("r1"));
        assert!(first_wire.take().is_empty(), "its reply never left");

        let (_, request) = first.receive_with_receipt("client", join("r1", 4));
        let (_, event) = first.receive_with_receipt(
            "client",
            from_client("remind", r#"{"room":"r1","after_ms":5}"#, 5, false),
        );
        first.run("r1");
        assert_eq!(request.fate(), Fate::Settled, "answered Unavailable");
        assert_eq!(event.fate(), Fate::Lost, "dropped while another serves r1");
        let replies = first_wire.take();
        let [reply] = replies.as_slice() else {
            panic!("one reply, got {replies:?}");
        };
        assert_eq!(reply.status, Some(RpcStatus::Unavailable));
        assert_eq!(reply.wire_id, Some(id(0x22, 4)));
        assert_eq!(
            kept.fate(),
            Fate::Settled,
            "a settled receipt stays settled"
        );
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

    /// [`ROOMS`] whose client reaches it over an `https` accepting binding under `receipt:
    /// durable` (docs/adr/0109, D10): its outbox holds `mailbox` envelopes at most, each kept
    /// `retain_ms` unless the client acknowledges it.
    fn rooms_durable(mailbox: u32, retain_ms: u64) -> Machine {
        let peers: &'static [Peer] = Box::leak(Box::new([
            Peer {
                name: "client",
                config: PeerConfig {
                    transport: "https",
                    ..CLIENT
                },
                link: PeerLink::HttpsAccept {
                    listen: "127.0.0.1:9443",
                    authenticator: crate::peers::HttpsAuthenticator::Bearer,
                    max_hold_ms: 1_000,
                    max_body_bytes: 65_536,
                    absence_ms: 60_000,
                    mailbox,
                    max_clients: 16,
                    max_response_bytes: 65_536,
                    rate: crate::peers::EdgeRates::NONE,
                    trusted_proxies: &[],
                    sign: None,
                    max_skew_ms: 300_000,
                    fresh_ms: None,
                    receipt: crate::peers::Receipt::Durable {
                        window: 256,
                        retry_ms: 1_000,
                    },
                    retain_ms,
                },
            },
            ROOMS.peers[1],
        ]));
        Machine { peers, ..ROOMS }
    }

    /// The envelopes the store's outbox of the client holds, as `(number, event)`.
    fn outboxed(store: &MemoryStore) -> Vec<(u64, String)> {
        outboxed_of(store, "client")
    }

    /// The envelopes the store's outbox of `client` holds, as `(number, event)`.
    fn outboxed_of(store: &MemoryStore, client: &str) -> Vec<(u64, String)> {
        store
            .outbox(client)
            .expect("the outbox read")
            .map(|outbox| {
                outbox
                    .entries
                    .iter()
                    .map(|entry| {
                        let envelope = Envelope::decode(&mut SceCursor::new(&entry.envelope))
                            .expect("an envelope");
                        (entry.number, envelope.event_type.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// docs/adr/0109, D10: what a session sends a durable client is appended to the client's
    /// outbox by the write that keeps the session's checkpoint, and by no other path: nothing of
    /// it reaches the transport, the checkpoint holds nothing for it, and the outboxes the server
    /// answers from hold it under the store's number. Red if `Pool::checkpoint` writes without
    /// the appends `Pool::outbound` takes out of the checkpoint.
    #[test]
    fn a_reply_to_a_durable_client_is_appended_by_the_write_of_its_checkpoint() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = pool_of(
            rooms_durable(8, 60_000),
            "p1",
            Arc::new(store.clone()),
            &clock,
            &probe,
        );
        deliver(&pool, join("r1", 1));
        assert!(
            wire.take().is_empty(),
            "nothing for the durable client went through the transport"
        );
        assert_eq!(
            outboxed(&store),
            vec![(1, "service.response.join".to_string())]
        );
        let outbox = store
            .outbox("client")
            .expect("the outbox read")
            .expect("the outbox the write opened");
        assert_eq!(outbox.entries[0].sender.as_deref(), Some("r1"));
        assert_eq!(outbox.entries[0].at_ms, T0);
        let loaded = checkpoint::load(&store, "r1")
            .expect("read")
            .expect("saved");
        assert!(
            loaded
                .checkpoint
                .endpoints
                .iter()
                .all(|section| section.router.unreleased.is_empty()),
            "the checkpoint holds nothing for the client: its outbox does"
        );
        let outboxes = pool.outboxes().expect("the pool keeps the client's outbox");
        let incarnation = outboxes
            .open("client", "ffffffffffffffffffffffffffffffff", T0)
            .expect("the client's exchange opens its outbox");
        assert_eq!(incarnation, outbox.incarnation, "the store's incarnation");
        let carried = outboxes.response("client", 65_536).expect("an outbox held");
        assert_eq!(carried.first, Some(1));
        assert_eq!(carried.body, outbox.entries[0].envelope);
        assert_eq!(
            outboxes.entries("client"),
            Some(crate::outbox::HeldOutbox {
                incarnation: outbox.incarnation.clone(),
                envelopes: vec![(1, outbox.entries[0].envelope.clone())],
            })
        );
    }

    /// docs/adr/0109, D10: `mailbox` is the durable window of the client, shared by every
    /// session: a reply admitted while the client's outbox holds `mailbox` envelopes is refused
    /// at admit as row 9 and told to the session that sent it, nothing is appended for it, and
    /// an acknowledgement makes room again. Red if `write_envelope` takes no place in the
    /// window, or `Outboxes::reserve` counts what the store holds or what is reserved apart.
    #[test]
    fn a_send_past_a_durable_clients_window_is_refused_at_admit_until_an_ack_makes_room() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, _wire) = pool_of(
            rooms_durable(2, 60_000),
            "p1",
            Arc::new(store.clone()),
            &clock,
            &probe,
        );
        deliver(&pool, join("r1", 1));
        deliver(&pool, join("r2", 2));
        assert_eq!(outboxed(&store).len(), 2, "two sessions, one window");
        assert!(lock(&probe.unhandled).is_empty());
        deliver(&pool, join("r3", 3));
        assert_eq!(
            outboxed(&store).len(),
            2,
            "a reply past the window is not appended"
        );
        assert_eq!(
            lock(&probe.unhandled).as_slice(),
            ["error.communication".to_string()],
            "the session that sent it is told, as row 9"
        );
        let outboxes = pool.outboxes().expect("the pool keeps the client's outbox");
        let incarnation = outboxes
            .open("client", "ffffffffffffffffffffffffffffffff", T0)
            .expect("the outbox opened");
        assert_eq!(
            outboxes
                .acknowledge("client", &incarnation, 1, T0)
                .expect("an acknowledgement"),
            1
        );
        deliver(&pool, join("r3", 4));
        assert_eq!(
            outboxed(&store)
                .iter()
                .map(|(number, _)| *number)
                .collect::<Vec<_>>(),
            vec![2, 3],
            "the place the acknowledgement freed was taken, numbered on"
        );
    }

    /// docs/adr/0109, D10: an outbox entry kept longer than `retain_ms` is deleted by the expiry
    /// and told to the host; the session that sent it is told as row 15 at its next run, whether
    /// it was in memory or out of it (the store made it due at once), and the write that keeps
    /// what its machine made of it takes the note. Red if `Pool::build` or `Input::Notes` raises
    /// no row, or the write of `Pool::checkpoint` does not take the notes it told.
    #[test]
    fn an_entry_past_retain_ms_expires_and_its_sender_is_told_at_its_next_run() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, _wire) = pool_of(
            rooms_durable(8, 1_500),
            "p1",
            Arc::new(store.clone()),
            &clock,
            &probe,
        );
        deliver(&pool, join("r1", 1));
        deliver(&pool, join("r2", 2));
        clock.advance(1_001);
        deliver(&pool, join("r1", 3));
        assert_eq!(pool.evict_idle(), vec!["r2".to_string()]);
        clock.advance(600);
        assert_eq!(
            pool.expire(),
            vec!["r1".to_string()],
            "the sender in memory is run"
        );
        let expired: Vec<(u64, Option<String>)> = pool
            .take_errors()
            .into_iter()
            .filter_map(|error| match error {
                PoolError::OutboxExpired {
                    client,
                    number,
                    sender,
                    ..
                } if client == "client" => Some((number, sender)),
                _ => None,
            })
            .collect();
        assert_eq!(
            expired,
            vec![(1, Some("r1".to_string())), (2, Some("r2".to_string()))]
        );
        assert_eq!(
            outboxed(&store)
                .iter()
                .map(|(number, _)| *number)
                .collect::<Vec<_>>(),
            vec![3],
            "the young entry stays"
        );
        pool.run("r1");
        assert!(
            pool.due().contains(&"r2".to_string()),
            "the sender out of memory is due at once"
        );
        pool.run("r2");
        assert_eq!(
            lock(&probe.unhandled).as_slice(),
            [
                "error.communication".to_string(),
                "error.communication".to_string()
            ],
            "each sender is told once"
        );
        for key in ["r1", "r2"] {
            assert!(
                store.notes(key).expect("the notes").is_empty(),
                "the write of {key} took its note"
            );
        }
    }

    /// docs/adr/0109, D10: an orderly stop admits nothing more, saves every session in memory as
    /// a clean checkpoint and releases its lease; a pool started on the store after it restores
    /// each without `restored`. Red if `Pool::stop` takes its checkpoints as `Take::IfChanged`,
    /// or leaves a session in memory.
    #[test]
    fn a_stop_saves_every_session_clean_and_a_start_after_it_raises_no_restored() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (first, wire) = pool("p1", &store, &clock, &probe);
        deliver(&first, join("r1", 1));
        deliver(&first, join("r2", 2));
        wire.take();
        let report = first.stop(Instant::now() + Duration::from_secs(5));
        let mut saved = report.saved.clone();
        saved.sort();
        assert_eq!(saved, vec!["r1".to_string(), "r2".to_string()]);
        assert!(report.not_saved.is_empty(), "{:?}", report.not_saved);
        for key in ["r1", "r2"] {
            assert!(!first.is_live(key), "{key} is dropped");
            let loaded = checkpoint::load(&store, key).expect("read").expect("saved");
            assert!(loaded.checkpoint.clean, "{key}'s last checkpoint is clean");
        }
        assert_eq!(
            first.receive("client", join("r3", 3)),
            None,
            "a stopped pool admits nothing"
        );
        assert_eq!(
            wire.take()
                .iter()
                .map(|sent| sent.status)
                .collect::<Vec<_>>(),
            vec![Some(RpcStatus::Unavailable)],
            "a request it does not admit is answered as unavailable"
        );

        let (again, _) = pool("p1", &store, &clock, &probe);
        deliver(&again, join("r1", 4));
        deliver(&again, join("r2", 5));
        assert!(
            lock(&probe.unhandled).is_empty(),
            "a restore from a clean checkpoint raises no restored row: {:?}",
            lock(&probe.unhandled)
        );
        assert_eq!(probe.restored.load(Ordering::SeqCst), 2);
    }

    /// The pool a [`StopsWhileNaming`] store stops, once it is armed.
    type Stopper = Mutex<Option<std::sync::Weak<Pool<Rooms, Wire, Clock>>>>;

    /// A store that stops the pool it is armed with the first time it is asked which key an
    /// entry names: a stop decided while `Pool::due` reads the store, outside the table's lock.
    struct StopsWhileNaming {
        inner: MemoryStore,
        pool: Stopper,
    }

    impl SessionStore for StopsWhileNaming {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(key)
        }

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            self.inner.write(key, write)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.inner.due_before(ms, limit)
        }

        fn indexed(&self, key: &str) -> String {
            self.inner.indexed(key)
        }

        fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
            let armed = lock(&self.pool).take();
            if let Some(pool) = armed.and_then(|pool| pool.upgrade()) {
                pool.stop(Instant::now() + Duration::from_secs(5));
            }
            self.inner.key_of(indexed)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
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

    /// docs/adr/0109, D10: a stop decided while `Pool::due` names from the store what came due,
    /// outside the table's lock, admits no session it named: the stop is read with the table
    /// held, as every admit reads it, and the session stays out of memory for the process that
    /// serves the key next. Red if `Pool::due` reads the stop only before it reads the store.
    #[test]
    fn a_stop_decided_while_due_reads_the_store_admits_nothing_it_named() {
        let (clock, probe) = (Clock::new(), Arc::new(Probe::default()));
        let store = Arc::new(StopsWhileNaming {
            inner: MemoryStore::new(),
            pool: Mutex::new(None),
        });
        let (pool, _wire) = pool_over(
            "p1",
            Arc::clone(&store) as Arc<dyn SessionStore>,
            &clock,
            &probe,
        );
        deliver(
            &pool,
            from_client("remind", r#"{"room":"r1","after_ms":5000}"#, 1, false),
        );
        clock.advance(1_000);
        assert_eq!(pool.evict_idle(), vec!["r1".to_string()]);
        clock.advance(4_000);
        let pool = Arc::new(pool);
        *lock(&store.pool) = Some(Arc::downgrade(&pool));
        assert!(
            pool.due().is_empty(),
            "a session named after the stop was decided was admitted"
        );
        assert!(pool.health().stopped(), "{:?}", pool.health());
        assert!(
            hold(&pool.table).is_empty(),
            "the stopped pool holds a session it named"
        );
        assert!(!pool.is_live("r1"));
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

    /// A pool of `machine` as [`pool_of`] makes one, reporting to a recording adapter it answers
    /// with (docs/adr/0109, D16).
    fn pool_recorded(
        machine: Machine,
        owner: &str,
        store: Arc<dyn SessionStore>,
        clock: &Clock,
        probe: &Arc<Probe>,
    ) -> (Pool<Rooms, Wire, Clock>, Wire, Arc<Recording>) {
        let recording = Arc::new(Recording::new());
        let wire = Wire::default();
        let pool = Pool::observed(
            machine,
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
            Arc::clone(&recording) as Arc<dyn Metrics>,
        )
        .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
        pool.peer_ready("client");
        pool.peer_ready("other");
        (pool, wire, recording)
    }

    /// docs/adr/0109, D16: each session the pool brings into memory is a load, one restored from
    /// a clean checkpoint is a clean restore, one saved and dropped as idle is an idle eviction,
    /// and the live sessions rise and fall with them. Red if `Pool::run`, `Pool::load`,
    /// `Pool::build` or `Pool::evict` stops reporting what it did.
    #[test]
    fn a_pool_reports_each_session_it_loads_restores_and_evicts() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire, recording) =
            pool_recorded(ROOMS, "p1", Arc::new(store.clone()), &clock, &probe);
        deliver(&pool, join("r1", 1));
        deliver(&pool, join("r2", 2));
        wire.take();
        assert_eq!(recording.counted(Counter::Loads), 2);
        assert_eq!(recording.level(Gauge::LiveSessions), 2);
        assert_eq!(
            recording.counted_in(&sce_rust_metrics::POOL_RESTORES),
            0,
            "both were started"
        );

        clock.advance(1_000);
        let mut evicted = pool.evict_idle();
        evicted.sort();
        assert_eq!(evicted, vec!["r1".to_string(), "r2".to_string()]);
        assert_eq!(
            recording.counted(Counter::Evictions {
                reason: Eviction::Idle
            }),
            2
        );
        assert_eq!(recording.level(Gauge::LiveSessions), 0);

        deliver(&pool, join("r1", 3));
        assert_eq!(recording.counted(Counter::Loads), 3);
        assert_eq!(
            recording.counted(Counter::Restores {
                checkpoint: RestoredFrom::Clean
            }),
            1
        );
        assert_eq!(
            recording.counted(Counter::Restores {
                checkpoint: RestoredFrom::NotClean
            }),
            0
        );
        assert_eq!(recording.level(Gauge::LiveSessions), 1);
        assert_eq!(recording.peak(Gauge::LiveSessions), 2);
    }

    /// docs/adr/0109, D16: a session restored from a checkpoint written while it ran (another
    /// process took its key over) is a restore that is not clean, the one that raises `restored`;
    /// the process that lost the key reports the renewal the store refused, and the session it
    /// dropped leaves its live sessions. Red if `Pool::build` reports every restore as clean, or
    /// `Pool::renew_leases` reports no refusal.
    #[test]
    fn a_pool_reports_a_restore_that_raises_restored_and_a_renewal_refused() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (first, first_wire, first_recording) =
            pool_recorded(ROOMS, "p1", Arc::new(store.clone()), &clock, &probe);
        let (second, second_wire, second_recording) =
            pool_recorded(ROOMS, "p2", Arc::new(store.clone()), &clock, &probe);
        deliver(&first, join("r1", 1));
        first_wire.take();
        clock.advance(LEASE_TTL_MS + 1);
        deliver(&second, join("r1", 2));
        second_wire.take();
        assert_eq!(
            second_recording.counted(Counter::Restores {
                checkpoint: RestoredFrom::NotClean
            }),
            1
        );

        assert_eq!(first.renew_leases(), vec!["r1".to_string()]);
        assert_eq!(first_recording.counted(Counter::LeaseRenewalsRefused), 1);
        first.run("r1");
        assert_eq!(
            first_recording.level(Gauge::LiveSessions),
            0,
            "the session whose key it lost left memory"
        );
        assert_eq!(second_recording.counted(Counter::LeaseRenewalsRefused), 0);
        assert_eq!(second_recording.level(Gauge::LiveSessions), 1);
    }

    /// docs/adr/0109, D16: an envelope shed at `max_live` and one shed at `max_inbox` are each
    /// reported under their own bound, and nothing else is reported shed. Red if `Pool::shed`
    /// reports another bound than the one `Pool::try_post` met.
    #[test]
    fn a_pool_reports_each_envelope_it_sheds_by_the_bound_it_met() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire, recording) =
            pool_recorded(rooms_bounded(1, 1), "p1", Arc::new(store), &clock, &probe);
        // r1 waits for an answer from another machine: it is not quiet, so no room is made.
        deliver(&pool, from_client("ask", r#"{"room":"r1"}"#, 1, false));
        assert_eq!(pool.receive("client", join("r2", 2)), None);
        assert_eq!(
            recording.counted(Counter::PoolSheds {
                reason: PoolShed::MaxLive
            }),
            1
        );
        assert_eq!(
            pool.receive("client", join("r1", 3)),
            Some("r1".to_string())
        );
        assert_eq!(pool.receive("client", join("r1", 4)), None);
        assert_eq!(
            recording.counted(Counter::PoolSheds {
                reason: PoolShed::MaxInbox
            }),
            1
        );
        assert_eq!(recording.counted_in(&sce_rust_metrics::POOL_SHEDS), 2);
        pool.run("r1");
        wire.take();
        assert_eq!(recording.counted_in(&sce_rust_metrics::POOL_SHEDS), 2);
    }

    /// docs/adr/0109, D16: the process holds its durable client's outbox and reports its depth
    /// as the envelopes it holds, through reads, appends, an acknowledgement and an expiry, and
    /// each entry the expiry deleted. Red if `Outboxes` moves the depth in one of those paths and
    /// not in its reverse, or `Outboxes::expire` reports no expiry.
    #[test]
    fn a_pool_reports_its_durable_clients_outbox_depth_and_expiries() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, _wire, recording) = pool_recorded(
            rooms_durable(8, 1_500),
            "p1",
            Arc::new(store.clone()),
            &clock,
            &probe,
        );
        let outboxes = Arc::clone(pool.outboxes().expect("the client's outbox"));
        let held = || {
            outboxes
                .entries("client")
                .map_or(0, |kept| kept.envelopes.len() as i64)
        };
        deliver(&pool, join("r1", 1));
        deliver(&pool, join("r2", 2));
        let incarnation = outboxes
            .open("client", "ffffffffffffffffffffffffffffffff", T0)
            .expect("the outbox opened");
        assert_eq!(held(), 2);
        assert_eq!(recording.level(Gauge::OutboxDepth), held());

        clock.advance(1_001);
        deliver(&pool, join("r1", 3));
        assert_eq!(held(), 3);
        assert_eq!(recording.level(Gauge::OutboxDepth), held());

        outboxes
            .acknowledge("client", &incarnation, 1, clock.at())
            .expect("an acknowledgement");
        assert_eq!(held(), 2);
        assert_eq!(recording.level(Gauge::OutboxDepth), held());

        clock.advance(600);
        assert_eq!(
            pool.expire(),
            vec!["r2".to_string()],
            "only r2's entry is past retain_ms; r1's first was acknowledged"
        );
        assert_eq!(recording.counted(Counter::OutboxExpiries), 1);
        assert_eq!(held(), 1);
        assert_eq!(recording.level(Gauge::OutboxDepth), held());
    }

    /// A store whose listing of dues fails while `failing` is set, as a medium that cannot be
    /// read: what a readiness check reads.
    struct Unlisted {
        inner: MemoryStore,
        failing: AtomicBool,
    }

    impl SessionStore for Unlisted {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            self.inner.get(key)
        }

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            self.inner.write(key, write)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            if self.failing.load(Ordering::SeqCst) {
                return Err(Failure::new("the medium cannot be read"));
            }
            self.inner.due_before(ms, limit)
        }

        fn indexed(&self, key: &str) -> String {
            self.inner.indexed(key)
        }

        fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
            self.inner.key_of(indexed)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.inner.indexed_after(after, limit)
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

    /// docs/adr/0109, D16: a pool accepts and reaches its store while it serves, is unready while
    /// its store cannot be read, says it is stopping while a stop waits for a run, and stopped
    /// once the stop ended, after which an envelope is shed as the pool stopping. Red if
    /// `Pool::health` reads its state from another flag than the stop sets, or does not ask the
    /// store.
    #[test]
    fn a_pool_says_whether_it_accepts_stops_and_reaches_its_store() {
        let (clock, probe) = (Clock::new(), Arc::new(Probe::default()));
        let store = Arc::new(Unlisted {
            inner: MemoryStore::new(),
            failing: AtomicBool::new(false),
        });
        let (pool, wire, recording) = pool_recorded(
            ROOMS,
            "p1",
            Arc::clone(&store) as Arc<dyn SessionStore>,
            &clock,
            &probe,
        );
        assert_eq!(
            pool.health(),
            Health {
                accepting: true,
                stopping: false,
                store_reachable: true,
            }
        );
        assert!(pool.health().ready());
        store.failing.store(true, Ordering::SeqCst);
        assert!(!pool.health().store_reachable);
        assert!(
            !pool.health().ready(),
            "a pool whose store fails is unready"
        );
        store.failing.store(false, Ordering::SeqCst);

        deliver(&pool, join("r1", 1));
        wire.take();
        let pool = Arc::new(pool);
        let slot = hold(&pool.table).get("r1").cloned().expect("r1's slot");
        // A run of r1 is going on: the stop waits for it until its deadline.
        let running = hold(&slot.live);
        let stopper = {
            let pool = Arc::clone(&pool);
            std::thread::spawn(move || pool.stop(Instant::now() + Duration::from_millis(300)))
        };
        let until = Instant::now() + Duration::from_secs(5);
        let mut seen = pool.health();
        while !seen.stopping && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
            seen = pool.health();
        }
        assert!(seen.stopping && !seen.accepting, "{seen:?}");
        let report = stopper.join().expect("the stop did not panic");
        drop(running);
        assert_eq!(report.not_saved.len(), 1, "{:?}", report.not_saved);
        let stopped = pool.health();
        assert!(stopped.stopped(), "{stopped:?}");
        assert!(!stopped.ready());

        assert_eq!(pool.receive("client", join("r2", 2)), None);
        assert_eq!(
            recording.counted(Counter::PoolSheds {
                reason: PoolShed::Stopping
            }),
            1
        );
    }

    /// The open machine `phone`, whose instances post to the rooms over `https` under
    /// `receipt: durable` and are judged by their completion records (docs/adr/0113, D5): one
    /// of them is a member at once, as a member a minute after its last exchange.
    const PHONE: OpenPeer = OpenPeer {
        peer: Peer {
            name: "phone",
            config: PeerConfig {
                transport: "https",
                responders: Responders::Instance,
                ..CLIENT
            },
            link: PeerLink::HttpsAccept {
                listen: "0.0.0.0:8443",
                authenticator: crate::peers::HttpsAuthenticator::Key,
                max_hold_ms: 30_000,
                max_body_bytes: 1_048_576,
                absence_ms: 120_000,
                mailbox: 1024,
                max_clients: 100_000,
                max_response_bytes: 1_048_576,
                rate: crate::peers::EdgeRates::DEFAULT,
                trusted_proxies: &[],
                sign: None,
                max_skew_ms: 300_000,
                fresh_ms: None,
                receipt: crate::peers::Receipt::Durable {
                    window: 1024,
                    retry_ms: 1_000,
                },
                retain_ms: 60_000,
            },
        },
        members: crate::peers::Members {
            lease_ms: 60_000,
            provisional_ms: 60_000,
            max_instances: 1,
        },
    };

    /// The rooms, bound to the instances of [`PHONE`] as well as to the client.
    const OPEN_ROOMS: Machine = Machine {
        open_peers: &[PHONE],
        ..ROOMS
    };

    /// A pool of [`OPEN_ROOMS`] over `store`, which keeps the members of `phone` too.
    fn open_pool(
        store: &MemoryStore,
        clock: &Clock,
        probe: &Arc<Probe>,
    ) -> (Pool<Rooms, Wire, Clock>, Wire) {
        let wire = Wire::default();
        let pool = Pool::with_members(
            OPEN_ROOMS,
            PoolConfig {
                owner: "p1".to_string(),
                lease_ttl_ms: LEASE_TTL_MS,
                renew_every_ms: 3_000,
            },
            Rooms {
                probe: Arc::clone(probe),
            },
            Arc::new(store.clone()),
            wire.clone(),
            clock.clone(),
        )
        .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
        pool.peer_ready("client");
        (pool, wire)
    }

    /// The request the instance `instance` of `phone` numbered `number` in its incarnation: a
    /// join of `room`, under the wire invoke id `id(0x44, number)`.
    fn from_instance(instance: &str, room: &str, number: u32) -> Vec<u8> {
        let (envelope_id, wire_id) = (id(0x33, number), id(0x44, number));
        let data = alloc::format!(r#"{{"room":"{room}"}}"#);
        Envelope {
            id: (&envelope_id).into(),
            source: within("phone").unwrap(),
            instance: Some(within(instance).unwrap()),
            event_type: within("service.request.join").unwrap(),
            pattern: PatternKind::RpcRequest,
            datacontenttype: PayloadCodec::Json,
            data: within(data.as_bytes()).unwrap(),
            invoke_id: Some((&wire_id).into()),
            sequence_no: Some(u64::from(number)),
            ..Envelope::new()
        }
        .encode_to_vec()
    }

    /// The incarnation `store` makes `member` a member under, at the clock's moment: the offer a
    /// join with `nonce` drew, confirmed by the exchange that names it.
    fn joined(store: &MemoryStore, member: &str, nonce: &str, clock: &Clock) -> u64 {
        let terms = PHONE.members.terms();
        let offered = store
            .offer(member, nonce, terms.provisional_ms, clock.at())
            .expect("a join is answered");
        match store
            .admit(member, offered.incarnation, 1, terms, clock.at())
            .expect("an exchange is judged")
        {
            sce_rust_session_store::Admission::Confirmed { .. } => offered.incarnation,
            other => panic!("the offer of {member} was not confirmed: {other:?}"),
        }
    }

    /// docs/adr/0113, D5, and docs/formal/OpenInstances.tla, `OpenInstances_without_recheck`:
    /// the write that keeps a session's checkpoint names the incarnation each envelope it took in
    /// was taken in under, and keeps nothing once the store no longer holds one. The session is
    /// restored from what the store keeps and what it took in is lost to its sender, who carries
    /// it again; an envelope its session then finds fenced is told as fenced, which the edge
    /// answers `412`. Red if the pool's write names no incarnation: the join made under the
    /// ended incarnation is kept, and the room holds two members.
    #[test]
    fn a_run_under_an_incarnation_that_ended_is_not_kept_and_its_session_is_restored() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = open_pool(&store, &clock, &probe);
        let first = joined(&store, "phone/a1", "n1", &clock);
        pool.peer_ready("phone/a1");
        let standing = Standing {
            incarnation: first,
            mark: 1,
        };
        let (key, taken) =
            pool.receive_member("phone/a1", first, standing, from_instance("a1", "r1", 1));
        assert_eq!(key.as_deref(), Some("r1"));
        pool.run("r1");
        assert_eq!(taken.fate(), Fate::Settled);
        assert_eq!(stored_members(&store, "r1"), 1);
        assert!(
            wire.take().is_empty(),
            "an instance's replies are kept in its outbox"
        );
        let replies = outboxed_of(&store, "phone/a1");
        assert_eq!(replies.len(), 1, "{replies:?}");

        // The instance joins again, which ends its first incarnation, while its next request,
        // taken in by the edge under the first, waits for its session.
        let second = joined(&store, "phone/a1", "n2", &clock);
        let (_, late) =
            pool.receive_member("phone/a1", first, standing, from_instance("a1", "r1", 2));
        pool.run("r1");
        assert_eq!(late.fate(), Fate::Lost, "its sender carries it again");
        assert_eq!(
            stored_members(&store, "r1"),
            1,
            "nothing of the run under the ended incarnation is kept"
        );
        assert!(
            pool.take_errors().iter().any(|error| matches!(
                error,
                PoolError::Fenced { key, incarnation, current }
                    if key == "r1" && *incarnation == first && *current == Some(second)
            )),
            "the refused write is told"
        );
        assert_eq!(
            outboxed_of(&store, "phone/a1").len(),
            1,
            "no reply left for what was not kept"
        );

        // Carried again with the store's latest answer, its restored session fences it.
        let (_, again) = pool.receive_member(
            "phone/a1",
            first,
            Standing {
                incarnation: second,
                mark: 1,
            },
            from_instance("a1", "r1", 2),
        );
        pool.run("r1");
        assert_eq!(again.fate(), Fate::Fenced { current: second });
        assert_eq!(stored_members(&store, "r1"), 1);
    }

    /// docs/adr/0113, D4 and D6: what concerns one instance (its readiness, the store's answer
    /// for its member) reaches only the sessions whose routers bind it, and the end of its
    /// membership unbinds it there, so the next instance takes its place among the deployment's
    /// `max_instances`. Red if readiness is told to every session, or if an ended member stays
    /// bound: the next instance's request is then shed.
    #[test]
    fn an_instance_reaches_only_the_sessions_that_bind_it_and_its_end_unbinds_it() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, _wire) = open_pool(&store, &clock, &probe);
        deliver(&pool, join("r2", 1));
        let a1 = joined(&store, "phone/a1", "n1", &clock);
        pool.peer_ready("phone/a1");
        let (key, _) = pool.receive_member(
            "phone/a1",
            a1,
            Standing {
                incarnation: a1,
                mark: 1,
            },
            from_instance("a1", "r1", 1),
        );
        pool.run(key.as_deref().expect("placed"));
        assert_eq!(
            pool.peer_not_ready("phone/a1"),
            vec!["r1".to_string()],
            "r2 binds no instance and is not told"
        );
        assert_eq!(pool.peer_ready("phone/a1"), vec!["r1".to_string()]);
        assert_eq!(
            pool.stand(
                "phone/a1",
                Standing {
                    incarnation: a1,
                    mark: 2
                }
            ),
            vec!["r1".to_string()]
        );
        pool.run("r1");
        assert!(pool.end_members().is_empty(), "the member stands");

        clock.advance(PHONE.members.lease_ms + 1);
        assert_eq!(pool.end_members(), vec!["r1".to_string()]);
        pool.run("r1");
        assert!(
            pool.peer_ready("phone/a1").is_empty(),
            "no session binds it any more"
        );
        let b2 = joined(&store, "phone/b2", "n1", &clock);
        let (_, next) = pool.receive_member(
            "phone/b2",
            b2,
            Standing {
                incarnation: b2,
                mark: 1,
            },
            from_instance("b2", "r1", 1),
        );
        pool.run("r1");
        assert_eq!(next.fate(), Fate::Settled, "{:?}", pool.take_errors());
        assert_eq!(stored_members(&store, "r1"), 2);
    }

    /// docs/adr/0113, D4, and docs/adr/0109, D10: an instance of an open machine whose binding is
    /// durable keeps its replies in an outbox of its own, which ends with its membership: what it
    /// still owes is let go and told to the session that sent it, as SCE_MESH.md section 16.7 row
    /// 15, and the outbox is closed, so the instance's next incarnation is handed nothing the one
    /// before was owed. Red if `end_members` leaves the outbox standing, or if the pool sends an
    /// instance's reply on the transport instead of keeping it.
    #[test]
    fn an_instances_outbox_ends_with_its_membership() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, wire) = open_pool(&store, &clock, &probe);
        let a1 = joined(&store, "phone/a1", "n1", &clock);
        pool.peer_ready("phone/a1");
        let (key, taken) = pool.receive_member(
            "phone/a1",
            a1,
            Standing {
                incarnation: a1,
                mark: 1,
            },
            from_instance("a1", "r1", 1),
        );
        pool.run(key.as_deref().expect("placed"));
        assert_eq!(taken.fate(), Fate::Settled, "{:?}", pool.take_errors());
        assert!(wire.take().is_empty(), "kept in its outbox, not sent");
        assert_eq!(outboxed_of(&store, "phone/a1").len(), 1);

        clock.advance(PHONE.members.lease_ms + 1);
        let run = pool.end_members();
        assert!(run.contains(&"r1".to_string()), "{run:?}");
        assert!(
            store.outbox("phone/a1").expect("the outbox read").is_none(),
            "the outbox ended with the member"
        );
        assert!(
            pool.take_errors().iter().any(|error| matches!(
                error,
                PoolError::OutboxExpired { client, sender, .. }
                    if client == "phone/a1" && sender.as_deref() == Some("r1")
            )),
            "what it still owed is told"
        );
    }

    /// docs/adr/0113, D4: a pool that stopped ends no member whose lease ran out, since it can
    /// tell no session of the end; the store keeps the member for a process that serves the
    /// sessions binding it. Red if `Pool::end_members` reads no stop: the store then ends the
    /// member, and the session saved by the stop binds it with nothing left to end it.
    #[test]
    fn a_stopped_pool_ends_no_member() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let (pool, _wire) = open_pool(&store, &clock, &probe);
        let a1 = joined(&store, "phone/a1", "n1", &clock);
        pool.peer_ready("phone/a1");
        let (key, _) = pool.receive_member(
            "phone/a1",
            a1,
            Standing {
                incarnation: a1,
                mark: 1,
            },
            from_instance("a1", "r1", 1),
        );
        pool.run(key.as_deref().expect("placed"));
        assert_eq!(
            pool.stop(Instant::now() + Duration::from_secs(5)).saved,
            ["r1"]
        );
        clock.advance(PHONE.members.lease_ms + 1);
        assert!(
            pool.end_members().is_empty(),
            "a stopped pool told a session"
        );
        let ended = store
            .end_expired(clock.at(), WAKE_BATCH)
            .expect("the store answers");
        assert_eq!(
            ended
                .iter()
                .map(|ended| ended.member.as_str())
                .collect::<Vec<_>>(),
            ["phone/a1"],
            "the stopped pool ended the member in the store"
        );
    }

    /// docs/adr/0113, D4: a pool keeps the members of the open machines its machine is bound to,
    /// so one built without a store of members is refused. Red if `Pool::observed` builds it.
    #[test]
    fn a_machine_bound_to_an_open_machine_is_pooled_only_with_its_members() {
        let refused = Pool::new(
            OPEN_ROOMS,
            PoolConfig {
                owner: "p1".to_string(),
                lease_ttl_ms: LEASE_TTL_MS,
                renew_every_ms: 3_000,
            },
            Rooms {
                probe: Arc::new(Probe::default()),
            },
            Arc::new(MemoryStore::new()),
            Wire::default(),
            Clock::new(),
        );
        assert!(matches!(
            refused,
            Err(PoolConfigError::NoMembers { machine }) if machine == "rooms"
        ));
    }

    /// The rooms, bound to the instances of [`PHONE`] under a `transport` receipt.
    const TRANSPORT_ROOMS: Machine = Machine {
        open_peers: &[OpenPeer {
            peer: Peer {
                link: PeerLink::HttpsAccept {
                    listen: "0.0.0.0:8443",
                    authenticator: crate::peers::HttpsAuthenticator::Key,
                    max_hold_ms: 30_000,
                    max_body_bytes: 1_048_576,
                    absence_ms: 120_000,
                    mailbox: 1024,
                    max_clients: 100_000,
                    max_response_bytes: 1_048_576,
                    rate: crate::peers::EdgeRates::DEFAULT,
                    trusted_proxies: &[],
                    sign: None,
                    max_skew_ms: 300_000,
                    fresh_ms: None,
                    receipt: crate::peers::Receipt::Transport,
                    retain_ms: 60_000,
                },
                ..PHONE.peer
            },
            ..PHONE
        }],
        ..ROOMS
    };

    /// A pool of [`TRANSPORT_ROOMS`] over `store`, which keeps the members of `phone` too.
    fn transport_pool(
        store: &MemoryStore,
        clock: &Clock,
        probe: &Arc<Probe>,
    ) -> Pool<Rooms, Wire, Clock> {
        Pool::with_members(
            TRANSPORT_ROOMS,
            PoolConfig {
                owner: "p1".to_string(),
                lease_ttl_ms: LEASE_TTL_MS,
                renew_every_ms: 3_000,
            },
            Rooms {
                probe: Arc::clone(probe),
            },
            Arc::new(store.clone()),
            Wire::default(),
            clock.clone(),
        )
        .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"))
    }

    /// docs/adr/0113, D5: an instance whose binding takes a `transport` receipt is admitted by its
    /// edge and taken in by the section 10.5 window, whose guarantee it has; completion records
    /// are a `durable` receipt's. Red if the pool hands such an envelope to the records: the
    /// router refuses it as not durable, nothing is taken in, and the room stays empty though its
    /// envelope is settled.
    #[test]
    fn an_instance_under_a_transport_receipt_is_taken_in_by_the_window() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let pool = transport_pool(&store, &clock, &probe);
        let incarnation = joined(&store, "phone/a1", "n1", &clock);
        let (key, taken) = pool.receive_member(
            "phone/a1",
            incarnation,
            Standing {
                incarnation,
                mark: 1,
            },
            from_instance("a1", "r1", 1),
        );
        pool.run(key.as_deref().expect("placed"));
        assert_eq!(taken.fate(), Fate::Settled);
        assert_eq!(stored_members(&store, "r1"), 1, "{:?}", pool.take_errors());
    }

    /// docs/adr/0113, D6: an instance under a `transport` receipt is held to `max_instances` as
    /// one judged by its records is: the envelope of one past the bound is shed, lost to its edge,
    /// which answers `503` so that its sender carries it again, and no instance that stands is put
    /// out for it. Red if the pool takes the router's refusal as a host error and settles the
    /// envelope: the edge then answers `200` for what no session took in.
    #[test]
    fn an_instance_under_a_transport_receipt_past_the_bound_is_shed() {
        let (store, clock, probe) = (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
        let pool = transport_pool(&store, &clock, &probe);
        let a1 = joined(&store, "phone/a1", "n1", &clock);
        let (key, first) = pool.receive_member(
            "phone/a1",
            a1,
            Standing {
                incarnation: a1,
                mark: 1,
            },
            from_instance("a1", "r1", 1),
        );
        pool.run(key.as_deref().expect("placed"));
        assert_eq!(first.fate(), Fate::Settled);
        // The session r1 binds a1, as many instances as the deployment's `max_instances` allows;
        // the edge of another instance admitted it under an incarnation of its own.
        let (_, past) = pool.receive_member(
            "phone/b2",
            a1 + 1,
            Standing {
                incarnation: a1 + 1,
                mark: 1,
            },
            from_instance("b2", "r1", 1),
        );
        pool.run("r1");
        assert_eq!(past.fate(), Fate::Lost, "shed, and carried again");
        assert!(
            pool.take_errors().iter().any(|error| matches!(
                error,
                PoolError::Endpoint {
                    key,
                    error: RouterError::InstancesFull { .. },
                } if key == "r1"
            )),
            "the shed is told"
        );
        assert_eq!(stored_members(&store, "r1"), 1, "a1 is not put out");
    }

    /// The loom models of who holds the pool's locks, and when (docs/adr/0115, D13; the model
    /// `SessionPool`, docs/formal/SessionPool.tla): a stop racing a post, the wake of a session
    /// that came due, a tell to the sessions that bind an instance, and an entry of a followed
    /// journal handed to the session its key names, under every
    /// interleaving, within three preemptions, of the locks, the flags and the slots' `Arc` of
    /// `sync`, which this build takes from loom. What each asserts
    /// holds under every schedule: once `Pool::stop` returned nothing more is admitted, every
    /// input admitted before it was applied or answered once, and no session is left in memory.
    ///
    /// Bounded at three preemptions, as the bounded inbox ring's models are
    /// (`backends/rust/runtime/src/inbox/ring.rs`). Built only as the crate's tests under
    /// `--cfg loom` and run by name, `pool::tests::loom_models` (`scripts/gates/forge-rust.sh`
    /// gives the command). The pool's other tests are not run in that build: loom's locks exist
    /// only inside a model.
    #[cfg(all(loom, test))]
    mod loom_models {
        use super::following::{say, view_of, views};
        use super::*;
        use loom::thread;

        /// Run `body` under every interleaving within the preemption bound.
        fn model(body: impl Fn() + Sync + Send + 'static) {
            let mut builder = loom::model::Builder::new();
            builder.preemption_bound = Some(3);
            builder.check(body);
        }

        /// The client's `n`th envelope, a `remind` of `room` due at once: applied, the room
        /// sends one `reminded`; not a request, so one the pool does not take in is told to
        /// the host.
        fn remind_now(room: &str, n: u32) -> Vec<u8> {
            from_client(
                "remind",
                &alloc::format!(r#"{{"room":"{room}","after_ms":0}}"#),
                n,
                false,
            )
        }

        /// How many reminders of `room` the transport was handed.
        fn reminded(sent: &[Sent], room: &str) -> usize {
            let data = alloc::format!(r#"{{"room":"{room}"}}"#);
            sent.iter()
                .filter(|sent| sent.event == "reminded" && sent.data == data)
                .count()
        }

        /// How many envelopes for `key` the pool told the host it shed or dropped.
        fn told(errors: &[PoolError], key: &str) -> usize {
            errors
                .iter()
                .filter(|error| {
                    matches!(
                        error,
                        PoolError::Shed { key: of, .. } | PoolError::Dropped { key: of, .. }
                            if of == key
                    )
                })
                .count()
        }

        /// No session of `key` is in memory, and no slot the table holds has an input
        /// waiting: the stop left nothing behind it.
        fn assert_nothing_left<F: SessionFactory<Wire, Clock>>(
            pool: &Pool<F, Wire, Clock>,
            key: &str,
        ) {
            assert!(
                !pool.is_live(key),
                "a session was in memory after the stop returned"
            );
            assert!(
                pool.slots().iter().all(|slot| hold(&slot.inbox).is_empty()),
                "an input waits in a pool that stopped"
            );
        }

        /// A post for r1 races a stop. Admitted before the stop, it is applied, by the poster's
        /// run or the stop's drain, and the stop saves the session and drops it; refused, it is
        /// told to the host. Either way once, and a post made after the stop returned is refused.
        /// Fails if a post reads the stop before it holds the table: it is then admitted after
        /// the stop drained and saved, and its session loaded after the stop returned.
        #[test]
        fn a_post_racing_a_stop_is_served_before_it_or_refused_once() {
            model(|| {
                let (store, clock, probe) =
                    (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
                let (pool, wire) = pool("p1", &store, &clock, &probe);
                let pool = loom::sync::Arc::new(pool);
                let poster = {
                    let pool = loom::sync::Arc::clone(&pool);
                    thread::spawn(move || {
                        if let Some(key) = pool.receive("client", remind_now("r1", 1)) {
                            pool.run(&key);
                        }
                    })
                };
                pool.stop(Instant::now() + Duration::from_secs(30));
                assert_eq!(
                    pool.receive("client", remind_now("r2", 2)),
                    None,
                    "a post made after the stop returned was admitted"
                );
                poster.join().expect("the poster");
                assert_nothing_left(&pool, "r1");
                let errors = pool.take_errors();
                assert_eq!(
                    reminded(&wire.take(), "r1") + told(&errors, "r1"),
                    1,
                    "the post was not applied or told once: {errors:?}"
                );
                assert_eq!(told(&errors, "r2"), 1, "{errors:?}");
            });
        }

        /// r1 is out of memory and its reminder came due; a call of `Pool::due` and the run of
        /// what it names race a stop. The wake is admitted before the stop, and the stop's drain
        /// or the waker's run fires the reminder and the stop saves the session; or it is not,
        /// and the store still lists the reminder for the process that serves the key next.
        /// Either way once, and a call of `Pool::due` after the stop returned wakes nothing.
        /// Fails if `Pool::due` reads the stop only before it reads the store: the wake is then
        /// admitted after the stop drained and saved, and its session loaded after it returned.
        #[test]
        fn a_wake_racing_a_stop_fires_before_it_or_stays_due() {
            model(|| {
                let (store, clock, probe) =
                    (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
                let (pool, wire) = pool("p1", &store, &clock, &probe);
                deliver(
                    &pool,
                    from_client("remind", r#"{"room":"r1","after_ms":5000}"#, 1, false),
                );
                clock.advance(1_000);
                assert_eq!(pool.evict_idle(), vec!["r1".to_string()]);
                clock.advance(4_000);
                wire.take();
                let pool = loom::sync::Arc::new(pool);
                let waker = {
                    let pool = loom::sync::Arc::clone(&pool);
                    thread::spawn(move || {
                        for key in pool.due() {
                            pool.run(&key);
                        }
                    })
                };
                pool.stop(Instant::now() + Duration::from_secs(30));
                assert!(
                    pool.due().is_empty(),
                    "a wake made after the stop returned was admitted"
                );
                waker.join().expect("the waker");
                assert_nothing_left(&pool, "r1");
                let fired = reminded(&wire.take(), "r1");
                let listed = store
                    .due_before(LATEST_MOMENT, 8)
                    .expect("the listing")
                    .iter()
                    .any(|due| due.key == "r1");
                assert_eq!(
                    fired + usize::from(listed),
                    1,
                    "the reminder fired {fired} times, and the store lists it: {listed}"
                );
            });
        }

        /// The end of phone/a1's membership, as `Pool::end_members` tells it.
        fn a1_ended() -> Input {
            Input::EndMember("phone/a1".to_string())
        }

        /// Whether the last durable checkpoint of the session `key` binds `peer`.
        fn binds(store: &MemoryStore, key: &str, peer: &str) -> bool {
            checkpoint::load(store, key)
                .expect("read")
                .expect("saved")
                .checkpoint
                .endpoints
                .iter()
                .any(|section| section.router.peers.iter().any(|bound| bound.peer == peer))
        }

        /// r1 binds the instance phone/a1, and a tell of the end of its membership to the
        /// sessions that bind it (`Pool::tell_holders`, the path of `Pool::end_members`) races a
        /// stop. Told before the stop, it is applied, by the teller's run or the stop's drain,
        /// before the stop saves r1, whose checkpoint then binds a1 no more; refused, no key is
        /// answered and the checkpoint binds it still. A tell made after the stop returned is
        /// refused. Fails if a tell reads the stop before it holds the table: it then reaches
        /// r1's inbox after the stop drained it, and the stop saves r1 without it.
        #[test]
        fn a_tell_racing_a_stop_is_applied_before_it_or_refused() {
            model(|| {
                let (store, clock, probe) =
                    (MemoryStore::new(), Clock::new(), Arc::new(Probe::default()));
                let (pool, _wire) = open_pool(&store, &clock, &probe);
                let a1 = joined(&store, "phone/a1", "n1", &clock);
                pool.peer_ready("phone/a1");
                let (key, _) = pool.receive_member(
                    "phone/a1",
                    a1,
                    Standing {
                        incarnation: a1,
                        mark: 1,
                    },
                    from_instance("a1", "r1", 1),
                );
                pool.run(key.as_deref().expect("placed"));
                assert!(binds(&store, "r1", "phone/a1"), "r1 binds the instance");
                let pool = loom::sync::Arc::new(pool);
                let teller = {
                    let pool = loom::sync::Arc::clone(&pool);
                    thread::spawn(move || {
                        let told = pool.tell_holders("phone/a1", a1_ended);
                        for key in &told {
                            pool.run(key);
                        }
                        told
                    })
                };
                pool.stop(Instant::now() + Duration::from_secs(30));
                assert!(
                    pool.tell_holders("phone/a1", a1_ended).is_empty(),
                    "a tell made after the stop returned was admitted"
                );
                let told = teller.join().expect("the teller");
                assert_nothing_left(&pool, "r1");
                let bound = binds(&store, "r1", "phone/a1");
                assert_eq!(
                    bound,
                    told.is_empty(),
                    "the tell answered {told:?}, and the saved session binds the instance: {bound}"
                );
            });
        }

        /// An entry of the tellers' journal for r1, read by the views' reader (`Pool::follow`, the
        /// post of `Pool::post_followed`), races a stop. Handed before the stop, it is taken in,
        /// by the follower's run or the stop's drain, and the view the stop saves holds it and its
        /// position; refused, no view is saved and the reader keeps no mark past it, so the
        /// process that follows the journal next hands it again. Either way once, and a call of
        /// `Pool::follow` after the stop returned hands nothing. Fails if a followed entry is
        /// admitted without the stop read with the table held: it is then put in a slot made
        /// after the stop drained and saved its sessions, and waits there after it returned.
        #[test]
        fn a_followed_entry_racing_a_stop_is_taken_before_it_or_handed_again() {
            model(|| {
                let (store, clock) = (MemoryStore::new(), Clock::new());
                say(&store, &clock, &[("r1", "a")]);
                let pool = loom::sync::Arc::new(views("p1", &store, &clock, 64));
                let follower = {
                    let pool = loom::sync::Arc::clone(&pool);
                    thread::spawn(move || {
                        for key in pool.follow() {
                            pool.run(&key);
                        }
                    })
                };
                pool.stop(Instant::now() + Duration::from_secs(30));
                assert!(
                    pool.follow().is_empty(),
                    "an entry was handed after the stop returned"
                );
                follower.join().expect("the follower");
                assert_nothing_left(&pool, "r1");
                let saved = checkpoint::load(&store.of_machine("views"), "r1").expect("read");
                if saved.is_some() {
                    assert_eq!(
                        view_of(&store, "r1"),
                        (
                            vec!["a".to_string()],
                            "journal://tellers/r1/1".to_string(),
                            Some(1)
                        ),
                        "the view the stop saved did not take the entry in once"
                    );
                } else {
                    assert_eq!(
                        store.read_through("views").expect("the mark"),
                        0,
                        "the reader passed an entry no view took in"
                    );
                }
            });
        }
    }
}
