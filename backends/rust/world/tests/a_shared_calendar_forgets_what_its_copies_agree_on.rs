// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule whose copies forget what they all hold, and only that.
//
// A copy that is written to for long enough has to drop what is settled, and that is safe
// only when every copy holds the change. The list keeps nothing it could forget: it holds the
// live values and no tombstone, since the layer below it delivers each change once and after
// what it follows (docs/adr/0085, sections 2 and 4). What grows is the layer's log, which keeps
// every change it sent or delivered so that it can answer a repair, and the layer forgets a
// change at the stable cut: the pointwise minimum, over the members of the view, of what each
// is known to have delivered, a member not heard from counting as having delivered nothing.
// The copies' settle round, which agreed on a hash of what each held up to an instant, is that
// cut, read from the vectors the layer keeps.
//
// What is asked here is that copies that hold the same forget every change by themselves, with
// no word from any host, and that what the schedule shows does not move when they do; that a
// copy whose host's clock is set back still stamps after what it holds; that a copy that lacks
// a change holds the cut back, so that no copy forgets what it may be asked for, and that the
// cut moves once the copy is repaired; that a change the network gives a copy again after the
// cut has passed it is not taken in again; that a copy restored from an old save, a new
// incarnation, is not one the others forget for; and that all of it holds over links that
// reorder and duplicate.

mod common;

use common::calendar::*;
use common::repair::{delivered_of, lose_everything, MINUTE};
use common::*;
use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::Engine;
use sce_rust_world::{Fate, Outcome, World};

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn variable(world: &mut World, who: &str, name: &str) -> serde_json::Value {
    todo_variables_of(world, who)[name].clone()
}

fn refusals(world: &mut World) -> Vec<i64> {
    COPIES
        .iter()
        .map(|who| whole(&variable(world, who, "refusals")) as i64)
        .collect()
}

/// How many changes each copy's layer keeps not yet stable.
fn logs(world: &mut World) -> Vec<usize> {
    COPIES
        .iter()
        .map(|who| {
            world
                .causal_layer(who, PEOPLE)
                .expect("a causal layer")
                .log_len()
        })
        .collect()
}

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

/// Copies that hold the same forget every change by themselves: while they write, their
/// layers keep what is not yet stable, and once every copy has said what it holds, each
/// layer's log is empty and what the schedule shows is what it showed. Red if the cut is a
/// maximum rather than a minimum: a copy then forgets a change another lacks, and the repair
/// below it has nothing to answer with.
#[test]
fn copies_that_hold_the_same_forget_every_change_by_themselves() {
    let mut world = a_calendar(1);
    let mut kept_while_writing = 0;
    for day in 0..30 {
        for (who, id) in [("alice", 1), ("bob", 2), ("carol", 3)] {
            let start = instant(2026, 3, 1 + (day % 28), 8 + id as i64, 0);
            todo_set(&mut world, who, id, START, start);
        }
        // The changes are on their way: no copy can know yet that every other holds them.
        world.advance_ms(1);
        kept_while_writing = kept_while_writing.max(logs(&mut world).into_iter().max().unwrap());
        quiet(&mut world);
        world.advance_ms(60_000);
    }
    assert!(
        kept_while_writing > 0,
        "no layer kept a change while it was not yet stable"
    );
    assert_eq!(
        logs(&mut world),
        [0; 3],
        "a copy kept what every copy holds"
    );
    let list = agreed_todos(&mut world);
    let shown = agendas(&mut world);
    assert_eq!(list, todo_oracle(&world));
    assert_eq!(refusals(&mut world), [0; 3]);
    assert_eq!(agendas(&mut world), shown);
    assert!(world.host_errors().is_empty());
}

/// A copy stamps every change after what it holds, whatever its host's clock says: the clock
/// is `hlc_send`'s, which takes the later of the reading and the clock's own. A host that gives
/// a copy an instant long past, a device whose clock was set back, still has its change
/// stamped after every change the copy took in, and taken in everywhere.
#[test]
fn a_copy_given_a_past_instant_still_stamps_after_what_it_holds() {
    let mut world = a_calendar(6);
    for day in 0..8 {
        an_afternoon(&mut world, day);
    }
    let latest = agreed_todos(&mut world)
        .values()
        .map(|field| field.stamp.0)
        .max()
        .expect("the calendar holds changes");

    // Alice's host says it is the first millisecond of the world.
    let value = instant(2027, 1, 1, 9, 0);
    todo_edit_at(&mut world, "alice", 1, START, value, "", 1);
    quiet(&mut world);

    let list = agreed_todos(&mut world);
    let field = &list[&(1, START)];
    assert_eq!(field.value, value, "the change was not taken in");
    assert_eq!(field.live.len(), 1, "it wrote over what alice held");
    assert!(
        field.stamp.0 >= latest,
        "the change was stamped at {}, before the latest stamp held, {latest}",
        field.stamp.0
    );
    assert_eq!(refusals(&mut world), [0; 3]);
}

/// A copy that lacks a change holds the stable cut back: the others keep in their logs every
/// change it has not said it took in, so that a repair can still answer it, and forget them
/// only once it is repaired. Red if a member whose vector is not known is left out of the
/// cut, or if the log is pruned at a copy's own delivered vector rather than at the cut.
#[test]
fn a_copy_that_lacks_a_change_holds_the_cut_back_until_it_is_repaired() {
    let mut world = a_calendar(2);
    for day in 0..4 {
        an_afternoon(&mut world, day);
    }
    assert_eq!(logs(&mut world), [0; 3]);

    lose_everything(&mut world, "bob", true);
    for day in 4..8 {
        for (who, id) in [("alice", 1), ("carol", 3)] {
            let start = instant(2026, 3, 1 + day, 8 + id as i64, 0);
            todo_set(&mut world, who, id, START, start);
        }
        world.advance_ms(60_000);
    }
    let kept = logs(&mut world);
    assert!(
        kept[0] >= 8 && kept[2] >= 8,
        "alice and carol forgot changes bob lacks: {kept:?}"
    );
    assert!(todos_of(&mut world, "bob") != todos_of(&mut world, "alice"));

    lose_everything(&mut world, "bob", false);
    quiet(&mut world);
    assert_eq!(
        logs(&mut world),
        [0; 3],
        "the cut did not move once bob was repaired"
    );
    agreed_todos(&mut world);
    assert_eq!(refusals(&mut world), [0; 3]);
}

/// What the cut has passed is not taken in again: a change the network gives a copy once more,
/// long after every copy took it in and forgot it, is dropped by its number, and a value it
/// wrote and a later change wrote over does not come back. Red if a change at or below its
/// author's column is delivered again.
#[test]
fn a_change_given_again_after_the_cut_is_not_taken_in_again() {
    let mut world = a_calendar(3);
    let mark = world.trace().len();
    an_afternoon(&mut world, 0);
    // The first of these changes is written over by the afternoon after it.
    let first_change = world
        .trace()
        .into_iter()
        .skip(mark)
        .find(|line| {
            matches!(line.fate, Fate::Sent { .. })
                && line.from == "alice"
                && Envelope::decode(&mut SceCursor::new(&line.bytes))
                    .is_ok_and(|envelope| envelope.event_type == "record_list.change")
        })
        .expect("alice sent a change");
    for day in 1..4 {
        an_afternoon(&mut world, day);
    }
    assert_eq!(logs(&mut world), [0; 3]);
    let list = agreed_todos(&mut world);
    let held = (
        variable(&mut world, "bob", "adds"),
        variable(&mut world, "bob", "values"),
    );

    world.deliver_again(
        &first_change.from,
        "bob",
        PEOPLE,
        first_change.bytes.clone(),
    );
    quiet(&mut world);

    assert_eq!(
        (
            variable(&mut world, "bob", "adds"),
            variable(&mut world, "bob", "values")
        ),
        held,
        "bob took in again a change the cut had passed"
    );
    assert_eq!(agreed_todos(&mut world), list);
    assert_eq!(refusals(&mut world), [0; 3]);
}

/// A copy restored from an old save is a new incarnation (docs/adr/0085, section 3.3), not the
/// one the others forget for: what it makes is not delivered while the view of epoch 0 is the
/// only one, so a copy that holds values the others wrote over cannot bring them back, and the
/// others keep in their logs what its old incarnation never said it took in.
#[test]
fn a_copy_restored_from_an_old_save_is_not_one_the_others_forget_for() {
    let mut world = a_calendar(4);
    for day in 0..4 {
        an_afternoon(&mut world, day);
    }
    let old: SavedState = world
        .actor::<TodoPeer>("bob")
        .engine()
        .save_at(0)
        .expect("a running machine saves");
    lose_everything(&mut world, "bob", true);
    for day in 4..12 {
        an_afternoon_without(&mut world, day, "bob");
    }
    let kept_by_alice = logs(&mut world)[0];
    assert!(kept_by_alice > 0);
    let others_hold = agreed_todos_of(&mut world, &["alice", "carol"]);

    // Bob comes back from the old save, with values the others have written over, as a new
    // incarnation, and writes.
    world.restart::<TodoPeer>("bob", move |_, clock| {
        Engine::<TodoPeer>::restore_with(TodoPeer::new(), &old, clock, 0)
            .expect("what a machine saved restores")
    });
    start_layer(&mut world, "bob", 1);
    lose_everything(&mut world, "bob", false);
    todo_set(&mut world, "bob", 2, START, instant(2027, 1, 1, 9, 0));
    world.advance_ms(10 * MINUTE);

    assert_eq!(
        agreed_todos_of(&mut world, &["alice", "carol"]),
        others_hold,
        "a change of bob's new incarnation reached a copy"
    );
    for who in ["alice", "carol"] {
        assert_eq!(
            delivered_of(&mut world, who, replica_of("bob", 1)),
            0,
            "{who}"
        );
    }
    assert!(
        logs(&mut world)[0] >= kept_by_alice,
        "alice forgot what bob's old incarnation never said it took in"
    );
}

/// An afternoon in which `away` writes nothing.
fn an_afternoon_without(world: &mut World, day: i64, away: &str) {
    for (who, id) in [("alice", 1), ("bob", 2), ("carol", 3), ("bob", 1)] {
        if who == away {
            continue;
        }
        let start = instant(2026, 3, 1 + (day % 28), 8 + id as i64, 0);
        todo_set(world, who, id, START, start);
        todo_set(world, who, id, LENGTH, HOUR_MS * (1 + day % 3));
    }
    world.advance_ms(60_000);
}

/// Over links that reorder and duplicate, copies that forget what they all hold never bring a
/// value back, never disagree, and end with their logs empty. The same afternoons over a plain
/// network say what every field should hold.
#[test]
fn forgetting_over_links_that_reorder_and_duplicate_never_brings_a_value_back() {
    let mut world = a_calendar(5);
    for (a, b) in links_of(&COPIES) {
        world.reorder(a, b);
        world.degrade(a, b, 0, 50);
    }
    for day in 0..60 {
        an_afternoon(&mut world, day);
    }
    quiet(&mut world);
    assert_eq!(refusals(&mut world), [0; 3]);
    assert_eq!(logs(&mut world), [0; 3]);
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
    assert!(world.host_errors().is_empty());
}
