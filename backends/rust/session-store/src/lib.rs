// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a server keeps the sessions of a keyed pool (ADR 0107, decision D6).
//!
//! A host core that serves a machine as a pool of sessions, one per key (D3), saves an idle
//! session and drops it from memory, and loads it again when a request or one of its delayed
//! `<send>`s needs it (D5). What it saves is the session's saved state (`sce-saved-state`, format
//! 3, `sce_rust_runtime::saved_state`), which keeps each waiting delayed send as the wall-clock
//! moment it comes due. This crate is the place those states are kept, behind one contract,
//! [`SessionStore`], so that a deployment can put another store behind it.
//!
//! The contract keeps, for each session, which is a machine and a key (ADR 0114, D2), so that
//! the machines of a deployment keep their sessions in one store and an equal key of two of them
//! names two sessions; a handle keeps the sessions of one machine ([`SessionStore::machine`]), and
//! every call that names a key names the session of that machine:
//!
//! * the saved state, which the store never reads, with a **version** that every write moves, so
//!   that a write made from a state read earlier is refused when another write came between
//!   (compare and set, [`SessionStore::put`]);
//! * the **earliest due** of the state's waiting sends, written in the same transaction as the
//!   state, so that the index the host core wakes unloaded sessions from
//!   ([`SessionStore::due_before`]) never disagrees with what the states hold;
//! * a **lease**: a record of the process that owns the key and until when, whose **fence** rises
//!   every time a lease on any key is granted, and which every write carries. A process that lost
//!   its lease, through a pause longer than the lease or a clock that ran slow, still believes it
//!   holds it and may still write; the store refuses the write because its fence is no longer the
//!   current one. That is the fencing token of Kleppmann's analysis of distributed locks: the clock
//!   decides only when another process may take a key over, and the fence, checked where the data
//!   is, decides whose writes land, so two server processes never both write one session.
//!
//! Everything one call writes is one transaction: a put writes the state, its version and its
//! earliest due, after the lease check, or writes nothing. Many writes may share one transaction
//! ([`SessionStore::write_many`], ADR 0109 D15; module [`batch`]): the batch is made durable once,
//! and each of its items is decided on its own, so that a host commits the checkpoints of many
//! sessions together and pays for one sync.
//!
//! * [`MemoryStore`] keeps it all in the process, for the tests of the crates that are given a
//!   store.
//! * `SqliteStore` (feature `sqlite`, on by default) is the reference store: SQLite in WAL mode
//!   with `synchronous=FULL`, so a put that returned survives a power loss.
//! * [`conformance`] is what every implementation is held to: both of these run it, and a store
//!   written elsewhere runs the same functions.
//!
//! The keys a host generates and derives are kept behind a contract of their own, [`KeyStore`]
//! (ADR 0109, D7; module [`keys`]): [`MemoryKeys`] in memory, and `SqliteStore` in the same file
//! as the sessions, held to [`key_conformance`].
//!
//! The credentials a server checks a client's name by, the installation pins of the `install`
//! authenticator and the bearer tokens its host issues, are kept behind [`CredentialStore`] (ADR
//! 0109, D3; module [`credentials`]), each as the SHA-256 of its secret: [`MemoryCredentials`] in
//! memory, and `SqliteStore` in the same file (layout 3), held to [`credential_conformance`].
//!
//! What a server sends a client of a durable `https` binding is kept in the client's **outbox**
//! until the client acknowledges it (ADR 0109, D10; module [`outbox`]): appended by the write
//! that keeps the checkpoint of the session that sent it ([`SessionStore::write`]), deleted by
//! the acknowledgement that covers it, and expired, with a note for its sender, once it is older
//! than the binding keeps. Both stores keep it (`SqliteStore` from layout 5), held to
//! [`outbox_conformance`].
//!
//! The values of the key-value feature a host serves its machines are kept behind [`KvStore`]
//! (ADR 0109, D13; module [`kv`]), with the first result of each put-if by the identity of its
//! invocation in the transaction that judged it: [`MemoryStore`] in memory, beside its sessions,
//! and `SqliteStore` in the same file (layout 6), held to [`kv_conformance`]. A removal of a
//! session deletes the first results of the sessions it ends ([`Write::sessions`]).
//!
//! A host's audit log is kept behind [`AuditStore`] (ADR 0109, D14; module [`audit`]): streams of
//! rows appended in order, each chained on the one before by its writer, the signed checkpoints
//! beside them, and segments deleted whole by retention: [`MemoryStore`] in memory, and
//! `SqliteStore` in the same file (layout 7), held to [`audit_conformance`]. A removal of a
//! session deletes the identities its sessions appended under, never a row.
//!
//! The facts the sessions made are kept in the **journal**, behind [`JournalStore`] (ADR 0114;
//! module [`journal`]): appended by the write that keeps the checkpoint that holds them
//! ([`Write::journal`]), in one order of positions a reader reads up to the horizon, and read as of
//! an earlier position and a valid time; beside them, what another system's adapter observed, once
//! for each id the other system gave. [`MemoryStore`] in memory, and `SqliteStore` in the same file
//! (layout 8), held to [`journal_conformance`]. A removal of a session removes none of it. How far
//! each of its readers has taken it in is kept beside it ([`JournalStore::keep_read`], layout 12).
//!
//! The members of an open machine (ADR 0113, D4 and D5; module [`membership`]) are kept behind
//! [`MembershipStore`]: an offer per joining instance, a member per confirmed one with its lease
//! and its low-water mark, and the condition every write made under an incarnation is kept on
//! ([`Write::under`]): [`MemoryStore`] in memory, and `SqliteStore` in the same file (layout 9),
//! held to [`membership_conformance`]. Beside a member, the person its incarnation acts for (ADR
//! 0116, D1 and D6; [`Delegation`]) until a moment no exchange moves, which a renewal moves and a
//! revocation ends with the incarnation (layout 10); and whether an offer waits for a proof of a
//! person, which alone confirms it (layout 11).
//!
//! # Building the SQLite store
//!
//! The reference store links the system's SQLite (3.14 or later), found through `pkg-config`, or
//! through `vcpkg` on Windows with MSVC; it does not compile a copy of its own into the program,
//! because a database engine takes its security fixes from the one place the system updates. Where
//! none is installed:
//!
//! * Debian and Ubuntu: `apt install libsqlite3-dev pkg-config`; Fedora: `dnf install sqlite-devel
//!   pkgconf`; macOS: `brew install sqlite pkg-config`, or the SDK's `libsqlite3` with
//!   `SQLITE3_LIB_DIR` set; Windows: `vcpkg install sqlite3:x64-windows`.
//! * Elsewhere, `SQLITE3_LIB_DIR` (and `SQLITE3_INCLUDE_DIR`) name a build of SQLite, and
//!   `SQLITE3_STATIC=1` links it statically; these are read by `libsqlite3-sys`.
//! * A program that would rather carry its own copy decides that for itself: it depends on
//!   `libsqlite3-sys` with the feature `bundled`, and cargo's feature unification gives this crate
//!   the same copy. It then owns that copy's updates.
//!
//! WAL keeps an index in shared memory beside the database, so the database must be on a local
//! file system, not a network one.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

pub mod audit;
pub mod audit_conformance;
pub mod batch;
pub mod conformance;
pub mod credential_conformance;
pub mod credentials;
pub mod journal;
pub mod journal_conformance;
pub mod key_conformance;
pub mod keys;
pub mod kv;
pub mod kv_conformance;
pub mod membership;
pub mod membership_conformance;
mod memory;
pub mod outbox;
pub mod outbox_conformance;
mod rules;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use audit::{
    AuditAppend, AuditCheckpoint, AuditHead, AuditRow, AuditStore, Chain, NoCheckpointAt,
};
pub use batch::{Batched, Committed, Decided};
pub use credentials::{
    CredentialStore, Digest, MemoryCredentials, PinRow, PinStore, Revoked, TokenRow, TokenStore,
};
pub use journal::{
    Actor, ActorName, AsOf, AsOfCursor, AsOfError, AtPosition, Fact, FactData, Journal,
    JournalEntry, JournalPage, JournalStore, NoJournal, NoPositionLeft, Observation, ObservedActor,
    ObservedRead, ObservedStream, PastHorizon, Read, SessionsStream, Stream, StreamName,
};
pub use keys::{KeyRow, KeyState, KeyStore, KeyUse, MemoryKeys};
pub use kv::{Identity, KvEntry, KvStore, Put, PutIf};
pub use membership::{
    Admission, Delegation, Ended, Joining, Member, Membership, MembershipStore, NoMembership,
    Offer, Offered, Person, PersonKey, Renewal, Revocation, Terms, Under,
};
pub use memory::MemoryStore;
pub use outbox::{
    Append, Appended, Change, Expired, Expiry, NoOutbox, Note, Outbox, OutboxEntry, Write, Written,
};
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;

/// The latest moment a store holds, in milliseconds since the Unix epoch: the largest signed
/// 64-bit number, since that is what SQLite's integers are, and what the saved state's own
/// moments are clamped to. A due or an expiry later than this is kept as this; it is some 292
/// million years after the epoch, so no clock gives one.
pub const LATEST_MOMENT: u64 = i64::MAX as u64;

/// A moment as a store keeps it: `ms`, or [`LATEST_MOMENT`] if it is later.
pub fn moment(ms: u64) -> u64 {
    ms.min(LATEST_MOMENT)
}

/// A store of saved sessions, one per key. See the crate documentation for what it keeps and why.
///
/// Every method is one transaction: it reads and writes what it names, or, when it returns an
/// error, writes nothing. Numbers the store hands out, a version or a fence, come from one counter
/// of the store that only rises, so neither is ever given twice, to the same key or to another,
/// even after a session was deleted or a lease released. A handle may be shared between threads;
/// whether two handles on one store may live in two processes is the implementation's to say.
pub trait SessionStore: Send + Sync {
    /// The machine whose sessions this handle keeps (ADR 0114, D2): every call that names a key
    /// names the session of this machine and that key, its state, its lease and its notes, and an
    /// equal key of another machine names another session. The empty name is the machine of a
    /// store that keeps one, which is what a store answers unless it keeps several apart
    /// ([`MemoryStore::of_machine`], `SqliteStore::of_machine`). What is not a session's (the
    /// journal, the outboxes, the counter that numbers versions and fences) is one for every
    /// machine of the store.
    fn machine(&self) -> &str {
        ""
    }

    /// The session `key` names, or `None` if it holds none.
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure>;

    /// The contract's one write of a session, in one transaction: keep or remove the session
    /// `key` names ([`Write::change`]), append each of [`Write::outbox`] to its client's outbox
    /// (module [`outbox`]), append each fact of [`Write::journal`] to the session's stream of the
    /// journal (module [`journal`]), and take the session's notes through
    /// [`Write::notes_through`]; answer the session's new version (`0` once removed) and where each
    /// append was kept.
    ///
    /// Refused, writing nothing of any of it, unless [`Write::fence`] is the fence of the lease
    /// currently held on `key` ([`Conflict::Fence`], checked first: a fenced-out writer's view of
    /// the version says nothing), the session's version is [`Write::expected_version`], `0`
    /// meaning it holds no session ([`Conflict::Version`]), and each incarnation of
    /// [`Write::under`] is the one its member stands under at [`Write::now_ms`]
    /// ([`Conflict::Incarnation`], ADR 0113 D5). A write that is kept makes each of those members
    /// kept, leased from [`Write::now_ms`] for its lease (module [`membership`]); a store that
    /// keeps no members fails a write that names one with [`NoMembership`], writing nothing. The
    /// lease's expiry is not checked: while a lease that ran out is not taken over, no one else
    /// can write the key and its holder still may; the moment another process takes it, the new
    /// fence refuses the old holder's writes. Removing a session the store holds none of, from version `0`, removes nothing and
    /// succeeds; a removal takes every note of the session. The lease is not touched: it is the
    /// holder's to release.
    ///
    /// An append to a client whose outbox the store holds none of opens one in the append's
    /// incarnation; an outbox it holds keeps its own. Each append takes the next number of its
    /// outbox, in the order given, and is kept at [`Write::now_ms`].
    ///
    /// Each fact takes a position above every one the journal gave before, in the order given. A
    /// store that keeps no journal fails a write that gives facts with [`NoJournal`], writing
    /// nothing, rather than lose them; every store of this crate keeps one ([`JournalStore`]), held
    /// to [`journal_conformance`]. The stream is the session's: [`Journal::machine`] names the
    /// machine of the handle the write is made through, which its host keeps it to.
    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError>;

    /// Write `batch` in one transaction, durable as a whole when this returns, and decided item by
    /// item, in the order given (ADR 0109, D15; module [`batch`]): each item is judged as
    /// [`SessionStore::write`] or [`SessionStore::acknowledge`] would judge it alone, against what
    /// the items before it left, and one the store refuses for its version or its fence
    /// ([`WriteError::Conflict`]), or that fails alone ([`WriteError::Failed`]), leaves nothing of
    /// itself while the others are kept. Answers each item's decision, in the order given.
    ///
    /// Refused as a whole, keeping nothing of any item, when the transaction cannot be made or
    /// made durable ([`Failure`]): a medium that failed. A store answers an item failed only when
    /// it can leave that item out and still keep the others.
    ///
    /// The name is the contract's word for a change of the store: a batch keeps and removes
    /// sessions, appends to outboxes and the journal and takes notes, as [`SessionStore::write`]
    /// does, and deletes what acknowledgements cover; [`SessionStore::put`] is the narrower word
    /// for a write that only keeps a state.
    ///
    /// As it stands, each item is its own transaction: correct, durable on return, and no cheaper
    /// than the items written one by one. A store that can share one transaction among them
    /// implements this, as both stores of this crate do, held to
    /// [`conformance::a_batch_is_decided_item_by_item`] and its neighbours.
    fn write_many(&self, batch: &[Batched<'_>]) -> Result<Vec<Decided>, Failure> {
        Ok(batch
            .iter()
            .map(|item| match *item {
                Batched::Write { key, write } => self.write(key, &write).map(Committed::Written),
                Batched::Acknowledge {
                    client,
                    incarnation,
                    through,
                    now_ms,
                } => self
                    .acknowledge(client, incarnation, through, now_ms)
                    .map(Committed::Acknowledged)
                    .map_err(WriteError::Failed),
            })
            .collect())
    }

    /// Keep `state` as the session `key` names, with `earliest_due` as the earliest moment one of
    /// its waiting sends comes due (`None` when none waits), and answer its new version: a
    /// [`SessionStore::write`] that appends nothing and takes no note. A store implements
    /// [`SessionStore::write`] and leaves this as it is.
    fn put(
        &self,
        key: &str,
        state: &[u8],
        earliest_due: Option<u64>,
        expected_version: u64,
        fence: u64,
    ) -> Result<u64, WriteError> {
        self.write(
            key,
            &Write::put(state, earliest_due, expected_version, fence),
        )
        .map(|written| written.version)
    }

    /// Remove the session `key` names, under the same two checks as [`SessionStore::write`]; with
    /// `expected_version` `0` and no session held, there is nothing to remove and it succeeds. A
    /// [`SessionStore::write`] that appends nothing; a store leaves this as it is.
    fn delete(&self, key: &str, expected_version: u64, fence: u64) -> Result<(), WriteError> {
        self.write(key, &Write::delete(expected_version, fence))
            .map(|_| ())
    }

    /// At most `limit` sessions whose earliest due is at or before `ms`, earliest first, sessions
    /// due at the same moment in the order of the entries the store indexes them under
    /// ([`SessionStore::indexed`]). At or before, because a send due at the moment a restore runs
    /// is armed as due now, as `sce_rust_runtime::saved_state` arms it. The host core keeps a timer
    /// on the first of `due_before(LATEST_MOMENT, 1)` and loads what this lists when it fires, by
    /// the key [`SessionStore::key_of`] answers for each entry.
    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure>;

    /// The entry the store indexes the session `key` under: what [`SessionStore::due_before`] and
    /// [`SessionStore::indexed_after`] list it as. A store that keeps its keys as they are
    /// answers `key`; one that keeps each key as a keyed digest of it, so that no key is in the
    /// clear on its medium (ADR 0109, D8), answers that digest. It is computed, not read: it is
    /// the same for a key the store holds no session for, and two keys never share one.
    fn indexed(&self, key: &str) -> String {
        key.to_string()
    }

    /// The key of the session the store holds under the entry `indexed`, one that
    /// [`SessionStore::due_before`] or [`SessionStore::indexed_after`] listed, or `None` when it
    /// holds none there: a host wakes what came due by the key this answers. A store that keeps
    /// its keys as they are answers `indexed` itself when it holds a session there; one that keeps
    /// keyed digests reads the key from the session it holds.
    fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
        Ok(self.get(indexed)?.map(|_| indexed.to_string()))
    }

    /// At most `limit` of the sessions the store holds, as the entries it indexes them under
    /// ([`SessionStore::indexed`]), in the order of their UTF-8 bytes, each after `after` when it
    /// is given. A pass over every session (the re-seal of ADR 0109, D8) reads them a page at a
    /// time, each page after the last entry of the one before.
    fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure>;

    /// Grant `owner` the lease on `key` until `now_ms + ttl_ms`, with a fence above every fence the
    /// store granted before.
    ///
    /// Refused with [`Held`] when another owner holds a lease on `key` that has not run out at
    /// `now_ms` (a lease runs out at its `expires_ms`). A lease that ran out is taken over. An owner
    /// that asks again while it holds the lease is granted a new one, whose fence refuses the
    /// earlier grant's writes: that is a process that started again under the same name, and
    /// fencing out its previous life is what keeps the two from both writing.
    fn acquire(
        &self,
        key: &str,
        owner: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Lease, AcquireError>;

    /// Extend the lease of fence `fence` on `key` to `now_ms + ttl_ms`; its fence does not change.
    ///
    /// Refused with [`Lost::Superseded`] when it is no longer the lease held on `key` (another was
    /// granted, or it was released), and with [`Lost::Expired`] when it ran out at or before
    /// `now_ms`: a holder whose lease ran out acquires again, takes the new fence, and reads the
    /// session again, since another process may have written in between.
    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError>;

    /// Give up the lease of fence `fence` on `key`, so that another owner may take it at once.
    /// Refused with [`Lost::Superseded`] when it is no longer the lease held on `key`. A released
    /// lease's fence writes nothing more.
    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError>;

    // The outbox (ADR 0109, D10; module [`outbox`]). A store that serves no binding with
    // `receipt: durable` need not keep one: these answer [`NoOutbox`] unless it does, and every
    // store of this crate does, held to [`outbox_conformance`].

    /// Append `appends` to their clients' outboxes at `now_ms`, in one transaction and under no
    /// session: an answer the host sends a client in a session's place. Answers where each was
    /// kept, in their order.
    fn append(&self, appends: &[Append<'_>], now_ms: u64) -> Result<Vec<Appended>, Failure> {
        let _ = (appends, now_ms);
        Err(Failure::new(NoOutbox))
    }

    /// The outbox of `client`, oldest entry first, or `None` when the store holds none for it.
    fn outbox(&self, client: &str) -> Result<Option<Outbox>, Failure> {
        let _ = client;
        Err(Failure::new(NoOutbox))
    }

    /// The outbox of `client`, opened in `incarnation` at `now_ms` when the store holds none for
    /// it: what a client's exchange numbers in from its first exchange on.
    fn open_outbox(&self, client: &str, incarnation: &str, now_ms: u64) -> Result<Outbox, Failure> {
        let _ = (client, incarnation, now_ms);
        Err(Failure::new(NoOutbox))
    }

    /// Delete from the outbox of `client` every entry numbered at most `through`, when
    /// `incarnation` is the one it numbers in, at `now_ms`; answer how many were deleted. An
    /// acknowledgement of another incarnation counts in a numbering the outbox does not hold, and
    /// deletes nothing.
    fn acknowledge(
        &self,
        client: &str,
        incarnation: &str,
        through: u64,
        now_ms: u64,
    ) -> Result<u64, Failure> {
        let _ = (client, incarnation, through, now_ms);
        Err(Failure::new(NoOutbox))
    }

    /// Delete from the outbox of `client` at most `limit` entries appended before `before_ms`,
    /// oldest first, and, in the same transaction, write a [`Note`] for the session each was sent
    /// by, when the store still holds that session, and make it due at once (its earliest due the
    /// Unix epoch), so it is told at its next run. An outbox left holding nothing, to which
    /// nothing was appended or acknowledged since `before_ms`, is removed itself
    /// ([`Expiry::closed`]).
    fn expire(&self, client: &str, before_ms: u64, limit: usize) -> Result<Expiry, Failure> {
        let _ = (client, before_ms, limit);
        Err(Failure::new(NoOutbox))
    }

    /// The notes an expiry wrote for the session `key` that no write of it took yet, in the order
    /// written.
    fn notes(&self, key: &str) -> Result<Vec<Note>, Failure> {
        let _ = key;
        Err(Failure::new(NoOutbox))
    }

    /// At most `limit` of the clients the store holds an outbox for, as the entries
    /// [`SessionStore::indexed`] answers for their names, in the order of their UTF-8 bytes, each
    /// after `after` when it is given. A pass over every outbox (the re-seal of ADR 0109, D8) reads
    /// them a page at a time, each page after the last entry of the one before.
    fn outbox_clients_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, Failure> {
        let _ = (after, limit);
        Err(Failure::new(NoOutbox))
    }

    /// Replace the envelope of the entry numbered `number` in the outbox of `client` with
    /// `envelope` when it is `held`, its number, its moment and its sender kept, and answer whether
    /// it did: `false`, writing nothing, when the outbox holds no such entry or its envelope is
    /// another. This is how a store that seals what it keeps seals an envelope again under its
    /// active key (ADR 0109, D8).
    fn reseal_entry(
        &self,
        client: &str,
        number: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        let _ = (client, number, held, envelope);
        Err(Failure::new(NoOutbox))
    }

    /// Replace the envelope of the note `id` of the session `key` with `envelope` when it is
    /// `held`, every other field kept, and answer whether it did: `false`, writing nothing, when
    /// the session holds no such note or its envelope is another. As [`SessionStore::reseal_entry`]
    /// is for an entry.
    fn reseal_note(
        &self,
        key: &str,
        id: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        let _ = (key, id, held, envelope);
        Err(Failure::new(NoOutbox))
    }
}

/// A saved session as a store holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    /// The saved state, as it was put: the store does not read it.
    pub state: Vec<u8>,
    /// The version the put that wrote it answered; never `0`.
    pub version: u64,
    /// The earliest moment one of its waiting sends comes due, in milliseconds since the Unix
    /// epoch, or `None` when none waits.
    pub earliest_due: Option<u64>,
}

/// A session that comes due, as [`SessionStore::due_before`] lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    /// The entry the store indexes the session under ([`SessionStore::indexed`]): its key, or a
    /// keyed digest of it, whose key [`SessionStore::key_of`] answers.
    pub key: String,
    /// Its earliest due, in milliseconds since the Unix epoch.
    pub due: u64,
}

/// A lease a store granted or renewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lease {
    /// The fencing token every write under this lease carries.
    pub fence: u64,
    /// When it runs out, in milliseconds since the Unix epoch by the clock the holder gave.
    pub expires_ms: u64,
}

/// Why a store refused [`SessionStore::acquire`]: another owner holds the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub owner: String,
    /// When its lease runs out unless it is renewed: the earliest the key can be taken over.
    pub expires_ms: u64,
}

/// Why a store refused [`SessionStore::write`], [`SessionStore::put`] or [`SessionStore::delete`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// `presented` is not the fence of the lease held on the key; `current` is that lease's fence,
    /// or `None` when no lease is held.
    Fence {
        presented: u64,
        current: Option<u64>,
    },
    /// The session's version is `current`, not the `expected` the write was made from: another
    /// write came between.
    Version { expected: u64, current: u64 },
    /// The write was made under the incarnation `presented`, which the store no longer holds:
    /// its member stands under `current`, or under none (ADR 0113, D5). Nothing of a run is kept
    /// once its incarnation ended, whichever process ended it.
    Incarnation {
        presented: u64,
        current: Option<u64>,
    },
}

/// Why a store refused [`SessionStore::renew`] or [`SessionStore::release`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lost {
    /// The lease is no longer the one held on the key: `current` is the fence of the one that is,
    /// or `None` when none is.
    Superseded { current: Option<u64> },
    /// The lease ran out at `expires_ms`, before the renewal.
    Expired { expires_ms: u64 },
}

/// A store that could not do what it was asked: its medium failed, or it was given what it cannot
/// keep. Nothing was written.
#[derive(Debug)]
pub struct Failure(Box<dyn Error + Send + Sync>);

impl Failure {
    /// A failure for `cause`, which is kept as its source: a store written elsewhere wraps the
    /// error of its own medium here, as the SQLite store wraps SQLite's.
    pub fn new(cause: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(cause.into())
    }
}

/// The answer of a refused [`SessionStore::put`] or [`SessionStore::delete`].
#[derive(Debug)]
pub enum WriteError {
    /// The store refused the write; another process wrote first or holds the key.
    Conflict(Conflict),
    Failed(Failure),
}

/// The answer of a refused [`SessionStore::acquire`].
#[derive(Debug)]
pub enum AcquireError {
    Held(Held),
    Failed(Failure),
}

/// The answer of a refused [`SessionStore::renew`] or [`SessionStore::release`].
#[derive(Debug)]
pub enum LeaseError {
    Lost(Lost),
    Failed(Failure),
}

impl fmt::Display for Held {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the key is leased to '{}' until {}",
            self.owner, self.expires_ms
        )
    }
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fence {
                presented,
                current: Some(current),
            } => write!(
                f,
                "fence {presented} is not the current lease's, which is {current}"
            ),
            Self::Fence {
                presented,
                current: None,
            } => write!(
                f,
                "fence {presented} is not held: no lease is held on the key"
            ),
            Self::Version { expected, current } => write!(
                f,
                "the session is at version {current}, not the {expected} the write was made from"
            ),
            Self::Incarnation {
                presented,
                current: Some(current),
            } => write!(
                f,
                "the write was made under incarnation {presented}, and its member stands under \
                 {current}"
            ),
            Self::Incarnation {
                presented,
                current: None,
            } => write!(
                f,
                "the write was made under incarnation {presented}, and its member stands no more"
            ),
        }
    }
}

impl fmt::Display for Lost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Superseded {
                current: Some(current),
            } => write!(f, "the lease was superseded by fence {current}"),
            Self::Superseded { current: None } => {
                write!(
                    f,
                    "the lease is no longer held: no lease is held on the key"
                )
            }
            Self::Expired { expires_ms } => write!(f, "the lease ran out at {expires_ms}"),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the session store failed: {}", self.0)
    }
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict(conflict) => write!(f, "{conflict}"),
            Self::Failed(failure) => write!(f, "{failure}"),
        }
    }
}

impl fmt::Display for AcquireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Held(held) => write!(f, "{held}"),
            Self::Failed(failure) => write!(f, "{failure}"),
        }
    }
}

impl fmt::Display for LeaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lost(lost) => write!(f, "{lost}"),
            Self::Failed(failure) => write!(f, "{failure}"),
        }
    }
}

impl Error for Held {}
impl Error for Conflict {}
impl Error for Lost {}

impl Error for Failure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.0)
    }
}

impl Error for WriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Conflict(conflict) => Some(conflict),
            Self::Failed(failure) => Some(failure),
        }
    }
}

impl Error for AcquireError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Held(held) => Some(held),
            Self::Failed(failure) => Some(failure),
        }
    }
}

impl Error for LeaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Lost(lost) => Some(lost),
            Self::Failed(failure) => Some(failure),
        }
    }
}

impl From<Conflict> for WriteError {
    fn from(conflict: Conflict) -> Self {
        Self::Conflict(conflict)
    }
}

impl From<Failure> for WriteError {
    fn from(failure: Failure) -> Self {
        Self::Failed(failure)
    }
}

impl From<Held> for AcquireError {
    fn from(held: Held) -> Self {
        Self::Held(held)
    }
}

impl From<Failure> for AcquireError {
    fn from(failure: Failure) -> Self {
        Self::Failed(failure)
    }
}

impl From<Lost> for LeaseError {
    fn from(lost: Lost) -> Self {
        Self::Lost(lost)
    }
}

impl From<Failure> for LeaseError {
    fn from(failure: Failure) -> Self {
        Self::Failed(failure)
    }
}
