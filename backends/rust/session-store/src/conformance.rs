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
//!
//! Each function needs a harness whose store holds nothing when it starts, since it reads the whole
//! index of dues. The clock is the one the functions give; nothing here waits on time. A failed
//! expectation panics, as a test's does.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::sync::Barrier;
use std::thread;

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
        dues(&[("a", 10), ("b", 10), ("e", 20), ("c", 30)])
    );
    assert_eq!(
        listed(&store, LATEST_MOMENT, 2),
        dues(&[("a", 10), ("b", 10)])
    );
    assert_eq!(
        listed(&store, 20, 10),
        dues(&[("a", 10), ("b", 10), ("e", 20)]),
        "a session due at the moment asked is due"
    );
    assert_eq!(listed(&store, 9, 10), dues(&[]));
    assert_eq!(listed(&store, LATEST_MOMENT, 0), dues(&[]));

    store
        .put("a", b"a", Some(25), a.version, a.lease.fence)
        .expect("moving a due later");
    store
        .put("b", b"b", None, b.version, b.lease.fence)
        .expect("clearing a due");
    assert_eq!(
        listed(&store, LATEST_MOMENT, 10),
        dues(&[("e", 20), ("a", 25), ("c", 30)])
    );
    store
        .delete("c", c.version, c.lease.fence)
        .expect("deleting a session that was due");
    assert_eq!(
        listed(&store, LATEST_MOMENT, 10),
        dues(&[("e", 20), ("a", 25)])
    );

    create(&store, "f", Some(u64::MAX));
    assert_eq!(
        read(&store, "f").and_then(|stored| stored.earliest_due),
        Some(LATEST_MOMENT)
    );
    assert_eq!(
        listed(&store, u64::MAX, 10),
        dues(&[("e", 20), ("a", 25), ("f", LATEST_MOMENT)])
    );
}

/// After any history, refused writes included, the index of dues is exactly what the stored
/// sessions say: a write that is refused moves neither, and one that lands moves both.
pub fn state_and_due_are_written_together(harness: &impl Harness) {
    let store = harness.handle();
    let keys = ["k1", "k2", "k3"];
    let agree = |when: &str| {
        // Ordered as the index orders: by due, then by key.
        let ordered: BTreeSet<(u64, String)> = keys
            .iter()
            .filter_map(|&key| {
                read(&store, key)
                    .and_then(|stored| stored.earliest_due)
                    .map(|due| (due, key.to_string()))
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
    assert_eq!(listed(&first, LATEST_MOMENT, 10), dues(&[("k", 7)]));
    drop(second);

    let third = harness.handle();
    third
        .delete("k", version, lease.fence)
        .expect("a delete through a handle opened after the write");
    assert_eq!(read(&first, "k"), None);
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

fn dues(pairs: &[(&str, u64)]) -> Vec<Due> {
    pairs
        .iter()
        .map(|&(key, due)| Due {
            key: key.to_string(),
            due,
        })
        .collect()
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
