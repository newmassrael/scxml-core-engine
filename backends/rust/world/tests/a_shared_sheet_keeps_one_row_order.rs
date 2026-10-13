// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The order of a shared sheet's rows: two people insert and remove rows, offline and
// at the same time, and every copy has to end with the same rows in the same order,
// which is the order the people meant.
//
// Cells are values and a copy keeps the later of two writes to one; rows are an
// ORDER, and two rows inserted after the same row have no later: both stay, and they
// have to stand the same way on every copy whichever reached it first. That is what
// the replicated growable array rule in `machines/sheet_peer.scxml` is for, and what
// this holds it to.
//
// Equality of the copies says they agree, not that they are right, so each run is
// also held to what the rule says and to the intent behind it:
//   * a second implementation of the rule, written here over a plain vector, applied
//     to the inserts the people sent, in the order they sent them;
//   * a row stands after the row it was inserted after, on every copy;
//   * a removed row is hidden and the rest keep their order.

mod common;

use common::*;
use sce_rust_world::{Outcome, Rng};

/// The rule, over a vector: a row goes directly after the row it names, except that
/// it steps over the rows already there with a larger id. A row already there is not
/// put in again.
fn place(order: &mut Vec<u32>, key: u32, after: u32) {
    if order.contains(&key) {
        return;
    }
    let mut at = if after == 0 {
        0
    } else {
        order
            .iter()
            .position(|&held| held == after)
            .unwrap_or_else(|| panic!("row {after} is not there to be inserted after"))
            + 1
    };
    while at < order.len() && order[at] > key {
        at += 1;
    }
    order.insert(at, key);
}

/// The rows the people's inserts make, by the rule above, and the ids they removed;
/// read from the network and not from any copy.
fn oracle_rows(world: &sce_rust_world::World) -> (Vec<u32>, Vec<u32>) {
    let mut order = Vec::new();
    let mut gone = Vec::new();
    for (event, data) in sent_to_the_hub(world) {
        let number = |field: &str| data[field].as_u64().expect("a number") as u32;
        match event.as_str() {
            "sheet.row" => place(&mut order, number("key"), number("after")),
            "sheet.gone" if !gone.contains(&number("key")) => gone.push(number("key")),
            _ => {}
        }
    }
    gone.sort_unstable();
    (order, gone)
}

/// Every row stands after the row it was inserted after.
fn assert_each_row_follows_its_predecessor(world: &sce_rust_world::World, order: &[u32]) {
    for (event, data) in sent_to_the_hub(world) {
        if event != "sheet.row" {
            continue;
        }
        let key = data["key"].as_u64().unwrap() as u32;
        let after = data["after"].as_u64().unwrap() as u32;
        let here = order.iter().position(|&held| held == key).expect("the row");
        if after != 0 {
            let there = order
                .iter()
                .position(|&held| held == after)
                .expect("its predecessor");
            assert!(
                there < here,
                "row {key} should stand after row {after}: {order:?}"
            );
        }
    }
}

#[test]
fn rows_added_one_after_another_stand_in_that_order() {
    let mut world = ordered_sheet(1, 20);
    let first = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let second = add_row(&mut world, "alice", first);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let third = add_row(&mut world, "bob", second);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (rows, gone) = agreed_rows(&mut world);
    assert_eq!(rows, [first, second, third]);
    assert!(gone.is_empty());
    assert_eq!((rows, gone), oracle_rows(&world));
    assert!(world.host_errors().is_empty());
}

#[test]
fn two_rows_added_after_the_same_row_at_once_stand_alike_on_every_copy() {
    let mut world = ordered_sheet(1, 20);
    let top = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // Neither has seen the other's: both go after `top`.
    let alices = add_row(&mut world, "alice", top);
    let bobs = add_row(&mut world, "bob", top);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (rows, _) = agreed_rows(&mut world);
    // Both stay, and the larger id stands nearer the row they were added after.
    let (larger, smaller) = if alices > bobs {
        (alices, bobs)
    } else {
        (bobs, alices)
    };
    assert_eq!(rows, [top, larger, smaller]);
    assert_each_row_follows_its_predecessor(&world, &rows);
}

#[test]
fn a_row_added_after_a_row_being_removed_survives_in_its_place() {
    let mut world = ordered_sheet(2, 20);
    let doomed = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // At the same moment: bob removes the row, alice adds one after it.
    drop_row(&mut world, "bob", doomed);
    let survivor = add_row(&mut world, "alice", doomed);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (rows, gone) = agreed_rows(&mut world);
    assert_eq!(
        rows,
        [doomed, survivor],
        "the removed row stays as the place"
    );
    assert_eq!(gone, [doomed]);
    for who in ["alice", "bob", "server"] {
        assert_eq!(
            shown_by(&mut world, who),
            [survivor],
            "{who} shows the survivor"
        );
    }
}

#[test]
fn removing_a_row_twice_removes_it_once() {
    let mut world = ordered_sheet(3, 20);
    let row = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    drop_row(&mut world, "alice", row);
    drop_row(&mut world, "bob", row);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (rows, gone) = agreed_rows(&mut world);
    assert_eq!(rows, [row]);
    assert_eq!(gone, [row]);
}

/// The Mesh core drops an envelope it has seen, so a row the people send once never
/// reaches a copy twice. The same row can still come twice in two envelopes, as it
/// does when a person's device sends again what it is not sure was taken, and the
/// copy has to hold it once. Here the second arrives as an event of its own.
#[test]
fn a_row_that_arrives_twice_in_two_envelopes_is_held_once() {
    let mut world = ordered_sheet(1, 20);
    let top = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let second = add_row(&mut world, "alice", top);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    for who in ["alice", "bob", "server"] {
        let again = format!(r#"{{"key":{second},"after":{top}}}"#);
        world.raise(who, "sheet.row", &again);
        world.raise(who, "sheet.row", &again);
    }
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (rows, gone) = agreed_rows(&mut world);
    assert_eq!(rows, [top, second]);
    assert!(gone.is_empty());
}

#[test]
fn rows_added_offline_take_their_place_when_the_link_is_back() {
    let mut world = ordered_sheet(4, 20);
    let top = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    world.partition("alice", "server");
    world.partition("bob", "server");
    // While both are cut off each adds rows after `top`, and alice one after her own.
    let a1 = add_row(&mut world, "alice", top);
    let a2 = add_row(&mut world, "alice", a1);
    let b1 = add_row(&mut world, "bob", top);
    world.advance_ms(1_000);
    world.heal("alice", "server");
    world.heal("bob", "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let (rows, _) = agreed_rows(&mut world);
    assert_eq!(rows.len(), 4);
    assert_each_row_follows_its_predecessor(&world, &rows);
    // Alice's pair stays together and in order wherever bob's row goes between.
    let at = |key: u32| rows.iter().position(|&held| held == key).unwrap();
    assert!(at(a1) < at(a2));
    assert!(rows.contains(&b1));
    assert_eq!((rows, Vec::<u32>::new()), oracle_rows(&world));
}

/// An afternoon of inserts and removals at random places, links cut and restored,
/// delays spread out and envelopes delivered twice: for every seed the three copies
/// end with one order and one set of removed rows, which is what the rule says of the
/// inserts the people sent, with every row after its predecessor.
#[test]
fn a_random_afternoon_of_row_edits_ends_with_one_order() {
    for seed in 0..40 {
        let mut rng = Rng::new(seed);
        let mut world = ordered_sheet(seed, 5);
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        // Each person starts their stream to the hub with a row that arrives. A Mesh
        // receiver takes the first sequence number it sees from a sender as the
        // start of that sender's stream, so an envelope cut before any other has
        // arrived and sent again later is below the start by then and is dropped for
        // good. A deployment that orders has to open its streams on a delivered
        // envelope; this is where the people do.
        add_row(&mut world, "alice", 0);
        add_row(&mut world, "bob", 0);
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "seed {seed}");
        let mut cut = [false, false];
        for _ in 0..50 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            let (held, _) = rows_of(&mut world, who);
            if held.is_empty() || rng.below(4) != 0 {
                // Add: after a row this person holds, or at the top.
                let after = if held.is_empty() || rng.below(5) == 0 {
                    0
                } else {
                    held[usize::try_from(rng.below(held.len() as u64)).unwrap()]
                };
                add_row(&mut world, who, after);
            } else {
                let row = held[usize::try_from(rng.below(held.len() as u64)).unwrap()];
                drop_row(&mut world, who, row);
            }
            if rng.below(6) == 0 {
                let which = usize::from(rng.below(2) == 1);
                let name = ["alice", "bob"][which];
                if cut[which] {
                    world.heal(name, "server");
                } else {
                    world.partition(name, "server");
                }
                cut[which] = !cut[which];
            }
            world.run_until_quiet(i64::try_from(rng.below(60)).unwrap() + 1);
        }
        for (which, name) in ["alice", "bob"].into_iter().enumerate() {
            if cut[which] {
                world.heal(name, "server");
            }
        }
        assert_eq!(
            world.run_until_quiet(600_000),
            Outcome::Quiet,
            "seed {seed}"
        );

        let diagnosis = diagnosis(&mut world);
        let (rows, gone) = {
            let held =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| agreed_rows(&mut world)));
            match held {
                Ok(held) => held,
                Err(_) => panic!("seed {seed}: the copies disagree. {diagnosis}"),
            }
        };
        assert_eq!((rows.clone(), gone), oracle_rows(&world), "seed {seed}");
        assert_each_row_follows_its_predecessor(&world, &rows);
        for who in ["alice", "bob", "server"] {
            assert_eq!(
                variables_of(&mut world, who)["orphans"],
                0,
                "seed {seed}: {who} met a row whose predecessor it did not hold"
            );
        }
        assert!(world.host_errors().is_empty(), "seed {seed}");
        assert!(!rows.is_empty(), "seed {seed}");
    }
}
