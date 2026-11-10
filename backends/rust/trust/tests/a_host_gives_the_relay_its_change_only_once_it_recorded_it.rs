// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A change a host keeps a journal of is signed once, as any change is, and is a record: the
// operation's number, its event and the data it travels with, which the host writes to its
// journal; no relay is given it, nor any operation of the chain after it, until the host says it
// recorded it (docs/adr/0083, section 3.1: the outbox, written ahead). A host that restarts gives
// its records back, and they are taken as its own, verified, before it signs anything
// (section 3.4). A send that never will be signed is withdrawn with every send after it
// (docs/adr/0093, section 6.2).
//
// Before the outbox, what a device signed went to the relay with the next round whether its host
// had recorded it or not, and a host that crashed before its commit had published a change its
// journal did not hold.

use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, OwnRecord, Sent, Trust};
use serde_json::json;

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;

const CHANGE_EVENT: &str = "record_list.change";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// The log of the list: its genesis, and alice and bob granted writing.
fn entries() -> Vec<Entry> {
    let scratch = Trust::open(LIST);
    let genesis = Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![admin().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin())
    .finish();
    scratch.append(&genesis).unwrap();
    let mut all = vec![genesis];
    for node in [ALICE, BOB] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role: 2,
                key: person(node).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        scratch.append(&entry).unwrap();
        all.push(entry);
    }
    all
}

/// A host of the list whose person is `node`, holding the whole log when `with_log`.
fn host(node: u64, with_log: bool) -> Trust {
    let host = Trust::open(LIST);
    if with_log {
        for entry in entries() {
            host.append(&entry).unwrap();
        }
    }
    host.set_identity(node, person(node));
    host
}

/// A change of `node`'s stamped at `wall` that writes `value` into row `row`, field 3.
fn write(node: u64, row: u64, wall: i64, value: i64) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": row, "field": 3, "value": value.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

fn as_pairs(records: &[OwnRecord]) -> Vec<(String, String)> {
    records
        .iter()
        .map(|record| (record.event.clone(), record.data.clone()))
        .collect()
}

/// A change made for a journal is signed at once and held back as a record until the host
/// recorded it; a change sent with no journal is not; the record and what is held back survive
/// the host's text; and a host that restarts with an older state takes its records back as its
/// own, recorded already. Red if a record is not held back, if `recorded` frees nothing, or if the
/// text loses what is held back.
#[test]
fn a_record_is_held_back_until_the_host_recorded_it() {
    let alice = host(ALICE, true);
    assert_eq!(
        alice.keep_sent(CHANGE_EVENT, &write(ALICE, 1, 1000, 1)),
        Sent::Signed
    );
    assert!(
        alice.unrecorded().is_empty(),
        "a send with no journal is no record"
    );
    assert_eq!(alice.released_through(), None);
    let older = alice.to_text();

    assert_eq!(alice.keep_made(&write(ALICE, 2, 1100, 2)), Sent::Signed);
    assert_eq!(alice.keep_made(&write(ALICE, 3, 1200, 3)), Sent::Signed);
    let records = alice.unrecorded();
    assert_eq!(
        records.iter().map(|record| record.seq).collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert!(records.iter().all(|record| record.event == CHANGE_EVENT));
    assert_eq!(
        as_pairs(&records),
        alice.operations(ALICE, 2, 3),
        "a record is the operation as the relay is given it"
    );
    assert_eq!(alice.released_through(), Some(1));
    assert_eq!(
        alice.keep_made(&write(ALICE, 2, 1100, 2)),
        Sent::Held,
        "a change made again is the operation held, and no new record"
    );

    let reread = host(ALICE, true);
    assert!(reread.restore_from_text(&alice.to_text()) >= 3);
    assert_eq!(
        reread.released_through(),
        Some(1),
        "what is held back is in the text"
    );

    alice.recorded(2);
    assert_eq!(alice.released_through(), Some(2));
    alice.recorded(3);
    assert_eq!(alice.released_through(), None);
    assert!(alice.unrecorded().is_empty());

    // Alice's host restarts from the state it saved before the two changes, and gives back what
    // its journal holds past it.
    let restarted = host(ALICE, true);
    assert_eq!(restarted.restore_from_text(&older), 1);
    assert_eq!(restarted.restore_records(&as_pairs(&records)), Ok(2));
    assert_eq!(restarted.last_seq(ALICE), 3);
    assert_eq!(
        restarted.released_through(),
        None,
        "a record given back is recorded"
    );
    assert!(
        restarted.deliver().is_empty(),
        "the machine made what its person signed, and is told none of it"
    );
    assert_eq!(restarted.restore_records(&as_pairs(&records)), Ok(0));
    assert_eq!(
        restarted.keep_sent(CHANGE_EVENT, &write(ALICE, 2, 1100, 2)),
        Sent::Held
    );
}

/// A host's journal holds its own person's operations alone: another's, given back as a record,
/// is refused. Red if the author of a record is not checked.
#[test]
fn a_record_of_another_device_is_refused() {
    let bob = host(BOB, true);
    assert_eq!(bob.keep_made(&write(BOB, 1, 1000, 1)), Sent::Signed);
    let alice = host(ALICE, true);
    match alice.restore_records(&as_pairs(&bob.unrecorded())) {
        Err(why) => assert!(why.contains("node 2's"), "{why}"),
        other => panic!("bob's record was taken as alice's: {other:?}"),
    }
}

/// A change made for a journal that waited and is signed by a round is a record too, held back
/// as one made at once is. Red if a waiting send forgets that its host keeps a journal of it.
#[test]
fn a_change_that_waited_is_a_record_once_it_is_signed() {
    let alice = host(ALICE, false);
    let made = write(ALICE, 1, 1000, 1);
    assert!(matches!(alice.keep_made(&made), Sent::Waiting(_)));
    let reread = host(ALICE, false);
    reread.restore_from_text(&alice.to_text());
    for entry in entries() {
        reread.append(&entry).unwrap();
    }
    let waited = reread.sign_waiting();
    assert_eq!(waited.taken, vec![(CHANGE_EVENT.to_string(), made)]);
    assert_eq!(
        reread
            .unrecorded()
            .iter()
            .map(|record| record.seq)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(reread.released_through(), Some(0));
}

/// A send that never will be signed is withdrawn with every send after it, in order, and none of
/// them is signed (docs/adr/0093, section 6.2): the second of three goes back along the chain the
/// first makes, and the third, which would be signed alone, was made on a state that held the
/// second. Red if the send that cannot be signed is withdrawn alone.
#[test]
fn a_send_that_never_will_be_signed_withdraws_every_send_after_it() {
    let alice = host(ALICE, false);
    let sends = [
        write(ALICE, 1, 1000, 1),
        write(ALICE, 2, 500, 2),
        write(ALICE, 3, 3000, 3),
    ];
    for data in &sends {
        assert!(matches!(alice.keep_made(data), Sent::Waiting(_)), "{data}");
    }
    for entry in entries() {
        alice.append(&entry).unwrap();
    }
    let waited = alice.sign_waiting();
    assert_eq!(
        waited.taken,
        vec![(CHANGE_EVENT.to_string(), sends[0].clone())]
    );
    let withdrawn: Vec<&str> = waited
        .dropped
        .iter()
        .map(|(_, data, _)| data.as_str())
        .collect();
    assert_eq!(withdrawn, vec![sends[1].as_str(), sends[2].as_str()]);
    assert!(
        waited.dropped[0].2.contains("goes back"),
        "{:?}",
        waited.dropped
    );
    assert!(
        waited.dropped[1].2.contains("withdrawn"),
        "{:?}",
        waited.dropped
    );
    assert_eq!((waited.waiting, alice.waiting()), (0, 0));
    assert_eq!(alice.last_seq(ALICE), 1);
}
