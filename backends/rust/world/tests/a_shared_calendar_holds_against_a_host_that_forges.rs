// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared list whose hub is run by someone its members do not trust. Every copy's host holds
// the administrator's certificates (sce-rust-trust): a write leaves its author's host with the
// author's signature, and arrives at the next host only when its author is a member who may
// write and the signature is true. The machines are the ones of every other test and know
// nothing of it; the hosts put the rule in front of them and behind them.
//
// What is asked here is that members who are told nothing of signatures go on agreeing on one
// list through a hub that signs nothing of its own; that a write the hub made up in a member's
// name reaches no copy, though the hub holds it; that a write by a member the certificates
// make a reader reaches no copy, though that member's own copy holds it; that a member taken
// off by a later certificate is refused from then on; and that the word that an instant is
// settled, which carries no signature and makes a copy forget, is not taken from the hub
// unless a host was told to.

mod common;

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use common::*;
use sce_rust_trust::{Certificate, Identity, Trust};
use sce_rust_world::{Outcome, World};

const CALENDAR: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn key_of(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn certificate(node: u64, role: u8, serial: u64) -> Certificate {
    certificate_from(node, role, serial, 0)
}

fn certificate_from(node: u64, role: u8, serial: u64, from: i64) -> Certificate {
    Certificate::issue(
        &administrator(),
        CALENDAR,
        node,
        role,
        key_of(node).public(),
        serial,
        from,
    )
}

/// The hosts of alice, bob and the hub, in that order. Each holds the certificates of alice
/// (a writer) and bob (with `bob_role`), and is installed in front of and behind its machine;
/// the machines of `told` are given the roster the certificates make. `now` is the clock the
/// hosts of the persons sign a note at: the time the machine is given as the edit's `now`.
struct Hosts {
    alice: Trust,
    bob: Trust,
    hub: Trust,
    now: Arc<AtomicI64>,
}

impl Hosts {
    /// A person changes a number, with the hosts' clock reading the world's time.
    fn edit(&self, world: &mut World, who: &str, id: u64, field: u64, value: i64) {
        self.now.store(world.now_ms(), Ordering::SeqCst);
        todo_set(world, who, id, field, value);
    }

    fn of(&self, who: &str) -> &Trust {
        match who {
            "alice" => &self.alice,
            "bob" => &self.bob,
            _ => &self.hub,
        }
    }

    /// An administrator's certificate reaches every host.
    fn certify(&self, certificate: &Certificate) {
        for host in [&self.alice, &self.bob, &self.hub] {
            host.accept(certificate).unwrap();
        }
    }

    /// The host tells the machine of `who` the roster the certificates make.
    /// The hub's machine is told who ever wrote, a person's the role they have now.
    fn tell(&self, world: &mut World, who: &str) {
        let events = if who == "server" {
            self.of(who).hub_role_events()
        } else {
            self.of(who).role_events()
        };
        for (event, data) in events {
            world.raise(who, &event, &data);
        }
    }
}

fn untrusting_world(seed: u64, bob_role: u8, told: &[&str]) -> (World, Hosts) {
    let mut world = todo_world(seed, 20);
    let hosts = Hosts {
        alice: Trust::new(CALENDAR, &[administrator().public()]),
        bob: Trust::new(CALENDAR, &[administrator().public()]),
        hub: Trust::new(CALENDAR, &[administrator().public()]),
        now: Arc::new(AtomicI64::new(0)),
    };
    hosts.alice.set_identity(ALICE, key_of(ALICE));
    hosts.bob.set_identity(BOB, key_of(BOB));
    for person in [&hosts.alice, &hosts.bob] {
        let now = hosts.now.clone();
        person.set_clock(move || now.load(Ordering::SeqCst));
    }
    hosts.certify(&certificate(ALICE, 2, 1));
    hosts.certify(&certificate(BOB, bob_role, 1));
    for who in ["alice", "bob", "server"] {
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

/// What the copies of alice and bob hold, and that they hold one list and one digest.
fn agree(world: &mut World) -> usize {
    let list = agreed_todos(world);
    let digest = todo_variables_of(world, "server")["digest"].clone();
    for who in ["alice", "bob"] {
        assert_eq!(todo_variables_of(world, who)["digest"], digest, "{who}");
    }
    list.len()
}

/// A write of a number to a field, in a member's name, as a hub that made it up would write it.
fn forged_write(node: u64, id: u64, value: i64) -> String {
    format!(
        r#"{{"id":{id},"field":{DUE},"value":"{value}","text":"","wallTime":"5000","counter":0,"nodeId":"{node}"}}"#
    )
}

/// Members who were told nothing of signatures agree on one list through a hub that signs
/// nothing of its own, and the hub's machine, which is the only copy told what arrives,
/// holds what it would hold with no host at all.
#[test]
fn members_agree_through_a_hub_that_signs_nothing() {
    let (mut world, _hosts) = untrusting_world(1, 2, &["alice", "bob", "server"]);
    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    todo_title(&mut world, "alice", 3, "write it down");
    quiet(&mut world);
    assert_eq!(agree(&mut world), 3);
    assert!(world.host_errors().is_empty(), "nothing honest was refused");
    assert!(todos_of(&mut world, "server").contains_key(&(2, DUE)));
}

/// A write the hub made up in a member's name is believed by the hub's own machine and by no
/// copy: it leaves the hub with no signature, and a host that checks refuses it.
#[test]
fn a_write_the_hub_made_up_in_a_members_name_reaches_no_copy() {
    let (mut world, _hosts) = untrusting_world(2, 2, &["alice", "bob", "server"]);
    todo_set(&mut world, "alice", 1, DUE, 10);
    quiet(&mut world);
    world.host_errors();

    world.raise("server", "todo.write", &forged_write(ALICE, 7, 99));
    quiet(&mut world);

    assert!(
        todos_of(&mut world, "server").contains_key(&(7, DUE)),
        "the hub's own machine believed it"
    );
    for who in ["alice", "bob"] {
        let held = todos_of(&mut world, who);
        assert!(
            !held.contains_key(&(7, DUE)),
            "{who} holds the made-up write"
        );
        assert!(held.contains_key(&(1, DUE)), "{who} lost a real one");
    }
    let errors = world.host_errors();
    assert!(
        errors
            .iter()
            .any(|error| error.contains("carries no signature")),
        "the hosts that checked refused it: {errors:?}"
    );
}

/// A member the certificates make a reader signs a write with the key their certificate
/// gave, and the hub's host refuses it: the member's own copy, which was not told the roster,
/// holds it, and no other does.
#[test]
fn a_write_by_a_reader_reaches_no_copy_but_the_readers_own() {
    let (mut world, _hosts) = untrusting_world(3, 1, &["alice", "server"]);
    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);

    assert!(todos_of(&mut world, "bob").contains_key(&(2, DUE)));
    assert!(!todos_of(&mut world, "server").contains_key(&(2, DUE)));
    assert!(!todos_of(&mut world, "alice").contains_key(&(2, DUE)));
    assert!(todos_of(&mut world, "server").contains_key(&(1, DUE)));
    assert!(refused(&mut world, "may not write") >= 1);
}

/// A later certificate takes a member off: what the member signs after it is refused by every
/// host that holds it, and what was admitted before stays.
#[test]
fn a_member_taken_off_by_a_later_certificate_is_refused_from_then_on() {
    let (mut world, hosts) = untrusting_world(4, 2, &["alice", "bob", "server"]);
    todo_set(&mut world, "bob", 1, DUE, 1);
    quiet(&mut world);
    assert_eq!(agree(&mut world), 1);

    hosts.certify(&certificate_from(BOB, 0, 2, world.now_ms()));
    todo_set(&mut world, "bob", 2, DUE, 2);
    quiet(&mut world);

    let hub = todos_of(&mut world, "server");
    assert!(hub.contains_key(&(1, DUE)), "what was admitted stays");
    assert!(!hub.contains_key(&(2, DUE)), "bob was taken off");
    assert!(!todos_of(&mut world, "alice").contains_key(&(2, DUE)));
    assert!(
        refused(&mut world, "may not write") >= 1,
        "bob's certificate now says role 0"
    );
}

/// The word that an instant is settled makes a copy forget, and carries no signature: a host
/// that trusts nobody does not take it from the hub, so that the hub cannot have a copy forget
/// a tombstone and take in again, with an old signed write, what was deleted. A host told to
/// take it from the hub it trusts does.
#[test]
fn the_hub_cannot_settle_an_instant_for_a_host_that_trusts_nobody() {
    let (mut world, hosts) = untrusting_world(5, 2, &["alice", "bob", "server"]);
    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);
    world.advance_ms(60_000);
    world.host_errors();

    let settle = |world: &mut World, before: i64| {
        world.raise(
            "server",
            "todo.settle",
            &format!(r#"{{"before":"{before}","members":2}}"#),
        );
        quiet(world);
    };
    settle(&mut world, 30_000);
    assert!(
        refused(&mut world, "todo.floor") >= 2,
        "both persons' hosts refused it"
    );
    for who in ["alice", "bob"] {
        assert_eq!(
            whole(&todo_variables_of(&mut world, who)["floor"]),
            0,
            "{who}"
        );
    }

    // The hub settled it for itself; a later instant is a round of its own.
    hosts.alice.allow_unsigned(&["todo.floor"]);
    hosts.bob.allow_unsigned(&["todo.floor"]);
    settle(&mut world, 40_000);
    for who in ["alice", "bob"] {
        assert_eq!(
            whole(&todo_variables_of(&mut world, who)["floor"]),
            40_000,
            "{who}"
        );
    }
    assert_eq!(agree(&mut world), 2);
}

/// An instant is settled for hosts that trust nobody when an administrator attests it: the
/// administrator's host signs the instant and the hash its own copy held of it, and the hub's
/// host sends the hub's floor on with that signature. A signature of another hash, or of
/// another instant, settles nothing. The run is deterministic, so a first run learns the
/// hash the second is attested with.
#[test]
fn an_instant_an_administrator_attested_is_settled_for_every_host() {
    let run = |attest: Option<(i64, u32)>| {
        let (mut world, hosts) = untrusting_world(6, 2, &["alice", "bob", "server"]);
        todo_set(&mut world, "alice", 1, DUE, 10);
        todo_set(&mut world, "bob", 2, DUE, 20);
        quiet(&mut world);
        world.advance_ms(60_000);
        world.host_errors();
        if let Some((before, hash)) = attest {
            hosts
                .hub
                .attest_floor(&administrator(), before, hash)
                .unwrap();
        }
        world.raise("server", "todo.settle", r#"{"before":"30000","members":2}"#);
        quiet(&mut world);
        world
    };

    let mut first = run(None);
    let hash = whole(&todo_variables_of(&mut first, "alice")["frozenHash"]) as u32;
    for who in ["alice", "bob"] {
        assert_eq!(whole(&todo_variables_of(&mut first, who)["floor"]), 0);
    }

    for (before, hash) in [(30_000, hash ^ 1), (20_000, hash)] {
        let mut world = run(Some((before, hash)));
        for who in ["alice", "bob"] {
            assert_eq!(
                whole(&todo_variables_of(&mut world, who)["floor"]),
                0,
                "{who} took a floor attested for {before} with {hash}"
            );
        }
    }

    let mut world = run(Some((30_000, hash)));
    for who in ["alice", "bob"] {
        assert_eq!(
            whole(&todo_variables_of(&mut world, who)["floor"]),
            30_000,
            "{who}"
        );
    }
    assert_eq!(agree(&mut world), 2);
}

/// The members hold a key the hub does not, and what they write reaches the hub sealed: its
/// machine holds the stamp of each write and a text it cannot read, relays it as it is, and the
/// members' copies hold what was written and agree on one list, over a resync as well.
#[test]
fn the_hub_holds_what_its_members_wrote_without_being_able_to_read_it() {
    let (mut world, hosts) = untrusting_world(8, 2, &["alice", "bob", "server"]);
    for person in [&hosts.alice, &hosts.bob] {
        person.set_epoch_key(1, [5; 32]);
    }
    todo_title(&mut world, "alice", 1, "buy milk");
    hosts.edit(&mut world, "bob", 2, DUE, 987_654_321);
    todo_title(&mut world, "bob", 3, "call dad");
    quiet(&mut world);

    let hub = todo_variables_of(&mut world, "server");
    let held = hub["values"].as_array().expect("values are a list");
    assert_eq!(held.len(), 3);
    for value in held {
        assert!(value["text"].as_str().unwrap().starts_with('~'), "{value}");
        assert_eq!(whole(&value["value"]), 0, "{value}");
    }
    let hub_text = hub.to_string();
    for secret in ["buy milk", "call dad", "987654321"] {
        assert!(!hub_text.contains(secret), "the hub holds {secret}");
    }

    for round in 0..2 {
        assert_eq!(
            todos_of(&mut world, "alice"),
            todos_of(&mut world, "bob"),
            "round {round}"
        );
        for who in ["alice", "bob"] {
            let copy = todo_variables_of(&mut world, who).to_string();
            for secret in ["buy milk", "call dad", "987654321"] {
                assert!(copy.contains(secret), "{who} lost {secret}");
            }
            assert_eq!(
                todo_variables_of(&mut world, who)["digest"],
                hub["digest"],
                "{who}"
            );
        }
        for who in ["alice", "bob", "server"] {
            world.raise(who, "todo.resync", "");
        }
        quiet(&mut world);
    }
    let errors = world.host_errors();
    assert!(errors.is_empty(), "{errors:?}");
}

/// A member is taken off and an administrator begins a new epoch for the members left: what alice
/// writes afterwards reaches the hub (sealed) and carol's... here, the one member that is left,
/// and bob's copy, which was given no key of it, holds none of it, while what was written before
/// is still what his copy holds.
#[test]
fn a_member_taken_off_reads_nothing_written_after_the_next_epoch() {
    let (mut world, hosts) = untrusting_world(9, 2, &["alice", "bob", "server"]);
    for person in [&hosts.alice, &hosts.bob] {
        person.set_epoch_key(1, [5; 32]);
    }
    todo_title(&mut world, "alice", 1, "before");
    quiet(&mut world);
    assert_eq!(todos_of(&mut world, "bob").len(), 1);
    world.host_errors();

    // The administrator's host takes bob off and begins epoch 2 for who is left.
    world.advance_ms(1_000);
    hosts.certify(&certificate_from(BOB, 0, 2, world.now_ms()));
    let administrator = Trust::new(CALENDAR, &[administrator().public()]);
    administrator.set_epoch_key(1, [5; 32]);
    for node in [ALICE, BOB] {
        administrator.accept(&certificate(node, 2, 1)).unwrap();
    }
    administrator
        .accept(&certificate_from(BOB, 0, 2, world.now_ms()))
        .unwrap();
    let left: Vec<_> = administrator
        .current_members()
        .into_iter()
        .map(|(_, key)| key)
        .collect();
    assert_eq!(left, vec![key_of(ALICE).public()]);
    let rotation = administrator
        .rotate_epoch(&self::administrator(), &left)
        .unwrap();
    assert_eq!(hosts.alice.receive_grant(&rotation.grants[0].1), Ok(2));

    world.advance_ms(1_000);
    todo_title(&mut world, "alice", 2, "after");
    quiet(&mut world);

    assert!(todos_of(&mut world, "alice").contains_key(&(2, TITLE)));
    let hub = todo_variables_of(&mut world, "server").to_string();
    assert!(!hub.contains("after"), "the hub holds the text");
    let bob = todos_of(&mut world, "bob");
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

/// What a member wrote before they were taken off stays: when everyone resyncs afterwards, the
/// hub sends their values to the copies and the tombstone of the one a person wrote over (as
/// itself, which the host puts who really wrote over it back on), and no host refuses any of it,
/// so that the copies still hold one list. What the member signs after they were taken off is
/// refused.
#[test]
fn what_a_member_wrote_before_they_were_taken_off_still_reaches_every_copy() {
    let (mut world, hosts) = untrusting_world(7, 2, &["alice", "server"]);
    hosts.edit(&mut world, "bob", 1, DUE, 1);
    quiet(&mut world);
    world.advance_ms(1_000);
    // Alice writes over bob's value: a note that carries her, and the time she made it at.
    hosts.edit(&mut world, "alice", 1, DUE, 2);
    hosts.edit(&mut world, "bob", 2, DUE, 5);
    quiet(&mut world);
    assert_eq!(agree(&mut world), 2);
    world.host_errors();

    world.advance_ms(1_000);
    hosts.certify(&certificate_from(BOB, 0, 2, world.now_ms()));
    for who in ["alice", "server"] {
        hosts.tell(&mut world, who);
    }
    world.advance_ms(1_000);
    for who in ["alice", "bob", "server"] {
        world.raise(who, "todo.resync", "");
    }
    quiet(&mut world);
    let errors = world.host_errors();
    assert!(errors.is_empty(), "nothing true was refused: {errors:?}");
    assert_eq!(agree(&mut world), 2);
    assert!(todos_of(&mut world, "server").contains_key(&(2, DUE)));

    hosts.edit(&mut world, "bob", 3, DUE, 9);
    quiet(&mut world);
    assert!(!todos_of(&mut world, "server").contains_key(&(3, DUE)));
    assert!(refused(&mut world, "may not write") >= 1);
}
