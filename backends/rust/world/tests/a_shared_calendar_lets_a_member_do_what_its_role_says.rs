// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a member of a shared schedule may do. The host knows who a member is and tells every
// copy the member's role (`todo.role`): 1 may read, 2 may also write, 3 may also do what an
// owner does. A list the host has not set up has no roster and refuses no one.
//
// Every copy judges a change by the roster, its own and its peers' alike. A person's own copy
// refuses a change its member may not make, whole, before it stamps or sends anything, and
// counts it (`denied`): the member is told at once and nothing of it exists anywhere. A copy
// given a peer's change whose author may not write takes it in as no change, and counts it:
// that is the check that does not rest on every copy being honest, made by every copy, since
// no hub is left to make it for the others (docs/adr/0085, section 2.4). A copy that was never
// told, or that was changed, sends a change that no other copy takes in.
//
// What is asked here is that a list with no roster refuses no one; that a reader's change is
// refused by its own copy and no one else holds a trace of it; that a change a member that may
// not write sends is dropped and counted by every copy it reaches; that a writer's changes go
// through and reach every copy; that a role the host gives later takes effect for what comes
// after it, and a role of 0 takes the member off; and that the people the host named go on
// agreeing on one list whatever was refused.

mod common;

use common::*;
use sce_rust_world::{Outcome, World};

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

fn denied(world: &mut World, who: &str) -> i128 {
    whole(&todo_variables_of(world, who)["denied"])
}

/// The host tells every copy the member's role.
fn give_role(world: &mut World, node: u64, role: u8) {
    for who in COPIES {
        world.raise(
            who,
            "todo.role",
            &format!(r#"{{"node":"{node}","role":{role}}}"#),
        );
    }
    quiet(world);
}

/// How many fields the three copies hold, alike.
fn agree(world: &mut World) -> usize {
    agreed_todos(world).len()
}

/// A list the host has not set up refuses no one.
#[test]
fn a_list_with_no_roster_refuses_no_one() {
    let mut world = todo_world(1, 20);
    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);
    assert_eq!(agree(&mut world), 2);
    for who in COPIES {
        assert_eq!(denied(&mut world, who), 0, "{who}");
    }
}

/// A reader's change is refused by the reader's own copy, whole: nobody holds a trace of it,
/// and the writer's goes through as it always did.
#[test]
fn a_readers_change_is_refused_by_its_own_copy() {
    let mut world = todo_world(2, 20);
    give_role(&mut world, 1, 2);
    give_role(&mut world, 2, 1);
    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    todo_set(&mut world, "bob", 3, DUE, 30);
    quiet(&mut world);

    assert_eq!(
        denied(&mut world, "bob"),
        2,
        "bob's two changes are refused by bob"
    );
    for who in ["alice", "carol"] {
        assert_eq!(
            denied(&mut world, who),
            0,
            "{who} was sent nothing to refuse"
        );
    }
    assert_eq!(
        agree(&mut world),
        1,
        "only alice's change exists, on every copy"
    );
    assert!(world.host_errors().is_empty());
}

/// A change that a member who may not write sent its group (its copy was never told, or was
/// changed) is taken in as no change and counted by every copy that was told, and none of them
/// holds it: every copy is the check that does not rest on the others. Red if a copy judges only
/// its own member's changes and takes a peer's in as it is.
#[test]
fn every_copy_drops_what_a_member_that_may_not_write_sends_it() {
    let mut world = todo_world(3, 20);
    // Alice and carol are told; bob's copy is not: it believes it may write.
    for who in ["alice", "carol"] {
        world.raise(who, "todo.role", r#"{"node":"1","role":2}"#);
        world.raise(who, "todo.role", r#"{"node":"2","role":1}"#);
        world.raise(who, "todo.role", r#"{"node":"3","role":2}"#);
    }
    quiet(&mut world);

    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);

    assert_eq!(denied(&mut world, "bob"), 0, "bob's copy was not told");
    for who in ["alice", "carol"] {
        assert_eq!(denied(&mut world, who), 1, "{who} counts what it dropped");
        let held = todos_of(&mut world, who);
        assert!(held.contains_key(&(1, DUE)), "{who}");
        assert!(
            !held.contains_key(&(2, DUE)),
            "{who} holds a change of a member that may not write"
        );
    }
    agreed_todos_of(&mut world, &["alice", "carol"]);
    assert!(todos_of(&mut world, "bob").contains_key(&(2, DUE)));
    assert!(world.host_errors().is_empty());
}

/// A role the host gives later takes effect for what comes after it: a reader that is made a
/// writer writes, a writer that is made a reader is refused, and a role of 0 takes the member
/// off the roster, so that it may not write on a list that has one.
#[test]
fn a_role_given_later_takes_effect_for_what_comes_after() {
    let mut world = todo_world(4, 20);
    give_role(&mut world, 1, 2);
    give_role(&mut world, 2, 1);
    give_role(&mut world, 3, 2);
    todo_set(&mut world, "bob", 1, DUE, 1);
    quiet(&mut world);
    assert_eq!(denied(&mut world, "bob"), 1);

    give_role(&mut world, 2, 2);
    todo_set(&mut world, "bob", 2, DUE, 2);
    quiet(&mut world);
    assert_eq!(denied(&mut world, "bob"), 1, "bob may write now");

    give_role(&mut world, 1, 1);
    todo_set(&mut world, "alice", 3, DUE, 3);
    quiet(&mut world);
    assert_eq!(denied(&mut world, "alice"), 1, "alice is a reader now");

    give_role(&mut world, 2, 0);
    todo_set(&mut world, "bob", 4, DUE, 4);
    quiet(&mut world);
    assert_eq!(
        denied(&mut world, "bob"),
        2,
        "a member off the roster of a list that has one may not write"
    );
    let list = todos_of(&mut world, "carol");
    assert!(list.contains_key(&(2, DUE)));
    assert!(!list.contains_key(&(1, DUE)));
    assert!(!list.contains_key(&(3, DUE)));
    assert!(!list.contains_key(&(4, DUE)));
    agree(&mut world);
    assert!(world.host_errors().is_empty());
}

/// The people the host named, who are all writers, go on agreeing on one list over links
/// that reorder and repeat, whatever a reader tried.
#[test]
fn writers_agree_on_one_list_whatever_a_reader_tried() {
    let mut world = todo_world(5, 20);
    for (a, b) in links_of(&COPIES) {
        world.reorder(a, b);
        world.degrade(a, b, 0, 20);
    }
    give_role(&mut world, 1, 2);
    give_role(&mut world, 2, 2);
    give_role(&mut world, 3, 2);
    for round in 0..6 {
        todo_set(&mut world, "alice", 1 + round % 3, DUE, round as i64);
        todo_set(
            &mut world,
            "bob",
            1 + (round + 1) % 3,
            DUE,
            100 + round as i64,
        );
        quiet(&mut world);
        world.advance_ms(1_000);
    }
    give_role(&mut world, 2, 1);
    todo_set(&mut world, "bob", 9, DUE, 9);
    todo_set(&mut world, "alice", 4, DUE, 4);
    quiet(&mut world);
    assert!(denied(&mut world, "bob") >= 1);
    let list = todos_of(&mut world, "carol");
    assert!(!list.contains_key(&(9, DUE)));
    assert!(list.contains_key(&(4, DUE)));
    agree(&mut world);
    assert!(world.host_errors().is_empty());
}
