// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A device's host is given what its machine sends, and a machine sends a change again: an
// application that keeps its own log of commands builds the machine again from it, and the machine
// sends every change it makes as it made it the first time. What is asked: that the same change sent
// again, after the host restarted from its store too, leaves as the operation the host holds (its
// number, its signature, its sealed texts) and nothing is signed; that a change of the device under
// a dot it signed with another content is refused before anything is signed; and that a host whose
// state came back from a store older than the device's last operation signs nothing new until it
// holds the device's chain as far as a relay does, and then signs at the next number, where a host
// that did not wait signs a second operation at a number it had used and every host given both
// freezes the device. A change is one operation, whole (docs/adr/0085, section 1).

use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Head, Identity, Sealed, Sent, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const CHANGE: &str = "record_list.change";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

/// The log of a list in which alice (node 1) and bob (node 2) write, and that has begun `epochs`
/// epochs.
fn list(epochs: u32) -> Vec<Entry> {
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
    let mut entries = vec![genesis];
    for node in [1, 2] {
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
        entries.push(entry);
    }
    for epoch in 1..=epochs {
        let entry = scratch.draft(Body::Epoch { epoch }).sign(&admin()).finish();
        scratch.append(&entry).unwrap();
        entries.push(entry);
    }
    entries
}

/// A host that holds the log `entries`, for the person of `own` when there is one, with the key of
/// every epoch the log began.
fn host_in(entries: &[Entry], own: Option<u64>, epochs: u32) -> Trust {
    let trust = Trust::open(LIST);
    for entry in entries {
        trust.append(entry).unwrap();
    }
    if let Some(node) = own {
        trust.set_identity(node, person(node));
    }
    for epoch in 1..=epochs {
        trust.set_epoch_key(epoch, [5; 32]);
    }
    trust
}

/// One write of `value` and `text` to `field` of row `id`, as a change holds it.
fn put(id: u64, field: u64, value: i64, text: &str) -> Value {
    json!({"kind": 0, "id": id, "field": field, "value": value.to_string(), "text": text,
           "wallTime": "0", "counter": 0, "nodeId": "0"})
}

/// Alice's change stamped at `wall` of the operations `ops`.
fn change(wall: i64, ops: Vec<Value>) -> Value {
    json!({"by": ALICE.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": ops})
}

/// A change of alice's stamped at `wall`: a write of `value` and `text` to field 3 of row `id`.
fn write(id: u64, wall: i64, value: i64, text: &str) -> Value {
    change(wall, vec![put(id, 3, value, text)])
}

/// Alice's change stamped at `at` that writes over her write of row `id`, field 3, stamped at
/// `wall`, and writes nothing.
fn note(id: u64, wall: i64, at: i64) -> Value {
    change(
        at,
        vec![
            json!({"kind": 1, "id": id, "field": 3, "value": "0", "text": "",
                    "wallTime": wall.to_string(), "counter": 0, "nodeId": ALICE.to_string()}),
        ],
    )
}

fn sealed(host: &Trust, event: &str, data: &Value) -> Sealed {
    match host.seal_sent(event, &data.to_string()) {
        Ok(Some(sealed)) => sealed,
        other => panic!("expected {event} to leave, got {other:?}"),
    }
}

fn refused(host: &Trust, event: &str, data: &Value) -> String {
    match host.seal_sent(event, &data.to_string()) {
        Err(reason) => reason,
        other => panic!("expected {event} to be refused, got {other:?}"),
    }
}

fn parsed(data: &str) -> Value {
    serde_json::from_str(data).unwrap()
}

fn field(data: &str, name: &str) -> Value {
    parsed(data)[name].clone()
}

/// Alice's machine makes four changes: a write, a write, a change that writes the first over, and
/// a change of two writes; the host restarts from its store, and the machine, built again from her
/// commands, sends all of it again. Each leaves as the operation that was made, with its number and
/// its signature, and the chain does not grow.
fn sent_again_leaves_as_it_was_made(epochs: u32) {
    let entries = list(epochs);
    let alice = host_in(&entries, Some(ALICE), epochs);
    let sends = [
        (CHANGE, write(1, 1000, 10, "milk")),
        (CHANGE, write(2, 2000, 11, "bread")),
        (CHANGE, note(1, 1000, 2500)),
        (
            CHANGE,
            change(3000, vec![put(4, 3, 7, ""), put(4, 4, 8, "")]),
        ),
    ];
    let made: Vec<Sealed> = sends
        .iter()
        .map(|(event, data)| sealed(&alice, event, data))
        .collect();
    assert!(made.iter().all(|one| one.signed), "{made:?}");
    let numbers: Vec<Value> = made.iter().map(|one| field(&one.data, "seq")).collect();
    assert_eq!(
        numbers,
        vec![json!("1"), json!("2"), json!("3"), json!("4")]
    );
    if epochs > 0 {
        assert!(
            !made[0].data.contains("milk")
                && field(&made[0].data, "ops")[0]["text"]
                    .as_str()
                    .unwrap()
                    .starts_with('~'),
            "the content is sealed: {}",
            made[0].data
        );
    }

    // Sent again on the same host, each is the operation it holds.
    for ((event, data), first) in sends.iter().zip(&made) {
        let again = sealed(&alice, event, data);
        assert!(!again.signed, "{event} was signed a second time");
        assert_eq!(parsed(&again.data), parsed(&first.data));
    }

    // The host restarts from its store, and the machine sends it all again.
    let restarted = host_in(&entries, Some(ALICE), epochs);
    assert_eq!(restarted.restore_from_text(&alice.to_text()), 4);
    for ((event, data), first) in sends.iter().zip(&made) {
        let again = sealed(&restarted, event, data);
        assert!(!again.signed, "{event} was signed again after a restart");
        assert_eq!(
            parsed(&again.data),
            parsed(&first.data),
            "the same number, signature and text"
        );
    }
    assert_eq!(restarted.last_seq(ALICE), 4);
    assert_eq!(restarted.signed_operations(), alice.signed_operations());

    // What it makes next is the next number.
    let next = sealed(&restarted, CHANGE, &write(3, 4000, 1, ""));
    assert!(next.signed);
    assert_eq!(field(&next.data, "seq"), json!("5"));
}

#[test]
fn a_write_or_a_note_sent_again_leaves_as_the_operation_that_was_made() {
    sent_again_leaves_as_it_was_made(0);
}

#[test]
fn a_sealed_write_sent_again_leaves_with_the_text_it_was_sealed_with() {
    sent_again_leaves_as_it_was_made(1);
}

/// A write of the device under a stamp it signed with another value or another text is the device
/// signing two things for one write: its host refuses it before anything is sealed or signed, says
/// so, and holds what it held. A restart changes nothing of it.
#[test]
fn another_content_under_a_stamp_the_device_signed_is_refused_and_nothing_is_signed() {
    let entries = list(1);
    let alice = host_in(&entries, Some(ALICE), 1);
    let first = sealed(&alice, CHANGE, &write(1, 1000, 10, "milk"));
    assert!(first.signed);

    for other in [write(1, 1000, 11, "milk"), write(1, 1000, 10, "bread")] {
        let reason = refused(&alice, CHANGE, &other);
        assert!(reason.contains("another content"), "{reason}");
        assert!(reason.contains("at 1"), "{reason}");
    }
    assert_eq!(alice.last_seq(ALICE), 1);
    assert_eq!(alice.signed_operations().len(), 1, "nothing was held of it");
    assert!(alice.equivocating().is_empty());

    let restarted = host_in(&entries, Some(ALICE), 1);
    restarted.restore_from_text(&alice.to_text());
    let reason = refused(&restarted, CHANGE, &write(1, 1000, 11, "milk"));
    assert!(reason.contains("another content"), "{reason}");
    assert_eq!(restarted.signed_operations().len(), 1);
    // The write it made is the one it holds, and what follows is numbered after it.
    assert!(!sealed(&restarted, CHANGE, &write(1, 1000, 10, "milk")).signed);
    let next = sealed(&restarted, CHANGE, &write(1, 2000, 11, "milk"));
    assert_eq!(field(&next.data, "seq"), json!("2"));
}

/// Alice's device signs four operations, and its store holds the first two: the device was stopped
/// before it saved again, or came back from a backup. The relay holds all four. A host that comes
/// back from that store and waits for its chain signs nothing new, still gives what it holds, and
/// signs at 5 once it holds 3 and 4; a host that does not wait signs a second operation at 3, which
/// every host that is given both holds as proof and freezes the device at.
#[test]
fn a_device_whose_state_came_back_signs_nothing_new_until_it_holds_its_chain_again() {
    let entries = list(0);
    let alice = host_in(&entries, Some(ALICE), 0);
    for n in 1..=2 {
        sealed(&alice, CHANGE, &write(n, 1000 + n as i64, 1, ""));
    }
    let older = alice.to_text();
    for n in 3..=4 {
        sealed(&alice, CHANGE, &write(n, 1000 + n as i64, 1, ""));
    }
    // What a relay that was given all four says of the device's chain.
    let published: Vec<Head> = alice.heads();
    assert_eq!(published[0].seq, 4);

    let restored = host_in(&entries, Some(ALICE), 0);
    assert_eq!(restored.restore_from_text(&older), 2);
    assert!(
        !restored.awaits_own_chain(),
        "a restore alone does not wait"
    );
    restored.await_own_chain();
    assert!(restored.awaits_own_chain());

    let reason = refused(&restored, CHANGE, &write(9, 5000, 1, ""));
    assert!(reason.contains("signs nothing new"), "{reason}");
    assert!(
        !sealed(&restored, CHANGE, &write(1, 1001, 1, "")).signed,
        "what it holds still leaves"
    );
    assert_eq!(restored.own_chain_behind(&published), Some((ALICE, 3, 4)));
    assert!(!restored.confirm_own_chain(&published), "it holds 2 of 4");
    assert!(restored.awaits_own_chain());

    for (event, data) in alice.operations(ALICE, 3, 4) {
        restored.take_operation(&event, &data).unwrap();
    }
    assert_eq!(restored.own_chain_behind(&published), None);
    assert!(restored.confirm_own_chain(&published));
    assert!(!restored.awaits_own_chain());
    let next = sealed(&restored, CHANGE, &write(9, 5000, 1, ""));
    assert!(next.signed);
    assert_eq!(field(&next.data, "seq"), json!("5"));

    let witness = host_in(&entries, None, 0);
    for (event, data) in alice.operations(ALICE, 1, 4) {
        witness.take_operation(&event, &data).unwrap();
    }
    witness.take_operation(CHANGE, &next.data).unwrap();
    assert!(witness.equivocating().is_empty());
    assert_eq!(witness.last_seq(ALICE), 5);

    // A host that did not wait signs at 3, and the device is frozen there.
    let careless = host_in(&entries, Some(ALICE), 0);
    careless.restore_from_text(&older);
    let forked = sealed(&careless, CHANGE, &write(9, 5000, 1, ""));
    assert_eq!(field(&forked.data, "seq"), json!("3"));
    let other = host_in(&entries, None, 0);
    for (event, data) in alice.operations(ALICE, 1, 4) {
        other.take_operation(&event, &data).unwrap();
    }
    other.take_operation(CHANGE, &forked.data).unwrap();
    assert_eq!(other.equivocating(), vec![(ALICE, 3)]);
}

/// A host with no person has no chain of its own to wait for.
#[test]
fn a_host_with_no_person_waits_for_nothing() {
    let hub = host_in(&list(0), None, 0);
    hub.await_own_chain();
    assert_eq!(hub.own_chain_behind(&[]), None);
    assert_eq!(hub.own_chain_walk(&[]), None);
    assert!(hub.confirm_own_chain(&[]));
    assert!(!hub.awaits_own_chain());
}

/// Where a restored device reads its own chain back from is the number after the last at which the
/// relay's heads and its host agree: past the host's highest when the relay names more, past the
/// relay's head when the relay names less (its head stops below a conflict or a gap, and it may
/// hold more), at the relay's head when the relay names there an operation the host does not hold,
/// and from the first when the relay names nothing of the device.
#[test]
fn a_restored_device_reads_its_chain_back_from_where_it_and_the_relay_agree() {
    let entries = list(0);
    let alice = host_in(&entries, Some(ALICE), 0);
    for n in 1..=4 {
        sealed(&alice, CHANGE, &write(n, 1000 + n as i64, 1, ""));
    }
    let older = alice.to_text();
    let restored = host_in(&entries, Some(ALICE), 0);
    restored.restore_from_text(&older);
    let at = |seq: u64| -> Vec<Head> {
        let id = alice.heads()[0].id;
        let id = if seq == 4 {
            id
        } else {
            // The id of the operation at `seq`, as the relay names it in a head stopped there.
            let restarted = host_in(&entries, Some(ALICE), 0);
            for (event, data) in alice.operations(ALICE, 1, seq) {
                restarted.take_operation(&event, &data).unwrap();
            }
            restarted.heads()[0].id
        };
        vec![Head {
            node: ALICE,
            seq,
            id,
        }]
    };

    // The relay names as much as the host holds, or more: from the number after the host's last.
    assert_eq!(restored.own_chain_walk(&at(4)), Some((ALICE, 5)));
    let shorter = host_in(&entries, Some(ALICE), 0);
    for (event, data) in alice.operations(ALICE, 1, 2) {
        shorter.take_operation(&event, &data).unwrap();
    }
    assert_eq!(shorter.own_chain_walk(&at(4)), Some((ALICE, 3)));
    // The relay's head stops below what the host holds: from the number after the head.
    assert_eq!(restored.own_chain_walk(&at(2)), Some((ALICE, 3)));
    // The relay names, at a number the host holds, an operation it does not hold there.
    let mut other = at(2);
    other[0].id = [7; 32];
    assert_eq!(restored.own_chain_walk(&other), Some((ALICE, 2)));
    // The relay names nothing of the device.
    assert_eq!(restored.own_chain_walk(&[]), Some((ALICE, 1)));
}

/// A host whose sends a courier carries keeps what its person may not sign yet. Alice's machine
/// makes two changes that write and one that writes the first over before her host holds any of
/// the log: each waits, in its order, the first sent again waits once, and another content under
/// its dot is refused at once. The host's text keeps what waits, and a host restarted from it holds
/// the same. Given the log, the host still holds no key of the epoch it is in, and what waits, and a
/// change made then, waits on; given the key, all of it is signed in its order at 1 to 4, and the
/// first sent again is the operation held.
#[test]
fn what_a_host_may_not_sign_yet_waits_and_is_signed_in_its_order() {
    let alice = Trust::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    let sends = [
        (CHANGE, write(1, 1000, 10, "milk")),
        (CHANGE, write(2, 2000, 11, "bread")),
        (CHANGE, note(1, 1000, 2500)),
    ];
    for (event, data) in &sends {
        match alice.keep_sent(event, &data.to_string()) {
            Sent::Waiting(reason) => assert!(reason.contains("has not begun"), "{reason}"),
            other => panic!("{event}: expected it to wait, got {other:?}"),
        }
    }
    assert!(matches!(
        alice.keep_sent(CHANGE, &sends[0].1.to_string()),
        Sent::Waiting(_)
    ));
    match alice.keep_sent(CHANGE, &write(1, 1000, 12, "tea").to_string()) {
        Sent::Refused(reason) => assert!(reason.contains("another content"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(alice.waiting(), 3);
    assert!(alice.sign_waiting().is_empty(), "nothing may be signed yet");
    assert!(alice.signed_operations().is_empty());

    let restarted = Trust::open(LIST);
    restarted.set_identity(ALICE, person(ALICE));
    assert_eq!(restarted.restore_from_text(&alice.to_text()), 0);
    assert_eq!(restarted.waiting(), 3, "what waits is in the host's text");

    for entry in &list(1) {
        restarted.append(entry).unwrap();
    }
    assert!(restarted.sign_waiting().is_empty(), "no key of epoch 1 yet");
    let later = write(2, 3000, 1, "");
    match restarted.keep_sent(CHANGE, &later.to_string()) {
        Sent::Waiting(reason) => assert!(reason.contains("no key of epoch 1"), "{reason}"),
        other => panic!("expected the write to wait, got {other:?}"),
    }

    restarted.set_epoch_key(1, [5; 32]);
    let waited = restarted.sign_waiting();
    let mut in_order: Vec<(String, String)> = sends
        .iter()
        .map(|(event, data)| (event.to_string(), data.to_string()))
        .collect();
    in_order.push((CHANGE.to_string(), later.to_string()));
    assert_eq!(waited.taken, in_order);
    assert!(waited.dropped.is_empty());
    assert_eq!(restarted.waiting(), 0);
    assert_eq!(restarted.last_seq(ALICE), 4);
    assert_eq!(
        restarted.keep_sent(CHANGE, &sends[0].1.to_string()),
        Sent::Held
    );
}

/// Alice's device signed two changes that write and one that writes over, and its store holds the
/// first alone: her machine, built again from her commands, sends all three again while the
/// restored host waits for her chain. The first is held; the two the store lacks wait, and say why.
/// Once her chain is read back, each that waited is the operation held: `sign_waiting` names them
/// held and none taken, and her chain does not grow.
#[test]
fn what_waited_and_turned_out_held_is_told_apart_from_what_was_signed() {
    let entries = list(1);
    let alice = host_in(&entries, Some(ALICE), 1);
    let sends = [
        (CHANGE, write(1, 1000, 10, "milk")),
        (CHANGE, write(2, 2000, 11, "bread")),
        (CHANGE, note(1, 1000, 2500)),
    ];
    assert!(sealed(&alice, sends[0].0, &sends[0].1).signed);
    let older = alice.to_text();
    for (event, data) in &sends[1..] {
        assert!(sealed(&alice, event, data).signed, "{event}");
    }
    let published = alice.heads();

    let restored = host_in(&entries, Some(ALICE), 1);
    assert_eq!(restored.restore_from_text(&older), 1);
    restored.await_own_chain();
    assert_eq!(
        restored.keep_sent(CHANGE, &sends[0].1.to_string()),
        Sent::Held
    );
    for (event, data) in &sends[1..] {
        assert!(
            matches!(
                restored.keep_sent(event, &data.to_string()),
                Sent::Waiting(_)
            ),
            "{event} waits: the store does not hold it"
        );
    }
    let early = restored.sign_waiting();
    assert!(early.is_empty(), "{early:?}");
    assert_eq!(early.waiting, 2);
    assert!(
        early
            .why_waiting
            .as_deref()
            .is_some_and(|why| why.contains("signs nothing new")),
        "{early:?}"
    );

    for (event, data) in alice.operations(ALICE, 2, 3) {
        restored.take_operation(&event, &data).unwrap();
    }
    assert!(restored.confirm_own_chain(&published));
    let waited = restored.sign_waiting();
    let in_order: Vec<(String, String)> = sends[1..]
        .iter()
        .map(|(event, data)| (event.to_string(), data.to_string()))
        .collect();
    assert_eq!(waited.held, in_order, "{waited:?}");
    assert!(
        waited.taken.is_empty(),
        "nothing new was signed: {waited:?}"
    );
    assert!(waited.dropped.is_empty(), "{waited:?}");
    assert_eq!((waited.waiting, waited.why_waiting.clone()), (0, None));
    assert_eq!(restored.waiting(), 0);
    assert_eq!(restored.last_seq(ALICE), 3);
    assert_eq!(restored.signed_operations(), alice.signed_operations());
}

/// What waits is refused, each with why, once the log the host holds says its person may not write:
/// alice is made a reader before her host signed what her machine sent. Nothing is signed, nothing
/// waits any more, and the same write sent again is refused at once.
#[test]
fn what_waited_is_refused_once_the_log_says_the_person_may_not_write() {
    let alice = Trust::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    let data = write(1, 1000, 10, "milk").to_string();
    assert!(matches!(alice.keep_sent(CHANGE, &data), Sent::Waiting(_)));
    for entry in &list(0) {
        alice.append(entry).unwrap();
    }
    let reader = alice
        .draft(Body::Grant {
            node: ALICE,
            role: 1,
            key: person(ALICE).public(),
            final_seq: Some(0),
        })
        .sign(&admin())
        .finish();
    alice.append(&reader).unwrap();

    let waited = alice.sign_waiting();
    assert!(waited.taken.is_empty());
    assert_eq!(waited.dropped.len(), 1, "{waited:?}");
    let (event, sent, why) = &waited.dropped[0];
    assert_eq!((event.as_str(), sent.as_str()), (CHANGE, data.as_str()));
    assert!(why.contains("may not write (role 1"), "{why}");
    assert_eq!(alice.waiting(), 0);
    assert!(alice.signed_operations().is_empty());
    assert!(matches!(alice.keep_sent(CHANGE, &data), Sent::Refused(_)));
}
