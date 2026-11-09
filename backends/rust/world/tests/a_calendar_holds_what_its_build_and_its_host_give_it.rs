// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// How much a shared schedule holds: what the build gives it and what the host gives a
// list of it. A copy keeps every list in a room it is built with (`sce:limit`, the bounds a
// deployment chooses: docs/SCE_ACCEPTED_SUBSET.md §2.15), and past the room a write fails;
// so a list the host lets grow to the end of the room is a list that can fail half way,
// which no copy can undo. The host therefore gives each list less than the build holds
// (`todo.room`: values) and a copy refuses a change of its own, whole, before it stamps or
// sends anything, when it would hold more values than it was let: the list is full
// (`full`). A copy keeps only what is current (docs/adr/0085, section 2), so a change that
// writes over as many values as it writes never fills it.
//
// The world builds the to-do list three times: as written, with a handful of room (`tiny`),
// and with the room of a team of twenty with a great many events (`large`, which C11 runs
// too, docs/adr/0082). What is asked is that a list refuses at the room the host gave it
// and not before, and that every copy agrees on what it holds when it does; that a change to
// a register that has a value is not refused for the values, since it adds none, however
// often it is written over; that a list given more room by another account's host out of the
// same build is given it; and that the large build holds, over a calendar a build as written
// cannot, and keeps what every other test asks of a list: the copies agree, a copy whose
// links lost everything is repaired, and every layer forgets once every copy holds a change.

mod common;

use common::repair::lose_everything;
use common::*;
use sce_rust_world::{Outcome, World};
use serde_json::Value;

fn quiet(world: &mut World) {
    assert_eq!(world.run_until_quiet(600_000), Outcome::Quiet);
}

/// What a test reads of a copy of the list built as `$peer`: a machine saves and restores
/// through functions its own generated code defines, so the readers are written once for
/// each build and not once over all of them.
macro_rules! readers {
    ($name:ident, $peer:ty, $persist:path) => {
        // A build takes the readers it needs: the one as written asks no more than its
        // control does, and the trait is in scope already for it.
        #[allow(dead_code, unused_imports)]
        mod $name {
            use super::*;
            use $persist as _;

            pub type Peer = $peer;

            /// The variables a copy saves.
            pub fn variables(world: &mut World, who: &str) -> Value {
                let saved = world
                    .actor::<Peer>(who)
                    .engine()
                    .save_at(0)
                    .expect("a running machine saves")
                    .to_json();
                let saved: Value = serde_json::from_str(&saved).expect("a saved state is JSON");
                saved["variables"].clone()
            }

            pub fn count(world: &mut World, who: &str, name: &str) -> usize {
                variables(world, who)[name]
                    .as_array()
                    .unwrap_or_else(|| panic!("{name} is a list"))
                    .len()
            }

            pub fn number(world: &mut World, who: &str, name: &str) -> i128 {
                whole(&variables(world, who)[name])
            }

            /// What a copy holds of the registers, in an order that does not depend on
            /// the order the writes came in: the (to-do, field, number, stamp) of every
            /// value.
            pub fn held(world: &mut World, who: &str) -> Vec<(i128, i128, i128, i128, i128, i128)> {
                let mut all: Vec<_> = variables(world, who)["values"]
                    .as_array()
                    .expect("values are a list")
                    .iter()
                    .map(|v| {
                        (
                            whole(&v["id"]),
                            whole(&v["field"]),
                            whole(&v["value"]),
                            whole(&v["wallTime"]),
                            whole(&v["counter"]),
                            whole(&v["nodeId"]),
                        )
                    })
                    .collect();
                all.sort_unstable();
                all
            }

            /// Every copy holds what the first holds.
            pub fn agree(world: &mut World) {
                let first = held(world, COPIES[0]);
                for who in &COPIES[1..] {
                    assert_eq!(
                        held(world, who),
                        first,
                        "{who} and {} hold other values",
                        COPIES[0]
                    );
                }
            }
        }
    };
}

readers!(written, common::TodoPeer, common::TodoPeerPersist);
readers!(
    tiny,
    common::tiny::todo_peer::TodoPeerPolicy,
    common::tiny::todo_peer::TodoPeerPersist
);
readers!(
    large,
    common::large::todo_peer::TodoPeerPolicy,
    common::large::todo_peer::TodoPeerPersist
);

/// The host gives every copy the same room.
fn give_room(world: &mut World, values: u32) {
    for who in COPIES {
        world.raise(who, "todo.room", &format!(r#"{{"values":{values}}}"#));
    }
    quiet(world);
}

fn tiny_world(seed: u64) -> World {
    let mut world = todo_world_of(seed, 20, |world, who, router| {
        world.add(who, tiny::Peer::new(), router);
    });
    quiet(&mut world);
    world
}

/// A list refuses a change that would give it one more value than the host let it, and
/// every other change goes through: the copies hold exactly what they were let, nobody
/// holds half a change, and the room the build has is not reached.
#[test]
fn a_list_refuses_at_the_room_its_host_gave_it_and_not_before() {
    let mut world = tiny_world(1);
    give_room(&mut world, 8);

    // Twelve registers of one to-do, written one after another by alice.
    for field in 1..=12u64 {
        todo_set(&mut world, "alice", 1, field, 100 + field as i64);
        quiet(&mut world);
        world.advance_ms(1_000);
    }
    assert_eq!(tiny::number(&mut world, "alice", "full"), 4);
    for who in COPIES {
        assert_eq!(tiny::count(&mut world, who, "values"), 8, "{who}");
        assert_eq!(
            tiny::number(&mut world, who, "refusals"),
            0,
            "{who}: the build's room"
        );
    }
    assert_eq!(tiny::number(&mut world, "bob", "full"), 0);
    assert_eq!(tiny::number(&mut world, "carol", "full"), 0);
    tiny::agree(&mut world);

    // A register that has a value takes another: it adds none.
    todo_set(&mut world, "bob", 1, 1, 999);
    quiet(&mut world);
    assert_eq!(tiny::number(&mut world, "bob", "full"), 0);
    for who in COPIES {
        assert_eq!(tiny::count(&mut world, who, "values"), 8, "{who}");
    }
    tiny::agree(&mut world);
    assert!(world.host_errors().is_empty());
}

/// A register written over again and again holds one value, and the copy keeps nothing of
/// what it wrote over: a list let hold one value takes every one of a dozen changes to it,
/// and refuses none. Red if a write-over leaves anything behind in the copy that counts
/// against its room.
#[test]
fn a_register_written_over_again_and_again_is_never_refused() {
    let mut world = tiny_world(2);
    give_room(&mut world, 1);
    todo_set(&mut world, "alice", 1, 1, 1);
    quiet(&mut world);
    for round in 0..12 {
        world.advance_ms(1_000);
        let who = COPIES[round % 3];
        todo_set(&mut world, who, 1, 1, 10 + round as i64);
        quiet(&mut world);
    }
    for who in COPIES {
        assert_eq!(tiny::number(&mut world, who, "full"), 0, "{who}");
        assert_eq!(tiny::count(&mut world, who, "values"), 1, "{who}");
        assert_eq!(tiny::count(&mut world, who, "adds"), 1, "{who}");
        assert_eq!(tiny::number(&mut world, who, "refusals"), 0, "{who}");
    }
    tiny::agree(&mut world);
    assert!(world.host_errors().is_empty());
}

/// Two lists out of one build are given the room their hosts give them: the same writes are
/// refused at eight in one and go through to twelve in the other.
#[test]
fn two_accounts_of_one_build_are_given_each_their_own_room() {
    let writes = |room: u32| -> (i128, usize) {
        let mut world = tiny_world(3);
        give_room(&mut world, room);
        for field in 1..=12u64 {
            todo_set(&mut world, "alice", 1, field, field as i64);
            quiet(&mut world);
            world.advance_ms(1_000);
        }
        tiny::agree(&mut world);
        (
            tiny::number(&mut world, "alice", "full"),
            tiny::count(&mut world, "carol", "values"),
        )
    };
    assert_eq!(writes(8), (4, 8));
    assert_eq!(writes(12), (0, 12));
}

const FIELDS: [u64; 6] = [16, 17, 18, 19, 20, 21];

/// The events `first` to `last` of a calendar, one after another, alice's and bob's by
/// turns, each with the six fields of a plain schedule, over links that lose nothing.
fn write_calendar(world: &mut World, first: u64, last: u64) {
    for id in first..=last {
        let who = if id % 2 == 0 { "alice" } else { "bob" };
        for field in FIELDS {
            todo_set(world, who, id, field, (id * 100 + field) as i64);
        }
        world.advance_ms(60_000);
    }
}

/// The build with the room of a team: a calendar of more events than the build as written
/// holds is held whole, the copies agree on it, a copy whose links lost everything for half of
/// it is repaired, and every layer forgets once every copy holds a change. The build as
/// written is the control: the same calendar passes its room and it refuses.
#[test]
fn a_large_build_holds_a_calendar_a_build_as_written_cannot() {
    const EVENTS: u64 = 300;
    let mut world = todo_world_of(11, 20, |world, who, router| {
        world.add(who, large::Peer::new(), router);
    });
    quiet(&mut world);
    give_room(&mut world, 60_000);

    let started = std::time::Instant::now();
    write_calendar(&mut world, 1, EVENTS / 2);
    quiet(&mut world);
    lose_everything(&mut world, "carol", true);
    write_calendar(&mut world, EVENTS / 2 + 1, EVENTS);
    lose_everything(&mut world, "carol", false);
    quiet(&mut world);
    println!(
        "{} writes in {:?}",
        EVENTS * FIELDS.len() as u64,
        started.elapsed()
    );

    let wanted = (EVENTS * FIELDS.len() as u64) as usize;
    for who in COPIES {
        assert_eq!(large::count(&mut world, who, "values"), wanted, "{who}");
        assert_eq!(large::number(&mut world, who, "refusals"), 0, "{who}");
        assert_eq!(large::number(&mut world, who, "full"), 0, "{who}");
    }
    large::agree(&mut world);
    assert_layers_rest(&mut world, &COPIES, "the large build");
    assert!(world.host_errors().is_empty());

    // The control: the build as written runs out of room over the same calendar.
    let mut written_world = todo_world(11, 20);
    quiet(&mut written_world);
    write_calendar(&mut written_world, 1, EVENTS);
    quiet(&mut written_world);
    let refused: i128 = COPIES
        .iter()
        .map(|who| written::number(&mut written_world, who, "refusals"))
        .sum();
    assert!(
        refused > 0,
        "the build as written holds the calendar the large one does"
    );
}
