// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a host gives a machine built again is the history of the list (docs/adr/0083, section 2.2;
// docs/adr/0085, sections 3.1 and 7.3): every change it holds that a machine would be told, the
// device's own included, whole, in a causal order whatever order the host took them in, each with
// its tag read from its link (its author, its number and its causal context), from a stable cut a
// host folded its journal at; then what the device sent that waits to be signed, with no tag,
// which the machine takes in provisionally (docs/adr/0093, section 6.1). It moves nothing: the
// next round tells the machine what it would have told it.
//
// Before it, a host built a machine again by a resync round from the relay, which needs a
// network, or by its own log of commands run again, which makes new operations.

use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Sent, Tag, Told, Trust};
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

/// A host of the list whose person is `node`, holding the whole log.
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

/// A change of `by`'s that writes over the write `node` stamped at `wall`.
fn supersede(node: u64, wall: i64, by: u64) -> String {
    json!({"by": by.to_string(), "wallTime": wall.to_string(), "counter": 1, "ops": [
        {"kind": 1, "id": 1, "field": 3, "value": "0", "text": "",
         "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()}]})
    .to_string()
}

/// What `host`'s person signs for `data`, as the operation travels.
fn signed(host: &Trust, data: &str) -> String {
    let sealed = host
        .seal_sent(CHANGE_EVENT, data)
        .unwrap()
        .expect("a change is signed");
    assert!(sealed.signed, "{data} is a new operation");
    sealed.data
}

/// A change of a history as its time and author (`wallTime/by`) and its tag as author, number and
/// context; no tag for one that waits.
type Read = (String, Option<(u64, u64, Vec<(u64, u64)>)>);

/// A history as each change's time and author (`wallTime/by`) and its tag.
fn read(history: &[Told]) -> Vec<Read> {
    history
        .iter()
        .map(|told| {
            assert_eq!(told.event, CHANGE_EVENT);
            let data: Value = serde_json::from_str(&told.data).unwrap();
            let what = format!(
                "{}/{}",
                data["wallTime"].as_str().unwrap_or(""),
                data["by"].as_str().unwrap_or("")
            );
            let tag = told.tag.as_ref().map(
                |Tag {
                     author,
                     seq,
                     context,
                 }| (*author, *seq, context.clone()),
            );
            (what, tag)
        })
        .collect()
}

/// A host given a write-over before the write it names, and its own change after both, gives its
/// history in their causal order, each with the tag of its link, its own change included; the
/// history moves nothing, and the next delivery tells the machine what it would have. Red if the
/// history is given in the order the host took the changes in, or if it moves the vector.
#[test]
fn a_history_is_given_in_causal_order_with_the_tags_of_the_links() {
    let alice = host(ALICE);
    let bob = host(BOB);
    let carol = host(CAROL);
    let a1 = signed(&alice, &write(ALICE, 1000, 1));
    bob.take_operation(CHANGE_EVENT, &a1).unwrap();
    bob.deliver();
    let b1 = signed(&bob, &supersede(ALICE, 1000, BOB));

    carol.take_operation(CHANGE_EVENT, &b1).unwrap();
    carol.take_operation(CHANGE_EVENT, &a1).unwrap();
    let want: Vec<Read> = vec![
        ("1000/1".to_string(), Some((ALICE, 1, vec![]))),
        ("1000/2".to_string(), Some((BOB, 1, vec![(ALICE, 1)]))),
    ];
    assert_eq!(read(&carol.history(&[])), want);
    assert!(
        carol.delivered().is_empty(),
        "a history moves nothing of what the machine was told"
    );
    assert_eq!(carol.deliver().len(), 2);

    // Carol's own change, made after both, is in her history after them, with its context.
    signed(&carol, &write(CAROL, 2000, 3));
    let mut with_own = want;
    with_own.push((
        "2000/3".to_string(),
        Some((CAROL, 1, vec![(ALICE, 1), (BOB, 1)])),
    ));
    assert_eq!(read(&carol.history(&[])), with_own);
}

/// A history from a stable cut gives only what lies past it, and what the device sent that waits
/// to be signed comes last, with no tag, in the order it was sent: a copy recalls it as
/// provisional. Red if the cut is not where the history begins, or if what waits is left out.
#[test]
fn a_history_begins_at_its_cut_and_ends_with_what_waits() {
    let alice = host(ALICE);
    let carol = host(CAROL);
    let a1 = signed(&alice, &write(ALICE, 1000, 1));
    let a2 = signed(&alice, &write(ALICE, 1100, 2));
    for data in [&a1, &a2] {
        carol.take_operation(CHANGE_EVENT, data).unwrap();
    }
    assert_eq!(
        read(&carol.history(&[(ALICE, 1)])),
        vec![("1100/1".to_string(), Some((ALICE, 2, vec![])))]
    );
    assert!(carol.history(&[(ALICE, 2)]).is_empty());

    // A device whose state came back from a store keeps what it sends waiting.
    carol.await_own_chain();
    let waits = write(CAROL, 3000, 9);
    assert!(matches!(
        carol.keep_sent(CHANGE_EVENT, &waits),
        Sent::Waiting(_)
    ));
    let history = carol.history(&[(ALICE, 2)]);
    assert_eq!(history.len(), 1);
    assert!(history[0].provisional());
    assert_eq!(history[0].data, waits);
    assert_eq!(read(&history), vec![("3000/3".to_string(), None)]);
}
