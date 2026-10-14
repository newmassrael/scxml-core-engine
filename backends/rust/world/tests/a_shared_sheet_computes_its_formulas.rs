// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A cell of the shared sheet may hold a formula that reads other cells, as in a
// spreadsheet. What is replicated is still the cells, each a last-writer-wins register
// whose content is now a number or a formula, merged as a value is; what a copy shows is
// a function of the cells, rows and columns it holds (sce:std/merge `sheet_settle` and
// `sheet_show`), so two copies that hold the same show the same, whatever order the
// writes reached them in.
//
// Equality of the copies says they agree, not that they are right, so each run is also
// held to an oracle that does not read the copies: the writes the people sent the hub
// give the cells (the latest stamp wins its cell, formula and all) and the orders of
// rows and columns, and `Sheet::shown` works out what they show by following each cell's
// references, which is a second statement of what the library's passes compute.

mod common;

use common::*;
use sce_rust_world::{Outcome, Rng, World};

use Operand::{Cell, Number};

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
}

#[test]
fn a_formula_shows_what_the_cell_it_reads_holds() {
    let mut world = sheet(1, 20);
    type_into(&mut world, "alice", 1, 1, 5);
    quiet(&mut world);
    type_formula(&mut world, "bob", (2, 1), ADD, Cell(1, 1), Number(2));
    quiet(&mut world);

    let shown = agreed_shown(&mut world);
    assert_eq!(shown[&(1, 1)], (5, 0));
    assert_eq!(shown[&(2, 1)], (7, 0));
    assert_eq!(shown, expected_shown(&world));
    assert_eq!(
        contents_of(&mut world, "alice")[&(2, 1)],
        Content::Formula {
            op: ADD,
            a: Cell(1, 1),
            b: Number(2)
        },
        "the formula travelled as data"
    );
    assert!(world.host_errors().is_empty());
}

#[test]
fn a_chain_of_references_follows_its_source_when_that_changes_on_another_copy() {
    let mut world = sheet(2, 20);
    type_into(&mut world, "alice", 1, 1, 1);
    type_formula(&mut world, "bob", (1, 2), COPY, Cell(1, 1), Number(0));
    type_formula(&mut world, "bob", (1, 3), MULTIPLY, Cell(1, 2), Number(10));
    quiet(&mut world);
    let shown = agreed_shown(&mut world);
    assert_eq!((shown[&(1, 2)], shown[&(1, 3)]), ((1, 0), (10, 0)));

    // Alice changes the source, and bob's chain, which he wrote and she did not,
    // moves on every copy.
    world.advance_ms(100);
    type_into(&mut world, "alice", 1, 1, 7);
    quiet(&mut world);
    let shown = agreed_shown(&mut world);
    assert_eq!((shown[&(1, 2)], shown[&(1, 3)]), ((7, 0), (70, 0)));
    assert_eq!(shown, expected_shown(&world));
}

#[test]
fn two_formulas_that_each_read_the_other_are_a_cycle_on_every_copy() {
    let mut world = sheet(3, 20);
    world.partition("alice", "server");
    world.partition("bob", "server");
    // Each is acyclic alone: A1 reads the empty B1, and B1 reads the empty A1.
    type_formula(&mut world, "alice", (1, 1), COPY, Cell(1, 2), Number(0));
    type_formula(&mut world, "bob", (1, 2), COPY, Cell(1, 1), Number(0));
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    assert_eq!(shown_of(&mut world, "alice")[&(1, 1)], (0, 0));
    assert_eq!(shown_of(&mut world, "bob")[&(1, 2)], (0, 0));

    world.heal("alice", "server");
    world.heal("bob", "server");
    quiet(&mut world);

    let shown = agreed_shown(&mut world);
    assert_eq!(shown[&(1, 1)], (0, CYCLE));
    assert_eq!(shown[&(1, 2)], (0, CYCLE));
    assert_eq!(shown, expected_shown(&world));

    // A third cell that reads the cycle shows it too, and breaking the cycle with a
    // number frees them all.
    type_formula(&mut world, "alice", (1, 3), ADD, Cell(1, 1), Number(1));
    quiet(&mut world);
    assert_eq!(agreed_shown(&mut world)[&(1, 3)], (0, CYCLE));
    world.advance_ms(100);
    type_into(&mut world, "bob", 1, 1, 3);
    quiet(&mut world);
    let shown = agreed_shown(&mut world);
    assert_eq!(
        (shown[&(1, 1)], shown[&(1, 2)], shown[&(1, 3)]),
        ((3, 0), (3, 0), (4, 0))
    );
    assert_eq!(shown, expected_shown(&world));
}

#[test]
fn a_division_by_zero_and_an_empty_cell_have_the_values_the_library_defines() {
    let mut world = sheet(4, 20);
    type_into(&mut world, "alice", 1, 1, 0);
    type_formula(&mut world, "alice", (1, 2), DIVIDE, Number(7), Cell(1, 1));
    // The reference to a place no cell holds reads 0.
    type_formula(&mut world, "bob", (1, 3), ADD, Cell(9, 9), Number(4));
    type_formula(&mut world, "bob", (1, 4), ADD, Cell(1, 2), Number(1));
    type_formula(
        &mut world,
        "bob",
        (1, 5),
        DIVIDE,
        Number(i64::MIN),
        Number(-1),
    );
    type_formula(&mut world, "bob", (1, 6), ADD, Number(i64::MAX), Number(1));
    quiet(&mut world);

    let shown = agreed_shown(&mut world);
    assert_eq!(shown[&(1, 2)], (0, DIVIDED_BY_ZERO));
    assert_eq!(shown[&(1, 3)], (4, 0));
    assert_eq!(
        shown[&(1, 4)],
        (0, DIVIDED_BY_ZERO),
        "the fault reaches its readers"
    );
    assert_eq!(shown[&(1, 5)], (0, OVERFLOWED));
    assert_eq!(shown[&(1, 6)], (0, OVERFLOWED));
    assert_eq!(shown, expected_shown(&world));
}

/// A sum covers the rows and columns between its corners in the order the sheet holds
/// them, so a row another person inserts between them is summed, and one removed is not.
#[test]
fn a_sum_follows_the_order_of_the_rows_and_columns() {
    let mut world = ordered_sheet(5, 20);
    let top = add_row(&mut world, "alice", 0);
    let bottom = add_row(&mut world, "alice", top);
    let left = add_col(&mut world, "alice", 0);
    let total = add_col(&mut world, "alice", left);
    quiet(&mut world);
    type_into(&mut world, "alice", top, left, 1);
    type_into(&mut world, "alice", bottom, left, 2);
    type_formula(
        &mut world,
        "alice",
        (top, total),
        SUM,
        Cell(top, left),
        Cell(bottom, left),
    );
    quiet(&mut world);
    assert_eq!(agreed_shown(&mut world)[&(top, total)], (3, 0));

    // Bob puts a row between the corners and types into it.
    let between = add_row(&mut world, "bob", top);
    quiet(&mut world);
    let (rows, _) = agreed_rows(&mut world);
    assert_eq!(rows, [top, between, bottom]);
    type_into(&mut world, "bob", between, left, 100);
    quiet(&mut world);
    assert_eq!(agreed_shown(&mut world)[&(top, total)], (103, 0));

    // Alice removes it again: its mark keeps its place, and its number is not summed.
    drop_row(&mut world, "alice", between);
    quiet(&mut world);
    let shown = agreed_shown(&mut world);
    assert_eq!(shown[&(top, total)], (3, 0));
    assert_eq!(shown[&(between, left)], (100, 0), "hidden, not erased");
    assert_eq!(shown, expected_shown(&world));

    // A corner outside the orders is a fault, not a guess.
    type_formula(
        &mut world,
        "bob",
        (bottom, total),
        SUM,
        Cell(top, left),
        Cell(1, 1),
    );
    quiet(&mut world);
    assert_eq!(agreed_shown(&mut world)[&(bottom, total)], (0, BAD_RANGE));
    assert!(world.host_errors().is_empty());
}

#[test]
fn formulas_written_offline_survive_the_merge() {
    let mut world = sheet(6, 20);
    world.partition("bob", "server");
    type_into(&mut world, "alice", 1, 1, 4);
    type_formula(&mut world, "alice", (1, 2), MULTIPLY, Cell(1, 1), Number(3));
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    // Bob holds none of alice's cells, so what he reads reads 0.
    type_formula(&mut world, "bob", (1, 3), ADD, Cell(1, 2), Cell(1, 1));
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    assert_eq!(shown_of(&mut world, "bob")[&(1, 3)], (0, 0));
    assert_eq!(shown_of(&mut world, "server")[&(1, 2)], (12, 0));

    world.heal("bob", "server");
    quiet(&mut world);

    let shown = agreed_shown(&mut world);
    assert_eq!(shown[&(1, 2)], (12, 0));
    assert_eq!(shown[&(1, 3)], (16, 0));
    assert_eq!(shown, expected_shown(&world));
    for who in ["alice", "bob", "server"] {
        assert_eq!(
            contents_of(&mut world, who),
            oracle_contents(&world),
            "{who}"
        );
    }
}

/// The later write wins a cell whole: a formula over a number and a number over a
/// formula, each replacing all of what was there.
#[test]
fn a_later_write_replaces_a_formula_with_a_number_and_back() {
    let mut world = sheet(7, 20);
    type_into(&mut world, "alice", 1, 1, 2);
    type_formula(&mut world, "alice", (1, 2), COPY, Cell(1, 1), Number(0));
    quiet(&mut world);
    world.advance_ms(100);
    type_into(&mut world, "bob", 1, 2, 50);
    quiet(&mut world);
    assert_eq!(agreed_shown(&mut world)[&(1, 2)], (50, 0));
    world.advance_ms(100);
    type_formula(&mut world, "alice", (1, 2), ADD, Cell(1, 1), Number(1));
    quiet(&mut world);
    assert_eq!(agreed_shown(&mut world)[&(1, 2)], (3, 0));
    assert_eq!(contents_of(&mut world, "bob"), oracle_contents(&world));
}

/// A random afternoon of numbers and formulas over a small grid of rows and columns, with
/// links cut and restored, delays spread out and envelopes delivered twice: for every
/// seed the three copies hold the cells the oracle says, show what the oracle works out
/// from them, and show it identically.
#[test]
fn a_random_afternoon_of_formulas_ends_with_one_sheet() {
    for seed in 0..40 {
        let mut rng = Rng::new(seed);
        let mut world = ordered_sheet(seed, 5);
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        // The grid: three rows and three columns, which each person starts their
        // stream to the hub with (the other tests of ordered bindings say why). A
        // person adds after a row or a column they hold, so the first of each is
        // let through before the others are added.
        let mut rows = vec![add_row(&mut world, "alice", 0)];
        let mut cols = vec![add_col(&mut world, "alice", 0)];
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "seed {seed}");
        rows.push(add_row(&mut world, "bob", 0));
        rows.push(add_row(&mut world, "alice", rows[0]));
        cols.push(add_col(&mut world, "bob", 0));
        cols.push(add_col(&mut world, "bob", cols[0]));
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "seed {seed}");

        // Mostly places the grid holds, sometimes one it does not.
        let pick = |rng: &mut Rng, ids: &[u64]| -> u64 {
            if rng.below(12) == 0 {
                7
            } else {
                ids[usize::try_from(rng.below(ids.len() as u64)).unwrap()]
            }
        };
        let numbers = [i64::MIN, -3, -1, 0, 0, 1, 2, 3, 100, i64::MAX];
        let mut cut = [false, false];
        for _ in 0..40 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            let place = (pick(&mut rng, &rows), pick(&mut rng, &cols));
            let operand = |rng: &mut Rng| {
                if rng.below(10) < 7 {
                    Cell(pick(rng, &rows), pick(rng, &cols))
                } else {
                    Number(numbers[usize::try_from(rng.below(numbers.len() as u64)).unwrap()])
                }
            };
            match rng.below(10) {
                0..=2 => type_into(
                    &mut world,
                    who,
                    place.0,
                    place.1,
                    numbers[usize::try_from(rng.below(numbers.len() as u64)).unwrap()],
                ),
                3 => {
                    let (a, b) = (
                        Cell(pick(&mut rng, &rows), pick(&mut rng, &cols)),
                        Cell(pick(&mut rng, &rows), pick(&mut rng, &cols)),
                    );
                    type_formula(&mut world, who, place, SUM, a, b);
                }
                4 if rng.below(6) == 0 => {
                    let row = rows[usize::try_from(rng.below(rows.len() as u64)).unwrap()];
                    drop_row(&mut world, who, row);
                }
                _ => {
                    let op = [COPY, ADD, SUBTRACT, MULTIPLY, DIVIDE, 9]
                        [usize::try_from(rng.below(6)).unwrap()];
                    let (a, b) = (operand(&mut rng), operand(&mut rng));
                    type_formula(&mut world, who, place, op, a, b);
                }
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

        let expected = oracle_contents(&world);
        for who in ["alice", "bob", "server"] {
            assert_eq!(contents_of(&mut world, who), expected, "seed {seed}: {who}");
        }
        let want = expected_shown(&world);
        for who in ["alice", "bob", "server"] {
            let held = shown_of(&mut world, who);
            let variables = variables_of(&mut world, who);
            assert_eq!(
                held,
                want,
                "seed {seed}: {who} (refusals {}, orphans {}) holds the rows {} (removed {}) and the columns {} (removed {}); the oracle has {:?} and {:?}",
                variables["refusals"],
                variables["orphans"],
                variables["rows"],
                variables["gone"],
                variables["cols"],
                variables["goneCols"],
                oracle_order(&world, "sheet.row", "sheet.gone"),
                oracle_order(&world, "sheet.col", "sheet.gone_col"),
            );
        }
        assert_eq!(want.len(), expected.len(), "seed {seed}");
        assert!(world.host_errors().is_empty(), "seed {seed}");
    }
}
