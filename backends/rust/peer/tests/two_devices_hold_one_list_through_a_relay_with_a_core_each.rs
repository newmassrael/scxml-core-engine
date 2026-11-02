// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Devices of one list, each holding it through a peer core, and a relay in the same process; every
// text that crosses between a core and the relay is carried here by hand, as a host's transport
// would carry it. Alice's device holds the administrator's key and opens the list through its core;
// bob's is invited with the hash of the genesis and granted; carol's may read. What is asked: that
// what alice's machine sends, in the record list's own events, reaches bob's machine as the event
// alice's machine sent; that a device saved and restored in the middle of a round goes on and both
// devices come to hold the same; that what a device that may not write sends is refused, and
// reaches no one; that a host that answers out of turn is told so; that the group key is followed by
// the core, and the administrator's core admits a device to the list and its group in one act, so
// that what alice writes under the group's epoch opens on bob's device; that what the relay is owed
// of such an act outlives a save and a failed round; that an administrator's act the core cannot do
// is refused with its reason and writes nothing; that a device taken off is refused by the relay;
// and that a device whose machine is built again from its own log of commands writes nothing twice,
// refuses another content under a stamp it signed, and, restored from a save older than what it
// published, reads its own chain back from the relay before it signs, so that no host holds a frozen
// chain of it.

use sce_rust_peer::{
    CourierError, Device, GroupStep, Kept, LogError, MlsError, Outcome, Peer, PeerError, Profile,
    Publication, RoundKind,
};
use sce_rust_relay::{Authority, Limits, MemoryStore, Relay};
use sce_rust_trust::membership::{Body, Entry, Log};
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

/// Alice's device: it holds the administrator's key and opens the list, which makes her its owner,
/// and grants bob writing and carol reading.
fn alice() -> Peer {
    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    alice.set_administrator(administrator());
    assert_eq!(
        alice.open_list(Profile::Untrusted).unwrap(),
        genesis().hash(),
        "the genesis a person is invited with"
    );
    for (node, role) in [(BOB, 2u8), (CAROL, 1)] {
        alice.grant(node, role, person(node).public()).unwrap();
    }
    alice
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

/// Alice's device, which holds the administrator's key and a device of the list's group: it opens
/// the list, and the group with it.
fn alice_with_group() -> Peer {
    let mut alice = Peer::open(LIST).with_group(Device::new(LIST, ALICE, person(ALICE)), ALICE);
    alice.set_identity(ALICE, person(ALICE));
    alice.set_administrator(administrator());
    alice.open_list(Profile::Untrusted).unwrap();
    alice
}

/// Bob's device, invited, with a device of the list's group, which has left a key package with the
/// relay.
fn bob_with_a_key_package(network: &mut Network) -> Peer {
    let mut bob =
        Peer::invited(LIST, genesis().hash()).with_group(Device::new(LIST, BOB, person(BOB)), BOB);
    bob.set_identity(BOB, person(BOB));
    bob.publish_key_package().unwrap();
    network.finish(&mut bob, BOB);
    bob
}

/// The list's content key is its MLS group's, and the core follows the group: bob's device leaves a
/// key package with the relay, alice's administrator device reads it and admits him in one act (the
/// grant, the commit and the entry that names it), the next round gives the relay the entries, then
/// the commit and his Welcome, and bob's core joins from the Welcome a round reads, giving his trust
/// the key of the epoch; what alice writes then is sealed under it, and bob's machine is told it
/// opened.
#[test]
fn the_core_follows_the_group_and_what_is_sealed_under_its_epoch_opens() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice_with_group();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let mut bob = bob_with_a_key_package(&mut network);
    assert_eq!(
        alice.add_member(BOB, 2, person(BOB).public()),
        Err(PeerError::NoKeyPackage(BOB)),
        "no round of alice's has read his key package yet"
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);

    alice.add_member(BOB, 2, person(BOB).public()).unwrap();
    assert_eq!(alice.standing().owed, 2, "the commit and his Welcome");
    assert_eq!(alice.trust().role(BOB), 2);
    let admitted = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(admitted.refused().is_empty(), "{:?}", admitted.refused());
    assert_eq!(admitted.owed, 0);
    assert_eq!(
        admitted.round.pushed, 2,
        "the grant and the entry that names the commit"
    );

    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(joined.steps, vec![GroupStep::Joined(1)], "{joined:?}");
    assert_eq!(
        bob.group().unwrap().device().epoch_key(),
        alice.group().unwrap().device().epoch_key()
    );
    assert_eq!(bob.trust().epochs(), vec![1]);
    assert_eq!(bob.standing().keys, alice.standing().keys);

    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 3000, 5)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = named(&told, WRITE);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(writes[0]["value"], "5", "opened under the group's key");

    // Alice takes him off the list and the group in one act: the group moves to an epoch his
    // device holds no key of, and what she writes under it reaches him no more.
    alice.remove_member(BOB).unwrap();
    assert_eq!(alice.standing().owed, 1, "the commit, and no Welcome");
    let removed = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(removed.refused().is_empty(), "{:?}", removed.refused());
    assert_eq!(removed.owed, 0);
    assert_eq!(alice.trust().epochs(), vec![1, 2]);
    assert_eq!(
        alice.outbound(WRITE, &write(2, ALICE, 4000, 6)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let cut = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(named(&cut, WRITE).is_empty(), "{:?}", cut.events());
    assert_eq!(bob.standing().role, 0);
    assert_eq!(bob.trust().epochs(), vec![1], "no key of the epoch after");
}

/// What the relay is owed of an administrator's act is kept: in the save, so that a device that
/// stops before its next round still gives the relay the commit its log names, and across a round
/// whose delivery the transport lost, whose outcome says so. The round after gives it, and bob joins.
#[test]
fn what_the_relay_is_owed_outlives_a_save_and_a_failed_round() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice_with_group();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let mut bob = bob_with_a_key_package(&mut network);
    network.round(&mut alice, ALICE, RoundKind::Sync);
    alice.add_member(BOB, 2, person(BOB).public()).unwrap();

    // Alice's device stops with the commit and the Welcome not given, and starts again from its
    // save, with its secrets given again.
    let saved = alice.save();
    let group = alice.group().unwrap().device().to_bytes().unwrap();
    drop(alice);
    let device = Device::from_bytes(LIST, ALICE, person(ALICE), &group).unwrap();
    let mut alice = Peer::open(LIST).with_group(device, ALICE);
    alice.set_identity(ALICE, person(ALICE));
    alice.set_administrator(administrator());
    alice.restore(&saved).unwrap();
    assert_eq!(alice.standing().owed, 2);

    // The round ends, and the transport fails as the commit is given: it is owed still, and the
    // outcome says why.
    alice.start(RoundKind::Sync).unwrap();
    while let Some(request) = alice.poll_request() {
        if op(&request) == "commit" {
            alice.fail("the connection closed").unwrap();
            continue;
        }
        let response = network.carry(ALICE, &request);
        alice.respond(&response).unwrap();
    }
    let lost = alice
        .take_outcome()
        .expect("the round ended")
        .expect("the round held, though what it owed was not given");
    assert_eq!(lost.owed, 2, "{lost:?}");
    assert!(
        lost.refused().iter().any(|why| why.contains("owed")),
        "{:?}",
        lost.refused()
    );
    assert_eq!(alice.standing().owed, 2);

    let given = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(given.owed, 0, "{given:?}");
    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(joined.steps, vec![GroupStep::Joined(1)], "{joined:?}");

    // A save of the core before it kept what the relay is owed is still read; a save that says it
    // keeps it and does not is not.
    let mut old: Value = serde_json::from_str(&alice.save()).unwrap();
    old["version"] = json!(1);
    old.as_object_mut().unwrap().remove("owed");
    let mut fresh = Peer::open(LIST);
    fresh.set_identity(ALICE, person(ALICE));
    assert!(fresh.restore(&old.to_string()).is_ok());
    old["version"] = json!(2);
    let mut fresh = Peer::open(LIST);
    assert!(matches!(
        fresh.restore(&old.to_string()),
        Err(PeerError::Unreadable(_))
    ));
}

/// An administrator's act the core cannot do is refused with why, and nothing of it is written.
#[test]
fn an_administrators_act_the_core_cannot_do_writes_nothing() {
    // No administrator's key, no person, a list opened already, a genesis that is not the one the
    // device was invited with.
    let mut plain = Peer::open(LIST);
    plain.set_identity(ALICE, person(ALICE));
    assert_eq!(
        plain.open_list(Profile::Untrusted),
        Err(PeerError::NoAdministrator)
    );
    assert_eq!(
        plain.grant(BOB, 2, person(BOB).public()),
        Err(PeerError::NoAdministrator)
    );
    assert_eq!(plain.remove_member(BOB), Err(PeerError::NoAdministrator));
    assert_eq!(
        plain.add_member(BOB, 2, person(BOB).public()),
        Err(PeerError::NoAdministrator)
    );
    let mut nobody = Peer::open(LIST);
    nobody.set_administrator(administrator());
    assert_eq!(
        nobody.open_list(Profile::Untrusted),
        Err(PeerError::NoPerson)
    );
    assert_eq!(nobody.standing().genesis, None);
    let mut elsewhere = Peer::invited(LIST, genesis().hash());
    elsewhere.set_identity(ALICE, person(ALICE));
    elsewhere.set_administrator(Identity::from_seed([8; 32]));
    assert_eq!(
        elsewhere.open_list(Profile::Untrusted),
        Err(PeerError::Log(LogError::NotTheGenesis))
    );
    assert_eq!(elsewhere.standing().genesis, None, "nothing written");

    let mut alice = alice();
    let head = alice.trust().head();
    assert_eq!(alice.open_list(Profile::Untrusted), Err(PeerError::Opened));
    for role in [0, 4] {
        assert_eq!(
            alice.grant(BOB, role, person(BOB).public()),
            Err(PeerError::NoSuchRole(role))
        );
        assert_eq!(
            alice.add_member(BOB, role, person(BOB).public()),
            Err(PeerError::NoSuchRole(role))
        );
    }
    assert_eq!(alice.remove_member(9), Err(PeerError::NotAMember(9)));
    assert_eq!(
        alice.add_member(BOB, 2, person(BOB).public()),
        Err(PeerError::NoGroupDevice)
    );
    assert_eq!(alice.trust().head(), head, "nothing was written");
    let standing = alice.standing();
    assert_eq!(standing.genesis, Some(genesis().hash()));
    assert!(standing.administrator);
    assert_eq!(standing.role, 3);
    assert_eq!(standing.members, vec![(ALICE, 3), (BOB, 2), (CAROL, 1)]);

    // A grant that takes writing away carries where the device's chain ends, as the log requires
    // of it; one that gives writing does not.
    alice.grant(BOB, 1, person(BOB).public()).unwrap();
    alice.grant(CAROL, 2, person(CAROL).public()).unwrap();
    let grants: Vec<Body> = alice
        .trust()
        .entries()
        .iter()
        .rev()
        .take(2)
        .map(|entry| entry.body.clone())
        .collect();
    assert_eq!(
        grants,
        vec![
            Body::Grant {
                node: CAROL,
                role: 2,
                key: person(CAROL).public(),
                final_seq: None,
            },
            Body::Grant {
                node: BOB,
                role: 1,
                key: person(BOB).public(),
                final_seq: Some(0),
            },
        ]
    );

    // A list keyed by its group admits a device only with its key package; another role for a
    // member the group holds is a grant alone. Opening a list with a device already in a group is
    // refused before anything is written.
    let mut grouped = alice_with_group();
    assert_eq!(
        grouped.grant(BOB, 2, person(BOB).public()),
        Err(PeerError::AdmittedThroughGroup(BOB))
    );
    grouped.grant(ALICE, 2, person(ALICE).public()).unwrap();
    assert_eq!(grouped.standing().role, 2);
    let mut again = Peer::open(LIST).with_group(
        Device::from_bytes(
            LIST,
            ALICE,
            person(ALICE),
            &grouped.group().unwrap().device().to_bytes().unwrap(),
        )
        .unwrap(),
        ALICE,
    );
    again.set_identity(ALICE, person(ALICE));
    again.set_administrator(administrator());
    assert_eq!(
        again.open_list(Profile::Untrusted),
        Err(PeerError::Group(MlsError::InGroup))
    );
    assert_eq!(again.standing().genesis, None, "nothing written");

    // A list whose keys an administrator draws is not one the core takes a member off: the
    // members left would need a new key the core does not hand over.
    alice
        .trust()
        .rotate_epoch(&administrator(), &[person(ALICE).public()])
        .unwrap();
    let head = alice.trust().head();
    assert_eq!(alice.remove_member(BOB), Err(PeerError::DrawnKeys));
    assert_eq!(alice.trust().head(), head, "nothing was written");
}

/// Alice takes bob off a list with no keys: one entry of the log, which ends his chain at the last
/// operation of his her device holds. What he writes after it is refused by the relay, and reaches
/// no one; his own device, once it read the entry, says he is no member.
#[test]
fn a_device_taken_off_writes_to_no_one() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    let mut bob = invited(BOB);
    network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(bob.outbound(WRITE, &write(1, BOB, 1000, 1)), Kept::Taken);
    network.round(&mut bob, BOB, RoundKind::Sync);
    network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(alice.trust().last_seq(BOB), 1);

    alice.remove_member(BOB).unwrap();
    assert_eq!(alice.standing().members, vec![(ALICE, 3), (CAROL, 1)]);
    let removed = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(removed.round.pushed, 1, "the removal");
    assert_eq!(
        alice
            .trust()
            .entries()
            .last()
            .map(|entry| entry.body.clone()),
        Some(Body::Grant {
            node: BOB,
            role: 0,
            key: person(BOB).public(),
            final_seq: Some(1),
        })
    );

    assert_eq!(bob.outbound(WRITE, &write(2, BOB, 2000, 2)), Kept::Taken);
    let cut = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(cut.round.pushed, 0);
    assert_eq!(cut.refused().len(), 1, "{:?}", cut.refused());
    assert_eq!(bob.standing().role, 0);
    let after = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(named(&after, WRITE).is_empty(), "{:?}", after.events());
}

/// Alice's application keeps its own log of commands and builds her machine again from it after
/// her device restarted from its save: the machine sends every write and note again, as it made
/// them. The core gives each back as the operation it holds, the relay is given nothing, and bob's
/// machine is told nothing twice; a resync tells the rebuilt machine the list.
#[test]
fn a_device_that_replays_its_own_log_after_a_restart_writes_nothing_twice() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    let mut bob = invited(BOB);
    let sends = [
        (WRITE, write(1, ALICE, 1000, 10)),
        (WRITE, write(1, ALICE, 2000, 20)),
        (SUPERSEDE, supersede(1, ALICE, 1000, ALICE, 2000)),
        (WRITE, write(2, ALICE, 3000, 30)),
    ];
    for (event, data) in &sends {
        assert_eq!(alice.outbound(event, data), Kept::Taken, "{event} {data}");
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(named(&told, WRITE).len(), 3);
    assert_eq!(named(&told, SUPERSEDE).len(), 1);
    let held = alice.trust().signed_operations();

    let saved = alice.save();
    drop(alice);
    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    alice.restore(&saved).unwrap();
    let back = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(back.refused().is_empty(), "{:?}", back.refused());
    assert!(
        !alice.trust().awaits_own_chain(),
        "the relay holds no more of her chain than the save"
    );

    for (event, data) in &sends {
        assert_eq!(alice.outbound(event, data), Kept::Held, "{event} {data}");
    }
    assert_eq!(
        alice.trust().signed_operations(),
        held,
        "the same operations, numbers and signatures, and no more"
    );
    let replayed = network.round(&mut alice, ALICE, RoundKind::Resync);
    assert!(replayed.refused().is_empty(), "{:?}", replayed.refused());
    assert_eq!(replayed.round.pushed, 0, "the relay is given nothing new");
    assert_eq!(
        named(&replayed, WRITE).len(),
        3,
        "the rebuilt machine is told the list"
    );

    let again = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(again.events().is_empty(), "{:?}", again.events());
    assert!(bob.trust().equivocating().is_empty());
    assert_eq!(alice.trust().heads(), bob.trust().heads());
}

/// The machine sends a write under a stamp the device signed, with another value: it would be the
/// device signing two things for one write. The core refuses it and signs nothing, and the relay and
/// bob hold the write as it was first made.
#[test]
fn a_write_sent_again_with_another_content_is_refused_and_nothing_is_signed() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 1000, 10)),
        Kept::Taken
    );
    let before = alice.trust().signed_operations();
    match alice.outbound(WRITE, &write(1, ALICE, 1000, 11)) {
        Kept::Refused(reason) => assert!(reason.contains("another content"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(alice.trust().signed_operations(), before);
    let round = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(round.round.pushed, 4 + 1, "the log and the one write");

    let mut bob = invited(BOB);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = named(&told, WRITE);
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0]["value"], "10");
    assert!(bob.trust().equivocating().is_empty());
}

/// Alice's device writes four times and publishes them, and comes back from a save that held two.
/// The core signs nothing new until a round has read her chain back: the round asks the relay for
/// her operations past what the core holds right after the relay's heads, tells her machine the two
/// it held no more, and the device then signs at 5. Bob holds one chain of hers, not a frozen one.
#[test]
fn a_device_restored_from_an_older_save_reads_its_chain_back_before_it_signs() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    let mut bob = invited(BOB);
    for n in 1..=2 {
        assert_eq!(
            alice.outbound(WRITE, &write(n, ALICE, 1000 + i64::from(n), 1)),
            Kept::Taken
        );
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let older = alice.save();
    for n in 3..=4 {
        assert_eq!(
            alice.outbound(WRITE, &write(n, ALICE, 1000 + i64::from(n), 1)),
            Kept::Taken
        );
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(bob.trust().last_seq(ALICE), 4);
    drop(alice);

    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    assert_eq!(alice.restore(&older).unwrap(), 2);
    assert!(alice.trust().awaits_own_chain());
    match alice.outbound(WRITE, &write(5, ALICE, 5000, 5)) {
        Kept::Refused(reason) => assert!(reason.contains("signs nothing new"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 1001, 1)),
        Kept::Held,
        "what the core holds is given as it was"
    );
    assert_eq!(alice.trust().last_seq(ALICE), 2);

    alice.start(RoundKind::Sync).unwrap();
    let mut asked = Vec::new();
    while let Some(request) = alice.poll_request() {
        asked.push(op(&request));
        let response = network.carry(ALICE, &request);
        alice.respond(&response).unwrap();
    }
    let back = alice
        .take_outcome()
        .expect("the round ended")
        .expect("the round held");
    let heads = asked.iter().position(|name| name == "heads");
    let operations = asked.iter().position(|name| name == "operations");
    assert!(operations.is_some(), "{asked:?}");
    assert_eq!(operations, heads.map(|at| at + 1), "{asked:?}");
    assert!(back.refused().is_empty(), "{:?}", back.refused());
    assert_eq!(back.round.pushed, 0);
    let rows: Vec<Value> = named(&back, WRITE)
        .into_iter()
        .map(|write| write["id"].clone())
        .collect();
    assert_eq!(
        rows,
        vec![json!(3), json!(4)],
        "the writes it held no more, once"
    );
    assert!(!alice.trust().awaits_own_chain());
    assert_eq!(alice.trust().last_seq(ALICE), 4);

    assert_eq!(
        alice.outbound(WRITE, &write(5, ALICE, 5000, 5)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = named(&told, WRITE);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(writes[0]["id"], 5);
    assert!(bob.trust().equivocating().is_empty(), "no frozen chain");
    assert_eq!(bob.trust().last_seq(ALICE), 5);
    assert_eq!(alice.trust().heads(), bob.trust().heads());
}
