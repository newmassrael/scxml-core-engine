// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A shared list whose content key is an MLS group's, carried through a relay that holds no key. The
// administrator's device runs the group: a device leaves a key package with the relay, the
// administrator adds it (the log names the commit), and the new member joins from the Welcome the
// relay carries; the members' machines write and agree on one list, sealed under the key of each epoch.
// When the administrator takes a member off, the group moves to an epoch the member cannot derive: the
// relay no longer lets that device read, and what is written afterwards opens for the members that are
// left and for no one else.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use sce_rust_mesh::endpoint::Admission;
use sce_rust_mls::host::{Member, Step};
use sce_rust_mls::Device;
use sce_rust_relay::{
    Authority, Courier, DirectLink, Keep, Limits, MemoryStore, Record, Relay, Request, Response,
};
use sce_rust_trust::membership::{Body, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use sce_rust_world::{Outcome, World};

const LIST: &str = "home";

fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn key_of(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

type Shared = Arc<Mutex<Relay<MemoryStore>>>;

/// A device's machine (named `copy`), its MLS member and its courier to the relay.
struct Party {
    copy: &'static str,
    node: u64,
    member: Member,
    courier: Courier<DirectLink<MemoryStore>>,
}

struct Group {
    world: World,
    relay: Shared,
    authority: Authority,
    parties: Vec<Party>,
}

impl Group {
    /// The list begins with alice (node 1, who runs the group and holds the administrator's key)
    /// alone in it.
    fn begin(seed: u64) -> Self {
        let world = todo_world(seed, 20);
        let authority = Authority::new([3; 32], "the-relay");
        let relay: Shared = Arc::new(Mutex::new(
            Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
        ));
        let mut group = Self {
            world,
            relay,
            authority,
            parties: Vec::new(),
        };
        let genesis = Log::genesis_draft(
            LIST,
            Body::Genesis {
                administrators: vec![administrator().public()],
                quorum: 1,
                profile: Profile::Untrusted,
            },
        )
        .sign(&administrator())
        .finish();
        group.attach("alice", 1, Some(&genesis));
        let alice = &mut group.parties[0];
        let grant = alice
            .member
            .trust()
            .draft(Body::Grant {
                node: 1,
                role: 3,
                key: key_of(1).public(),
                final_seq: None,
            })
            .sign(&administrator())
            .finish();
        alice.member.trust().append(&grant).unwrap();
        alice.member.device_mut().create_group().unwrap();
        group.pump();
        group
    }

    /// A device of `node`, run by the machine `copy`, that has been told the genesis (which is all an
    /// invitation carries).
    fn attach(&mut self, copy: &'static str, node: u64, genesis: Option<&sce_rust_trust::Entry>) {
        let trust = Trust::open(LIST);
        if let Some(genesis) = genesis {
            trust.append(genesis).unwrap();
        } else {
            let first = self.parties[0].member.trust().entries()[0].clone();
            trust.append(&first).unwrap();
        }
        trust.set_identity(node, key_of(node));
        let member = Member::new(Device::new(LIST, node, key_of(node)), trust.clone(), node);
        let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
        let token = self.authority.issue(LIST, node, u64::MAX);
        let courier = Courier::new(
            trust.clone(),
            DirectLink::new(self.relay.clone(), token, clock),
        );
        self.world.set_sealer(copy, Box::new(Keep::new(trust)));
        self.parties.push(Party {
            copy,
            node,
            member,
            courier,
        });
    }

    fn party(&mut self, node: u64) -> &mut Party {
        self.parties.iter_mut().find(|p| p.node == node).unwrap()
    }

    /// Rounds of every courier, what each brings taken in by its member and raised into its machine,
    /// until a round of each brings nothing new.
    fn pump(&mut self) {
        for round in 0..30 {
            let mut moved = false;
            for party in &mut self.parties {
                let result = party.courier.sync().unwrap();
                moved |= result.pushed > 0 || !result.events.is_empty();
                for (event, data) in result.events {
                    self.world.raise(party.copy, &event, &data);
                }
                party.member.absorb(&result.messages);
                moved |= !party.member.advance().is_empty() || !result.messages.is_empty();
            }
            quiet(&mut self.world);
            // What a machine sends to a hub it does not have is kept by its host and refused to the
            // mesh; those are the host's errors, and nothing else is.
            self.world.host_errors();
            if !moved {
                return;
            }
            assert!(round < 29, "the couriers did not settle");
        }
    }

    fn agreed(&mut self, copies: &[&str]) -> usize {
        let first = todos_of(&mut self.world, copies[0]);
        for copy in &copies[1..] {
            assert_eq!(first, todos_of(&mut self.world, copy), "{copy}");
        }
        first.len()
    }

    fn feed_text(&self) -> String {
        let token = self.authority.issue(LIST, 1, u64::MAX);
        let text = self.relay.lock().unwrap().handle(
            &token,
            &Request::Feed {
                cursor: 0,
                limit: 500,
            }
            .to_json(),
            0,
        );
        assert!(matches!(
            Response::from_json(&text).unwrap(),
            Response::Feed { .. }
        ));
        text
    }
}

/// `node` leaves a key package with the relay, and alice adds it: the commit and the Welcome go to the
/// relay, and the new member joins when its courier reads them.
fn add(group: &mut Group, copy: &'static str, node: u64) {
    group.attach(copy, node, None);
    let package = group.party(node).member.device_mut().key_package().unwrap();
    group
        .party(node)
        .courier
        .publish_key_package(&package)
        .unwrap();
    group.pump();

    let package = group
        .party(1)
        .member
        .key_packages_of(node)
        .last()
        .cloned()
        .expect("alice read the key package from the relay");
    let issued = group
        .party(1)
        .member
        .add_member(&administrator(), node, 2, key_of(node).public(), &package)
        .unwrap();
    let alice = group.party(1);
    alice.courier.sync().unwrap();
    alice
        .courier
        .publish_commit(issued.epoch, &issued.commit)
        .unwrap();
    alice
        .courier
        .publish_welcome(issued.epoch, node, issued.welcome.as_ref().unwrap())
        .unwrap();
    group.pump();
}

#[test]
fn members_agree_on_one_list_sealed_under_the_key_of_each_epoch_the_group_reaches() {
    let mut group = Group::begin(31);
    add(&mut group, "bob", 2);
    assert_eq!(group.party(2).member.device().epoch(), Ok(1));
    assert_eq!(
        group.party(1).member.device().epoch_key(),
        group.party(2).member.device().epoch_key()
    );

    todo_set(&mut group.world, "alice", 1, DUE, 10);
    todo_title(&mut group.world, "bob", 2, "buy milk");
    quiet(&mut group.world);
    group.pump();
    assert_eq!(group.agreed(&["alice", "bob"]), 2);

    // The relay holds what was written sealed, and none of the text.
    let held = group.feed_text();
    assert!(!held.contains("buy milk"), "the relay holds the text");
    assert!(held.contains('~'));

    // A third member is added in epoch 2: bob follows the commit, the new member joins, and what is
    // written after is sealed under the key of epoch 2, which all three derive.
    add(&mut group, "server", 3);
    assert_eq!(group.party(2).member.device().epoch(), Ok(2));
    assert_eq!(group.party(3).member.device().epoch(), Ok(2));
    let key = group.party(1).member.device().epoch_key().unwrap();
    for node in [2, 3] {
        assert_eq!(group.party(node).member.device().epoch_key().unwrap(), key);
    }
    todo_set(&mut group.world, "bob", 3, DUE, 30);
    quiet(&mut group.world);
    group.pump();

    // A member that joined in epoch 2 holds no key of epoch 1 and is given no history: what was
    // written before is parked, not lost, and not opened.
    assert!(group.party(3).courier.parked() > 0);
    let todos = todos_of(&mut group.world, "alice");
    assert!(todos.contains_key(&(3, DUE)));
    let late = todos_of(&mut group.world, "server");
    assert!(
        late.contains_key(&(3, DUE)),
        "what was written in its epoch reaches it"
    );
    assert!(
        !late.contains_key(&(1, DUE)),
        "what was written before it joined does not"
    );
}

/// The administrator takes bob off. The group moves to an epoch he cannot derive, the relay lets his
/// device read and write no more, and what alice writes afterwards opens for carol and not for bob.
#[test]
fn a_member_taken_off_is_cut_off_at_the_next_epoch() {
    let mut group = Group::begin(32);
    add(&mut group, "bob", 2);
    add(&mut group, "server", 3);
    todo_set(&mut group.world, "alice", 1, DUE, 10);
    quiet(&mut group.world);
    group.pump();
    let old = group.party(2).member.device().epoch_key().unwrap();

    let issued = group
        .party(1)
        .member
        .remove_member(&administrator(), 2)
        .unwrap();
    {
        let alice = group.party(1);
        alice.courier.sync().unwrap();
        alice
            .courier
            .publish_commit(issued.epoch, &issued.commit)
            .unwrap();
    }
    // Bob still tries: his host reads the log, which says he is off, and the relay lets his device
    // read nothing more, so the commit never reaches him and the group never opens to him again.
    let steps = {
        let bob = group.party(2);
        let result = bob.courier.sync();
        match result {
            Ok(round) => {
                bob.member.absorb(&round.messages);
                bob.member.advance()
            }
            // The relay refuses a device that is off the log: it reads nothing more.
            Err(_) => Vec::new(),
        }
    };
    assert!(!steps.contains(&Step::Applied(issued.epoch as u64)));
    group.pump();

    let carol = group.party(3).member.device().epoch_key().unwrap();
    assert_eq!(group.party(1).member.device().epoch_key().unwrap(), carol);
    assert_ne!(carol, old, "the key changed");

    // What alice writes now is sealed under the new epoch: carol's machine holds it, bob's host
    // opens none of it even when handed it.
    todo_set(&mut group.world, "alice", 2, DUE, 77);
    quiet(&mut group.world);
    group.pump();
    assert!(todos_of(&mut group.world, "server").contains_key(&(2, DUE)));
    assert!(!todos_of(&mut group.world, "bob").contains_key(&(2, DUE)));

    let relay_ops: Vec<String> = {
        let token = group.authority.issue(LIST, 1, u64::MAX);
        let text = group.relay.lock().unwrap().handle(
            &token,
            &Request::Feed {
                cursor: 0,
                limit: 500,
            }
            .to_json(),
            0,
        );
        let Response::Feed { records, .. } = Response::from_json(&text).unwrap() else {
            panic!("feed");
        };
        records
            .into_iter()
            .filter_map(|record| match record {
                Record::Operation { data, .. } => Some(data),
                _ => None,
            })
            .collect()
    };
    let newest = relay_ops.last().unwrap();
    let mut bob_host = group.party(2).member.trust().clone();
    let reason = bob_host.admit("relay", "todo.write", newest).unwrap_err();
    assert!(reason.contains("no key of epoch 3"), "{reason}");
}
