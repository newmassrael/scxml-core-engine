// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule whose copies decide for themselves when they may forget. A copy that
// is written to for long enough has to drop what is settled (`todo.compact`), and that is
// safe only when every copy holds every change up to the instant and none will stamp one
// at or before it again. Telling every copy the instant puts that knowledge in the host.
// `todo.settle` puts it in the copies: the hub asks each person's copy what it holds up to
// the instant (`todo.propose`), each moves its clock past the instant and answers with a
// hash of its adds and tombstones up to it (`todo.ack`), and only when both answers are
// the hub's own does the hub tell every copy to forget (`todo.floor`). The answers do not
// depend on the order messages arrive in or on how often: a hash that is the same is the
// same set, and a clock that has passed the instant stamps nothing at or before it.
//
// What is asked here is that copies that hold the same settle the instant and forget; that
// a copy that lacks a change, or whose answer differs, settles nothing, and settles once it
// is repaired; that a write of a time that was settled and forgotten is not taken in again;
// that a copy that is not the one the others settled with (restarted from an old save) is
// found out and forgets nothing, instead of showing values that were written over; and
// that all of it holds over links that reorder and duplicate.

mod common;

use common::calendar::*;
use common::resync::{restore_from, resync, save};
use common::*;
use sce_rust_world::{Outcome, World};

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

const COPIES: [&str; 3] = ["alice", "bob", "server"];

fn variable(world: &mut World, who: &str, name: &str) -> serde_json::Value {
    todo_variables_of(world, who)[name].clone()
}

fn floors(world: &mut World) -> Vec<i64> {
    COPIES
        .iter()
        .map(|who| whole(&variable(world, who, "floor")) as i64)
        .collect()
}

fn stale(world: &mut World) -> Vec<bool> {
    COPIES
        .iter()
        .map(|who| variable(world, who, "stale").as_bool().expect("a bool"))
        .collect()
}

fn late(world: &mut World) -> Vec<i64> {
    COPIES
        .iter()
        .map(|who| whole(&variable(world, who, "late")) as i64)
        .collect()
}

fn refusals(world: &mut World) -> Vec<i64> {
    COPIES
        .iter()
        .map(|who| whole(&variable(world, who, "refusals")) as i64)
        .collect()
}

fn held(world: &mut World, who: &str, list: &str) -> usize {
    variable(world, who, list).as_array().expect("a list").len()
}

/// The host asks the hub to settle everything stamped at or before `before`; the copies
/// settle it among themselves.
fn settle(world: &mut World, before: i64) {
    world.raise(
        "server",
        "todo.settle",
        &format!(r#"{{"before":"{before}"}}"#),
    );
    quiet(world);
}

fn an_afternoon(world: &mut World, day: i64) {
    for (who, id) in [("alice", 1), ("bob", 2), ("alice", 3), ("bob", 1)] {
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

fn agendas(world: &mut World) -> Vec<Vec<Occurrence>> {
    (1..=3)
        .map(|id| {
            agreed_agenda(
                world,
                id,
                instant(2026, 3, 1, 0, 0),
                instant(2026, 5, 1, 0, 0),
            )
        })
        .collect()
}

/// Copies that hold the same settle the instant by themselves: every copy forgets, none is
/// told by the host, and what the schedule shows is what it showed.
#[test]
fn copies_that_hold_the_same_settle_an_instant_by_themselves() {
    let mut world = a_calendar(1);
    for day in 0..30 {
        an_afternoon(&mut world, day);
    }
    let list = agreed_todos(&mut world);
    let shown = agendas(&mut world);
    let adds = held(&mut world, "server", "adds");

    let before = world.now_ms() - 1;
    settle(&mut world, before);

    assert_eq!(floors(&mut world), [before; 3]);
    assert_eq!(stale(&mut world), [false; 3]);
    assert_eq!(late(&mut world), [0; 3]);
    assert_eq!(refusals(&mut world), [0; 3]);
    assert!(
        held(&mut world, "server", "adds") < adds,
        "the hub still holds the {adds} adds it held before it was settled"
    );
    assert_eq!(agreed_todos(&mut world), list);
    assert_eq!(agendas(&mut world), shown);
    let digests: Vec<_> = COPIES
        .iter()
        .map(|who| variable(&mut world, who, "digest"))
        .collect();
    assert_eq!(digests[0], digests[2]);
    assert_eq!(digests[1], digests[2]);
    assert!(world.host_errors().is_empty());
}

/// A copy that has lost changes for good answers with another hash: the round settles
/// nothing, no copy forgets, and once the walk has repaired the copy the same request
/// settles.
#[test]
fn a_copy_that_lacks_a_change_settles_nothing_until_it_is_repaired() {
    let mut world = a_calendar(2);
    for day in 0..6 {
        an_afternoon(&mut world, day);
    }
    let saved = save(&mut world, "bob");
    for day in 6..16 {
        an_afternoon(&mut world, day);
    }
    restore_from(&mut world, "bob", saved);
    let adds = held(&mut world, "server", "adds");

    let before = world.now_ms() - 1;
    settle(&mut world, before);
    assert_eq!(floors(&mut world), [0; 3], "a copy forgot what one lacked");
    assert_eq!(held(&mut world, "server", "adds"), adds);
    assert_eq!(stale(&mut world), [false; 3]);

    resync(&mut world, "bob");
    quiet(&mut world);
    settle(&mut world, before);
    assert_eq!(floors(&mut world), [before; 3]);
    assert_eq!(stale(&mut world), [false; 3]);
    assert!(held(&mut world, "server", "adds") < adds);
    agreed_todos(&mut world);
}

/// What was settled is settled: a write stamped at or before the instant that a copy no
/// longer holds, the add of a value written over, arrives again and is refused. Taken in
/// again, nothing would say the value was written over and it would come back.
#[test]
fn a_write_of_a_settled_time_is_not_taken_in_again() {
    let mut world = a_calendar(3);
    for day in 0..8 {
        an_afternoon(&mut world, day);
    }
    let tombstones = variable(&mut world, "server", "tombstones");
    let named = tombstones.as_array().expect("a list")[0].clone();
    let list = agreed_todos(&mut world);

    let before = world.now_ms() - 1;
    settle(&mut world, before);
    assert_eq!(floors(&mut world), [before; 3]);
    let (adds, values) = (
        variable(&mut world, "server", "adds"),
        variable(&mut world, "server", "values"),
    );

    // The write a tombstone named, sent to the hub once more.
    let element = whole(&named["element"]);
    world.raise(
        "server",
        "todo.write",
        &format!(
            r#"{{"id":{},"field":{},"value":"0","text":"","wallTime":"{}","counter":{},"nodeId":"{}"}}"#,
            element / 65_536,
            element % 65_536,
            whole(&named["wallTime"]),
            whole(&named["counter"]),
            whole(&named["nodeId"]),
        ),
    );
    quiet(&mut world);

    assert_eq!(late(&mut world)[2], 1, "the hub took the write in");
    assert_eq!(variable(&mut world, "server", "adds"), adds);
    assert_eq!(variable(&mut world, "server", "values"), values);
    assert_eq!(agreed_todos(&mut world), list);
}

/// A copy that is not the one the others settled with, restarted from an old save, is not
/// told to forget what it cannot know was written over. It holds values that the others
/// forgot the tombstones of, so it is found out, forgets nothing and says it is stale; the
/// instant is not settled for the copies that are left, and the host starts it again from
/// what its peers hold. That copy is then settled with like the others.
#[test]
fn a_copy_restarted_from_an_old_save_is_found_stale_and_started_again() {
    let mut world = a_calendar(4);
    for day in 0..4 {
        an_afternoon(&mut world, day);
    }
    let old = save(&mut world, "bob");
    for day in 4..24 {
        an_afternoon(&mut world, day);
    }
    let before = world.now_ms() - 1;
    settle(&mut world, before);
    assert_eq!(floors(&mut world), [before; 3]);

    // Bob comes back from the old save, with values the others have written over and
    // tombstones forgotten.
    restore_from(&mut world, "bob", old);
    let held_by_bob = variable(&mut world, "bob", "adds");
    world.advance_ms(60_000);
    let after = world.now_ms() - 1;
    settle(&mut world, after);
    assert_eq!(
        stale(&mut world),
        [false, true, false],
        "bob was not found out"
    );
    assert_eq!(whole(&variable(&mut world, "bob", "floor")), 0);
    assert_eq!(
        variable(&mut world, "bob", "adds"),
        held_by_bob,
        "a copy found stale forgot something"
    );
    assert_eq!(
        floors(&mut world)[0],
        before,
        "the others settled an instant a copy had not agreed to"
    );

    // Started again from nothing but who it is, and walked to what the hub holds.
    let empty = {
        let mut fresh = todo_world(4, 20);
        quiet(&mut fresh);
        save(&mut fresh, "bob")
    };
    restore_from(&mut world, "bob", empty);
    resync(&mut world, "bob");
    quiet(&mut world);
    world.advance_ms(60_000);
    let again = world.now_ms() - 1;
    settle(&mut world, again);
    assert_eq!(stale(&mut world), [false; 3]);
    assert_eq!(floors(&mut world), [again; 3]);
    agreed_todos(&mut world);
}

/// Over links that reorder and duplicate, copies that settle every few afternoons never
/// bring a value back, never disagree, and never find a copy that took part stale.
#[test]
fn settling_over_links_that_reorder_and_duplicate_never_brings_a_value_back() {
    let mut world = a_calendar(5);
    for link in ["alice", "bob"] {
        world.reorder(link, "server");
        world.degrade(link, "server", 0, 50);
    }
    for day in 0..60 {
        an_afternoon(&mut world, day);
        if day % 5 == 4 {
            let before = world.now_ms() - 1;
            settle(&mut world, before);
        }
    }
    quiet(&mut world);
    assert_eq!(stale(&mut world), [false; 3]);
    assert_eq!(refusals(&mut world), [0; 3]);
    // The same afternoons over a network that is plain and a calendar that never settles
    // say what every field should hold: what a copy that settled holds is the same values,
    // none of the ones that were written over come back, and none is lost.
    let values = |list: &std::collections::BTreeMap<(u64, u64), Field>| {
        list.iter()
            .map(|(key, field)| {
                let mut held: Vec<i64> = field.live.iter().map(|one| one.value).collect();
                held.sort_unstable();
                (*key, held)
            })
            .collect::<Vec<_>>()
    };
    let list = agreed_todos(&mut world);
    assert!(list.len() >= 3 * 2);
    let mut plain = a_calendar(5);
    for day in 0..60 {
        an_afternoon(&mut plain, day);
    }
    assert_eq!(values(&list), values(&agreed_todos(&mut plain)));
    let floors = floors(&mut world);
    assert!(floors[0] > 0 && floors[0] == floors[1] && floors[1] == floors[2]);
    assert!(world.host_errors().is_empty());
}
