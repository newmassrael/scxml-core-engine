// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A keyed pool that follows journals (docs/adr/0114, D4; its protocol is
//! docs/formal/EventJournal.tla, `Follow` with `CursorAtomic`).
//!
//! The pool reads the journal of its store ([`Pool::follow_from`]) in the order of its positions,
//! up to the horizon, and hands each entry of a journal its machine follows ([`Follows`]) to the
//! session the entry's key field names, as an external event of the entry's event, whose data is
//! the entry's record read by the record the build described ([`FollowedEntry`]) and whose origin
//! is `journal://<machine>/<key>/<position>`, or `journal://<stream>/<position>` for an observed
//! stream. What decides that each entry takes effect once is the session's, not the reader's:
//!
//! * A session keeps, for each journal it follows, the position of the last entry it took in, in
//!   its checkpoint ([`Checkpoint::followed`](crate::checkpoint::Checkpoint::followed)), written
//!   with what that entry did. An entry at or
//!   below it is a repeat, dropped. A restore reads it back, so whatever the reader hands again
//!   after a crash takes effect once.
//! * The reader hands a session the entries of its key in the order of their positions, so a
//!   session that took an entry took every one of its key below it: an entry handed and lost (its
//!   session dropped before a durable checkpoint held it) is handed again before any later entry
//!   of its key, and none of those is handed to a session the key no longer runs in while it is
//!   out.
//! * The reader's mark, kept in the store under the machine's name
//!   ([`JournalStore::keep_read`]), is the position through which every entry it read was taken
//!   in by a durable checkpoint; a pool that starts reads on from it, and a view rebuilt under a
//!   new name reads from the start. A mark kept late repeats entries, which their sessions drop.
//! * An entry of a key another process holds is that process's to hand: it is taken in once that
//!   session's stored checkpoint holds its position, and handed here once the lease runs out.
//! * A follower that cannot keep up stops the reader: a session's inbox at `sessions.max_inbox`
//!   holds back the later entries of its key, a pool at `sessions.max_live` stops the reading, and
//!   the reader holds at most [`FOLLOW_WINDOW`] entries not yet taken in. Nothing is dropped.
//!
//! An entry that can be handed to no session (its schema no kind the build described, its data no
//! value of its record, its key unreadable) is told to the host ([`PoolError::Unfollowed`]) and
//! passed.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::sync::Weak;
use alloc::vec::Vec;
use std::sync::Arc;

use sce_rust_runtime::{EventMetadata, EventOrigin, EventType};
use sce_rust_session_store::{JournalEntry as StoredEntry, JournalStore, StreamName};

use super::{
    checkpoint, key_in, lock, Fate, Full, Input, LoadError, Pool, PoolError, Receipt,
    SessionFactory, SharedSlot, Slot,
};
use crate::endpoint::{EngineCall, EngineEvent, Environment, Transport};
use crate::peers::{FollowedEntry, Follows, KeyFrom};

/// The most entries one call of [`Pool::follow`] reads from the journal.
pub const FOLLOW_PAGE: usize = 256;

/// The most entries the reader holds read and not yet taken in: a follower that cannot keep up
/// stops the reading here, and nothing is dropped.
pub const FOLLOW_WINDOW: usize = 4_096;

/// The `_event.origintype` of an entry a session is handed: the processor of the journal, the type
/// a fact is said by.
pub const JOURNAL_ORIGIN_TYPE: &str = "sce:journal";

/// An entry of a followed journal, as the session its key names is handed it.
pub(super) struct Followed {
    /// The journal it is of: a machine's name, or an observed stream's.
    pub(super) journal: String,
    pub(super) position: u64,
    /// The event it is delivered as, its data and its origin.
    pub(super) event: String,
    pub(super) data: String,
    pub(super) origin: String,
    /// Settled once a durable checkpoint holds what the session made of it, or that it was a
    /// repeat; lost when the session is dropped before.
    pub(super) receipt: Receipt,
}

impl Followed {
    /// The external event the session is told.
    pub(super) fn call(&self) -> EngineCall {
        EngineCall::Raise(EngineEvent {
            name: self.event.clone(),
            metadata: EventMetadata {
                data: self.data.clone(),
                event_type: EventType::External,
                origin: Some(EventOrigin::new(JOURNAL_ORIGIN_TYPE, self.origin.clone())),
                ..EventMetadata::default()
            },
        })
    }
}

/// An entry the reader read and has not seen taken in.
struct Open<M, T, E> {
    /// The key of the session it is for.
    key: String,
    journal: String,
    event: String,
    data: String,
    origin: String,
    /// Its last hand-out; `None` while it is to be handed out.
    handed: Option<Handed<M, T, E>>,
}

/// The last hand-out of an open entry.
struct Handed<M, T, E> {
    /// What tells whether the session took the entry in.
    receipt: Receipt,
    /// The slot the entry was put in.
    slot: Weak<Slot<M, T, E>>,
}

/// What a pool's reader of the journal holds between its calls.
pub(super) struct Reader<M, T, E> {
    /// Whether the mark the store keeps was read.
    started: bool,
    /// The position of the last entry read.
    read: u64,
    /// The mark the store keeps of the reader.
    kept: u64,
    /// The entries read and not yet taken in, by position.
    open: BTreeMap<u64, Open<M, T, E>>,
}

impl<M, T, E> Reader<M, T, E> {
    pub(super) fn new() -> Self {
        Self {
            started: false,
            read: 0,
            kept: 0,
            open: BTreeMap::new(),
        }
    }

    /// The position through which every entry read was taken in.
    fn mark(&self) -> u64 {
        self.open
            .keys()
            .next()
            .map_or(self.read, |first| first.saturating_sub(1))
    }
}

/// What one call of [`Pool::follow`] decided so far.
struct Pass<M, T, E> {
    /// The keys of the sessions it handed an entry to, which the host runs.
    run: BTreeSet<String>,
    /// The keys whose later entries wait behind one not handed now.
    held: BTreeSet<String>,
    /// For each key with an entry out, the slot its last one went to: a later entry of the key
    /// goes into that slot or waits.
    out: BTreeMap<String, Weak<Slot<M, T, E>>>,
    /// The pool holds its most sessions, and none can be dropped to make room.
    full: bool,
}

impl<M, T, E> Pass<M, T, E> {
    fn new() -> Self {
        Self {
            run: BTreeSet::new(),
            held: BTreeSet::new(),
            out: BTreeMap::new(),
            full: false,
        }
    }
}

/// Why an entry was not put in the inbox of its session.
enum Unhanded {
    /// A bound of what the pool holds in memory.
    Full(Full),
    /// An earlier entry of its key is out in a slot the key's session is no longer in: the
    /// entry waits until that one is settled or lost.
    Moved,
}

/// `key` as one segment of a URI path (RFC 3986, section 3.3): each byte that is not unreserved,
/// percent-encoded, so that an origin names its key whatever the key holds.
fn segment(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for byte in key.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&alloc::format!("%{byte:02X}"));
        }
    }
    out
}

/// The journal `entry` is of, and the origin a session is told it by (D4).
fn journal_and_origin(entry: &StoredEntry) -> (&str, String) {
    match &entry.stream {
        StreamName::Session { machine, key } => (
            machine.as_str(),
            alloc::format!("journal://{machine}/{}/{}", segment(key), entry.position),
        ),
        StreamName::Observed { name } => (
            name.as_str(),
            alloc::format!("journal://{name}/{}", entry.position),
        ),
    }
}

impl<F, T, E> Pool<F, T, E>
where
    F: SessionFactory<T, E>,
    T: Transport + 'static,
    E: Environment + Clone + 'static,
{
    /// This pool, reading the journals its machine follows (deploy.yaml's `follows:`) from
    /// `journal`, the store that keeps them: the one the followed machines' pools write, or a
    /// handle on it. The host then calls [`Self::follow`] on its schedule.
    pub fn follow_from(mut self, journal: Arc<dyn JournalStore>) -> Self {
        self.journal = Some(journal);
        self
    }

    /// Read on in the journals the machine follows and hand each entry to the session its key
    /// names (the module documentation says what makes each entry take effect once): first what
    /// was handed and lost, then up to [`FOLLOW_PAGE`] entries past the last one
    /// read, and keep the reader's mark in the store once it rose. The keys answered are the
    /// sessions to run. A pool given no journal, or that follows none, or that is stopping, reads
    /// nothing.
    pub fn follow(&self) -> Vec<String> {
        let Some(journal) = self.journal.clone() else {
            return Vec::new();
        };
        let follows = self.sessions.follows;
        if follows.journals.is_empty() || self.stopping.load(core::sync::atomic::Ordering::SeqCst) {
            return Vec::new();
        }
        let mut reader = lock(&self.reader);
        if !reader.started {
            match journal.read_through(self.machine.name) {
                Ok(mark) => {
                    reader.read = mark;
                    reader.kept = mark;
                    reader.started = true;
                }
                Err(failure) => {
                    self.error(PoolError::Store { key: None, failure });
                    return Vec::new();
                }
            }
        }
        // What the sessions took in since the last call is passed; what they lost is to be
        // handed again.
        reader.open.retain(|_, open| {
            match open.handed.as_ref().map(|handed| handed.receipt.fate()) {
                Some(Fate::Settled) => return false,
                // Fenced, like lost, took nothing of the entry in; an entry of a journal is never
                // an instance's, so the fence does not reach it, and it is handed again either way.
                Some(Fate::Lost | Fate::Fenced { .. }) => open.handed = None,
                Some(Fate::Pending) | None => {}
            }
            true
        });
        let mut pass = Pass::new();
        let waiting: Vec<u64> = reader.open.keys().copied().collect();
        for position in waiting {
            if pass.full {
                break;
            }
            self.hand(&mut reader, position, &mut pass);
        }
        if !pass.full && reader.open.len() < FOLLOW_WINDOW {
            match journal.after(reader.read, FOLLOW_PAGE) {
                Ok(page) => {
                    for entry in page.entries {
                        if pass.full || reader.open.len() >= FOLLOW_WINDOW {
                            break;
                        }
                        reader.read = entry.position;
                        if let Some(open) = self.opened(&follows, &entry) {
                            reader.open.insert(entry.position, open);
                            self.hand(&mut reader, entry.position, &mut pass);
                        }
                    }
                }
                Err(failure) => self.error(PoolError::Store { key: None, failure }),
            }
        }
        let mark = reader.mark();
        if mark > reader.kept {
            match journal.keep_read(self.machine.name, mark) {
                Ok(kept) => reader.kept = kept,
                Err(failure) => self.error(PoolError::Store { key: None, failure }),
            }
        }
        pass.run.into_iter().collect()
    }

    /// The entry `entry` of the journal, as the reader holds it until it is taken in: `None` for
    /// one of a journal the machine does not follow, and for one no session can be handed, which
    /// is told to the host.
    fn opened(&self, follows: &Follows, entry: &StoredEntry) -> Option<Open<F::Machine, T, E>> {
        let (journal, origin) = journal_and_origin(entry);
        if !follows.follows(journal) {
            return None;
        }
        let unfollowed = |reason: String| {
            self.error(PoolError::Unfollowed {
                journal: journal.to_string(),
                position: entry.position,
                reason,
            });
            None
        };
        let Some(&FollowedEntry { event, record }) = follows.entry(&entry.schema) else {
            return unfollowed(alloc::format!(
                "its schema '{}' is of no entry the machine follows",
                entry.schema
            ));
        };
        let data = match sce_rust_runtime::record_cbor::decode(&entry.data, record) {
            Ok(data) => data,
            Err(refusal) => {
                return unfollowed(alloc::format!(
                    "its data is no record of '{event}': '{}' {}",
                    refusal.pointer,
                    refusal.reason
                ))
            }
        };
        let key = match self.sessions.key_from {
            KeyFrom::Field(field) => key_in(&data, field).and_then(|key| self.bounded_key(key)),
            KeyFrom::Client => Err("the pool is keyed by its clients' names".to_string()),
        };
        let key = match key {
            Ok(key) => key,
            Err(reason) => return unfollowed(alloc::format!("it names no session: {reason}")),
        };
        Some(Open {
            key,
            journal: journal.to_string(),
            event: event.to_string(),
            data,
            origin,
            handed: None,
        })
    }

    /// Hand the entry at `position` to the session of its key, unless an earlier entry of the key
    /// waits, it is out already, its key is held by another process, or there is no room: in
    /// the order of the positions of each key, as the module says.
    fn hand(
        &self,
        reader: &mut Reader<F::Machine, T, E>,
        position: u64,
        pass: &mut Pass<F::Machine, T, E>,
    ) {
        let Some(open) = reader.open.get(&position) else {
            return;
        };
        let key = open.key.clone();
        if let Some(Handed { slot, .. }) = &open.handed {
            // Out: the key's later entries follow it into its slot, or wait until its receipt is
            // settled or lost.
            pass.out.insert(key, Weak::clone(slot));
            return;
        }
        if pass.held.contains(&key) {
            return;
        }
        if self.held_elsewhere(&key) {
            // The other process hands it; it is taken in once that session's stored checkpoint
            // holds its position.
            if self.stored_position(&key, &open.journal) >= position {
                reader.open.remove(&position);
            } else {
                pass.held.insert(key);
            }
            return;
        }
        let receipt = Receipt::new(Fate::Pending);
        let input = Input::Followed(Followed {
            journal: open.journal.clone(),
            position,
            event: open.event.clone(),
            data: open.data.clone(),
            origin: open.origin.clone(),
            receipt: receipt.clone(),
        });
        match self.post_followed(&key, input, pass.out.get(&key)) {
            Ok(slot) => {
                if let Some(open) = reader.open.get_mut(&position) {
                    open.handed = Some(Handed {
                        receipt,
                        slot: Arc::downgrade(&slot),
                    });
                }
                pass.out.insert(key.clone(), Arc::downgrade(&slot));
                pass.run.insert(key);
            }
            Err(Unhanded::Moved | Unhanded::Full(Full::Inbox)) => {
                pass.held.insert(key);
            }
            Err(Unhanded::Full(Full::Live | Full::Stopping)) => pass.full = true,
        }
    }

    /// Put `input`, an entry of a followed journal, in the inbox of the session `key` within the
    /// pool's bounds, making room as an envelope's post does, and answer the slot it was put in:
    /// when `after` is given, the slot an earlier entry of the key is out in, only if the key's
    /// session is still in it, judged under the one lock of the table the post is made under, so
    /// that no entry of a key reaches a session before an earlier one.
    fn post_followed(
        &self,
        key: &str,
        input: Input,
        after: Option<&Weak<Slot<F::Machine, T, E>>>,
    ) -> Result<SharedSlot<F::Machine, T, E>, Unhanded> {
        let mut input = input;
        let mut room_made = false;
        loop {
            if self.stopping.load(core::sync::atomic::Ordering::SeqCst) {
                return Err(Unhanded::Full(Full::Stopping));
            }
            let mut table = lock(&self.table);
            if let Some(after) = after {
                let same = after
                    .upgrade()
                    .zip(table.get(key))
                    .is_some_and(|(handed, current)| Arc::ptr_eq(&handed, current));
                if !same {
                    return Err(Unhanded::Moved);
                }
            }
            match self.post_in(&mut table, key, input) {
                Ok(slot) => return Ok(slot),
                Err((back, Full::Live)) if !room_made => {
                    drop(table);
                    if !self.make_room() {
                        return Err(Unhanded::Full(Full::Live));
                    }
                    room_made = true;
                    input = back;
                }
                Err((_, full)) => return Err(Unhanded::Full(full)),
            }
        }
    }

    /// Whether another process holds the lease on `key` now.
    fn held_elsewhere(&self, key: &str) -> bool {
        let now = self.wall_now();
        lock(&self.deferred)
            .get(&self.store.indexed(key))
            .is_some_and(|until| *until > now)
    }

    /// The position of the last entry of `journal` the stored checkpoint of `key` took in; `0`
    /// when the store holds none, or one this core cannot read.
    fn stored_position(&self, key: &str, journal: &str) -> u64 {
        match checkpoint::load(&*self.store, key) {
            Ok(loaded) => loaded
                .and_then(|loaded| loaded.checkpoint.followed.get(journal).copied())
                .unwrap_or(0),
            Err(LoadError::Store(failure)) => {
                self.error(PoolError::Store {
                    key: Some(key.to_string()),
                    failure,
                });
                0
            }
            Err(unreadable @ LoadError::Unreadable { .. }) => {
                self.error(PoolError::NotBuilt {
                    key: key.to_string(),
                    reason: unreadable.to_string(),
                });
                0
            }
        }
    }
}
