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
//
// What a device keeps is one sealed state, its secrets with the rest, and every restore here is into
// a core given nothing but the list: that a device restored so goes on with the protocol as it would
// have (rounds, writes, the administrator's acts, the group's epochs); that a state is refused whole,
// with nothing taken, under another key, changed, cut, or for another list or node; and that no
// secret and no content is in it in the clear.

use sce_rust_peer::{
    CourierError, DataKey, Device, GroupStep, Kept, LogError, MlsError, Outcome, Peer, PeerError,
    Profile, Publication, RoundKind,
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

/// The device's core stopped and made again from its sealed state, as a host starts it: a core given
/// nothing but the list (and the genesis it was invited with), and the state and its key.
fn restarted(peer: Peer, node: u64) -> Peer {
    let sealed = peer
        .save_sealed()
        .expect("a device with a person seals its state");
    drop(peer);
    let mut again = made_again(node);
    again
        .restore_sealed(node, &sealed.state, &sealed.key)
        .expect("the state opens under its key");
    again
}

/// A core of the list given nothing: alice's opened it, the others were invited with its genesis.
fn made_again(node: u64) -> Peer {
    if node == ALICE {
        Peer::open(LIST)
    } else {
        Peer::invited(LIST, genesis().hash())
    }
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
/// feed in; each is restarted from its sealed state in a new core given nothing, and goes on.
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
    let saved = alice.save_sealed().unwrap();
    drop(alice);
    let mut alice = Peer::open(LIST);
    let kept = alice
        .restore_sealed(ALICE, &saved.state, &saved.key)
        .unwrap();
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
    let mut bob = restarted(bob, BOB);
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

/// A device that may only read, by the log its core has read, is refused what its machine sends by
/// its own core: no host would admit an operation signed under a grant that does not let its
/// person write, and one signed would take a number of the device's chain no host holds. Nothing
/// is signed, the relay is given nothing, and bob's machine is never told it. A device with no
/// person signs nothing, and an event that is not signed is kept nowhere.
#[test]
fn what_a_device_that_may_not_write_sends_is_refused_and_reaches_no_one() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    network.round(&mut alice, ALICE, RoundKind::Sync);

    let mut carol = invited(CAROL);
    network.round(&mut carol, CAROL, RoundKind::Sync);
    match carol.outbound(WRITE, &write(5, CAROL, 1000, 1)) {
        Kept::Refused(reason) => assert!(reason.contains("may not write"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(carol.trust().signed_operations().is_empty());
    assert_eq!(carol.standing().waiting, 0, "nothing is kept");
    let refused = network.round(&mut carol, CAROL, RoundKind::Sync);
    assert!(refused.refused().is_empty(), "{:?}", refused.refused());
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
/// or gives bytes that are not a sealed state, is told so; an answer that is not a response of the
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
    let key = DataKey::from_bytes([5; 32]);
    let not_sealed: [&[u8]; 3] = [b"{}", b"not a sealed state", b""];
    for bytes in not_sealed {
        assert!(matches!(
            fresh.restore_sealed(ALICE, bytes, &key),
            Err(PeerError::Unreadable(_))
        ));
    }
    assert_eq!(
        fresh.save_sealed().map(|_| ()),
        Err(PeerError::NoPerson),
        "a sealed state is the state of one person's device"
    );
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
    // sealed state, its group's device and its keys with it.
    let mut alice = restarted(alice, ALICE);
    assert_eq!(alice.standing().owed, 2);
    assert!(alice.group().is_some(), "the group's device came back");

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
}

/// Alice's administrator's device and bob's are each stopped and made again from their sealed
/// states, at the points where a device keeps the most that cannot be had again, with no key given
/// again: alice's after her round read bob's key package and before she admitted him, bob's after
/// he joined. Each goes on as it would have. Alice admits bob from the key package her state kept,
/// with the administrator's key and the group's device it kept; bob opens what alice writes under
/// the epoch, with the key his state kept; alice takes him off, and the group moves to an epoch his
/// device holds no key of.
#[test]
fn a_device_restored_from_its_sealed_state_goes_on_with_no_key_given_again() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice_with_group();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let mut bob = bob_with_a_key_package(&mut network);
    network.round(&mut alice, ALICE, RoundKind::Sync);

    let mut alice = restarted(alice, ALICE);
    let standing = alice.standing();
    assert_eq!(standing.person, Some((ALICE, person(ALICE).public())));
    assert!(standing.administrator, "the administrator's key came back");
    assert!(standing.awaits_own_chain);
    let back = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(back.refused().is_empty(), "{:?}", back.refused());
    assert!(!alice.awaits_own_chain());
    alice
        .add_member(BOB, 2, person(BOB).public())
        .expect("the key package the round read came back with the state");
    let admitted = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(admitted.refused().is_empty(), "{:?}", admitted.refused());
    assert_eq!(admitted.owed, 0);
    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(joined.steps, vec![GroupStep::Joined(1)], "{joined:?}");

    let epoch_key = bob.group().unwrap().device().epoch_key();
    let mut bob = restarted(bob, BOB);
    assert_eq!(
        bob.standing().keys,
        vec![1],
        "the key of the epoch came back"
    );
    assert_eq!(bob.group().unwrap().device().epoch().ok(), Some(1));
    assert_eq!(bob.group().unwrap().device().epoch_key(), epoch_key);
    assert_eq!(alice.group().unwrap().device().epoch_key(), epoch_key);
    network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(!bob.awaits_own_chain());

    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 3000, 5)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = named(&told, WRITE);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(
        writes[0]["value"], "5",
        "opened under the key the state kept"
    );
    assert_eq!(bob.outbound(WRITE, &write(2, BOB, 3500, 6)), Kept::Taken);
    network.round(&mut bob, BOB, RoundKind::Sync);
    let theirs = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(named(&theirs, WRITE).len(), 1, "{:?}", theirs.events());

    alice.remove_member(BOB).unwrap();
    let removed = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(removed.refused().is_empty(), "{:?}", removed.refused());
    assert_eq!(alice.trust().epochs(), vec![1, 2]);
    network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(bob.standing().role, 0);
    assert_eq!(bob.trust().epochs(), vec![1], "no key of the epoch after");
}

/// A sealed state is refused whole, and the core it was given to takes nothing of it, when the key is
/// another, when a byte of it was changed, when it was cut, and when it is another list's or another
/// node's; the right key for the right device then restores it.
#[test]
fn a_sealed_state_is_refused_whole_under_another_key_changed_cut_or_for_another_device() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 1000, 10)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let sealed = alice.save_sealed().unwrap();
    let state = &sealed.state;
    let mut changed = state.clone();
    changed[state.len() / 2] ^= 1;

    let untouched = Peer::open(LIST).standing();
    // What is changed, the list and the node restored into, the bytes, the key, and whether the
    // bytes are a sealed state that does not open (rather than none at all).
    type Case<'a> = (&'a str, &'a str, u64, &'a [u8], DataKey, bool);
    let cases: [Case; 6] = [
        (
            "another key",
            LIST,
            ALICE,
            state,
            DataKey::from_bytes([0; 32]),
            true,
        ),
        (
            "a changed byte",
            LIST,
            ALICE,
            &changed,
            DataKey::from_bytes(*sealed.key.as_bytes()),
            true,
        ),
        (
            "another node",
            LIST,
            BOB,
            state,
            DataKey::from_bytes(*sealed.key.as_bytes()),
            true,
        ),
        (
            "another list",
            "work",
            ALICE,
            state,
            DataKey::from_bytes(*sealed.key.as_bytes()),
            true,
        ),
        (
            "cut inside what it seals",
            LIST,
            ALICE,
            &state[..state.len() - 1],
            DataKey::from_bytes(*sealed.key.as_bytes()),
            true,
        ),
        (
            "cut short of a sealed state",
            LIST,
            ALICE,
            &state[..10],
            DataKey::from_bytes(*sealed.key.as_bytes()),
            false,
        ),
    ];
    for (what, list, node, bytes, key, opens_not) in cases {
        let mut fresh = Peer::open(list);
        let refused = fresh.restore_sealed(node, bytes, &key);
        if opens_not {
            assert_eq!(refused, Err(PeerError::NotOpened), "{what}");
        } else {
            assert!(
                matches!(refused, Err(PeerError::Unreadable(_))),
                "{what}: {refused:?}"
            );
        }
        assert_eq!(fresh.standing(), untouched, "{what}: nothing was taken");
        assert!(fresh.trust().entries().is_empty(), "{what}");
        assert!(fresh.trust().signed_operations().is_empty(), "{what}");
    }

    let mut fresh = Peer::open(LIST);
    assert_eq!(fresh.restore_sealed(ALICE, state, &sealed.key), Ok(1));
    assert_eq!(fresh.trust().heads(), alice.trust().heads());
    assert_eq!(
        fresh.standing().person,
        Some((ALICE, person(ALICE).public()))
    );
}

/// What a sealed state holds is not in it in the clear: not the seed of the person's key, not the
/// administrator's, not the key of an epoch, not the list's content. Under one key it is the same
/// bytes each time, which is what lets a recording hold it; a key drawn for each save makes other
/// bytes each time.
#[test]
fn a_sealed_state_holds_no_secret_and_no_content_in_the_clear() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice_with_group();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let mut bob = bob_with_a_key_package(&mut network);
    network.round(&mut alice, ALICE, RoundKind::Sync);
    alice.add_member(BOB, 2, person(BOB).public()).unwrap();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 3000, 5)),
        Kept::Taken
    );
    let epoch_keys = alice.trust().keys_for_backup();
    assert!(!epoch_keys.is_empty(), "alice holds the key of an epoch");

    let key = DataKey::from_bytes([42; 32]);
    let sealed = alice.save_sealed_under(&key).unwrap();
    let holds = |secret: &[u8]| sealed.windows(secret.len()).any(|window| window == secret);
    let (seed, administrator_seed) = ([ALICE as u8; 32], [9u8; 32]);
    assert!(!holds(&seed[..]), "the person's seed");
    assert!(!holds(&administrator_seed[..]), "the administrator's seed");
    for (epoch, epoch_key) in &epoch_keys {
        assert!(!holds(&epoch_key[..]), "the key of epoch {epoch}");
    }
    let clear: [&[u8]; 3] = [b"record_list", b"wallTime", b"genesis"];
    for text in clear {
        assert!(!holds(text), "{}", String::from_utf8_lossy(text));
    }

    assert_eq!(
        alice.save_sealed_under(&key).unwrap(),
        sealed,
        "one key, the same bytes"
    );
    let (first, second) = (alice.save_sealed().unwrap(), alice.save_sealed().unwrap());
    assert_ne!(
        first.key.as_bytes(),
        second.key.as_bytes(),
        "a key for each save"
    );
    assert_ne!(first.state, second.state);
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
    // An invited device signs nothing before a round has read it the log that grants it.
    network.round(&mut bob, BOB, RoundKind::Sync);
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

    let mut alice = restarted(alice, ALICE);
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
/// The core signs nothing new until a round has read her chain back, and keeps the write her machine
/// makes meanwhile: the round asks the relay for her operations past what the core holds right after
/// the relay's heads, tells her machine the two it held no more, signs the write that waited at 5
/// and gives it the relay. Sent again, it is the operation she holds. Bob holds one chain of hers,
/// not a frozen one.
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
    // The older state and its key, as a host would find them in a copy of its storage: they open
    // as well as the last, which is why the core waits for its own chain.
    let older = alice.save_sealed().unwrap();
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
    assert_eq!(
        alice
            .restore_sealed(ALICE, &older.state, &older.key)
            .unwrap(),
        2
    );
    assert!(alice.trust().awaits_own_chain());
    match alice.outbound(WRITE, &write(5, ALICE, 5000, 5)) {
        Kept::Waiting(reason) => assert!(reason.contains("signs nothing new"), "{reason}"),
        other => panic!("expected the write to wait, got {other:?}"),
    }
    assert_eq!(
        alice.outbound(WRITE, &write(1, ALICE, 1001, 1)),
        Kept::Held,
        "what the core holds is given as it was"
    );
    assert_eq!(alice.trust().last_seq(ALICE), 2, "nothing was signed");
    assert_eq!(alice.standing().waiting, 1);

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
    assert_eq!(back.round.pushed, 1, "the write that waited");
    assert_eq!(
        back.taken(),
        vec![(WRITE.to_string(), write(5, ALICE, 5000, 5))]
    );
    assert!(back.dropped().is_empty());
    assert!(!back.rebuild());
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
    assert_eq!(alice.trust().last_seq(ALICE), 5, "signed after her last");
    assert_eq!(alice.standing().waiting, 0);

    assert_eq!(
        alice.outbound(WRITE, &write(5, ALICE, 5000, 5)),
        Kept::Held,
        "the write that waited, signed once"
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

/// Bob's device is used before it has read anything of the list: his machine writes a field,
/// writes it again and notes that the second write is over the first, and sends the first again, as
/// a machine built again from its application's log does. His core can sign none of it yet and
/// keeps each, once, in the order it was made, and refuses at once another content under the stamp
/// of a write that waits. What waits is in his sealed state: his device stops and starts again from
/// it. His first round reads the log that lets him write, signs what waited in its order, and gives
/// it the relay in the same round; alice's machine is told it.
#[test]
fn what_a_device_sends_before_it_may_sign_waits_and_reaches_everyone_in_order() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    network.round(&mut alice, ALICE, RoundKind::Sync);

    let mut bob = invited(BOB);
    let sends = [
        (WRITE, write(1, BOB, 1000, 10)),
        (WRITE, write(1, BOB, 2000, 20)),
        (SUPERSEDE, supersede(1, BOB, 1000, BOB, 2000)),
    ];
    for (event, data) in &sends {
        match bob.outbound(event, data) {
            Kept::Waiting(reason) => assert!(reason.contains("has not begun"), "{reason}"),
            other => panic!("{event} {data}: expected it to wait, got {other:?}"),
        }
    }
    assert!(
        matches!(bob.outbound(WRITE, &sends[0].1), Kept::Waiting(_)),
        "sent again, it waits once"
    );
    match bob.outbound(WRITE, &write(1, BOB, 1000, 11)) {
        Kept::Refused(reason) => assert!(reason.contains("another content"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(bob.standing().waiting, 3);
    assert!(bob.trust().signed_operations().is_empty());

    let mut bob = restarted(bob, BOB);
    assert_eq!(
        bob.standing().waiting,
        3,
        "what waits is in the sealed state"
    );
    let first = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(first.refused().is_empty(), "{:?}", first.refused());
    assert!(first.dropped().is_empty());
    let in_order: Vec<(String, String)> = sends
        .iter()
        .map(|(event, data)| (event.to_string(), data.clone()))
        .collect();
    assert_eq!(first.taken(), in_order);
    assert_eq!(first.round.pushed, 3, "the three that waited");
    assert_eq!(bob.standing().waiting, 0);
    assert_eq!(bob.trust().last_seq(BOB), 3);

    let told = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(named(&told, WRITE).len(), 2, "{:?}", told.events());
    assert_eq!(named(&told, SUPERSEDE).len(), 1, "{:?}", told.events());
    assert_eq!(alice.trust().heads(), bob.trust().heads());
    assert!(alice.trust().equivocating().is_empty());
}

/// Bob's device reads the list before alice has granted him anything, and his machine writes: the
/// log his core holds names him nowhere yet, so the write waits. Alice grants him writing, and the
/// round of his that reads the grant signs the write and gives it the relay.
#[test]
fn a_write_made_before_the_grant_is_signed_by_the_round_that_reads_it() {
    let mut network = Network::new(Limits::default());
    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    alice.set_administrator(administrator());
    alice.open_list(Profile::Untrusted).unwrap();
    network.round(&mut alice, ALICE, RoundKind::Sync);

    let mut bob = invited(BOB);
    let early = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(
        early.stalled(),
        "the relay lets no one read who is no member"
    );
    assert_eq!(
        bob.trust().entries().len(),
        2,
        "the genesis and alice's grant"
    );
    match bob.outbound(WRITE, &write(1, BOB, 1000, 7)) {
        Kept::Waiting(reason) => {
            assert!(reason.contains("grants node 2 nothing yet"), "{reason}")
        }
        other => panic!("expected the write to wait, got {other:?}"),
    }

    alice.grant(BOB, 2, person(BOB).public()).unwrap();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let granted = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(granted.refused().is_empty(), "{:?}", granted.refused());
    assert_eq!(granted.taken().len(), 1, "{granted:?}");
    assert_eq!(granted.round.pushed, 1, "the write that waited");
    let told = network.round(&mut alice, ALICE, RoundKind::Sync);
    let writes = named(&told, WRITE);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(writes[0]["value"], "7");
}

/// A device whose machine wrote before it read the log, and whose log turns out not to let its
/// person write: carol may only read, and bob was taken off before his device first read the list.
/// The round that reads the log refuses what waited, each with why, and says so: the machine took
/// the write in and no other copy will hold it, so the host is told to build it again. Nothing was
/// signed, the relay was given nothing, nothing waits any more, and the same write sent again is
/// refused at once.
#[test]
fn what_waited_for_a_grant_that_does_not_let_the_person_write_is_refused_and_said() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    alice.remove_member(BOB).unwrap();
    network.round(&mut alice, ALICE, RoundKind::Sync);

    for (node, role) in [(CAROL, "role 1"), (BOB, "role 0")] {
        let mut device = invited(node);
        let sent = write(1, node, 1000, 1);
        assert!(matches!(device.outbound(WRITE, &sent), Kept::Waiting(_)));
        let read = network.round(&mut device, node, RoundKind::Sync);
        let dropped = read.dropped();
        assert_eq!(dropped.len(), 1, "{read:?}");
        assert_eq!(
            (dropped[0].0.as_str(), dropped[0].1.as_str()),
            (WRITE, sent.as_str())
        );
        assert!(
            dropped[0].2.contains("may not write") && dropped[0].2.contains(role),
            "{}",
            dropped[0].2
        );
        assert!(read.taken().is_empty());
        assert!(
            read.rebuild(),
            "the machine holds a write no other copy will"
        );
        assert_eq!(read.round.pushed, 0);
        assert_eq!(device.standing().waiting, 0);
        assert!(device.trust().signed_operations().is_empty());
        assert!(
            matches!(device.outbound(WRITE, &sent), Kept::Refused(_)),
            "sent again, it is refused at once"
        );
    }
}
