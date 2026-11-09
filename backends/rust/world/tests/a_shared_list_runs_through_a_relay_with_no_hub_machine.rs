// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A shared list whose members never talk to each other or to a hub machine: each member's host
// signs what its machine sends, a courier carries it to a relay (sce-rust-relay) that verifies and
// keeps it and holds no key, and the other member's courier brings it out and tells that member's
// machine. The machines are copies of the standard shared record list
// (sce:std/shared/record_list.scxml), the document the trust layer secures, each run by its host,
// and know nothing of it; the world's hub actor is given nothing. What is asked:
// that two members agree on one list through the relay, in the profile where the hub is not
// trusted (content sealed, the relay reads none of it) and in the one where it is (the same relay,
// the same protocol); that an instant is settled on the members' checkpoints, carried through the
// relay, with no machine of the hub and no administrator; that a member taken off is refused
// by the relay and reaches no copy; and that a copy built again in the middle of the run from its
// application's own log of commands writes nothing twice, and the copies still agree with no
// conflict held. Beside it stands the control that says why a host does not build a copy again
// that way: the same rebuild, when the other member's clock runs ahead and his changes reached
// her copy between her commands, has writes refused, publishes a note no live change made, and
// leaves her copy unlike his.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use sce_rust_relay::{
    Authority, Courier, DirectLink, Keep, Limits, MemoryStore, Relay, Request, Response,
};
use sce_rust_runtime::Engine;
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
        let mut world = record_world(seed, 20);
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
        let alice = records_of(&mut self.world, "alice");
        assert_eq!(alice, records_of(&mut self.world, "bob"));
        let live = |world: &mut World, who: &str| {
            let mut adds: Vec<String> = record_variables_of(world, who)["adds"]
                .as_array()
                .expect("a copy's live writes are a list")
                .iter()
                .map(|add| add.to_string())
                .collect();
            adds.sort();
            adds
        };
        let alices = live(&mut self.world, "alice");
        assert_eq!(
            live(&mut self.world, "bob"),
            alices,
            "one set of live writes"
        );
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
    record_set(&mut members.world, "alice", 1, DUE, 10);
    record_set(&mut members.world, "bob", 2, DUE, 20);
    record_title(&mut members.world, "alice", 3, "buy milk");
    // Both write the same field before either hears of the other.
    record_set(&mut members.world, "alice", 4, DUE, 111);
    record_set(&mut members.world, "bob", 4, DUE, 222);
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

/// The members settle an instant on their checkpoints, carried through the relay, with no machine
/// of a hub and no administrator: one member's word is not a settlement, both members' are. The
/// list keeps no tombstones and has nothing to forget (docs/adr/0085, section 2), so no machine
/// takes part: the settlement is the core's word of stability until its stable cut replaces it
/// (landing 5), and the copies agree before it and after it.
#[test]
fn an_instant_is_settled_on_the_checkpoints_the_relay_carried() {
    let mut members = Members::new(23, Profile::Untrusted);
    members.pump();
    record_set(&mut members.world, "alice", 1, DUE, 10);
    record_set(&mut members.world, "bob", 2, DUE, 20);
    quiet(&mut members.world);
    members.pump();
    assert_eq!(members.agreed(), 2);
    members.world.advance_ms(60_000);
    members.world.host_errors();
    let digest = 7;

    // One member's word is not a settlement.
    members
        .courier("alice")
        .publish_checkpoint(30_000, digest)
        .unwrap();
    members.pump();
    assert_eq!(members.courier("bob").floor_event(30_000, digest), None);

    // Both members' words are.
    members
        .courier("bob")
        .publish_checkpoint(30_000, digest)
        .unwrap();
    members.pump();
    for who in ["alice", "bob"] {
        assert!(
            members.courier(who).floor_event(30_000, digest).is_some(),
            "{who}: both members signed"
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
    record_set(&mut members.world, "bob", 1, DUE, 10);
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
    record_set(&mut members.world, "bob", 2, DUE, 99);
    quiet(&mut members.world);
    members.world.host_errors();
    members.pump();
    let round = members.courier("bob").sync().unwrap();
    assert!(
        round.stalled,
        "the relay no longer lets bob's device read or write"
    );
    let alice_sees = records_of(&mut members.world, "alice");
    assert!(
        alice_sees.contains_key(&(1, DUE)),
        "what he made before stays"
    );
    assert!(
        !alice_sees.contains_key(&(2, DUE)),
        "what he made after reaches no copy"
    );
}

/// Alice's application keeps its own log of commands as its original, and in the middle of the run
/// builds her copy of the list again from it: a new machine, told her node, given her commands
/// again with the instants they had. The machine sends every write and note again, as it made them
/// the first time; her host gives each back as the operation it holds and signs nothing, so the
/// relay is given nothing and bob's machine is told nothing twice. A resync tells the rebuilt copy
/// the list, and the two copies agree as they did, with no conflict held anywhere.
#[test]
fn a_copy_built_again_from_its_own_commands_writes_nothing_twice() {
    let mut members = Members::new(25, Profile::Untrusted);
    members.pump();
    // What alice's application keeps of her commands: the row, the field, the number, the text and
    // the instant. The third writes over the first, so its change is a write and a note.
    let commands: [(u64, u64, i64, &str, i64); 4] = [
        (1, DUE, 10, "", 20_000),
        (3, TITLE, 0, "buy milk", 20_001),
        (1, DUE, 11, "", 20_002),
        (4, DUE, 111, "", 20_003),
    ];
    for (id, field, value, text, now) in commands {
        record_edit_at(&mut members.world, "alice", id, field, value, text, now);
    }
    record_edit_at(&mut members.world, "bob", 2, DUE, 20, "", 10_000);
    record_edit_at(&mut members.world, "bob", 4, DUE, 222, "", 10_001);
    quiet(&mut members.world);
    members.pump();
    assert_eq!(members.agreed(), 4);
    let held = records_of(&mut members.world, "alice");
    let operations = members.alice.signed_operations();
    let last = members.alice.last_seq(ALICE);
    assert_eq!(
        last, 4,
        "four changes, the third a write-over and a write in one"
    );

    // Her copy is built again from nothing, and her commands are given to it again.
    members.world.restart::<RecordList>("alice", |_, clock| {
        let mut engine = Engine::new(RecordList::new());
        engine.set_clock(clock);
        engine.initialize();
        engine
    });
    members
        .world
        .raise("alice", "record_list.join", r#"{"node":"1"}"#);
    for (id, field, value, text, now) in commands {
        record_edit_at(&mut members.world, "alice", id, field, value, text, now);
    }
    quiet(&mut members.world);
    let errors = members.world.host_errors();
    assert_eq!(errors.len(), 4, "every change sent again: {errors:?}");
    assert!(
        errors.iter().all(|error| error.contains("held already")),
        "each is the operation her host holds, signed once: {errors:?}"
    );
    assert_eq!(members.alice.signed_operations(), operations);
    assert_eq!(members.alice.last_seq(ALICE), last);

    // A resync tells the rebuilt copy the list; it sends nothing on what it is told.
    let told = members.courier("alice").resync().unwrap();
    assert_eq!(told.pushed, 0, "the relay is given nothing new");
    assert!(told.refused.is_empty(), "{:?}", told.refused);
    for (event, data) in told.events {
        members.world.raise("alice", &event, &data);
    }
    quiet(&mut members.world);
    assert!(
        members.world.host_errors().is_empty(),
        "the rebuilt copy sent nothing on what it was told"
    );

    let round = members.courier("bob").sync().unwrap();
    assert!(
        round.events.is_empty(),
        "bob's machine is told nothing twice: {:?}",
        round.events
    );
    assert_eq!(round.pushed, 0);
    members.pump();
    assert_eq!(members.agreed(), 4);
    assert_eq!(records_of(&mut members.world, "alice"), held);
    assert_eq!(members.alice.signed_operations(), operations);
    for host in [&members.alice, &members.bob] {
        assert!(host.equivocating().is_empty(), "no chain is frozen");
        assert!(host.divergences().is_empty());
    }
}

/// The negative control of ADR 0093 (its Landing order, step 1), kept for good: ADR 0083
/// section 7.1's "must fail, checked once" made permanent. It holds the defect as it is today,
/// and it is the reason a host does not build a copy again by giving it its commands. The arm
/// that builds the copy from its journal and the history, and passes, is ADR 0093's step 2.
///
/// Bob's clock runs half a minute ahead of alice's, and each of his changes reaches her copy
/// before her next command. Every change of his moves her clock up to his time (`HlcReceive`,
/// once per change), and her close keeps the greater of that and her `now` (`HlcSend`), so her
/// three changes are stamped at his time, (50000, 2), (50001, 2) and (50002, 2), and not at the
/// 20000.. she gave. Her second change writes over her first value and writes anew; her third
/// writes over bob's first and writes anew. Her chain holds three changes, one operation each
/// (docs/adr/0085, section 1).
///
/// Her copy is then built again as a host does it today: the core kept (a restored core holds
/// what this one does), a fresh machine, her commands given again with their `now`, and no
/// operation of bob's between them. Measured, from the code:
///
/// - The fresh clock stamps the changes (20000, 0), (20001, 0) and (20002, 0). A change is known
///   by the bytes it is signed as, so none of the three is held; each has a time earlier than a
///   change already on her chain, which the core refuses (`Why::GoesBack`): 3 refused. Her copy
///   took each in before it sent it.
/// - The second change writes over the first value, which is now the re-stamped one, and the
///   third finds no value of bob's to write over: the set a change writes over differs too, but
///   a write-over is an operation of its change and is refused with it, so nothing is published.
///
/// So the refusals come from the re-stamped clock alone, and the third cause the consumer's note
/// gave (the parent's own judgement and the row numbers it assigns) is not in this world, whose
/// host names every row itself. After a resync on every device her copy holds, in each register
/// her second and third changes edited, the live run's value and the re-stamped one, two values of
/// one author; bob's holds one, and the rest of the two copies agree.
#[test]
fn control_a_copy_rebuilt_from_its_commands_among_a_peer_ahead_refuses_and_differs() {
    let mut members = Members::new(26, Profile::Untrusted);
    members.pump();
    // The row, the number and the instant of each command; every one edits the due date.
    let alice_commands: [(u64, i64, i64); 3] = [(1, 10, 20_000), (1, 11, 20_001), (2, 30, 20_002)];
    let bob_commands: [(u64, i64, i64); 3] = [(2, 20, 50_000), (3, 40, 50_001), (4, 50, 50_002)];
    for ((bob_id, bob_value, bob_now), (id, value, now)) in
        bob_commands.into_iter().zip(alice_commands)
    {
        record_edit_at(
            &mut members.world,
            "bob",
            bob_id,
            DUE,
            bob_value,
            "",
            bob_now,
        );
        quiet(&mut members.world);
        members.pump();
        record_edit_at(&mut members.world, "alice", id, DUE, value, "", now);
        quiet(&mut members.world);
        members.pump();
    }
    assert_eq!(members.agreed(), 4);
    let held = records_of(&mut members.world, "alice");
    for key in [(1, DUE), (2, DUE)] {
        let stamp = held[&key].stamp;
        assert!(
            stamp.2 == ALICE && stamp.0 >= 50_000,
            "{key:?}: alice's change is stamped at bob's time, past every `now` she gave: {stamp:?}"
        );
    }
    let last = members.alice.last_seq(ALICE);
    assert_eq!(last, 3, "three changes, one operation each");
    assert_eq!(members.bob.last_seq(BOB), 3);
    let operations = members.alice.signed_operations().len();

    // Her copy is built again from her commands, the core kept.
    members.world.restart::<RecordList>("alice", |_, clock| {
        let mut engine = Engine::new(RecordList::new());
        engine.set_clock(clock);
        engine.initialize();
        engine
    });
    members
        .world
        .raise("alice", "record_list.join", r#"{"node":"1"}"#);
    for (id, value, now) in alice_commands {
        record_edit_at(&mut members.world, "alice", id, DUE, value, "", now);
    }
    quiet(&mut members.world);
    let errors = members.world.host_errors();
    assert_eq!(errors.len(), 3, "three changes sent: {errors:?}");
    let refused = errors
        .iter()
        .filter(|error| error.contains("record_list.change: its stamp goes back along this device"))
        .count();
    assert_eq!(
        refused, 3,
        "every replayed change goes back along her chain, its write-overs with it: {errors:?}"
    );
    let published = errors
        .iter()
        .filter(|error| error.contains("kept for the relay"))
        .count();
    assert_eq!(
        published, 0,
        "no part of a change is signed apart from it: {errors:?}"
    );
    let made = members.alice.last_seq(ALICE) - last;
    assert_eq!(made, 0, "the rebuild made no new operation of hers");
    assert_eq!(members.alice.signed_operations().len(), operations);
    let rebuilt = records_of(&mut members.world, "alice");
    assert_eq!(rebuilt[&(1, DUE)].stamp, (20_001, 0, ALICE));
    assert_eq!(rebuilt[&(2, DUE)].stamp, (20_002, 0, ALICE));

    // A resync on every device.
    let mut pushed = 0;
    let mut dropped = 0;
    for who in ["alice", "bob"] {
        let told = members.courier(who).resync().unwrap();
        pushed += told.pushed;
        dropped += told.dropped.len();
        for (event, data) in told.events {
            members.world.raise(who, &event, &data);
        }
        quiet(&mut members.world);
    }
    assert_eq!(pushed as u64, made, "the relay is given the new operation");
    assert_eq!(
        refused + dropped,
        3,
        "sends refused or dropped: {refused} refused, {dropped} dropped"
    );
    members.pump();

    let alice_holds = records_of(&mut members.world, "alice");
    let bob_holds = records_of(&mut members.world, "bob");
    assert_eq!(bob_holds, held, "bob's copy is what the live run left");
    assert_ne!(
        alice_holds, bob_holds,
        "the rebuilt copy differs from bob's"
    );
    for key in [(1, DUE), (2, DUE)] {
        let mine = &alice_holds[&key].live;
        let his = &bob_holds[&key].live;
        assert_eq!(his.len(), 1, "{key:?}: bob holds one value");
        assert_eq!(
            mine.len(),
            2,
            "{key:?}: alice holds the live value and the re-stamped one: {mine:?}"
        );
        assert!(mine.contains(&his[0]), "{key:?}: {mine:?}");
        assert!(
            mine.iter().all(|value| value.stamp.2 == ALICE),
            "{key:?}: both are hers: {mine:?}"
        );
    }
    for key in [(3, DUE), (4, DUE)] {
        assert_eq!(alice_holds[&key], bob_holds[&key], "{key:?}");
    }
    let adds_of = |world: &mut World, who: &str| {
        record_variables_of(world, who)["adds"]
            .as_array()
            .expect("adds are a list")
            .len()
    };
    assert_eq!(
        adds_of(&mut members.world, "alice"),
        adds_of(&mut members.world, "bob") + 2,
        "alice's adds hold the two re-stamped dots bob's do not"
    );
    for host in [&members.alice, &members.bob] {
        assert!(host.equivocating().is_empty(), "no chain is frozen");
    }
}
