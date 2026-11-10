// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A change is stable once every member of the list has been told it (docs/adr/0085, sections 3,
// 3.1 and 4): the stable cut is the pointwise minimum, over the members, of what each was told,
// read from the causal context of each member's latest operation, and a member that has written
// nothing since it was told more says so by a mark, an operation of its chain that carries no
// change and that no machine is told. A host folds its journal at the cut.
//
// Before the cut, a host learned that an instant was settled from checkpoints of a hash every
// writer agreed on, and no host could say which changes every member had been told.

use sce_rust_mesh::endpoint::Admission;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::message::{CONTEXT_FIELD, MARK_EVENT};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;
/// A reader: a member who writes nothing.
const DAVE: u64 = 4;

const CHANGE_EVENT: &str = "record_list.change";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// The log of the list: its genesis, alice, bob and carol granted writing, and dave reading.
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
    for (node, role) in [(ALICE, 2), (BOB, 2), (CAROL, 2), (DAVE, 1)] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role,
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

/// A host of the list whose person is `node`, holding the whole log.
fn host(node: u64) -> Trust {
    let host = Trust::open(LIST);
    for entry in entries() {
        host.append(&entry).unwrap();
    }
    host.set_identity(node, person(node));
    host
}

/// A change of `node`'s stamped at `wall` that writes row 1, field 3.
fn write(node: u64, wall: i64) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": 1, "field": 3, "value": "1", "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

/// `host`'s person signs `data`, a new operation of its chain.
fn sign(host: &Trust, data: &str) {
    let sealed = host
        .seal_sent(CHANGE_EVENT, data)
        .unwrap()
        .expect("a change is signed");
    assert!(sealed.signed, "{data} is a new operation");
}

/// Give `to` every operation `from` holds, and tell its machine what it may be told.
fn carry(from: &Trust, to: &Trust) {
    take_all(from, to);
    to.deliver();
}

/// Give `to` every operation `from` holds, and tell its machine nothing yet.
fn take_all(from: &Trust, to: &Trust) {
    for (event, data) in from.all_operations() {
        to.take_operation(&event, &data).unwrap();
    }
}

/// The cut is the pointwise minimum over the members of what each was told: a member of which a
/// host holds no operation holds every column at 0, a member's latest operation says what it was
/// told, and a column moves only once the last member is known to have been told it. Red if the
/// minimum becomes a maximum, or if a member whose vector is unknown is left out.
#[test]
fn the_cut_is_the_least_any_member_was_told() {
    let alice = host(ALICE);
    let bob = host(BOB);
    let carol = host(CAROL);
    let dave = host(DAVE);
    sign(&alice, &write(ALICE, 1000));
    carry(&alice, &bob);
    assert!(
        alice.stable_cut().is_empty(),
        "no other member is known to have been told anything: {:?}",
        alice.stable_cut()
    );
    sign(&bob, &write(BOB, 1100));
    carry(&bob, &alice);
    carry(&bob, &carol);
    carry(&bob, &dave);
    assert!(
        alice.stable_cut().is_empty(),
        "carol and dave were told both and signed nothing: {:?}",
        alice.stable_cut()
    );
    sign(&carol, &write(CAROL, 1200));
    assert_eq!(dave.sign_mark(), Some(1), "dave was told two changes");
    carry(&carol, &alice);
    carry(&dave, &alice);
    // Bob's change names alice 1; carol's names alice 1 and bob 1; dave's mark names both; alice
    // was told everything. Carol's own change is known to have been told to carol and alice only.
    assert_eq!(alice.stable_cut(), vec![(ALICE, 1), (BOB, 1)]);
    carry(&alice, &bob);
    carry(&alice, &carol);
    sign(&bob, &write(BOB, 1300));
    carry(&bob, &alice);
    assert_eq!(
        alice.stable_cut(),
        vec![(ALICE, 1), (BOB, 1)],
        "bob's second change is held by bob and alice alone, and dave was not told carol's"
    );
}

/// A member that writes nothing moves the cut by its mark alone: the mark is an operation of its
/// chain whose context is what its machine was told, every host takes it, a reader's included, no
/// machine is told it, and a mark is owed for a change told and not for a mark told. Red if a
/// received mark is not read as that member's latest, if a machine is told a mark, if a reader's
/// mark is refused, or if a mark is owed for a mark.
#[test]
fn a_member_that_writes_nothing_moves_the_cut_by_its_mark() {
    let alice = host(ALICE);
    let bob = host(BOB);
    let carol = host(CAROL);
    let dave = host(DAVE);
    sign(&alice, &write(ALICE, 1000));
    for other in [&bob, &carol, &dave] {
        carry(&alice, other);
    }
    sign(&bob, &write(BOB, 1100));
    sign(&carol, &write(CAROL, 1200));
    carry(&bob, &alice);
    carry(&carol, &alice);
    assert!(
        alice.stable_cut().is_empty(),
        "dave is not known to have been told anything"
    );

    assert_eq!(dave.sign_mark(), Some(1), "dave was told alice's change");
    assert_eq!(dave.sign_mark(), None, "nothing more was told since");
    let (event, mark) = dave
        .all_operations()
        .into_iter()
        .find(|(event, _)| event == MARK_EVENT)
        .expect("dave's mark is an operation of his chain");
    let carried: Value = serde_json::from_str(&mark).unwrap();
    assert_eq!(carried[CONTEXT_FIELD], json!([["1", "1"]]));

    take_all(&dave, &alice);
    assert!(alice.deliver().is_empty(), "no machine is told a mark");
    assert!(alice.delivered().contains(&(DAVE, 1)));
    assert_eq!(
        alice.stable_cut(),
        vec![(ALICE, 1)],
        "every member is known to have been told alice's change"
    );

    // Bob is told dave's mark and nothing else new: he owes no mark for it.
    carry(&dave, &bob);
    assert!(bob.delivered().contains(&(DAVE, 1)));
    assert_eq!(
        bob.sign_mark(),
        None,
        "a mark is owed for a change, not a mark"
    );

    // A mark is the trust layer's: no machine sends one, and a copy is told none.
    assert!(alice.seal_sent(MARK_EVENT, r#"{"by":"1"}"#).is_err());
    let mut admission = alice.clone();
    assert!(admission.admit("dave", &event, &mark).is_err());
}
