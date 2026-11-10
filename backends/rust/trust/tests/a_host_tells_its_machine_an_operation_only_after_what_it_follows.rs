// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A host tells its machine each operation of a device once, whole, and only after every operation
// it follows: the ones before it in its own chain, and the ones of other devices its author's
// machine had been told when its author signed it (its causal context, signed in its link). This
// is causal delivery (docs/adr/0085, section 3; Birman, Schiper and Stephenson, ACM TOCS 9(3),
// 1991), the rule the shared list's effect without tombstones rests on: a write-over is never told
// before the write it names, so the write cannot come back after it.
//
// Before the rule, a host told its machine every operation it held as it took it in, in the order
// it was given, so each test below that gives operations out of their causal order failed: the
// machine was told a write-over before its write, or an operation past a gap in its chain.

use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::message::{self, Link, SIGNATURE_FIELD};
use sce_rust_trust::{encoding, Identity, TakeError, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const CHANGE_EVENT: &str = "record_list.change";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// The log of the list: its genesis, and alice, bob and carol granted writing.
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
    for node in [ALICE, BOB, CAROL] {
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

/// A host of the list whose person is `node`, holding the whole log and nothing else.
fn host(node: u64) -> Trust {
    let host = Trust::open(LIST);
    for entry in entries() {
        host.append(&entry).unwrap();
    }
    host.set_identity(node, person(node));
    host
}

/// A change of `node`'s stamped at `wall` that writes `value` into row 1, field 3.
fn write(node: u64, wall: i64, value: i64) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": 1, "field": 3, "value": value.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

/// A change of `by`'s that writes over the write `node` stamped at `wall`, made after its machine
/// was told it: stamped at the same time, a counter past it.
fn supersede(node: u64, wall: i64, by: u64) -> String {
    json!({"by": by.to_string(), "wallTime": wall.to_string(), "counter": 1, "ops": [
        {"kind": 1, "id": 1, "field": 3, "value": "0", "text": "",
         "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()}]})
    .to_string()
}

/// What `host`'s person signs for `data`, as the operation travels.
fn signed(host: &Trust, event: &str, data: &str) -> String {
    let sealed = host
        .seal_sent(event, data)
        .unwrap()
        .expect("the event is signed");
    assert!(sealed.signed, "{event} {data} is a new operation");
    sealed.data
}

fn take(host: &Trust, event: &str, data: &str) {
    host.take_operation(event, data).unwrap();
}

/// The events a host's machine is told now, by their event and the change's time and author
/// (`wallTime/by`).
fn told(host: &Trust) -> Vec<(String, String)> {
    host.deliver()
        .into_iter()
        .map(|(event, data)| {
            let data: Value = serde_json::from_str(&data).unwrap();
            let by = data["by"].as_str().unwrap_or("");
            let wall = data["wallTime"].as_str().unwrap_or("");
            (event, format!("{wall}/{by}"))
        })
        .collect()
}

fn event(event: &str, what: &str) -> (String, String) {
    (event.to_string(), what.to_string())
}

/// A write-over signed after its author's machine was told the write it names carries that
/// write in its context; a host given the write-over first holds it, says what it lacks, and tells
/// its machine the two in their causal order once it holds the write; the write given again is
/// told no more. Red if a host tells what it takes as it takes it (the order the relay gave), or if
/// the context's check is taken away.
#[test]
fn a_write_over_is_told_only_after_the_write_it_names() {
    let alice = host(ALICE);
    let bob = host(BOB);
    let carol = host(CAROL);
    let a1 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1000, 1));
    assert_eq!(
        alice.delivered(),
        vec![(ALICE, 1)],
        "a machine is told none of what its own person signed"
    );
    assert!(told(&alice).is_empty());

    take(&bob, CHANGE_EVENT, &a1);
    assert_eq!(told(&bob), vec![event(CHANGE_EVENT, "1000/1")]);
    assert_eq!(bob.delivered(), vec![(ALICE, 1)]);
    let b1 = signed(&bob, CHANGE_EVENT, &supersede(ALICE, 1000, BOB));
    let carried: Value = serde_json::from_str(&b1).unwrap();
    assert_eq!(
        carried[message::CONTEXT_FIELD],
        json!([["1", "1"]]),
        "the write-over carries what its author's machine was told of alice"
    );

    take(&carol, CHANGE_EVENT, &b1);
    assert!(
        told(&carol).is_empty(),
        "a write-over given before its write is held"
    );
    assert_eq!(carol.lacking(), vec![(ALICE, 1, 1)]);
    take(&carol, CHANGE_EVENT, &a1);
    assert_eq!(
        told(&carol),
        vec![event(CHANGE_EVENT, "1000/1"), event(CHANGE_EVENT, "1000/2")]
    );
    assert!(carol.lacking().is_empty());
    take(&carol, CHANGE_EVENT, &a1);
    take(&carol, CHANGE_EVENT, &b1);
    assert!(
        told(&carol).is_empty(),
        "what the machine was told is not told again"
    );
    assert_eq!(carol.delivered(), vec![(ALICE, 1), (BOB, 1)]);
}

/// An operation past a gap in its device's chain is held until the operation before it arrives,
/// and the gap is what the host says it lacks. Red if a host tells the next operation it holds of a
/// device rather than the one after the last it told.
#[test]
fn an_operation_past_a_gap_in_its_chain_waits_for_the_gap() {
    let alice = host(ALICE);
    let carol = host(CAROL);
    let a1 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1000, 1));
    let a2 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1001, 2));
    let a3 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1002, 3));
    take(&carol, CHANGE_EVENT, &a3);
    take(&carol, CHANGE_EVENT, &a2);
    assert!(told(&carol).is_empty());
    assert_eq!(carol.lacking(), vec![(ALICE, 1, 1)]);
    take(&carol, CHANGE_EVENT, &a1);
    assert_eq!(
        told(&carol),
        vec![
            event(CHANGE_EVENT, "1000/1"),
            event(CHANGE_EVENT, "1001/1"),
            event(CHANGE_EVENT, "1002/1")
        ]
    );
}

/// A machine that lost what it was told is told every operation again, in causal order; and what
/// a host's machine was told survives the host's own text, so a host restored from it tells its
/// machine nothing twice. Red if forgetting keeps the vector, or if the text drops it.
#[test]
fn what_a_machine_was_told_is_kept_and_forgotten_as_a_whole() {
    let alice = host(ALICE);
    let bob = host(BOB);
    let a1 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1000, 1));
    take(&bob, CHANGE_EVENT, &a1);
    let b1 = signed(&bob, CHANGE_EVENT, &supersede(ALICE, 1000, BOB));
    let carol = host(CAROL);
    take(&carol, CHANGE_EVENT, &b1);
    take(&carol, CHANGE_EVENT, &a1);
    assert_eq!(told(&carol).len(), 2);

    let restored = host(CAROL);
    assert!(restored.restore_from_text(&carol.to_text()) >= 2);
    assert_eq!(restored.delivered(), vec![(ALICE, 1), (BOB, 1)]);
    assert!(restored.deliver().is_empty());

    carol.forget_delivered();
    assert!(carol.delivered().is_empty());
    assert_eq!(
        told(&carol),
        vec![event(CHANGE_EVENT, "1000/1"), event(CHANGE_EVENT, "1000/2")]
    );
}

/// An operation whose context names an operation of a device whose chain is frozen below it is
/// told once the machine was told that device up to where it is frozen: no machine is ever told
/// what lies past a freeze, and waiting for it would hold the operation, and all that follows it,
/// for good. Red if a context's entry is met only by the number it names.
#[test]
fn a_context_that_names_what_no_machine_is_told_does_not_hold_for_good() {
    let alice = host(ALICE);
    let bob = host(BOB);
    let a1 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1000, 1));
    // Bob's device signs a note, and then (set up again from his seed, holding nothing) a write
    // at the same number: two operations at one number, which freeze his chain from 1.
    take(&bob, CHANGE_EVENT, &a1);
    let b1 = signed(&bob, CHANGE_EVENT, &supersede(ALICE, 1000, BOB));
    let again = host(BOB);
    let b1_again = signed(&again, CHANGE_EVENT, &write(BOB, 1500, 9));
    // Alice's machine is told bob's note before anyone holds the second, and she writes after it.
    take(&alice, CHANGE_EVENT, &b1);
    assert_eq!(told(&alice), vec![event(CHANGE_EVENT, "1000/2")]);
    let a2 = signed(&alice, CHANGE_EVENT, &write(ALICE, 2000, 2));

    let carol = host(CAROL);
    take(&carol, CHANGE_EVENT, &a1);
    take(&carol, CHANGE_EVENT, &b1_again);
    take(&carol, CHANGE_EVENT, &b1);
    take(&carol, CHANGE_EVENT, &a2);
    assert_eq!(
        told(&carol),
        vec![event(CHANGE_EVENT, "1000/1"), event(CHANGE_EVENT, "2000/1")],
        "nothing of bob's frozen chain is told, and what follows it is not held for it"
    );
    assert!(carol.lacking().is_empty());
}

/// An operation whose context names its own author, or a device the log had not granted by the
/// entry the operation names, is refused: the first is said by its number already, and the second
/// would hold the operation waiting for good. Red if either check is taken away.
#[test]
fn a_context_names_only_devices_the_log_granted_other_than_its_author() {
    let carol = host(CAROL);
    let (next, log_hash) = carol.head();
    for (context, why) in [
        (vec![(ALICE, 1)], "its own author"),
        (vec![(7, 1)], "node 7"),
    ] {
        let value: Value = serde_json::from_str(&write(ALICE, 1000, 1)).unwrap();
        let signing = message::signing(LIST, CHANGE_EVENT, &value)
            .unwrap()
            .unwrap();
        let link = Link {
            seq: 1,
            prev: message::NO_OPERATION,
            log: next - 1,
            log_hash,
            context,
        };
        let bytes = message::operation_bytes(&signing.message, &link);
        let mut data = value.clone();
        link.write_into(&mut data).unwrap();
        data[SIGNATURE_FIELD] = Value::String(encoding::encode(&person(ALICE).sign(&bytes)));
        match carol.take_operation(CHANGE_EVENT, &data.to_string()) {
            Err(TakeError::Refused(reason)) => {
                assert!(reason.contains(why), "{why}: {reason}")
            }
            other => panic!("{why}: {other:?}"),
        }
    }
}

/// What a host lacks is read from what it holds, not from what its machine was told so far: a host
/// that holds a device's first and third operations, and has told its machine nothing yet (a round
/// tells it when it ends, after it asked for what it lacks), lacks the second at once, so a gap is
/// asked for in the round that first sees it. Red if `lacking` reads the delivered vector, which
/// would say the first is next and lacks nothing.
#[test]
fn a_gap_is_lacking_as_soon_as_the_operation_past_it_is_held() {
    let alice = host(ALICE);
    let carol = host(CAROL);
    let a1 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1000, 1));
    signed(&alice, CHANGE_EVENT, &write(ALICE, 1001, 2));
    let a3 = signed(&alice, CHANGE_EVENT, &write(ALICE, 1002, 3));
    take(&carol, CHANGE_EVENT, &a1);
    take(&carol, CHANGE_EVENT, &a3);
    assert!(carol.delivered().is_empty(), "nothing was told yet");
    assert_eq!(carol.lacking(), vec![(ALICE, 2, 2)]);
    assert_eq!(told(&carol), vec![event(CHANGE_EVENT, "1000/1")]);
    assert_eq!(
        carol.lacking(),
        vec![(ALICE, 2, 2)],
        "told or not, 2 is lacking"
    );
}

/// An eviction is a notice to the machine of what it was told and is to be without: a device's
/// operation a host held and never told its machine, frozen by a second operation at its number,
/// is no eviction, and the same operation told before the second came is one. Red if what is
/// evicted is read from what the host held rather than from what its machine was told.
#[test]
fn an_operation_never_told_is_never_evicted() {
    let alice = host(ALICE);
    let again = host(ALICE);
    let first = signed(&alice, CHANGE_EVENT, &write(ALICE, 1000, 1));
    let second = signed(&again, CHANGE_EVENT, &write(ALICE, 2000, 2));

    let untold = host(CAROL);
    take(&untold, CHANGE_EVENT, &first);
    take(&untold, CHANGE_EVENT, &second);
    assert_eq!(untold.equivocating(), vec![(ALICE, 1)]);
    assert!(
        untold.take_evicted().is_empty(),
        "its machine was told nothing of alice's"
    );
    assert!(
        told(&untold).is_empty(),
        "nothing of a frozen chain is told"
    );

    let told_first = host(BOB);
    take(&told_first, CHANGE_EVENT, &first);
    assert_eq!(told(&told_first), vec![event(CHANGE_EVENT, "1000/1")]);
    take(&told_first, CHANGE_EVENT, &second);
    assert_eq!(told_first.equivocating(), vec![(ALICE, 1)]);
    assert_eq!(
        told_first.take_evicted(),
        vec![(ALICE, 1)],
        "its machine was told the first, and is to be without it"
    );
}
