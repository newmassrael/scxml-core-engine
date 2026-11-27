// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every implementation of [`JournalStore`] is held to (ADR 0114; module
//! [`journal`](crate::journal)).
//!
//! The contract is the documentation of [`JournalStore`] and of [`Write::journal`]; these functions
//! are that documentation as histories with the answers a store must give, and the properties of
//! docs/formal/EventJournal.tla among them. The two stores of this crate run them
//! (`tests/every_journal_store_keeps_one_contract.rs`), a sealing store runs them over both, and a
//! store written elsewhere runs the same functions from its own `#[test]`s, as with
//! [`crate::conformance`].
//!
//! * [`an_entry_is_made_by_its_sessions_write_or_not_at_all`]: a write's facts are kept with it,
//!   in order, and a write the store refuses leaves none of them and no position a reader sees.
//! * [`a_batch_appends_item_by_item`]: a batch appends the facts of each write it keeps, and none
//!   of a write it refuses.
//! * [`a_removed_session_keeps_its_journal`]: a removal appends what the session made last and
//!   removes nothing of its stream, which a session made again under the key goes on.
//! * [`an_input_makes_one_entry_and_a_reader_misses_none`] (`AppendOnce`, `NothingLost`,
//!   `FollowedOnce`, `NonePassed`): writers that contend for many sessions make one entry for each
//!   input, and a reader beside them reads each once, in order.
//! * [`a_reader_waits_for_a_lower_number_still_open`] (`NonePassed`, `Stable`): a transaction that
//!   took its position and commits after one of a higher position is not passed, for a store that
//!   can be made to do it ([`JournalHarness::begin`]).
//! * [`a_page_is_resumed_where_it_ended`]: reads are pages, resumed from the last entry of the one
//!   before.
//! * [`a_moment_is_never_below_the_one_before_it`]: the recorded moment does not go back when the
//!   clock does, and a moment is turned into the largest position at or before it.
//! * [`an_answer_as_of_a_position_does_not_change`] (`Stable`): an answer is ordered by valid time
//!   and then position, is the same after later writes, and is refused above the horizon.
//! * [`an_observed_entry_is_kept_once_for_its_origin`]: one entry for a stream and an origin id,
//!   whoever asks and however often.
//! * [`a_session_and_an_observed_stream_share_no_name`]: a session's write appends to no observed
//!   stream, nor an observation to a machine's.
//! * [`a_write_stopped_partway_leaves_its_state_and_its_entries_or_neither`] (`NothingLost`): a
//!   write that fails at any point leaves neither its state nor an entry, for every handle opened
//!   after.
//! * [`a_fact_is_kept_with_the_bytes_made_for_its_position`]: bytes made at an entry's position are
//!   the bytes it keeps.
//! * [`data_is_replaced_only_from_what_it_holds`] (ADR 0109, D8): an entry's data is replaced only
//!   from the bytes it holds, every other field kept.
//! * [`handles_share_the_journal`]: what one handle appended, another reads.
//! * [`a_reader_keeps_how_far_it_read`] (`NonePassed`, D4): a reader's mark is kept by its name,
//!   only rises, and is refused above the horizon, for a position still open as well.
//!
//! Each function needs a harness whose store holds nothing when it starts. The clock is the one
//! the functions give; nothing here waits on time. A failed expectation panics, as a test's does.
//! The functions are themselves held to a store that keeps the contract out of the order of its
//! numbers and to stores that break it, which they must refuse (this module's own tests).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Barrier, Mutex, PoisonError};
use std::thread;

use crate::batch::{Batched, Committed, Decided};
use crate::journal::{
    AsOf, AsOfCursor, AsOfError, AtPosition, Fact, FactData, Journal, JournalEntry, JournalPage,
    JournalStore, Observation, PastHorizon, Stream, StreamName,
};
use crate::outbox::{Append, Write, Written};
use crate::{Conflict, Failure, Lease, SessionStore, Stored, WriteError, LATEST_MOMENT};

/// What commits a transaction that began: see [`JournalHarness::begin`].
pub type Commit<'a> = Box<dyn FnOnce() -> Result<Written, WriteError> + 'a>;

/// A point of a write at which a harness can make its store fail: see
/// [`JournalHarness::faults`].
pub struct Fault<'a> {
    /// Where, for the message of a failed expectation.
    pub at: &'static str,
    /// Make every write that appends fail there from here on.
    pub set: Box<dyn Fn() + 'a>,
    /// Stop.
    pub clear: Box<dyn Fn() + 'a>,
}

/// What gives the functions of this module their store.
pub trait JournalHarness {
    type Store: JournalStore;

    /// Another handle on the one store this harness holds: it sees everything every other handle
    /// wrote. For a store in a database, a connection of its own, as a second process would have.
    fn handle(&self) -> Self::Store;

    /// Begin the transaction of `write` to the session `key` names, up to where its facts have
    /// taken their positions, and answer what commits it: between the two, other writes of the
    /// store are made and commit, with higher positions. What is answered and dropped without
    /// being called is a writer that is gone: the store ends its transaction, as a database ends
    /// the one of a connection that closed.
    ///
    /// A store that gives positions as a transaction begins, as a sequence of a database server
    /// does, answers it, so that the kit commits two transactions out of the order of their
    /// numbers. `None`, as it stands, is "not applicable": the store writes one transaction at a
    /// time, so a second cannot begin while the first is open, and its numbers are committed in
    /// their order.
    fn begin<'a>(&'a self, key: &'a str, write: Write<'a>) -> Option<Commit<'a>> {
        let _ = (key, write);
        None
    }

    /// The points inside a write at which this harness can make its store fail, as a medium that
    /// fails there would: for a store in a database, one for each row a write writes. None, as it
    /// stands, beside the failure every store is put through (a fact whose bytes cannot be made).
    fn faults(&self) -> Vec<Fault<'_>> {
        Vec::new()
    }
}

/// Whether a harness could force the history a function needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forced {
    Yes,
    /// The store writes one transaction at a time ([`JournalHarness::begin`]).
    NotApplicable,
}

/// The machine whose sessions the functions write.
const MACHINE: &str = "ledger";
/// The schema of the facts they make.
const SCHEMA: &str = "entry.recorded";

/// A fact of `data` with no valid time.
fn fact(data: &[u8]) -> Fact<'_> {
    Fact {
        schema: SCHEMA,
        revision: 1,
        data: FactData::Bytes(data),
        valid_ms: None,
    }
}

/// A fact of `data` that holds from `valid_ms`.
fn dated(data: &[u8], valid_ms: i64) -> Fact<'_> {
    Fact {
        valid_ms: Some(valid_ms),
        ..fact(data)
    }
}

/// The facts of a write of a session of [`MACHINE`].
fn journal<'a>(facts: &'a [Fact<'a>]) -> Option<Journal<'a>> {
    Some(Journal {
        machine: MACHINE,
        facts,
    })
}

/// The stream of the session `key` of [`MACHINE`].
fn of_session(key: &str) -> Stream<'_> {
    Stream::Session {
        machine: MACHINE,
        key,
    }
}

/// A write that keeps `state` from `version` and appends `facts` at `now_ms`.
fn keeps<'a>(
    state: &'a [u8],
    version: u64,
    fence: u64,
    facts: &'a [Fact<'a>],
    now_ms: u64,
) -> Write<'a> {
    Write {
        journal: journal(facts),
        now_ms,
        ..Write::put(state, None, version, fence)
    }
}

fn lease(store: &impl JournalStore, key: &str) -> Lease {
    store
        .acquire(key, "a", LATEST_MOMENT, 0)
        .expect("a lease on a key no one else holds")
}

/// What `page`, a read past `after` of at most `limit` entries, holds, held to what every such read
/// answers: entries whose positions rise, each past `after` and none above the horizon.
fn checked(page: JournalPage, after: u64, limit: usize) -> JournalPage {
    let mut last = after;
    for entry in &page.entries {
        assert!(
            entry.position > last,
            "position {} after {last}",
            entry.position
        );
        assert!(
            entry.position <= page.horizon,
            "position {} above the horizon, {}",
            entry.position,
            page.horizon
        );
        last = entry.position;
    }
    assert!(page.entries.len() <= limit, "a page past its limit");
    page
}

/// A page of the journal past `after`.
fn page(store: &impl JournalStore, after: u64, limit: usize) -> JournalPage {
    let read = store
        .after(after, limit)
        .expect("a read the store could not make");
    checked(read, after, limit)
}

/// Every entry of the journal a reader may read.
fn all(store: &impl JournalStore) -> Vec<JournalEntry> {
    page(store, 0, usize::MAX).entries
}

/// Every entry of `stream` a reader may read.
fn all_of(store: &impl JournalStore, stream: Stream<'_>) -> Vec<JournalEntry> {
    let read = store
        .stream(stream, 0, usize::MAX)
        .expect("a read the store could not make");
    let entries = checked(read, 0, usize::MAX).entries;
    let name = stream.to_name();
    assert!(
        entries.iter().all(|entry| entry.stream == name),
        "an entry of another stream: {entries:?}"
    );
    entries
}

/// The data of `entries`, as text.
fn texts(entries: &[JournalEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| String::from_utf8_lossy(&entry.data).into_owned())
        .collect()
}

/// The last position a reader may read, `0` for an empty journal.
fn last_position(store: &impl JournalStore) -> u64 {
    all(store).last().map_or(0, |entry| entry.position)
}

/// The answer as of `position` and `valid_ms` of `stream`, whole.
fn answer(
    store: &impl JournalStore,
    stream: Stream<'_>,
    position: u64,
    valid_ms: i64,
) -> Vec<JournalEntry> {
    store
        .as_of(
            stream,
            &AsOf { position, valid_ms },
            AsOfCursor::START,
            usize::MAX,
        )
        .expect("an answer as of a position the horizon passed")
        .entries
}

fn read(store: &impl SessionStore, key: &str) -> Option<Stored> {
    store.get(key).expect("a read the store could not make")
}

fn conflict<T: std::fmt::Debug>(result: Result<T, WriteError>) -> Conflict {
    match result {
        Err(WriteError::Conflict(conflict)) => conflict,
        other => panic!("a write that should have been refused gave {other:?}"),
    }
}

/// A fact whose bytes cannot be made, as a fact sealed by a store that lost its keys cannot.
struct CannotBeMade;

impl AtPosition for CannotBeMade {
    fn bytes(&self, _: u64) -> Result<Vec<u8>, Failure> {
        Err(Failure::new("the bytes cannot be made"))
    }
}

/// A fact whose bytes say the position they were made for, and which keeps what it was asked.
struct Stamped(Mutex<Vec<u64>>);

impl AtPosition for Stamped {
    fn bytes(&self, position: u64) -> Result<Vec<u8>, Failure> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(position);
        Ok(format!("made at {position}").into_bytes())
    }
}

/// The facts of a write are kept with it, in their order, each as it was given and at the write's
/// moment; a write the store refuses, for its version or for its fence, leaves none of them, and
/// no position a reader sees: the next entry is above every one before it, and the readers' order
/// holds only what was kept. A write that gives no fact appends nothing.
pub fn an_entry_is_made_by_its_sessions_write_or_not_at_all(harness: &impl JournalHarness) {
    let store = harness.handle();
    assert!(all(&store).is_empty(), "a journal nothing was appended to");
    let held = lease(&store, "k");
    let facts = [
        fact(b"opened"),
        Fact {
            schema: "entry.dated",
            revision: 3,
            ..dated(b"dated", -5)
        },
    ];
    let first = store
        .write("k", &keeps(b"one", 0, held.fence, &facts, 1_000))
        .expect("a first write that appends");
    let entries = all(&store);
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert!(
        entries[0].position >= 1,
        "a position is never 0: {entries:?}"
    );
    let kept = |position: u64, schema: &str, revision: u32, valid_ms: Option<i64>, data: &[u8]| {
        JournalEntry {
            position,
            recorded_ms: 1_000,
            stream: of_session("k").to_name(),
            origin: None,
            schema: schema.to_string(),
            revision,
            valid_ms,
            data: data.to_vec(),
        }
    };
    assert_eq!(
        entries,
        vec![
            kept(entries[0].position, SCHEMA, 1, None, b"opened"),
            kept(entries[1].position, "entry.dated", 3, Some(-5), b"dated"),
        ],
        "each fact as it was given, in the order given"
    );
    assert_eq!(all_of(&store, of_session("k")), entries);

    let refused = [fact(b"refused")];
    assert_eq!(
        conflict(store.write("k", &keeps(b"stale", 0, held.fence, &refused, 1_100))),
        Conflict::Version {
            expected: 0,
            current: first.version
        }
    );
    let taken = lease(&store, "k");
    assert_eq!(
        conflict(store.write(
            "k",
            &keeps(b"fenced", first.version, held.fence, &refused, 1_100)
        )),
        Conflict::Fence {
            presented: held.fence,
            current: Some(taken.fence)
        }
    );
    assert_eq!(
        conflict(store.write(
            "other",
            &keeps(b"unleased", 0, taken.fence, &refused, 1_100)
        )),
        Conflict::Fence {
            presented: taken.fence,
            current: None
        }
    );
    assert_eq!(
        all(&harness.handle()),
        entries,
        "a refused write left an entry"
    );

    let none = store
        .write(
            "k",
            &keeps(b"no fact", first.version, taken.fence, &[], 1_200),
        )
        .expect("a write that gives no fact");
    store
        .put("k", b"no journal", None, none.version, taken.fence)
        .expect("a write that says nothing of the journal");
    assert_eq!(all(&store), entries, "a write of no fact appended one");

    let other = lease(&store, "other");
    let next = [fact(b"of another")];
    store
        .write("other", &keeps(b"other", 0, other.fence, &next, 1_300))
        .expect("a write of another session");
    let entries = all(&store);
    assert_eq!(texts(&entries), ["opened", "dated", "of another"]);
    assert_eq!(
        entries[2].stream,
        StreamName::Session {
            machine: MACHINE.to_string(),
            key: "other".to_string()
        }
    );
    assert_eq!(texts(&all_of(&store, of_session("k"))), ["opened", "dated"]);
    assert_eq!(texts(&all_of(&store, of_session("other"))), ["of another"]);
    assert!(all_of(&store, of_session("nobody")).is_empty());
}

/// A batch appends the facts of each write it keeps, in the batch's order, and none of a write it
/// refuses or that fails alone; a key written twice in one batch makes the entries of both.
pub fn a_batch_appends_item_by_item(harness: &impl JournalHarness) {
    let store = harness.handle();
    let kept = lease(&store, "kept");
    let fenced = lease(&store, "fenced");
    let taken = lease(&store, "fenced");
    let failed = lease(&store, "failed");
    let last = lease(&store, "last");
    let (of_kept, of_fenced, of_last) = (
        [fact(b"kept 1"), fact(b"kept 2")],
        [fact(b"fenced")],
        [fact(b"last")],
    );
    let of_failed = [
        fact(b"failed 1"),
        Fact {
            data: FactData::AtPosition(&CannotBeMade),
            ..fact(b"")
        },
    ];
    let decided = store
        .write_many(&[
            Batched::Write {
                key: "kept",
                write: keeps(b"kept", 0, kept.fence, &of_kept, 2_000),
            },
            Batched::Write {
                key: "fenced",
                write: keeps(b"fenced", 0, fenced.fence, &of_fenced, 2_000),
            },
            Batched::Write {
                key: "failed",
                write: keeps(b"failed", 0, failed.fence, &of_failed, 2_000),
            },
            Batched::Write {
                key: "last",
                write: keeps(b"last", 0, last.fence, &of_last, 2_000),
            },
        ])
        .expect("a batch the store could make");
    assert_eq!(decided.len(), 4, "{decided:?}");
    assert_eq!(
        conflict(decided_write(&decided[1])),
        Conflict::Fence {
            presented: fenced.fence,
            current: Some(taken.fence)
        }
    );
    assert!(
        matches!(decided[2], Err(WriteError::Failed(_))),
        "an item whose fact could not be made gave {:?}",
        decided[2]
    );
    decided_write(&decided[0]).expect("an item before a refused one is kept");
    decided_write(&decided[3]).expect("an item after a failed one is kept");
    let reader = harness.handle();
    assert_eq!(
        texts(&all(&reader)),
        ["kept 1", "kept 2", "last"],
        "the entries of what the batch kept, in its order, and no other"
    );
    assert_eq!(read(&reader, "fenced"), None);
    assert_eq!(read(&reader, "failed"), None);
}

/// What a batch decided of a write, its failure told as a refusal of nothing it names.
fn decided_write(decided: &Decided) -> Result<Written, WriteError> {
    match decided {
        Ok(Committed::Written(written)) => Ok(written.clone()),
        Ok(Committed::Acknowledged(_)) => panic!("a write answered as an acknowledgement"),
        Err(WriteError::Conflict(conflict)) => Err(WriteError::Conflict(*conflict)),
        Err(WriteError::Failed(failure)) => {
            Err(WriteError::Failed(Failure::new(failure.to_string())))
        }
    }
}

/// ADR 0114, D8: a removal appends what the session made last, in its own transaction, and removes
/// nothing of its stream; a session made again under the key goes on in the same stream; and what
/// the journal held before stays as it was.
pub fn a_removed_session_keeps_its_journal(harness: &impl JournalHarness) {
    let store = harness.handle();
    let held = lease(&store, "k");
    let opened = [fact(b"opened")];
    let version = store
        .write("k", &keeps(b"open", 0, held.fence, &opened, 1_000))
        .expect("a session")
        .version;
    let before = all(&store);
    let ended = [fact(b"ended")];
    store
        .write(
            "k",
            &Write {
                journal: journal(&ended),
                now_ms: 2_000,
                sessions: &["root-1"],
                ..Write::delete(version, held.fence)
            },
        )
        .expect("a removal that appends");
    assert_eq!(read(&store, "k"), None);
    let after = all_of(&store, of_session("k"));
    assert_eq!(texts(&after), ["opened", "ended"]);
    assert_eq!(after[..1], before, "an entry changed with its session");

    let again = [fact(b"opened again")];
    store
        .write("k", &keeps(b"open again", 0, held.fence, &again, 3_000))
        .expect("a session made again under the key");
    assert_eq!(
        texts(&all_of(&harness.handle(), of_session("k"))),
        ["opened", "ended", "opened again"]
    );
}

/// Writers on threads, each with a handle of its own, take the next input of each of many sessions
/// and write it from the version they read, again when another wrote first: each input makes one
/// entry, whoever won it (`AppendOnce`), and each input a state holds has its entry
/// (`NothingLost`). A reader beside them, a page at a time from its cursor, has read every entry,
/// once and in order, when the writers are done (`FollowedOnce`, `NonePassed`).
pub fn an_input_makes_one_entry_and_a_reader_misses_none(harness: &impl JournalHarness) {
    const WRITERS: usize = 4;
    const SESSIONS: [&str; 3] = ["s0", "s1", "s2"];
    const INPUTS: u64 = 8;
    let store = harness.handle();
    let leases: Vec<Lease> = SESSIONS.iter().map(|key| lease(&store, key)).collect();
    let handles: Vec<_> = (0..WRITERS).map(|_| harness.handle()).collect();
    let reader = harness.handle();
    let start = Barrier::new(WRITERS + 1);
    let done = AtomicBool::new(false);

    let followed: Vec<JournalEntry> = thread::scope(|scope| {
        let writers: Vec<_> = handles
            .iter()
            .map(|handle| {
                let (start, leases) = (&start, &leases);
                scope.spawn(move || {
                    start.wait();
                    for (key, lease) in SESSIONS.iter().zip(leases) {
                        take_inputs(handle, key, lease.fence, INPUTS);
                    }
                })
            })
            .collect();
        let follower = {
            let (start, done, reader) = (&start, &done, &reader);
            scope.spawn(move || {
                let mut taken: Vec<JournalEntry> = Vec::new();
                start.wait();
                loop {
                    // Asked before the page is read, so that a page read after it is whole.
                    let last = done.load(Ordering::SeqCst);
                    let cursor = taken.last().map_or(0, |entry| entry.position);
                    let read = page(reader, cursor, 5).entries;
                    if read.is_empty() && last {
                        return taken;
                    }
                    taken.extend(read);
                    thread::yield_now();
                }
            })
        };
        let ended: Vec<_> = writers.into_iter().map(|writer| writer.join()).collect();
        // Told whatever a writer did, so that the follower ends before its failure is told.
        done.store(true, Ordering::SeqCst);
        let followed = follower.join().expect("the reader panicked");
        for writer in ended {
            writer.expect("a writer panicked");
        }
        followed
    });

    let entries = all(&store);
    assert_eq!(
        followed, entries,
        "the reader beside the writers missed an entry or read one twice"
    );
    assert_eq!(
        entries.len() as u64,
        SESSIONS.len() as u64 * INPUTS,
        "an input made another number of entries than one"
    );
    for key in SESSIONS {
        assert_eq!(
            read(&store, key).map(|stored| stored.state),
            Some(INPUTS.to_string().into_bytes()),
            "the state of '{key}'"
        );
        let expected: Vec<String> = (1..=INPUTS).map(|n| format!("{key} input {n}")).collect();
        assert_eq!(
            texts(&all_of(&store, of_session(key))),
            expected,
            "each input of '{key}' once, in the order its states were written"
        );
    }
}

/// Write the inputs of the session `key` up to the `inputs`th, each from the version read, reading
/// again whenever another writer took it first.
fn take_inputs(store: &impl JournalStore, key: &str, fence: u64, inputs: u64) {
    loop {
        let stored = read(store, key);
        let (count, version) = stored.map_or((0, 0), |stored| {
            let count: u64 = String::from_utf8(stored.state)
                .expect("a count is a text")
                .parse()
                .expect("a count is a number");
            (count, stored.version)
        });
        if count >= inputs {
            return;
        }
        let state = (count + 1).to_string();
        let data = format!("{key} input {state}");
        let facts = [fact(data.as_bytes())];
        match store.write(key, &keeps(state.as_bytes(), version, fence, &facts, 1_000)) {
            Ok(_) | Err(WriteError::Conflict(Conflict::Version { .. })) => {}
            Err(other) => panic!("an input failed: {other}"),
        }
    }
}

/// ADR 0114, D3: two transactions that took their positions in one order and commit in the other.
/// While the one of the lower position is open, no read answers the higher, though it committed;
/// neither is a moment turned into it, nor an answer given as of it. Once the lower commits, both
/// are read, in the order of their positions, by a reader that went on from its cursor; and an
/// answer given before does not change. A transaction that took a position and is refused leaves a
/// gap the horizon passes, and so does one whose writer is gone before it commits: a store ends the
/// transactions of a writer that is gone. [`Forced::NotApplicable`] for a store that writes one transaction at a
/// time, which is held to [`an_input_makes_one_entry_and_a_reader_misses_none`].
pub fn a_reader_waits_for_a_lower_number_still_open(harness: &impl JournalHarness) -> Forced {
    const NOW: u64 = 5_000;
    let store = harness.handle();
    let (low, high) = (lease(&store, "low"), lease(&store, "high"));
    let first = [fact(b"before both")];
    let before_both = lease(&store, "before");
    store
        .write("before", &keeps(b"b", 0, before_both.fence, &first, NOW))
        .expect("a write before both");
    let begin_at = last_position(&store);

    let of_low = [fact(b"low")];
    let Some(commit_low) = harness.begin("low", keeps(b"low", 0, low.fence, &of_low, NOW)) else {
        return Forced::NotApplicable;
    };
    let of_high = [fact(b"high")];
    store
        .write("high", &keeps(b"high", 0, high.fence, &of_high, NOW))
        .expect("a write made and committed while the other is open");

    let reader = harness.handle();
    let while_open = page(&reader, begin_at, usize::MAX);
    assert!(
        while_open.entries.is_empty(),
        "a read passed a lower position still open: {:?}",
        while_open.entries
    );
    assert!(
        all_of(&reader, of_session("high")).is_empty(),
        "a read of a stream passed a lower position still open"
    );
    let placed = reader.position_at(NOW).expect("the position of a moment");
    assert!(
        placed <= while_open.horizon,
        "a moment was turned into {placed}, above the horizon, {}",
        while_open.horizon
    );
    let above = AsOf {
        position: while_open.horizon + 1,
        valid_ms: i64::MAX,
    };
    match reader.as_of(of_session("high"), &above, AsOfCursor::START, usize::MAX) {
        Err(AsOfError::PastHorizon(refused)) => assert_eq!(
            refused,
            PastHorizon {
                asked: above.position,
                horizon: while_open.horizon
            }
        ),
        other => panic!("an answer was given above the horizon: {other:?}"),
    }
    let given = (
        answer(&reader, of_session("low"), while_open.horizon, i64::MAX),
        answer(&reader, of_session("high"), while_open.horizon, i64::MAX),
    );

    commit_low().expect("the transaction of the lower position commits");
    let after = page(&reader, begin_at, usize::MAX).entries;
    assert_eq!(
        texts(&after),
        ["low", "high"],
        "a reader that waited reads both, the lower position first; another order is a harness \
         that did not begin the transaction it committed"
    );
    assert_eq!(
        given,
        (
            answer(&reader, of_session("low"), while_open.horizon, i64::MAX),
            answer(&reader, of_session("high"), while_open.horizon, i64::MAX),
        ),
        "an answer changed after it was given"
    );

    // A transaction that took its position and is refused at its commit.
    let of_refused = [fact(b"refused")];
    let commit_refused = harness
        .begin("low", keeps(b"stale", 0, low.fence, &of_refused, NOW))
        .expect("a harness that began one transaction begins another");
    let version = read(&store, "high").map_or(0, |stored| stored.version);
    let of_next = [fact(b"next")];
    store
        .write("high", &keeps(b"next", version, high.fence, &of_next, NOW))
        .expect("a write committed while the one to be refused is open");
    assert!(
        matches!(
            commit_refused(),
            Err(WriteError::Conflict(Conflict::Version { .. }))
        ),
        "a transaction made from a version that is not the session's is refused at its commit"
    );
    assert_eq!(
        texts(&page(&reader, begin_at, usize::MAX).entries),
        ["low", "high", "next"],
        "the horizon passes the position a refused transaction took"
    );

    // A transaction whose writer is gone before it commits (EventJournal.tla, `Release`).
    let of_gone = [fact(b"gone")];
    drop(
        harness
            .begin("low", keeps(b"gone", 0, low.fence, &of_gone, NOW))
            .expect("a harness that began one transaction begins another"),
    );
    let version = read(&store, "high").map_or(0, |stored| stored.version);
    let of_last = [fact(b"last")];
    store
        .write("high", &keeps(b"last", version, high.fence, &of_last, NOW))
        .expect("a write committed after the writer of another is gone");
    assert_eq!(
        texts(&page(&reader, begin_at, usize::MAX).entries),
        ["low", "high", "next", "last"],
        "the horizon passes the position of a transaction whose writer is gone"
    );
    Forced::Yes
}

/// A read answers at most its limit, in order, and the next is resumed from the last entry of the
/// one before, entries appended in between included; a limit of none answers the horizon alone.
pub fn a_page_is_resumed_where_it_ended(harness: &impl JournalHarness) {
    let store = harness.handle();
    let (a, b) = (lease(&store, "a"), lease(&store, "b"));
    let of_a = [fact(b"a1"), fact(b"a2"), fact(b"a3")];
    let a_version = store
        .write("a", &keeps(b"a", 0, a.fence, &of_a, 1_000))
        .expect("three entries")
        .version;
    let of_b = [fact(b"b1"), fact(b"b2")];
    store
        .write("b", &keeps(b"b", 0, b.fence, &of_b, 1_000))
        .expect("two entries of another session");

    let none = page(&store, 0, 0);
    assert!(none.entries.is_empty());
    assert!(
        none.horizon >= last_position(&store),
        "the horizon is told with a page of none"
    );
    let first = page(&store, 0, 2).entries;
    assert_eq!(texts(&first), ["a1", "a2"]);
    let more = [fact(b"a4")];
    store
        .write("a", &keeps(b"a again", a_version, a.fence, &more, 1_000))
        .expect("an entry appended between two pages");
    let second = page(&store, first[1].position, 2).entries;
    assert_eq!(texts(&second), ["a3", "b1"]);
    let rest = page(&store, second[1].position, 10).entries;
    assert_eq!(texts(&rest), ["b2", "a4"]);
    assert!(page(&store, rest[1].position, 10).entries.is_empty());
    assert!(page(&store, u64::MAX, 10).entries.is_empty());

    let stream = of_session("a");
    let of_stream = |after: u64, limit: usize| {
        let read = store.stream(stream, after, limit).expect("a page");
        checked(read, after, limit)
    };
    let first = of_stream(0, 3).entries;
    assert_eq!(texts(&first), ["a1", "a2", "a3"]);
    assert_eq!(texts(&of_stream(first[2].position, 3).entries), ["a4"]);
    assert!(of_stream(0, 0).entries.is_empty());
    assert!(of_stream(u64::MAX, 3).entries.is_empty());
}

/// ADR 0114, D5: the recorded moment is the write's, unless the entry before it was recorded later:
/// it never goes back when the clock does, for an observed entry and a session's alike, nor after a
/// handle is opened again. A moment is turned into the largest position recorded at or before it,
/// `0` when none was.
pub fn a_moment_is_never_below_the_one_before_it<H: JournalHarness>(harness: &H) {
    let store = harness.handle();
    let held = lease(&store, "k");
    let mut version = 0;
    let mut write = |store: &H::Store, data: &str, now_ms: u64| {
        let facts = [fact(data.as_bytes())];
        version = store
            .write("k", &keeps(b"s", version, held.fence, &facts, now_ms))
            .expect("a write that appends")
            .version;
    };
    write(&store, "at 1000", 1_000);
    write(&store, "at 3000", 3_000);
    write(&store, "back to 2000", 2_000);
    store
        .observe("bank", "tx-1", &fact(b"back to 500"), 500)
        .expect("an observation after the clock went back");
    let store = harness.handle();
    write(&store, "at 4000", 4_000);
    let entries = all(&store);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.recorded_ms)
            .collect::<Vec<_>>(),
        vec![1_000, 3_000, 3_000, 3_000, 4_000],
        "a recorded moment went back with the clock"
    );
    for (ms, expected) in [
        (0, 0),
        (999, 0),
        (1_000, entries[0].position),
        (2_999, entries[0].position),
        (3_000, entries[3].position),
        (3_999, entries[3].position),
        (4_000, entries[4].position),
        (u64::MAX, entries[4].position),
    ] {
        assert_eq!(
            store.position_at(ms).expect("the position of a moment"),
            expected,
            "the position of {ms}"
        );
    }
    write(&store, "past every moment", u64::MAX);
    assert_eq!(
        all(&store).last().map(|entry| entry.recorded_ms),
        Some(LATEST_MOMENT),
        "a moment past the latest is kept as it"
    );
}

/// ADR 0114, D5: the answer as of a position and a valid time is the entries of the stream at or
/// below both, ordered by valid time, the recorded moment where there is none, and then by
/// position; a page is resumed after its last entry; and the answer is the same after later
/// writes, one that says an earlier valid time included. A position above the horizon is refused.
pub fn an_answer_as_of_a_position_does_not_change(harness: &impl JournalHarness) {
    let store = harness.handle();
    let (k, o) = (lease(&store, "k"), lease(&store, "o"));
    let mut version = 0;
    for (data, now_ms) in [
        (fact(b"undated"), 1_000),
        (dated(b"at 500", 500), 2_000),
        (dated(b"before the epoch", -10), 3_000),
        (dated(b"at 500 again", 500), 4_000),
    ] {
        version = store
            .write("k", &keeps(b"k", version, k.fence, &[data], now_ms))
            .expect("an entry")
            .version;
    }
    let elsewhere = [dated(b"of another stream", 400)];
    store
        .write("o", &keeps(b"o", 0, o.fence, &elsewhere, 4_500))
        .expect("an entry of another stream");
    let stream = of_session("k");
    let positions: Vec<u64> = all_of(&store, stream)
        .iter()
        .map(|entry| entry.position)
        .collect();
    let end = positions[3];
    assert_eq!(
        texts(&answer(&store, stream, end, i64::MAX)),
        ["before the epoch", "at 500", "at 500 again", "undated"],
        "by valid time and then by position"
    );
    assert_eq!(
        texts(&answer(&store, stream, end, 500)),
        ["before the epoch", "at 500", "at 500 again"],
        "a fact that holds from the moment asked holds at it"
    );
    assert_eq!(
        texts(&answer(&store, stream, end, 499)),
        ["before the epoch"]
    );
    assert_eq!(
        texts(&answer(&store, stream, positions[1], 1_000)),
        ["at 500", "undated"],
        "what was recorded later is not of an earlier position"
    );
    assert!(answer(&store, stream, end, -11).is_empty());
    assert!(answer(&store, stream, 0, i64::MAX).is_empty());

    let as_of = AsOf {
        position: end,
        valid_ms: i64::MAX,
    };
    let part = |after: AsOfCursor, limit: usize| {
        store
            .as_of(stream, &as_of, after, limit)
            .expect("a page of an answer")
            .entries
    };
    let whole = part(AsOfCursor::START, usize::MAX);
    assert!(part(AsOfCursor::START, 0).is_empty());
    let first = part(AsOfCursor::START, 2);
    assert_eq!(first[..], whole[..2]);

    // Later writes, one of them a correction that holds from before what was answered, between two
    // pages of one answer.
    let corrected = [dated(b"a correction", 100), fact(b"later")];
    store
        .write("k", &keeps(b"k", version, k.fence, &corrected, 5_000))
        .expect("a correction, which is an entry");
    assert_eq!(
        part(first[1].cursor(), 1)[..],
        whole[2..3],
        "a page is resumed where the one before it ended"
    );
    assert_eq!(part(whole[2].cursor(), 10)[..], whole[3..]);
    assert!(part(whole[3].cursor(), 10).is_empty());
    assert_eq!(
        part(AsOfCursor::START, usize::MAX),
        whole,
        "an answer changed after it was given"
    );
    assert_eq!(
        texts(&answer(&store, stream, last_position(&store), i64::MAX)),
        [
            "before the epoch",
            "a correction",
            "at 500",
            "at 500 again",
            "undated",
            "later"
        ],
        "a later position holds the correction"
    );

    let horizon = page(&store, 0, 0).horizon;
    let past = AsOf {
        position: horizon + 1,
        valid_ms: i64::MAX,
    };
    match store.as_of(stream, &past, AsOfCursor::START, 10) {
        Err(AsOfError::PastHorizon(refused)) => assert_eq!(
            refused,
            PastHorizon {
                asked: horizon + 1,
                horizon
            }
        ),
        other => panic!("an answer was given above the horizon: {other:?}"),
    }
}

/// ADR 0114, D6: an observation is kept once for its stream and origin id. A repeat, with the same
/// data or other, from this handle, another or several at once, answers the position the first was
/// kept at and appends nothing; the same id in another stream is another entry. What was observed
/// is read in the journal's one order, with its origin id.
pub fn an_observed_entry_is_kept_once_for_its_origin<H: JournalHarness>(harness: &H) {
    const OBSERVERS: usize = 4;
    let store = harness.handle();
    let observe = |store: &H::Store, name: &str, origin: &str, data: &[u8]| {
        store
            .observe(name, origin, &fact(data), 1_000)
            .expect("an observation")
    };
    let first = observe(&store, "bank", "tx-1", b"one");
    assert!(matches!(first, Observation::Kept { .. }), "{first:?}");
    let repeated = Observation::Repeated {
        position: first.position(),
    };
    assert_eq!(observe(&store, "bank", "tx-1", b"one"), repeated);
    assert_eq!(
        observe(&harness.handle(), "bank", "tx-1", b"another"),
        repeated,
        "the entry held answers a repeat that says otherwise"
    );
    let elsewhere = observe(&store, "post", "tx-1", b"of the post");
    assert!(
        matches!(elsewhere, Observation::Kept { .. }),
        "an origin id is of its stream: {elsewhere:?}"
    );
    let second = observe(&store, "bank", "tx-2", b"two");
    assert!(second.position() > elsewhere.position());

    let held = lease(&store, "k");
    let beside = [fact(b"of a session")];
    store
        .write("k", &keeps(b"s", 0, held.fence, &beside, 2_000))
        .expect("a session's entry beside them");
    let bank = Stream::Observed { name: "bank" };
    let entries = all_of(&store, bank);
    assert_eq!(texts(&entries), ["one", "two"]);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.origin.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("tx-1"), Some("tx-2")]
    );
    assert_eq!(
        texts(&all(&store)),
        ["one", "of the post", "two", "of a session"],
        "one order of every stream"
    );
    assert_eq!(
        texts(&answer(&store, bank, second.position(), i64::MAX)),
        ["one", "two"],
        "an observed stream is read as of a position as a session's is"
    );

    let handles: Vec<_> = (0..OBSERVERS).map(|_| harness.handle()).collect();
    let start = Barrier::new(OBSERVERS);
    let observed: Vec<Observation> = thread::scope(|scope| {
        let observers: Vec<_> = handles
            .iter()
            .map(|handle| {
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    handle
                        .observe("bank", "tx-3", &fact(b"three"), 3_000)
                        .expect("an observation beside others")
                })
            })
            .collect();
        observers
            .into_iter()
            .map(|observer| observer.join().expect("an observer panicked"))
            .collect()
    });
    let kept = observed
        .iter()
        .filter(|observation| matches!(observation, Observation::Kept { .. }))
        .count();
    assert_eq!(kept, 1, "{observed:?}");
    assert!(
        observed
            .iter()
            .all(|observation| observation.position() == observed[0].position()),
        "{observed:?}"
    );
    assert_eq!(texts(&all_of(&store, bank)), ["one", "two", "three"]);
}

/// ADR 0114, D6: no session appends to an observed stream. A write that would append under the name
/// of one is refused, its state with it, and an observation under the name of a machine whose
/// sessions made entries; a write that appends nothing is of no stream.
pub fn a_session_and_an_observed_stream_share_no_name(harness: &impl JournalHarness) {
    let store = harness.handle();
    store
        .observe("bank", "tx-1", &fact(b"observed"), 1_000)
        .expect("an observation");
    let held = lease(&store, "k");
    let facts = [fact(b"of a session")];
    let under = |machine: &'static str| Write {
        journal: Some(Journal {
            machine,
            facts: &facts,
        }),
        now_ms: 2_000,
        ..Write::put(b"s", None, 0, held.fence)
    };
    assert!(
        matches!(store.write("k", &under("bank")), Err(WriteError::Failed(_))),
        "a session appended under the name of an observed stream"
    );
    assert_eq!(read(&store, "k"), None, "a refused write kept its state");
    assert_eq!(texts(&all(&store)), ["observed"]);

    let version = store
        .write(
            "k",
            &Write {
                journal: Some(Journal {
                    machine: "bank",
                    facts: &[],
                }),
                ..Write::put(b"s", None, 0, held.fence)
            },
        )
        .expect("a write that appends nothing")
        .version;
    store
        .write(
            "k",
            &Write {
                expected_version: version,
                ..under(MACHINE)
            },
        )
        .expect("a session's entry under its machine");
    assert!(
        store
            .observe(MACHINE, "tx-2", &fact(b"observed"), 3_000)
            .is_err(),
        "an observation was kept under the name of a machine whose sessions made entries"
    );
    assert_eq!(texts(&all(&store)), ["observed", "of a session"]);
    assert_eq!(
        texts(&all_of(&store, Stream::Observed { name: "bank" })),
        ["observed"]
    );
    assert!(all_of(&store, Stream::Observed { name: MACHINE }).is_empty());
    assert!(all_of(
        &store,
        Stream::Session {
            machine: "bank",
            key: "k"
        }
    )
    .is_empty());
}

/// `NothingLost`: a write that fails partway, at any point the harness can make its store fail at
/// ([`JournalHarness::faults`]) or because a fact's bytes could not be made, leaves neither its
/// state nor an entry, for the handle it was made on and for one opened after; and the same write,
/// made again once nothing fails, leaves both. So does an observation.
pub fn a_write_stopped_partway_leaves_its_state_and_its_entries_or_neither(
    harness: &impl JournalHarness,
) {
    let store = harness.handle();
    let held = lease(&store, "k");
    let opened = [fact(b"opened")];
    let version = store
        .write("k", &keeps(b"one", 0, held.fence, &opened, 1_000))
        .expect("a session")
        .version;
    let (state, entries) = (read(&store, "k"), all(&store));
    let unmade = [
        fact(b"made"),
        Fact {
            data: FactData::AtPosition(&CannotBeMade),
            ..fact(b"")
        },
    ];
    assert!(
        matches!(
            store.write("k", &keeps(b"two", version, held.fence, &unmade, 2_000)),
            Err(WriteError::Failed(_))
        ),
        "a write whose fact could not be made"
    );
    assert!(store.observe("bank", "tx-1", &unmade[1], 2_000).is_err());
    for handle in [&store, &harness.handle()] {
        assert_eq!(
            (read(handle, "k"), all(handle)),
            (state.clone(), entries.clone()),
            "a write whose fact could not be made left something of itself"
        );
    }
    drop(store);

    let mut version = version;
    for fault in harness.faults() {
        let store = harness.handle();
        let before = (read(&store, "k"), all(&store));
        let facts = [fact(b"first"), fact(b"second")];
        let outbox = [Append {
            client: "phone",
            incarnation: "00112233445566778899aabbccddeeff",
            sender: Some("k"),
            envelope: b"reply",
        }];
        let write = Write {
            outbox: &outbox,
            ..keeps(b"next", version, held.fence, &facts, 3_000)
        };
        (fault.set)();
        assert!(
            matches!(store.write("k", &write), Err(WriteError::Failed(_))),
            "a write that fails at {} was answered otherwise",
            fault.at
        );
        // The process that made it goes, and another opens the store.
        drop(store);
        (fault.clear)();
        let store = harness.handle();
        assert_eq!(
            (read(&store, "k"), all(&store)),
            before,
            "a write that failed at {} left its state or an entry",
            fault.at
        );
        version = store
            .write("k", &write)
            .expect("the same write once nothing fails")
            .version;
        assert_eq!(
            read(&store, "k").map(|stored| stored.version),
            Some(version)
        );
        assert_eq!(
            texts(&all(&store)[before.1.len()..]),
            ["first", "second"],
            "the write made again after it failed at {}",
            fault.at
        );
    }

    let store = harness.handle();
    let last = [fact(b"last")];
    store
        .write("k", &keeps(b"last", version, held.fence, &last, 4_000))
        .expect("a write after every failure");
    assert_eq!(texts(&all(&store)).last().map(String::as_str), Some("last"));
}

/// The bytes a fact makes for its position are the bytes its entry keeps, and it is asked for the
/// position the entry has, of a session's entry and of an observed one.
pub fn a_fact_is_kept_with_the_bytes_made_for_its_position(harness: &impl JournalHarness) {
    let store = harness.handle();
    let held = lease(&store, "k");
    let stamped = Stamped(Mutex::new(Vec::new()));
    let made = Fact {
        data: FactData::AtPosition(&stamped),
        ..fact(b"")
    };
    let facts = [made, fact(b"given"), made];
    store
        .write("k", &keeps(b"s", 0, held.fence, &facts, 1_000))
        .expect("a write of facts made at their positions");
    let observed = store
        .observe("bank", "tx-1", &made, 2_000)
        .expect("an observation of one");
    let entries = all(&store);
    let expected = |at: usize| format!("made at {}", entries[at].position);
    assert_eq!(
        texts(&entries),
        [expected(0), "given".to_string(), expected(2), expected(3)]
    );
    assert_eq!(entries[3].position, observed.position());
    let asked = stamped.0.lock().unwrap_or_else(PoisonError::into_inner);
    for at in [0, 2, 3] {
        assert!(
            asked.contains(&entries[at].position),
            "asked for {asked:?}, kept at {}",
            entries[at].position
        );
    }
}

/// ADR 0109, D8: an entry's data is replaced only from the bytes it holds, every other field kept:
/// a replacement made from bytes another one already replaced writes nothing.
pub fn data_is_replaced_only_from_what_it_holds(harness: &impl JournalHarness) {
    let store = harness.handle();
    let held = lease(&store, "k");
    let facts = [dated(b"one", 70), fact(b"two")];
    store
        .write("k", &keeps(b"s", 0, held.fence, &facts, 1_000))
        .expect("two entries");
    store
        .observe("bank", "tx-1", &fact(b"three"), 2_000)
        .expect("an observed one");
    let before = all(&store);
    let at = before[0].position;
    let replace = |position: u64, from: &[u8], to: &[u8]| {
        store
            .reseal_journaled(position, from, to)
            .expect("a replacement the store could make or refuse")
    };
    assert!(
        !replace(at, b"other bytes", b"one again"),
        "replaced from bytes the entry does not hold"
    );
    assert!(
        !replace(last_position(&store) + 1, b"one", b"one again"),
        "a position the journal holds no entry at"
    );
    assert!(!replace(0, b"one", b"one again"), "no entry is at 0");
    assert!(!replace(u64::MAX, b"one", b"one again"));
    assert!(replace(at, b"one", b"one again"));
    assert!(
        !replace(at, b"one", b"stale"),
        "a replacement made from bytes already replaced is refused"
    );
    assert!(replace(before[2].position, b"three", b"three again"));
    let replaced = |at: usize, data: &[u8]| JournalEntry {
        data: data.to_vec(),
        ..before[at].clone()
    };
    assert_eq!(
        all(&harness.handle()),
        vec![
            replaced(0, b"one again"),
            before[1].clone(),
            replaced(2, b"three again")
        ],
        "every other field is kept"
    );
    assert_eq!(
        texts(&answer(&store, of_session("k"), before[1].position, 100)),
        ["one again"],
        "a valid time is kept"
    );
}

/// What one handle appended, by a write, a batch or an observation, another reads, and goes on
/// from.
pub fn handles_share_the_journal(harness: &impl JournalHarness) {
    let writer = harness.handle();
    let reader = harness.handle();
    let held = lease(&writer, "k");
    let facts = [fact(b"written")];
    let version = writer
        .write("k", &keeps(b"s", 0, held.fence, &facts, 1_000))
        .expect("a write that appends")
        .version;
    assert_eq!(texts(&all(&reader)), ["written"]);
    reader
        .observe("bank", "tx-1", &fact(b"observed"), 1_000)
        .expect("an observation through another handle");
    let batched = [fact(b"batched")];
    let third = harness.handle();
    let decided = third
        .write_many(&[Batched::Write {
            key: "k",
            write: keeps(b"s", version, held.fence, &batched, 1_000),
        }])
        .expect("a batch through a third");
    decided_write(&decided[0]).expect("its write");
    let entries = all(&writer);
    assert_eq!(texts(&entries), ["written", "observed", "batched"]);
    drop(third);
    assert_eq!(all(&reader), entries);
    assert_eq!(all(&harness.handle()), entries);
}

/// A reader's mark (D4): `0` for a reader that kept none, kept by its name and read through any
/// handle, never lower than one kept before, and refused above the horizon, where an entry can
/// still appear: past the last position, and, for a store that can be made to hold one open
/// ([`JournalHarness::begin`]), at a position a transaction took and has not committed. Red if a
/// store lowers a mark, or keeps one a follower that starts from it would pass an entry by.
pub fn a_reader_keeps_how_far_it_read(harness: &impl JournalHarness) {
    let store = harness.handle();
    assert_eq!(
        store.read_through("view").expect("a read of a mark"),
        0,
        "a reader that kept no mark reads from the start"
    );
    let held = lease(&store, "k");
    let facts = [fact(b"one"), fact(b"two")];
    let version = store
        .write("k", &keeps(b"s", 0, held.fence, &facts, 1_000))
        .expect("a write that appends")
        .version;
    let last = last_position(&store);
    let below = last.saturating_sub(1);
    assert_eq!(
        store
            .keep_read("view", below)
            .expect("a mark below the horizon"),
        below
    );
    assert_eq!(
        store
            .keep_read("view", below.saturating_sub(1))
            .expect("a lower mark"),
        below,
        "a mark fell"
    );
    assert_eq!(
        harness
            .handle()
            .read_through("view")
            .expect("a read of a mark"),
        below,
        "a mark is the store's, read through any handle"
    );
    assert_eq!(
        store.read_through("other").expect("a read of a mark"),
        0,
        "a mark is its reader's own"
    );
    let past = store.keep_read("view", last + 1);
    assert!(past.is_err(), "a mark above the horizon was kept: {past:?}");
    assert_eq!(
        store.read_through("view").expect("a read of a mark"),
        below,
        "a refused mark changed what was kept"
    );
    let pending = [fact(b"three")];
    if let Some(commit) = harness.begin("k", keeps(b"s", version, held.fence, &pending, 1_000)) {
        let open = store.keep_read("view", last + 1);
        assert!(
            open.is_err(),
            "a mark at a position still open was kept: {open:?}"
        );
        commit().expect("the commit of the transaction that took it");
        assert_eq!(
            store
                .keep_read("view", last + 1)
                .expect("a mark at a position committed"),
            last + 1
        );
    } else {
        assert_eq!(
            store
                .keep_read("view", last)
                .expect("a mark at the horizon"),
            last
        );
    }
}

/// The kit held to stores whose transactions commit out of the order of their numbers: one that
/// keeps the contract, which it must accept, and ones that break it, each in one way, which it must
/// refuse.
#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Arc, MutexGuard};

    use super::*;
    use crate::journal::recorded;
    use crate::{AcquireError, Due, LeaseError, MemoryStore, ObservedStream, SessionsStream};

    /// How a store breaks the contract, if it does.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Breach {
        None,
        /// Its horizon is the last position it gave, whatever is still open below it.
        ReadsToItsHead,
        /// A moment is turned into a position the horizon has not passed.
        PlacesAMomentAboveItsHorizon,
        /// An answer is given as of a position above the horizon.
        AnswersAboveItsHorizon,
        /// The entries of a write refused at its commit are kept.
        KeepsARefusedEntry,
        /// The state is written before the entries, by a write of its own.
        WritesItsEntriesApart,
        /// The recorded moment is the clock's, wherever it went.
        GoesBackWithTheClock,
        /// The transaction of a writer that is gone stays open.
        LeavesAnAbandonedTransactionOpen,
    }

    /// A store that numbers the entries of a transaction as it begins and makes them visible as it
    /// commits, as one over the sequence of a database server does: its sessions a store in
    /// memory, its journal its own.
    #[derive(Clone)]
    struct NumbersAtBegin {
        sessions: MemoryStore,
        numbers: Arc<Mutex<Numbers>>,
        breach: Breach,
    }

    #[derive(Default)]
    struct Numbers {
        /// The last position given.
        last: u64,
        /// The positions of transactions that began and have not ended.
        open: BTreeSet<u64>,
        /// What committed.
        entries: BTreeMap<u64, JournalEntry>,
        recorded_ms: u64,
        /// The marks of the journal's readers.
        readers: BTreeMap<String, u64>,
    }

    impl Numbers {
        /// Give the next position to a transaction that begins.
        fn give(&mut self) -> u64 {
            self.last += 1;
            self.open.insert(self.last);
            self.last
        }

        fn keep(&mut self, entry: JournalEntry) {
            self.recorded_ms = entry.recorded_ms;
            self.entries.insert(entry.position, entry);
        }
    }

    impl NumbersAtBegin {
        fn new(breach: Breach) -> Self {
            Self {
                sessions: MemoryStore::new(),
                numbers: Arc::default(),
                breach,
            }
        }

        fn numbers(&self) -> MutexGuard<'_, Numbers> {
            self.numbers.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn horizon(&self, numbers: &Numbers) -> u64 {
            if self.breach == Breach::ReadsToItsHead {
                return numbers.last;
            }
            numbers
                .open
                .first()
                .map_or(numbers.last, |lowest| lowest - 1)
        }

        fn entry(
            &self,
            numbers: &Numbers,
            position: u64,
            stream: Stream<'_>,
            origin: Option<&str>,
            fact: &Fact<'_>,
            now_ms: u64,
        ) -> Result<JournalEntry, Failure> {
            let last_ms = if self.breach == Breach::GoesBackWithTheClock {
                0
            } else {
                numbers.recorded_ms
            };
            Ok(JournalEntry {
                position,
                recorded_ms: recorded(last_ms, now_ms),
                stream: stream.to_name(),
                origin: origin.map(str::to_string),
                schema: fact.schema.to_string(),
                revision: fact.revision,
                valid_ms: fact.valid_ms,
                data: fact.data.at(position)?.into_owned(),
            })
        }

        /// The positions the facts of `write` take as its transaction begins.
        fn begin(&self, write: &Write<'_>) -> Vec<u64> {
            let facts = write.journal.map_or(0, |journal| journal.facts.len());
            let mut numbers = self.numbers();
            (0..facts).map(|_| numbers.give()).collect()
        }

        /// Commit the transaction that took `positions`: its entries become visible, or it is
        /// refused and they never do.
        fn commit(
            &self,
            key: &str,
            write: &Write<'_>,
            positions: &[u64],
        ) -> Result<Written, WriteError> {
            let mut numbers = self.numbers();
            let committed = self.committed(&mut numbers, key, write, positions);
            for position in positions {
                numbers.open.remove(position);
            }
            committed
        }

        fn committed(
            &self,
            numbers: &mut Numbers,
            key: &str,
            write: &Write<'_>,
            positions: &[u64],
        ) -> Result<Written, WriteError> {
            let state = Write {
                journal: None,
                ..*write
            };
            let Some(journal) = write.journal.filter(|journal| !journal.facts.is_empty()) else {
                return self.sessions.write(key, &state);
            };
            let observed = numbers.entries.values().any(|entry| {
                entry.stream.stream()
                    == Stream::Observed {
                        name: journal.machine,
                    }
            });
            if observed {
                return Err(WriteError::Failed(Failure::new(ObservedStream {
                    name: journal.machine.to_string(),
                })));
            }
            let stream = Stream::Session {
                machine: journal.machine,
                key,
            };
            let apart = if self.breach == Breach::WritesItsEntriesApart {
                Some(self.sessions.write(key, &state)?)
            } else {
                None
            };
            let entries = journal
                .facts
                .iter()
                .zip(positions)
                .map(|(fact, position)| {
                    self.entry(numbers, *position, stream, None, fact, write.now_ms)
                })
                .collect::<Result<Vec<JournalEntry>, Failure>>()?;
            let written = match apart {
                Some(written) => Ok(written),
                None => self.sessions.write(key, &state),
            };
            if written.is_ok() || self.breach == Breach::KeepsARefusedEntry {
                for entry in entries {
                    numbers.keep(entry);
                }
            }
            written
        }

        /// The entries a reader may read, in order.
        fn readable(&self, numbers: &Numbers, stream: Option<Stream<'_>>) -> Vec<JournalEntry> {
            let horizon = self.horizon(numbers);
            numbers
                .entries
                .values()
                .filter(|entry| entry.position <= horizon)
                .filter(|entry| stream.is_none_or(|asked| entry.stream.stream() == asked))
                .cloned()
                .collect()
        }

        fn page(&self, stream: Option<Stream<'_>>, after: u64, limit: usize) -> JournalPage {
            let numbers = self.numbers();
            JournalPage {
                entries: self
                    .readable(&numbers, stream)
                    .into_iter()
                    .filter(|entry| entry.position > after)
                    .take(limit)
                    .collect(),
                horizon: self.horizon(&numbers),
            }
        }
    }

    impl SessionStore for NumbersAtBegin {
        fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
            SessionStore::get(&self.sessions, key)
        }

        fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
            let positions = self.begin(write);
            // Another transaction may take positions and commit in between.
            thread::yield_now();
            self.commit(key, write, &positions)
        }

        fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
            self.sessions.due_before(ms, limit)
        }

        fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
            self.sessions.indexed_after(after, limit)
        }

        fn acquire(
            &self,
            key: &str,
            owner: &str,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, AcquireError> {
            self.sessions.acquire(key, owner, ttl_ms, now_ms)
        }

        fn renew(
            &self,
            key: &str,
            fence: u64,
            ttl_ms: u64,
            now_ms: u64,
        ) -> Result<Lease, LeaseError> {
            self.sessions.renew(key, fence, ttl_ms, now_ms)
        }

        fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
            self.sessions.release(key, fence)
        }
    }

    impl JournalStore for NumbersAtBegin {
        fn after(&self, position: u64, limit: usize) -> Result<JournalPage, Failure> {
            Ok(self.page(None, position, limit))
        }

        fn stream(
            &self,
            stream: Stream<'_>,
            after: u64,
            limit: usize,
        ) -> Result<JournalPage, Failure> {
            Ok(self.page(Some(stream), after, limit))
        }

        fn position_at(&self, ms: u64) -> Result<u64, Failure> {
            let numbers = self.numbers();
            let horizon = if self.breach == Breach::PlacesAMomentAboveItsHorizon {
                numbers.last
            } else {
                self.horizon(&numbers)
            };
            Ok(numbers
                .entries
                .values()
                .filter(|entry| entry.position <= horizon && entry.recorded_ms <= ms)
                .map(|entry| entry.position)
                .max()
                .unwrap_or(0))
        }

        fn as_of(
            &self,
            stream: Stream<'_>,
            as_of: &AsOf,
            after: AsOfCursor,
            limit: usize,
        ) -> Result<JournalPage, AsOfError> {
            let numbers = self.numbers();
            let horizon = self.horizon(&numbers);
            if self.breach != Breach::AnswersAboveItsHorizon {
                as_of.admitted(horizon)?;
            }
            let mut entries = self.readable(&numbers, Some(stream));
            entries.retain(|entry| as_of.includes(entry) && entry.cursor() > after);
            entries.sort_by_key(JournalEntry::cursor);
            entries.truncate(limit);
            Ok(JournalPage { entries, horizon })
        }

        fn observe(
            &self,
            name: &str,
            origin: &str,
            fact: &Fact<'_>,
            now_ms: u64,
        ) -> Result<Observation, Failure> {
            let mut numbers = self.numbers();
            let stream = Stream::Observed { name };
            for entry in numbers.entries.values() {
                if entry.stream.stream() == stream && entry.origin.as_deref() == Some(origin) {
                    return Ok(Observation::Repeated {
                        position: entry.position,
                    });
                }
                if let StreamName::Session { machine, .. } = &entry.stream {
                    if machine == name {
                        return Err(Failure::new(SessionsStream {
                            name: name.to_string(),
                        }));
                    }
                }
            }
            let position = numbers.give();
            let entry = self.entry(&numbers, position, stream, Some(origin), fact, now_ms);
            numbers.open.remove(&position);
            numbers.keep(entry?);
            Ok(Observation::Kept { position })
        }

        fn reseal_journaled(
            &self,
            position: u64,
            held: &[u8],
            data: &[u8],
        ) -> Result<bool, Failure> {
            let mut numbers = self.numbers();
            Ok(match numbers.entries.get_mut(&position) {
                Some(entry) if entry.data == held => {
                    entry.data = data.to_vec();
                    true
                }
                _ => false,
            })
        }

        fn read_through(&self, reader: &str) -> Result<u64, Failure> {
            Ok(self.numbers().readers.get(reader).copied().unwrap_or(0))
        }

        fn keep_read(&self, reader: &str, position: u64) -> Result<u64, Failure> {
            let mut numbers = self.numbers();
            let horizon = self.horizon(&numbers);
            if position > horizon {
                return Err(Failure::new(PastHorizon {
                    asked: position,
                    horizon,
                }));
            }
            let kept = numbers.readers.entry(reader.to_string()).or_insert(0);
            *kept = (*kept).max(position);
            Ok(*kept)
        }
    }

    /// A transaction that began, ended when its writer is gone.
    struct Began<'a> {
        store: &'a NumbersAtBegin,
        positions: Vec<u64>,
    }

    impl Drop for Began<'_> {
        fn drop(&mut self) {
            if self.store.breach == Breach::LeavesAnAbandonedTransactionOpen {
                return;
            }
            let mut numbers = self.store.numbers();
            for position in &self.positions {
                numbers.open.remove(position);
            }
        }
    }

    struct OutOfOrder(NumbersAtBegin);

    impl JournalHarness for OutOfOrder {
        type Store = NumbersAtBegin;

        fn handle(&self) -> NumbersAtBegin {
            self.0.clone()
        }

        fn begin<'a>(&'a self, key: &'a str, write: Write<'a>) -> Option<Commit<'a>> {
            let began = Began {
                store: &self.0,
                positions: self.0.begin(&write),
            };
            Some(Box::new(move || {
                began.store.commit(key, &write, &began.positions)
            }))
        }
    }

    fn broken(breach: Breach) -> OutOfOrder {
        OutOfOrder(NumbersAtBegin::new(breach))
    }

    macro_rules! accepts {
        ($($case:ident),* $(,)?) => {
            $(
                #[test]
                fn $case() {
                    super::$case(&broken(Breach::None));
                }
            )*
        };
    }

    accepts!(
        an_entry_is_made_by_its_sessions_write_or_not_at_all,
        a_batch_appends_item_by_item,
        a_removed_session_keeps_its_journal,
        an_input_makes_one_entry_and_a_reader_misses_none,
        a_page_is_resumed_where_it_ended,
        a_moment_is_never_below_the_one_before_it,
        an_answer_as_of_a_position_does_not_change,
        an_observed_entry_is_kept_once_for_its_origin,
        a_session_and_an_observed_stream_share_no_name,
        a_write_stopped_partway_leaves_its_state_and_its_entries_or_neither,
        a_fact_is_kept_with_the_bytes_made_for_its_position,
        data_is_replaced_only_from_what_it_holds,
        handles_share_the_journal,
        a_reader_keeps_how_far_it_read,
    );

    #[test]
    fn a_reader_that_waits_for_the_horizon_is_accepted() {
        assert_eq!(
            a_reader_waits_for_a_lower_number_still_open(&broken(Breach::None)),
            Forced::Yes,
            "the fake was not made to commit out of the order of its numbers"
        );
    }

    #[test]
    #[should_panic(expected = "a read passed a lower position still open")]
    fn a_store_that_reads_to_its_head_is_refused() {
        a_reader_waits_for_a_lower_number_still_open(&broken(Breach::ReadsToItsHead));
    }

    #[test]
    #[should_panic(expected = "a moment was turned into")]
    fn a_store_that_places_a_moment_above_its_horizon_is_refused() {
        a_reader_waits_for_a_lower_number_still_open(&broken(Breach::PlacesAMomentAboveItsHorizon));
    }

    #[test]
    #[should_panic(expected = "an answer was given above the horizon")]
    fn a_store_that_answers_above_its_horizon_is_refused() {
        a_reader_waits_for_a_lower_number_still_open(&broken(Breach::AnswersAboveItsHorizon));
    }

    #[test]
    #[should_panic(
        expected = "the horizon passes the position of a transaction whose writer is gone"
    )]
    fn a_store_that_leaves_an_abandoned_transaction_open_is_refused() {
        a_reader_waits_for_a_lower_number_still_open(&broken(
            Breach::LeavesAnAbandonedTransactionOpen,
        ));
    }

    #[test]
    #[should_panic(expected = "a refused write left an entry")]
    fn a_store_that_keeps_a_refused_entry_is_refused() {
        super::an_entry_is_made_by_its_sessions_write_or_not_at_all(&broken(
            Breach::KeepsARefusedEntry,
        ));
    }

    #[test]
    #[should_panic(expected = "left something of itself")]
    fn a_store_that_writes_its_entries_apart_is_refused() {
        super::a_write_stopped_partway_leaves_its_state_and_its_entries_or_neither(&broken(
            Breach::WritesItsEntriesApart,
        ));
    }

    #[test]
    #[should_panic(expected = "a recorded moment went back")]
    fn a_store_that_goes_back_with_the_clock_is_refused() {
        super::a_moment_is_never_below_the_one_before_it(&broken(Breach::GoesBackWithTheClock));
    }
}
