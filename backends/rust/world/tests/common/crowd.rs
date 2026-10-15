// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of a crowd have in common: a hundred people on the hub of the to-do list
// or of the sheet, and the ways of holding every copy to the one list or sheet the
// network says they should end with.
//
// The documents (`todo_peer.scxml`, `sheet_peer.scxml`) are written for two people: the
// hub passes a write on to `#alice` unless it is node 1's and to `#bob` unless it is node
// 2's. In a crowd those two names are topics ([`World::topic`]), which is how a hub that
// does not list its readers reaches a hundred of them: `#alice` is every person but the
// second and `#bob` every person but the first, so a write of the first or the second
// person reaches all the others once, and a write of any other person reaches each of
// the others twice and its writer twice as well, which a copy takes in as writes it
// already holds. Any of the hundred may write: a copy sends a write to the hub from the
// handler of its own edit and sends nothing for one it is given, so a write the hub sends
// back to its writer is applied once and ends there.

use std::collections::BTreeMap;

use sce_rust_mesh::router::Router;
use sce_rust_runtime::StatePolicy;
use sce_rust_world::World;

use super::{
    contents_of, peer, person, sheet_of, shown_of, todos_of, Content, Field, Held, Shown,
    GAP_TIMEOUT_MS,
};

/// The hub's name, which the documents write as `#server`.
pub const HUB: &str = "server";

/// The node id of the hub. It is no person's, and it is not 1 or 2, which the hub's
/// relaying names.
const HUB_NODE: u64 = 100_000;

/// Names for `count` people, in the order of their nodes.
pub fn crowd(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("c{n:03}")).collect()
}

/// The node id of the person at `index` of the crowd, from 1.
pub fn node_in_crowd(index: usize) -> u64 {
    u64::try_from(index).expect("an index") + 1
}

/// A world of the people `names`, each `latency_ms` from the hub, of the machines
/// `new` makes, which are told to start by the event `join`, with the data `data` gives
/// for a node and whether it is the hub's.
pub fn crowd_world<P: StatePolicy + 'static>(
    seed: u64,
    names: &[String],
    latency_ms: i64,
    new: impl Fn() -> P,
    join: &str,
    data: impl Fn(u64, bool) -> String,
) -> World {
    let mut world = World::new(seed);
    for who in names {
        world.add(who, new(), person(who, false));
    }
    let mut hub = Router::new(HUB, 256, GAP_TIMEOUT_MS).expect("a window the core can hold");
    for who in names {
        hub.add_peer(who, peer(&[], false));
    }
    for topic in ["alice", "bob"] {
        hub.add_peer(topic, peer(&[], false));
    }
    world.add(HUB, new(), hub);
    for who in names {
        world.link(who, HUB, latency_ms);
    }
    let but = |one: usize| -> Vec<&str> {
        names
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != one)
            .map(|(_, who)| who.as_str())
            .collect()
    };
    world.topic(HUB, "alice", &but(1));
    world.topic(HUB, "bob", &but(0));
    for (index, who) in names.iter().enumerate() {
        world.raise(who, join, &data(node_in_crowd(index), false));
    }
    world.raise(HUB, join, &data(HUB_NODE, true));
    world
}

/// Every copy of the crowd holds the list of to-dos the hub does; which.
pub fn agreed_todos_among(world: &mut World, names: &[String]) -> BTreeMap<(u64, u64), Field> {
    let server = todos_of(world, HUB);
    for who in names {
        assert_eq!(
            todos_of(world, who),
            server,
            "{who} and the hub hold different to-do lists"
        );
    }
    server
}

/// Every copy of the crowd holds the cells the hub does, with the stamps that put them
/// there; which.
pub fn agreed_among(world: &mut World, names: &[String]) -> BTreeMap<(u64, u64), Held> {
    let server = sheet_of(world, HUB);
    for who in names {
        assert_eq!(
            sheet_of(world, who),
            server,
            "{who} and the hub hold different sheets"
        );
    }
    server
}

/// Every copy of the crowd holds what the hub does in each cell, formulas and all; which.
pub fn agreed_contents_among(world: &mut World, names: &[String]) -> BTreeMap<(u64, u64), Content> {
    let server = contents_of(world, HUB);
    for who in names {
        assert_eq!(
            contents_of(world, who),
            server,
            "{who} and the hub hold different cells"
        );
    }
    server
}

/// Every copy of the crowd shows what the hub does; which.
pub fn agreed_shown_among(world: &mut World, names: &[String]) -> BTreeMap<(u64, u64), Shown> {
    let server = shown_of(world, HUB);
    for who in names {
        assert_eq!(
            shown_of(world, who),
            server,
            "{who} and the hub show different sheets"
        );
    }
    server
}
