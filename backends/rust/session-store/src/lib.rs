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
//! The contract keeps, for each key:
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
//! earliest due, after the lease check, or writes nothing.
//!
//! * [`MemoryStore`] keeps it all in the process, for the tests of the crates that are given a
//!   store.
//! * `SqliteStore` (feature `sqlite`, on by default) is the reference store: SQLite in WAL mode
//!   with `synchronous=FULL`, so a put that returned survives a power loss.
//! * [`conformance`] is what every implementation is held to: both of these run it, and a store
//!   written elsewhere runs the same functions.
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

pub mod conformance;
mod memory;
mod rules;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use memory::MemoryStore;
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
    /// The session `key` names, or `None` if it holds none.
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure>;

    /// Keep `state` as the session `key` names, with `earliest_due` as the earliest moment one of
    /// its waiting sends comes due (`None` when none waits), and answer its new version.
    ///
    /// Refused, writing nothing, unless `fence` is the fence of the lease currently held on `key`
    /// ([`Conflict::Fence`], checked first: a fenced-out writer's view of the version says nothing)
    /// and the session's version is `expected_version`, `0` meaning it holds no session
    /// ([`Conflict::Version`]). The lease's expiry is not checked: while a lease that ran out is not
    /// taken over, no one else can write the key and its holder still may; the moment another
    /// process takes it, the new fence refuses the old holder's writes.
    fn put(
        &self,
        key: &str,
        state: &[u8],
        earliest_due: Option<u64>,
        expected_version: u64,
        fence: u64,
    ) -> Result<u64, WriteError>;

    /// Remove the session `key` names, under the same two checks as [`SessionStore::put`]; with
    /// `expected_version` `0` and no session held, there is nothing to remove and it succeeds. The
    /// lease is not touched: it is the holder's to release.
    fn delete(&self, key: &str, expected_version: u64, fence: u64) -> Result<(), WriteError>;

    /// At most `limit` sessions whose earliest due is at or before `ms`, earliest first, sessions
    /// due at the same moment in the order of their keys. At or before, because a send due at the
    /// moment a restore runs is armed as due now, as `sce_rust_runtime::saved_state` arms it. The
    /// host core keeps a timer on the first of `due_before(LATEST_MOMENT, 1)` and loads what this
    /// lists when it fires.
    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure>;

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

/// Why a store refused [`SessionStore::put`] or [`SessionStore::delete`].
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
