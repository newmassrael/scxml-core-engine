// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The core of a device whose copy keeps a journal (docs/adr/0083, landing 3, at change units by
// docs/adr/0085, section 7.3): the change the copy offers (`record_list.made`) is signed as one
// operation in the call that takes it, and is a record the host writes to its journal, which no
// round gives the relay until the host says it recorded it; a device started again from an older
// state and its journal takes its records back and signs nothing; what a machine built again is
// given is the history, every change in causal order with its tag; a round that reads the
// device's own chain back past what the core held says the stamp rose; and the stable cut is what
// every member is known to have been told, moved by the marks of the members that write nothing.
//
// Before it, the core signed what the machine sent to `#people` and gave it the relay with the
// next round, whatever its host had recorded, and a rebuilt machine was told the list by a resync
// round from the relay.

use sce_rust_peer::{Kept, Outcome, OwnStamp, Peer, Profile, RoundKind, SealedState, Tag, Told};
use sce_rust_relay::{Authority, Limits, MemoryStore, Record, Relay, Request, Response};
use sce_rust_trust::membership::{Body, Entry, Log};
use sce_rust_trust::message::MARK_EVENT;
use sce_rust_trust::Identity;
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const CHANGE: &str = "record_list.change";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

/// The administrator's key, which alice's device holds.
fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn genesis() -> Entry {
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![administrator().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator())
    .finish()
}

/// What a copy offers its host for a change of one write of `value` to field 3 of row `id`,
/// stamped by `node` at `wall`, closed by the command `command`: `record_list.made`'s payload.
fn made(id: u32, node: u64, wall: i64, value: i64, command: u64) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": id, "field": 3, "value": value.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}],
        "command": command.to_string()})
    .to_string()
}

/// The change `made` offers, as `record_list.change` carries it.
fn change_of(offered: &str) -> String {
    let mut value: Value = serde_json::from_str(offered).unwrap();
    value.as_object_mut().unwrap().remove("command");
    value.to_string()
}

/// The relay, and the authority whose tokens it takes.
struct Network {
    relay: Relay<MemoryStore>,
    authority: Authority,
}

impl Network {
    fn new() -> Self {
        let authority = Authority::new([3; 32], "the-relay");
        Self {
            relay: Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
            authority,
        }
    }

    fn carry(&mut self, node: u64, request: &str) -> String {
        let token = self.authority.issue(LIST, node, u64::MAX);
        self.relay.handle(&token, request, 0)
    }

    /// A round of `peer`, every request carried to the relay and every answer back.
    fn round(&mut self, peer: &mut Peer, node: u64) -> Outcome {
        peer.start(RoundKind::Sync).unwrap();
        while let Some(request) = peer.poll_request() {
            let response = self.carry(node, &request);
            peer.respond(&response).unwrap();
        }
        let outcome = peer
            .take_outcome()
            .expect("the round ended")
            .expect("the round held");
        assert!(outcome.refused().is_empty(), "{:?}", outcome.refused());
        outcome
    }

    /// Every operation the relay holds of the list, as it holds it.
    fn operations_held(&mut self) -> Vec<(String, String)> {
        let text = self.carry(
            ALICE,
            &Request::Feed {
                cursor: 0,
                limit: 500,
            }
            .to_json(),
        );
        let Ok(Response::Feed { records, .. }) = Response::from_json(&text) else {
            panic!("the relay gives its feed: {text}");
        };
        records
            .into_iter()
            .filter_map(|record| match record {
                Record::Operation { event, data } => Some((event, data)),
                _ => None,
            })
            .collect()
    }
}

/// Alice's device: it holds the administrator's key, opens the list, and grants bob writing and
/// carol reading.
fn alice() -> Peer {
    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    alice.set_administrator(administrator());
    alice.open_list(Profile::Untrusted).unwrap();
    for (node, role) in [(BOB, 2u8), (CAROL, 1)] {
        alice.grant(node, role, person(node).public()).unwrap();
    }
    alice
}

fn invited(node: u64) -> Peer {
    let mut peer = Peer::invited(LIST, genesis().hash());
    peer.set_identity(node, person(node));
    peer
}

/// The device of `node` started again from `sealed`, into a core given nothing but the list.
fn restore(node: u64, sealed: &SealedState) -> Peer {
    let mut again = Peer::invited(LIST, genesis().hash());
    again
        .restore_sealed(node, &sealed.state, &sealed.key)
        .expect("the state opens under its key");
    again
}

/// The host writes every record the core gives to its journal, and says so: what it returns.
fn record(peer: &mut Peer) -> Vec<(String, String)> {
    let records = peer.take_records();
    if let Some(last) = records.last() {
        peer.recorded(last.seq);
    }
    records
        .into_iter()
        .map(|record| (record.event, record.data))
        .collect()
}

/// A history's changes as each one's time and author (`wallTime/by`) and its tag.
fn read(changes: &[Told]) -> Vec<(String, Option<Tag>)> {
    changes
        .iter()
        .map(|told| {
            let data: Value = serde_json::from_str(&told.data).unwrap();
            let what = format!(
                "{}/{}",
                data["wallTime"].as_str().unwrap_or(""),
                data["by"].as_str().unwrap_or("")
            );
            (what, told.tag.clone())
        })
        .collect()
}

fn tag(author: u64, seq: u64, context: &[(u64, u64)]) -> Option<Tag> {
    Some(Tag {
        author,
        seq,
        context: context.to_vec(),
    })
}

/// The change a copy offered is signed in the call that takes it, its command left out, and is a
/// record: the round gives the relay nothing of it until the host recorded it, and the round after
/// gives it; the same change sent again is the operation held. Red if a round gives the relay a
/// record the host has not recorded, or if `recorded` frees nothing.
#[test]
fn a_change_reaches_the_relay_only_once_its_host_recorded_it() {
    let mut network = Network::new();
    let mut alice = alice();
    let offered = made(1, ALICE, 1000, 10, 7);
    assert_eq!(
        alice.made(&offered),
        Kept::Taken,
        "the verdict is `accepted`"
    );
    let records = alice.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!((records[0].seq, records[0].event.as_str()), (1, CHANGE));
    let carried: Value = serde_json::from_str(&records[0].data).unwrap();
    assert!(
        carried.get("command").is_none(),
        "the command is the host's, and is not signed"
    );

    network.round(&mut alice, ALICE);
    assert!(
        network.operations_held().is_empty(),
        "the relay is given nothing the host has not recorded"
    );
    alice.recorded(records[0].seq);
    assert!(alice.take_records().is_empty());
    let given = network.round(&mut alice, ALICE);
    assert_eq!(given.round.pushed, 1, "the record, and nothing more");
    assert_eq!(
        network.operations_held(),
        vec![(records[0].event.clone(), records[0].data.clone())]
    );
    assert_eq!(
        alice.outbound(CHANGE, &change_of(&offered)),
        Kept::Held,
        "one change, signed once"
    );
}

/// A device started again from a state older than its journal takes its records back, signs
/// nothing and gives the relay nothing; the history it hands its rebuilt machine is every change
/// in causal order with its tag, its own included; and a device started again with no journal
/// reads its own chain back in its first round, which says the stamp rose. Red if a record given
/// back is signed again, if the history is not in causal order, or if the risen stamp is not said.
#[test]
fn a_device_started_again_signs_nothing_and_is_handed_its_history() {
    let mut network = Network::new();
    let mut alice = alice();
    let mut bob = invited(BOB);
    network.round(&mut alice, ALICE);
    network.round(&mut bob, BOB);

    assert_eq!(alice.made(&made(1, ALICE, 1000, 10, 1)), Kept::Taken);
    record(&mut alice);
    network.round(&mut alice, ALICE);
    network.round(&mut bob, BOB);
    assert_eq!(bob.made(&made(2, BOB, 2000, 20, 1)), Kept::Taken);
    record(&mut bob);
    network.round(&mut bob, BOB);
    let older = bob.save_sealed().expect("bob's device seals its state");

    assert_eq!(bob.made(&made(3, BOB, 3000, 30, 2)), Kept::Taken);
    let journal = record(&mut bob);
    assert_eq!(journal.len(), 1);
    network.round(&mut bob, BOB);
    let held = network.operations_held();
    assert_eq!(held.len(), 3);

    // Started again from the older state and the journal.
    let mut again = restore(BOB, &older);
    assert_eq!(
        again.own_stamp(),
        OwnStamp {
            stamp: Some((2000, 0)),
            read_back: false
        }
    );
    assert_eq!(again.restore_records(&journal), Ok(1));
    assert!(
        again.take_records().is_empty(),
        "a record given back is recorded"
    );
    assert_eq!(again.own_stamp().stamp, Some((3000, 0)));
    let history = again.history(&[]);
    assert_eq!(history.roster.len(), 3, "the roster the log makes");
    assert_eq!(
        read(&history.changes),
        vec![
            ("1000/1".to_string(), tag(ALICE, 1, &[])),
            ("2000/2".to_string(), tag(BOB, 1, &[(ALICE, 1)])),
            ("3000/2".to_string(), tag(BOB, 2, &[(ALICE, 1)])),
        ]
    );
    assert!(history.changes.iter().all(|told| !told.provisional()));
    assert_eq!(
        read(&again.history(&[(ALICE, 1), (BOB, 1)]).changes),
        vec![("3000/2".to_string(), tag(BOB, 2, &[(ALICE, 1)]))],
        "a history from a cut gives what lies past it"
    );
    let first = network.round(&mut again, BOB);
    assert_eq!(first.round.pushed, 0);
    assert_eq!(network.operations_held(), held, "the relay gained nothing");
    assert_eq!(first.raised, None, "the journal held the stamp already");
    assert!(again.own_stamp().read_back);

    // Started again from the older state alone: the round reads the chain back, and says so.
    let mut lost = restore(BOB, &older);
    let read_back = network.round(&mut lost, BOB);
    assert_eq!(read_back.raised, Some((3000, 0)));
    assert_eq!(read_back.round.pushed, 0);
    assert_eq!(network.operations_held(), held);
}

/// The stable cut is what every member is known to have been told: alice's change is stable once
/// bob, a writer who wrote nothing since, and carol, a reader, have marked that they were told it;
/// their marks reach the relay and alice's core, and no machine is told one. Red if a mark is not
/// read as its member's latest, if a reader's mark is refused, or if a machine is told a mark.
#[test]
fn a_change_is_stable_once_every_member_marked_it() {
    let mut network = Network::new();
    let mut alice = alice();
    let mut bob = invited(BOB);
    let mut carol = invited(CAROL);
    network.round(&mut alice, ALICE);
    assert_eq!(alice.made(&made(1, ALICE, 1000, 10, 1)), Kept::Taken);
    record(&mut alice);
    network.round(&mut alice, ALICE);
    network.round(&mut bob, BOB);
    network.round(&mut carol, CAROL);
    assert!(alice.stable_cut().is_empty());
    assert_eq!(alice.mark(), None, "alice was told nothing of another's");

    assert_eq!(bob.mark(), Some(1));
    assert_eq!(carol.mark(), Some(1), "a reader marks too");
    assert_eq!(bob.mark(), None, "nothing more was told since");
    network.round(&mut bob, BOB);
    network.round(&mut carol, CAROL);
    let marks = network
        .operations_held()
        .into_iter()
        .filter(|(event, _)| event == MARK_EVENT)
        .count();
    assert_eq!(marks, 2, "the relay holds both marks");

    let told = network.round(&mut alice, ALICE);
    assert!(
        told.events()
            .iter()
            .all(|(event, _)| event != MARK_EVENT && event != CHANGE),
        "no machine is told a mark: {:?}",
        told.events()
    );
    assert_eq!(alice.stable_cut(), vec![(ALICE, 1)]);
}
