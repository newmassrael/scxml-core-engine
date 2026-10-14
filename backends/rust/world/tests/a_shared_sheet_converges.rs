// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared sheet of numbers: two people, `alice` and `bob`, each with a copy of the
// sheet (the fixture `sheet_peer.scxml`), and the hub both send to, which is the same
// document told it is the hub. The people edit cells, offline and at the same time,
// and the question is the one a shared spreadsheet answers: do the three copies end
// with the same sheet, and is it the sheet the edits meant.
//
// Equality of the copies says they agree, not that they are right, so each run is
// also held to an oracle that does not read the copies: of all the writes the
// people sent, the one with the latest stamp is the cell's value.

mod common;

use common::*;
use sce_rust_world::{Outcome, Rng};

#[test]
fn edits_to_different_cells_made_at_once_are_all_kept() {
    let mut world = sheet(1, 20);
    type_into(&mut world, "alice", 0, 0, 11);
    type_into(&mut world, "bob", 0, 1, 22);

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    assert_eq!(sheet.len(), 2);
    assert_eq!(sheet[&(0, 0)].0, 11);
    assert_eq!(sheet[&(0, 1)].0, 22);
    assert_eq!(sheet, oracle(&world));
    assert!(world.host_errors().is_empty());
}

#[test]
fn two_edits_to_one_cell_at_the_same_instant_leave_one_value_everywhere() {
    let mut world = sheet(1, 20);
    type_into(&mut world, "alice", 3, 3, 1);
    type_into(&mut world, "bob", 3, 3, 2);

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    // The stamps tie on time and counter, so the node id decides: bob's.
    assert_eq!(sheet[&(3, 3)].0, 2);
    assert_eq!(sheet, oracle(&world));
}

/// A person who has seen a number and types over it means to replace it, even when
/// their own clock is behind the one that wrote it.
#[test]
fn an_edit_made_after_seeing_a_write_wins_over_it_whatever_the_clock_says() {
    let mut world = sheet(1, 20);
    // The people's clocks differ: alice's host says 1000 ms, bob's says 10.
    world.raise(
        "alice",
        "sheet.edit",
        r#"{"row":0,"col":0,"value":1,"now":1000}"#,
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    assert_eq!(
        sheet_of(&mut world, "bob")[&(0, 0)].0,
        1,
        "bob saw the write"
    );

    world.raise(
        "bob",
        "sheet.edit",
        r#"{"row":0,"col":0,"value":2,"now":10}"#,
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // Bob's clock reads 10, but he had received a write stamped 1000, so his
    // stamp sorts after it, and his number is the one every copy holds.
    let sheet = agreed(&mut world);
    assert_eq!(sheet[&(0, 0)].0, 2);
    assert_eq!(sheet, oracle(&world));
}

#[test]
fn edits_made_offline_are_merged_when_the_link_is_back() {
    let mut world = sheet(5, 20);
    world.partition("alice", "server");
    type_into(&mut world, "alice", 1, 1, 100);
    type_into(&mut world, "alice", 1, 2, 101);
    // What alice's endpoint could not send is kept until the link is back, with no
    // timer running, so there is nothing left for the world to do: it is quiet.
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    // Bob types into one of the cells alice did, later by the world's clock.
    world.advance_ms(500);
    type_into(&mut world, "bob", 1, 2, 202);
    type_into(&mut world, "bob", 2, 2, 203);
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    // While alice is cut off, bob and the hub agree and alice holds only her own.
    assert_eq!(sheet_of(&mut world, "alice").len(), 2);
    assert_eq!(sheet_of(&mut world, "server").len(), 2);

    world.heal("alice", "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    assert_eq!(sheet.len(), 3);
    assert_eq!(sheet[&(1, 1)].0, 100, "alice's own cell reached the others");
    assert_eq!(sheet[&(1, 2)].0, 202, "bob typed over alice's number later");
    assert_eq!(sheet[&(2, 2)].0, 203);
    assert_eq!(sheet, oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// A clock that reads past what a double holds exactly: 2^53 + 1 and 2^53 are one
/// number to a double and two to a stamp. Alice's host says the first and bob's the
/// second, so alice's write is the later, and it is the one every copy keeps; a
/// stamp that lost its last bit on the way would tie them and let the node id pick
/// bob's. The stamp is read back from the copies exactly.
#[test]
fn a_clock_reading_past_two_to_the_53rd_is_kept_to_the_millisecond() {
    let beyond: i64 = (1 << 53) + 1;
    let mut world = sheet(1, 20);
    type_into_at(&mut world, "alice", 0, 0, 1, beyond);
    type_into_at(&mut world, "bob", 0, 0, 2, beyond - 1);

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    assert_eq!(sheet[&(0, 0)], (1, (beyond as u64, 0, 1)));
    assert_eq!(sheet, oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// The widest cell address and number a cell holds: row and column ids as large as
/// an id can be, a number at either end of an int64, and the largest wall time.
#[test]
fn the_widest_ids_and_numbers_a_cell_holds_are_kept_exactly() {
    let mut world = sheet(1, 20);
    type_into_at(
        &mut world,
        "alice",
        u64::MAX,
        u64::MAX - 1,
        i64::MIN,
        i64::MAX,
    );
    type_into_at(&mut world, "bob", 0, u64::MAX, i64::MAX, 7);

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    assert_eq!(
        sheet[&(u64::MAX, u64::MAX - 1)],
        (i64::MIN, (i64::MAX as u64, 0, 1))
    );
    assert_eq!(sheet[&(0, u64::MAX)], (i64::MAX, (7, 0, 2)));
    assert_eq!(sheet, oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// Many people-minutes in a few milliseconds: random edits to a small sheet at
/// random times, links cut and restored, delays spread out and envelopes delivered
/// twice, and for every seed the three copies end alike and as the oracle says.
#[test]
fn a_random_afternoon_of_edits_ends_with_one_sheet() {
    for seed in 0..40 {
        let mut rng = Rng::new(seed);
        let mut world = sheet(seed, 5);
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        // Links that do not keep order: a cell's value does not depend on the writes
        // before it, so a copy may take them in any order.
        world.reorder("alice", "server");
        world.reorder("bob", "server");
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        let mut cut = [false, false];
        for _ in 0..60 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            type_into(
                &mut world,
                who,
                rng.below(4),
                rng.below(4),
                i64::try_from(rng.below(1_000)).unwrap(),
            );
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

        let sheet = agreed(&mut world);
        assert_eq!(sheet, oracle(&world), "seed {seed}");
        assert!(world.host_errors().is_empty(), "seed {seed}");
        assert!(!sheet.is_empty(), "seed {seed}");
    }
}
