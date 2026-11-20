// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every implementation of [`AuditStore`] is held to (ADR 0109, D14).
//!
//! The contract is the documentation of [`AuditStore`]; these functions are that documentation as
//! histories with the answers a store must give. The two stores of this crate run them
//! (`tests/every_audit_store_keeps_one_contract.rs`), a sealing store runs them over both, and a
//! store written elsewhere runs the same functions from its own `#[test]`s, as with
//! [`crate::conformance`]. The chains here are bytes the functions choose: the store keeps them
//! and computes none.
//!
//! * [`rows_are_appended_one_past_the_head`]: a row is appended only one past its stream's head,
//!   and read back in order, a page at a time.
//! * [`a_checkpoint_is_kept_with_its_row_or_on_its_own`]: a checkpoint is kept by the append of its
//!   row or by itself, once for a stream and a sequence.
//! * [`an_append_repeated_under_its_identity_answers_its_first_sequence`]: the identity of an
//!   append is kept with its row, and a repeat appends nothing.
//! * [`a_segment_is_deleted_whole_up_to_a_checkpoint`]: retention deletes rows only through a
//!   sequence a checkpoint is kept at, keeps that checkpoint, and leaves the head.
//! * [`streams_are_kept_apart`]: each stream has its own head, rows and checkpoints.
//! * [`handles_share_the_log`]: what one handle appended, another reads.
//! * [`an_entry_and_a_signature_are_replaced_only_from_what_they_hold`] (ADR 0109, D8): the
//!   streams are listed a page at a time, and an entry or a signature is replaced only from the
//!   bytes it holds, its chain kept.
//! * [`a_removal_of_a_session_deletes_its_identities_and_no_row`]: for a store that keeps the
//!   sessions too, the removal of a session deletes the identities its sessions appended under,
//!   and keeps every row.
//!
//! Each function needs a harness whose store holds nothing when it starts.

use crate::audit::{AuditAppend, AuditCheckpoint, AuditHead, AuditRow, AuditStore, Chain};
use crate::kv::Identity;
use crate::{SessionStore, Write, LATEST_MOMENT};

/// What gives the functions of this module their store.
pub trait AuditHarness {
    type Store: AuditStore;

    /// Another handle on the one store this harness holds: it sees everything every other handle
    /// wrote. For a store in a database, a connection of its own, as a second process would have.
    fn handle(&self) -> Self::Store;
}

/// A chain the functions choose for row `sequence` of a stream: its bytes are the stream's first
/// byte and the sequence, so two rows never share one.
fn chain(stream: &str, sequence: u64) -> Chain {
    let mut chain = [0u8; 32];
    chain[0] = stream.as_bytes().first().copied().unwrap_or(0);
    chain[24..].copy_from_slice(&sequence.to_be_bytes());
    chain
}

/// Row `sequence` of `stream`, appended at `at_ms`.
fn row(stream: &str, sequence: u64, at_ms: u64) -> AuditRow {
    AuditRow {
        stream: stream.to_string(),
        sequence,
        at_ms,
        entry: format!("{{\"n\":{sequence}}}").into_bytes(),
        chain: chain(stream, sequence),
    }
}

/// Append `row` with no checkpoint and no identity, expecting it appended.
fn appended(store: &impl AuditStore, row: &AuditRow) {
    assert_eq!(
        store.audit_append(row, None, None).expect("an append"),
        AuditAppend::Appended,
        "row {} of {} follows its head",
        row.sequence,
        row.stream
    );
}

/// The sequences of `stream`'s rows past `after`.
fn sequences(store: &impl AuditStore, stream: &str, after: u64) -> Vec<u64> {
    store
        .audit_rows(stream, after, usize::MAX)
        .expect("the rows")
        .iter()
        .map(|row| row.sequence)
        .collect()
}

/// A row is appended only one past its stream's head, and read back in order.
pub fn rows_are_appended_one_past_the_head(harness: &impl AuditHarness) {
    let store = harness.handle();
    assert_eq!(
        store.audit_head("s").expect("a head"),
        AuditHead::EMPTY,
        "a stream nothing was appended to"
    );
    let first = row("s", 1, 1_000);
    appended(&store, &first);
    let head = AuditHead {
        sequence: 1,
        chain: chain("s", 1),
    };
    assert_eq!(store.audit_head("s").expect("the head"), head);
    assert_eq!(
        store
            .audit_append(&row("s", 1, 1_001), None, None)
            .expect("a second row 1"),
        AuditAppend::Moved { head },
        "a number already appended"
    );
    assert_eq!(
        store
            .audit_append(&row("s", 3, 1_001), None, None)
            .expect("a row past a gap"),
        AuditAppend::Moved { head },
        "a row that does not follow the head"
    );
    for sequence in 2..=5 {
        appended(&store, &row("s", sequence, 1_000 + sequence));
    }
    assert_eq!(
        store.audit_rows("s", 0, usize::MAX).expect("every row")[0],
        first,
        "a row is read back as it was appended"
    );
    assert_eq!(sequences(&store, "s", 0), [1, 2, 3, 4, 5]);
    assert_eq!(sequences(&store, "s", 3), [4, 5]);
    let page: Vec<u64> = store
        .audit_rows("s", 1, 2)
        .expect("a page")
        .iter()
        .map(|row| row.sequence)
        .collect();
    assert_eq!(page, [2, 3], "a page of two after 1");
    assert!(store
        .audit_rows("s", 5, 10)
        .expect("past the head")
        .is_empty());
}

/// A checkpoint is kept by the append of its row or by itself, once for a stream and a sequence.
pub fn a_checkpoint_is_kept_with_its_row_or_on_its_own(harness: &impl AuditHarness) {
    let store = harness.handle();
    appended(&store, &row("s", 1, 10));
    assert_eq!(
        store
            .audit_append(&row("s", 2, 20), Some(b"signature at 2"), None)
            .expect("an append with its checkpoint"),
        AuditAppend::Appended
    );
    appended(&store, &row("s", 3, 30));
    let at_3 = AuditCheckpoint {
        stream: "s".to_string(),
        sequence: 3,
        chain: chain("s", 3),
        signature: b"signature at 3".to_vec(),
    };
    assert!(store
        .audit_checkpoint(&at_3)
        .expect("a checkpoint of the head"));
    assert!(
        !store
            .audit_checkpoint(&AuditCheckpoint {
                signature: b"another".to_vec(),
                ..at_3.clone()
            })
            .expect("a second checkpoint at 3"),
        "one checkpoint for a stream and a sequence"
    );
    assert_eq!(
        store.audit_checkpoints("s").expect("the checkpoints"),
        [
            AuditCheckpoint {
                stream: "s".to_string(),
                sequence: 2,
                chain: chain("s", 2),
                signature: b"signature at 2".to_vec(),
            },
            at_3,
        ],
        "in the order of their sequences, the first kept at 3"
    );
    assert_eq!(
        store
            .audit_append(&row("s", 3, 40), Some(b"refused"), None)
            .expect("a row that does not follow"),
        AuditAppend::Moved {
            head: AuditHead {
                sequence: 3,
                chain: chain("s", 3)
            }
        }
    );
    assert_eq!(
        store.audit_checkpoints("s").expect("the checkpoints").len(),
        2,
        "a refused append keeps no checkpoint"
    );
}

/// The identity of an append is kept with its row, and a repeat appends nothing.
pub fn an_append_repeated_under_its_identity_answers_its_first_sequence(
    harness: &impl AuditHarness,
) {
    let store = harness.handle();
    let start = Identity {
        session: "session-1",
        invoke: "audit",
        start: 1,
    };
    assert_eq!(store.audit_first(&start).expect("no identity"), None);
    assert_eq!(
        store
            .audit_append(&row("s", 1, 10), None, Some(&start))
            .expect("the first start"),
        AuditAppend::Appended
    );
    assert_eq!(store.audit_first(&start).expect("its identity"), Some(1));
    assert_eq!(
        store
            .audit_append(&row("s", 2, 20), Some(b"signature"), Some(&start))
            .expect("the start repeated"),
        AuditAppend::Repeated { sequence: 1 },
        "the first sequence, and nothing appended"
    );
    assert_eq!(sequences(&store, "s", 0), [1]);
    assert!(store.audit_checkpoints("s").expect("none").is_empty());
    let next = Identity { start: 2, ..start };
    assert_eq!(
        store
            .audit_append(&row("s", 2, 20), None, Some(&next))
            .expect("the next start"),
        AuditAppend::Appended
    );
    assert_eq!(
        store
            .audit_append(
                &row("s", 2, 30),
                None,
                Some(&Identity { start: 3, ..start })
            )
            .expect("a start that lost the head"),
        AuditAppend::Moved {
            head: AuditHead {
                sequence: 2,
                chain: chain("s", 2)
            }
        }
    );
    assert_eq!(
        store
            .audit_first(&Identity { start: 3, ..start })
            .expect("no identity"),
        None,
        "a refused append keeps no identity"
    );
}

/// Retention deletes rows only through a sequence a checkpoint is kept at.
pub fn a_segment_is_deleted_whole_up_to_a_checkpoint(harness: &impl AuditHarness) {
    let store = harness.handle();
    for sequence in 1..=6 {
        let signature = (sequence % 2 == 0).then_some(&b"signature"[..]);
        assert_eq!(
            store
                .audit_append(&row("s", sequence, sequence * 100), signature, None)
                .expect("an append"),
            AuditAppend::Appended
        );
    }
    assert!(
        store.audit_drop_through("s", 3).is_err(),
        "no checkpoint is kept at 3"
    );
    assert_eq!(
        sequences(&store, "s", 0),
        [1, 2, 3, 4, 5, 6],
        "nothing deleted"
    );
    assert_eq!(
        store.audit_drop_through("s", 4).expect("a segment"),
        4,
        "rows 1 to 4"
    );
    assert_eq!(sequences(&store, "s", 0), [5, 6]);
    let kept: Vec<u64> = store
        .audit_checkpoints("s")
        .expect("the checkpoints")
        .iter()
        .map(|checkpoint| checkpoint.sequence)
        .collect();
    assert_eq!(
        kept,
        [4, 6],
        "the checkpoint at the cut anchors what remains"
    );
    assert_eq!(
        store.audit_head("s").expect("the head").sequence,
        6,
        "the head outlives retention"
    );
    appended(&store, &row("s", 7, 700));
    assert_eq!(
        store
            .audit_drop_through("s", 4)
            .expect("the same cut again"),
        0,
        "nothing left before it"
    );
}

/// Each stream has its own head, rows and checkpoints.
pub fn streams_are_kept_apart(harness: &impl AuditHarness) {
    let store = harness.handle();
    appended(&store, &row("a", 1, 1));
    appended(&store, &row("b", 1, 2));
    assert_eq!(
        store
            .audit_append(&row("a", 2, 3), Some(b"of a"), None)
            .expect("a checkpoint of a"),
        AuditAppend::Appended
    );
    assert_eq!(sequences(&store, "a", 0), [1, 2]);
    assert_eq!(sequences(&store, "b", 0), [1]);
    assert_eq!(store.audit_head("b").expect("b's head").sequence, 1);
    assert!(store.audit_checkpoints("b").expect("b's").is_empty());
    assert!(
        store.audit_drop_through("b", 2).is_err(),
        "a's checkpoint anchors nothing of b"
    );
    assert_eq!(store.audit_head("c").expect("c's head"), AuditHead::EMPTY);
}

/// What one handle appended, another reads.
pub fn handles_share_the_log(harness: &impl AuditHarness) {
    let writer = harness.handle();
    let reader = harness.handle();
    let start = Identity {
        session: "s",
        invoke: "i",
        start: 1,
    };
    assert_eq!(
        writer
            .audit_append(&row("s", 1, 5), Some(b"signature"), Some(&start))
            .expect("an append"),
        AuditAppend::Appended
    );
    assert_eq!(reader.audit_head("s").expect("the head").sequence, 1);
    assert_eq!(sequences(&reader, "s", 0), [1]);
    assert_eq!(reader.audit_checkpoints("s").expect("one").len(), 1);
    assert_eq!(
        reader
            .audit_append(&row("s", 2, 6), None, Some(&start))
            .expect("the start repeated on the other handle"),
        AuditAppend::Repeated { sequence: 1 }
    );
}

/// ADR 0109, D8: the streams are listed a page at a time by the names the store keeps them under,
/// and a row's entry and a checkpoint's signature are replaced only from the bytes they hold,
/// every other column kept: a replacement made from bytes another one already replaced writes
/// nothing, so a change made in between is not overwritten.
pub fn an_entry_and_a_signature_are_replaced_only_from_what_they_hold(harness: &impl AuditHarness) {
    let store = harness.handle();
    let first = row("a", 1, 10);
    appended(&store, &first);
    assert_eq!(
        store
            .audit_append(&row("a", 2, 11), Some(b"signature at 2"), None)
            .expect("a row with its checkpoint"),
        AuditAppend::Appended
    );
    appended(&store, &row("b", 1, 12));

    let mut names = vec![store.audit_indexed("a"), store.audit_indexed("b")];
    names.sort();
    assert_eq!(
        store.audit_streams_after(None, 10).expect("every stream"),
        names,
        "each stream by the name the store keeps it under, in the order of its bytes"
    );
    assert_eq!(
        store.audit_streams_after(None, 1).expect("a page of one"),
        names[..1].to_vec()
    );
    assert_eq!(
        store
            .audit_streams_after(Some(names[0].as_str()), 10)
            .expect("the page after the first"),
        names[1..].to_vec()
    );

    assert!(
        !store
            .audit_reseal_row("a", 1, b"other bytes", b"replaced")
            .expect("a replacement from bytes the row does not hold"),
        "replaced from bytes the row does not hold"
    );
    assert!(
        !store
            .audit_reseal_row("a", 9, &first.entry, b"replaced")
            .expect("a replacement of no row"),
        "a sequence the stream holds no row at"
    );
    assert!(store
        .audit_reseal_row("a", 1, &first.entry, b"replaced")
        .expect("a replacement from the bytes held"));
    assert!(
        !store
            .audit_reseal_row("a", 1, &first.entry, b"stale")
            .expect("a replacement from the bytes it held before"),
        "a replacement made from bytes already replaced is refused"
    );
    assert_eq!(
        store.audit_rows("a", 0, 1).expect("row 1")[0],
        AuditRow {
            entry: b"replaced".to_vec(),
            ..first
        },
        "the sequence, the moment and the chain are kept"
    );

    assert!(!store
        .audit_reseal_checkpoint("a", 2, b"other bytes", b"resigned")
        .expect("a replacement from bytes the checkpoint does not hold"));
    assert!(!store
        .audit_reseal_checkpoint("a", 1, b"signature at 2", b"resigned")
        .expect("a replacement of no checkpoint"));
    assert!(store
        .audit_reseal_checkpoint("a", 2, b"signature at 2", b"resigned")
        .expect("a replacement from the bytes held"));
    assert!(!store
        .audit_reseal_checkpoint("a", 2, b"signature at 2", b"stale")
        .expect("a replacement from the bytes it held before"));
    assert_eq!(
        store.audit_checkpoints("a").expect("a's checkpoints"),
        vec![AuditCheckpoint {
            stream: "a".to_string(),
            sequence: 2,
            chain: chain("a", 2),
            signature: b"resigned".to_vec(),
        }],
        "the sequence and the chain are kept"
    );

    assert_eq!(
        store.audit_head("a").expect("a's head"),
        AuditHead {
            sequence: 2,
            chain: chain("a", 2),
        },
        "a replacement leaves the head"
    );
    appended(&store, &row("a", 3, 13));
    assert_eq!(sequences(&store, "b", 0), [1], "b is untouched");
}

/// The removal of a session deletes the identities its sessions appended under, and keeps every
/// row.
pub fn a_removal_of_a_session_deletes_its_identities_and_no_row<H>(harness: &H)
where
    H: AuditHarness,
    H::Store: SessionStore,
{
    let store = harness.handle();
    let ended = Identity {
        session: "root-1",
        invoke: "audit",
        start: 1,
    };
    let other = Identity {
        session: "root-2",
        ..ended
    };
    assert_eq!(
        store
            .audit_append(&row("s", 1, 1), None, Some(&ended))
            .expect("an append of the session that ends"),
        AuditAppend::Appended
    );
    assert_eq!(
        store
            .audit_append(&row("s", 2, 2), None, Some(&other))
            .expect("an append of another"),
        AuditAppend::Appended
    );
    let lease = store
        .acquire("key-1", "owner", LATEST_MOMENT, 0)
        .expect("a lease");
    let version = store
        .write("key-1", &Write::put(b"state", None, 0, lease.fence))
        .expect("the session kept")
        .version;
    store
        .write(
            "key-1",
            &Write {
                sessions: &["root-1"],
                ..Write::delete(version, lease.fence)
            },
        )
        .expect("the removal");
    assert_eq!(store.audit_first(&ended).expect("the ended one's"), None);
    assert_eq!(
        store.audit_first(&other).expect("another session's"),
        Some(2)
    );
    assert_eq!(sequences(&store, "s", 0), [1, 2], "every row is kept");
}
