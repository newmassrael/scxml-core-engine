// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The device's side of a relay's `conflict` (ADR 0083 section 4.2). A courier given `conflict` for
// an operation it gave the relay takes in what the relay held and counts the operation as given.
// When the operation is of its own person and that froze the person's chain, the device signs
// nothing more, the round ends there and says so (`Round::conflicted`); when the chain was frozen
// there already, or the operation is another device's that the courier carried as proof, the round
// goes on. Each answer is one record, and a round that froze the person's chain one more.

use std::sync::{Arc, Mutex};

use sce_rust_relay::client::keep;
use sce_rust_relay::logs::{capture, list_digest};
use sce_rust_relay::{Authority, Courier, DirectLink, Kept, Limits, MemoryStore, Relay};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;

const CHANGE: &str = "record_list.change";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// The entries of the list: alice and bob write.
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

/// A host that holds the log, as the person of `own`.
fn host(own: u64) -> Trust {
    let trust = Trust::open(LIST);
    for entry in entries() {
        trust.append(&entry).unwrap();
    }
    trust.set_identity(own, person(own));
    trust
}

fn authority() -> Authority {
    Authority::new([7; 32], "relay-a")
}

/// A change of alice's stamped at `wall` of one write, to field 3 of row `id`.
fn write(id: u32, wall: i64) -> String {
    json!({"by": ALICE.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": id, "field": 3, "value": id.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

fn parsed(data: &str) -> Value {
    serde_json::from_str(data).unwrap()
}

type Shared = Arc<Mutex<Relay<MemoryStore>>>;

fn relay() -> Shared {
    Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority(), Limits::default()).unwrap(),
    ))
}

fn link(relay: &Shared, node: u64) -> DirectLink<MemoryStore> {
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    DirectLink::new(
        relay.clone(),
        authority().issue(LIST, node, u64::MAX),
        clock,
    )
}

/// What this thread logged since it last asked, each record as its line.
fn lines() -> Vec<String> {
    capture::taken()
        .into_iter()
        .map(|taken| taken.line)
        .collect()
}

fn count(lines: &[String], line: &str) -> usize {
    lines.iter().filter(|each| each.as_str() == line).count()
}

/// A device of alice's set up again from her seed with no state holds the relay's first two
/// operations of hers and signs a third and a fourth of its own, where the relay holds another
/// third. Its round gives the relay the fourth, which does not follow the relay's third, and is
/// answered `conflict`: its core takes in the relay's third, finds its own chain frozen at 3 as
/// every host will, ends the round there with nothing more given or read, says so, and signs
/// nothing after it. Its next round gives the relay the rest of the proof, is answered `conflict`
/// for its own third, goes on, and says nothing froze: the chain was frozen there already.
#[test]
fn a_device_told_its_own_operation_conflicts_freezes_at_once_and_says_so() {
    capture::start();
    let digest = list_digest(LIST);
    let relay = relay();
    let alice = host(ALICE);
    for n in 1..=3 {
        assert_eq!(
            keep(&alice, CHANGE, &write(n, 1000 + i64::from(n))),
            Kept::Taken
        );
    }
    Courier::new(alice.clone(), link(&relay, ALICE))
        .sync()
        .unwrap();
    let held = |from: u64, to: u64| alice.operations(ALICE, from, to);

    let copy = host(ALICE);
    for (event, data) in held(1, 2) {
        copy.take_operation(&event, &data).unwrap();
    }
    assert_eq!(keep(&copy, CHANGE, &write(7, 2007)), Kept::Taken);
    assert_eq!(keep(&copy, CHANGE, &write(8, 2008)), Kept::Taken);
    let before = relay.lock().unwrap().records(LIST);
    let _ = lines();

    let round = Courier::new(copy.clone(), link(&relay, ALICE))
        .sync()
        .unwrap();
    assert_eq!(round.conflicted, Some(3), "{round:?}");
    assert_eq!(copy.equivocating(), vec![(ALICE, 3)]);
    assert_eq!(
        round.pushed, 1,
        "the fourth, kept as proof, and nothing after it"
    );
    assert_eq!(
        round.evicted,
        vec![(ALICE, 3), (ALICE, 4)],
        "its machine was told its third and fourth, and is to be without them"
    );
    assert_eq!(
        relay.lock().unwrap().records(LIST),
        before + 1,
        "the round ended at the answer: its own third was not given"
    );
    let first = lines();
    let answered = format!(
        "event=conflict_answered list={digest} node={ALICE} seq=4 frozen=3 held=1 own=true \
         ends=true"
    );
    let frozen = format!("event=own_chain_frozen list={digest} node={ALICE} frozen=3 evicted=2");
    assert_eq!(count(&first, &answered), 1, "{first:#?}");
    assert_eq!(count(&first, &frozen), 1, "{first:#?}");
    assert!(
        !first.iter().any(|line| line.contains("op=feed")),
        "the round read no feed after it ended: {first:#?}"
    );
    // What it holds at 3 is its own third and the relay's.
    let ours = copy.operations(ALICE, 3, 3);
    assert_eq!(ours.len(), 2, "{ours:?}");
    let relays = parsed(&held(3, 3)[0].1);
    assert!(ours.iter().any(|(_, data)| parsed(data) == relays));
    // It signs nothing more.
    assert!(
        matches!(keep(&copy, CHANGE, &write(9, 2009)), Kept::Refused(_)),
        "a frozen chain signs nothing"
    );

    // The next round gives the relay its own third, the rest of the proof, and froze nothing new.
    let _ = lines();
    let again = Courier::new(copy.clone(), link(&relay, ALICE))
        .sync()
        .unwrap();
    assert_eq!(again.conflicted, None, "{again:?}");
    assert_eq!(relay.lock().unwrap().records(LIST), before + 2);
    let second = lines();
    let answered_again = format!(
        "event=conflict_answered list={digest} node={ALICE} seq=3 frozen=3 held=1 own=true \
         ends=false"
    );
    assert_eq!(count(&second, &answered_again), 1, "{second:#?}");
    assert!(
        !second
            .iter()
            .any(|line| line.starts_with("event=own_chain_frozen")),
        "{second:#?}"
    );
}

/// A courier that gives the relay the proof of another device's conflict, which the relay lacks,
/// is answered `conflict` for it, takes in what the relay held, counts it as given, says nothing
/// froze of its own device, and goes on with its round.
#[test]
fn a_courier_that_carries_another_devices_conflict_is_answered_conflict_and_goes_on() {
    capture::start();
    let digest = list_digest(LIST);
    let relay = relay();
    let alice = host(ALICE);
    assert_eq!(keep(&alice, CHANGE, &write(1, 1001)), Kept::Taken);
    Courier::new(alice.clone(), link(&relay, ALICE))
        .sync()
        .unwrap();
    // A copy of alice's that holds nothing signs another first operation, which reaches bob alone;
    // he holds alice's first too, and so the proof.
    let copy = host(ALICE);
    assert_eq!(keep(&copy, CHANGE, &write(5, 1005)), Kept::Taken);
    let bob = host(BOB);
    for (event, data) in alice
        .operations(ALICE, 1, 1)
        .into_iter()
        .chain(copy.operations(ALICE, 1, 1))
    {
        bob.take_operation(&event, &data).unwrap();
    }
    assert_eq!(bob.equivocating(), vec![(ALICE, 1)]);
    let before = relay.lock().unwrap().records(LIST);
    let _ = lines();

    let round = Courier::new(bob.clone(), link(&relay, BOB)).sync().unwrap();
    assert_eq!(round.conflicted, None, "{round:?}");
    assert_eq!(
        relay.lock().unwrap().records(LIST),
        before + 1,
        "the relay kept the proof it lacked"
    );
    assert_eq!(
        round.pushed, 1,
        "the copy's first, and nothing else: {round:?}"
    );
    let taken = lines();
    let answered = format!(
        "event=conflict_answered list={digest} node={ALICE} seq=1 frozen=1 held=1 own=false \
         ends=false"
    );
    assert_eq!(count(&taken, &answered), 1, "{taken:#?}");
    assert!(
        taken.iter().any(|line| line.contains("op=feed")),
        "the round went on to read the feed: {taken:#?}"
    );
    assert!(
        !taken
            .iter()
            .any(|line| line.starts_with("event=own_chain_frozen")),
        "{taken:#?}"
    );
}
