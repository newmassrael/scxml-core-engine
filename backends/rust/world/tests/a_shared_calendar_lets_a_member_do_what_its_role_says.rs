// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a member of a shared schedule may do. The host knows who a member is (it
// authenticated the connection) and tells every copy the member's role (`todo.role`): 1 may
// read, 2 may also write, 3 may also do what an owner does. A list the host has not set up
// has no roster and refuses no one.
//
// Two copies judge a change, and they judge it for different reasons. A person's own copy
// refuses a change its member may not make, whole, before it stamps or sends anything, and
// counts it (`denied`): the member is told at once and nothing of it exists anywhere. The
// hub refuses what it is sent by a member that may not write, which is the check that does
// not rest on every copy being honest: a copy that was never told, or that was changed, sends
// a write the hub does not take in, counts it, and does not pass it on.
//
// What is asked here is that a list with no roster refuses no one; that a reader's change is
// refused by its own copy and no one else holds a trace of it; that a write sent to the hub
// by a member that may not write is dropped and counted there and reaches no copy; that a
// writer's changes go through and reach every copy; that a role the host gives later takes
// effect for what comes after it, and a role of 0 takes the member off; and that the people
// the host named go on agreeing on one list whatever was refused.

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
    for who in ["alice", "bob", "server"] {
        world.raise(
            who,
            "todo.role",
            &format!(r#"{{"node":"{node}","role":{role}}}"#),
        );
    }
    quiet(world);
}

/// What alice's and bob's copies hold and what the hub's does, and the digests.
fn agree(world: &mut World) -> usize {
    let list = agreed_todos(world);
    let digest = todo_variables_of(world, "server")["digest"].clone();
    for who in ["alice", "bob"] {
        assert_eq!(
            todo_variables_of(world, who)["digest"],
            digest,
            "{who} and the hub hold other digests"
        );
    }
    list.len()
}

/// A list the host has not set up refuses no one.
#[test]
fn a_list_with_no_roster_refuses_no_one() {
    let mut world = todo_world(1, 20);
    quiet(&mut world);
    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);
    assert_eq!(agree(&mut world), 2);
    for who in ["alice", "bob", "server"] {
        assert_eq!(denied(&mut world, who), 0, "{who}");
    }
}

/// A reader's change is refused by the reader's own copy, whole: nobody holds a trace of it,
/// and the writer's goes through as it always did.
#[test]
fn a_readers_change_is_refused_by_its_own_copy() {
    let mut world = todo_world(2, 20);
    quiet(&mut world);
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
    assert_eq!(denied(&mut world, "alice"), 0);
    assert_eq!(
        denied(&mut world, "server"),
        0,
        "the hub was sent nothing to refuse"
    );
    assert_eq!(
        agree(&mut world),
        1,
        "only alice's change exists, on every copy"
    );
    assert!(world.host_errors().is_empty());
}

/// A write that a member who may not write sent the hub (its copy was never told, or was
/// changed) is dropped and counted at the hub, and reaches no copy: the hub is the check that
/// does not rest on the copies.
#[test]
fn the_hub_drops_what_a_member_that_may_not_write_sends_it() {
    let mut world = todo_world(3, 20);
    quiet(&mut world);
    // The hub and alice are told; bob's copy is not: it believes it may write.
    for who in ["alice", "server"] {
        world.raise(who, "todo.role", r#"{"node":"1","role":2}"#);
        world.raise(who, "todo.role", r#"{"node":"2","role":1}"#);
    }
    quiet(&mut world);

    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);

    assert_eq!(denied(&mut world, "bob"), 0, "bob's copy was not told");
    assert!(
        denied(&mut world, "server") >= 1,
        "the hub counts what it dropped"
    );
    let hub = todos_of(&mut world, "server");
    assert!(hub.contains_key(&(1, DUE)));
    assert!(
        !hub.contains_key(&(2, DUE)),
        "the hub holds a write of a member that may not write"
    );
    assert!(
        !todos_of(&mut world, "alice").contains_key(&(2, DUE)),
        "the write reached a copy"
    );
    assert!(world.host_errors().is_empty());
}

/// A role the host gives later takes effect for what comes after it: a reader that is made a
/// writer writes, a writer that is made a reader is refused, and a role of 0 takes the member
/// off the roster, so that it may not write on a list that has one.
#[test]
fn a_role_given_later_takes_effect_for_what_comes_after() {
    let mut world = todo_world(4, 20);
    quiet(&mut world);
    give_role(&mut world, 1, 2);
    give_role(&mut world, 2, 1);
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
    let list = todos_of(&mut world, "server");
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
    for link in ["alice", "bob"] {
        world.reorder(link, "server");
        world.degrade(link, "server", 0, 20);
    }
    quiet(&mut world);
    give_role(&mut world, 1, 2);
    give_role(&mut world, 2, 2);
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
    let list = todos_of(&mut world, "server");
    assert!(!list.contains_key(&(9, DUE)));
    assert!(list.contains_key(&(4, DUE)));
    agree(&mut world);
    assert!(world.host_errors().is_empty());
}

/// What a link may deliver is the host's to decide where the link is known: the host of the hub
/// forbids a member's link the writes, and what that member's copy sends under those names is
/// dropped before the hub's machine is told, whoever the envelope names as its source or the
/// write names as its author. The member still reads: what the others wrote reaches its copy.
/// Lifted, the member's copy walks the digest with the hub and its write is taken in.
#[test]
fn the_host_of_the_hub_drops_what_a_link_may_not_deliver() {
    let mut world = todo_world(6, 20);
    quiet(&mut world);
    world.forbid_events_from("server", "bob", &["todo.write", "todo.supersede"]);
    todo_set(&mut world, "alice", 1, DUE, 10);
    todo_set(&mut world, "bob", 2, DUE, 20);
    quiet(&mut world);

    let hub = todos_of(&mut world, "server");
    assert!(hub.contains_key(&(1, DUE)));
    assert!(
        !hub.contains_key(&(2, DUE)),
        "the hub holds a write of a link that may not deliver it"
    );
    assert!(!todos_of(&mut world, "alice").contains_key(&(2, DUE)));
    assert!(
        todos_of(&mut world, "bob").contains_key(&(1, DUE)),
        "the member still reads"
    );
    let errors = world.host_errors();
    assert!(
        errors.iter().any(|error| error.contains("EventForbidden")),
        "the host is told what it dropped: {errors:?}"
    );

    world.forbid_events_from("server", "bob", &[]);
    world.raise("bob", "todo.resync", "");
    quiet(&mut world);
    assert!(todos_of(&mut world, "server").contains_key(&(2, DUE)));
    assert_eq!(agree(&mut world), 2);
}
