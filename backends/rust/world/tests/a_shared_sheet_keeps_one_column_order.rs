// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The order of a shared sheet's columns. Columns are kept in order and removed the way
// rows are (`a_shared_sheet_keeps_one_row_order`), by the same library algorithms
// (sce:std/merge `rga_insert`, `rga_remove`) over lists of their own, so what is held
// here is that the two do not touch each other: a column added is not a row, a column
// removed does not hide a row, and a cell is written at the ids a row and a column
// were given.
//
// Each run is also held to a second statement of the rule, written over a plain
// vector in `common`, applied to the inserts the people sent in the order they sent
// them.

mod common;

use common::*;
use sce_rust_world::{Outcome, Rng};

/// The columns the people's inserts make, and the ids they removed.
fn oracle_cols(world: &sce_rust_world::World) -> (Vec<u64>, Vec<u64>) {
    oracle_order(world, "sheet.col", "sheet.gone_col")
}

#[test]
fn columns_added_one_after_another_stand_in_that_order() {
    let mut world = ordered_sheet(1, 20);
    let first = add_col(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let second = add_col(&mut world, "alice", first);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let third = add_col(&mut world, "bob", second);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (cols, gone) = agreed_cols(&mut world);
    assert_eq!(cols, [first, second, third]);
    assert!(gone.is_empty());
    assert_eq!((cols, gone), oracle_cols(&world));
    assert!(world.host_errors().is_empty());
}

#[test]
fn two_columns_added_after_the_same_column_at_once_stand_alike_on_every_copy() {
    let mut world = ordered_sheet(1, 20);
    let left = add_col(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let alices = add_col(&mut world, "alice", left);
    let bobs = add_col(&mut world, "bob", left);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (cols, _) = agreed_cols(&mut world);
    let (larger, smaller) = if alices > bobs {
        (alices, bobs)
    } else {
        (bobs, alices)
    };
    assert_eq!(cols, [left, larger, smaller]);
    assert_eq!((cols, Vec::<u64>::new()), oracle_cols(&world));
}

#[test]
fn a_column_added_after_a_column_being_removed_survives_in_its_place() {
    let mut world = ordered_sheet(2, 20);
    let doomed = add_col(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    drop_col(&mut world, "bob", doomed);
    let survivor = add_col(&mut world, "alice", doomed);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (cols, gone) = agreed_cols(&mut world);
    assert_eq!(cols, [doomed, survivor]);
    assert_eq!(gone, [doomed]);
    for who in ["alice", "bob", "server"] {
        assert_eq!(columns_shown_by(&mut world, who), [survivor]);
    }
}

/// Rows and columns have lists of their own and take their ids from one counter, so a
/// column removed hides no row, a row removed hides no column, and no id is issued
/// twice across the two.
#[test]
fn rows_and_columns_are_kept_apart() {
    let mut world = ordered_sheet(3, 20);
    let row = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let col = add_col(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    assert_ne!(row, col, "one counter issues the ids of both");

    drop_col(&mut world, "bob", col);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let (rows, rows_gone) = agreed_rows(&mut world);
    let (cols, cols_gone) = agreed_cols(&mut world);
    assert_eq!((rows, rows_gone), (vec![row], vec![]));
    assert_eq!((cols, cols_gone), (vec![col], vec![col]));
}

/// A cell is written at the ids a row and a column were given, which are the
/// replicated array's ids and as wide as one.
#[test]
fn a_cell_is_written_at_the_ids_of_a_row_and_a_column() {
    let mut world = ordered_sheet(4, 20);
    let row = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    let col = add_col(&mut world, "bob", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    type_into(&mut world, "alice", row, col, 42);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    assert_eq!(sheet[&(row, col)].0, 42);
    assert_eq!(sheet, oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// An afternoon of inserts and removals of columns at random places, links cut and
/// restored, delays spread out and envelopes delivered twice: for every seed the three
/// copies end with one order and one set of removed columns, as the rule says of the
/// inserts the people sent.
#[test]
fn a_random_afternoon_of_column_edits_ends_with_one_order() {
    for seed in 0..40 {
        let mut rng = Rng::new(seed);
        let mut world = ordered_sheet(seed, 5);
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        // Each person opens their stream to the hub with a column that arrives, as
        // the rows' afternoon does.
        add_col(&mut world, "alice", 0);
        add_col(&mut world, "bob", 0);
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "seed {seed}");
        let mut cut = [false, false];
        for _ in 0..50 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            let (held, _) = cols_of(&mut world, who);
            if held.is_empty() || rng.below(4) != 0 {
                let after = if held.is_empty() || rng.below(5) == 0 {
                    0
                } else {
                    held[usize::try_from(rng.below(held.len() as u64)).unwrap()]
                };
                add_col(&mut world, who, after);
            } else {
                let col = held[usize::try_from(rng.below(held.len() as u64)).unwrap()];
                drop_col(&mut world, who, col);
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
        let held =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| agreed_cols(&mut world)));
        let (cols, gone) = match held {
            Ok(held) => held,
            Err(_) => panic!("seed {seed}: the copies disagree. {diagnosis}"),
        };
        assert_eq!((cols.clone(), gone), oracle_cols(&world), "seed {seed}");
        for who in ["alice", "bob", "server"] {
            assert_eq!(
                variables_of(&mut world, who)["orphans"],
                0,
                "seed {seed}: {who} met a column whose predecessor it did not hold"
            );
        }
        assert!(world.host_errors().is_empty(), "seed {seed}");
        assert!(!cols.is_empty(), "seed {seed}");
    }
}
