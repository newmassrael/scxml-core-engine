// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule that is written to for longer than a copy has room for. A copy keeps, for
// every field of every to-do, the values nobody has written over, and nothing else: a write is
// an add, and a write-over takes the add it names out, and keeps nothing of itself, since the
// layer below the list delivers a write-over after the write it names and delivers no write
// twice (docs/adr/0085, section 2.2). So the room a copy has is the schedule it holds, and not
// every write it has seen written over: a copy that kept a tombstone for each write-over passed
// the bound of its lists (1024 entries) within two hundred afternoons, and needed the host to
// tell it when it might forget.
//
// What does grow is the layer's log, which keeps each change until every copy holds it, and
// forgets it at the stable cut by itself. Its room is the deployment's: a change offered past
// it is refused by the layer (docs/adr/0085, section 3, "offer and verdict").
//
// What is asked here is that a calendar written to for two hundred afternoons, far past what a
// list of tombstones held, never refuses a write and holds only what is current; that the copies
// still agree and still show what the schedule says, and two values written at once both stay;
// and that a layer given less room than its copies leave unstable refuses what passes it.

mod common;

use common::calendar::*;
use common::*;
use sce_rust_mesh::causal::Start;
use sce_rust_world::{Outcome, World};

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn refusals(world: &mut World) -> Vec<u64> {
    COPIES
        .iter()
        .map(|who| whole(&todo_variables_of(world, who)["refusals"]) as u64)
        .collect()
}

fn held(world: &mut World, who: &str, list: &str) -> usize {
    todo_variables_of(world, who)[list]
        .as_array()
        .expect("a list")
        .len()
}

/// A day of the calendar: the people move and retitle a few events, and then the network
/// settles. `day` is the number of the afternoon.
fn an_afternoon(world: &mut World, day: i64) {
    for (who, id) in [("alice", 1), ("bob", 2), ("carol", 3), ("bob", 1)] {
        let start = instant(2026, 3, 1 + (day % 28), 8 + id as i64, 0);
        todo_set(world, who, id, START, start);
        todo_set(world, who, id, LENGTH, HOUR_MS * (1 + day % 3));
    }
    quiet(world);
    world.advance_ms(60_000);
}

fn a_calendar(seed: u64) -> World {
    let mut world = todo_world(seed, 20);
    for id in 1..=3 {
        schedule_todo(
            &mut world,
            "alice",
            id,
            &Plan::once(instant(2026, 3, 1, 9, 0), HOUR_MS).repeating(DAILY, 1),
        );
    }
    quiet(&mut world);
    world
}

/// Two hundred afternoons, eight writes each and every one writing over another, which a list
/// that kept a tombstone for each write-over could not hold: no copy refuses a write, each holds
/// one add for each value it keeps and no more, the copies agree, the agenda is the one the
/// schedule says, and every layer's log is empty once the copies rest. Red if a write-over
/// leaves its add in `adds`: the adds then grow with every write-over until a write is refused.
#[test]
fn a_calendar_written_to_for_long_holds_only_what_is_current() {
    let mut world = a_calendar(1);
    let mut most_adds = 0;
    for day in 0..200 {
        an_afternoon(&mut world, day);
        most_adds = most_adds.max(held(&mut world, "carol", "adds"));
    }
    assert_eq!(
        refusals(&mut world),
        [0, 0, 0],
        "a write was refused for want of room"
    );
    for who in COPIES {
        assert_eq!(
            held(&mut world, who, "adds"),
            held(&mut world, who, "values"),
            "{who} keeps an add for a value it no longer holds"
        );
    }
    assert!(
        most_adds < 100,
        "a copy held {most_adds} adds, which is not what is current"
    );

    let list = agreed_todos(&mut world);
    assert!(list.len() >= 3 * 2);
    for id in 1..=3 {
        agreed_agenda(
            &mut world,
            id,
            instant(2026, 3, 1, 0, 0),
            instant(2026, 5, 1, 0, 0),
        );
    }
    assert_layers_rest(&mut world, &COPIES, "after two hundred afternoons");
    assert!(world.host_errors().is_empty());
}

/// Forgetting is the layer's, and changes no value a register holds: two people write one
/// field at once, both values stay, and once every layer has forgotten every change the
/// values and the agendas are what they were.
#[test]
fn forgetting_changes_no_value_a_register_holds() {
    let mut world = todo_world(2, 20);
    schedule_todo(
        &mut world,
        "alice",
        1,
        &Plan::once(instant(2026, 3, 1, 9, 0), HOUR_MS).repeating(WEEKLY, 1),
    );
    quiet(&mut world);
    for day in 0..12 {
        an_afternoon(&mut world, day);
    }
    // Two people write one field at once: both values stay, and neither is forgotten.
    todo_set(&mut world, "alice", 2, START, instant(2026, 4, 1, 10, 0));
    todo_set(&mut world, "bob", 2, START, instant(2026, 4, 2, 10, 0));
    world.advance_ms(1);
    let kept: usize = COPIES
        .iter()
        .map(|who| world.causal_layer(who, PEOPLE).expect("a layer").log_len())
        .sum();
    assert!(
        kept > 0,
        "no layer kept the two changes while they were on their way"
    );
    quiet(&mut world);

    assert_layers_rest(&mut world, &COPIES, "after the two writes");
    let list = agreed_todos(&mut world);
    assert_eq!(
        list[&(2, START)].live.len(),
        2,
        "the two values written at once are both still there"
    );
    assert_eq!(list, todo_oracle(&world));
    let (from, to) = (instant(2026, 3, 1, 0, 0), instant(2026, 6, 1, 0, 0));
    for id in 1..=3 {
        agreed_agenda(&mut world, id, from, to);
    }
    assert_eq!(refusals(&mut world), [0, 0, 0]);
}

/// The room for the changes not yet stable is the layer's: a layer given room for two, whose
/// copy is cut off so that nothing it makes becomes stable, refuses the third change it is
/// offered, takes no number for it, and its host is told.
#[test]
fn a_layer_refuses_a_change_past_its_room() {
    let mut world = todo_world(3, 20);
    world.start_causal(
        "alice",
        PEOPLE,
        Start::NewIncarnation(replica_of("alice", 0)),
        2,
        VECTOR_EVERY_MS,
    );
    world.partition("alice", "bob");
    world.partition("alice", "carol");
    for id in 1..=3 {
        todo_set(&mut world, "alice", id, DUE, id as i64);
        world.advance_ms(10);
    }
    let errors = world.host_errors();
    assert_eq!(
        errors
            .iter()
            .filter(|error| error.contains("NoRoomForHistory"))
            .count(),
        1,
        "{errors:?}"
    );
    let layer = world.causal_layer("alice", PEOPLE).expect("a layer");
    assert_eq!(layer.own_seq(), 2, "the refused change took a number");
    assert_eq!(layer.log_len(), 2);
}
