// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every implementation of [`KvStore`] is held to (ADR 0109, D13).
//!
//! The contract is the documentation of [`KvStore`]; these functions are that documentation as
//! histories with the answers a store must give. The two stores of this crate run them
//! (`tests/every_kv_store_keeps_one_contract.rs`), a sealing store runs them over both, and a
//! store written elsewhere runs the same functions from its own `#[test]`s, as with
//! [`crate::conformance`].
//!
//! * [`a_value_is_read_at_the_version_its_write_gave_it`]: every put and every delete that
//!   removed a key takes the next number of one counter, so a key removed and written again does
//!   not return to a version it had.
//! * [`a_put_if_writes_only_over_the_version_it_names`]: a put-if writes over the version it names,
//!   `0` for an absent key, and answers a lost race with the key's version as it is.
//! * [`the_quota_is_judged_only_for_a_write_that_would_be_made`]: a new key past the quota is
//!   refused and nothing is written; a replacement, and a put-if whose version does not match, are
//!   not judged by it.
//! * [`a_put_if_repeated_under_its_identity_answers_its_first_result`]: the first result of each
//!   identity is kept, a failure the host kept included, and answers every repeat without a write.
//! * [`the_keys_are_walked_in_the_order_of_their_bytes`]: next-key walks keys in the order of their
//!   UTF-8 bytes, which is not the order of their UTF-16 code units, within a prefix, after a key.
//! * [`the_values_are_listed_a_page_at_a_time`]: every value is listed in the order of its key, a
//!   page at a time.
//! * [`a_value_is_replaced_only_from_the_bytes_it_holds`]: a re-seal replaces a value's bytes and
//!   keeps its version, and only over the bytes it read.
//! * [`handles_share_the_values`]: what one handle wrote, another reads, its first results
//!   included.
//! * [`a_removal_of_a_session_deletes_its_identities_first_results`]: for a store that keeps the
//!   sessions too, the removal of a session deletes the first results of the sessions it names,
//!   and no other.
//!
//! Each function needs a harness whose store holds nothing when it starts.

use crate::kv::{Identity, KvEntry, KvStore, Put, PutIf};
use crate::{SessionStore, Write, LATEST_MOMENT};

/// What gives the functions of this module their store.
pub trait KvHarness {
    type Store: KvStore;

    /// Another handle on the one store this harness holds: it sees everything every other handle
    /// wrote. For a store in a database, a connection of its own, as a second process would have.
    fn handle(&self) -> Self::Store;
}

/// The identity of start `start` of the `<invoke>` `invoke` in the session `session`.
fn identity<'a>(session: &'a str, invoke: &'a str, start: u64) -> Identity<'a> {
    Identity {
        session,
        invoke,
        start,
    }
}

/// The value `key` holds, as the store answers it.
fn value(store: &impl KvStore, key: &str) -> Option<KvEntry> {
    store.get(key).expect("reading a value")
}

/// Every put and every delete that removed a key takes the next number of one counter.
pub fn a_value_is_read_at_the_version_its_write_gave_it(harness: &impl KvHarness) {
    let store = harness.handle();
    assert_eq!(value(&store, "a"), None, "a key nothing was written under");
    assert_eq!(
        store.put("a", b"one", None).expect("a put"),
        Put::Written {
            replaced: false,
            version: 1
        }
    );
    assert_eq!(
        store.put("a", b"two", None).expect("a put over it"),
        Put::Written {
            replaced: true,
            version: 2
        }
    );
    assert_eq!(
        store.put("b", b"", None).expect("an empty value"),
        Put::Written {
            replaced: false,
            version: 3
        }
    );
    assert_eq!(
        value(&store, "a"),
        Some(KvEntry {
            value: b"two".to_vec(),
            version: 2
        })
    );
    assert_eq!(
        value(&store, "b"),
        Some(KvEntry {
            value: Vec::new(),
            version: 3
        }),
        "a written empty value is held"
    );
    assert!(store.delete("a").expect("a delete"), "it held a value");
    assert!(
        !store.delete("a").expect("a second delete"),
        "it holds none, and takes no number"
    );
    assert_eq!(value(&store, "a"), None);
    assert_eq!(
        store.put("a", b"three", None).expect("written again"),
        Put::Written {
            replaced: false,
            version: 5
        },
        "past both the write and the delete before it: the delete took 4, the second none"
    );
}

/// A put-if writes over the version it names, and answers a lost race with the key's version.
pub fn a_put_if_writes_only_over_the_version_it_names(harness: &impl KvHarness) {
    let store = harness.handle();
    assert_eq!(
        store
            .put_if("k", b"first", Some(0), None, None)
            .expect("a put-if over an absent key"),
        PutIf::Written { version: 1 }
    );
    assert_eq!(
        store
            .put_if("k", b"again", Some(0), None, None)
            .expect("a put-if that lost"),
        PutIf::Unwritten { version: 1 },
        "the key is no longer absent"
    );
    assert_eq!(
        store
            .put_if("k", b"second", Some(1), None, None)
            .expect("a put-if over its version"),
        PutIf::Written { version: 2 }
    );
    assert_eq!(
        store
            .put_if("k", b"none", None, None, None)
            .expect("a put-if naming no version"),
        PutIf::Unwritten { version: 2 },
        "a number past 2^64 - 1 names no version and is never matched"
    );
    assert_eq!(
        store
            .put_if("other", b"x", Some(7), None, None)
            .expect("a put-if over an absent key at another version"),
        PutIf::Unwritten { version: 0 }
    );
    assert_eq!(value(&store, "other"), None, "a lost race writes nothing");
    assert_eq!(
        value(&store, "k"),
        Some(KvEntry {
            value: b"second".to_vec(),
            version: 2
        })
    );
}

/// A new key past the quota is refused, and the quota judges nothing else.
pub fn the_quota_is_judged_only_for_a_write_that_would_be_made(harness: &impl KvHarness) {
    let store = harness.handle();
    let quota = Some(1);
    assert!(matches!(
        store.put("a", b"1", quota).expect("the first key"),
        Put::Written { version: 1, .. }
    ));
    assert_eq!(
        store.put("b", b"2", quota).expect("a key past the quota"),
        Put::OverQuota
    );
    assert_eq!(value(&store, "b"), None, "nothing written past the quota");
    assert_eq!(
        store.put("a", b"3", quota).expect("a replacement"),
        Put::Written {
            replaced: true,
            version: 2
        },
        "a replacement adds no key: the refusal took no number"
    );
    assert_eq!(
        store
            .put_if("b", b"4", Some(0), quota, None)
            .expect("a put-if of a key past the quota"),
        PutIf::OverQuota
    );
    assert_eq!(
        store
            .put_if("b", b"5", Some(9), quota, None)
            .expect("a put-if whose version does not match"),
        PutIf::Unwritten { version: 0 },
        "a put-if that would not write is not judged by the quota"
    );
    assert_eq!(
        store
            .put_if("a", b"6", Some(2), quota, None)
            .expect("a put-if over a key held"),
        PutIf::Written { version: 3 }
    );
    assert!(matches!(
        store.put("b", b"7", None).expect("a put with no quota"),
        Put::Written { version: 4, .. }
    ));
}

/// The first result of each identity is kept and answers every repeat.
pub fn a_put_if_repeated_under_its_identity_answers_its_first_result(harness: &impl KvHarness) {
    let store = harness.handle();
    let first = identity("s-1", "putif", 1);
    assert_eq!(store.first_result(&first).expect("a first result"), None);
    assert_eq!(
        store
            .put_if("k", b"a", Some(0), None, Some(&first))
            .expect("the first start"),
        PutIf::Written { version: 1 }
    );
    assert_eq!(
        store.first_result(&first).expect("its first result"),
        Some(PutIf::Written { version: 1 }),
        "kept in the write's transaction"
    );
    assert_eq!(
        store
            .put_if("k", b"b", Some(0), None, Some(&first))
            .expect("the start repeated"),
        PutIf::Written { version: 1 },
        "the first result, where the store alone would answer a lost race"
    );
    assert_eq!(
        value(&store, "k"),
        Some(KvEntry {
            value: b"a".to_vec(),
            version: 1
        }),
        "a repeat writes nothing"
    );
    let second = identity("s-1", "putif", 2);
    assert_eq!(
        store
            .put_if("k", b"c", Some(0), None, Some(&second))
            .expect("the next start"),
        PutIf::Unwritten { version: 1 },
        "another start is judged against the store as it is"
    );
    assert_eq!(
        store
            .put_if("k", b"d", Some(1), None, Some(&second))
            .expect("the next start repeated with another version"),
        PutIf::Unwritten { version: 1 },
        "a lost race is a first result too"
    );
    let over = identity("s-1", "full", 1);
    assert_eq!(
        store
            .put_if("new", b"e", Some(0), Some(1), Some(&over))
            .expect("a start past the quota"),
        PutIf::OverQuota
    );
    assert_eq!(
        store
            .put_if("new", b"e", Some(0), None, Some(&over))
            .expect("the start past the quota repeated"),
        PutIf::OverQuota,
        "a failure is a first result"
    );
    assert_eq!(value(&store, "new"), None);
    let injected = identity("s-2", "putif", 1);
    let failed = PutIf::Failed {
        code: "unavailable".to_string(),
    };
    assert_eq!(
        store
            .keep_first(&injected, &failed)
            .expect("a failure the host kept"),
        failed
    );
    assert_eq!(
        store
            .keep_first(&injected, &PutIf::Written { version: 9 })
            .expect("a second result offered"),
        failed,
        "the first kept is the result"
    );
    assert_eq!(
        store
            .put_if("k", b"f", Some(1), None, Some(&injected))
            .expect("the failed start repeated"),
        failed,
        "answered by the failure, and the store not asked"
    );
    assert_eq!(value(&store, "k").map(|entry| entry.version), Some(1));
}

/// next-key walks the keys in the order of their UTF-8 bytes.
pub fn the_keys_are_walked_in_the_order_of_their_bytes(harness: &impl KvHarness) {
    let store = harness.handle();
    // U+FF61 is EF BD A1 in UTF-8 and FF61 in UTF-16; U+1F600 is F0 9F 98 80 in UTF-8 and the
    // surrogates D83D DE00 in UTF-16: the two orders disagree on them.
    for key in ["a/2", "a/1", "b", "\u{1F600}", "\u{FF61}", "a"] {
        store.put(key, key.as_bytes(), None).expect("a key");
    }
    let next = |after: &str, prefix: &str| store.next_key(after, prefix).expect("a walk");
    assert_eq!(next("", ""), Some("a".to_string()), "the least key");
    assert_eq!(next("a", ""), Some("a/1".to_string()), "after is excluded");
    assert_eq!(next("", "a/"), Some("a/1".to_string()), "within a prefix");
    assert_eq!(next("a/1", "a/"), Some("a/2".to_string()));
    assert_eq!(next("a/2", "a/"), None, "the prefix's keys are spent");
    assert_eq!(
        next("0", "b"),
        Some("b".to_string()),
        "after before the prefix"
    );
    assert_eq!(next("c", "b"), None, "after past the prefix");
    assert_eq!(next("b", ""), Some("\u{FF61}".to_string()));
    assert_eq!(
        next("\u{FF61}", ""),
        Some("\u{1F600}".to_string()),
        "U+FF61 sorts before U+1F600 by their UTF-8 bytes"
    );
    assert_eq!(next("\u{1F600}", ""), None);
    assert_eq!(next("", "z"), None, "no key has the prefix");
    store.delete("a/1").expect("a delete");
    assert_eq!(
        next("a", "a/"),
        Some("a/2".to_string()),
        "a removed key is not walked"
    );
}

/// Every value is listed in the order of its key, a page at a time.
pub fn the_values_are_listed_a_page_at_a_time(harness: &impl KvHarness) {
    let store = harness.handle();
    assert!(store.entries_after(None, 10).expect("none").is_empty());
    for key in ["c", "a", "b"] {
        store.put(key, key.as_bytes(), None).expect("a value");
    }
    let keys = |listed: Vec<(String, KvEntry)>| -> Vec<String> {
        listed.into_iter().map(|(key, _)| key).collect()
    };
    assert_eq!(
        keys(store.entries_after(None, 2).expect("a page")),
        ["a", "b"]
    );
    assert_eq!(
        keys(store.entries_after(Some("b"), 2).expect("the next page")),
        ["c"]
    );
    assert!(store
        .entries_after(Some("c"), 2)
        .expect("past the last")
        .is_empty());
    let listed = store.entries_after(Some("a"), 1).expect("one");
    assert_eq!(
        listed,
        [(
            "b".to_string(),
            KvEntry {
                value: b"b".to_vec(),
                version: 3
            }
        )]
    );
}

/// A re-seal replaces a value's bytes over the bytes it read, and keeps its version.
pub fn a_value_is_replaced_only_from_the_bytes_it_holds(harness: &impl KvHarness) {
    let store = harness.handle();
    store.put("k", b"held", None).expect("a value");
    assert!(!store
        .reseal("k", b"other", b"new")
        .expect("from other bytes"));
    assert!(!store
        .reseal("absent", b"", b"new")
        .expect("of an absent key"));
    assert!(store
        .reseal("k", b"held", b"new")
        .expect("from the bytes held"));
    assert_eq!(
        value(&store, "k"),
        Some(KvEntry {
            value: b"new".to_vec(),
            version: 1
        }),
        "the version does not move"
    );
    assert!(matches!(
        store.put("other", b"x", None).expect("the next write"),
        Put::Written { version: 2, .. }
    ));
}

/// What one handle wrote, another reads.
pub fn handles_share_the_values(harness: &impl KvHarness) {
    let writer = harness.handle();
    let reader = harness.handle();
    writer.put("k", b"v", None).expect("a value");
    let start = identity("s", "i", 1);
    writer
        .put_if("p", b"w", Some(0), None, Some(&start))
        .expect("a put-if");
    assert_eq!(
        value(&reader, "k"),
        Some(KvEntry {
            value: b"v".to_vec(),
            version: 1
        })
    );
    assert_eq!(
        reader.first_result(&start).expect("its first result"),
        Some(PutIf::Written { version: 2 })
    );
    assert_eq!(
        reader
            .put_if("p", b"again", Some(0), None, Some(&start))
            .expect("the start repeated on the other handle"),
        PutIf::Written { version: 2 }
    );
    assert_eq!(
        reader.next_key("", "").expect("a walk"),
        Some("k".to_string())
    );
}

/// The removal of a session deletes the first results of the sessions it names, in its
/// transaction, and no other; a keep deletes none.
pub fn a_removal_of_a_session_deletes_its_identities_first_results<H>(harness: &H)
where
    H: KvHarness,
    H::Store: SessionStore,
{
    let store = harness.handle();
    let root = identity("root-1", "putif", 1);
    let child = identity("child-1", "putif", 1);
    let other = identity("root-2", "putif", 1);
    for (n, start) in [root, child, other].iter().enumerate() {
        store
            .put_if(&format!("k{n}"), b"v", Some(0), None, Some(start))
            .expect("a put-if under an identity");
    }
    // The store keeps both contracts, each with a `get` and a `put` of its own: each call names its
    // contract.
    let lease = store
        .acquire("key-1", "owner", LATEST_MOMENT, 0)
        .expect("a lease");
    let version = SessionStore::put(&store, "key-1", b"state", None, 0, lease.fence)
        .expect("the session kept");
    store
        .write(
            "key-1",
            &Write {
                sessions: &["root-1", "child-1"],
                ..Write::put(b"state again", None, version, lease.fence)
            },
        )
        .expect("a keep");
    assert!(
        store.first_result(&root).expect("after a keep").is_some(),
        "a keep deletes no first result"
    );
    let version = SessionStore::get(&store, "key-1")
        .expect("the session")
        .expect("held")
        .version;
    store
        .write(
            "key-1",
            &Write {
                sessions: &["root-1", "child-1"],
                ..Write::delete(version, lease.fence)
            },
        )
        .expect("the removal");
    assert_eq!(store.first_result(&root).expect("the root's"), None);
    assert_eq!(store.first_result(&child).expect("the child's"), None);
    assert_eq!(
        store.first_result(&other).expect("another session's"),
        Some(PutIf::Written { version: 3 }),
        "a session the removal does not name keeps its first results"
    );
    assert!(
        value(&store, "k0").is_some(),
        "the values the first results were of are the store's, and stay"
    );
}
