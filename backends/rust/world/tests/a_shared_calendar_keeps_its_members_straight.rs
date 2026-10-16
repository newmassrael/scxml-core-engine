// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule among more people than two, whose members come and go. The document
// is written for a hub and two named people (`#alice`, `#bob`); a team of six reaches the
// hub through those two names as topics (tests/common/crowd.rs), and a person other than
// the first two is reached by either. What is asked is what a team needs when one of its
// members is not in the room: that a person who is not one of the first two is repaired
// by the digest walk as the first two are, whoever lost the changes (the person or the
// hub); that a person who joins after the others settled holds what is current and takes
// part in the next settle; that a settle that names fewer members than there are does not
// wait for one that never answers, and that the member left out is found out when it
// comes back and is started again; and that the host that drops a member and takes it back
// ends with one list on every copy.

mod common;

use common::crowd::*;
use common::resync::{restore_from, resync, save};
use common::*;
use sce_rust_world::{Outcome, World};
use serde_json::json;

const PEOPLE: usize = 6;

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn team(seed: u64) -> (World, Vec<String>) {
    let names = crowd(PEOPLE);
    let mut world = crowd_world(seed, &names, 20, TodoPeer::new, "todo.join", |node, hub| {
        format!(r#"{{"node":"{node}","hub":{hub}}}"#)
    });
    quiet(&mut world);
    (world, names)
}

fn variable(world: &mut World, who: &str, name: &str) -> serde_json::Value {
    todo_variables_of(world, who)[name].clone()
}

/// The members a team has when it is written to: each of `who` changes a few to-dos.
fn an_afternoon(world: &mut World, who: &[&str], day: i64) {
    for (index, person) in who.iter().enumerate() {
        let id = 1 + (index as u64 + day as u64) % 4;
        todo_set(world, person, id, DUE, day * 100 + index as i64);
        todo_set(world, person, id, DONE, day % 2);
    }
    quiet(world);
    world.advance_ms(60_000);
}

fn settle(world: &mut World, before: i64, members: usize) {
    world.raise(
        HUB,
        "todo.settle",
        &format!(r#"{{"before":"{before}","members":{members}}}"#),
    );
    quiet(world);
}

fn floor_of(world: &mut World, who: &str) -> i128 {
    whole(&variable(world, who, "floor"))
}

fn digest_of(world: &mut World, who: &str) -> serde_json::Value {
    variable(world, who, "digest")
}

/// Every copy holds the digest the hub does, which is the digest of what it holds.
fn same_digest(world: &mut World, names: &[String]) {
    let hub = digest_of(world, HUB);
    for who in names {
        assert_eq!(digest_of(world, who), hub, "{who} holds another digest");
    }
}

/// A person who is not one of the first two, restored from an old save, is repaired by the
/// walk: the hub answers it by the name every person but the second reads, and it takes
/// the nodes that are meant for it. Before the nodes named whom they were for, the hub
/// answered a person by its node being 1 or 2 and nobody else.
#[test]
fn a_person_down_the_roster_is_repaired_by_resync() {
    let (mut world, names) = team(1);
    let all: Vec<&str> = names.iter().map(String::as_str).collect();
    for day in 0..4 {
        an_afternoon(&mut world, &all, day);
    }
    let saved = save(&mut world, "c005");
    for day in 4..10 {
        an_afternoon(&mut world, &all, day);
    }
    restore_from(&mut world, "c005", saved);
    resync(&mut world, "c005");
    quiet(&mut world);

    agreed_todos_among(&mut world, &names);
    same_digest(&mut world, &names);
    assert!(world.host_errors().is_empty());
}

/// The hub restored from an old save is repaired for every person by its own word alone: it
/// shows the root of its digest to all of them (to 0), and each answers by the walk. The
/// host does not have to tell each person.
#[test]
fn the_hub_restored_from_an_old_save_is_repaired_for_every_person() {
    let (mut world, names) = team(2);
    let all: Vec<&str> = names.iter().map(String::as_str).collect();
    for day in 0..4 {
        an_afternoon(&mut world, &all, day);
    }
    let saved = save(&mut world, HUB);
    for day in 4..10 {
        an_afternoon(&mut world, &all, day);
    }
    restore_from(&mut world, HUB, saved);
    resync(&mut world, HUB);
    quiet(&mut world);

    agreed_todos_among(&mut world, &names);
    same_digest(&mut world, &names);
    assert!(world.host_errors().is_empty());
}

/// A person who joins after the others settled holds what is current once it has walked
/// the digest with the hub, takes part in the next settle, and its first write is taken
/// in everywhere: what the others forgot is what was written over, and a copy that has
/// nothing was never holding a value that was.
#[test]
fn a_person_who_joins_after_the_others_settled_takes_what_is_current() {
    let (mut world, names) = team(3);
    let newcomer = names[PEOPLE - 1].clone();
    let others: Vec<&str> = names[..PEOPLE - 1].iter().map(String::as_str).collect();
    world.partition(&newcomer, HUB);
    for day in 0..8 {
        an_afternoon(&mut world, &others, day);
    }
    let first = world.now_ms() - 1;
    settle(&mut world, first, PEOPLE - 1);
    for who in &others {
        assert_eq!(floor_of(&mut world, who), i128::from(first), "{who}");
    }
    assert_eq!(floor_of(&mut world, HUB), i128::from(first));

    // The newcomer's link comes up and it walks the digest with the hub.
    world.heal(&newcomer, HUB);
    resync(&mut world, &newcomer);
    quiet(&mut world);
    assert_eq!(
        todos_of(&mut world, &newcomer),
        todos_of(&mut world, HUB),
        "the newcomer does not hold what is current"
    );
    assert_eq!(variable(&mut world, &newcomer, "stale"), json!(false));

    // It writes, and everyone takes it in; then the next settle names it among them.
    world.advance_ms(60_000);
    todo_set(&mut world, &newcomer, 1, DUE, 4242);
    quiet(&mut world);
    world.advance_ms(60_000);
    let second = world.now_ms() - 1;
    settle(&mut world, second, PEOPLE);
    for who in names.iter().map(String::as_str).chain([HUB]) {
        assert_eq!(floor_of(&mut world, who), i128::from(second), "{who}");
        assert_eq!(whole(&variable(&mut world, who, "late")), 0, "{who}");
        assert_eq!(variable(&mut world, who, "stale"), json!(false), "{who}");
    }
    agreed_todos_among(&mut world, &names);
    assert!(world.host_errors().is_empty());
}

/// A settle that names a member that never answers waits, and the host that drops the
/// member settles without it: the members named are the ones the settle waits for.
#[test]
fn a_settle_that_names_fewer_members_does_not_wait_for_one_that_never_answers() {
    let (mut world, names) = team(4);
    let gone = names[3].clone();
    let present: Vec<&str> = names
        .iter()
        .filter(|who| **who != gone)
        .map(String::as_str)
        .collect();
    world.partition(&gone, HUB);
    for day in 0..6 {
        an_afternoon(&mut world, &present, day);
    }

    // The settle waits for a member that is not there.
    let first = world.now_ms() - 1;
    settle(&mut world, first, PEOPLE);
    for who in &present {
        assert_eq!(floor_of(&mut world, who), 0, "{who} settled without {gone}");
    }

    // The host drops it: the same instant settles among those that are.
    settle(&mut world, first + 1, PEOPLE - 1);
    for who in present.iter().copied().chain([HUB]) {
        assert_eq!(floor_of(&mut world, who), i128::from(first + 1), "{who}");
        assert_eq!(variable(&mut world, who, "stale"), json!(false), "{who}");
    }
    assert_eq!(
        floor_of(&mut world, &gone),
        0,
        "{gone} was not there to settle"
    );
}

/// The member that was dropped, back with a change it made while it was out, finds that it
/// was left out: its change is stamped before what the others settled and is refused as
/// late, it is told the floor and says it is stale, and once the host starts it again from
/// what the others hold it is current and takes part in the settle after.
#[test]
fn a_member_left_out_of_a_settle_comes_back_stale_and_is_started_again() {
    let (mut world, names) = team(5);
    let gone = names[3].clone();
    let present: Vec<&str> = names
        .iter()
        .filter(|who| **who != gone)
        .map(String::as_str)
        .collect();
    for day in 0..3 {
        an_afternoon(
            &mut world,
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
            day,
        );
    }
    world.partition(&gone, HUB);
    // A change the member makes while it is out, and the others' afternoons.
    todo_set(&mut world, &gone, 1, DUE, 777);
    quiet(&mut world);
    world.advance_ms(60_000);
    for day in 3..8 {
        an_afternoon(&mut world, &present, day);
    }
    let before = world.now_ms() - 1;
    settle(&mut world, before, PEOPLE - 1);
    assert_eq!(floor_of(&mut world, HUB), i128::from(before));

    // The member is back: its link comes up, it walks the digest, and the hub tells it the
    // floor in answer to what it holds.
    world.heal(&gone, HUB);
    resync(&mut world, &gone);
    quiet(&mut world);
    settle(&mut world, before + 1, PEOPLE);
    assert_eq!(
        variable(&mut world, &gone, "stale"),
        json!(true),
        "a member that holds what the others forgot to write over is not told so"
    );

    // The host starts it again, empty, and it takes what the others hold.
    let empty = {
        let (mut fresh, _) = team(5);
        save(&mut fresh, &gone)
    };
    restore_from(&mut world, &gone, empty);
    resync(&mut world, &gone);
    quiet(&mut world);
    assert_eq!(variable(&mut world, &gone, "stale"), json!(false));
    assert_eq!(
        todos_of(&mut world, &gone),
        todos_of(&mut world, HUB),
        "the member started again does not hold what is current"
    );
    world.advance_ms(60_000);
    let next = world.now_ms() - 1;
    settle(&mut world, next, PEOPLE);
    for who in names.iter().map(String::as_str).chain([HUB]) {
        assert_eq!(floor_of(&mut world, who), i128::from(next), "{who}");
        assert_eq!(variable(&mut world, who, "stale"), json!(false), "{who}");
    }
    agreed_todos_among(&mut world, &names);
    assert!(world.host_errors().is_empty());
}
