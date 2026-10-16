// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule that is written to for longer than a copy has room for. A copy keeps,
// for every field of every to-do, the values nobody has written over, and for the
// changes themselves an observed-remove set: every write is an add, and a write written
// over is a tombstone, and merging never drops either. Left alone, a copy that is written
// to for long enough passes the bound of its lists (1024 entries) and refuses the next
// write: the room a copy has is not the schedule it holds, it is every change it has ever
// taken in.
//
// What lets a copy forget is the difference the observed-remove set already is
// (`orset_live`): once every copy has taken in a tombstone, the tombstone and the add it
// names can go and the set's value is the same. The host says so with `todo.compact`, an
// instant up to which everything is settled everywhere, and each copy drops the tombstones
// stamped at or before it with the adds they name, and takes them out of its digest.
//
// What is asked here is that a calendar written to for a year of afternoons, far past the
// bound, never refuses a write when it is compacted now and then; that the copies still
// agree and still show what the schedule says; that the digest is still the digest of what
// each copy holds (a copy that finds a difference with the walk of `todo.resync` finds
// none where there is none); and, as the control, that the same writes without compacting
// do pass the bound, so that the compaction is what is being tested.

mod common;

use common::calendar::*;
use common::*;
use sce_rust_world::{Outcome, World};

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn refusals(world: &mut World) -> Vec<u64> {
    ["alice", "bob", "server"]
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

/// Every copy settles what is stamped at or before `before`.
fn compact_everywhere(world: &mut World, before: i64) {
    for who in ["alice", "bob", "server"] {
        world.raise(who, "todo.compact", &format!(r#"{{"before":"{before}"}}"#));
    }
    quiet(world);
}

/// A day of the calendar: both people move and retitle a few events, and then the
/// network settles. `day` is the number of the afternoon.
fn an_afternoon(world: &mut World, day: i64) {
    for (who, id) in [("alice", 1), ("bob", 2), ("alice", 3), ("bob", 1)] {
        let start = instant(2026, 3, 1 + (day % 28), 8 + id as i64, 0);
        todo_set(world, who, id, START, start);
        todo_set(world, who, id, LENGTH, HOUR_MS * (1 + day % 3));
    }
    quiet(world);
    world.advance_ms(60_000);
}

/// Without compacting, the same calendar passes the bound: the copies refuse a write,
/// which is the limit the compaction lifts.
#[test]
fn without_compacting_a_copy_runs_out_of_room() {
    let mut world = todo_world(1, 20);
    for id in 1..=3 {
        schedule_todo(
            &mut world,
            "alice",
            id,
            &Plan::once(instant(2026, 3, 1, 9, 0), HOUR_MS).repeating(DAILY, 1),
        );
    }
    quiet(&mut world);
    // Eight writes an afternoon, and every one is an add that stays: 200 afternoons are
    // more than the 1024 entries the lists hold.
    for day in 0..200 {
        an_afternoon(&mut world, day);
    }
    assert!(
        refusals(&mut world).iter().any(|n| *n > 0),
        "a copy that is never compacted refuses a write once every change it has taken in \
         is more than its lists hold"
    );
}

/// A year of afternoons, 120 of them, with the copies told every tenth that everything
/// up to a minute ago is settled: no copy refuses a write, the lists stay small, the
/// copies agree, and the agenda is the one the schedule says.
#[test]
fn a_calendar_compacted_now_and_then_never_refuses_a_write() {
    let mut world = todo_world(1, 20);
    for id in 1..=3 {
        schedule_todo(
            &mut world,
            "alice",
            id,
            &Plan::once(instant(2026, 3, 1, 9, 0), HOUR_MS).repeating(DAILY, 1),
        );
    }
    quiet(&mut world);
    let mut most_adds = 0;
    for day in 0..120 {
        an_afternoon(&mut world, day);
        if day % 10 == 9 {
            let before = world.now_ms() - 1;
            compact_everywhere(&mut world, before);
            most_adds = most_adds.max(held(&mut world, "server", "adds"));
        }
    }
    assert_eq!(
        refusals(&mut world),
        [0, 0, 0],
        "a write was refused for want of room"
    );
    assert!(
        most_adds < 200,
        "after being compacted a copy holds {most_adds} adds, which is not what is current"
    );

    // The copies agree on the list, and each agenda is the schedule's.
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
    assert!(world.host_errors().is_empty());
}

/// Compacting changes what a copy holds and not what it shows: the values, the agendas and
/// what the network says the list is stay as they were.
#[test]
fn compacting_changes_no_value_a_register_holds() {
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
    // Two people write one field at once: both values stay, and neither is settled away.
    todo_set(&mut world, "alice", 2, START, instant(2026, 4, 1, 10, 0));
    todo_set(&mut world, "bob", 2, START, instant(2026, 4, 2, 10, 0));
    quiet(&mut world);

    let before_list = agreed_todos(&mut world);
    let before_adds = held(&mut world, "server", "adds");
    let (from, to) = (instant(2026, 3, 1, 0, 0), instant(2026, 6, 1, 0, 0));
    let agendas: Vec<_> = (1..=3)
        .map(|id| agreed_agenda(&mut world, id, from, to))
        .collect();

    let now = world.now_ms();
    compact_everywhere(&mut world, now);

    assert_eq!(agreed_todos(&mut world), before_list);
    let agendas_after: Vec<_> = (1..=3)
        .map(|id| agreed_agenda(&mut world, id, from, to))
        .collect();
    assert_eq!(agendas_after, agendas);
    assert!(
        held(&mut world, "server", "adds") < before_adds,
        "the adds of values that were written over are gone"
    );
    assert_eq!(
        before_list[&(2, START)].live.len(),
        2,
        "the two values written at once are both still there"
    );
    assert_eq!(refusals(&mut world), [0, 0, 0]);
}

/// The digest is still the digest of what a copy holds after it forgets: copies that were
/// compacted by the same instant agree on it, so that a resync between them walks to
/// nothing, and a copy that was not (it had no link when the others settled) is found
/// different and is repaired by the walk, without the settled changes coming back to
/// life.
#[test]
fn copies_settled_alike_still_agree_on_their_digest() {
    let mut world = todo_world(3, 20);
    schedule_todo(
        &mut world,
        "alice",
        1,
        &Plan::once(instant(2026, 3, 1, 9, 0), HOUR_MS).repeating(DAILY, 1),
    );
    quiet(&mut world);
    for day in 0..20 {
        an_afternoon(&mut world, day);
    }
    let now = world.now_ms();
    compact_everywhere(&mut world, now);
    let digests: Vec<_> = ["alice", "bob", "server"]
        .iter()
        .map(|who| todo_variables_of(&mut world, who)["digest"].clone())
        .collect();
    assert_eq!(digests[0], digests[2], "alice and the hub hold one digest");
    assert_eq!(digests[1], digests[2], "bob and the hub hold one digest");

    // A resync between copies that agree sends nothing a write could be made of.
    let walked_before = sent_to_the_hub(&world)
        .iter()
        .filter(|(event, _)| event == "todo.node")
        .count();
    for who in ["alice", "server", "bob"] {
        world.raise(who, "todo.resync", "");
        quiet(&mut world);
    }
    let walked = sent_to_the_hub(&world)
        .iter()
        .filter(|(event, _)| event == "todo.node")
        .count()
        - walked_before;
    let list = agreed_todos(&mut world);
    assert!(!list.is_empty());
    assert_eq!(refusals(&mut world), [0, 0, 0]);
    assert!(
        walked < 40,
        "copies that agree walked {walked} messages: the digest of a compacted copy is not \
         the digest of what it holds"
    );
}
