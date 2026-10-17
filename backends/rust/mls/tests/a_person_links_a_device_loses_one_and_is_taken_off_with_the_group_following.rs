// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A person with several devices, and the group that follows the log. Bob has a laptop (node 2) and
// links a phone (node 5): the log records that bob's device vouched for it, the group adds it, and a new
// device holds no key of the epochs before it joined until a device that holds them sends it. Bob then
// loses the laptop: the administrator takes that device off, the group moves to an epoch the laptop
// cannot derive, and the phone, which is a member of its own, goes on. Taking the person off is a removal
// for each of their devices.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_mls::host::{Issued, Member};
use sce_rust_mls::{Applied, Device, MlsError};
use sce_rust_trust::membership::{link_bytes, Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: usize = 0;
const BOB: usize = 1;
const CAROL: usize = 2;
const PHONE: usize = 3;
const NODES: [u64; 4] = [1, 2, 3, 5];

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn identity(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn write(author: u64, id: u64, text: &str) -> String {
    json!({"id": id.to_string(), "field": "3", "value": "0", "text": text,
           "wallTime": (1000 + id).to_string(), "counter": "0", "nodeId": author.to_string()})
    .to_string()
}

struct Net {
    members: Vec<Member>,
}

impl Net {
    /// alice (node 1) runs the group and holds the administrator's key; bob (2), carol (3) and
    /// bob's phone (5) are devices that exist and are in no group yet.
    fn begin() -> Self {
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
        let members: Vec<Member> = NODES
            .iter()
            .map(|node| {
                let trust = Trust::open(LIST);
                trust.append(&genesis).unwrap();
                trust.set_identity(*node, identity(*node));
                Member::new(Device::new(LIST, *node, identity(*node)), trust, *node)
            })
            .collect();
        let net = Self { members };
        let grant = net.members[ALICE]
            .trust()
            .draft(Body::Grant {
                node: 1,
                role: 3,
                key: identity(1).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        net.share(&[grant], &[ALICE, BOB, CAROL, PHONE]);
        net
    }

    fn share(&self, entries: &[Entry], to: &[usize]) {
        for index in to {
            for entry in entries {
                let _ = self.members[*index].trust().append(entry);
            }
        }
    }

    fn trust(&self, index: usize) -> Trust {
        self.members[index].trust().clone()
    }

    /// alice adds `index`, the log reaches everyone, and `index` joins (or follows) the group.
    fn add(&mut self, index: usize, parent: Option<usize>) -> Issued {
        let node = NODES[index];
        let package = self.members[index].device_mut().key_package().unwrap();
        let issued = match parent {
            None => self.members[ALICE]
                .add_member(&admin(), node, 2, identity(node).public(), &package)
                .unwrap(),
            Some(parent) => {
                let statement = link_bytes(LIST, NODES[parent], node, &identity(node).public());
                self.members[ALICE]
                    .add_linked_member(
                        &admin(),
                        node,
                        2,
                        identity(node).public(),
                        NODES[parent],
                        identity(NODES[parent]).sign(&statement),
                        identity(node).sign(&statement),
                        &package,
                    )
                    .unwrap()
            }
        };
        self.share(&issued.entries, &[BOB, CAROL, PHONE]);
        self.deliver(&issued, Some(index));
        issued
    }

    /// The commit reaches every device that is in the group, and the Welcome the one it adds.
    fn deliver(&mut self, issued: &Issued, joining: Option<usize>) {
        for index in 0..self.members.len() {
            if index == ALICE {
                continue;
            }
            let trust = self.trust(index);
            let device = self.members[index].device_mut();
            if Some(index) == joining {
                device
                    .join(issued.welcome.as_ref().unwrap(), &issued.commit, &trust)
                    .unwrap();
                device.publish_epoch_key(&trust).unwrap();
            } else if device.epoch().is_ok() && device.is_active() {
                if let Ok(Applied::Epoch(_)) = device.apply_commit(&issued.commit, &trust) {
                    device.publish_epoch_key(&trust).unwrap();
                }
            }
        }
    }

    fn sealed(&self, index: usize, id: u64, text: &str) -> String {
        let mut host = self.trust(index);
        host.seal("#server", "todo.write", &write(NODES[index], id, text))
            .unwrap()
            .unwrap()
    }

    fn opens(&self, index: usize, op: &str) -> Result<String, String> {
        let mut host = self.trust(index);
        host.admit("relay", "todo.write", op).map(|told| {
            serde_json::from_str::<Value>(&told.unwrap()).unwrap()["text"]
                .as_str()
                .unwrap()
                .to_string()
        })
    }
}

#[test]
fn a_linked_device_gets_the_history_a_device_of_the_same_person_sends_it_and_no_other() {
    let mut net = Net::begin();
    net.members[ALICE].device_mut().create_group().unwrap();
    net.add(BOB, None);
    let sealed_1 = net.sealed(ALICE, 1, "written in epoch one");
    net.add(CAROL, None);
    let sealed_2 = net.sealed(ALICE, 2, "written in epoch two");

    // Bob links his phone: the log says bob's device vouched for it, and the group adds it.
    net.add(PHONE, Some(BOB));
    for index in [ALICE, BOB, CAROL, PHONE] {
        assert_eq!(net.trust(index).person_of(5), 2, "the phone is bob's");
        assert_eq!(net.trust(index).devices_of(5), vec![2, 5]);
    }
    let key = net.members[ALICE].device().epoch_key().unwrap();
    for index in [BOB, CAROL, PHONE] {
        assert_eq!(net.members[index].device().epoch_key().unwrap(), key);
    }

    // The phone holds the key of epoch 3 only: what was written before is not opened for it.
    assert_eq!(net.trust(PHONE).epochs(), vec![3]);
    for op in [&sealed_1, &sealed_2] {
        let reason = net.opens(PHONE, op).unwrap_err();
        assert!(reason.contains("holds no key of epoch"), "{reason}");
    }

    // Bob's laptop sends it the keys of the epochs it holds, and the phone opens what it could not.
    let bob_key = identity(5).public();
    let grants = net.members[BOB].history_grants(&bob_key, &[1, 2, 7]);
    assert_eq!(grants.len(), 2, "an epoch bob holds no key of is left out");
    let phone = net.trust(PHONE);
    for (_, grant) in &grants {
        phone.receive_grant(grant).unwrap();
    }
    assert_eq!(phone.epochs(), vec![1, 2, 3]);
    assert_eq!(net.opens(PHONE, &sealed_1).unwrap(), "written in epoch one");
    assert_eq!(net.opens(PHONE, &sealed_2).unwrap(), "written in epoch two");
}

#[test]
fn a_lost_device_is_taken_off_and_the_persons_other_device_goes_on_until_the_person_is_taken_off() {
    let mut net = Net::begin();
    net.members[ALICE].device_mut().create_group().unwrap();
    net.add(BOB, None);
    net.add(CAROL, None);
    net.add(PHONE, Some(BOB));
    let laptop_key = net.members[BOB].device().epoch_key().unwrap();

    // The laptop is lost: the administrator takes that device off, and the group moves on.
    let issued = net.members[ALICE].remove_member(&admin(), 2).unwrap();
    net.share(&issued.entries, &[BOB, CAROL, PHONE]);
    net.deliver(&issued, None);
    assert_eq!(
        net.members[BOB].device().epoch_key(),
        Err(MlsError::Removed)
    );
    let phone_key = net.members[PHONE].device().epoch_key().unwrap();
    assert_eq!(phone_key, net.members[ALICE].device().epoch_key().unwrap());
    assert_eq!(net.members[CAROL].device().epoch_key().unwrap(), phone_key);
    assert_ne!(phone_key, laptop_key);
    let after = net.sealed(ALICE, 3, "after the laptop");
    assert!(net.opens(CAROL, &after).is_ok());
    assert!(net.opens(PHONE, &after).is_ok());
    assert!(net
        .opens(BOB, &after)
        .unwrap_err()
        .contains("holds no key of epoch 4"));

    // Taking the person off is a removal for each device: the laptop already is, the phone is now.
    let issued = net.members[ALICE].remove_person(&admin(), 2).unwrap();
    assert_eq!(issued.len(), 1, "only the phone was still a member");
    net.share(&issued[0].entries, &[BOB, CAROL, PHONE]);
    net.deliver(&issued[0], None);
    assert_eq!(
        net.members[PHONE].device().epoch_key(),
        Err(MlsError::Removed)
    );
    assert_eq!(
        net.members[CAROL].device().epoch_key().unwrap(),
        net.members[ALICE].device().epoch_key().unwrap()
    );
    for index in [ALICE, BOB, CAROL, PHONE] {
        assert_eq!(net.trust(index).role(2), 0);
        assert_eq!(net.trust(index).role(5), 0);
        assert_eq!(
            net.trust(index).devices_of(2),
            vec![2, 5],
            "the log remembers whose they were"
        );
    }
}
