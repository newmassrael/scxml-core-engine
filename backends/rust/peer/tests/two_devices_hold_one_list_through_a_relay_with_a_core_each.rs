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
// that what alice writes under the group's epoch opens on bob's device; that a list keyed by its
// group is sealed from its first write, so that what alice writes alone reaches the relay sealed and
// opens for bob once he is admitted and given the key of the epoch before his, and that a list whose
// log names no epoch of its group signs nothing in the clear; that what the relay is owed
// of such an act outlives a save and a failed round; that an administrator's act the core cannot do
// is refused with its reason and writes nothing; that a device taken off is refused by the relay;
// and that a device whose machine is built again from its own log of commands writes nothing twice,
// refuses another content under a stamp it signed, and, restored from a save older than what it
// published, reads its own chain back from the relay before it signs, so that no host holds a frozen
// chain of it; that the same holds on a list keyed by its group, with sends made before or after
// the resync round and a write of another device's among them; and that a device whose chain the
// relay holds past a conflict reads it back past the relay's head and signs nothing again.
//
// What a device keeps is one sealed state, its secrets with the rest, and every restore here is into
// a core given nothing but the list: that a device restored so goes on with the protocol as it would
// have (rounds, writes, the administrator's acts, the group's epochs); that a state is refused whole,
// with nothing taken, under another key, changed, cut, or for another list or node; and that no
// secret and no content is in it in the clear.

use sce_rust_peer::{
    CourierError, DataKey, Device, GroupStep, Kept, LogError, MlsError, Outcome, Peer, PeerError,
    Profile, Publication, RoundKind, SealedState, OWNER,
};
use sce_rust_relay::{Authority, Code, Limits, MemoryStore, Record, Relay, Request, Response};
use sce_rust_trust::membership::{Body, Entry, Log};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const CHANGE: &str = "record_list.change";
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

/// One write of `value` and `text` to `field` of row `id`, as a change holds it: a 64-bit number as
/// text, a narrower one as a number, and the dot of its change, which it does not carry, 0.
fn put(id: u32, field: u16, value: i64, text: &str) -> Value {
    json!({"kind": 0, "id": id, "field": field, "value": value.to_string(), "text": text,
           "wallTime": "0", "counter": 0, "nodeId": "0"})
}

/// One write-over of the write of row `id`, field 3, that `node` stamped at `wall`.
fn put_over(id: u32, node: u64, wall: i64) -> Value {
    json!({"kind": 1, "id": id, "field": 3, "value": "0", "text": "",
           "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()})
}

/// A change of `node`'s stamped at `wall`, as the record list's `record_list.change` carries it.
fn change(node: u64, wall: i64, ops: Vec<Value>) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": ops})
        .to_string()
}

/// A change of one write of `value` to field 3 of row `id`, stamped by `node` at `wall`.
fn write(id: u32, node: u64, wall: i64, value: i64) -> String {
    change(node, wall, vec![put(id, 3, value, "")])
}

/// A change of one write of the title `text` to field 1 of row `id`, stamped by `node` at `wall`.
fn titled(id: u32, node: u64, wall: i64, text: &str) -> String {
    change(node, wall, vec![put(id, 1, 0, text)])
}

/// The change of `by`'s stamped at `at` that writes over the write of row `id`, field 3, that
/// `node` stamped at `wall`, and writes the field anew with `value`.
fn rewrite(id: u32, node: u64, wall: i64, by: u64, at: i64, value: i64) -> String {
    change(
        by,
        at,
        vec![put_over(id, node, wall), put(id, 3, value, "")],
    )
}

/// The operations of `kind` of every change a round told the machine, in their order, each with
/// its change's author as `by`.
fn ops_told(outcome: &Outcome, kind: u64) -> Vec<Value> {
    named(outcome, CHANGE)
        .into_iter()
        .flat_map(|change| {
            let by = change["by"].clone();
            change["ops"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(move |op| op["kind"] == kind)
                .map(move |mut op| {
                    op["by"] = by.clone();
                    op
                })
        })
        .collect()
}

/// The writes of every change a round told the machine.
fn writes(outcome: &Outcome) -> Vec<Value> {
    ops_told(outcome, 0)
}

/// The write-overs of every change a round told the machine.
fn write_overs(outcome: &Outcome) -> Vec<Value> {
    ops_told(outcome, 1)
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

    /// What the relay holds of the list, read from the start of its feed by alice's device.
    fn records_held(&mut self) -> Vec<Record> {
        let text = self.carry(
            ALICE,
            &Request::Feed {
                cursor: 0,
                limit: 500,
            }
            .to_json(),
        );
        let Ok(Response::Feed { records, more, .. }) = Response::from_json(&text) else {
            panic!("the relay gives its feed: {text}");
        };
        assert!(!more, "the feed is read whole");
        records
    }

    /// The writes of every change the relay holds of the list, each as the change it keeps carries
    /// it.
    fn writes_held(&mut self) -> Vec<Value> {
        self.records_held()
            .into_iter()
            .filter_map(|record| match record {
                Record::Operation { event, data } if event == CHANGE => {
                    serde_json::from_str::<Value>(&data).ok()
                }
                _ => None,
            })
            .flat_map(|change| change["ops"].as_array().cloned().unwrap_or_default())
            .filter(|op| op["kind"] == 0)
            .collect()
    }

    /// How many operations the relay holds of the list, one for each change, the proof of a
    /// conflict included: what grows by one for each operation a device signs and gives it.
    fn operations_held(&mut self) -> usize {
        self.records_held()
            .iter()
            .filter(|record| matches!(record, Record::Operation { .. }))
            .count()
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
        alice.outbound(CHANGE, &write(1, ALICE, 1000, 10)),
        Kept::Taken
    );
    let first = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(first.refused().is_empty(), "{:?}", first.refused());
    assert_eq!(first.round.pushed, 4 + 1, "the log and the write");
    assert!(
        named(&first, CHANGE).is_empty(),
        "alice's machine is not told its own write back"
    );

    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(told.refused().is_empty(), "{:?}", told.refused());
    assert!(!told.stalled());
    assert_eq!(named(&told, ROLE).len(), 3, "the roster the log makes");
    let changes = named(&told, CHANGE);
    assert_eq!(changes.len(), 1);
    assert!(
        changes[0].get("sig").is_none() && changes[0].get("seq").is_none(),
        "the machine is told the change, not the operation it travelled as: {}",
        changes[0]
    );
    let first = writes(&told);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0]["by"], "1");
    assert_eq!(first[0]["value"], "10");
    assert_eq!(first[0]["id"], 1);
    assert_eq!(
        told.events()[0].0,
        ROLE,
        "the roster comes before the operations"
    );

    // Alice writes the field again over her first write: one change of a write-over and a write,
    // one operation, told whole.
    assert_eq!(
        alice.outbound(CHANGE, &rewrite(1, ALICE, 1000, ALICE, 2000, 20)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let names: Vec<String> = told.events().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec![CHANGE.to_string()]);
    let notes = write_overs(&told);
    assert_eq!(notes[0]["by"], "1");
    assert_eq!(notes[0]["wallTime"], "1000");
    assert_eq!(writes(&told)[0]["value"], "20");

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
            alice.outbound(CHANGE, &write(n, ALICE, 1000 + i64::from(n), i64::from(n))),
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
    let mut rows: Vec<Value> = writes(&told)
        .into_iter()
        .map(|write| write["id"].clone())
        .collect();
    rows.sort_by_key(|id| id.as_u64());
    assert_eq!(
        rows,
        vec![json!(1), json!(2), json!(3), json!(4)],
        "every change once, the two taken before the stop included"
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
    match carol.outbound(CHANGE, &write(5, CAROL, 1000, 1)) {
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
    assert!(named(&told, CHANGE).is_empty(), "{:?}", told.events());

    let mut nobody = Peer::invited(LIST, genesis().hash());
    network.round(&mut nobody, BOB, RoundKind::Sync);
    assert!(matches!(
        nobody.outbound(CHANGE, &write(6, BOB, 1000, 1)),
        Kept::Refused(_)
    ));
    assert_eq!(
        alice.outbound("record_list.edit", r#"{"id":1}"#),
        Kept::NotSigned
    );
    // A part of a change as the list sent it before a change was one message is no event that is
    // signed (docs/adr/0085, section 1.4).
    let part =
        r#"{"id":1,"field":3,"value":"1","text":"","wallTime":"1000","counter":0,"nodeId":"1"}"#;
    assert_eq!(alice.outbound("record_list.write", part), Kept::NotSigned);
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
    assert_eq!(
        alice.standing().owed,
        3,
        "the commit, his Welcome and the key of the epoch before his"
    );
    assert_eq!(alice.trust().role(BOB), 2);
    let admitted = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(admitted.refused().is_empty(), "{:?}", admitted.refused());
    assert_eq!(admitted.owed, 0);
    assert_eq!(
        admitted.round.pushed, 2,
        "the grant and the entry that names the commit"
    );

    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(joined.steps, vec![GroupStep::Joined(2)], "{joined:?}");
    assert!(joined.refused().is_empty(), "{:?}", joined.refused());
    assert_eq!(
        bob.group().unwrap().device().epoch_key(),
        alice.group().unwrap().device().epoch_key()
    );
    assert_eq!(
        bob.trust().epochs(),
        vec![1, 2],
        "the epoch the opening began, given, and his own"
    );
    assert_eq!(bob.standing().keys, alice.standing().keys);

    assert_eq!(
        alice.outbound(CHANGE, &write(1, ALICE, 3000, 5)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = writes(&told);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(writes[0]["value"], "5", "opened under the group's key");

    // Alice takes him off the list and the group in one act: the group moves to an epoch his
    // device holds no key of, and what she writes under it reaches him no more.
    alice.remove_member(BOB).unwrap();
    assert_eq!(alice.standing().owed, 1, "the commit, and no Welcome");
    let removed = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(removed.refused().is_empty(), "{:?}", removed.refused());
    assert_eq!(removed.owed, 0);
    assert_eq!(alice.trust().epochs(), vec![1, 2, 3]);
    assert_eq!(
        alice.outbound(CHANGE, &write(2, ALICE, 4000, 6)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let cut = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(named(&cut, CHANGE).is_empty(), "{:?}", cut.events());
    assert_eq!(bob.standing().role, 0);
    assert_eq!(
        bob.trust().epochs(),
        vec![1, 2],
        "no key of the epoch after"
    );
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
    assert_eq!(alice.standing().owed, 3);
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
    assert_eq!(lost.owed, 3, "{lost:?}");
    assert!(
        lost.refused().iter().any(|why| why.contains("owed")),
        "{:?}",
        lost.refused()
    );
    assert_eq!(alice.standing().owed, 3);

    let given = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(given.owed, 0, "{given:?}");
    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(joined.steps, vec![GroupStep::Joined(2)], "{joined:?}");
    assert_eq!(bob.trust().epochs(), vec![1, 2]);
}

/// A list keyed by its group is sealed from its first write. Its opening moves the group of one to
/// an epoch the log names, before anything is written, so what alice's device writes while she is
/// alone in the list (a title and a number) reaches the relay sealed: the relay holds no value and
/// no text of it in the clear. Bob's device, admitted after, joins at the epoch that admits him and
/// is given the key of the first, and his machine is told both writes, opened, in the round that
/// reads them.
#[test]
fn what_the_first_device_writes_alone_is_sealed_at_the_relay_and_opens_for_one_admitted_later() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice_with_group();
    let bodies: Vec<Body> = alice
        .trust()
        .entries()
        .iter()
        .map(|entry| entry.body.clone())
        .collect();
    assert_eq!(bodies.len(), 3, "{bodies:?}");
    assert!(
        matches!(bodies[2], Body::MlsEpoch { epoch: 1, .. }),
        "the entry that names the first epoch follows the owner's grant: {bodies:?}"
    );
    let standing = alice.standing();
    assert_eq!((standing.epoch, standing.keys.clone()), (1, vec![1]));
    assert_eq!(standing.owed, 1, "the commit of the first epoch");

    assert_eq!(
        alice.outbound(CHANGE, &titled(1, ALICE, 1000, "buy milk")),
        Kept::Taken
    );
    assert_eq!(
        alice.outbound(CHANGE, &write(2, ALICE, 1500, 7)),
        Kept::Taken
    );
    let opened = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(opened.refused().is_empty(), "{:?}", opened.refused());
    assert_eq!(
        opened.owed, 0,
        "the relay took the commit of the first epoch"
    );

    let held = network.writes_held();
    assert_eq!(held.len(), 2, "{held:?}");
    for write in &held {
        assert_eq!(write["value"], "0", "no number in the clear: {write}");
        let text = write["text"].as_str().unwrap_or_default();
        assert!(
            !text.is_empty(),
            "the content is sealed into the text: {write}"
        );
        assert!(!text.contains("buy milk"), "no title in the clear: {write}");
    }

    let mut bob = bob_with_a_key_package(&mut network);
    network.round(&mut alice, ALICE, RoundKind::Sync);
    alice.add_member(BOB, 2, person(BOB).public()).unwrap();
    let admitted = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(admitted.refused().is_empty(), "{:?}", admitted.refused());
    assert_eq!(
        admitted.owed, 0,
        "the commit, the Welcome and the key given"
    );

    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(joined.refused().is_empty(), "{:?}", joined.refused());
    assert_eq!(joined.steps, vec![GroupStep::Joined(2)], "{joined:?}");
    assert_eq!(bob.trust().epochs(), vec![1, 2]);
    let told = writes(&joined);
    assert_eq!(told.len(), 2, "{:?}", joined.events());
    assert!(
        told.iter().any(|write| write["text"] == "buy milk"),
        "the title opened: {told:?}"
    );
    assert!(
        told.iter().any(|write| write["value"] == "7"),
        "the number opened: {told:?}"
    );

    // What bob writes is sealed under the epoch he joined at, and opens on alice's device; the
    // relay still holds nothing in the clear.
    assert_eq!(bob.outbound(CHANGE, &write(3, BOB, 2000, 9)), Kept::Taken);
    network.round(&mut bob, BOB, RoundKind::Sync);
    let theirs = network.round(&mut alice, ALICE, RoundKind::Sync);
    let from_bob = writes(&theirs);
    assert_eq!(from_bob.len(), 1, "{:?}", theirs.events());
    assert_eq!(from_bob[0]["value"], "9");
    let held = network.writes_held();
    assert_eq!(held.len(), 3, "{held:?}");
    assert!(held.iter().all(|write| write["value"] == "0"), "{held:?}");
}

/// A list an earlier core opened, whose log names no epoch of its group (the genesis and the
/// owner's grant, and the group made with no entry after them): what its device writes waits, and is
/// not signed in the clear, until the log names an epoch. Admitting the first member names one;
/// the write that waited is signed under it by the next round, reaches the relay sealed, and opens
/// for the member, who joined at that epoch and is given no key before it, since there is none.
#[test]
fn a_list_whose_log_names_no_epoch_of_its_group_signs_no_write_until_it_names_one() {
    let mut network = Network::new(Limits::default());
    let mut device = Device::new(LIST, ALICE, person(ALICE));
    device.create_group().unwrap();
    let mut alice = Peer::open(LIST).with_group(device, ALICE);
    alice.set_identity(ALICE, person(ALICE));
    alice.set_administrator(administrator());
    alice.trust().append(&genesis()).unwrap();
    alice
        .trust()
        .administer(
            &administrator(),
            alice
                .trust()
                .grant_body(ALICE, OWNER, person(ALICE).public()),
        )
        .unwrap();
    assert_eq!(alice.standing().epoch, 0);

    let kept = alice.outbound(CHANGE, &titled(1, ALICE, 1000, "buy milk"));
    assert!(
        matches!(&kept, Kept::Waiting(why) if why.contains("names no epoch")),
        "{kept:?}"
    );
    assert_eq!(alice.standing().waiting, 1);
    let opened = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(opened.refused().is_empty(), "{:?}", opened.refused());
    assert!(
        network.writes_held().is_empty(),
        "nothing signed, and nothing in the relay's clear"
    );

    let mut bob = bob_with_a_key_package(&mut network);
    network.round(&mut alice, ALICE, RoundKind::Sync);
    alice.add_member(BOB, 2, person(BOB).public()).unwrap();
    assert_eq!(
        alice.standing().owed,
        2,
        "the commit and his Welcome: no epoch before his to give"
    );
    let admitted = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(admitted.refused().is_empty(), "{:?}", admitted.refused());
    assert_eq!(admitted.taken().len(), 1, "the write that waited, signed");
    assert_eq!(alice.standing().waiting, 0);
    let held = network.writes_held();
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0]["value"], "0");
    assert!(
        !held[0].to_string().contains("buy milk"),
        "sealed: {}",
        held[0]
    );

    let joined = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(joined.steps, vec![GroupStep::Joined(1)], "{joined:?}");
    let again = network.round(&mut bob, BOB, RoundKind::Sync);
    let mut told = writes(&joined);
    told.extend(writes(&again));
    assert_eq!(told.len(), 1, "{:?} {:?}", joined.events(), again.events());
    assert_eq!(told[0]["text"], "buy milk");
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
    assert_eq!(joined.steps, vec![GroupStep::Joined(2)], "{joined:?}");
    assert_eq!(
        bob.trust().epochs(),
        vec![1, 2],
        "the key of the epoch before his was given from the key alice's state kept"
    );

    let epoch_key = bob.group().unwrap().device().epoch_key();
    let mut bob = restarted(bob, BOB);
    assert_eq!(
        bob.standing().keys,
        vec![1, 2],
        "the keys of the epochs came back"
    );
    assert_eq!(bob.group().unwrap().device().epoch().ok(), Some(2));
    assert_eq!(bob.group().unwrap().device().epoch_key(), epoch_key);
    assert_eq!(alice.group().unwrap().device().epoch_key(), epoch_key);
    network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(!bob.awaits_own_chain());

    assert_eq!(
        alice.outbound(CHANGE, &write(1, ALICE, 3000, 5)),
        Kept::Taken
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = writes(&told);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(
        writes[0]["value"], "5",
        "opened under the key the state kept"
    );
    assert_eq!(bob.outbound(CHANGE, &write(2, BOB, 3500, 6)), Kept::Taken);
    network.round(&mut bob, BOB, RoundKind::Sync);
    let theirs = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(named(&theirs, CHANGE).len(), 1, "{:?}", theirs.events());

    alice.remove_member(BOB).unwrap();
    let removed = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(removed.refused().is_empty(), "{:?}", removed.refused());
    assert_eq!(alice.trust().epochs(), vec![1, 2, 3]);
    network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(bob.standing().role, 0);
    assert_eq!(
        bob.trust().epochs(),
        vec![1, 2],
        "no key of the epoch after"
    );
}

/// A sealed state is refused whole, and the core it was given to takes nothing of it, when the key is
/// another, when a byte of it was changed, when it was cut, and when it is another list's or another
/// node's; the right key for the right device then restores it.
#[test]
fn a_sealed_state_is_refused_whole_under_another_key_changed_cut_or_for_another_device() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    assert_eq!(
        alice.outbound(CHANGE, &write(1, ALICE, 1000, 10)),
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
        alice.outbound(CHANGE, &write(1, ALICE, 3000, 5)),
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
    assert_eq!(bob.outbound(CHANGE, &write(1, BOB, 1000, 1)), Kept::Taken);
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

    assert_eq!(bob.outbound(CHANGE, &write(2, BOB, 2000, 2)), Kept::Taken);
    let cut = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(cut.round.pushed, 0);
    assert_eq!(cut.refused().len(), 1, "{:?}", cut.refused());
    assert_eq!(bob.standing().role, 0);
    let after = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(named(&after, CHANGE).is_empty(), "{:?}", after.events());
}

/// Alice's application keeps its own log of commands and builds her machine again from it after
/// her device restarted from its save: the machine sends every change again, as it made it. The
/// core gives each back as the operation it holds, the relay is given nothing, and bob's machine is
/// told nothing twice; a resync tells the rebuilt machine the list.
#[test]
fn a_device_that_replays_its_own_log_after_a_restart_writes_nothing_twice() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    let mut bob = invited(BOB);
    let sends = [
        (CHANGE, write(1, ALICE, 1000, 10)),
        (CHANGE, rewrite(1, ALICE, 1000, ALICE, 2000, 20)),
        (CHANGE, write(2, ALICE, 3000, 30)),
    ];
    for (event, data) in &sends {
        assert_eq!(alice.outbound(event, data), Kept::Taken, "{event} {data}");
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    assert_eq!(named(&told, CHANGE).len(), 3);
    assert_eq!(write_overs(&told).len(), 1);
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
        named(&replayed, CHANGE).len(),
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
        alice.outbound(CHANGE, &write(1, ALICE, 1000, 10)),
        Kept::Taken
    );
    let before = alice.trust().signed_operations();
    match alice.outbound(CHANGE, &write(1, ALICE, 1000, 11)) {
        Kept::Refused(reason) => assert!(reason.contains("another content"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(alice.trust().signed_operations(), before);
    let round = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(round.round.pushed, 4 + 1, "the log and the one write");

    let mut bob = invited(BOB);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = writes(&told);
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
            alice.outbound(CHANGE, &write(n, ALICE, 1000 + i64::from(n), 1)),
            Kept::Taken
        );
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    // The older state and its key, as a host would find them in a copy of its storage: they open
    // as well as the last, which is why the core waits for its own chain.
    let older = alice.save_sealed().unwrap();
    for n in 3..=4 {
        assert_eq!(
            alice.outbound(CHANGE, &write(n, ALICE, 1000 + i64::from(n), 1)),
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
    match alice.outbound(CHANGE, &write(5, ALICE, 5000, 5)) {
        Kept::Waiting(reason) => assert!(reason.contains("signs nothing new"), "{reason}"),
        other => panic!("expected the write to wait, got {other:?}"),
    }
    assert_eq!(
        alice.outbound(CHANGE, &write(1, ALICE, 1001, 1)),
        Kept::Held,
        "what the core holds is given as it was"
    );
    assert_eq!(alice.trust().last_seq(ALICE), 2, "nothing was signed");
    assert_eq!(alice.standing().waiting, 1);
    // Her machine, built again from her commands, sends a write the save does not hold: it waits
    // too, and the round finds it held once it has read her chain back.
    assert!(
        matches!(
            alice.outbound(CHANGE, &write(3, ALICE, 1003, 1)),
            Kept::Waiting(_)
        ),
        "the save does not hold it"
    );
    assert_eq!(alice.standing().waiting, 2);

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
        vec![(CHANGE.to_string(), write(5, ALICE, 5000, 5))],
        "the write signed now, and no other"
    );
    assert_eq!(
        back.held(),
        vec![(CHANGE.to_string(), write(3, ALICE, 1003, 1))],
        "the write her chain, read back, held"
    );
    assert!(back.dropped().is_empty());
    assert!(!back.rebuild());
    let rows: Vec<Value> = writes(&back)
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
        alice.outbound(CHANGE, &write(5, ALICE, 5000, 5)),
        Kept::Held,
        "the write that waited, signed once"
    );
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let told = network.round(&mut bob, BOB, RoundKind::Sync);
    let writes = writes(&told);
    assert_eq!(writes.len(), 1, "{:?}", told.events());
    assert_eq!(writes[0]["id"], 5);
    assert!(bob.trust().equivocating().is_empty(), "no frozen chain");
    assert_eq!(bob.trust().last_seq(ALICE), 5);
    assert_eq!(alice.trust().heads(), bob.trust().heads());
}

/// Bob's device is used before it has read anything of the list: his machine writes a field,
/// writes it again over the first in one change, writes another, and sends the first again, as a
/// machine built again from its application's log does. His core can sign none of it yet and keeps
/// each, once, in the order it was made, and refuses at once another content under the dot of a
/// change that waits. What waits is in his sealed state: his device stops and starts again from
/// it. His first round reads the log that lets him write, signs what waited in its order, and gives
/// it the relay in the same round; alice's machine is told it.
#[test]
fn what_a_device_sends_before_it_may_sign_waits_and_reaches_everyone_in_order() {
    let mut network = Network::new(Limits::default());
    let mut alice = alice();
    network.round(&mut alice, ALICE, RoundKind::Sync);

    let mut bob = invited(BOB);
    let sends = [
        (CHANGE, write(1, BOB, 1000, 10)),
        (CHANGE, rewrite(1, BOB, 1000, BOB, 2000, 20)),
        (CHANGE, write(2, BOB, 3000, 30)),
    ];
    for (event, data) in &sends {
        match bob.outbound(event, data) {
            Kept::Waiting(reason) => assert!(reason.contains("has not begun"), "{reason}"),
            other => panic!("{event} {data}: expected it to wait, got {other:?}"),
        }
    }
    assert!(
        matches!(bob.outbound(CHANGE, &sends[0].1), Kept::Waiting(_)),
        "sent again, it waits once"
    );
    match bob.outbound(CHANGE, &write(1, BOB, 1000, 11)) {
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
    assert_eq!(named(&told, CHANGE).len(), 3, "{:?}", told.events());
    assert_eq!(write_overs(&told).len(), 1, "{:?}", told.events());
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
    match bob.outbound(CHANGE, &write(1, BOB, 1000, 7)) {
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
    let writes = writes(&told);
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
        assert!(matches!(device.outbound(CHANGE, &sent), Kept::Waiting(_)));
        let read = network.round(&mut device, node, RoundKind::Sync);
        let dropped = read.dropped();
        assert_eq!(dropped.len(), 1, "{read:?}");
        assert_eq!(
            (dropped[0].0.as_str(), dropped[0].1.as_str()),
            (CHANGE, sent.as_str())
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
            matches!(device.outbound(CHANGE, &sent), Kept::Refused(_)),
            "sent again, it is refused at once"
        );
    }
}

/// Alice's device and bob's, both writers, of a list keyed by its group when `keyed` (bob admitted
/// to the group and given its key) and of one with no keys otherwise: what each restore below
/// starts from.
fn two_writers(network: &mut Network, keyed: bool) -> (Peer, Peer) {
    if keyed {
        let mut alice = alice_with_group();
        network.round(&mut alice, ALICE, RoundKind::Sync);
        let mut bob = bob_with_a_key_package(network);
        network.round(&mut alice, ALICE, RoundKind::Sync);
        alice.add_member(BOB, 2, person(BOB).public()).unwrap();
        network.round(&mut alice, ALICE, RoundKind::Sync);
        let joined = network.round(&mut bob, BOB, RoundKind::Sync);
        assert_eq!(joined.steps, vec![GroupStep::Joined(2)], "{joined:?}");
        (alice, bob)
    } else {
        let mut alice = alice();
        network.round(&mut alice, ALICE, RoundKind::Sync);
        let mut bob = invited(BOB);
        network.round(&mut bob, BOB, RoundKind::Sync);
        (alice, bob)
    }
}

/// Alice's core restored from `sealed`, as a host starts it.
fn restored_from(sealed: &SealedState) -> Peer {
    let mut again = made_again(ALICE);
    again
        .restore_sealed(ALICE, &sealed.state, &sealed.key)
        .expect("the state opens under its key");
    again
}

/// Where alice's device comes back from before her machine sends again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Restore {
    /// It does not stop: the machine is built again in the same run.
    None,
    /// The state it saved after its last round.
    Last,
    /// A state it saved after its first write, older than what it signed after.
    Older,
}

/// Alice's machine, built again from her own log of commands, sends every change she made
/// again: on a list keyed by its group and on one with no keys, before and after the resync round
/// that tells the rebuilt machine the list, with no restart, after a restart from the state saved
/// last, and after one from an older state, and with a write of bob's that her machine wrote over
/// and shares on the resync, or with none. In every case her core signs nothing new, the relay gains
/// no operation, and nothing is refused. What waited because the older state did not hold it is
/// named held once the round has read her chain back, and is in no `taken`.
#[test]
fn a_device_whose_machine_sends_its_writes_again_signs_nothing_new_in_any_case() {
    for keyed in [false, true] {
        for restore in [Restore::None, Restore::Last, Restore::Older] {
            for before_resync in [true, false] {
                for theirs in [false, true] {
                    sends_again_and_signs_nothing(keyed, restore, before_resync, theirs);
                }
            }
        }
    }
}

fn sends_again_and_signs_nothing(keyed: bool, restore: Restore, before_resync: bool, theirs: bool) {
    let case = format!(
        "keyed={keyed} restore={restore:?} sends_before_the_resync={before_resync} bob={theirs}"
    );
    let mut network = Network::new(Limits::default());
    let (mut alice, mut bob) = two_writers(&mut network, keyed);
    let bobs = write(7, BOB, 900, 7);
    if theirs {
        assert_eq!(bob.outbound(CHANGE, &bobs), Kept::Taken, "{case}");
        network.round(&mut bob, BOB, RoundKind::Sync);
        network.round(&mut alice, ALICE, RoundKind::Sync);
    }

    let first = (CHANGE, write(1, ALICE, 1000, 10));
    assert_eq!(alice.outbound(first.0, &first.1), Kept::Taken, "{case}");
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let older = alice.save_sealed().unwrap();
    let mut after = vec![(CHANGE, rewrite(1, ALICE, 1000, ALICE, 2000, 20))];
    if theirs {
        // Alice writes over bob's write and writes the field anew, in one change.
        after.push((CHANGE, rewrite(7, BOB, 900, ALICE, 2500, 70)));
    }
    for (event, data) in &after {
        assert_eq!(alice.outbound(event, data), Kept::Taken, "{case}: {data}");
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let signed = alice.trust().last_seq(ALICE);
    assert_eq!(signed, 1 + after.len() as u64, "{case}");
    let held_before = network.operations_held();

    let mut alice = match restore {
        Restore::None => alice,
        Restore::Last => restarted(alice, ALICE),
        Restore::Older => {
            drop(alice);
            restored_from(&older)
        }
    };
    // What the rebuilt machine sends: every change of hers, in the order she made them, and bob's
    // change, which it holds and is given again.
    let mut replay = vec![first.clone()];
    replay.extend(after.iter().cloned());
    if theirs {
        replay.push((CHANGE, bobs.clone()));
    }
    let lost: Vec<(String, String)> = if restore == Restore::Older {
        after
            .iter()
            .map(|(event, data)| (event.to_string(), data.clone()))
            .collect()
    } else {
        Vec::new()
    };
    let send_all = |alice: &mut Peer, waits_for_her_chain: bool| {
        for (event, data) in &replay {
            let kept = alice.outbound(event, data);
            let waits = waits_for_her_chain && lost.contains(&(event.to_string(), data.clone()));
            if waits {
                assert!(
                    matches!(&kept, Kept::Waiting(why) if why.contains("signs nothing new")),
                    "{case}: {data} waits for her chain, and is {kept:?}"
                );
            } else {
                assert_eq!(kept, Kept::Held, "{case}: {data}");
            }
        }
    };
    if before_resync {
        send_all(&mut alice, true);
    }
    let resync = network.round(&mut alice, ALICE, RoundKind::Resync);
    if !before_resync {
        send_all(&mut alice, false);
    }

    assert!(
        resync.refused().is_empty(),
        "{case}: {:?}",
        resync.refused()
    );
    assert!(resync.taken().is_empty(), "{case}: {:?}", resync.taken());
    assert!(
        resync.dropped().is_empty(),
        "{case}: {:?}",
        resync.dropped()
    );
    assert_eq!(resync.round.pushed, 0, "{case}: the relay is given nothing");
    let held = if before_resync { lost } else { Vec::new() };
    assert_eq!(resync.held(), held, "{case}");
    assert_eq!(alice.trust().last_seq(ALICE), signed, "{case}: nothing new");
    assert_eq!(alice.standing().waiting, 0, "{case}");
    assert!(!alice.awaits_own_chain(), "{case}");
    assert!(alice.trust().equivocating().is_empty(), "{case}");
    let again = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert_eq!(again.round.pushed, 0, "{case}");
    assert_eq!(
        network.operations_held(),
        held_before,
        "{case}: the relay gained no operation"
    );
    network.round(&mut bob, BOB, RoundKind::Sync);
    assert!(bob.trust().equivocating().is_empty(), "{case}");
    assert_eq!(bob.trust().last_seq(ALICE), signed, "{case}");
}

/// A second core of alice's device, made from the trust of an older state of hers (its log
/// `entries`, its text `trust_text` and its keys of epochs `keys`) and not told to wait for her
/// chain: what a second process of one application, or a copy of its storage put back, is. What it
/// signs is at a number her device used already.
fn careless_copy(entries: &[Entry], trust_text: &str, keys: &[(u32, [u8; 32])]) -> Trust {
    let copy = Trust::open(LIST);
    for entry in entries {
        copy.append(entry).unwrap();
    }
    copy.set_identity(ALICE, person(ALICE));
    for (epoch, key) in keys {
        assert!(copy.set_epoch_key(*epoch, *key));
    }
    copy.restore_from_text(trust_text);
    copy
}

/// The relay holds alice's chain past a conflict: a second core of her device, made from an older
/// state and not waiting for her chain, signed another write at a number she had used, so the relay
/// holds two operations of hers there and its head of her stops below them. Her device then comes
/// back from that older state, and her machine sends the changes it made after it. Read
/// back by the relay's heads alone, her chain would seem to end where the state ends: her core would
/// sign them again at the numbers after it, under the log as it is now, and the relay would take
/// each as one more operation of hers, at every restart. Her core reads her chain back from where it
/// and the relay agree to where the relay holds nothing more, finds the conflict, and signs nothing:
/// what waited is refused, each with why, the machine is to be built again, and the relay gains
/// nothing. On a list keyed by its group and on one with no keys.
#[test]
fn a_device_whose_chain_the_relay_holds_past_a_conflict_reads_it_back_and_signs_nothing_again() {
    for keyed in [false, true] {
        reads_its_chain_past_a_conflict(keyed);
    }
}

fn reads_its_chain_past_a_conflict(keyed: bool) {
    let case = format!("keyed={keyed}");
    let mut network = Network::new(Limits::default());
    let (mut alice, _bob) = two_writers(&mut network, keyed);
    for n in 1..=2 {
        let data = write(n, ALICE, 1000 + i64::from(n), i64::from(n));
        assert_eq!(alice.outbound(CHANGE, &data), Kept::Taken, "{case}");
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    let older = alice.save_sealed().unwrap();
    let older_entries = alice.trust().entries();
    let older_text = alice.trust().to_text();
    let older_keys = alice.trust().keys_for_backup();
    let after = [
        (CHANGE, write(3, ALICE, 1003, 3)),
        (CHANGE, rewrite(3, ALICE, 1003, ALICE, 1004, 4)),
    ];
    for (event, data) in &after {
        assert_eq!(alice.outbound(event, data), Kept::Taken, "{case}: {data}");
    }
    network.round(&mut alice, ALICE, RoundKind::Sync);
    // The log moves on: bob is made a reader.
    alice.grant(BOB, 1, person(BOB).public()).unwrap();
    network.round(&mut alice, ALICE, RoundKind::Sync);
    drop(alice);

    let copy = careless_copy(&older_entries, &older_text, &older_keys);
    let other = copy
        .seal_sent(CHANGE, &write(9, ALICE, 5000, 9))
        .unwrap()
        .expect("a write is signed");
    assert!(other.signed, "{case}");
    let submitted = network.carry(
        ALICE,
        &Request::Submit {
            event: CHANGE.to_string(),
            data: other.data,
        }
        .to_json(),
    );
    assert!(
        matches!(
            Response::from_json(&submitted),
            Ok(Response::Refused {
                code: Code::Conflict {
                    node: ALICE,
                    seq: 3,
                    frozen: 3,
                    ..
                },
                ..
            })
        ),
        "{case}: the relay holds it as proof, and answers that it conflicts: {submitted}"
    );
    let held_before = network.operations_held();

    let mut alice = restored_from(&older);
    for (event, data) in &after {
        assert!(
            matches!(alice.outbound(event, data), Kept::Waiting(_)),
            "{case}: {data} waits: the state does not hold it"
        );
    }
    let back = network.round(&mut alice, ALICE, RoundKind::Sync);
    assert!(
        back.taken().is_empty(),
        "{case}: nothing is signed again: {:?}",
        back.taken()
    );
    assert_eq!(back.round.pushed, 0, "{case}: {back:?}");
    assert_eq!(
        network.operations_held(),
        held_before,
        "{case}: the relay gained no operation"
    );
    assert_eq!(
        alice.trust().equivocating(),
        vec![(ALICE, 3)],
        "{case}: her core found the conflict the relay held"
    );
    assert_eq!(
        back.conflicted(),
        Some(3),
        "{case}: the round that found it says so"
    );
    let dropped = back.dropped();
    assert_eq!(dropped.len(), after.len(), "{case}: {back:?}");
    for ((event, data, why), (sent, made)) in dropped.iter().zip(&after) {
        assert_eq!((event.as_str(), data.as_str()), (*sent, made.as_str()));
        assert!(why.contains("conflicts at 3"), "{case}: {why}");
    }
    assert!(back.rebuild(), "{case}");
    assert_eq!(alice.standing().waiting, 0, "{case}");
    assert!(!alice.awaits_own_chain(), "{case}");
}
