// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of a copy that has lost changes for good have in common: the digest every
// copy keeps of its changes and a second statement of it, written here from what the copy
// holds, and the check that the three copies stand alike, as the oracle says, keeping the
// digest of exactly what they hold.

use std::collections::BTreeMap;

use sce_rust_world::World;
use serde_json::Value;

use super::{agreed_todos, todo_oracle, todo_variables_of, whole, Field};

/// A minute of the world's time, in milliseconds.
pub const MINUTE: i64 = 60_000;

/// The hash of one change, written here a second time beside the statechart's (`todo.keep`):
/// a 32-bit mix of the register, the dot and the kind, 0 for an add and 1 for a tombstone.
fn change_hash(kind: i128, id: i128, field: i128, wall: i128, counter: i128, node: i128) -> i128 {
    const M: i128 = 4_294_967_296;
    let mut h = (id * 16 + field) % M;
    h = (h * 1_000_003 + wall % M) % M;
    h = (h * 1_000_003 + counter) % M;
    h = (h * 1_000_003 + node % M) % M;
    h = (h * 1_000_003 + kind + 1) % M;
    h ^= h / 65_536;
    h = (h * 69_069 + 12_345) % M;
    h ^= h / 8_192;
    h
}

/// A digest as a map from (depth, prefix) to hash, without the nodes whose hash is 0: a
/// node that every change under it cancelled out is a node that holds nothing.
pub type Trie = BTreeMap<(u64, u64), i128>;

/// The digest a copy keeps.
pub fn digest_of(world: &mut World, who: &str) -> Trie {
    let variables = todo_variables_of(world, who);
    variables["digest"]
        .as_array()
        .expect("the digest is a list")
        .iter()
        .map(|node| {
            (
                (whole(&node["depth"]) as u64, whole(&node["prefix"]) as u64),
                whole(&node["hash"]),
            )
        })
        .filter(|(_, hash)| *hash != 0)
        .collect()
}

/// The digest of what a copy holds, built from its live writes and its tombstones and from
/// nothing the copy keeps as a digest.
pub fn digest_of_what_is_held(world: &mut World, who: &str) -> Trie {
    let variables = todo_variables_of(world, who);
    let mut changes: Vec<(i128, i128)> = Vec::new();
    for value in variables["values"].as_array().expect("values are a list") {
        let n = |name: &str| whole(&value[name]);
        let wall = n("wallTime");
        changes.push((
            wall / i128::from(MINUTE),
            change_hash(0, n("id"), n("field"), wall, n("counter"), n("nodeId")),
        ));
    }
    for tombstone in variables["tombstones"]
        .as_array()
        .expect("tombstones are a list")
    {
        let n = |name: &str| whole(&tombstone[name]);
        let wall = n("wallTime");
        let element = n("element");
        changes.push((
            wall / i128::from(MINUTE),
            change_hash(
                1,
                element / 16,
                element % 16,
                wall,
                n("counter"),
                n("nodeId"),
            ),
        ));
    }
    let mut trie = Trie::new();
    for (minute, hash) in changes {
        for depth in 0..=16u32 {
            let prefix = (minute / 3i128.pow(16 - depth)) as u64;
            *trie.entry((u64::from(depth), prefix)).or_insert(0) ^= hash;
        }
    }
    trie.retain(|_, hash| *hash != 0);
    trie
}

/// Every copy keeps the digest of what it holds, and the three keep one.
pub fn agreed_digest(world: &mut World) -> Trie {
    for who in ["alice", "bob", "server"] {
        assert_eq!(
            digest_of(world, who),
            digest_of_what_is_held(world, who),
            "{who} keeps a digest that is not the digest of what it holds"
        );
    }
    let server = digest_of(world, "server");
    for who in ["alice", "bob"] {
        assert_eq!(
            digest_of(world, who),
            server,
            "{who} and the hub keep different digests"
        );
    }
    server
}

pub fn refusals(world: &mut World, who: &str) -> Value {
    todo_variables_of(world, who)["refusals"].clone()
}

/// The three copies stand alike and as the oracle says, keep their digests, and refused
/// nothing; the list they agree on.
pub fn assert_converged(world: &mut World, context: &str) -> BTreeMap<(u64, u64), Field> {
    for who in ["alice", "bob", "server"] {
        assert_eq!(refusals(world, who), 0, "{context}: {who} refused a change");
    }
    let list = agreed_todos(world);
    assert_eq!(list, todo_oracle(world), "{context}");
    agreed_digest(world);
    assert!(world.host_errors().is_empty(), "{context}");
    list
}
