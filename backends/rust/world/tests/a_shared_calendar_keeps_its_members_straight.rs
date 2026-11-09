// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared schedule among more people than three, whose members come and go. A team of six
// are the members of one causal group (tests/common/crowd.rs, docs/adr/0085 section 3.2):
// every copy sends its changes to the group and reads it, and the layer below the list
// delivers each change to every copy once, after what it follows, and forgets a change once
// every member of the view holds it. What is asked is what a team needs when one of its
// members is not in the room: that a member down the roster whose links lost everything is
// repaired as any is; that a member who was away while the others wrote holds what is current
// once it is back, and that its first write is taken in everywhere; that no copy forgets a
// change a member of the view has not said it holds, however long the member is away, and
// that every copy forgets it once the member is back; and that a member that made a change
// while it was away has it taken in by the others when it comes back, with no copy found
// stale, since none forgot what it needed.
//
// What the view does not do yet is change: a member that leaves for good holds the stable cut
// where it stopped until view changes land (landing 7), and the host cannot drop it from the
// group before then.

mod common;

use common::crowd::*;
use common::*;
use sce_rust_world::{Outcome, World};

const PEOPLE_IN_THE_TEAM: usize = 6;

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn team(seed: u64) -> (World, Vec<String>) {
    let names = crowd(PEOPLE_IN_THE_TEAM);
    let world = crowd_group_world(seed, &names, 20, TodoPeer::new, "todo.join", |node| {
        format!(r#"{{"node":"{node}"}}"#)
    });
    (world, names)
}

/// Every link of `who` to another member of the team loses everything it carries, both
/// ways, or carries it again (`lose` false).
fn lose_everything_in(world: &mut World, names: &[String], who: &str, lose: bool) {
    for other in names.iter().filter(|other| other.as_str() != who) {
        world.degrade(who, other, if lose { 100 } else { 0 }, 0);
    }
}

/// How many changes each member's layer keeps not yet stable.
fn logs(world: &mut World, names: &[String]) -> Vec<usize> {
    names
        .iter()
        .map(|who| {
            world
                .causal_layer(who, PEOPLE)
                .expect("a causal layer")
                .log_len()
        })
        .collect()
}

/// The members a team has when it is written to: each of `who` changes a few to-dos, and
/// the world's time runs on a minute.
fn an_afternoon(world: &mut World, who: &[&str], day: i64) {
    for (index, person) in who.iter().enumerate() {
        let id = 1 + (index as u64 + day as u64) % 4;
        todo_set(world, person, id, DUE, day * 100 + index as i64);
        todo_set(world, person, id, DONE, day % 2);
    }
    world.advance_ms(60_000);
}

/// A member down the roster whose links lost everything for a few afternoons is repaired as
/// any member is: the vectors find what it lacks and what the others lack of it.
#[test]
fn a_member_down_the_roster_is_repaired() {
    let (mut world, names) = team(1);
    let all: Vec<&str> = names.iter().map(String::as_str).collect();
    for day in 0..4 {
        an_afternoon(&mut world, &all, day);
    }
    quiet(&mut world);
    lose_everything_in(&mut world, &names, "c005", true);
    for day in 4..10 {
        an_afternoon(&mut world, &all, day);
    }
    lose_everything_in(&mut world, &names, "c005", false);
    quiet(&mut world);

    let list = agreed_todos_among(&mut world, &names);
    assert_eq!(list, todo_oracle(&world));
    assert_eq!(logs(&mut world, &names), vec![0; PEOPLE_IN_THE_TEAM]);
    assert!(world.host_errors().is_empty());
}

/// A member that was away while the others wrote holds what is current once its links are
/// back, and its first write is taken in everywhere: what the others kept for it is what it
/// is given.
#[test]
fn a_member_away_while_the_others_wrote_takes_what_is_current() {
    let (mut world, names) = team(3);
    let newcomer = names[PEOPLE_IN_THE_TEAM - 1].clone();
    let others: Vec<&str> = names[..PEOPLE_IN_THE_TEAM - 1]
        .iter()
        .map(String::as_str)
        .collect();
    for other in &others {
        world.partition(&newcomer, other);
    }
    for day in 0..8 {
        an_afternoon(&mut world, &others, day);
    }

    for other in &others {
        world.heal(&newcomer, other);
    }
    quiet(&mut world);
    assert_eq!(
        todos_of(&mut world, &newcomer),
        todos_of(&mut world, &names[0]),
        "the newcomer does not hold what is current"
    );

    world.advance_ms(60_000);
    todo_set(&mut world, &newcomer, 1, DUE, 4242);
    quiet(&mut world);
    let list = agreed_todos_among(&mut world, &names);
    assert_eq!(list[&(1, DUE)].value, 4242);
    assert_eq!(
        list[&(1, DUE)].live.len(),
        1,
        "the newcomer wrote over what it holds"
    );
    assert_eq!(logs(&mut world, &names), vec![0; PEOPLE_IN_THE_TEAM]);
    assert!(world.host_errors().is_empty());
}

/// The stable cut waits for every member of the view: while one is away and never says what
/// it holds, every other keeps in its log what it may be asked for, however long; once it is
/// back and repaired, every copy forgets. Red if a member not heard from is left out of the
/// cut: the others would then forget what the absent member lacks, and could not repair it.
#[test]
fn no_copy_forgets_what_a_member_away_has_not_said_it_holds() {
    let (mut world, names) = team(4);
    let gone = names[3].clone();
    let present: Vec<&str> = names
        .iter()
        .filter(|who| **who != gone)
        .map(String::as_str)
        .collect();
    for who in &present {
        world.partition(&gone, who);
    }
    for day in 0..6 {
        an_afternoon(&mut world, &present, day);
    }
    world.advance_ms(10 * 60_000);
    let kept = logs(&mut world, &names);
    for (who, kept) in names.iter().zip(&kept) {
        if *who != gone {
            assert!(
                *kept >= 6 * 2 * present.len(),
                "{who} forgot a change {gone} has not said it holds: it keeps {kept}"
            );
        }
    }

    for who in &present {
        world.heal(&gone, who);
    }
    quiet(&mut world);
    assert_eq!(logs(&mut world, &names), vec![0; PEOPLE_IN_THE_TEAM]);
    agreed_todos_among(&mut world, &names);
}

/// A member that made a change while it was away has it taken in by the others when it is
/// back, and takes in theirs: no copy forgot what another needed, so none is found stale and
/// none has to be started again.
#[test]
fn a_member_back_with_a_change_it_made_while_away_is_taken_in() {
    let (mut world, names) = team(5);
    let gone = names[3].clone();
    let all: Vec<&str> = names.iter().map(String::as_str).collect();
    let present: Vec<&str> = all.iter().copied().filter(|who| *who != gone).collect();
    for day in 0..3 {
        an_afternoon(&mut world, &all, day);
    }
    quiet(&mut world);
    for who in &present {
        world.partition(&gone, who);
    }
    todo_set(&mut world, &gone, 7, DUE, 777);
    world.advance_ms(60_000);
    for day in 3..8 {
        an_afternoon(&mut world, &present, day);
    }
    for who in &present {
        world.heal(&gone, who);
    }
    quiet(&mut world);

    let list = agreed_todos_among(&mut world, &names);
    assert_eq!(list[&(7, DUE)].value, 777);
    assert_eq!(list, todo_oracle(&world));
    for who in &names {
        assert_eq!(todo_variables_of(&mut world, who)["refusals"], 0, "{who}");
    }
    assert!(world.host_errors().is_empty());
}
