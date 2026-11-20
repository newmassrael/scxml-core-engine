// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every [`SessionStore`] that keeps an outbox is held to (ADR 0109, D10; module
//! [`outbox`](crate::outbox)).
//!
//! The contract is the documentation of [`SessionStore::write`] and of the outbox calls; these
//! functions are that documentation as histories with the answers a store must give. The stores
//! of this crate run them (`tests/every_session_store_keeps_one_contract.rs`), the sealing store
//! runs them over both, and a store written elsewhere runs the same functions from its own
//! `#[test]`s, with the [`Harness`] of the session kit:
//!
//! * [`an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all`]: the appends of a write
//!   are kept with it, numbered on in the outbox's incarnation, and a write the store refuses
//!   appends nothing.
//! * [`an_outbox_is_opened_once_and_numbers_on`]: an outbox opened by an exchange keeps the first
//!   incarnation it was given, and appends from every handle number on in it, with a session or
//!   without one.
//! * [`an_acknowledgement_deletes_what_it_covers_in_its_incarnation`]: an acknowledgement
//!   deletes the entries numbered at most its mark, and only in the outbox's own incarnation.
//! * [`an_old_entry_expires_and_its_sender_is_noted`]: an expiry deletes what is older than its
//!   moment, writes a note for each entry's sender and makes it due at once, closes an outbox
//!   left empty and untouched, and a write of the sender takes its notes, a removal all of them.
//! * [`a_batch_appends_and_acknowledges_item_by_item`] (ADR 0109, D15): a batch's
//!   acknowledgements and writes are decided in order, item by item, and a write it refuses
//!   appends nothing.
//!
//! Each function needs a harness whose store holds nothing when it starts. The clock is the one
//! the functions give; nothing here waits on time. A failed expectation panics, as a test's does.

use crate::batch::{Batched, Committed};
use crate::conformance::Harness;
use crate::outbox::{Append, Appended, Write};
use crate::{Conflict, Lease, SessionStore, WriteError, LATEST_MOMENT};

/// The incarnation the functions open outboxes in.
const FIRST: &str = "00112233445566778899aabbccddeeff";
/// Another incarnation, which an outbox held already does not take.
const SECOND: &str = "ffeeddccbbaa99887766554433221100";

/// The appends of `envelopes` to `client`, in `FIRST`, sent by `sender`.
fn appends<'a>(
    client: &'a str,
    sender: Option<&'a str>,
    envelopes: &[&'a [u8]],
) -> Vec<Append<'a>> {
    envelopes
        .iter()
        .copied()
        .map(|envelope| Append {
            client,
            incarnation: FIRST,
            sender,
            envelope,
        })
        .collect()
}

/// What the outbox of `client` holds, as `(number, envelope)` pairs, oldest first; empty when
/// the store holds no outbox for it.
fn held(store: &impl SessionStore, client: &str) -> Vec<(u64, Vec<u8>)> {
    store
        .outbox(client)
        .expect("an outbox the store could not read")
        .map(|outbox| {
            outbox
                .entries
                .into_iter()
                .map(|entry| (entry.number, entry.envelope))
                .collect()
        })
        .unwrap_or_default()
}

/// The numbers of `appended`.
fn numbers(appended: &[Appended]) -> Vec<u64> {
    appended.iter().map(|kept| kept.number).collect()
}

fn lease(store: &impl SessionStore, key: &str) -> Lease {
    store
        .acquire(key, "a", LATEST_MOMENT, 0)
        .expect("a lease on a key no one holds")
}

/// The appends of a write are kept with it, each numbered on in its outbox's incarnation and kept
/// at the write's moment; a write the store refuses, for its version or its fence, appends
/// nothing, and the next one numbers on from where the last kept one stopped.
pub fn an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all(harness: &impl Harness) {
    let store = harness.handle();
    let held_lease = lease(&store, "k");
    let first = appends("phone", Some("k"), &[b"one", b"two"]);
    let written = store
        .write(
            "k",
            &Write {
                outbox: &first,
                now_ms: 1_000,
                ..Write::put(b"sent two", Some(50), 0, held_lease.fence)
            },
        )
        .expect("a first write that appends");
    assert_ne!(written.version, 0);
    assert_eq!(numbers(&written.appended), vec![1, 2]);
    assert!(
        written
            .appended
            .iter()
            .all(|kept| kept.client == "phone" && kept.incarnation == FIRST),
        "{:?}",
        written.appended
    );
    let outbox = store
        .outbox("phone")
        .expect("the outbox")
        .expect("an outbox opened by the append");
    assert_eq!(outbox.incarnation, FIRST);
    assert_eq!(outbox.next, 3);
    assert!(outbox
        .entries
        .iter()
        .all(|entry| entry.at_ms == 1_000 && entry.sender.as_deref() == Some("k")));

    let refused = appends("phone", Some("k"), &[b"stale"]);
    let stale = store.write(
        "k",
        &Write {
            outbox: &refused,
            now_ms: 1_100,
            ..Write::put(b"stale", None, 0, held_lease.fence)
        },
    );
    assert_eq!(
        conflict(stale),
        Conflict::Version {
            expected: 0,
            current: written.version
        }
    );
    let fenced = store.write(
        "k",
        &Write {
            outbox: &refused,
            now_ms: 1_100,
            ..Write::put(b"fenced", None, written.version, held_lease.fence + 1_000)
        },
    );
    assert!(
        matches!(conflict(fenced), Conflict::Fence { .. }),
        "a write under another fence"
    );
    assert_eq!(
        held(&store, "phone"),
        vec![(1, b"one".to_vec()), (2, b"two".to_vec())],
        "a refused write appended"
    );

    let other = SECOND_APPEND;
    let next = store
        .write(
            "k",
            &Write {
                outbox: &[
                    Append {
                        client: "phone",
                        incarnation: SECOND,
                        sender: Some("k"),
                        envelope: b"three",
                    },
                    other,
                ],
                now_ms: 1_200,
                ..Write::put(b"sent four", None, written.version, held_lease.fence)
            },
        )
        .expect("a write from the session's version");
    assert_eq!(numbers(&next.appended), vec![3, 1]);
    assert_eq!(
        next.appended[0].incarnation, FIRST,
        "an outbox the store holds keeps its own incarnation"
    );
    assert_eq!(next.appended[1].incarnation, SECOND);
    assert_eq!(held(&store, "tablet"), vec![(1, b"four".to_vec())]);
    assert_eq!(
        store
            .get("k")
            .expect("the session")
            .map(|stored| stored.state),
        Some(b"sent four".to_vec()),
        "the session and its appends are one write"
    );

    // A removal appends with it, as a session that finished sends its last answer.
    let last = appends("phone", Some("k"), &[b"last"]);
    let removed = store
        .write(
            "k",
            &Write {
                outbox: &last,
                now_ms: 1_300,
                ..Write::delete(next.version, held_lease.fence)
            },
        )
        .expect("a removal that appends");
    assert_eq!(removed.version, 0);
    assert_eq!(numbers(&removed.appended), vec![4]);
    assert_eq!(store.get("k").expect("the session"), None);
}

/// The second outbox [`an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all`] appends
/// to, opened by the append in [`SECOND`].
const SECOND_APPEND: Append<'static> = Append {
    client: "tablet",
    incarnation: SECOND,
    sender: None,
    envelope: b"four",
};

/// An outbox opened by an exchange keeps the incarnation it was opened in, whatever another
/// opening or append offers; appends through every handle, from a session's write or from none,
/// number on in it.
pub fn an_outbox_is_opened_once_and_numbers_on(harness: &impl Harness) {
    let store = harness.handle();
    assert_eq!(store.outbox("phone").expect("no outbox yet"), None);
    let opened = store
        .open_outbox("phone", FIRST, 500)
        .expect("an outbox opened");
    assert_eq!(
        (opened.incarnation.as_str(), opened.next),
        (FIRST, 1),
        "an outbox opened holds nothing and numbers from 1"
    );
    assert!(opened.entries.is_empty());
    let again = harness
        .handle()
        .open_outbox("phone", SECOND, 600)
        .expect("an outbox opened again");
    assert_eq!(again.incarnation, FIRST, "the first incarnation is kept");

    let other = harness.handle();
    let kept = other
        .append(
            &[Append {
                client: "phone",
                incarnation: SECOND,
                sender: None,
                envelope: b"by no session",
            }],
            700,
        )
        .expect("an append under no session");
    assert_eq!(numbers(&kept), vec![1]);
    assert_eq!(kept[0].incarnation, FIRST);
    let session = lease(&store, "k");
    let after = appends("phone", Some("k"), &[b"by a session"]);
    let written = store
        .write(
            "k",
            &Write {
                outbox: &after,
                now_ms: 800,
                ..Write::put(b"s", None, 0, session.fence)
            },
        )
        .expect("a write that appends");
    assert_eq!(numbers(&written.appended), vec![2]);
    let outbox = harness
        .handle()
        .outbox("phone")
        .expect("the outbox")
        .expect("an outbox held");
    assert_eq!(outbox.next, 3);
    assert_eq!(
        outbox
            .entries
            .iter()
            .map(|entry| (entry.number, entry.at_ms, entry.sender.clone()))
            .collect::<Vec<_>>(),
        vec![(1, 700, None), (2, 800, Some("k".to_string()))]
    );
}

/// An acknowledgement deletes every entry numbered at most its mark, and only when it counts in
/// the outbox's own incarnation; one of another incarnation, or of a client with no outbox,
/// deletes nothing.
pub fn an_acknowledgement_deletes_what_it_covers_in_its_incarnation(harness: &impl Harness) {
    let store = harness.handle();
    let kept = store
        .append(&appends("phone", None, &[b"1", b"2", b"3", b"4"]), 100)
        .expect("four appends");
    assert_eq!(numbers(&kept), vec![1, 2, 3, 4]);
    assert_eq!(
        store
            .acknowledge("phone", SECOND, 4, 200)
            .expect("an acknowledgement of another incarnation"),
        0
    );
    assert_eq!(held(&store, "phone").len(), 4);
    assert_eq!(
        store
            .acknowledge("phone", FIRST, 2, 200)
            .expect("an acknowledgement"),
        2
    );
    assert_eq!(
        held(&harness.handle(), "phone"),
        vec![(3, b"3".to_vec()), (4, b"4".to_vec())],
        "what the acknowledgement covered is deleted, for every handle"
    );
    assert_eq!(
        store
            .acknowledge("phone", FIRST, 2, 300)
            .expect("the same acknowledgement again"),
        0,
        "an acknowledgement is idempotent"
    );
    assert_eq!(
        store
            .acknowledge("phone", FIRST, 10, 300)
            .expect("an acknowledgement past the last"),
        2
    );
    assert!(held(&store, "phone").is_empty());
    assert_eq!(
        store
            .outbox("phone")
            .expect("the outbox")
            .map(|outbox| outbox.next),
        Some(5),
        "an outbox emptied by acknowledgements numbers on"
    );
    assert_eq!(
        store
            .acknowledge("nobody", FIRST, 10, 300)
            .expect("an acknowledgement of no outbox"),
        0
    );
}

/// An expiry deletes, oldest first and up to its limit, the entries appended before its moment;
/// writes a note for each entry a session sent, and makes that session due at once; closes an
/// outbox left holding nothing that nothing touched since its moment. A write of the session
/// takes its notes through the mark it gives, and a removal takes all of them.
pub fn an_old_entry_expires_and_its_sender_is_noted(harness: &impl Harness) {
    let store = harness.handle();
    let sender = lease(&store, "k");
    let version = store
        .put("k", b"sender", Some(90_000), 0, sender.fence)
        .expect("the sending session");
    store
        .append(&appends("phone", Some("k"), &[b"old 1", b"old 2"]), 1_000)
        .expect("two old appends");
    store
        .append(&appends("phone", None, &[b"old 3"]), 1_500)
        .expect("an old append no session sent");
    store
        .append(&appends("phone", Some("k"), &[b"young"]), 9_000)
        .expect("a young append");

    let first = store.expire("phone", 5_000, 1).expect("an expiry of one");
    assert_eq!(
        first
            .expired
            .iter()
            .map(|expired| (expired.client.as_str(), expired.entry.number))
            .collect::<Vec<_>>(),
        vec![("phone", 1)],
        "the oldest, and no more than the limit"
    );
    assert!(!first.closed);
    let rest = store
        .expire("phone", 5_000, 10)
        .expect("an expiry of the rest");
    assert_eq!(
        rest.expired
            .iter()
            .map(|expired| expired.entry.number)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(rest.expired[1].entry.sender, None);
    assert_eq!(held(&store, "phone"), vec![(4, b"young".to_vec())]);

    let notes = harness.handle().notes("k").expect("the sender's notes");
    assert_eq!(
        notes
            .iter()
            .map(|note| (note.client.as_str(), note.number, note.at_ms))
            .collect::<Vec<_>>(),
        vec![("phone", 1, 1_000), ("phone", 2, 1_000)],
        "a note for each entry the session sent, none for the one no session sent"
    );
    assert!(notes[0].id < notes[1].id);
    assert_eq!(notes[1].envelope, b"old 2".to_vec());
    assert_eq!(
        store
            .get("k")
            .expect("the sender")
            .and_then(|stored| stored.earliest_due),
        Some(0),
        "the sender is due at once, to be told"
    );

    let told = store
        .write(
            "k",
            &Write {
                notes_through: notes[0].id,
                ..Write::put(b"told of one", None, version, sender.fence)
            },
        )
        .expect("a write that takes the first note");
    assert_eq!(
        store
            .notes("k")
            .expect("the notes left")
            .iter()
            .map(|note| note.number)
            .collect::<Vec<_>>(),
        vec![2]
    );
    store
        .delete("k", told.version, sender.fence)
        .expect("the session removed");
    assert!(
        store.notes("k").expect("no notes").is_empty(),
        "a removal takes every note"
    );

    store
        .append(&appends("phone", None, &[b"later"]), 12_000)
        .expect("an append after the next expiry's moment");
    let last = store
        .expire("phone", 10_000, 10)
        .expect("an expiry of the young entry");
    assert_eq!(
        last.expired
            .iter()
            .map(|expired| expired.entry.number)
            .collect::<Vec<_>>(),
        vec![4]
    );
    assert!(!last.closed, "an outbox that holds an entry is not closed");
    assert!(
        store.notes("k").expect("no notes").is_empty(),
        "no note is written for a session the store no longer holds"
    );
    assert_eq!(
        store
            .acknowledge("phone", FIRST, 5, 12_000)
            .expect("the later entry acknowledged"),
        1
    );
    let touched = store
        .expire("phone", 11_000, 10)
        .expect("an expiry of an outbox touched after its moment");
    assert!(
        !touched.closed,
        "an outbox touched after the expiry's moment is not closed"
    );
    let closed = store
        .expire("phone", 20_000, 10)
        .expect("an expiry of an empty outbox");
    assert!(closed.expired.is_empty());
    assert!(closed.closed, "an empty outbox untouched since is closed");
    assert_eq!(store.outbox("phone").expect("no outbox"), None);
    let reopened = store
        .append(
            &[Append {
                client: "phone",
                incarnation: SECOND,
                sender: None,
                envelope: b"anew",
            }],
            30_000,
        )
        .expect("an append to a closed outbox");
    assert_eq!(
        (reopened[0].incarnation.as_str(), reopened[0].number),
        (SECOND, 1),
        "a closed outbox opens anew, in another incarnation"
    );
}

/// ADR 0109, D15: a batch's acknowledgements and writes are decided item by item, in order. An
/// acknowledgement deletes what it covers; a write fenced out appends nothing, so the next kept
/// append takes the number it would have taken; a kept write appends with its session; and an
/// acknowledgement of another incarnation deletes nothing.
pub fn a_batch_appends_and_acknowledges_item_by_item(harness: &impl Harness) {
    let store = harness.handle();
    let k1 = lease(&store, "k1");
    let k2 = lease(&store, "k2");
    let opening = appends("phone", Some("k1"), &[b"one", b"two"]);
    let opened = store
        .write(
            "k1",
            &Write {
                outbox: &opening,
                now_ms: 1_000,
                ..Write::put(b"sent two", None, 0, k1.fence)
            },
        )
        .expect("a write that opens the outbox");
    // The holder of k2 asks again: the first grant's writes are fenced out.
    let k2_again = lease(&store, "k2");
    let refused_appends = appends("phone", Some("k2"), &[b"refused"]);
    let kept_appends = appends("phone", Some("k1"), &[b"three"]);
    let decided = store
        .write_many(&[
            Batched::Acknowledge {
                client: "phone",
                incarnation: FIRST,
                through: 1,
                now_ms: 1_100,
            },
            Batched::Write {
                key: "k2",
                write: Write {
                    outbox: &refused_appends,
                    now_ms: 1_100,
                    ..Write::put(b"fenced", None, 0, k2.fence)
                },
            },
            Batched::Write {
                key: "k1",
                write: Write {
                    outbox: &kept_appends,
                    now_ms: 1_100,
                    ..Write::put(b"sent three", None, opened.version, k1.fence)
                },
            },
            Batched::Acknowledge {
                client: "phone",
                incarnation: SECOND,
                through: 3,
                now_ms: 1_100,
            },
        ])
        .expect("a batch the store could make");
    assert_eq!(decided.len(), 4, "{decided:?}");
    assert!(
        matches!(decided[0], Ok(Committed::Acknowledged(1))),
        "the acknowledgement of entry 1: {:?}",
        decided[0]
    );
    match &decided[1] {
        Err(WriteError::Conflict(conflict)) => assert_eq!(
            *conflict,
            Conflict::Fence {
                presented: k2.fence,
                current: Some(k2_again.fence)
            }
        ),
        other => panic!("a fenced-out item of a batch gave {other:?}"),
    }
    match &decided[2] {
        Ok(Committed::Written(written)) => {
            assert_eq!(
                numbers(&written.appended),
                vec![3],
                "a refused item of the batch took a number"
            );
            assert_eq!(written.appended[0].incarnation, FIRST);
        }
        other => panic!("a kept item of a batch gave {other:?}"),
    }
    assert!(
        matches!(decided[3], Ok(Committed::Acknowledged(0))),
        "an acknowledgement of another incarnation: {:?}",
        decided[3]
    );
    assert_eq!(
        held(&harness.handle(), "phone"),
        vec![(2, b"two".to_vec()), (3, b"three".to_vec())],
        "what the batch kept of the outbox, for every handle"
    );
    assert_eq!(
        store.get("k2").expect("the fenced-out session"),
        None,
        "a refused item wrote its session"
    );
}

fn conflict<T: std::fmt::Debug>(result: Result<T, WriteError>) -> Conflict {
    match result {
        Err(WriteError::Conflict(conflict)) => conflict,
        other => panic!("a write that should have been refused gave {other:?}"),
    }
}
