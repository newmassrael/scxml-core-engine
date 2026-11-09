// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared list whose members do not trust one another's hosts. Every copy's host holds the
// administrator's certificates (sce-rust-trust): a change leaves its author's host with the
// author's signature, and reaches the next copy only when its author is a member who may write
// and the signature is true. The copies are the standard shared record list
// (sce:std/shared/record_list.scxml), the document the trust layer secures, each run by its host
// as the scenarios run it, and know nothing of it; the hosts put the rule in front of them and
// behind them. Below the list is the Mesh core's causal route (`delivery: causal`, docs/adr/0085
// section 3.2), one group every copy sends to and reads: it tags, orders and repairs what the
// hosts signed, and signs nothing of its own.
//
// What is asked here is that members who are told nothing of signatures go on agreeing on one
// list through a layer that signs nothing; that a change a forging host made up in a member's
// name reaches no copy, though the forger's own machine holds what it made; that a change by a
// member the certificates make a reader reaches no copy but that member's own; that a member
// taken off by a later certificate is refused from then on, while what they wrote before still
// reaches a copy that is given it late; that a change a link did not pass on is fetched from
// another member and checked as from its author; and that what members write travels sealed,
// and a member taken off opens nothing written after the next epoch.
//
// There is no hub: no machine passes changes on, and no host is told a word that makes a copy
// forget, since the list keeps nothing it could forget (docs/adr/0085, section 4). The host's
// own refusal of an unsigned word that a copy should forget (`record_list.floor`) is held where
// the trust layer is, in
// backends/rust/trust/tests/a_host_refuses_a_chain_a_settlement_or_an_amendment_that_does_not_hold_together.rs.

mod common;

use common::repair::changes_carried_since;
use common::*;
use sce_rust_mesh::endpoint::Sealer;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use sce_rust_world::{Outcome, World};
use serde_json::{json, Value};

const CALENDAR: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn key_of(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

/// The hosts of alice, bob and carol. Each holds the membership log (alice and carol writers and
/// bob with `bob_role`, the entries signed by the administrator), and is installed in front of
/// and behind its machine; the machines of `told` are given the roster the log makes.
struct Hosts {
    alice: Trust,
    bob: Trust,
    carol: Trust,
}

impl Hosts {
    fn of(&self, who: &str) -> &Trust {
        match who {
            "alice" => &self.alice,
            "bob" => &self.bob,
            "carol" => &self.carol,
            other => panic!("{other} has no host here"),
        }
    }

    fn all(&self) -> [&Trust; 3] {
        [&self.alice, &self.bob, &self.carol]
    }

    /// An entry the administrator signs reaches every host.
    fn publish(&self, body: Body) -> Entry {
        let entry = self.alice.draft(body).sign(&administrator()).finish();
        for host in self.all() {
            host.append(&entry).unwrap();
        }
        entry
    }

    /// The administrator gives the device of `node` the role `role`.
    fn grant(&self, node: u64, role: u8) {
        self.publish(Body::Grant {
            node,
            role,
            key: key_of(node).public(),
            final_seq: None,
        });
    }

    /// The administrator takes the device of `node` off, ending its chain at the last operation
    /// alice's host has seen of it.
    fn remove(&self, node: u64) {
        self.publish(Body::Grant {
            node,
            role: 0,
            key: key_of(node).public(),
            final_seq: Some(self.alice.last_seq(node)),
        });
    }

    /// The list begins epoch `epoch`, and every member's host holds its key `fill`.
    fn begin_epoch(&self, epoch: u32, fill: u8) {
        self.publish(Body::Epoch { epoch });
        for host in self.all() {
            host.set_epoch_key(epoch, [fill; 32]);
        }
    }

    /// The host tells the machine of `who` the roster the certificates make. A copy on a causal
    /// route is given a change as late as a repair brings it, which its host judges by the
    /// grant of the time the change was made at; the machine has no time to judge by, so it is
    /// told that a member who could write at some time is a writer still
    /// (`Trust::hub_role_events`), and does not refuse what its host let through.
    fn tell(&self, world: &mut World, who: &str) {
        for (event, data) in self.of(who).hub_role_events() {
            world.raise(who, &event, &data);
        }
    }
}

fn untrusting_world(seed: u64, bob_role: u8, told: &[&str]) -> (World, Hosts) {
    let mut world = record_group_world(seed, 20);
    let hosts = Hosts {
        alice: Trust::open(CALENDAR),
        bob: Trust::open(CALENDAR),
        carol: Trust::open(CALENDAR),
    };
    let genesis = Log::genesis_draft(
        CALENDAR,
        Body::Genesis {
            administrators: vec![administrator().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator())
    .finish();
    for host in hosts.all() {
        host.append(&genesis).unwrap();
    }
    hosts.alice.set_identity(ALICE, key_of(ALICE));
    hosts.bob.set_identity(BOB, key_of(BOB));
    hosts.carol.set_identity(CAROL, key_of(CAROL));
    hosts.grant(ALICE, 2);
    hosts.grant(BOB, bob_role);
    hosts.grant(CAROL, 2);
    for who in COPIES {
        world.set_admission(who, Box::new(hosts.of(who).clone()));
        world.set_sealer(who, Box::new(hosts.of(who).clone()));
    }
    for who in told {
        hosts.tell(&mut world, who);
    }
    quiet(&mut world);
    (world, hosts)
}

fn refused(world: &mut World, what: &str) -> usize {
    world
        .host_errors()
        .iter()
        .filter(|error| error.contains(what))
        .count()
}

/// The text of every envelope the network carried, as one string, to be searched for what must
/// not travel in the clear.
fn the_wire(world: &World) -> String {
    world
        .trace()
        .iter()
        .map(|line| String::from_utf8_lossy(&line.bytes).into_owned())
        .collect()
}

/// Members who were told nothing of signatures agree on one list through a layer that signs
/// nothing of its own, and no host refused anything honest.
#[test]
fn members_agree_through_a_layer_that_signs_nothing() {
    let (mut world, _hosts) = untrusting_world(1, 2, &COPIES);
    record_set(&mut world, "alice", 1, DUE, 10);
    record_set(&mut world, "bob", 2, DUE, 20);
    record_title(&mut world, "carol", 3, "write it down");
    quiet(&mut world);
    assert_eq!(agreed_records(&mut world, &COPIES).len(), 3);
    assert!(world.host_errors().is_empty(), "nothing honest was refused");
}

/// A host that forges: what its machine sends leaves as a change in another member's name, with
/// no signature of that member's.
struct Forger {
    in_the_name_of: u64,
}

impl Sealer for Forger {
    fn seal(&mut self, _target: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        if event != "record_list.change" {
            return Ok(None);
        }
        let mut change: Value = serde_json::from_str(data).map_err(|error| error.to_string())?;
        change["by"] = json!(self.in_the_name_of.to_string());
        Ok(Some(change.to_string()))
    }
}

/// A change a forging host made up in a member's name reaches no copy: it leaves the forger with
/// no signature of the member it names, and every host that checks refuses it. The forger's own
/// machine holds what it made, and nothing else moves.
#[test]
fn a_change_a_host_made_up_in_a_members_name_reaches_no_copy() {
    let (mut world, _hosts) = untrusting_world(2, 2, &COPIES);
    record_set(&mut world, "alice", 1, DUE, 10);
    quiet(&mut world);
    assert!(world.host_errors().is_empty());

    world.set_sealer(
        "carol",
        Box::new(Forger {
            in_the_name_of: ALICE,
        }),
    );
    record_set(&mut world, "carol", 7, DUE, 99);
    quiet(&mut world);

    assert!(
        records_of(&mut world, "carol").contains_key(&(7, DUE)),
        "the forger's own machine holds what it made"
    );
    for who in ["alice", "bob"] {
        let held = records_of(&mut world, who);
        assert!(
            !held.contains_key(&(7, DUE)),
            "{who} holds the made-up change"
        );
        assert!(held.contains_key(&(1, DUE)), "{who} lost a real one");
    }
    assert!(
        refused(&mut world, "EventNotAdmitted") >= 2,
        "the hosts of alice and bob refused the made-up change"
    );
}

/// A member the certificates make a reader signs a change with the key their certificate gave,
/// and every other member's host refuses it: the reader's own copy, which was not told the
/// roster, holds it, and no other does.
#[test]
fn a_change_by_a_reader_reaches_no_copy_but_the_readers_own() {
    let (mut world, _hosts) = untrusting_world(3, 1, &["alice", "carol"]);
    record_set(&mut world, "alice", 1, DUE, 10);
    record_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);

    assert!(records_of(&mut world, "bob").contains_key(&(2, DUE)));
    for who in ["alice", "carol"] {
        let held = records_of(&mut world, who);
        assert!(
            !held.contains_key(&(2, DUE)),
            "{who} holds the reader's change"
        );
        assert!(held.contains_key(&(1, DUE)), "{who}");
    }
    assert!(refused(&mut world, "may not write") >= 1);
}

/// A later certificate takes a member off: what the member signs after it is refused by every
/// host that holds it, and what was admitted before stays.
#[test]
fn a_member_taken_off_by_a_later_certificate_is_refused_from_then_on() {
    let (mut world, hosts) = untrusting_world(4, 2, &COPIES);
    record_set(&mut world, "bob", 1, DUE, 1);
    quiet(&mut world);
    assert_eq!(agreed_records(&mut world, &COPIES).len(), 1);

    hosts.remove(BOB);
    record_set(&mut world, "bob", 2, DUE, 2);
    quiet(&mut world);

    for who in ["alice", "carol"] {
        let held = records_of(&mut world, who);
        assert!(
            held.contains_key(&(1, DUE)),
            "{who}: what was admitted stays"
        );
        assert!(!held.contains_key(&(2, DUE)), "{who}: bob was taken off");
    }
    assert!(
        refused(&mut world, "may not write") >= 1,
        "bob's certificate now says role 0"
    );
}

/// What a member wrote before they were taken off still reaches a copy that is given it late:
/// carol's links lose everything while bob writes and alice writes over him, bob is taken off,
/// and then the layer repairs carol. Her host judges each of bob's changes by the grant of the
/// time it was made at and admits it, her machine, told that whoever could write is a writer,
/// takes it, and the copies hold one list. What bob signs afterwards is refused. Red if a copy's
/// machine is told the roster of the present (`Trust::role_events`): carol's would refuse bob's
/// change though her host let it through, and hold another list.
#[test]
fn what_a_member_wrote_before_they_were_taken_off_still_reaches_a_copy_given_it_late() {
    let (mut world, hosts) = untrusting_world(7, 2, &COPIES);
    for other in ["alice", "bob"] {
        world.degrade("carol", other, 100, 0);
    }
    record_set(&mut world, "bob", 1, DUE, 1);
    world.advance_ms(1_000);
    // Alice writes over bob's value: one change of a write-over and a write.
    record_set(&mut world, "alice", 1, DUE, 2);
    record_set(&mut world, "bob", 2, DUE, 5);
    world.advance_ms(1_000);
    assert!(!records_of(&mut world, "carol").contains_key(&(2, DUE)));

    hosts.remove(BOB);
    for who in COPIES {
        hosts.tell(&mut world, who);
    }
    for other in ["alice", "bob"] {
        world.degrade("carol", other, 0, 0);
    }
    quiet(&mut world);
    let errors = world.host_errors();
    assert!(errors.is_empty(), "nothing true was refused: {errors:?}");
    let list = agreed_records(&mut world, &COPIES);
    assert_eq!(list.len(), 2);
    assert_eq!(list[&(1, DUE)].value, 2);
    assert_eq!(list[&(2, DUE)].value, 5);

    record_set(&mut world, "bob", 3, DUE, 9);
    quiet(&mut world);
    for who in ["alice", "carol"] {
        assert!(
            !records_of(&mut world, who).contains_key(&(3, DUE)),
            "{who}"
        );
    }
    assert!(refused(&mut world, "may not write") >= 1);
}

/// A change a link did not pass on is fetched from another member that holds it, as the
/// envelope its author's host signed, and the copy's host checks it as it would from its author:
/// alice's link to bob loses everything, carol answers bob's vector with alice's changes, and bob
/// holds them with nothing refused.
#[test]
fn a_change_a_link_did_not_pass_on_is_fetched_from_another_member_and_checked() {
    let (mut world, _hosts) = untrusting_world(7, 2, &COPIES);
    world.degrade("alice", "bob", 100, 0);
    let mark = world.trace().len();
    record_set(&mut world, "alice", 1, DUE, 10);
    record_set(&mut world, "alice", 2, DUE, 20);
    world.advance_ms(10_000);

    let bob = records_of(&mut world, "bob");
    assert!(bob.contains_key(&(1, DUE)) && bob.contains_key(&(2, DUE)));
    let from_carol = changes_carried_since(&world, mark)
        .into_iter()
        .filter(|carried| carried.from == "carol" && carried.to == "bob")
        .count();
    assert!(from_carol > 0, "bob was given alice's changes by carol");
    let errors = world.host_errors();
    assert!(errors.is_empty(), "{errors:?}");

    world.degrade("alice", "bob", 0, 0);
    quiet(&mut world);
    assert_eq!(agreed_records(&mut world, &COPIES).len(), 2);
}

/// The members hold a key the layer below them does not, and what they write travels sealed:
/// no envelope on the network holds a text or a number a member wrote, every member's copy holds
/// what was written, and the copies agree on one list, over a repair as well.
#[test]
fn what_members_write_travels_sealed_and_every_member_reads_it() {
    let (mut world, hosts) = untrusting_world(8, 2, &COPIES);
    hosts.begin_epoch(1, 5);
    record_title(&mut world, "alice", 1, "buy milk");
    record_set(&mut world, "bob", 2, DUE, 987_654_321);
    quiet(&mut world);
    world.partition("carol", "alice");
    world.partition("carol", "bob");
    record_title(&mut world, "bob", 3, "call dad");
    world.advance_ms(1_000);
    world.heal("carol", "alice");
    world.heal("carol", "bob");
    quiet(&mut world);

    let wire = the_wire(&world);
    for secret in ["buy milk", "call dad", "987654321"] {
        assert!(!wire.contains(secret), "{secret} travelled in the clear");
    }
    assert_eq!(agreed_records(&mut world, &COPIES).len(), 3);
    for who in COPIES {
        let copy = record_variables_of(&mut world, who).to_string();
        for secret in ["buy milk", "call dad", "987654321"] {
            assert!(copy.contains(secret), "{who} lost {secret}");
        }
    }
    let errors = world.host_errors();
    assert!(errors.is_empty(), "{errors:?}");
}

/// A member is taken off and an administrator begins a new epoch for the members left: what
/// alice writes afterwards reaches carol and travels sealed, and bob's copy, which was given no
/// key of it, holds none of it, while what was written before is still what his copy holds.
#[test]
fn a_member_taken_off_reads_nothing_written_after_the_next_epoch() {
    let (mut world, hosts) = untrusting_world(9, 2, &COPIES);
    hosts.begin_epoch(1, 5);
    record_title(&mut world, "alice", 1, "before");
    quiet(&mut world);
    assert_eq!(records_of(&mut world, "bob").len(), 1);
    assert!(world.host_errors().is_empty());

    // The administrator's host takes bob off and begins epoch 2 for who is left.
    world.advance_ms(1_000);
    hosts.remove(BOB);
    let administrator = Trust::open(CALENDAR);
    for entry in hosts.alice.entries() {
        administrator.append(&entry).unwrap();
    }
    administrator.set_epoch_key(1, [5; 32]);
    let left: Vec<_> = administrator
        .current_members()
        .into_iter()
        .map(|(_, key)| key)
        .collect();
    assert_eq!(left.len(), 2, "alice and carol are left");
    assert!(!left.contains(&key_of(BOB).public()));
    let rotation = administrator
        .rotate_epoch(&self::administrator(), &left)
        .unwrap();
    for host in hosts.all() {
        assert_eq!(host.append(&rotation.entry), Ok(true));
    }
    for (to, grant) in &rotation.grants {
        let host = if *to == key_of(ALICE).public() {
            &hosts.alice
        } else {
            &hosts.carol
        };
        assert_eq!(host.receive_grant(grant), Ok(2));
    }

    world.advance_ms(1_000);
    record_title(&mut world, "alice", 2, "after");
    quiet(&mut world);

    for who in ["alice", "carol"] {
        assert!(
            records_of(&mut world, who).contains_key(&(2, TITLE)),
            "{who}"
        );
    }
    assert!(
        !the_wire(&world).contains("after"),
        "the text travelled in the clear"
    );
    let bob = records_of(&mut world, "bob");
    assert!(
        !bob.contains_key(&(2, TITLE)),
        "bob holds what was written after"
    );
    assert!(bob.contains_key(&(1, TITLE)), "bob lost what he had");
    assert!(
        refused(&mut world, "no key of epoch 2") >= 1,
        "bob's host says why"
    );
}
