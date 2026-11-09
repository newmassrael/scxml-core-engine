// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Many people sharing the to-do list (`todo_peer.scxml`) and the sheet (`sheet_peer.scxml`).
// The small tests of each ask whether three copies end with the same thing after an afternoon
// of edits at random times over links that delay, reorder and repeat; this asks it of a
// crowd, where a field is written by several people before any of them has seen the others.
//
// The list's crowd is one causal group of as many members as a view holds, 64 (the bound of
// the vector an envelope carries, docs/adr/0085 section 3.2): every copy sends to the group
// and reads it, and the layer below the list delivers each change to every copy once. The
// sheet's is a hundred people on one hub, which passes every write on to all the others by a
// topic (tests/common/crowd.rs).
//
// The equalities are the small tests': every copy holds what the first holds, and that is
// what an oracle says, which reads the changes the network carried and no copy.

mod common;

use common::crowd::*;
use common::*;
use sce_rust_world::{Outcome, Rng, World};

/// The sheet's crowd.
const HUNDRED: usize = 100;

/// The list's crowd: as many as a causal group's view holds.
const MEMBERS: usize = VIEW_BOUND;

/// The seeds of the afternoons run. Every copy takes every write, so a few afternoons are as
/// many as a debug build runs in a few seconds.
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

/// The same between every two members of a group.
fn make_group_links_rough(world: &mut World, names: &[String]) {
    let copies: Vec<&str> = names.iter().map(String::as_str).collect();
    for (a, b) in links_of(&copies) {
        world.jitter(a, b, 40);
        world.reorder(a, b);
        world.degrade(a, b, 0, 20);
    }
}

fn todo_crowd(seed: u64, names: &[String]) -> World {
    let mut world = crowd_group_world(seed, names, 5, TodoPeer::new, "todo.join", |node| {
        format!(r#"{{"node":"{node}"}}"#)
    });
    make_group_links_rough(&mut world, names);
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

/// A crowd forgets a change once every member has said it holds it, over links that delay,
/// reorder and repeat, and not before: while one member is away, every other keeps what it
/// may be asked for, and once it is back every log is empty.
#[test]
fn a_crowd_forgets_a_change_once_every_member_holds_it() {
    let names = crowd(MEMBERS);
    let mut world = todo_crowd(7, &names);
    let mut rng = Rng::new(7);
    let logs = |world: &mut World| -> Vec<usize> {
        names
            .iter()
            .map(|who| world.causal_layer(who, PEOPLE).expect("a layer").log_len())
            .collect()
    };
    for round in 0..3 {
        for who in &names {
            let id = 1 + rng.below(6);
            todo_set(
                &mut world,
                who,
                id,
                DUE,
                i64::try_from(rng.below(10_000)).unwrap(),
            );
        }
        assert_eq!(
            world.run_until_quiet(600_000),
            Outcome::Quiet,
            "round {round}"
        );
        assert!(
            logs(&mut world).iter().all(|kept| *kept == 0),
            "round {round}"
        );
        world.advance_ms(60_000);
    }

    // One member is away: the others keep what it has not said it holds.
    let away = names[MEMBERS - 1].clone();
    for who in &names[..MEMBERS - 1] {
        world.degrade(&away, who, 100, 0);
    }
    for who in &names[..MEMBERS - 1] {
        todo_set(&mut world, who, 1 + rng.below(6), DONE, 1);
    }
    world.advance_ms(10_000);
    let kept = logs(&mut world);
    assert!(
        kept[..MEMBERS - 1].iter().all(|kept| *kept >= MEMBERS - 1),
        "a copy forgot what {away} has not said it holds: {kept:?}"
    );

    for who in &names[..MEMBERS - 1] {
        world.degrade(&away, who, 0, 20);
    }
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
    assert!(logs(&mut world).iter().all(|kept| *kept == 0));
    let list = agreed_todos_among(&mut world, &names);
    assert_eq!(list, todo_oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// An afternoon of changes to a few to-dos by members chosen at random, at random times:
/// every copy ends with one list, which is the one the oracle says, and a field two people
/// wrote in the same instant, neither having seen the other's, holds both values everywhere.
#[test]
fn a_crowd_changing_a_list_at_random_times_ends_with_one_list() {
    const TITLES: [&str; 6] = [
        "milk",
        "post office",
        "\u{d560}\u{c77c}",
        "tax",
        "gym at 7",
        "abcdefghijklmnopqrstuvwxyz012345",
    ];
    const EDITS: usize = 110;
    let people = crowd(MEMBERS);
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
        for who in people.iter().map(String::as_str) {
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
        let first = one_of(&mut rng, &people[..MEMBERS / 2]);
        let second = one_of(&mut rng, &people[MEMBERS / 2..]);
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
/// any of the hundred people: every copy ends with the cells the hub holds, which are the
/// ones the oracle says, and shows what the oracle works out from them. The hub's topics
/// send a third person's write back to its writer, which applies it and sends nothing
/// (tests/common/crowd.rs), so the write of any person ends where it lands.
#[test]
fn a_hundred_copies_of_a_sheet_a_hundred_people_type_into_end_with_one_sheet() {
    const EDITS: usize = 150;
    let numbers = [i64::MIN, -3, -1, 0, 0, 1, 2, 3, 100, i64::MAX];
    let people = crowd(HUNDRED);
    let writers = &people[..];
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
