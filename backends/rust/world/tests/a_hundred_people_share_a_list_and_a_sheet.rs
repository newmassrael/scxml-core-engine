// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A hundred people on one hub, with the shared to-do list (`todo_peer.scxml`) and the
// shared sheet (`sheet_peer.scxml`). The small tests of each ask whether two people and
// the hub end with the same thing after an afternoon of edits at random times over links
// that delay, reorder and repeat; this asks it of a hundred, where the hub passes every
// write on to all the others by a topic (tests/common/crowd.rs) and a field is written
// by several people before any of them has seen the others.
//
// The equalities are the small tests': every copy holds what the hub holds, and what
// the hub holds is what an oracle says, which reads the writes the network carried to
// the hub and no copy. The links lose nothing, since what the hub publishes to a topic
// is not sent again: a link that is cut and restored is the small tests' to ask.

mod common;

use common::crowd::*;
use common::*;
use sce_rust_world::{Outcome, Rng, World};

const PEOPLE: usize = 100;

/// The seeds of the afternoons run. A hundred copies each take every write, so a few
/// afternoons are as many as a debug build runs in a few seconds.
const SEEDS: std::ops::Range<u64> = 0..6;

fn one_of<'a, T>(rng: &mut Rng, items: &'a [T]) -> &'a T {
    &items[usize::try_from(rng.below(items.len() as u64)).unwrap()]
}

/// Links that delay a write by up to 45 ms, keep no order and repeat a fifth of what they
/// carry, between the hub and everyone.
fn make_links_rough(world: &mut World, names: &[String]) {
    for who in names {
        world.jitter(who, HUB, 40);
        world.reorder(who, HUB);
        world.degrade(who, HUB, 0, 20);
    }
}

fn todo_crowd(seed: u64, names: &[String]) -> World {
    let mut world = crowd_world(seed, names, 5, TodoPeer::new, "todo.join", |node, hub| {
        format!(r#"{{"node":"{node}","hub":{hub}}}"#)
    });
    // The copies take the join the host gave them before anything is written.
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet, "seed {seed}");
    make_links_rough(&mut world, names);
    world
}

fn sheet_crowd(seed: u64, names: &[String]) -> World {
    let mut world = crowd_world(seed, names, 5, Peer::new, "sheet.join", |node, hub| {
        format!(r#"{{"node":{node},"hub":{hub}}}"#)
    });
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet, "seed {seed}");
    make_links_rough(&mut world, names);
    world
}

/// An afternoon of changes to a few to-dos by people chosen at random, at random times:
/// every copy ends with the list the hub holds, which is the one the oracle says, and
/// a field two people wrote in the same instant, neither having seen the other's, holds
/// both values everywhere.
#[test]
fn a_hundred_people_changing_a_list_at_random_times_end_with_one_list() {
    const TITLES: [&str; 6] = [
        "milk",
        "post office",
        "\u{d560}\u{c77c}",
        "tax",
        "gym at 7",
        "abcdefghijklmnopqrstuvwxyz012345",
    ];
    const EDITS: usize = 110;
    let people = crowd(PEOPLE);
    for seed in SEEDS {
        let mut rng = Rng::new(seed);
        let mut world = todo_crowd(seed, &people);
        for _ in 0..EDITS {
            let who = one_of(&mut rng, &people);
            let id = 1 + rng.below(5);
            match rng.below(4) {
                0 => todo_title(&mut world, who, id, one_of(&mut rng, &TITLES)),
                1 => todo_set(
                    &mut world,
                    who,
                    id,
                    DONE,
                    i64::try_from(rng.below(2)).unwrap(),
                ),
                2 => todo_set(
                    &mut world,
                    who,
                    id,
                    DUE,
                    i64::try_from(rng.below(365)).unwrap(),
                ),
                _ => todo_set(
                    &mut world,
                    who,
                    id,
                    REMOVED,
                    i64::try_from(rng.below(2)).unwrap(),
                ),
            }
            world.run_until_quiet(i64::try_from(rng.below(40)).unwrap() + 1);
        }
        assert_eq!(
            world.run_until_quiet(600_000),
            Outcome::Quiet,
            "seed {seed}"
        );

        // A copy that refused a write is the first thing to say, before the copies are
        // compared: a list that differs because a block failed is not a merge fault.
        for who in people.iter().map(String::as_str).chain([HUB]) {
            assert_eq!(
                todo_variables_of(&mut world, who)["refusals"],
                0,
                "seed {seed}: {who} refused a write"
            );
        }
        let list = agreed_todos_among(&mut world, &people);
        assert_eq!(list, todo_oracle(&world), "seed {seed}");
        assert!(!list.is_empty(), "seed {seed}");

        // Then two people retitle one to-do in the same instant, neither having seen
        // the other's: the title keeps both, on every copy of the crowd.
        let first = one_of(&mut rng, &people[..PEOPLE / 2]);
        let second = one_of(&mut rng, &people[PEOPLE / 2..]);
        todo_title(&mut world, first, 1, "tea");
        todo_title(&mut world, second, 1, "coffee");
        assert_eq!(
            world.run_until_quiet(600_000),
            Outcome::Quiet,
            "seed {seed}"
        );
        let list = agreed_todos_among(&mut world, &people);
        assert_eq!(list, todo_oracle(&world), "seed {seed}");
        let title = &list[&(1, TITLE)];
        let texts: Vec<_> = title
            .live
            .iter()
            .filter_map(|held| held.text.as_deref())
            .collect();
        assert!(
            texts.contains(&"tea") && texts.contains(&"coffee"),
            "seed {seed}: the title keeps both of two writes made at once, and holds {texts:?}"
        );
        assert!(world.host_errors().is_empty(), "seed {seed}");
    }
}

/// An afternoon of numbers and formulas typed into a few cells at random times by the
/// two people the hub may write for, with ninety-eight more reading the sheet: every
/// copy ends with the cells the hub holds, which are the ones the oracle says, and shows
/// what the oracle works out from them. The others do not write because a copy of the
/// sheet sends a write that carries its own node to the hub, and the hub's topics send a
/// third person's write back to them (tests/common/crowd.rs).
#[test]
fn a_hundred_copies_of_a_sheet_two_people_type_into_end_with_one_sheet() {
    const EDITS: usize = 150;
    let numbers = [i64::MIN, -3, -1, 0, 0, 1, 2, 3, 100, i64::MAX];
    let people = crowd(PEOPLE);
    let writers = &people[..2];
    for seed in SEEDS {
        let mut rng = Rng::new(seed);
        let mut world = sheet_crowd(seed, &people);
        let place = |rng: &mut Rng| (rng.below(3), rng.below(3));
        for _ in 0..EDITS {
            let who = one_of(&mut rng, writers);
            let (row, col) = place(&mut rng);
            let operand = |rng: &mut Rng| {
                if rng.below(10) < 7 {
                    let (row, col) = place(rng);
                    Operand::Cell(row, col)
                } else {
                    Operand::Number(*one_of(rng, &numbers))
                }
            };
            if rng.below(3) == 0 {
                let op = *one_of(&mut rng, &[COPY, ADD, SUBTRACT, MULTIPLY, DIVIDE]);
                let (a, b) = (operand(&mut rng), operand(&mut rng));
                type_formula(&mut world, who, (row, col), op, a, b);
            } else {
                type_into(&mut world, who, row, col, *one_of(&mut rng, &numbers));
            }
            world.run_until_quiet(i64::try_from(rng.below(40)).unwrap() + 1);
        }
        assert_eq!(
            world.run_until_quiet(600_000),
            Outcome::Quiet,
            "seed {seed}"
        );

        let sheet = agreed_among(&mut world, &people);
        assert_eq!(sheet, oracle(&world), "seed {seed}");
        let contents = agreed_contents_among(&mut world, &people);
        assert_eq!(contents, oracle_contents(&world), "seed {seed}");
        let shown = agreed_shown_among(&mut world, &people);
        assert_eq!(shown, expected_shown(&world), "seed {seed}");
        assert_eq!(shown.len(), contents.len(), "seed {seed}");
        assert!(world.host_errors().is_empty(), "seed {seed}");
    }
}
