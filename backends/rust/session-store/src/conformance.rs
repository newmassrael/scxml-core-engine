// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every implementation of [`SessionStore`] is held to.
//!
//! The contract is the documentation of [`SessionStore`]; these functions are that documentation as
//! histories with the answers a store must give. The two stores of this crate run them
//! (`tests/every_session_store_keeps_one_contract.rs`), and a store written elsewhere runs the same
//! functions from its own `#[test]`s, so that "keeps the contract" is a test and not a claim:
//!
//! ```text
//! struct MyHarness { url: String }
//!
//! impl Harness for MyHarness {
//!     type Store = MyStore;
//!     fn handle(&self) -> MyStore { MyStore::connect(&self.url) }
//! }
//!
//! #[test]
//! fn versions_are_compared_and_set() {
//!     conformance::versions_are_compared_and_set(&MyHarness::fresh());
//! }
//! ```
//!
//! * [`versions_are_compared_and_set`]: a put or a delete made from a version that is no longer
//!   the session's is refused and changes nothing, and no version is given twice, across a delete
//!   either.
//! * [`a_write_needs_the_current_fence`]: a write carries the fence of the lease held on its key;
//!   a holder whose lease was taken over, or released, writes nothing more, whatever version it
//!   read.
//! * [`a_lease_runs_out_and_is_taken_over`]: a lease is held against other owners until it runs
//!   out, a renewal moves its end and keeps its fence, and a lease that ran out is taken over with
//!   a higher fence.
//! * [`due_sessions_are_listed_earliest_first`]: the index of dues lists what is due at or before
//!   a moment, earliest first and by key among equals, up to a limit, and follows every put and
//!   delete.
//! * [`state_and_due_are_written_together`]: after any history, refused writes included, the
//!   index of dues is exactly what the stored sessions say.
//! * [`concurrent_writers_never_both_win`]: writers on threads, each with a handle of its own, that
//!   put from one version: exactly one is written, and what is stored is that one's state with that
//!   one's due.
//! * [`no_update_is_lost`]: writers that read, change and put back, again on a conflict, lose none
//!   of their changes.
//! * [`handles_share_one_store`]: what one handle wrote, another sees, its leases included.
//! * [`every_session_is_listed_by_its_entry_and_named_by_its_key`]: each key is indexed under
//!   an entry of its own, every session held is listed by its entry a page at a time in their
//!   order, and the entry of a held session names its key.
//! * [`a_batch_is_decided_item_by_item`] (ADR 0109, D15): a batch of writes keeps each one the
//!   rules admit and refuses each other for its version or its fence alone, and what it kept,
//!   its versions and its dues are as the same writes made one by one would leave them.
//! * [`a_batch_sees_its_earlier_items_and_is_seen_whole_on_return`] (ADR 0109, D15): each item
//!   is judged against what the items before it left, and once the batch answered every handle
//!   sees all it kept.
//! * [`a_session_is_its_machine_and_its_key`] (ADR 0114, D2): through handles of two machines on
//!   one store, an equal key names two sessions, each with its own state, lease, due and listing.
//!
//! Each function needs a harness whose store holds nothing when it starts, since it reads the whole
//! index of dues. The clock is the one the functions give; nothing here waits on time. A failed
//! expectation panics, as a test's does.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::sync::Barrier;
use std::thread;

use crate::batch::{Batched, Committed, Decided};
use crate::outbox::{Write, Written};
use crate::{
    AcquireError, Conflict, Due, Held, Lease, LeaseError, Lost, SessionStore, Stored, WriteError,
    LATEST_MOMENT,
};

/// What gives the functions of this module their store.
pub trait Harness {
    type Store: SessionStore;

    /// Another handle on the one store this harness holds: it sees everything every other handle
    /// wrote. For a store in a database, a connection of its own, as a second process would have.
    fn handle(&self) -> Self::Store;
}

/// A put or a delete made from a version that is no longer the session's is refused and changes
/// nothing; a version is never given twice, across a delete either.
pub fn versions_are_compared_and_set(harness: &impl Harness) {
    let store = harness.handle();
    let lease = granted(store.acquire("k", "a", 1_000, 0));
    assert_eq!(store.get("k").expect("reading a key never written"), None);

    let first = store
        .put("k", b"one", None, 0, lease.fence)
        .expect("a first put, made from no session");
    assert_ne!(first, 0, "a version of a stored session is never 0");
    assert_eq!(
        conflict(store.put("k", b"again", None, 0, lease.fence)),
        Conflict::Version {
            expected: 0,
            current: first
        },
        "a second put made from no session"
    );

    let second = store
        .put("k", b"two", Some(40), first, lease.fence)
        .expect("a put made from the session's version");
    assert!(second > first, "{second} after {first}");
    let stored = Stored {
        state: b"two".to_vec(),
        version: second,
        earliest_due: Some(40),
    };
    assert_eq!(read(&store, "k"), Some(stored.clone()));

    assert_eq!(
        conflict(store.put("k", b"stale", Some(1), first, lease.fence)),
        Conflict::Version {
            expected: first,
            current: second
        }
    );
    assert_eq!(
        conflict(store.delete("k", first, lease.fence)),
        Conflict::Version {
            expected: first,
            current: second
        }
    );
    assert_eq!(
        read(&store, "k"),
        Some(stored),
        "a refused write changed what is stored"
    );

    store
        .delete("k", second, lease.fence)
        .expect("a delete made from the session's version");
    assert_eq!(read(&store, "k"), None);
    assert_eq!(
        conflict(store.put("k", b"late", None, second, lease.fence)),
        Conflict::Version {
            expected: second,
            current: 0
        },
        "a put made from a session that was deleted since"
    );
    store
        .delete("k", 0, lease.fence)
        .expect("a delete made from no session, of no session");

    let third = store
        .put("k", b"three", None, 0, lease.fence)
        .expect("a put made from no session, after a delete");
    assert!(
        third > second,
        "the version after a delete, {third}, is not above the one before it, {second}"
    );
}

/// A write carries the fence of the lease held on its key; a holder whose lease was taken over or
/// released writes nothing more, whatever version it read. Fences rise across keys, and an owner
/// granted the lease again fences out its own earlier grant.
pub fn a_write_needs_the_current_fence(harness: &impl Harness) {
    let store = harness.handle();
    assert_eq!(
        conflict(store.put("k", b"x", None, 0, 1)),
        Conflict::Fence {
            presented: 1,
            current: None
        },
        "a write with no lease held"
    );

    let a = granted(store.acquire("k", "a", 100, 0));
    let first = store
        .put("k", b"a1", None, 0, a.fence)
        .expect("a write under the lease");
    let second = store
        .put("k", b"a2", None, first, a.fence)
        .expect("a second write under the lease");

    let b = granted(store.acquire("k", "b", 100, 150));
    assert!(b.fence > a.fence, "fence {} after {}", b.fence, a.fence);
    // `a` still believes it holds the key, and writes from the version it read, which is the
    // session's: the fence refuses it.
    assert_eq!(
        conflict(store.put("k", b"a3", None, second, a.fence)),
        Conflict::Fence {
            presented: a.fence,
            current: Some(b.fence)
        }
    );
    assert_eq!(
        conflict(store.delete("k", second, a.fence)),
        Conflict::Fence {
            presented: a.fence,
            current: Some(b.fence)
        }
    );
    assert_eq!(
        read(&store, "k").map(|stored| stored.state),
        Some(b"a2".to_vec()),
        "a fenced-out write changed what is stored"
    );
    let third = store
        .put("k", b"b1", None, second, b.fence)
        .expect("a write under the lease that took over");

    released(store.release("k", b.fence));
    assert_eq!(
        conflict(store.put("k", b"b2", None, third, b.fence)),
        Conflict::Fence {
            presented: b.fence,
            current: None
        },
        "a write under a released lease"
    );
    let c = granted(store.acquire("k", "c", 100, 160));
    assert!(c.fence > b.fence, "fence {} after {}", c.fence, b.fence);

    let elsewhere = granted(store.acquire("other", "a", 100, 160));
    assert!(
        elsewhere.fence > c.fence,
        "a fence on another key, {}, is not above every fence granted before, {}",
        elsewhere.fence,
        c.fence
    );

    // The holder asks again, as a process that started again under its name does: the new grant
    // fences out the old one.
    let again = granted(store.acquire("k", "c", 100, 170));
    assert!(again.fence > elsewhere.fence);
    assert_eq!(
        conflict(store.put("k", b"c1", None, third, c.fence)),
        Conflict::Fence {
            presented: c.fence,
            current: Some(again.fence)
        }
    );
    store
        .put("k", b"c1", None, third, again.fence)
        .expect("a write under the new grant");
}

/// A lease is held against other owners until it runs out; a renewal before then moves its end and
/// keeps its fence; a lease that ran out is taken over with a higher fence, and its holder can
/// neither renew nor release it.
pub fn a_lease_runs_out_and_is_taken_over(harness: &impl Harness) {
    let store = harness.handle();
    let a = granted(store.acquire("k", "a", 100, 1_000));
    assert_eq!(a.expires_ms, 1_100);
    assert_eq!(
        held(store.acquire("k", "b", 100, 1_099)),
        Held {
            owner: "a".to_string(),
            expires_ms: 1_100
        }
    );

    let renewed = store
        .renew("k", a.fence, 100, 1_050)
        .expect("a renewal before the lease runs out");
    assert_eq!(
        renewed,
        Lease {
            fence: a.fence,
            expires_ms: 1_150
        }
    );
    assert_eq!(
        held(store.acquire("k", "b", 100, 1_100)),
        Held {
            owner: "a".to_string(),
            expires_ms: 1_150
        },
        "the renewed lease is held past its first end"
    );
    assert_eq!(
        lost(store.renew("k", a.fence, 100, 1_150)),
        Lost::Expired { expires_ms: 1_150 },
        "a renewal at the moment the lease runs out"
    );

    let b = granted(store.acquire("k", "b", 100, 1_150));
    assert!(b.fence > a.fence, "fence {} after {}", b.fence, a.fence);
    assert_eq!(b.expires_ms, 1_250);
    assert_eq!(
        lost(store.renew("k", a.fence, 100, 1_160)),
        Lost::Superseded {
            current: Some(b.fence)
        }
    );
    assert_eq!(
        lost(store.release("k", a.fence)),
        Lost::Superseded {
            current: Some(b.fence)
        }
    );
    assert_eq!(
        held(store.acquire("k", "a", 100, 1_200)),
        Held {
            owner: "b".to_string(),
            expires_ms: 1_250
        },
        "a refused release took the lease away from its holder"
    );

    released(store.release("k", b.fence));
    assert_eq!(
        lost(store.release("k", b.fence)),
        Lost::Superseded { current: None }
    );
    assert_eq!(
        lost(store.renew("k", b.fence, 100, 1_200)),
        Lost::Superseded { current: None }
    );

    let far = granted(store.acquire("k", "a", u64::MAX, 1_200));
    assert_eq!(far.expires_ms, LATEST_MOMENT);
    assert_eq!(
        held(store.acquire("k", "b", 1, LATEST_MOMENT - 1)).expires_ms,
        LATEST_MOMENT
    );
}

/// The index of dues lists what is due at or before a moment, earliest first and by key among
/// equals, up to a limit; it follows every put and delete, and keeps a due past
/// [`LATEST_MOMENT`] as it.
pub fn due_sessions_are_listed_earliest_first(harness: &impl Harness) {
    let store = harness.handle();
    let c = create(&store, "c", Some(30));
    let b = create(&store, "b", Some(10));
    create(&store, "d", None);
    let a = create(&store, "a", Some(10));
    create(&store, "e", Some(20));

    assert_eq!(
        listed(&store, LATEST_MOMENT, 10),
        dues(&store, &[("a", 10), ("b", 10), ("e", 20), ("c", 30)])
    );
    assert_eq!(
        listed(&store, LATEST_MOMENT, 2),
        dues(&store, &[("a", 10), ("b", 10)])
    );
    assert_eq!(
        listed(&store, 20, 10),
        dues(&store, &[("a", 10), ("b", 10), ("e", 20)]),
        "a session due at the moment asked is due"
    );
    assert_eq!(listed(&store, 9, 10), dues(&store, &[]));
    assert_eq!(listed(&store, LATEST_MOMENT, 0), dues(&store, &[]));

    store
        .put("a", b"a", Some(25), a.version, a.lease.fence)
        .expect("moving a due later");
    store
        .put("b", b"b", None, b.version, b.lease.fence)
        .expect("clearing a due");
    assert_eq!(
        listed(&store, LATEST_MOMENT, 10),
        dues(&store, &[("e", 20), ("a", 25), ("c", 30)])
    );
    store
        .delete("c", c.version, c.lease.fence)
        .expect("deleting a session that was due");
    assert_eq!(
        listed(&store, LATEST_MOMENT, 10),
        dues(&store, &[("e", 20), ("a", 25)])
    );

    create(&store, "f", Some(u64::MAX));
    assert_eq!(
        read(&store, "f").and_then(|stored| stored.earliest_due),
        Some(LATEST_MOMENT)
    );
    assert_eq!(
        listed(&store, u64::MAX, 10),
        dues(&store, &[("e", 20), ("a", 25), ("f", LATEST_MOMENT)])
    );
}

/// After any history, refused writes included, the index of dues is exactly what the stored
/// sessions say: a write that is refused moves neither, and one that lands moves both.
pub fn state_and_due_are_written_together(harness: &impl Harness) {
    let store = harness.handle();
    let keys = ["k1", "k2", "k3"];
    let agree = |when: &str| {
        // Ordered as the index orders: by due, then by the entry the key is indexed under.
        let ordered: BTreeSet<(u64, String)> = keys
            .iter()
            .filter_map(|&key| {
                read(&store, key)
                    .and_then(|stored| stored.earliest_due)
                    .map(|due| (due, store.indexed(key)))
            })
            .collect();
        let expected: Vec<Due> = ordered
            .into_iter()
            .map(|(due, key)| Due { key, due })
            .collect();
        assert_eq!(
            listed(&store, LATEST_MOMENT, usize::MAX),
            expected,
            "the index of dues disagrees with the sessions {when}"
        );
    };

    let k1 = create(&store, "k1", Some(50));
    let k2 = create(&store, "k2", Some(10));
    agree("after two puts");

    let before = read(&store, "k1");
    assert_eq!(
        conflict(store.put("k1", b"stale", Some(5), 0, k1.lease.fence)),
        Conflict::Version {
            expected: 0,
            current: k1.version
        }
    );
    assert_eq!(read(&store, "k1"), before);
    agree("after a put refused for its version");

    // Another grant on the key, which fences out the one `k2` was written under.
    let taken = granted(store.acquire("k2", "a", 100, 2_000));
    assert_eq!(
        conflict(store.put("k2", b"fenced", Some(1), k2.version, k2.lease.fence)),
        Conflict::Fence {
            presented: k2.lease.fence,
            current: Some(taken.fence)
        }
    );
    agree("after a put refused for its fence");

    assert_eq!(
        conflict(store.put("k3", b"unleased", Some(3), 0, taken.fence)),
        Conflict::Fence {
            presented: taken.fence,
            current: None
        }
    );
    assert_eq!(read(&store, "k3"), None);
    agree("after a put refused on a key with no lease");

    store
        .put("k1", b"idle", None, k1.version, k1.lease.fence)
        .expect("a put that clears the due");
    agree("after a put that cleared a due");

    let k2_version = read(&store, "k2").map_or(0, |stored| stored.version);
    store
        .delete("k2", k2_version, taken.fence)
        .expect("a delete under the lease that took over");
    agree("after a delete");
}

/// Writers on threads, each with a handle of its own, put from one version at once: exactly one is
/// written, every other is told the winner's version, and what is stored is the winner's state
/// with the winner's due.
pub fn concurrent_writers_never_both_win(harness: &impl Harness) {
    const WRITERS: usize = 8;
    let store = harness.handle();
    let lease = granted(store.acquire("k", "a", LATEST_MOMENT, 0));
    let base = store
        .put("k", b"base", None, 0, lease.fence)
        .expect("the session the writers start from");
    let handles: Vec<_> = (0..WRITERS).map(|_| harness.handle()).collect();
    let start = Barrier::new(WRITERS);

    let outcomes: Vec<Result<u64, WriteError>> = thread::scope(|scope| {
        let writers: Vec<_> = handles
            .iter()
            .enumerate()
            .map(|(writer, handle)| {
                let start = &start;
                scope.spawn(move || {
                    let state = format!("writer {writer}");
                    start.wait();
                    handle.put(
                        "k",
                        state.as_bytes(),
                        Some(writer as u64),
                        base,
                        lease.fence,
                    )
                })
            })
            .collect();
        writers
            .into_iter()
            .map(|writer| writer.join().expect("a writer panicked"))
            .collect()
    });

    let winners: Vec<u64> = outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().ok().copied())
        .collect();
    assert_eq!(winners.len(), 1, "{outcomes:?}");
    let winner = winners[0];
    for outcome in &outcomes {
        match outcome {
            Ok(_) => {}
            Err(WriteError::Conflict(refused)) => assert_eq!(
                *refused,
                Conflict::Version {
                    expected: base,
                    current: winner
                }
            ),
            Err(other) => panic!("a writer failed: {other}"),
        }
    }

    let stored = read(&store, "k").expect("the winner's session");
    assert_eq!(stored.version, winner);
    let due = stored.earliest_due.expect("the winner's due");
    assert_eq!(
        stored.state,
        format!("writer {due}").into_bytes(),
        "the state stored is not the one written with the due stored"
    );
}

/// Writers on threads that read a count, add one and put it back, reading again whenever another
/// wrote first, lose none of their additions.
pub fn no_update_is_lost(harness: &impl Harness) {
    const WRITERS: usize = 4;
    const ADDITIONS: u64 = 25;
    let store = harness.handle();
    let lease = granted(store.acquire("count", "a", LATEST_MOMENT, 0));
    store
        .put("count", b"0", None, 0, lease.fence)
        .expect("the count the writers start from");
    let handles: Vec<_> = (0..WRITERS).map(|_| harness.handle()).collect();

    thread::scope(|scope| {
        let writers: Vec<_> = handles
            .iter()
            .map(|handle| {
                scope.spawn(move || {
                    let mut added = 0;
                    while added < ADDITIONS {
                        let stored = read(handle, "count").expect("the count");
                        let count: u64 = String::from_utf8(stored.state)
                            .expect("a count is a text")
                            .parse()
                            .expect("a count is a number");
                        let next = (count + 1).to_string();
                        match handle.put(
                            "count",
                            next.as_bytes(),
                            None,
                            stored.version,
                            lease.fence,
                        ) {
                            Ok(_) => added += 1,
                            Err(WriteError::Conflict(Conflict::Version { .. })) => {}
                            Err(other) => panic!("an addition failed: {other}"),
                        }
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().expect("a writer panicked");
        }
    });

    let stored = read(&store, "count").expect("the count");
    assert_eq!(
        String::from_utf8(stored.state).expect("a count is a text"),
        (WRITERS as u64 * ADDITIONS).to_string()
    );
}

/// What one handle wrote, another sees: sessions, dues and leases.
pub fn handles_share_one_store(harness: &impl Harness) {
    let first = harness.handle();
    let second = harness.handle();
    let lease = granted(first.acquire("k", "a", 100, 0));
    assert_eq!(
        held(second.acquire("k", "b", 100, 50)),
        Held {
            owner: "a".to_string(),
            expires_ms: 100
        },
        "a lease taken through one handle is held against another"
    );
    let version = second
        .put("k", b"from the second", Some(7), 0, lease.fence)
        .expect("a write through another handle, under the lease the first took");
    assert_eq!(
        read(&first, "k"),
        Some(Stored {
            state: b"from the second".to_vec(),
            version,
            earliest_due: Some(7),
        })
    );
    assert_eq!(listed(&first, LATEST_MOMENT, 10), dues(&first, &[("k", 7)]));
    drop(second);

    let third = harness.handle();
    third
        .delete("k", version, lease.fence)
        .expect("a delete through a handle opened after the write");
    assert_eq!(read(&first, "k"), None);
}

/// Each key is indexed under an entry of its own, the same each time it is asked; every session
/// held is listed by its entry, in the order of their bytes, a page at a time; the entry of a held
/// session names its key, and the entry of a key with no session names none.
pub fn every_session_is_listed_by_its_entry_and_named_by_its_key(harness: &impl Harness) {
    let store = harness.handle();
    let keys = ["b", "a", "d", "c"];
    let created: Vec<Created> = keys.iter().map(|&key| create(&store, key, None)).collect();
    let again = harness.handle();
    assert_eq!(
        store.indexed("a"),
        again.indexed("a"),
        "an entry is computed, the same through every handle"
    );
    let mut entries: Vec<String> = keys.iter().map(|&key| store.indexed(key)).collect();
    entries.sort();
    entries.dedup();
    assert_eq!(entries.len(), keys.len(), "two keys share an entry");

    let page = |after: Option<&str>, limit: usize| {
        store
            .indexed_after(after, limit)
            .expect("a listing the store could not make")
    };
    assert_eq!(page(None, 10), entries, "every session, in order");
    assert_eq!(page(None, 2), entries[..2], "at most the limit");
    assert_eq!(
        page(Some(entries[1].as_str()), 10),
        entries[2..],
        "after an entry, not at it"
    );
    assert!(
        page(Some(entries[3].as_str()), 10).is_empty(),
        "after the last"
    );
    assert!(page(None, 0).is_empty(), "a limit of none");
    let mut paged = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let next = page(after.as_deref(), 3);
        let Some(last) = next.last().cloned() else {
            break;
        };
        paged.extend(next);
        after = Some(last);
    }
    assert_eq!(paged, entries, "a pass a page at a time reads each once");

    for &key in &keys {
        assert_eq!(
            store
                .key_of(&store.indexed(key))
                .expect("the key of an entry"),
            Some(key.to_string()),
            "the entry of a held session names its key"
        );
    }
    assert_eq!(
        store
            .key_of(&store.indexed("never written"))
            .expect("the key of an entry with no session"),
        None
    );

    let c = &created[3];
    store
        .delete("c", c.version, c.lease.fence)
        .expect("deleting a session");
    assert_eq!(
        store
            .key_of(&store.indexed("c"))
            .expect("the key of a deleted session's entry"),
        None,
        "a deleted session's entry names no key"
    );
    assert!(
        !page(None, 10).contains(&store.indexed("c")),
        "a deleted session is not listed"
    );
}

/// ADR 0109, D15: a batch of writes is one transaction decided item by item. Of five writes, the
/// one made from a version that is no longer the session's and the one under a fence another
/// grant replaced are each refused as alone, and change nothing; the other three (a write over a
/// held session, a removal, a first write of a new one) are kept, each with a version of its own
/// above every one given before, and the index of dues follows exactly what was kept.
pub fn a_batch_is_decided_item_by_item(harness: &impl Harness) {
    let store = harness.handle();
    let kept = create(&store, "kept", Some(30));
    let stale = create(&store, "stale", Some(10));
    let fenced = create(&store, "fenced", Some(20));
    let removed = create(&store, "removed", Some(5));
    // The holder asks again, as a process that started again does: the first grant's writes are
    // fenced out.
    let taken = granted(store.acquire("fenced", "a", LATEST_MOMENT, 0));
    let new = granted(store.acquire("new", "a", LATEST_MOMENT, 0));
    let before_stale = read(&store, "stale");
    let before_fenced = read(&store, "fenced");
    let highest = new.fence;

    let decided = store
        .write_many(&[
            Batched::Write {
                key: "kept",
                write: Write::put(b"kept again", Some(40), kept.version, kept.lease.fence),
            },
            Batched::Write {
                key: "stale",
                write: Write::put(b"stale again", Some(1), 0, stale.lease.fence),
            },
            Batched::Write {
                key: "fenced",
                write: Write::put(b"fenced again", Some(2), fenced.version, fenced.lease.fence),
            },
            Batched::Write {
                key: "removed",
                write: Write::delete(removed.version, removed.lease.fence),
            },
            Batched::Write {
                key: "new",
                write: Write::put(b"new", Some(50), 0, new.fence),
            },
        ])
        .expect("a batch the store could make");
    assert_eq!(decided.len(), 5, "an answer for each item: {decided:?}");

    let kept_version = written(&decided[0]).version;
    assert_eq!(
        refused(&decided[1]),
        Conflict::Version {
            expected: 0,
            current: stale.version
        },
        "an item made from a version that is not the session's"
    );
    assert_eq!(
        refused(&decided[2]),
        Conflict::Fence {
            presented: fenced.lease.fence,
            current: Some(taken.fence)
        },
        "an item under a fence another grant replaced"
    );
    assert_eq!(
        written(&decided[3]),
        &Written {
            version: 0,
            appended: Vec::new()
        },
        "a removal"
    );
    let new_version = written(&decided[4]).version;
    assert!(
        kept_version > highest && new_version > highest && kept_version != new_version,
        "the versions of a batch, {kept_version} and {new_version}, are not each new above {highest}"
    );

    assert_eq!(
        read(&store, "kept"),
        Some(Stored {
            state: b"kept again".to_vec(),
            version: kept_version,
            earliest_due: Some(40),
        })
    );
    assert_eq!(
        read(&store, "stale"),
        before_stale,
        "a refused item changed its session"
    );
    assert_eq!(
        read(&store, "fenced"),
        before_fenced,
        "a fenced-out item changed its session"
    );
    assert_eq!(read(&store, "removed"), None);
    assert_eq!(
        read(&store, "new").map(|stored| stored.version),
        Some(new_version)
    );
    assert_eq!(
        listed(&store, LATEST_MOMENT, 10),
        dues(
            &store,
            &[("stale", 10), ("fenced", 20), ("kept", 40), ("new", 50)]
        ),
        "the index of dues is not what the batch kept"
    );
}

/// ADR 0109, D15: the items of a batch are judged in order, each against what the ones before it
/// left: a second first write of one key, after a first in the same batch, is refused as made
/// from a version that is no longer the session's. Once the batch answered, a handle opened after
/// it sees everything it kept, the store's numbers go on above it, and a batch of nothing is
/// answered with nothing.
pub fn a_batch_sees_its_earlier_items_and_is_seen_whole_on_return(harness: &impl Harness) {
    let store = harness.handle();
    let lease = granted(store.acquire("k", "a", LATEST_MOMENT, 0));
    let other = granted(store.acquire("other", "a", LATEST_MOMENT, 0));
    let decided = store
        .write_many(&[
            Batched::Write {
                key: "k",
                write: Write::put(b"first", Some(7), 0, lease.fence),
            },
            Batched::Write {
                key: "k",
                write: Write::put(b"second", None, 0, lease.fence),
            },
            Batched::Write {
                key: "other",
                write: Write::put(b"other", None, 0, other.fence),
            },
        ])
        .expect("a batch the store could make");
    assert_eq!(decided.len(), 3, "{decided:?}");
    let first = written(&decided[0]).version;
    assert_eq!(
        refused(&decided[1]),
        Conflict::Version {
            expected: 0,
            current: first
        },
        "an item judged without the item before it"
    );
    let other_version = written(&decided[2]).version;

    let again = harness.handle();
    assert_eq!(
        read(&again, "k"),
        Some(Stored {
            state: b"first".to_vec(),
            version: first,
            earliest_due: Some(7),
        }),
        "a handle opened after the batch does not see what it kept"
    );
    assert_eq!(
        read(&again, "other").map(|stored| stored.version),
        Some(other_version)
    );
    let next = again
        .put("k", b"third", None, first, lease.fence)
        .expect("a write after the batch, from the version it kept");
    assert!(
        next > first && next > other_version,
        "the store's numbers started again after a batch: {next}"
    );
    assert!(
        store
            .write_many(&[])
            .expect("a batch of nothing")
            .is_empty(),
        "a batch of nothing is answered with nothing"
    );
}

/// What a kept write of a batch answered.
/// A session is its machine and its key (ADR 0114, D2): through `rooms` and `view`, two handles
/// on one store, each of a machine of its own ([`SessionStore::machine`]), an equal key names two
/// sessions, each with its own state, version, lease and due, each handle lists only its own
/// machine's, and a fence of one is no fence of the other. Red if a store keys a session, or its
/// lease, by its key alone.
pub fn a_session_is_its_machine_and_its_key(rooms: &impl SessionStore, view: &impl SessionStore) {
    assert_ne!(
        rooms.machine(),
        view.machine(),
        "two handles of one machine"
    );
    let room = granted(rooms.acquire("k", "a", 1_000, 0));
    let viewed = granted(view.acquire("k", "b", 1_000, 0));
    assert_ne!(room.fence, viewed.fence, "one fence given twice");
    let first = rooms
        .put("k", b"room", Some(10), 0, room.fence)
        .expect("the first put of one machine's session");
    let second = view
        .put("k", b"view", Some(20), 0, viewed.fence)
        .expect("the first put of the other machine's session, made from none of its own");
    assert_eq!(
        read(rooms, "k").map(|stored| stored.state),
        Some(b"room".to_vec())
    );
    assert_eq!(
        read(view, "k").map(|stored| stored.state),
        Some(b"view".to_vec())
    );
    assert_eq!(listed(rooms, LATEST_MOMENT, 8), dues(rooms, &[("k", 10)]));
    assert_eq!(listed(view, LATEST_MOMENT, 8), dues(view, &[("k", 20)]));
    assert_eq!(
        rooms
            .indexed_after(None, 8)
            .expect("a listing of one machine's sessions"),
        [rooms.indexed("k")]
    );
    assert!(
        matches!(
            conflict(view.put("k", b"taken", None, second, room.fence)),
            Conflict::Fence { .. }
        ),
        "a fence of one machine's lease wrote the other's session"
    );
    rooms
        .delete("k", first, room.fence)
        .expect("the removal of one machine's session");
    assert_eq!(read(rooms, "k"), None);
    assert_eq!(
        read(view, "k").map(|stored| stored.version),
        Some(second),
        "a removal took the other machine's session"
    );
    released(rooms.release("k", room.fence));
    assert_eq!(
        held(view.acquire("k", "c", 1_000, 0)).owner,
        "b",
        "a release of one machine's lease let the other's go"
    );
}

fn written(decided: &Decided) -> &Written {
    match decided {
        Ok(Committed::Written(written)) => written,
        other => panic!("an item that should have been written gave {other:?}"),
    }
}

/// Why the store refused an item of a batch.
fn refused(decided: &Decided) -> Conflict {
    match decided {
        Err(WriteError::Conflict(conflict)) => *conflict,
        other => panic!("an item that should have been refused gave {other:?}"),
    }
}

/// A session [`create`] made: the lease it was written under and its version.
struct Created {
    lease: Lease,
    version: u64,
}

/// Take a lease on `key` and write a session there whose state is the key's text.
fn create(store: &impl SessionStore, key: &str, due: Option<u64>) -> Created {
    let lease = granted(store.acquire(key, "a", LATEST_MOMENT, 0));
    let version = store
        .put(key, key.as_bytes(), due, 0, lease.fence)
        .expect("a session written under its lease");
    Created { lease, version }
}

fn read(store: &impl SessionStore, key: &str) -> Option<Stored> {
    store.get(key).expect("a read the store could not make")
}

fn listed(store: &impl SessionStore, ms: u64, limit: usize) -> Vec<Due> {
    store
        .due_before(ms, limit)
        .expect("a listing the store could not make")
}

/// The listing of the sessions `pairs` names, each by the entry `store` indexes it under, in the
/// order the contract gives: by due, then by entry.
fn dues(store: &impl SessionStore, pairs: &[(&str, u64)]) -> Vec<Due> {
    let mut listed: Vec<Due> = pairs
        .iter()
        .map(|&(key, due)| Due {
            key: store.indexed(key),
            due,
        })
        .collect();
    listed.sort_by(|a, b| a.due.cmp(&b.due).then_with(|| a.key.cmp(&b.key)));
    listed
}

fn granted(result: Result<Lease, AcquireError>) -> Lease {
    result.expect("a lease that should have been granted")
}

fn released(result: Result<(), LeaseError>) {
    result.expect("a release of the lease held");
}

fn conflict<T: Debug>(result: Result<T, WriteError>) -> Conflict {
    match result {
        Err(WriteError::Conflict(conflict)) => conflict,
        other => panic!("a write that should have been refused gave {other:?}"),
    }
}

fn held(result: Result<Lease, AcquireError>) -> Held {
    match result {
        Err(AcquireError::Held(held)) => held,
        other => panic!("a lease that should have been held by another gave {other:?}"),
    }
}

fn lost<T: Debug>(result: Result<T, LeaseError>) -> Lost {
    match result {
        Err(LeaseError::Lost(lost)) => lost,
        other => panic!("a lease that should have been lost gave {other:?}"),
    }
}
