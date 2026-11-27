// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The checkpoint of a machine on a Rust host core (SCE_MESH.md section 10.11): the machine's
//! saved tree and the state of every endpoint in it, as one document a host writes in one durable
//! write, and the session store a server keeps those documents in.
//!
//! A saved state holds what a session's own steps change; an endpoint's state changes with the
//! network (an envelope admitted, a gap held, a resend due), on threads the machine does not own.
//! Two documents written at two moments could disagree about an envelope and lose it without
//! anyone being told, so the host core writes both as one [`Checkpoint`].
//!
//! Taking one is three steps, so that the write in the middle may be as slow as the medium makes
//! it:
//!
//! 1. Between two drives of the machine, the host saves the machine (its generated `save_at`) and
//!    seals each endpoint of its tree with the checkpoint's number ([`Endpoint::seal`], or
//!    [`Endpoint::checkpoint`] for a machine with one endpoint). From then on the envelopes the
//!    drives before produced belong to that checkpoint.
//! 2. The host writes the [`Checkpoint`] durably: through a [`CheckpointSink`], such as a
//!    [`StoreSink`] over a [`SessionStore`].
//! 3. Once the write is durable the host tells each endpoint ([`Endpoint::durable`]), which
//!    releases what the checkpoint holds: the envelopes it produced, the replies it made, and
//!    the acknowledgements a binding with durable receipt owes.
//!
//! A restore reads the last durable checkpoint ([`load`]), restores the machine from
//! [`Checkpoint::state`] (its generated `restore_with`) and each endpoint from its section
//! ([`Endpoint::restore`]), which releases what the checkpoint held and raises `restored`
//! (SCE_MESH.md section 16.7, row 17) unless the checkpoint is [`Checkpoint::clean`].
//!
//! A server keeps many sessions this way, one per key, and wakes a session that is not loaded
//! when its earliest delayed send comes due: the store keeps each checkpoint's
//! [`Checkpoint::earliest_due`] in the same write ([`StoreSink`]), [`next_wake`] is when the host
//! core's timer fires, and [`wake_due`] loads what is due, earliest first. A restored machine
//! arms a send already due as due now (saved-state format 3), so its delivery and the checkpoint
//! that removes it from the saved state are one write: a crash before it delivers the send after
//! the restart, and a crash after it finds the send gone and its envelope held by the checkpoint,
//! which is therefore due at once ([`DUE_AT_ONCE`]): the session is loaded at the restart, and its
//! restore releases the envelope, though its machine waits for nothing.
//!
//! What a checkpoint never holds: keys, credentials and trust state, which are the host's; a
//! request this machine has in flight, which a restore interrupts (`docs/SCE_ACCEPTED_SUBSET.md`
//! section 2.15), so no correlation is kept for it; and a causal route's state, which its copy's
//! journal keeps (SCE_MESH.md section 10.12).
//!
//! The document is JSON, `sce-mesh-checkpoint` format [`FORMAT`]
//! (`schemas/sce-mesh-checkpoint.v1.schema.json`). Every 64-bit number is the text of its digits,
//! as a saved state writes one, every envelope and id the lowercase hexadecimal of its bytes, and
//! every moment milliseconds since the Unix epoch, so a checkpoint is read on a host whose
//! monotonic clock began elsewhere.
//!
//! [`Endpoint::seal`]: crate::endpoint::Endpoint::seal
//! [`Endpoint::checkpoint`]: crate::endpoint::Endpoint::checkpoint
//! [`Endpoint::durable`]: crate::endpoint::Endpoint::durable
//! [`Endpoint::restore`]: crate::endpoint::Endpoint::restore

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::saved_state::{SavedState, StateRefusal};
use sce_rust_session_store::{Failure, SessionStore, WriteError, LATEST_MOMENT};

use crate::endpoint::Told;
use crate::router::RouterError;

/// The format of the checkpoint this core writes and reads.
pub const FORMAT: u32 = 1;

/// The stability status of the checkpoint wire surface, held in lockstep with
/// `x-sce-schema-status` in `schemas/sce-mesh-checkpoint.v1.schema.json` (`SCE_WIRE_CONTRACTS.md`).
/// A flip to `"stable"` changes both in one commit.
pub const SCHEMA_STATUS: &str = "pre-release";

/// A machine's checkpoint: its saved tree and the endpoint section of each machine name in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    /// Which checkpoint of the machine this is, counted by the host from 1: what each endpoint was
    /// sealed with and is told is durable. A restored endpoint is sealed next with a greater one.
    pub sequence: u64,
    /// Taken after the core stopped admitting (an orderly shutdown, its links closed first), so
    /// nothing a peer sent can have been admitted after it: its restore raises no `restored`.
    pub clean: bool,
    /// The machine's saved state: the tree of its sessions.
    pub state: SavedState,
    /// One section per endpoint of the tree, by machine name.
    pub endpoints: Vec<EndpointSection>,
    /// For each journal the session follows (docs/adr/0114, D4), by its name (a machine's or an
    /// observed stream's), the position of the last of its entries the session took in: written
    /// with what each entry did, so an entry at or below it is a repeat, which the session drops.
    /// Empty for a session that follows none, and then not written.
    pub followed: BTreeMap<String, u64>,
}

/// What one endpoint holds at a checkpoint, under the machine name it speaks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointSection {
    /// The machine the endpoint speaks for: the `source` of what it sends.
    pub machine: String,
    pub(crate) router: RouterState,
    pub(crate) retries: Vec<RetryState>,
    /// What the endpoint released for its machine that the host has not yet applied.
    pub(crate) calls: Vec<Told>,
}

/// The router's part of an endpoint section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RouterState {
    pub peers: Vec<PeerState>,
    pub senders: Vec<SenderState>,
    /// The envelopes produced and not yet released, in the order they were produced: a restore
    /// releases them, since the checkpoint that holds them is durable.
    pub unreleased: Vec<Unreleased>,
}

/// What a router keeps of one peer it binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PeerState {
    pub peer: String,
    /// The sequence number the next envelope to the peer is stamped with.
    pub next_sequence: u64,
    /// What waits for the peer's link to become ready, oldest first.
    pub queued: Vec<QueuedState>,
    /// What a binding with durable receipt sent and the peer has not acknowledged, by sequence.
    pub unacked: Vec<UnackedState>,
    /// What a binding the peer sends to with durable receipt has admitted from it.
    pub received: Option<ReceivedState>,
}

/// An envelope waiting for its link, with the moment it was admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueuedState {
    pub envelope: Vec<u8>,
    pub since: u64,
}

/// An envelope held until its receiver acknowledges it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnackedState {
    pub sequence: u64,
    pub envelope: Vec<u8>,
}

/// A durable receiver's marks for one source: every envelope numbered at most `held` was
/// admitted, and so was each of `above`. A checkpoint holds every one of them, so a restore
/// acknowledges up to `held`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedState {
    pub held: u64,
    pub above: Vec<u64>,
}

/// An envelope produced and not yet released, for `peer`, with its sequence number when it carries
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unreleased {
    pub peer: String,
    pub sequence: Option<u64>,
    pub envelope: Vec<u8>,
}

/// One sender's receive state: its duplicate window, oldest first; the sequence it is expected to
/// send next; the envelopes held past a gap; and the reply cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SenderState {
    pub source: String,
    pub window: Vec<[u8; 16]>,
    pub next: Option<u64>,
    pub held: Vec<HeldState>,
    pub replies: Vec<ReplyState>,
    /// Whether the stream is durable: none of its gaps ever times out.
    pub durable: bool,
    /// The completion records of an instance of an open machine whose binding receives durably
    /// (docs/adr/0113, D5), `None` for any other sender.
    pub records: Option<RecordsState>,
}

/// An instance's completion records: the incarnation they are of, the member's low-water mark the
/// store held, and a record of each request taken in at or above it, by its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordsState {
    pub incarnation: u64,
    pub mark: u64,
    pub kept: Vec<RecordState>,
}

/// A completion record: the request's number, its wire `invoke_id` when a reply answers it, and
/// its reply, `None` while it is handled or when it has none. A reply a checkpoint holds is one its
/// restore releases, so it is restored as sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordState {
    pub number: u64,
    pub invoke: Option<[u8; 16]>,
    pub reply: Option<Vec<u8>>,
}

/// An envelope held past an ordering gap, with its sequence and the moment it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HeldState {
    pub sequence: u64,
    pub arrived: u64,
    pub envelope: Vec<u8>,
}

/// A reply-cache entry: the request's envelope id and wire `invoke_id`, and its reply, `None`
/// while the request is being handled. A reply a checkpoint holds is one its restore releases, so
/// it is restored as sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplyState {
    pub request: [u8; 16],
    pub invoke: [u8; 16],
    pub reply: Option<Vec<u8>>,
}

/// A transmission waiting out its backoff: the sends made, the wait last chosen, and when it is due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetryState {
    pub peer: String,
    pub envelope: Vec<u8>,
    pub made: u32,
    pub backoff: Option<i64>,
    pub due: u64,
}

/// The two clocks a checkpoint is taken and restored against: the host's monotonic clock, which
/// what an endpoint holds is timed by while it runs, and the wall clock, which a checkpoint writes
/// every moment in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clocks {
    pub monotonic_ms: i64,
    /// Milliseconds since the Unix epoch.
    pub wall_ms: u64,
}

impl Clocks {
    /// The monotonic moment `at_ms` on the wall clock.
    pub(crate) fn to_wall(self, at_ms: i64) -> u64 {
        let ahead = at_ms.saturating_sub(self.monotonic_ms);
        if ahead >= 0 {
            self.wall_ms.saturating_add(ahead.unsigned_abs())
        } else {
            self.wall_ms.saturating_sub(ahead.unsigned_abs())
        }
    }

    /// The wall-clock moment `wall_ms` on the monotonic clock: a moment already past is as far in
    /// the past, so what came due while the process was down is due at once. A moment the
    /// monotonic clock cannot read (see [`Clocks::placed`]) is kept at the end of the clock it lies
    /// toward.
    pub(crate) fn to_monotonic(self, wall_ms: u64) -> i64 {
        self.placed(wall_ms).unwrap_or(if wall_ms >= self.wall_ms {
            i64::MAX
        } else {
            i64::MIN
        })
    }

    /// The wall-clock moment `wall_ms` on the monotonic clock exactly, or `None` when the
    /// monotonic clock cannot read it: further from the two clocks' reading than an i64 counts. A
    /// moment placed so is as far from the monotonic reading as from the wall one, so an age
    /// measured from it is the one it had on the wall; a moment moved to the end of the clock
    /// would be given an age it never had.
    pub(crate) fn placed(self, wall_ms: u64) -> Option<i64> {
        if wall_ms >= self.wall_ms {
            let ahead = i64::try_from(wall_ms - self.wall_ms).ok()?;
            self.monotonic_ms.checked_add(ahead)
        } else {
            let behind = i64::try_from(self.wall_ms - wall_ms).ok()?;
            self.monotonic_ms.checked_sub(behind)
        }
    }
}

/// Why a checkpoint could not be taken, read or restored.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckpointError {
    /// The text is not a checkpoint this core reads, or it holds a moment the clocks it is
    /// restored against cannot place (`Clocks::placed`); the words say where.
    Malformed(String),
    /// Its saved state is not one the runtime reads.
    State(StateRefusal),
    /// The router refused: it does not checkpoint (`Router::enable_checkpoints` was not called),
    /// or the number is not above the last one it was sealed with.
    Router(RouterError),
    /// The checkpoint holds no section for this machine.
    NoSection(String),
    /// The checkpoint holds what this router cannot take back: state for a peer it does not bind,
    /// or envelopes held for an acknowledgement on a binding that is not durable. Restoring it
    /// would lose what the checkpoint promised to keep.
    Unbound(String),
    /// The endpoint holds an event said for a person that the host has not applied
    /// (docs/adr/0116, D6): a checkpoint holds no person, and an event said for one is never run
    /// as nobody's, so the host applies and runs it before it seals.
    Person(String),
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(words) => {
                write!(f, "the checkpoint is not one this core reads: {words}")
            }
            Self::State(refusal) => write!(f, "the checkpoint's saved state: {refusal}"),
            Self::Router(error) => write!(f, "the router refused the checkpoint: {error:?}"),
            Self::NoSection(machine) => {
                write!(
                    f,
                    "the checkpoint holds no endpoint section for '{machine}'"
                )
            }
            Self::Unbound(words) => write!(f, "the checkpoint cannot be restored here: {words}"),
            Self::Person(words) => write!(f, "the checkpoint cannot hold a person: {words}"),
        }
    }
}

impl std::error::Error for CheckpointError {}

impl Checkpoint {
    /// The section of the endpoint that speaks for `machine`.
    pub fn endpoint(&self, machine: &str) -> Option<&EndpointSection> {
        self.endpoints
            .iter()
            .find(|section| section.machine == machine)
    }

    /// The earliest moment the checkpoint acts on its own once it is restored, in milliseconds
    /// since the Unix epoch, or `None` when it waits for nothing: what the tree waits for (a
    /// delayed send or a host-run invocation's deadline, of any session of it,
    /// [`SavedState::earliest_due`]), and what each endpoint section holds that its restore acts on
    /// ([`EndpointSection::earliest_due`]). What a session store keeps beside the checkpoint, so a
    /// server loads the session when it comes due.
    ///
    /// The endpoint's part is what makes this the checkpoint's due and not only its machine's: a
    /// send that came due leaves in the checkpoint that removes it from the saved state, so a
    /// process that fails after that checkpoint is durable and before it released the send leaves
    /// a session whose machine waits for nothing and whose checkpoint still holds the send. Only
    /// a restore releases it, and only a due loads a session nobody asks of.
    pub fn earliest_due(&self) -> Option<u64> {
        self.endpoints
            .iter()
            .filter_map(EndpointSection::earliest_due)
            .chain(self.state.earliest_due())
            .min()
    }

    /// The checkpoint as `sce-mesh-checkpoint` JSON.
    pub fn to_json(&self) -> String {
        let mut members = alloc::vec![
            ("format", Value::Number(FORMAT.to_string())),
            ("checkpoint", count(self.sequence)),
            ("clean", Value::Bool(self.clean)),
            ("state", saved_state_value(&self.state)),
            (
                "endpoints",
                Value::Array(
                    self.endpoints
                        .iter()
                        .map(EndpointSection::to_value)
                        .collect()
                ),
            ),
        ];
        // docs/adr/0114, D4: written only for a session that follows a journal, so the
        // checkpoint of one that follows none is what it was.
        if !self.followed.is_empty() {
            members.push((
                "followed",
                Value::Object(
                    self.followed
                        .iter()
                        .map(|(journal, position)| (journal.clone(), count(*position)))
                        .collect(),
                ),
            ));
        }
        json::write(&object(members))
    }

    /// Read `sce-mesh-checkpoint` JSON, judged by its `format` before anything else is read.
    pub fn from_json(text: &str) -> Result<Self, CheckpointError> {
        let value = json::parse(text)
            .map_err(|error| CheckpointError::Malformed(format!("the text is {error}")))?;
        match member(&value, "format", "checkpoint")? {
            Value::Number(written) if *written == FORMAT.to_string() => {}
            other => {
                return Err(CheckpointError::Malformed(format!(
                    "it is format {}, and this core reads format {FORMAT}",
                    json::write(other)
                )))
            }
        }
        let state = SavedState::from_json(&json::write(member(&value, "state", "checkpoint")?))
            .map_err(CheckpointError::State)?;
        Ok(Self {
            sequence: as_count(member(&value, "checkpoint", "checkpoint")?, "checkpoint")?,
            clean: as_flag(member(&value, "clean", "checkpoint")?, "clean")?,
            state,
            endpoints: as_items(member(&value, "endpoints", "checkpoint")?, "endpoints")?
                .iter()
                .enumerate()
                .map(|(i, section)| {
                    EndpointSection::from_value(section, &format!("endpoints[{i}]"))
                })
                .collect::<Result<_, _>>()?,
            followed: match value.member("followed") {
                None => BTreeMap::new(),
                Some(Value::Object(journals)) => journals
                    .iter()
                    .map(|(journal, position)| {
                        as_count(position, &format!("followed.{journal}"))
                            .map(|position| (journal.clone(), position))
                    })
                    .collect::<Result<_, _>>()?,
                Some(_) => {
                    return Err(CheckpointError::Malformed(
                        "'followed' is not an object".to_string(),
                    ))
                }
            },
        })
    }
}

/// Where a host keeps the checkpoints of one machine, durably.
///
/// A sink is the middle of the three steps of a checkpoint (module documentation): the host seals
/// the endpoints, writes through the sink, and reports the checkpoint durable to each endpoint only
/// once [`CheckpointSink::write`] answered. A sink whose medium makes a write durable later (a
/// group commit) is one whose host reports later; nothing the checkpoint produced leaves before.
pub trait CheckpointSink {
    /// Why a write failed. Nothing was written, and the checkpoint is not durable.
    type Error;

    /// Write `checkpoint` in place of the one before. When this answers `Ok`, the checkpoint
    /// survives a crash of the process.
    fn write(&mut self, checkpoint: &Checkpoint) -> Result<(), Self::Error>;
}

/// A [`CheckpointSink`] over a [`SessionStore`]: each checkpoint is the saved session of one key,
/// written with its earliest due in the same transaction, under the lease whose fence it carries
/// and over the version it read.
pub struct StoreSink<'s, S: ?Sized> {
    store: &'s S,
    key: String,
    version: u64,
    fence: u64,
}

impl<'s, S: SessionStore + ?Sized> StoreSink<'s, S> {
    /// A sink that writes the session `key` of `store`, which holds it at `version` (`0` for none),
    /// under the lease of fence `fence`.
    pub fn new(store: &'s S, key: &str, version: u64, fence: u64) -> Self {
        Self {
            store,
            key: key.to_string(),
            version,
            fence,
        }
    }

    /// The version of the session the last write answered: the one the next write is made over.
    pub fn version(&self) -> u64 {
        self.version
    }
}

impl<S: SessionStore + ?Sized> CheckpointSink for StoreSink<'_, S> {
    type Error = WriteError;

    fn write(&mut self, checkpoint: &Checkpoint) -> Result<(), WriteError> {
        self.version = self.store.put(
            &self.key,
            checkpoint.to_json().as_bytes(),
            checkpoint.earliest_due(),
            self.version,
            self.fence,
        )?;
        Ok(())
    }
}

/// A session a store holds, read back as its checkpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub key: String,
    /// The version the store holds it at: what the next write of it is made over.
    pub version: u64,
    pub checkpoint: Checkpoint,
}

/// Why sessions could not be loaded.
#[derive(Debug)]
pub enum LoadError {
    /// The store failed.
    Store(Failure),
    /// The session `key` holds no checkpoint this core reads.
    Unreadable { key: String, error: CheckpointError },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(failure) => write!(f, "{failure}"),
            Self::Unreadable { key, error } => write!(f, "the session '{key}': {error}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// The last durable checkpoint of the session `key`, or `None` when `store` holds none.
pub fn load<S: SessionStore + ?Sized>(store: &S, key: &str) -> Result<Option<Loaded>, LoadError> {
    let Some(stored) = store.get(key).map_err(LoadError::Store)? else {
        return Ok(None);
    };
    let unreadable = |error| LoadError::Unreadable {
        key: key.to_string(),
        error,
    };
    let text = core::str::from_utf8(&stored.state).map_err(|_| {
        unreadable(CheckpointError::Malformed(
            "the stored bytes are not UTF-8".to_string(),
        ))
    })?;
    let checkpoint = Checkpoint::from_json(text).map_err(unreadable)?;
    Ok(Some(Loaded {
        key: key.to_string(),
        version: stored.version,
        checkpoint,
    }))
}

/// The sessions of `store` whose earliest delayed send is due at or before `now_wall_ms`, at most
/// `limit` of them, earliest first, each loaded: restored, each arms its sends already due as due
/// now. A session deleted between the listing and its loading is left out. Each is loaded by the
/// key the store names for the entry it listed, which a store that indexes its keys by a keyed
/// digest reads from the session it holds (docs/adr/0109, D8).
pub fn wake_due<S: SessionStore + ?Sized>(
    store: &S,
    now_wall_ms: u64,
    limit: usize,
) -> Result<Vec<Loaded>, LoadError> {
    let due = store
        .due_before(now_wall_ms, limit)
        .map_err(LoadError::Store)?;
    let mut woken = Vec::with_capacity(due.len());
    for session in due {
        let Some(key) = store.key_of(&session.key).map_err(LoadError::Store)? else {
            continue;
        };
        if let Some(loaded) = load(store, &key)? {
            woken.push(loaded);
        }
    }
    Ok(woken)
}

/// When the host core's timer next fires: the least earliest due of the sessions `store` holds, in
/// milliseconds since the Unix epoch, or `None` when no session has a send waiting.
pub fn next_wake<S: SessionStore + ?Sized>(store: &S) -> Result<Option<u64>, Failure> {
    Ok(store
        .due_before(LATEST_MOMENT, 1)?
        .first()
        .map(|due| due.due))
}

/// The due of a checkpoint whose restore acts at once: the Unix epoch, which every moment a host
/// reads its clock at is past, so the session is loaded at the first look at what came due.
pub const DUE_AT_ONCE: u64 = 0;

impl EndpointSection {
    /// The earliest moment what this section holds acts once it is restored, without an input:
    /// [`DUE_AT_ONCE`] when it holds an envelope a checkpoint produced and did not release, an
    /// event released for the machine and not applied, an envelope waiting for its link, or one
    /// waiting for its receiver's acknowledgement, each of which the restore hands on; otherwise
    /// the due of the earliest transmission waiting out its backoff; `None` when it holds none of
    /// them. These are what keep an endpoint from being quiet (`Endpoint::is_quiet`), so a
    /// checkpoint taken of a quiet endpoint, as a session dropped from memory is kept, waits for
    /// nothing here.
    pub fn earliest_due(&self) -> Option<u64> {
        let at_once = !self.router.unreleased.is_empty()
            || !self.calls.is_empty()
            || self
                .router
                .peers
                .iter()
                .any(|peer| !peer.queued.is_empty() || !peer.unacked.is_empty());
        if at_once {
            return Some(DUE_AT_ONCE);
        }
        self.retries.iter().map(|retry| retry.due).min()
    }
}

// ── The document ────────────────────────────────────────────────────────

impl EndpointSection {
    fn to_value(&self) -> Value {
        object(alloc::vec![
            ("machine", text(&self.machine)),
            (
                "peers",
                Value::Array(self.router.peers.iter().map(PeerState::to_value).collect()),
            ),
            (
                "senders",
                Value::Array(
                    self.router
                        .senders
                        .iter()
                        .map(SenderState::to_value)
                        .collect()
                ),
            ),
            (
                "unreleased",
                Value::Array(
                    self.router
                        .unreleased
                        .iter()
                        .map(Unreleased::to_value)
                        .collect(),
                ),
            ),
            (
                "retries",
                Value::Array(self.retries.iter().map(RetryState::to_value).collect()),
            ),
            (
                "calls",
                Value::Array(self.calls.iter().map(told_value).collect()),
            ),
        ])
    }

    fn from_value(value: &Value, at: &str) -> Result<Self, CheckpointError> {
        Ok(Self {
            machine: as_text(member(value, "machine", at)?, &format!("{at}.machine"))?.to_string(),
            router: RouterState {
                peers: read_list(value, "peers", at, PeerState::from_value)?,
                senders: read_list(value, "senders", at, SenderState::from_value)?,
                unreleased: read_list(value, "unreleased", at, Unreleased::from_value)?,
            },
            retries: read_list(value, "retries", at, RetryState::from_value)?,
            calls: read_list(value, "calls", at, told_from_value)?,
        })
    }
}

impl PeerState {
    fn to_value(&self) -> Value {
        object(alloc::vec![
            ("peer", text(&self.peer)),
            ("nextseq", count(self.next_sequence)),
            (
                "queued",
                Value::Array(
                    self.queued
                        .iter()
                        .map(|queued| {
                            object(alloc::vec![
                                ("envelope", hex(&queued.envelope)),
                                ("since", count(queued.since)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "unacked",
                Value::Array(
                    self.unacked
                        .iter()
                        .map(|held| {
                            object(alloc::vec![
                                ("seq", count(held.sequence)),
                                ("envelope", hex(&held.envelope)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "received",
                match &self.received {
                    None => Value::Null,
                    Some(received) => object(alloc::vec![
                        ("held", count(received.held)),
                        (
                            "above",
                            Value::Array(received.above.iter().map(|seq| count(*seq)).collect()),
                        ),
                    ]),
                },
            ),
        ])
    }

    fn from_value(value: &Value, at: &str) -> Result<Self, CheckpointError> {
        let field = |name: &str| member(value, name, at);
        let path = |name: &str| format!("{at}.{name}");
        let queued = as_items(field("queued")?, &path("queued"))?
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let at = format!("{at}.queued[{i}]");
                Ok(QueuedState {
                    envelope: as_bytes(member(item, "envelope", &at)?, &at)?,
                    since: as_count(member(item, "since", &at)?, &at)?,
                })
            })
            .collect::<Result<_, CheckpointError>>()?;
        let unacked = as_items(field("unacked")?, &path("unacked"))?
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let at = format!("{at}.unacked[{i}]");
                Ok(UnackedState {
                    sequence: as_count(member(item, "seq", &at)?, &at)?,
                    envelope: as_bytes(member(item, "envelope", &at)?, &at)?,
                })
            })
            .collect::<Result<_, CheckpointError>>()?;
        let received = match field("received")? {
            Value::Null => None,
            received => {
                let at = path("received");
                Some(ReceivedState {
                    held: as_count(member(received, "held", &at)?, &at)?,
                    above: as_items(member(received, "above", &at)?, &at)?
                        .iter()
                        .map(|seq| as_count(seq, &at))
                        .collect::<Result<_, _>>()?,
                })
            }
        };
        Ok(Self {
            peer: as_text(field("peer")?, &path("peer"))?.to_string(),
            next_sequence: as_count(field("nextseq")?, &path("nextseq"))?,
            queued,
            unacked,
            received,
        })
    }
}

impl SenderState {
    fn to_value(&self) -> Value {
        object(alloc::vec![
            ("source", text(&self.source)),
            (
                "window",
                Value::Array(self.window.iter().map(|id| hex(id)).collect()),
            ),
            ("next", self.next.map_or(Value::Null, count)),
            (
                "held",
                Value::Array(
                    self.held
                        .iter()
                        .map(|held| {
                            object(alloc::vec![
                                ("seq", count(held.sequence)),
                                ("arrived", count(held.arrived)),
                                ("envelope", hex(&held.envelope)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "replies",
                Value::Array(
                    self.replies
                        .iter()
                        .map(|reply| {
                            object(alloc::vec![
                                ("request", hex(&reply.request)),
                                ("invoke", hex(&reply.invoke)),
                                ("reply", reply.reply.as_deref().map_or(Value::Null, hex),),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("durable", Value::Bool(self.durable)),
            (
                "records",
                match &self.records {
                    None => Value::Null,
                    Some(records) => object(alloc::vec![
                        ("incarnation", count(records.incarnation)),
                        ("mark", count(records.mark)),
                        (
                            "kept",
                            Value::Array(
                                records
                                    .kept
                                    .iter()
                                    .map(|record| {
                                        object(alloc::vec![
                                            ("number", count(record.number)),
                                            (
                                                "invoke",
                                                record
                                                    .invoke
                                                    .as_ref()
                                                    .map_or(Value::Null, |id| hex(id)),
                                            ),
                                            (
                                                "reply",
                                                record.reply.as_deref().map_or(Value::Null, hex)
                                            ),
                                        ])
                                    })
                                    .collect(),
                            ),
                        ),
                    ]),
                },
            ),
        ])
    }

    fn from_value(value: &Value, at: &str) -> Result<Self, CheckpointError> {
        let field = |name: &str| member(value, name, at);
        let path = |name: &str| format!("{at}.{name}");
        let held = as_items(field("held")?, &path("held"))?
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let at = format!("{at}.held[{i}]");
                Ok(HeldState {
                    sequence: as_count(member(item, "seq", &at)?, &at)?,
                    arrived: as_count(member(item, "arrived", &at)?, &at)?,
                    envelope: as_bytes(member(item, "envelope", &at)?, &at)?,
                })
            })
            .collect::<Result<_, CheckpointError>>()?;
        let replies = as_items(field("replies")?, &path("replies"))?
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let at = format!("{at}.replies[{i}]");
                Ok(ReplyState {
                    request: as_id(member(item, "request", &at)?, &at)?,
                    invoke: as_id(member(item, "invoke", &at)?, &at)?,
                    reply: match member(item, "reply", &at)? {
                        Value::Null => None,
                        reply => Some(as_bytes(reply, &at)?),
                    },
                })
            })
            .collect::<Result<_, CheckpointError>>()?;
        Ok(Self {
            source: as_text(field("source")?, &path("source"))?.to_string(),
            window: as_items(field("window")?, &path("window"))?
                .iter()
                .map(|id| as_id(id, &path("window")))
                .collect::<Result<_, _>>()?,
            next: match field("next")? {
                Value::Null => None,
                next => Some(as_count(next, &path("next"))?),
            },
            held,
            replies,
            durable: as_flag(field("durable")?, &path("durable"))?,
            records: match field("records")? {
                Value::Null => None,
                records => {
                    let at = path("records");
                    let kept = as_items(member(records, "kept", &at)?, &at)?
                        .iter()
                        .enumerate()
                        .map(|(i, item)| {
                            let at = format!("{at}.kept[{i}]");
                            Ok(RecordState {
                                number: as_count(member(item, "number", &at)?, &at)?,
                                invoke: match member(item, "invoke", &at)? {
                                    Value::Null => None,
                                    invoke => Some(as_id(invoke, &at)?),
                                },
                                reply: match member(item, "reply", &at)? {
                                    Value::Null => None,
                                    reply => Some(as_bytes(reply, &at)?),
                                },
                            })
                        })
                        .collect::<Result<_, CheckpointError>>()?;
                    Some(RecordsState {
                        incarnation: as_count(member(records, "incarnation", &at)?, &at)?,
                        mark: as_count(member(records, "mark", &at)?, &at)?,
                        kept,
                    })
                }
            },
        })
    }
}

impl Unreleased {
    fn to_value(&self) -> Value {
        object(alloc::vec![
            ("peer", text(&self.peer)),
            ("seq", self.sequence.map_or(Value::Null, count)),
            ("envelope", hex(&self.envelope)),
        ])
    }

    fn from_value(value: &Value, at: &str) -> Result<Self, CheckpointError> {
        Ok(Self {
            peer: as_text(member(value, "peer", at)?, at)?.to_string(),
            sequence: match member(value, "seq", at)? {
                Value::Null => None,
                seq => Some(as_count(seq, at)?),
            },
            envelope: as_bytes(member(value, "envelope", at)?, at)?,
        })
    }
}

impl RetryState {
    fn to_value(&self) -> Value {
        object(alloc::vec![
            ("peer", text(&self.peer)),
            ("envelope", hex(&self.envelope)),
            ("attempts", count(u64::from(self.made))),
            (
                "backoff",
                self.backoff
                    .map_or(Value::Null, |ms| Value::Text(ms.to_string())),
            ),
            ("due", count(self.due)),
        ])
    }

    fn from_value(value: &Value, at: &str) -> Result<Self, CheckpointError> {
        let made = as_count(member(value, "attempts", at)?, at)?;
        Ok(Self {
            peer: as_text(member(value, "peer", at)?, at)?.to_string(),
            envelope: as_bytes(member(value, "envelope", at)?, at)?,
            made: u32::try_from(made).map_err(|_| {
                CheckpointError::Malformed(format!(
                    "'{at}.attempts' ({made}) is past a count of sends"
                ))
            })?,
            backoff: match member(value, "backoff", at)? {
                Value::Null => None,
                backoff => Some(as_signed(backoff, at)?),
            },
            due: as_count(member(value, "due", at)?, at)?,
        })
    }
}

fn told_value(told: &Told) -> Value {
    match told {
        // A person is never sealed (`Endpoint::seal` refuses an endpoint that holds one).
        Told::Event {
            name,
            data,
            source,
            send_id,
            invoke_id,
            principal: _,
        } => object(alloc::vec![
            ("call", text("event")),
            ("event", text(name)),
            ("data", text(data)),
            ("source", text(source)),
            ("sendid", text(send_id)),
            ("invokeid", text(invoke_id)),
        ]),
        Told::End {
            invoke_id,
            token,
            failed,
            data,
            source,
        } => object(alloc::vec![
            ("call", text("end")),
            ("invokeid", text(invoke_id)),
            ("token", count(*token)),
            ("failed", Value::Bool(*failed)),
            ("data", text(data)),
            ("source", source.as_deref().map_or(Value::Null, text)),
        ]),
    }
}

fn told_from_value(value: &Value, at: &str) -> Result<Told, CheckpointError> {
    let field = |name: &str| member(value, name, at);
    let words = |name: &str| -> Result<String, CheckpointError> {
        Ok(as_text(field(name)?, &format!("{at}.{name}"))?.to_string())
    };
    match as_text(field("call")?, &format!("{at}.call"))? {
        "event" => Ok(Told::Event {
            name: words("event")?,
            data: words("data")?,
            source: words("source")?,
            send_id: words("sendid")?,
            invoke_id: words("invokeid")?,
            principal: None,
        }),
        "end" => Ok(Told::End {
            invoke_id: words("invokeid")?,
            token: as_count(field("token")?, &format!("{at}.token"))?,
            failed: as_flag(field("failed")?, &format!("{at}.failed"))?,
            data: words("data")?,
            source: match field("source")? {
                Value::Null => None,
                _ => Some(words("source")?),
            },
        }),
        other => Err(CheckpointError::Malformed(format!(
            "'{at}.call' is '{other}', which is neither 'event' nor 'end'"
        ))),
    }
}

/// The saved state as the JSON value its text is: the runtime writes it, and its text is JSON.
fn saved_state_value(state: &SavedState) -> Value {
    match json::parse(&state.to_json()) {
        Ok(value) => value,
        Err(error) => unreachable!("the runtime writes a saved state as JSON: {error}"),
    }
}

fn object(members: Vec<(&str, Value)>) -> Value {
    Value::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

fn text(words: &str) -> Value {
    Value::Text(words.to_string())
}

/// A 64-bit number as the text of its digits, as a saved state writes one.
fn count(n: u64) -> Value {
    Value::Text(n.to_string())
}

/// Bytes as the lowercase hexadecimal text of each.
fn hex(bytes: &[u8]) -> Value {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    Value::Text(out)
}

/// Each item of the array `name` of `value`, read by `read`.
fn read_list<T>(
    value: &Value,
    name: &str,
    at: &str,
    read: impl Fn(&Value, &str) -> Result<T, CheckpointError>,
) -> Result<Vec<T>, CheckpointError> {
    let list = member(value, name, at)?;
    let at = format!("{at}.{name}");
    as_items(list, &at)?
        .iter()
        .enumerate()
        .map(|(i, item)| read(item, &format!("{at}[{i}]")))
        .collect()
}

fn member<'v>(value: &'v Value, name: &str, at: &str) -> Result<&'v Value, CheckpointError> {
    value
        .member(name)
        .ok_or_else(|| CheckpointError::Malformed(format!("'{at}' has no '{name}'")))
}

fn as_text<'v>(value: &'v Value, at: &str) -> Result<&'v str, CheckpointError> {
    match value {
        Value::Text(words) => Ok(words),
        _ => Err(CheckpointError::Malformed(format!("'{at}' is not a text"))),
    }
}

fn as_flag(value: &Value, at: &str) -> Result<bool, CheckpointError> {
    match value {
        Value::Bool(flag) => Ok(*flag),
        _ => Err(CheckpointError::Malformed(format!(
            "'{at}' is not true or false"
        ))),
    }
}

fn as_items<'v>(value: &'v Value, at: &str) -> Result<&'v [Value], CheckpointError> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(CheckpointError::Malformed(format!(
            "'{at}' is not an array"
        ))),
    }
}

/// A 64-bit number written as the text of its digits, and only digits: `parse` also reads a
/// leading `+`, which no writer writes.
fn as_count(value: &Value, at: &str) -> Result<u64, CheckpointError> {
    let written = as_text(value, at)?;
    written
        .parse::<u64>()
        .ok()
        .filter(|_| !written.is_empty() && written.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| {
            CheckpointError::Malformed(format!("'{at}' ({written}) is not a whole number"))
        })
}

/// A signed 64-bit number written as the text of its digits, with a `-` when it is negative.
fn as_signed(value: &Value, at: &str) -> Result<i64, CheckpointError> {
    let written = as_text(value, at)?;
    let digits = written.strip_prefix('-').unwrap_or(written);
    written
        .parse::<i64>()
        .ok()
        .filter(|_| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| {
            CheckpointError::Malformed(format!("'{at}' ({written}) is not a whole number"))
        })
}

/// Bytes written as lowercase hexadecimal, two digits each.
fn as_bytes(value: &Value, at: &str) -> Result<Vec<u8>, CheckpointError> {
    let written = as_text(value, at)?;
    let malformed = || {
        CheckpointError::Malformed(format!(
            "'{at}' is not bytes written as lowercase hexadecimal"
        ))
    };
    let pairs = written.as_bytes().chunks_exact(2);
    if !pairs.remainder().is_empty() {
        return Err(malformed());
    }
    let digit = |d: u8| match d {
        b'0'..=b'9' => Some(d - b'0'),
        b'a'..=b'f' => Some(d - b'a' + 10),
        _ => None,
    };
    pairs
        .map(|pair| Some((digit(pair[0])? << 4) | digit(pair[1])?))
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(malformed)
}

/// A 16-byte id written as lowercase hexadecimal.
fn as_id(value: &Value, at: &str) -> Result<[u8; 16], CheckpointError> {
    <[u8; 16]>::try_from(as_bytes(value, at)?)
        .map_err(|_| CheckpointError::Malformed(format!("'{at}' is not 16 bytes")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::{Endpoint, EngineCall, MESH_PROCESSOR_TYPE};
    use crate::generated::envelope::Envelope;
    use crate::generated::pattern_kind::PatternKind;
    use crate::inbound::Delivery;
    use crate::router::{DurableSend, PeerConfig, Responders, Router};
    use crate::rpc::MESH_EVENT_PARAM;
    use crate::testing::{Fixed, Recorder};
    use alloc::vec;
    use sce_forge_runtime::codec::SceCursor;
    use sce_rust_runtime::saved_state::{
        SavedAct, SavedChild, SavedHostSend, SavedInvokes, SavedSend,
    };
    use sce_rust_runtime::{HostInvokeRequest, HostSendRequest};
    use sce_rust_session_store::MemoryStore;

    type Host = Endpoint<Recorder, Fixed>;

    fn config(peer: &'static str, replies: &'static [&'static str]) -> PeerConfig {
        PeerConfig {
            transport: "wss",
            buffer: None,
            retry: None,
            stamp_sequence: false,
            delivery: Delivery {
                dedup: true,
                ordered: false,
            },
            // A generated peer table holds this as a `const`; a test names
            // its peer at run time, so the one-element set is leaked.
            responders: Responders::Named(Box::leak(Box::new([peer]))),
            deadline_ms: None,
            reply_events: replies,
            causal: None,
        }
    }

    /// A host of `machine`, bound to `peer`, in its `incarnation`th life at
    /// `now`, its link to `peer` ready when `ready`. `setup` declares what the
    /// deployment says of the router.
    fn host(
        machine: &str,
        peer: &'static str,
        replies: &'static [&'static str],
        incarnation: u8,
        now: i64,
        ready: bool,
        setup: impl FnOnce(&mut Router),
    ) -> Host {
        let mut router = Router::new(machine, 8, 50).unwrap();
        router.add_peer(peer, config(peer, replies));
        setup(&mut router);
        let mut host = Endpoint::new(
            router,
            Recorder::default(),
            Fixed {
                now,
                ids: 0,
                incarnation,
            },
        );
        if ready {
            host.peer_ready(peer);
        }
        host
    }

    /// The server: it answers `client`'s `service.request.add` with
    /// `service.response.add`, and its deployment checkpoints it.
    fn serving(incarnation: u8, now: i64) -> Host {
        host(
            "server",
            "client",
            &["service.response.add"],
            incarnation,
            now,
            true,
            Router::enable_checkpoints,
        )
    }

    /// The client, which asks the server and is not checkpointed.
    fn asking() -> Host {
        host("client", "server", &[], 0, 0, true, |_| {})
    }

    /// The request `client` sends to ask `n`, as it leaves.
    fn ask(client: &mut Host, n: u64) -> Vec<u8> {
        let mut params = std::collections::HashMap::new();
        params.insert(
            MESH_EVENT_PARAM.to_string(),
            vec!["service.request.add".to_string()],
        );
        let response = client.invoke(&HostInvokeRequest {
            processor_type: "sce:mesh-rpc".to_string(),
            invoke_id: format!("ask-{n}"),
            src: "#server".to_string(),
            params,
            event_data: format!(r#"{{"n":{n}}}"#),
            token: n,
            ..HostInvokeRequest::default()
        });
        assert!(response.refusal.is_none(), "the request was refused");
        client
            .transport_mut()
            .sent
            .pop()
            .expect("the request left")
            .1
    }

    /// Drive the server's machine through what its endpoint released: each
    /// request is taken in, kept in `seen`, and answered with its own data.
    fn drive(server: &mut Host, seen: &mut Vec<String>) {
        for call in server.take_calls() {
            let EngineCall::Raise(event) = call else {
                continue;
            };
            if event.name != "service.request.add" {
                continue;
            }
            seen.push(event.metadata.data.clone());
            server.send(&HostSendRequest {
                processor_type: MESH_PROCESSOR_TYPE.to_string(),
                event_name: "service.response.add".to_string(),
                target: "#client".to_string(),
                event_data: event.metadata.data.clone(),
                invoke_id: event.metadata.invoke_id.clone(),
                ..HostSendRequest::default()
            });
        }
    }

    /// What `host` handed its transport since this was last asked.
    fn sent(host: &mut Host) -> Vec<Vec<u8>> {
        core::mem::take(&mut host.transport_mut().sent)
            .into_iter()
            .map(|(_, bytes)| bytes)
            .collect()
    }

    /// The requests the client saw answered, by the data of their answers.
    fn answered(client: &mut Host) -> Vec<String> {
        client
            .take_calls()
            .into_iter()
            .filter_map(|call| match call {
                EngineCall::EndInvoke {
                    failed: false,
                    data,
                    ..
                } => Some(data),
                _ => None,
            })
            .collect()
    }

    /// The names of the events `host` released for its machine.
    fn names(host: &mut Host) -> Vec<String> {
        host.take_calls()
            .into_iter()
            .filter_map(|call| match call {
                EngineCall::Raise(event) => Some(event.name),
                EngineCall::EndInvoke { .. } => None,
            })
            .collect()
    }

    /// A machine as its saved state: what it has taken in, as `seen`, and
    /// the delayed sends it has waiting.
    fn saved(seen: &[String], pending: Vec<SavedSend>) -> SavedState {
        SavedState {
            shape: "ledger".to_string(),
            configuration: vec!["running".to_string()],
            current: "running".to_string(),
            variables: vec![(
                "seen".to_string(),
                Value::Array(seen.iter().map(|s| Value::Text(s.clone())).collect()),
            )],
            history: Vec::new(),
            pending,
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: "ledger-1".to_string(),
            invoke_starts: Vec::new(),
            host_invokes: Vec::new(),
            host_invoke_starts: Vec::new(),
            host_invoke_token: 0,
            auto_send_seq: 0,
            external: Vec::new(),
            revisions: Vec::new(),
            features: None,
        }
    }

    /// What the machine of `state` had taken in.
    fn seen_in(state: &SavedState) -> Vec<String> {
        match state.variable("seen") {
            Ok(Value::Array(items)) => items
                .iter()
                .map(|item| match item {
                    Value::Text(words) => words.clone(),
                    other => panic!("'seen' holds {other:?}"),
                })
                .collect(),
            other => panic!("the state's 'seen' is {other:?}"),
        }
    }

    /// A delayed `<send event="tick" target="#hub">` due at `due`.
    fn tick_at(due: u64) -> SavedSend {
        SavedSend {
            due,
            act: SavedAct::Host(SavedHostSend {
                processor_type: MESH_PROCESSOR_TYPE.to_string(),
                event: "tick".to_string(),
                target: "#hub".to_string(),
                send_id: format!("tick-{due}"),
                ..SavedHostSend::default()
            }),
        }
    }

    /// A sink whose medium makes a write durable later: it keeps what it was
    /// given, and its host reports durability when the test says so.
    #[derive(Default)]
    struct Slow {
        written: Vec<u64>,
    }

    impl CheckpointSink for Slow {
        type Error = core::convert::Infallible;

        fn write(&mut self, checkpoint: &Checkpoint) -> Result<(), Self::Error> {
            self.written.push(checkpoint.sequence);
            Ok(())
        }
    }

    /// The session `key` of `store`, leased to a writer at `now`, as a sink
    /// that writes over what the store holds of it.
    fn sink<'s>(store: &'s MemoryStore, key: &str, now: u64) -> StoreSink<'s, MemoryStore> {
        let fence = store.acquire(key, "host", 60_000, now).unwrap().fence;
        let version = store.get(key).unwrap().map_or(0, |stored| stored.version);
        StoreSink::new(store, key, version, fence)
    }

    /// Nothing a drive produced leaves before the checkpoint that holds it is
    /// durable, whatever the medium took to write it, and each durable
    /// checkpoint releases what it holds and no more.
    #[test]
    fn nothing_a_drive_produced_leaves_before_its_checkpoint_is_durable() {
        let mut client = asking();
        let mut server = serving(0, 0);
        let mut seen = Vec::new();
        let mut slow = Slow::default();

        server.receive("client", ask(&mut client, 1));
        drive(&mut server, &mut seen);
        assert!(
            sent(&mut server).is_empty(),
            "a reply left before a checkpoint held it"
        );
        let first = server
            .checkpoint(1, saved(&seen, Vec::new()), false)
            .unwrap();
        slow.write(&first).unwrap();

        server.receive("client", ask(&mut client, 2));
        drive(&mut server, &mut seen);
        let second = server
            .checkpoint(2, saved(&seen, Vec::new()), false)
            .unwrap();
        slow.write(&second).unwrap();
        assert_eq!(slow.written, [1, 2]);
        assert!(
            sent(&mut server).is_empty(),
            "a reply left before its checkpoint was durable"
        );
        assert_eq!((server.sealed_through(), server.durable_through()), (2, 0));

        server.durable(1);
        let released = sent(&mut server);
        assert_eq!(
            released.len(),
            1,
            "the first checkpoint holds the first reply only"
        );
        client.receive("server", released[0].clone());
        assert_eq!(answered(&mut client), [r#"{"n":1}"#]);

        server.durable(2);
        let released = sent(&mut server);
        assert_eq!(released.len(), 1);
        client.receive("server", released[0].clone());
        assert_eq!(answered(&mut client), [r#"{"n":2}"#]);
        assert_eq!(server.durable_through(), 2);
    }

    /// A duplicate of a request whose reply is held for its checkpoint is
    /// dropped, the reply going once when the checkpoint is durable; a
    /// duplicate after that is answered with it.
    #[test]
    fn a_duplicate_of_a_request_whose_reply_is_held_waits_for_the_reply() {
        let mut client = asking();
        let mut server = serving(0, 0);
        let mut seen = Vec::new();
        let request = ask(&mut client, 1);
        server.receive("client", request.clone());
        drive(&mut server, &mut seen);

        server.receive("client", request.clone());
        assert!(server.take_calls().is_empty(), "the machine is told once");
        assert!(
            sent(&mut server).is_empty(),
            "a duplicate releases nothing held"
        );
        server
            .checkpoint(1, saved(&seen, Vec::new()), false)
            .unwrap();
        server.receive("client", request.clone());
        assert!(sent(&mut server).is_empty(), "sealed is not durable");

        server.durable(1);
        let first = sent(&mut server);
        assert_eq!(first.len(), 1);
        server.receive("client", request);
        assert_eq!(
            sent(&mut server),
            first,
            "a duplicate is answered with the first reply"
        );
    }

    /// A restart in the middle of a run keeps every effect a client saw
    /// answered: an answer leaves only once a durable checkpoint holds the
    /// effect, and the restore takes the last durable one, releasing the
    /// answers it held. What the machine took in after it is lost, and the
    /// client is told it may have been.
    #[test]
    fn a_restart_keeps_every_effect_a_client_saw_answered() {
        let store = MemoryStore::new();
        let mut writer = sink(&store, "ledger", 0);
        let mut client = asking();
        let mut server = serving(0, 0);
        let mut seen = Vec::new();
        let mut answers = Vec::new();
        for n in 1..=3 {
            server.receive("client", ask(&mut client, n));
            drive(&mut server, &mut seen);
            writer
                .write(
                    &server
                        .checkpoint(n, saved(&seen, Vec::new()), false)
                        .unwrap(),
                )
                .unwrap();
            server.durable(n);
            for bytes in sent(&mut server) {
                client.receive("server", bytes);
            }
            answers.extend(answered(&mut client));
        }
        // The fourth is written, and the process dies before it hears so.
        server.receive("client", ask(&mut client, 4));
        drive(&mut server, &mut seen);
        writer
            .write(
                &server
                    .checkpoint(4, saved(&seen, Vec::new()), false)
                    .unwrap(),
            )
            .unwrap();
        // The fifth is taken in and dies with the process.
        server.receive("client", ask(&mut client, 5));
        drive(&mut server, &mut seen);
        assert!(sent(&mut server).is_empty());
        drop(server);

        let loaded = load(&store, "ledger")
            .unwrap()
            .expect("the store holds the session");
        assert_eq!(loaded.checkpoint.sequence, 4);
        let mut restored = serving(1, 1_000);
        restored.restore(&loaded.checkpoint).unwrap();
        for bytes in sent(&mut restored) {
            client.receive("server", bytes);
        }
        answers.extend(answered(&mut client));

        let kept = seen_in(&loaded.checkpoint.state);
        assert_eq!(
            answers.len(),
            4,
            "the answer the checkpoint held left on restore"
        );
        for answer in &answers {
            assert!(
                kept.contains(answer),
                "{answer} was answered and is not in the restored state"
            );
        }
        assert!(!kept.contains(&r#"{"n":5}"#.to_string()));
        let rows = restored.take_calls();
        assert!(
            matches!(
                rows.as_slice(),
                [EngineCall::Raise(event)] if event.name == "error.communication"
                    && event.metadata.data
                        == r#"{"errorName":"communication","reason":"restored","source":"client"}"#
            ),
            "{rows:?}"
        );
    }

    /// A request the client sends again after the server restarted is
    /// answered with the reply its first copy produced, which the restored
    /// checkpoint keeps, and the machine is not told it again.
    #[test]
    fn a_duplicate_request_after_a_restart_gets_the_first_reply() {
        let store = MemoryStore::new();
        let mut writer = sink(&store, "ledger", 0);
        let mut client = asking();
        let mut server = serving(0, 0);
        let mut seen = Vec::new();
        let request = ask(&mut client, 1);
        server.receive("client", request.clone());
        drive(&mut server, &mut seen);
        writer
            .write(
                &server
                    .checkpoint(1, saved(&seen, Vec::new()), false)
                    .unwrap(),
            )
            .unwrap();
        server.durable(1);
        let first = sent(&mut server);
        assert_eq!(first.len(), 1, "the reply left, and is lost on the way");
        // A later checkpoint, taken after the reply left, holds it as sent.
        writer
            .write(
                &server
                    .checkpoint(2, saved(&seen, Vec::new()), false)
                    .unwrap(),
            )
            .unwrap();
        server.durable(2);
        drop(server);

        let loaded = load(&store, "ledger").unwrap().unwrap();
        let mut restored = serving(1, 1_000);
        restored.restore(&loaded.checkpoint).unwrap();
        assert_eq!(names(&mut restored), ["error.communication"]);
        assert!(
            sent(&mut restored).is_empty(),
            "nothing was left unreleased"
        );

        restored.receive("client", request);
        assert!(
            restored.take_calls().is_empty(),
            "the machine is not told the request again"
        );
        let again = sent(&mut restored);
        assert_eq!(
            again, first,
            "the duplicate is answered with the first reply"
        );
        client.receive("server", again[0].clone());
        assert_eq!(answered(&mut client), [r#"{"n":1}"#]);
        assert_eq!(seen_in(&loaded.checkpoint.state), [r#"{"n":1}"#]);
    }

    /// The sessions a store holds wake by their earliest waiting send, at or
    /// before now, earliest first; a session with none never wakes.
    #[test]
    fn due_sessions_wake_in_order_of_their_earliest_send() {
        let store = MemoryStore::new();
        let sessions: [(&str, &[u64]); 4] =
            [("a", &[300]), ("b", &[150, 100]), ("c", &[200]), ("d", &[])];
        for (key, dues) in sessions {
            let checkpoint = Checkpoint {
                sequence: 1,
                clean: false,
                state: saved(&[], dues.iter().map(|due| tick_at(*due)).collect()),
                endpoints: Vec::new(),
                followed: BTreeMap::new(),
            };
            sink(&store, key, 0).write(&checkpoint).unwrap();
        }
        assert_eq!(next_wake(&store).unwrap(), Some(100));
        let woken = wake_due(&store, 250, 10).unwrap();
        assert_eq!(
            woken
                .iter()
                .map(|loaded| loaded.key.as_str())
                .collect::<Vec<_>>(),
            ["b", "c"]
        );
        assert_eq!(woken[0].checkpoint.earliest_due(), Some(100));
        assert_eq!(
            wake_due(&store, 250, 1).unwrap().len(),
            1,
            "at most the limit"
        );
        assert!(wake_due(&store, 99, 10).unwrap().is_empty());
        assert!(wake_due(&store, LATEST_MOMENT, 10)
            .unwrap()
            .iter()
            .all(|loaded| loaded.key != "d"));
    }

    /// A running child's waiting send counts toward its tree's earliest due.
    #[test]
    fn a_childs_waiting_send_is_its_trees_earliest_due() {
        let mut state = saved(&[], vec![tick_at(500)]);
        state.invokes = SavedInvokes::Resumed(vec![SavedChild {
            id: "child".to_string(),
            start: 1,
            invoke_id: "running.1.child".to_string(),
            document: "ledger".to_string(),
            session: saved(&[], vec![tick_at(400)]),
        }]);
        let checkpoint = Checkpoint {
            sequence: 1,
            clean: false,
            state,
            endpoints: Vec::new(),
            followed: BTreeMap::new(),
        };
        assert_eq!(checkpoint.earliest_due(), Some(400));
    }

    /// The timer session: one delayed send to `hub`, its deployment
    /// checkpointing it.
    fn timing(incarnation: u8, now: i64) -> Host {
        host(
            "timer",
            "hub",
            &[],
            incarnation,
            now,
            true,
            Router::enable_checkpoints,
        )
    }

    /// Fire every send of `state` due at `wall_ms`, as a restored machine
    /// fires a send already due, and answer the state without them.
    fn fire(timer: &mut Host, state: &SavedState, wall_ms: u64) -> SavedState {
        let mut next = state.clone();
        let (due, waiting): (Vec<_>, Vec<_>) = state
            .pending
            .iter()
            .cloned()
            .partition(|send| send.due <= wall_ms);
        next.pending = waiting;
        for send in due {
            let SavedAct::Host(act) = send.act else {
                panic!("the timer's sends are host sends");
            };
            timer.send(&HostSendRequest {
                processor_type: act.processor_type,
                event_name: act.event,
                target: act.target,
                send_id: act.send_id,
                event_data: act.data,
                ..HostSendRequest::default()
            });
        }
        next
    }

    /// Hand what `timer` sent to `hub`, and count the ticks `hub`'s machine
    /// is told of.
    fn deliver(timer: &mut Host, hub: &mut Host) -> usize {
        for bytes in sent(timer) {
            hub.receive("timer", bytes);
        }
        names(hub).iter().filter(|name| *name == "tick").count()
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kill {
        /// After the send fired, before its checkpoint was written.
        BeforeTheWrite,
        /// After its checkpoint was written, before the host heard so.
        BeforeTheRelease,
        /// After its envelope left.
        AfterTheRelease,
    }

    /// A process killed anywhere between a due send's delivery and the
    /// checkpoint that removes it never delivers it twice and never loses it:
    /// the delivery and that checkpoint are one store write.
    #[test]
    fn a_kill_between_delivery_and_checkpoint_never_delivers_twice() {
        for kill in [
            Kill::BeforeTheWrite,
            Kill::BeforeTheRelease,
            Kill::AfterTheRelease,
        ] {
            let store = MemoryStore::new();
            let mut hub = host("hub", "timer", &[], 0, 0, true, |_| {});
            let initial = saved(&[], vec![tick_at(1_000_100)]);

            let mut first = timing(0, 0);
            let mut writer = sink(&store, "timer", 0);
            writer
                .write(&first.checkpoint(1, initial.clone(), false).unwrap())
                .unwrap();
            first.durable(1);
            first.environment_mut().now = 100;
            let after = fire(&mut first, &initial, 1_000_100);
            if kill != Kill::BeforeTheWrite {
                writer
                    .write(&first.checkpoint(2, after, false).unwrap())
                    .unwrap();
                if kill == Kill::AfterTheRelease {
                    first.durable(2);
                }
            }
            let mut delivered = deliver(&mut first, &mut hub);
            drop(first);

            let loaded = load(&store, "timer").unwrap().unwrap();
            let mut writer = sink(&store, "timer", 200);
            let mut second = timing(1, 200);
            second.restore(&loaded.checkpoint).unwrap();
            let after = fire(&mut second, &loaded.checkpoint.state, 1_000_200);
            let next = loaded.checkpoint.sequence + 1;
            writer
                .write(&second.checkpoint(next, after, false).unwrap())
                .unwrap();
            second.durable(next);
            delivered += deliver(&mut second, &mut hub);

            assert_eq!(
                delivered, 1,
                "{kill:?}: the tick was delivered {delivered} times"
            );
            let last = load(&store, "timer").unwrap().unwrap();
            assert_eq!(
                last.checkpoint.state.earliest_due(),
                None,
                "{kill:?}: the send is gone from the saved state"
            );
        }
    }

    /// A checkpoint that holds a send it has not released is due at once, though its machine
    /// waits for nothing: a process that fails after writing it and before releasing the send
    /// leaves the session to its store's index of dues alone, and the next process loads it there
    /// and its restore releases the send. Once a checkpoint taken after the release is written,
    /// the session waits for nothing.
    #[test]
    fn a_checkpoint_holding_a_send_it_has_not_released_is_due_at_once() {
        let store = MemoryStore::new();
        let initial = saved(&[], vec![tick_at(1_000_100)]);
        let mut first = timing(0, 0);
        let mut writer = sink(&store, "timer", 0);
        writer
            .write(&first.checkpoint(1, initial.clone(), false).unwrap())
            .unwrap();
        first.durable(1);
        assert_eq!(next_wake(&store).unwrap(), Some(1_000_100));

        first.environment_mut().now = 100;
        let after = fire(&mut first, &initial, 1_000_100);
        let holding = first.checkpoint(2, after, false).unwrap();
        assert_eq!(
            holding.state.earliest_due(),
            None,
            "the send left the state"
        );
        assert_eq!(holding.earliest_due(), Some(DUE_AT_ONCE));
        writer.write(&holding).unwrap();
        // The process fails here: the checkpoint is durable, the send never left.
        assert!(sent(&mut first).is_empty());
        drop(first);

        assert_eq!(next_wake(&store).unwrap(), Some(DUE_AT_ONCE));
        let woken = wake_due(&store, 1_000_200, 10).unwrap();
        let [loaded] = woken.as_slice() else {
            panic!("the session holding the send is not woken: {woken:?}");
        };
        let mut hub = host("hub", "timer", &[], 0, 0, true, |_| {});
        let mut second = timing(1, 200);
        second.restore(&loaded.checkpoint).unwrap();
        assert_eq!(deliver(&mut second, &mut hub), 1, "the restore releases it");
        // The host applies what the restore queued for the machine before it saves it again.
        second.take_calls();

        let mut writer = sink(&store, "timer", 200);
        let next = loaded.checkpoint.sequence + 1;
        let released = second
            .checkpoint(next, loaded.checkpoint.state.clone(), false)
            .unwrap();
        assert_eq!(released.earliest_due(), None);
        writer.write(&released).unwrap();
        second.durable(next);
        assert_eq!(next_wake(&store).unwrap(), None);
    }

    // ── Durable receipt ──────────────────────────────────────────────────

    const DURABLE: DurableSend = DurableSend {
        window: 2,
        retry_ms: 100,
    };

    /// `ecu`, which sends to `hmi` with durable receipt, its deployment
    /// checkpointing it.
    fn durable_sender(incarnation: u8, now: i64, ready: bool) -> Host {
        host("ecu", "hmi", &[], incarnation, now, ready, |router| {
            router.enable_checkpoints();
            router.send_durably("hmi", DURABLE).unwrap();
        })
    }

    /// `hmi`, which receives from `ecu` with durable receipt.
    fn durable_receiver() -> Host {
        host("hmi", "ecu", &[], 0, 0, true, |router| {
            router.enable_checkpoints();
            router.receive_durably("ecu").unwrap();
        })
    }

    fn note(event: &str) -> HostSendRequest {
        HostSendRequest {
            processor_type: MESH_PROCESSOR_TYPE.to_string(),
            event_name: event.to_string(),
            target: "#hmi".to_string(),
            ..HostSendRequest::default()
        }
    }

    /// A durable envelope is sent again after `retry_ms` until its receiver
    /// acknowledges it, and the receiver acknowledges only once a durable
    /// checkpoint holds it; a repeat it already admitted is dropped by its
    /// sequence number and answered with the acknowledgement it can give.
    #[test]
    fn a_durable_envelope_is_sent_again_until_its_receiver_acknowledges_it() {
        let mut ecu = durable_sender(0, 0, true);
        let mut hmi = durable_receiver();
        ecu.send(&note("go"));
        ecu.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
        ecu.durable(1);
        let first = sent(&mut ecu);
        assert_eq!(first.len(), 1, "released once durable");
        let envelope = Envelope::decode(&mut SceCursor::new(&first[0])).unwrap();
        assert_eq!(
            envelope.sequence_no,
            Some(1),
            "a durable binding numbers every envelope"
        );

        ecu.environment_mut().now = 99;
        ecu.tick();
        assert!(sent(&mut ecu).is_empty(), "sent again before retry_ms");
        ecu.environment_mut().now = 100;
        ecu.tick();
        let again = sent(&mut ecu);
        assert_eq!(
            again, first,
            "sent again with its id, sequence number and data"
        );

        hmi.receive("ecu", again[0].clone());
        assert_eq!(names(&mut hmi), ["go"]);
        hmi.receive("ecu", again[0].clone());
        assert!(
            hmi.take_calls().is_empty(),
            "a repeat is dropped by its number"
        );
        assert!(
            sent(&mut hmi).is_empty(),
            "nothing is acknowledged before a durable checkpoint"
        );

        hmi.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
        hmi.durable(1);
        let receipts = sent(&mut hmi);
        assert_eq!(receipts.len(), 1);
        let receipt = Envelope::decode(&mut SceCursor::new(&receipts[0])).unwrap();
        assert_eq!(receipt.pattern, PatternKind::DurableReceipt);
        assert_eq!(receipt.source, "hmi");
        assert_eq!(receipt.sequence_no, Some(1));
        hmi.receive("ecu", again[0].clone());
        assert_eq!(
            sent(&mut hmi).len(),
            1,
            "a repeat is answered with the acknowledgement"
        );

        ecu.receive("hmi", receipts[0].clone());
        assert!(
            ecu.take_calls().is_empty(),
            "an acknowledgement reaches no engine"
        );
        ecu.environment_mut().now = 1_000;
        ecu.tick();
        assert!(
            sent(&mut ecu).is_empty(),
            "nothing acknowledged is sent again"
        );
        assert_eq!(ecu.next_retry_due_ms(), None);
    }

    /// An endpoint that holds an envelope for a checkpoint not yet durable, or
    /// for a durable receiver's acknowledgement, is not quiet: a host that
    /// dropped it then would lose the envelope, or never send it again.
    #[test]
    fn an_endpoint_holding_for_durability_is_not_quiet() {
        let mut ecu = durable_sender(0, 0, true);
        let mut hmi = durable_receiver();
        assert!(ecu.is_quiet());
        ecu.send(&note("go"));
        assert!(!ecu.is_quiet(), "held for its checkpoint");
        ecu.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
        assert!(!ecu.is_quiet(), "sealed, and not yet durable");
        ecu.durable(1);
        let released = sent(&mut ecu);
        assert_eq!(released.len(), 1);
        assert!(!ecu.is_quiet(), "released, and not yet acknowledged");

        hmi.receive("ecu", released[0].clone());
        assert_eq!(names(&mut hmi), ["go"]);
        hmi.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
        hmi.durable(1);
        let receipts = sent(&mut hmi);
        assert_eq!(receipts.len(), 1);
        ecu.receive("hmi", receipts[0].clone());
        assert!(ecu.is_quiet(), "acknowledged, it holds nothing");
    }

    /// A durable binding holds at most its window: a send past it is refused
    /// as `backpressureDrop`, and nothing it accepted is dropped.
    #[test]
    fn a_durable_binding_refuses_a_send_past_its_window() {
        let mut ecu = durable_sender(0, 0, true);
        ecu.send(&note("a"));
        ecu.send(&note("b"));
        assert!(ecu.take_calls().is_empty());
        ecu.send(&note("c"));
        let rows = ecu.take_calls();
        assert!(
            matches!(
                rows.as_slice(),
                [EngineCall::Raise(event)] if event.metadata.data.contains(r#""reason":"backpressureDrop""#)
                    && event.metadata.data.contains(r#""queue_depth":2"#)
            ),
            "{rows:?}"
        );
        ecu.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
        ecu.durable(1);
        assert_eq!(sent(&mut ecu).len(), 2, "both accepted envelopes left");
    }

    /// A restored durable sender sends again, in order and at its link's
    /// readiness edge, every envelope no acknowledgement covered, with the
    /// ids and numbers they first had; one acknowledged stays acknowledged.
    #[test]
    fn a_restored_durable_sender_sends_again_what_was_not_acknowledged() {
        let store = MemoryStore::new();
        let mut writer = sink(&store, "ecu", 0);
        let mut ecu = durable_sender(0, 0, true);
        let mut hmi = durable_receiver();
        let mut first = Vec::new();
        for (n, event) in [(1, "a"), (2, "b"), (3, "c")] {
            ecu.send(&note(event));
            writer
                .write(&ecu.checkpoint(n, saved(&[], Vec::new()), false).unwrap())
                .unwrap();
            ecu.durable(n);
            first.extend(sent(&mut ecu));
            if n == 1 {
                // `a` is acknowledged; `b` and `c` are lost on the way.
                hmi.receive("ecu", first[0].clone());
                hmi.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
                hmi.durable(1);
                for receipt in sent(&mut hmi) {
                    ecu.receive("hmi", receipt);
                }
            }
        }
        drop(ecu);

        let loaded = load(&store, "ecu").unwrap().unwrap();
        let mut restored = durable_sender(1, 500, false);
        restored.restore(&loaded.checkpoint).unwrap();
        assert!(
            sent(&mut restored).is_empty(),
            "nothing is sent before the link is ready"
        );
        restored.peer_ready("hmi");
        assert_eq!(
            sent(&mut restored),
            &first[1..],
            "b and c, as they first left"
        );
    }

    /// A restore tells each source the machine holds receive state for that
    /// what it sent after the checkpoint may be lost, except a source whose
    /// binding receives durably, which loses nothing, and except after a
    /// clean checkpoint.
    #[test]
    fn a_restore_tells_each_source_that_may_have_lost_an_envelope() {
        let receiver = || {
            let mut router = Router::new("hmi", 8, 50).unwrap();
            router.add_peer("ecu", config("ecu", &[]));
            router.add_peer("hud", config("hud", &[]));
            router.enable_checkpoints();
            router.receive_durably("ecu").unwrap();
            Endpoint::new(router, Recorder::default(), Fixed::default())
        };
        let mut ecu = durable_sender(0, 0, true);
        ecu.send(&note("go"));
        ecu.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
        ecu.durable(1);
        let mut hud = host("hud", "hmi", &[], 0, 0, true, |_| {});
        hud.send(&note("look"));

        let mut hmi = receiver();
        hmi.receive("ecu", sent(&mut ecu).remove(0));
        hmi.receive("hud", sent(&mut hud).remove(0));
        assert_eq!(names(&mut hmi), ["go", "look"]);
        let checkpoint = hmi.checkpoint(1, saved(&[], Vec::new()), false).unwrap();

        let mut restored = receiver();
        restored.restore(&checkpoint).unwrap();
        let rows = restored.take_calls();
        assert!(
            matches!(
                rows.as_slice(),
                [EngineCall::Raise(event)] if event.metadata.data
                    == r#"{"errorName":"communication","reason":"restored","source":"hud"}"#
            ),
            "{rows:?}"
        );

        let mut clean = receiver();
        clean
            .restore(&Checkpoint {
                clean: true,
                ..checkpoint
            })
            .unwrap();
        assert!(
            clean.take_calls().is_empty(),
            "a clean checkpoint lost nothing"
        );
    }

    /// On a durable ordered stream a gap is an envelope its sender still
    /// holds: it is never timed out, and what follows it waits for it.
    #[test]
    fn a_durable_ordered_stream_never_times_out_a_gap() {
        let mut router = Router::new("hmi", 8, 50).unwrap();
        router.add_peer(
            "ecu",
            PeerConfig {
                delivery: Delivery {
                    dedup: true,
                    ordered: true,
                },
                ..config("ecu", &[])
            },
        );
        router.enable_checkpoints();
        router.receive_durably("ecu").unwrap();
        let mut hmi: Host = Endpoint::new(router, Recorder::default(), Fixed::default());

        let mut ecu = durable_sender(0, 0, true);
        for (n, event) in [(1, "a"), (2, "b")] {
            ecu.send(&note(event));
            ecu.checkpoint(n, saved(&[], Vec::new()), false).unwrap();
            ecu.durable(n);
        }
        let stream = sent(&mut ecu);
        hmi.receive("ecu", stream[1].clone());
        hmi.environment_mut().now = 10_000;
        hmi.tick();
        assert!(
            hmi.take_calls().is_empty(),
            "no gap of a durable stream times out"
        );
        hmi.receive("ecu", stream[0].clone());
        assert_eq!(names(&mut hmi), ["a", "b"]);
    }

    /// A checkpoint is written as the text it is read back from, every member
    /// of every section included.
    #[test]
    fn a_checkpoint_reads_back_as_it_was_written() {
        let checkpoint = Checkpoint {
            sequence: 7,
            clean: true,
            state: saved(&[r#"{"n":1}"#.to_string()], vec![tick_at(1_000_100)]),
            followed: BTreeMap::from([("directory".to_string(), 3), ("rooms".to_string(), 12)]),
            endpoints: vec![EndpointSection {
                machine: "server".to_string(),
                router: RouterState {
                    peers: vec![PeerState {
                        peer: "client".to_string(),
                        next_sequence: 4,
                        queued: vec![QueuedState {
                            envelope: vec![0x00, 0xab],
                            since: 1_000_050,
                        }],
                        unacked: vec![UnackedState {
                            sequence: 3,
                            envelope: vec![0xff],
                        }],
                        received: Some(ReceivedState {
                            held: 2,
                            above: vec![4, 6],
                        }),
                    }],
                    senders: vec![
                        SenderState {
                            source: "client".to_string(),
                            window: vec![[1; 16], [2; 16]],
                            next: Some(5),
                            held: vec![HeldState {
                                sequence: 6,
                                arrived: 1_000_060,
                                envelope: vec![0x10],
                            }],
                            replies: vec![
                                ReplyState {
                                    request: [1; 16],
                                    invoke: [9; 16],
                                    reply: Some(vec![0x20, 0x21]),
                                },
                                ReplyState {
                                    request: [2; 16],
                                    invoke: [8; 16],
                                    reply: None,
                                },
                            ],
                            durable: true,
                            records: None,
                        },
                        SenderState {
                            source: "phone/a1".to_string(),
                            window: Vec::new(),
                            next: None,
                            held: Vec::new(),
                            replies: Vec::new(),
                            durable: true,
                            records: Some(RecordsState {
                                incarnation: 12,
                                mark: 3,
                                kept: vec![
                                    RecordState {
                                        number: 3,
                                        invoke: Some([7; 16]),
                                        reply: Some(vec![0x22, 0x23]),
                                    },
                                    RecordState {
                                        number: 4,
                                        invoke: None,
                                        reply: None,
                                    },
                                ],
                            }),
                        },
                    ],
                    unreleased: vec![
                        Unreleased {
                            peer: "client".to_string(),
                            sequence: Some(3),
                            envelope: vec![0x30],
                        },
                        Unreleased {
                            peer: "client".to_string(),
                            sequence: None,
                            envelope: vec![0x31],
                        },
                    ],
                },
                retries: vec![RetryState {
                    peer: "client".to_string(),
                    envelope: vec![0x40],
                    made: 2,
                    backoff: Some(200),
                    due: 1_000_300,
                }],
                calls: vec![
                    Told::Event {
                        name: "service.request.add".to_string(),
                        data: r#"{"n":2}"#.to_string(),
                        source: "client".to_string(),
                        send_id: "s-1".to_string(),
                        invoke_id: "09090909-0909-0909-0909-090909090909".to_string(),
                        principal: None,
                    },
                    Told::End {
                        invoke_id: "ask".to_string(),
                        token: 3,
                        failed: true,
                        data: "late".to_string(),
                        source: None,
                    },
                ],
            }],
        };
        let text = checkpoint.to_json();
        assert_eq!(Checkpoint::from_json(&text), Ok(checkpoint));
    }

    /// The shared instance of the surface, which `sce-build` holds to the
    /// schema (`sce-build/tests/mesh_checkpoint_schema.rs`), is read by this
    /// core and written back byte for byte: what the schema accepts there is
    /// what this core writes.
    #[test]
    fn the_shared_instance_is_what_this_core_writes() {
        let text =
            include_str!("../../../../sce-build/tests/fixtures/mesh_checkpoint/checkpoint.json")
                .trim_end();
        let checkpoint = Checkpoint::from_json(text).unwrap();
        assert_eq!(checkpoint.endpoints.len(), 1);
        assert_eq!(checkpoint.to_json(), text);
    }

    /// `SCHEMA_STATUS` and the schema's header move together
    /// (`SCE_WIRE_CONTRACTS.md`).
    #[test]
    fn schema_file_declares_status() {
        let schema = json::parse(include_str!(
            "../../../../schemas/sce-mesh-checkpoint.v1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema.member("x-sce-schema-status"),
            Some(&Value::Text(SCHEMA_STATUS.to_string()))
        );
    }

    /// What is not a checkpoint of this format is refused, saying where.
    #[test]
    fn what_is_not_a_checkpoint_is_refused() {
        let checkpoint = Checkpoint {
            sequence: 1,
            clean: false,
            state: saved(&[], Vec::new()),
            endpoints: Vec::new(),
            followed: BTreeMap::new(),
        };
        let text = checkpoint.to_json();
        assert!(matches!(
            Checkpoint::from_json(&text.replacen(r#""format":1"#, r#""format":2"#, 1)),
            Err(CheckpointError::Malformed(_))
        ));
        assert!(matches!(
            Checkpoint::from_json(&text.replacen(r#""checkpoint":"1""#, r#""checkpoint":"+1""#, 1)),
            Err(CheckpointError::Malformed(_))
        ));
        assert!(matches!(
            Checkpoint::from_json("{"),
            Err(CheckpointError::Malformed(_))
        ));
        assert_eq!(
            as_bytes(&Value::Text("0g".to_string()), "x"),
            Err(CheckpointError::Malformed(
                "'x' is not bytes written as lowercase hexadecimal".to_string()
            ))
        );
        assert!(
            as_bytes(&Value::Text("ABCD".to_string()), "x").is_err(),
            "uppercase is not written"
        );
        assert_eq!(
            as_bytes(&Value::Text("00ff".to_string()), "x"),
            Ok(vec![0x00, 0xff])
        );
    }

    /// A restore is refused when the checkpoint holds no section for the
    /// machine, or holds state for a peer the router does not bind; a seal is
    /// refused when its number does not rise, and of a router that does not
    /// checkpoint.
    #[test]
    fn a_checkpoint_that_does_not_fit_is_refused() {
        let mut server = serving(0, 0);
        let checkpoint = server.checkpoint(1, saved(&[], Vec::new()), false).unwrap();
        assert_eq!(
            server.seal(1),
            Err(CheckpointError::Router(RouterError::CheckpointOutOfOrder {
                sealed: 1,
                given: 1
            }))
        );

        let mut other = host(
            "other",
            "client",
            &[],
            0,
            0,
            true,
            Router::enable_checkpoints,
        );
        assert_eq!(
            other.restore(&checkpoint),
            Err(CheckpointError::NoSection("other".to_string()))
        );

        let mut stranger = host(
            "server",
            "nobody",
            &[],
            0,
            0,
            true,
            Router::enable_checkpoints,
        );
        assert!(matches!(
            stranger.restore(&checkpoint),
            Err(CheckpointError::Unbound(_))
        ));

        let mut plain = asking();
        assert_eq!(
            plain.seal(1),
            Err(CheckpointError::Router(RouterError::NotCheckpointed))
        );
    }

    /// Two clocks map a monotonic moment to the wall and back, a moment past
    /// staying as far in the past.
    #[test]
    fn a_moment_crosses_between_the_two_clocks() {
        let clocks = Clocks {
            monotonic_ms: 500,
            wall_ms: 10_000,
        };
        assert_eq!(clocks.to_wall(700), 10_200);
        assert_eq!(clocks.to_wall(100), 9_600);
        assert_eq!(clocks.to_monotonic(10_200), 700);
        assert_eq!(clocks.to_monotonic(9_600), 100);
    }

    /// A moment a checkpoint wrote is placed on the monotonic clock exactly,
    /// one before the monotonic clock's start included, and not at all when
    /// it lies further from the two clocks' reading than an i64 counts;
    /// `to_monotonic` keeps such a moment at the end of the clock it lies
    /// toward. Red if `placed` saturates instead of refusing.
    #[test]
    fn a_moment_is_placed_exactly_or_not_at_all() {
        let clocks = Clocks {
            monotonic_ms: 500,
            wall_ms: 10_000,
        };
        assert_eq!(clocks.placed(10_200), Some(700));
        assert_eq!(clocks.placed(9_600), Some(100));
        assert_eq!(clocks.placed(0), Some(-9_500));
        assert_eq!(clocks.placed(u64::MAX), None);
        assert_eq!(clocks.to_monotonic(u64::MAX), i64::MAX);
        let far = Clocks {
            monotonic_ms: 0,
            wall_ms: u64::MAX,
        };
        assert_eq!(far.placed(0), None);
        assert_eq!(far.to_monotonic(0), i64::MIN);
        let edge = Clocks {
            monotonic_ms: -1,
            wall_ms: i64::MAX as u64,
        };
        assert_eq!(edge.placed(0), Some(i64::MIN));
        assert_eq!(edge.placed(i64::MAX as u64 + 1), Some(0));
    }
}
