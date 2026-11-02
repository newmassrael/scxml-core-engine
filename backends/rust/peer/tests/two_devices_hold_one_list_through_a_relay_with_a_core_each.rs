// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Devices of one list, each holding it through a peer core, and a relay in the same process; every
// text that crosses between a core and the relay is carried here by hand, as a host's transport
// would carry it. Alice's device holds the administrator's key and opens the list; bob's is invited
// with the hash of the genesis and granted; carol's may read. What is asked: that what alice's
// machine sends, in the record list's own events, reaches bob's machine as the event alice's machine
// sent; that a device saved and restored in the middle of a round goes on and both devices come to
// hold the same; that what a device that may not write sends is refused, and reaches no one; that a
// host that answers out of turn is told so; and that the group key is followed by the core, so that
// what alice writes under the group's epoch opens on bob's device.

use sce_rust_peer::{
    CourierError, Device, GroupStep, Kept, Outcome, Peer, PeerError, Publication, RoundKind,
};
use sce_rust_relay::{Authority, Limits, MemoryStore, Relay};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::Identity;
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const WRITE: &str = "record_list.write";
const SUPERSEDE: &str = "record_list.supersede";
const ROLE: &str = "record_list.role";

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

/// A write of `value` to field 3 of row `id`, stamped by `node` at `wall`, as the record list's
/// `record_list.write` carries it: a 64-bit number as text, a narrower one as a number.
fn write(id: u32, node: u64, wall: i64, value: i64) -> String {
    json!({"id": id, "field": 3, "value": value.to_string(), "text": "",
           "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()})
    .to_string()
}

/// The note, as `record_list.supersede` carries it, that `by` wrote over the write of row `id`
/// that `node` stamped at `wall`, in the change stamped `at`.
fn supersede(id: u32, node: u64, wall: i64, by: u64, at: i64) -> String {
    json!({"id": id, "field": 3, "wallTime": wall.to_string(), "counter": 0,
           "nodeId": node.to_string(), "by": by.to_string(),
           "atWallTime": at.to_string(), "atCounter": 0})
    .to_string()
}

/// The relay, and the authority whose tokens it takes.
struct Network {
    relay: Relay<MemoryStore>,
    authority: Authority,
}

impl Network {
    fn new(limits: Limits) -> Self {
        let authority = Authority::new([3; 32], "the-relay");
        Self {
            relay: Relay::open(MemoryStore::new(), authority.clone(), limits).unwrap(),
            authority,
        }
    }

    /// What the relay answers the device of `node`, as a transport carries it.
    fn carry(&mut self, node: u64, request: &str) -> String {
        let token = self.authority.issue(LIST, node, u64::MAX);
        self.relay.handle(&token, request, 0)
    }

    /// A round of `kind` of `peer`, every request carried to the relay and every answer back.
    fn round(&mut self, peer: &mut Peer, node: u64, kind: RoundKind) -> Outcome {
        peer.start(kind).unwrap();
        self.finish(peer, node)
    }

    /// The exchange `peer` runs, carried to its end.
    fn finish(&mut self, peer: &mut Peer, node: u64) -> Outcome {
        while let Some(request) = peer.poll_request() {
            let response = self.carry(node, &request);
            peer.respond(&response).unwrap();
        }
        peer.take_outcome()
            .expect("the exchange ended")
            .expect("the exchange held")
    }
}

/// Alice's device: it holds the administrator's key and opens the list, granting alice and bob
/// writing and carol reading.
fn alice() -> Peer {
    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    grant_all(&alice);
    alice
}

fn grant_all(alice: &Peer) {
    let trust = alice.trust();
    trust.append(&genesis()).unwrap();
    for (node, role) in [(ALICE, 3u8), (BOB, 2), (CAROL, 1)] {
        let entry = trust
            .draft(Body::Grant {
                node,
                role,
                key: person(node).public(),
                final_seq: None,
            })
            .sign(&administrator())
            .finish();
        trust.append(&entry).unwrap();
    }
}

/// A device invited with the hash of the genesis, which is all an invitation carries.
fn invited(node: u64) -> Peer {
    let mut peer = Peer::invited(LIST, genesis().hash());
    peer.set_identity(node, person(node));
    peer
}

/// What a request text asks of the relay.
fn op(request: &str) -> String {
    let value: Value = serde_json::from_str(request).unwrap();
    value["op"].as_str().unwrap().to_string()
}

fn named(outcome: &Outcome, event: &str) -> Vec<Value> {
    outcome
        .events()
        .into_iter()
        .filter(|(name, _)| name == event)
        .map(|(_, data)| serde_json::from_str(&data).unwrap())
        .collect()
}

#[test]
fn what_alices_machine_sends_reaches_bobs_machine_as_it_was_sent() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    let mut bob = invited(BOB);

    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 1000, 10)),
        Kept::Taken
    );
    let first = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(first.refused().is_empty(), "{:?}", first.refused());
    assert_eq!(first.round.pushed, 4 + 1, "the log and the write");
    assert!(
        named(&first, WRITE).is_empty(),
        "alice's machine is not told its own write back"
    );

    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(told.refused().is_empty(), "{:?}", told.refused());
    assert!(!told.stalled());
    assert_eq!(named(&told, ROLE).len(), 3, "the roster the log makes");
    let writes = named(&told, WRITE);
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0]["nodeId"], "1");
    assert_eq!(writes[0]["value"], "10");
    assert_eq!(writes[0]["id"], 1);
    assert!(
        writes[0].get("sig").is_none() && writes[0].get("seq").is_none(),
        "the machine is told the write, not the operation it travelled as: {}",
        writes[0]
    );
    assert_eq!(
        told.events()[0].0,
        ROLE,
        "the roster comes before the operations"
    );

    // Alice writes the field again and notes that her new write is over her first.
    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 2000, 20)),
        Kept::Taken
    );
    assert_eq!(
        alice.outbound(SUPERSEDE, &supersede(1, ALICE, 1000, ALICE, 2000)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let names: Vec<String> = told.events().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec![WRITE.to_string(), SUPERSEDE.to_string()]);
    let notes = named(&told, SUPERSEDE);
    assert_eq!(notes[0]["by"], "1");
    assert_eq!(notes[0]["wallTime"], "1000");

    // Nothing more is new, and both hold alice's chain to the same head.
    let again = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(again.events().is_empty(), "{:?}", again.events());
    assert_eq!(alice.trust().heads(), bob.trust().heads());
}

/// The relay gives two records a batch, so a round has several steps to stop between. Alice's
/// device is saved after it gave the relay part of what it lacks, bob's after it took part of the
/// feed in; each is restarted from its text in a new core, given its person again, and goes on.
#[test]
fn devices_saved_and_restored_in_the_middle_of_a_round_go_on_and_agree() {
    let mut network = Network::new(Limits {
        max_feed: 2,
        ..Limits::default()
    });
    let mut alice = alice();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    for n in 1..=4 {
        assert_eq!(
            alice.outbound(WRITE, &write(n, ALICE, 1000 + i64::from(n), i64::from(n))),
            Kept::Taken
        );
    }

    // Alice's round gives the relay two of her four writes, and the device stops.
    alice.start(RoundKind::Sync).unwrap();
    let mut submitted = 0;
    while submitted < 2 {
        let request = alice.poll_request().expect("the round asks");
        if op(&request) == "submit" {
            submitted += 1;
        }
        let response = network.carry(ALICE, &request);
        alice.respond(&response).unwrap();
    }
    let saved = alice.save();
    drop(alice);
    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    let kept = alice.restore(&saved).unwrap();
    assert_eq!(kept, 4, "her four writes");
    let finished = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(
        finished.round.pushed, 2,
        "the two the relay lacked, and no more"
    );

    // Bob's round takes the log and two of alice's writes in, and the device stops before the
    // answer to the next batch comes.
    let mut bob = invited(BOB);
    bob.start(RoundKind::Sync).unwrap();
    let mut feeds = 0;
    let pending = loop {
        let request = bob.poll_request().expect("the round asks");
        if op(&request) == "feed" {
            feeds += 1;
            // The log is records 0 to 3, alice's writes 4 to 7: the fourth batch is 6 and 7.
            if feeds == 4 {
                break request;
            }
        }
        let response = network.carry(BOB, &request);
        bob.respond(&response).unwrap();
    };
    assert_eq!(
        bob.poll_request().as_deref(),
        Some(pending.as_str()),
        "asked again, it asks the same"
    );
    let saved = bob.save();
    drop(bob);
    let mut bob = invited(BOB);
    bob.restore(&saved).unwrap();
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let mut rows: Vec<Value> = named(&told, WRITE)
        .into_iter()
        .map(|write| write["id"].clone())
        .collect();
    rows.sort_by_key(|id| id.as_u64());
    assert_eq!(
        rows,
        vec![json!(1), json!(2), json!(3), json!(4)],
        "every write once, the two taken before the stop included"
    );
    assert_eq!(named(&told, ROLE).len(), 3);
    assert_eq!(alice.trust().heads(), bob.trust().heads());

    let again = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(again.events().is_empty(), "{:?}", again.events());
}

/// A device that may only read signs what its machine sends (its host is not where that is
/// decided), and the relay refuses it: its round says so, and bob's machine is never told it. A
/// device with no person signs nothing, and an event that is not signed is kept nowhere.
#[test]
fn what_a_device_that_may_not_write_sends_is_refused_and_reaches_no_one() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    network.round(&mut alice, ALICE, RoundKind::Sync);

    let mut carol = invited(CAROL);
    network.round(&mut carol, CAROL, RoundKind::Sync);
    assert_eq!(
        carol.outbound(WRITE, &write(5, CAROL, 1000, 1)),
        Kept::Taken
    );
    let refused = network.round(&mut carol, CAROL, RoundKind::Sync);
    assert_eq!(refused.refused().len(), 1, "{:?}", refused.refused());
    assert_eq!(refused.round.pushed, 0);

    let mut bob = invited(BOB);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(named(&told, WRITE).is_empty(), "{:?}", told.events());

    let mut nobody = Peer::invited(LIST, genesis().hash());
    network.round(&mut nobody, BOB, RoundKind::Sync);
    assert!(matches!(
        nobody.outbound(WRITE, &write(6, BOB, 1000, 1)),
        Kept::Refused(_)
    ));
    assert_eq!(
        alice.outbound("record_list.propose", r#"{"before":"30000"}"#),
        Kept::NotSigned
    );
}

/// A host that answers before it asked, begins an exchange before taking the last one's outcome,
/// or gives a text that is not a saved one, is told so; an answer that is not a response of the
/// protocol is a failure of the transport.
#[test]
fn a_host_that_answers_out_of_turn_is_told_so() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    assert_eq!(alice.respond("{}"), Err(PeerError::NothingAsked));
    assert_eq!(alice.poll_request(), None, "no exchange runs");

    alice.start(RoundKind::Sync).unwrap();
    assert_eq!(
        alice.respond("{}"),
        Err(PeerError::NothingAsked),
        "nothing was asked yet"
    );
    let request = alice.poll_request().unwrap();
    alice.respond("not a response").unwrap();
    assert_eq!(alice.poll_request(), None);
    assert_eq!(
        alice.start(RoundKind::Sync),
        Err(PeerError::OutcomeNotTaken)
    );
    assert!(matches!(
        alice.take_outcome(),
        Some(Err(CourierError::Link(_)))
    ));
    assert_eq!(alice.take_outcome(), None, "an outcome is taken once");

    // The round goes on as if the first had not been begun.
    alice.start(RoundKind::Sync).unwrap();
    assert_eq!(alice.poll_request().as_deref(), Some(request.as_str()));
    let response = network.carry(ALICE, &request);
    alice.respond(&response).unwrap();
    assert!(network.finish(&mut alice, ALICE).refused().is_empty());

    // A failure of the transport ends the exchange the same way.
    alice.start(RoundKind::Sync).unwrap();
    alice.poll_request().unwrap();
    alice.fail("the connection closed").unwrap();
    assert_eq!(alice.poll_request(), None);
    assert!(matches!(
        alice.take_outcome(),
        Some(Err(CourierError::Link(_)))
    ));

    let mut fresh = Peer::open(LIST);
    assert!(matches!(fresh.restore("{}"), Err(PeerError::Unreadable(_))));
    assert!(matches!(
        fresh.restore("not JSON"),
        Err(PeerError::Unreadable(_))
    ));
    assert_eq!(fresh.publish_key_package(), Err(PeerError::NoGroupDevice));
    // A checkpoint is a publication like the group's: given after what the relay lacks.
    alice
        .publish(&Publication::Checkpoint {
            before: 30_000,
            digest: 7,
        })
        .unwrap();
    let published = network.finish(&mut alice, ALICE);
    assert!(published.events().is_empty());
    assert!(published.refused().is_empty(), "{:?}", published.refused());
}

/// The list's content key is its MLS group's, and the core follows the group: bob's device leaves a
/// key package with the relay, alice's administrator device reads it and adds him (the log names the
/// commit), and bob's core joins from the Welcome a round reads, giving his trust the key of the
/// epoch; what alice writes then is sealed under it, and bob's machine is told it opened.
#[test]
fn the_core_follows_the_group_and_what_is_sealed_under_its_epoch_opens() {
    let mut network = Network::new(Limits::default());
    let mut alice = Peer::open(LIST).with_group(Device::new(LIST, ALICE, person(ALICE)), ALICE);
    alice.set_identity(ALICE, person(ALICE));
    let trust = alice.trust().clone();
    trust.append(&genesis()).unwrap();
    let grant = trust
        .draft(Body::Grant {
            node: ALICE,
            role: 3,
            key: person(ALICE).public(),
            final_seq: None,
        })
        .sign(&administrator())
        .finish();
    trust.append(&grant).unwrap();
    alice
        .group_mut()
        .unwrap()
        .device_mut()
        .create_group()
        .unwrap();
    network.round(&mut alice, ALICE, RoundKind::Sync);

    let mut bob =
        Peer::invited(LIST, genesis().hash()).with_group(Device::new(LIST, BOB, person(BOB)), BOB);
    bob.set_identity(BOB, person(BOB));
    bob.publish_key_package().unwrap();
    network.finish(&mut bob, BOB);
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let package = alice
        .group()
        .unwrap()
        .key_packages_of(BOB)
        .last()
        .cloned()
        .expect("alice's core read bob's key package");

    let issued = alice
        .group_mut()
        .unwrap()
        .add_member(&administrator(), BOB, 2, person(BOB).public(), &package)
        .unwrap();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    alice
        .publish(&Publication::Commit {
            epoch: issued.epoch,
            commit: issued.commit.clone(),
        })
        .unwrap();
    network.finish(&mut alice, ALICE);
    alice
        .publish(&Publication::Welcome {
            epoch: issued.epoch,
            node: BOB,
            welcome: issued.welcome.clone().unwrap(),
        })
        .unwrap();
    network.finish(&mut alice, ALICE);

    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(
        joined.steps,
        vec![GroupStep::Joined(u64::from(issued.epoch))],
        "{joined:?}"
    );
    assert_eq!(
        bob.group().unwrap().device().epoch_key(),
        alice.group().unwrap().device().epoch_key()
    );
    assert_eq!(bob.trust().epochs(), vec![issued.epoch]);

    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 3000, 5)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = named(&told, WRITE);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(writes[0]["value"], "5", "opened under the group's key");
}
