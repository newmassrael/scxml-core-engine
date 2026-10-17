// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The group key agreement of a list (MLS, RFC 9420) bound to the list's membership log. What is asked
// here: that devices the log admitted agree on the key of each epoch and seal and open content under
// it; that a commit the log did not name is not applied, by the device that made it or by the
// others; that a device the log has no grant for (or has with another key) is not added; that a
// group whose roster is not the log's is undone; that a Welcome of another group with the same
// members is not the list's; that a removed device derives the key of no later epoch and opens
// nothing sealed after its removal; that a device keeps its place across a restart; and that a
// section has a key of its own.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_mls::{Applied, Device, MlsError};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn identity(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

/// Every host of the list, whose logs the administrator's entries reach.
struct Hosts {
    trusts: Vec<(u64, Trust)>,
}

impl Hosts {
    fn new(nodes: &[u64]) -> Self {
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
        let trusts = nodes
            .iter()
            .map(|node| {
                let trust = Trust::open(LIST);
                trust.append(&genesis).unwrap();
                trust.set_identity(*node, identity(*node));
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

    /// An entry the administrator signs reaches every host.
    fn publish(&self, body: Body) -> Entry {
        let entry = self.trusts[0].1.draft(body).sign(&admin()).finish();
        for (_, trust) in &self.trusts {
            trust.append(&entry).unwrap();
        }
        entry
    }

    fn grant(&self, node: u64, role: u8) {
        self.publish(Body::Grant {
            node,
            role,
            key: identity(node).public(),
            final_seq: None,
        });
    }

    fn remove(&self, node: u64) {
        self.publish(Body::Grant {
            node,
            role: 0,
            key: identity(node).public(),
            final_seq: Some(self.of(node).last_seq(node)),
        });
    }

    fn name(&self, epoch: u32, commit: [u8; 32]) {
        self.publish(Body::MlsEpoch { epoch, commit });
    }
}

fn device(node: u64) -> Device {
    Device::new(LIST, node, identity(node))
}

/// alice (node 1, who runs the group) alone in it, as the list begins.
fn begun() -> (Hosts, Device) {
    let hosts = Hosts::new(&[1, 2, 3]);
    hosts.grant(1, 3);
    let mut alice = device(1);
    alice.create_group().unwrap();
    alice.verify_roster(hosts.of(1)).unwrap();
    (hosts, alice)
}

/// alice adds `joiner` (whose key package is `package`), the log names the commit, alice merges and
/// the joiner joins; returns the Welcome and the commit.
fn add(
    hosts: &Hosts,
    alice: &mut Device,
    joiner: &mut Device,
    node: u64,
    package: &[u8],
) -> (Vec<u8>, Vec<u8>) {
    hosts.grant(node, 2);
    let pending = alice.stage_add(&[package.to_vec()], hosts.of(1)).unwrap();
    hosts.name(pending.epoch as u32, pending.commit_hash);
    alice.merge_pending(hosts.of(1)).unwrap();
    let welcome = pending.welcome.clone().unwrap();
    joiner
        .join(&welcome, &pending.commit, hosts.of(node))
        .unwrap();
    (welcome, pending.commit)
}

fn write(author: u64, id: u64, text: &str) -> String {
    json!({"id": id.to_string(), "field": "3", "value": "0", "text": text,
           "wallTime": (1000 + id).to_string(), "counter": "0", "nodeId": author.to_string()})
    .to_string()
}

#[test]
fn devices_the_log_admitted_agree_on_the_key_of_each_epoch_and_seal_under_it() {
    let (hosts, mut alice) = begun();
    let mut bob = device(2);
    let package = bob.key_package().unwrap();
    add(&hosts, &mut alice, &mut bob, 2, &package);

    assert_eq!(alice.epoch(), Ok(1));
    assert_eq!(bob.epoch(), Ok(1));
    let key = alice.epoch_key().unwrap();
    assert_eq!(bob.epoch_key().unwrap(), key, "one group, one key");
    assert_eq!(alice.members().unwrap(), bob.members().unwrap());

    // The key is what the host seals under: alice writes, bob's host opens it, and the hub's
    // (no key) holds only what is sealed.
    alice.publish_epoch_key(hosts.of(1)).unwrap();
    bob.publish_epoch_key(hosts.of(2)).unwrap();
    let mut alice_host = hosts.of(1).clone();
    let sent = alice_host
        .seal("#server", "todo.write", &write(1, 1, "buy milk"))
        .unwrap()
        .unwrap();
    let mut bob_host = hosts.of(2).clone();
    let told = bob_host
        .admit("alice", "todo.write", &sent)
        .unwrap()
        .unwrap();
    let told: Value = serde_json::from_str(&told).unwrap();
    assert_eq!(told["text"], "buy milk");
}

#[test]
fn a_commit_the_log_did_not_name_is_applied_by_no_one() {
    let (hosts, mut alice) = begun();
    let mut bob = device(2);
    let package = bob.key_package().unwrap();
    hosts.grant(2, 2);
    let pending = alice.stage_add(&[package], hosts.of(1)).unwrap();

    // The commit is made and the log does not name it: alice does not merge it.
    assert_eq!(
        alice.merge_pending(hosts.of(1)),
        Err(MlsError::CommitNotNamed { epoch: 1 })
    );
    // Nor does the log naming another commit make it the one.
    hosts.name(1, [3; 32]);
    assert_eq!(
        alice.merge_pending(hosts.of(1)),
        Err(MlsError::CommitNotNamed { epoch: 1 })
    );
    assert_eq!(alice.epoch(), Ok(0));
    alice.clear_pending().unwrap();

    // A device that is handed the commit and not the log's word applies nothing either.
    let mut carol = device(3);
    let welcome = pending.welcome.clone().unwrap();
    assert!(matches!(
        carol.join(&welcome, &pending.commit, hosts.of(3)),
        Err(MlsError::CommitNotNamed { epoch: 1 }) | Err(MlsError::Library(_))
    ));
}

#[test]
fn a_device_the_log_has_no_grant_for_or_has_with_another_key_is_not_added() {
    let (hosts, mut alice) = begun();

    // No grant at all.
    let mut stranger = device(2);
    let package = stranger.key_package().unwrap();
    assert!(matches!(
        alice.stage_add(&[package], hosts.of(1)),
        Err(MlsError::NotInLog(_))
    ));

    // A grant, and a device that signs with another key.
    hosts.grant(2, 2);
    let mut impostor = Device::new(LIST, 2, Identity::from_seed([77; 32]));
    let package = impostor.key_package().unwrap();
    assert!(matches!(
        alice.stage_add(&[package], hosts.of(1)),
        Err(MlsError::NotInLog(_))
    ));
    // The one the log gave the key to is added.
    let package = stranger.key_package().unwrap();
    assert!(alice.stage_add(&[package], hosts.of(1)).is_ok());
}

#[test]
fn a_group_whose_roster_is_not_the_logs_is_undone() {
    let (hosts, mut alice) = begun();
    let mut bob = device(2);
    let package = bob.key_package().unwrap();
    // The log has bob and carol, and the commit adds only bob.
    hosts.grant(2, 2);
    hosts.grant(3, 2);
    let pending = alice.stage_add(&[package], hosts.of(1)).unwrap();
    hosts.name(1, pending.commit_hash);
    let error = alice.merge_pending(hosts.of(1)).unwrap_err();
    assert!(matches!(error, MlsError::RosterDiffers(_)), "{error:?}");
    assert_eq!(alice.epoch(), Ok(0), "the commit was undone");
    assert!(alice.is_active());
    assert_eq!(alice.members().unwrap().len(), 1);
}

#[test]
fn a_welcome_of_another_group_with_the_same_members_is_not_the_lists() {
    let (hosts, mut alice) = begun();
    let mut bob = device(2);
    let for_the_rogue = bob.key_package().unwrap();
    let for_the_list = bob.key_package().unwrap();

    // Someone who holds alice's key makes a group of their own with the same list name and adds bob.
    let mut rogue = device(1);
    rogue.create_group().unwrap();
    hosts.grant(2, 2);
    let fake = rogue.stage_add(&[for_the_rogue], hosts.of(1)).unwrap();

    let pending = alice.stage_add(&[for_the_list], hosts.of(1)).unwrap();
    hosts.name(1, pending.commit_hash);
    alice.merge_pending(hosts.of(1)).unwrap();

    // Bob is handed the fake Welcome with the commit the log names: the group it makes does not
    // hold that commit's confirmation tag.
    let welcome = fake.welcome.clone().unwrap();
    assert_eq!(
        bob.join(&welcome, &pending.commit, hosts.of(2)),
        Err(MlsError::WelcomeNotTheLogs)
    );
    assert_eq!(bob.epoch(), Err(MlsError::NoGroup));
    // With the fake commit too, the log does not name it.
    assert_eq!(
        bob.join(&welcome, &fake.commit, hosts.of(2)),
        Err(MlsError::CommitNotNamed { epoch: 1 })
    );
    // The list's own Welcome is taken.
    bob.join(&pending.welcome.unwrap(), &pending.commit, hosts.of(2))
        .unwrap();
    assert_eq!(bob.epoch_key().unwrap(), alice.epoch_key().unwrap());
}

/// A removed device derives no key of a later epoch, and what is sealed after its removal does not
/// open for it, while the devices left agree on the new key.
#[test]
fn a_removed_device_derives_no_key_of_a_later_epoch_and_opens_nothing_sealed_after() {
    let (hosts, mut alice) = begun();
    let mut bob = device(2);
    let mut carol = device(3);
    let (bob_package, carol_package) = (bob.key_package().unwrap(), carol.key_package().unwrap());
    add(&hosts, &mut alice, &mut bob, 2, &bob_package);

    // Carol is added in epoch 2, which bob applies.
    hosts.grant(3, 2);
    let pending = alice.stage_add(&[carol_package], hosts.of(1)).unwrap();
    hosts.name(2, pending.commit_hash);
    alice.merge_pending(hosts.of(1)).unwrap();
    assert_eq!(
        bob.apply_commit(&pending.commit, hosts.of(2)),
        Ok(Applied::Epoch(2))
    );
    carol
        .join(&pending.welcome.unwrap(), &pending.commit, hosts.of(3))
        .unwrap();
    let before = [&alice, &bob, &carol].map(|d| d.epoch_key().unwrap());
    assert!(before.iter().all(|key| *key == before[0]));
    for (node, device) in [(1, &alice), (2, &bob), (3, &carol)] {
        device.publish_epoch_key(hosts.of(node)).unwrap();
    }

    // The administrator takes bob off, and the group moves to epoch 3 without him.
    hosts.remove(2);
    let removal = alice.stage_remove(&[2], hosts.of(1)).unwrap();
    hosts.name(3, removal.commit_hash);
    alice.merge_pending(hosts.of(1)).unwrap();
    assert_eq!(
        carol.apply_commit(&removal.commit, hosts.of(3)),
        Ok(Applied::Epoch(3))
    );
    assert_eq!(
        bob.apply_commit(&removal.commit, hosts.of(2)),
        Ok(Applied::Removed)
    );

    let after = alice.epoch_key().unwrap();
    assert_eq!(carol.epoch_key().unwrap(), after);
    assert_ne!(after, before[0], "the key changed");
    assert_eq!(bob.epoch_key(), Err(MlsError::Removed));
    assert!(!bob.is_active());
    // And he cannot make up the key from the one he had.
    assert_ne!(before[1], after);

    // What alice seals under epoch 3 opens for carol and not for bob.
    alice.publish_epoch_key(hosts.of(1)).unwrap();
    carol.publish_epoch_key(hosts.of(3)).unwrap();
    let mut alice_host = hosts.of(1).clone();
    let sent = alice_host
        .seal("#server", "todo.write", &write(1, 2, "after bob"))
        .unwrap()
        .unwrap();
    let mut carol_host = hosts.of(3).clone();
    assert!(carol_host.admit("alice", "todo.write", &sent).is_ok());
    let mut bob_host = hosts.of(2).clone();
    let reason = bob_host.admit("alice", "todo.write", &sent).unwrap_err();
    assert!(reason.contains("no key of epoch 3"), "{reason}");
}

#[test]
fn a_device_keeps_its_place_in_the_group_across_a_restart() {
    let (hosts, mut alice) = begun();
    let mut bob = device(2);
    let package = bob.key_package().unwrap();
    add(&hosts, &mut alice, &mut bob, 2, &package);
    let key = alice.epoch_key().unwrap();

    let kept = alice.to_bytes().unwrap();
    let restarted = Device::from_bytes(LIST, 1, identity(1), &kept).unwrap();
    assert_eq!(restarted.epoch(), Ok(1));
    assert_eq!(restarted.epoch_key().unwrap(), key);
    assert_eq!(restarted.members().unwrap(), alice.members().unwrap());

    // The restarted device goes on: it adds carol, and bob follows.
    let mut alice = restarted;
    let mut carol = device(3);
    let package = carol.key_package().unwrap();
    hosts.grant(3, 2);
    let pending = alice.stage_add(&[package], hosts.of(1)).unwrap();
    hosts.name(2, pending.commit_hash);
    alice.merge_pending(hosts.of(1)).unwrap();
    assert_eq!(
        bob.apply_commit(&pending.commit, hosts.of(2)),
        Ok(Applied::Epoch(2))
    );
    assert_eq!(bob.epoch_key().unwrap(), alice.epoch_key().unwrap());
}

#[test]
fn a_section_has_a_key_of_its_own_that_every_member_of_the_epoch_derives() {
    let (hosts, mut alice) = begun();
    let mut bob = device(2);
    let package = bob.key_package().unwrap();
    add(&hosts, &mut alice, &mut bob, 2, &package);

    let chores = alice.section_key("chores").unwrap();
    let finances = alice.section_key("finances").unwrap();
    assert_eq!(bob.section_key("chores").unwrap(), chores);
    assert_eq!(bob.section_key("finances").unwrap(), finances);
    assert_ne!(chores, finances);
    assert_ne!(chores, alice.epoch_key().unwrap());
    assert_ne!(
        chores,
        alice.section_key("chores ").unwrap(),
        "the name is the whole name"
    );
}
