// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A person has several devices, each with its own key and its own node, and the membership log is
// where it is written down which are whose. What is asked here: that a device a member linked is a
// member the log accepts only when the member and the device both signed for it, with no more than
// the member's role and as a node the log has not held; that the log says which devices are one
// person's; that what a device made and the administrator's host had not seen is not lost when other
// writers' checkpoints, made before the removal, show it (an amendment), and is when they do not; and
// that an epoch the list had destroyed the keys of opens for no host that keeps to it, and that
// destroying one takes the quorum and an epoch the list has moved past.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::membership::{link_bytes, Body, Entry, Log, LogError, Profile};
use sce_rust_trust::{GrantError, Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";

fn admin(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn genesis(quorum: u8) -> Entry {
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![admin(9).public(), admin(10).public()],
            quorum,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin(9))
    .finish()
}

/// Hosts of the list, whose logs the administrators' entries reach.
struct Hosts {
    trusts: Vec<(u64, Trust)>,
}

impl Hosts {
    /// `nodes`: the hosts that exist; node 0 is a hub that holds no key.
    fn new(nodes: &[u64], quorum: u8) -> Self {
        let genesis = genesis(quorum);
        let trusts = nodes
            .iter()
            .map(|node| {
                let trust = Trust::open(LIST);
                trust.append(&genesis).unwrap();
                if *node != 0 {
                    trust.set_identity(*node, person(*node));
                }
                (*node, trust)
            })
            .collect();
        Self { trusts }
    }

    fn of(&self, node: u64) -> &Trust {
        &self
            .trusts
            .iter()
            .find(|(held, _)| *held == node)
            .unwrap()
            .1
    }

    fn entry(&self, body: Body) -> Entry {
        self.trusts[0].1.draft(body).sign(&admin(9)).finish()
    }

    fn publish(&self, entry: &Entry) -> Result<(), LogError> {
        for (_, trust) in &self.trusts {
            trust.append(entry)?;
        }
        Ok(())
    }

    fn grant(&self, node: u64, role: u8) {
        self.publish(&self.entry(Body::Grant {
            node,
            role,
            key: person(node).public(),
            final_seq: None,
        }))
        .unwrap();
    }
}

/// The body that links `node` to `parent`, signed by both devices as each holds its own key.
fn linked(list: &str, parent: u64, node: u64, role: u8) -> Body {
    let statement = link_bytes(list, parent, node, &person(node).public());
    Body::LinkedGrant {
        node,
        role,
        key: person(node).public(),
        parent,
        parent_signature: person(parent).sign(&statement),
        signature: person(node).sign(&statement),
    }
}

fn write(author: u64, id: u64, wall: i64, text: &str) -> String {
    json!({"id": id.to_string(), "field": "3", "value": "0", "text": text,
           "wallTime": wall.to_string(), "counter": "0", "nodeId": author.to_string()})
    .to_string()
}

fn sent(host: &mut Trust, author: u64, id: u64, wall: i64) -> String {
    host.seal("#server", "todo.write", &write(author, id, wall, ""))
        .unwrap()
        .unwrap()
}

#[test]
fn a_member_links_a_new_device_and_the_log_says_whose_it_is() {
    let hosts = Hosts::new(&[0, 1, 5], 1);
    hosts.grant(1, 3);
    hosts.publish(&hosts.entry(linked(LIST, 1, 5, 2))).unwrap();
    for node in [0, 1, 5] {
        assert_eq!(hosts.of(node).person_of(5), 1);
        assert_eq!(hosts.of(node).person_of(1), 1);
        assert_eq!(hosts.of(node).devices_of(5), vec![1, 5]);
        assert_eq!(hosts.of(node).role(5), 2);
    }

    // The linked device writes, as any member does, and its operation is admitted.
    let mut phone = hosts.of(5).clone();
    let op = sent(&mut phone, 5, 1, 1000);
    assert!(hosts
        .of(0)
        .clone()
        .admit("phone", "todo.write", &op)
        .is_ok());

    // The entry reads back from the JSON it is sent as.
    let entry = hosts.entry(linked(LIST, 1, 7, 1));
    assert_eq!(Entry::from_json(&entry.to_json()), Some(entry));
}

#[test]
fn a_link_that_the_member_or_the_device_did_not_sign_or_that_asks_for_more_is_refused() {
    let hosts = Hosts::new(&[0, 1, 2], 1);
    hosts.grant(1, 2);
    hosts.grant(2, 1);
    let refused = |body: Body| hosts.publish(&hosts.entry(body)).unwrap_err();

    // More than the parent's role.
    assert_eq!(
        refused(linked(LIST, 1, 5, 3)),
        LogError::Invalid("a linked device has no more than its parent's role")
    );
    // No role at all.
    assert_eq!(
        refused(linked(LIST, 1, 5, 0)),
        LogError::Invalid("a linked device has a role of 1 to 3")
    );
    // A node that is no member links nothing.
    assert_eq!(
        refused(linked(LIST, 6, 5, 1)),
        LogError::Invalid("a device is linked by a member")
    );
    // A node the log has held is not a new device.
    assert_eq!(
        refused(linked(LIST, 1, 2, 1)),
        LogError::Invalid("a linked device is a node the log has not held before")
    );
    // The statement is for one list, one parent, one node and one key.
    let statement = link_bytes(LIST, 1, 5, &person(5).public());
    let good = |parent_signature, signature| Body::LinkedGrant {
        node: 5,
        role: 1,
        key: person(5).public(),
        parent: 1,
        parent_signature,
        signature,
    };
    assert_eq!(
        refused(good(person(2).sign(&statement), person(5).sign(&statement))),
        LogError::BadSignature,
        "signed for the member by another device"
    );
    assert_eq!(
        refused(good(person(1).sign(&statement), person(2).sign(&statement))),
        LogError::BadSignature,
        "the new key did not sign"
    );
    let other_list = link_bytes("work", 1, 5, &person(5).public());
    assert_eq!(
        refused(good(
            person(1).sign(&other_list),
            person(5).sign(&other_list)
        )),
        LogError::BadSignature,
        "a statement of another list"
    );
    let other_key = link_bytes(LIST, 1, 5, &person(6).public());
    assert_eq!(
        refused(good(person(1).sign(&other_key), person(5).sign(&other_key))),
        LogError::BadSignature,
        "a statement for another key"
    );
    assert!(hosts
        .publish(&hosts.entry(good(person(1).sign(&statement), person(5).sign(&statement))))
        .is_ok());
}

/// Removing a person is a removal for each device: the log does not take the others off by itself.
#[test]
fn taking_a_person_off_is_a_removal_for_each_of_their_devices() {
    let hosts = Hosts::new(&[0, 1, 5], 1);
    hosts.grant(1, 2);
    hosts.publish(&hosts.entry(linked(LIST, 1, 5, 2))).unwrap();

    let remove = |node: u64| Body::Grant {
        node,
        role: 0,
        key: person(node).public(),
        final_seq: Some(0),
    };
    hosts.publish(&hosts.entry(remove(1))).unwrap();
    assert_eq!(hosts.of(0).role(1), 0);
    assert_eq!(
        hosts.of(0).role(5),
        2,
        "the linked device is still a member"
    );
    for device in hosts.of(0).devices_of(1) {
        if hosts.of(0).role(device) > 0 {
            hosts.publish(&hosts.entry(remove(device))).unwrap();
        }
    }
    assert_eq!(hosts.of(0).role(5), 0);
    assert_eq!(
        hosts.of(0).devices_of(5),
        vec![1, 5],
        "the log remembers whose they were"
    );
}

/// What bob made after the administrator's host last heard of him, and that carol's host holds, is
/// not lost when carol's checkpoint, made before he was taken off, shows it.
#[test]
fn what_a_removed_device_made_and_others_held_is_kept_when_their_checkpoints_show_it() {
    let hosts = Hosts::new(&[0, 1, 2, 3], 1);
    hosts.grant(1, 3);
    hosts.grant(2, 2);
    hosts.grant(3, 2);

    // bob makes three operations; carol's host holds them all, the hub (and so the administrator's
    // host) only the first.
    let mut bob = hosts.of(2).clone();
    let ops: Vec<String> = (1..=3)
        .map(|n| sent(&mut bob, 2, n, 1000 + n as i64))
        .collect();
    let mut carol = hosts.of(3).clone();
    let mut hub = hosts.of(0).clone();
    for op in &ops {
        carol.admit("bob", "todo.write", op).unwrap();
    }
    hub.admit("bob", "todo.write", &ops[0]).unwrap();
    let before_removal = hosts.of(3).checkpoint(30_000, 7).unwrap();
    assert_eq!(
        before_removal
            .heads
            .iter()
            .find(|h| h.node == 2)
            .unwrap()
            .seq,
        3
    );

    // The administrator takes bob off at the last operation the hub saw.
    let removal = hosts.entry(Body::Grant {
        node: 2,
        role: 0,
        key: person(2).public(),
        final_seq: Some(hub.last_seq(2)),
    });
    hosts.publish(&removal).unwrap();
    let after_removal = hosts.of(3).checkpoint(40_000, 8).unwrap();
    for op in &ops[1..] {
        assert!(
            hub.admit("bob", "todo.write", op).is_err(),
            "past the last accepted"
        );
    }

    let amend = |node: u64, final_seq: u64, evidence: Vec<_>| {
        hosts.publish(&hosts.entry(Body::Amend {
            node,
            final_seq,
            evidence,
        }))
    };
    let invalid = |reason: &'static str| -> Result<(), LogError> { Err(LogError::Invalid(reason)) };
    let shown = "an amendment is shown by a checkpoint of another writer, made before the removal";

    // No evidence, evidence made after the removal, evidence that does not reach, and evidence of the
    // device itself, show nothing.
    assert_eq!(amend(2, 3, vec![]), invalid(shown));
    assert_eq!(amend(2, 3, vec![after_removal]), invalid(shown));
    assert_eq!(amend(2, 4, vec![before_removal.clone()]), invalid(shown));
    assert!(
        host_of(2, &hosts).checkpoint(30_000, 7).is_err(),
        "a device that is off the log makes no checkpoint"
    );
    // A device that is a member is not amended, and the amendment raises and does not lower.
    assert_eq!(
        amend(3, 9, vec![before_removal.clone()]),
        invalid("only a device taken off is amended")
    );
    assert_eq!(
        amend(2, 1, vec![before_removal.clone()]),
        invalid("an amendment raises the last accepted")
    );

    // Carol's checkpoint, made before the removal, shows bob's chain to 3.
    assert_eq!(amend(2, 3, vec![before_removal]), Ok(()));
    for op in &ops[1..] {
        assert!(hub.admit("bob", "todo.write", op).is_ok(), "now accepted");
    }
    assert_eq!(hub.last_seq(2), 3);
}

fn host_of(node: u64, hosts: &Hosts) -> Trust {
    hosts.of(node).clone()
}

#[test]
fn destroying_an_epoch_takes_the_quorum_and_an_epoch_the_list_has_moved_past() {
    let hosts = Hosts::new(&[0, 1], 2);
    // With a quorum of two, an entry that two administrators sign.
    let two = |body: Body| {
        hosts.trusts[0]
            .1
            .draft(body)
            .sign(&admin(9))
            .sign(&admin(10))
            .finish()
    };
    let one = |body: Body| hosts.trusts[0].1.draft(body).sign(&admin(9)).finish();
    hosts
        .publish(&one(Body::Grant {
            node: 1,
            role: 2,
            key: person(1).public(),
            final_seq: None,
        }))
        .unwrap();
    for epoch in 1..=2 {
        hosts.publish(&one(Body::Epoch { epoch })).unwrap();
    }

    let shred = |epoch| Body::Shred { epoch };
    assert!(matches!(
        hosts.publish(&one(shred(1))),
        Err(LogError::NotEnoughSignatures { needed: 2, had: 1 })
    ));
    assert_eq!(
        hosts.publish(&two(shred(2))),
        Err(LogError::Invalid(
            "an epoch is destroyed after the list has moved past it"
        )),
        "the epoch the list is in is not destroyed"
    );
    assert_eq!(
        hosts.publish(&two(shred(0))),
        Err(LogError::Invalid(
            "an epoch is destroyed after the list has moved past it"
        ))
    );
    assert_eq!(hosts.publish(&two(shred(1))), Ok(()));
    assert_eq!(
        hosts.publish(&two(shred(1))),
        Err(LogError::Invalid("an epoch is destroyed once"))
    );
    assert_eq!(hosts.of(1).shredded(), vec![1]);
}

/// A host that holds the key of an epoch destroys it when the log says so, takes no other, and
/// refuses what was sealed under it for good; a hub that holds no key carries it still.
#[test]
fn a_host_that_keeps_to_the_list_destroys_the_key_and_opens_nothing_sealed_under_it() {
    let hosts = Hosts::new(&[0, 1, 2], 1);
    hosts.grant(1, 2);
    hosts.grant(2, 2);
    let epoch_entry = |epoch| hosts.entry(Body::Epoch { epoch });
    hosts.publish(&epoch_entry(1)).unwrap();
    for node in [1, 2] {
        hosts.of(node).set_epoch_key(1, [5; 32]);
    }
    let mut alice = hosts.of(1).clone();
    let sealed_1 = alice
        .seal("#server", "todo.write", &write(1, 1, 1000, "old secret"))
        .unwrap()
        .unwrap();
    hosts.publish(&epoch_entry(2)).unwrap();
    for node in [1, 2] {
        hosts.of(node).set_epoch_key(2, [6; 32]);
    }
    assert_eq!(hosts.of(2).epochs(), vec![1, 2]);
    // Before the destruction, bob opens it.
    let mut bob = hosts.of(2).clone();
    let told = bob
        .admit("alice", "todo.write", &sealed_1)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&told).unwrap()["text"],
        "old secret"
    );

    // A key of epoch 1 that alice sent bob before the destruction, and bob has not taken in.
    let old_grant = hosts
        .of(1)
        .grant_key(1, &person(2).public())
        .expect("alice holds epoch 1");

    hosts
        .publish(&hosts.entry(Body::Shred { epoch: 1 }))
        .unwrap();
    assert_eq!(
        hosts.of(2).receive_grant(&old_grant),
        Err(GrantError::Shredded)
    );
    for node in [1, 2] {
        assert_eq!(
            hosts.of(node).epochs(),
            vec![2],
            "the key of epoch 1 is gone"
        );
        assert!(
            !hosts.of(node).set_epoch_key(1, [5; 32]),
            "and none is taken"
        );
    }
    // A host that reads the content of epoch 1 again from a store finds it unreadable, and does not
    // park it: no key will come.
    let mut fresh = Trust::open(LIST);
    for entry in hosts.of(0).entries() {
        fresh.append(&entry).unwrap();
    }
    fresh.set_identity(2, person(2));
    fresh.set_epoch_key(2, [6; 32]);
    let reason = fresh.admit("hub", "todo.write", &sealed_1).unwrap_err();
    assert!(reason.contains("destroyed"), "{reason}");
    assert!(matches!(
        fresh.take_operation("todo.write", &sealed_1),
        Err(sce_rust_trust::TakeError::Refused(_))
    ));
    // The hub holds no key, and carries what it is given, as it always did.
    let mut hub = hosts.of(0).clone();
    assert!(hub.admit("alice", "todo.write", &sealed_1).is_ok());

    // A key of the epoch that a member sends is not taken either.
    let grant = hosts
        .of(1)
        .grant_key(2, &person(2).public())
        .expect("alice holds epoch 2");
    assert!(hosts.of(2).receive_grant(&grant).is_ok());
    assert_eq!(
        hosts.of(1).grant_key(1, &person(2).public()),
        None,
        "alice holds none of epoch 1"
    );
}
