// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A shared list whose members never talk to each other or to a hub machine: each member's host
// signs what its machine sends, a courier carries it to a relay (sce-rust-relay) that verifies and
// keeps it and holds no key, and the other member's courier brings it out and tells that member's
// machine. The machines are the ones of every other test and know nothing of it. What is asked:
// that two members agree on one list through the relay, in the profile where the hub is not
// trusted (content sealed, the relay reads none of it) and in the one where it is (the same relay,
// the same protocol); that an instant is settled on the members' checkpoints, carried through the
// relay, with no machine of the hub and no administrator; and that a member taken off is refused
// by the relay and reaches no copy.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use sce_rust_relay::{
    Authority, Courier, DirectLink, Keep, Limits, MemoryStore, Relay, Request, Response,
};
use sce_rust_trust::membership::{Body, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use sce_rust_world::{Outcome, World};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;

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

/// Alice's and bob's machines and hosts, the relay, and a courier for each member.
struct Members {
    world: World,
    relay: Shared,
    authority: Authority,
    alice: Trust,
    bob: Trust,
    couriers: Vec<(&'static str, Courier<DirectLink<MemoryStore>>)>,
}

impl Members {
    fn new(seed: u64, profile: Profile) -> Self {
        let mut world = todo_world(seed, 20);
        let authority = Authority::new([3; 32], "the-relay");
        let relay: Shared = Arc::new(Mutex::new(
            Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
        ));
        let genesis = Log::genesis_draft(
            LIST,
            Body::Genesis {
                administrators: vec![administrator().public()],
                quorum: 1,
                profile,
            },
        )
        .sign(&administrator())
        .finish();
        let mut hosts = Vec::new();
        for (node, name) in [(ALICE, "alice"), (BOB, "bob")] {
            let host = Trust::open(LIST);
            host.append(&genesis).unwrap();
            host.set_identity(node, key_of(node));
            hosts.push((name, host));
        }
        let (alice, bob) = (hosts[0].1.clone(), hosts[1].1.clone());
        // The administrator's host is alice's here: it makes the entries, and they reach the
        // relay and bob by the courier like anything else.
        for node in [ALICE, BOB] {
            let entry = alice
                .draft(Body::Grant {
                    node,
                    role: 2,
                    key: key_of(node).public(),
                    final_seq: None,
                })
                .sign(&administrator())
                .finish();
            alice.append(&entry).unwrap();
        }
        if profile == Profile::Untrusted {
            let epoch = alice
                .draft(Body::Epoch { epoch: 1 })
                .sign(&administrator())
                .finish();
            alice.append(&epoch).unwrap();
            for host in [&alice, &bob] {
                host.set_epoch_key(1, [5; 32]);
            }
        }
        for (name, host) in [("alice", &alice), ("bob", &bob)] {
            world.set_sealer(name, Box::new(Keep::new(host.clone())));
        }
        let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
        let couriers = [("alice", ALICE, &alice), ("bob", BOB, &bob)]
            .into_iter()
            .map(|(name, node, host)| {
                let token = authority.issue(LIST, node, u64::MAX);
                let link = DirectLink::new(relay.clone(), token, clock.clone());
                (name, Courier::new(host.clone(), link))
            })
            .collect();
        Self {
            world,
            relay,
            authority,
            alice,
            bob,
            couriers,
        }
    }

    /// Rounds of every courier, and what they bring raised into the machines, until a round of
    /// each brings nothing new.
    fn pump(&mut self) {
        for round in 0..20 {
            let mut moved = false;
            for (who, courier) in &mut self.couriers {
                let result = courier.sync().unwrap();
                moved |= result.pushed > 0 || !result.events.is_empty();
                for (event, data) in result.events {
                    self.world.raise(who, &event, &data);
                }
            }
            quiet(&mut self.world);
            // What a machine sends to a hub it does not have is kept by the host and refused to the
            // mesh: those are the host's errors, and nothing else is.
            self.world.host_errors();
            if !moved {
                return;
            }
            assert!(round < 19, "the couriers did not settle");
        }
    }

    fn agreed(&mut self) -> usize {
        let alice = todos_of(&mut self.world, "alice");
        assert_eq!(alice, todos_of(&mut self.world, "bob"));
        let digest = todo_variables_of(&mut self.world, "alice")["digest"].clone();
        assert_eq!(todo_variables_of(&mut self.world, "bob")["digest"], digest);
        alice.len()
    }

    fn courier(&mut self, who: &str) -> &mut Courier<DirectLink<MemoryStore>> {
        &mut self
            .couriers
            .iter_mut()
            .find(|(name, _)| *name == who)
            .unwrap()
            .1
    }

    fn feed_text(&self) -> String {
        let token = self.authority.issue(LIST, ALICE, u64::MAX);
        let mut relay = self.relay.lock().unwrap();
        let text = relay.handle(
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

/// The two members edit, concurrently and apart, and agree through the relay; under the profile
/// where the hub is not trusted the relay holds sealed text and none of it is readable there.
fn two_members_agree_through_a_relay(profile: Profile, seed: u64) {
    let mut members = Members::new(seed, profile);
    members.pump();
    todo_set(&mut members.world, "alice", 1, DUE, 10);
    todo_set(&mut members.world, "bob", 2, DUE, 20);
    todo_title(&mut members.world, "alice", 3, "buy milk");
    // Both write the same field before either hears of the other.
    todo_set(&mut members.world, "alice", 4, DUE, 111);
    todo_set(&mut members.world, "bob", 4, DUE, 222);
    quiet(&mut members.world);
    members.pump();
    assert_eq!(members.agreed(), 4);

    let held = members.feed_text();
    if profile == Profile::Untrusted {
        assert!(!held.contains("buy milk"), "the relay holds the text");
        assert!(held.contains('~'), "what it holds is sealed");
    } else {
        assert!(
            held.contains("buy milk"),
            "under a trusted hub the content is plain"
        );
    }
    assert!(
        members.alice.divergences().is_empty() && members.bob.divergences().is_empty(),
        "no member holds less than another has named"
    );
}

#[test]
fn two_members_agree_through_a_relay_that_cannot_read_what_they_write() {
    two_members_agree_through_a_relay(Profile::Untrusted, 21);
}

#[test]
fn the_same_relay_serves_a_list_whose_hub_is_trusted() {
    two_members_agree_through_a_relay(Profile::TrustedHub, 22);
}

/// The members settle an instant on their checkpoints, carried through the relay: each machine
/// freezes what it holds up to the instant and each host signs the digest; when the checkpoints of
/// both are in hand the host tells its machine, which compares the digest with its own and
/// forgets. No machine of a hub, and no administrator, takes part.
#[test]
fn an_instant_is_settled_on_the_checkpoints_the_relay_carried() {
    let mut members = Members::new(23, Profile::Untrusted);
    members.pump();
    todo_set(&mut members.world, "alice", 1, DUE, 10);
    todo_set(&mut members.world, "bob", 2, DUE, 20);
    quiet(&mut members.world);
    members.pump();
    assert_eq!(members.agreed(), 2);
    members.world.advance_ms(60_000);
    members.world.host_errors();

    // A copy that is asked what it holds up to an instant (todo.propose, which a hub would send)
    // freezes the tombstones up to it and the digest of them. Here the host asks its own machine.
    for who in ["alice", "bob"] {
        members
            .world
            .raise(who, "todo.propose", r#"{"before":"30000"}"#);
    }
    quiet(&mut members.world);
    members.world.host_errors();
    let alice_hash = whole(&todo_variables_of(&mut members.world, "alice")["frozenHash"]) as u32;
    let bob_hash = whole(&todo_variables_of(&mut members.world, "bob")["frozenHash"]) as u32;
    assert_eq!(
        alice_hash, bob_hash,
        "two copies of one list freeze one digest"
    );

    // One member's word is not a settlement.
    members
        .courier("alice")
        .publish_checkpoint(30_000, alice_hash)
        .unwrap();
    members.pump();
    assert_eq!(members.courier("bob").floor_event(30_000, alice_hash), None);
    for who in ["alice", "bob"] {
        assert_eq!(
            whole(&todo_variables_of(&mut members.world, who)["floor"]),
            0,
            "{who}"
        );
    }

    // Both members' words are.
    members
        .courier("bob")
        .publish_checkpoint(30_000, bob_hash)
        .unwrap();
    members.pump();
    for who in ["alice", "bob"] {
        let (event, data) = members
            .courier(who)
            .floor_event(30_000, alice_hash)
            .expect("both members signed");
        members.world.raise(who, &event, &data);
    }
    quiet(&mut members.world);
    for who in ["alice", "bob"] {
        let vars = todo_variables_of(&mut members.world, who);
        assert_eq!(
            whole(&vars["floor"]),
            30_000,
            "{who}: frozenFor {} frozenHash {} stale {} errors {:?}",
            vars["frozenFor"],
            vars["frozenHash"],
            vars["stale"],
            members.world.host_errors()
        );
    }
    assert_eq!(members.agreed(), 2);
}

/// A member the administrator takes off writes on his own copy and finds the relay refuses his
/// device: what he wrote reaches no one, and what he wrote before is still everyone's.
#[test]
fn a_member_taken_off_is_refused_by_the_relay_and_reaches_no_copy() {
    let mut members = Members::new(24, Profile::Untrusted);
    members.pump();
    todo_set(&mut members.world, "bob", 1, DUE, 10);
    quiet(&mut members.world);
    members.pump();
    assert_eq!(members.agreed(), 1);

    let last = members.alice.last_seq(BOB);
    let removal = members
        .alice
        .draft(Body::Grant {
            node: BOB,
            role: 0,
            key: key_of(BOB).public(),
            final_seq: Some(last),
        })
        .sign(&administrator())
        .finish();
    members.alice.append(&removal).unwrap();
    // The relay holds the removal; bob's host has not heard of it, and his copy lets him write.
    members.courier("alice").sync().unwrap();
    todo_set(&mut members.world, "bob", 2, DUE, 99);
    quiet(&mut members.world);
    members.world.host_errors();
    members.pump();
    let round = members.courier("bob").sync().unwrap();
    assert!(
        round.stalled,
        "the relay no longer lets bob's device read or write"
    );
    let alice_sees = todos_of(&mut members.world, "alice");
    assert!(
        alice_sees.contains_key(&(1, DUE)),
        "what he made before stays"
    );
    assert!(
        !alice_sees.contains_key(&(2, DUE)),
        "what he made after reaches no copy"
    );
}
