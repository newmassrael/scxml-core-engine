// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared sheet of numbers: two people, `alice` and `bob`, each with a copy of the
// sheet (`machines/sheet_peer.scxml`), and the hub both send to, which is the same
// document told it is the hub. The people edit cells, offline and at the same time,
// and the question is the one a shared spreadsheet answers: do the three copies end
// with the same sheet, and is it the sheet the edits meant.
//
// Equality of the copies says they agree, not that they are right, so each run is
// also held to an oracle that does not read the copies: of all the writes the
// people sent, the one with the latest stamp is the cell's value.

use std::collections::BTreeMap;

use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::outbound::{OutboundBuffer, RetryPolicy};
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_world::{Fate, Outcome, Rng, World};
use serde_json::Value;

include!(concat!(env!("OUT_DIR"), "/machines.rs"));

use sheet_peer::{SheetPeerPersist, SheetPeerPolicy as Peer};

/// How the binding to a peer is configured: a WebSocket that delivers once, with
/// the envelopes written while the link is down kept until it is back.
fn peer(responders: &'static [&'static str]) -> PeerConfig {
    PeerConfig {
        transport: "wss",
        buffer: Some(OutboundBuffer {
            max_pending: 1_024,
            max_age_ms: 0,
        }),
        retry: Some(RetryPolicy {
            max_retries: 5,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1_000,
            jitter_pct: 0,
        }),
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders,
        deadline_ms: None,
        reply_events: &[],
    }
}

fn person(name: &str) -> Router {
    let mut router = Router::new(name, 256, 50).expect("a window the core can hold");
    router.add_peer("server", peer(&["server"]));
    router
}

fn hub() -> Router {
    let mut router = Router::new("server", 256, 50).expect("a window the core can hold");
    router.add_peer("alice", peer(&["alice"]));
    router.add_peer("bob", peer(&["bob"]));
    router
}

/// The sheet's world: alice (node 1) and bob (node 2), each `latency_ms` from the hub.
fn sheet(seed: u64, latency_ms: i64) -> World {
    let mut world = World::new(seed);
    world.add("alice", Peer::new(), person("alice"));
    world.add("bob", Peer::new(), person("bob"));
    world.add("server", Peer::new(), hub());
    world.link("alice", "server", latency_ms);
    world.link("bob", "server", latency_ms);
    world.raise("alice", "sheet.join", r#"{"node":1,"hub":false}"#);
    world.raise("bob", "sheet.join", r#"{"node":2,"hub":false}"#);
    world.raise("server", "sheet.join", r#"{"node":100,"hub":true}"#);
    world
}

/// A person types `value` into the cell at (`row`, `col`), when the host's clock
/// reads the world's time.
fn type_into(world: &mut World, who: &str, row: u32, col: u32, value: i32) {
    let now = world.now_ms();
    world.raise(
        who,
        "sheet.edit",
        &format!(r#"{{"row":{row},"col":{col},"value":{value},"now":{now}}}"#),
    );
}

/// One cell as a copy holds it: the number, and the stamp that put it there.
type Held = (i64, (u64, u64, u64));

/// The sheet a copy holds, by cell.
fn sheet_of(world: &mut World, who: &str) -> BTreeMap<(u64, u64), Held> {
    let saved = world
        .actor::<Peer>(who)
        .engine()
        .save_at(0)
        .expect("a running machine saves");
    let saved: Value = serde_json::from_str(&saved.to_json()).expect("a saved state is JSON");
    let mut cells = BTreeMap::new();
    for cell in saved["variables"]["cells"]
        .as_array()
        .expect("the cells are a list")
    {
        let number = |field: &str| {
            cell[field]
                .as_i64()
                .unwrap_or_else(|| panic!("{field} of {cell}"))
        };
        let key = (number("row") as u64, number("col") as u64);
        let stamp = (
            number("wallTime") as u64,
            number("counter") as u64,
            number("nodeId") as u64,
        );
        assert!(
            cells.insert(key, (number("value"), stamp)).is_none(),
            "{who} holds {key:?} twice"
        );
    }
    cells
}

/// The three copies hold one sheet; which.
fn agreed(world: &mut World) -> BTreeMap<(u64, u64), Held> {
    let alice = sheet_of(world, "alice");
    let bob = sheet_of(world, "bob");
    let server = sheet_of(world, "server");
    assert_eq!(alice, server, "alice and the hub hold different sheets");
    assert_eq!(bob, server, "bob and the hub hold different sheets");
    server
}

/// What the sheet should hold, read from the network and not from the copies: of
/// every write a person sent the hub, the one with the latest stamp wins its cell.
fn oracle(world: &World) -> BTreeMap<(u64, u64), Held> {
    let mut expected: BTreeMap<(u64, u64), Held> = BTreeMap::new();
    for line in world.trace() {
        if !matches!(line.fate, Fate::Sent { .. }) || line.to != "server" {
            continue;
        }
        let envelope = Envelope::decode(&mut SceCursor::new(&line.bytes)).expect("an envelope");
        let write: Value = serde_json::from_slice(envelope.data).expect("a write is JSON");
        let number = |field: &str| write[field].as_i64().expect("a number");
        let key = (number("row") as u64, number("col") as u64);
        let stamp = (
            number("wallTime") as u64,
            number("counter") as u64,
            number("nodeId") as u64,
        );
        let held = (number("value"), stamp);
        match expected.get(&key) {
            Some((_, latest)) if *latest >= stamp => {}
            _ => {
                expected.insert(key, held);
            }
        }
    }
    expected
}

#[test]
fn edits_to_different_cells_made_at_once_are_all_kept() {
    let mut world = sheet(1, 20);
    type_into(&mut world, "alice", 0, 0, 11);
    type_into(&mut world, "bob", 0, 1, 22);

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    assert_eq!(sheet.len(), 2);
    assert_eq!(sheet[&(0, 0)].0, 11);
    assert_eq!(sheet[&(0, 1)].0, 22);
    assert_eq!(sheet, oracle(&world));
    assert!(world.host_errors().is_empty());
}

#[test]
fn two_edits_to_one_cell_at_the_same_instant_leave_one_value_everywhere() {
    let mut world = sheet(1, 20);
    type_into(&mut world, "alice", 3, 3, 1);
    type_into(&mut world, "bob", 3, 3, 2);

    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    // The stamps tie on time and counter, so the node id decides: bob's.
    assert_eq!(sheet[&(3, 3)].0, 2);
    assert_eq!(sheet, oracle(&world));
}

/// A person who has seen a number and types over it means to replace it, even when
/// their own clock is behind the one that wrote it.
#[test]
fn an_edit_made_after_seeing_a_write_wins_over_it_whatever_the_clock_says() {
    let mut world = sheet(1, 20);
    // The people's clocks differ: alice's host says 1000 ms, bob's says 10.
    world.raise(
        "alice",
        "sheet.edit",
        r#"{"row":0,"col":0,"value":1,"now":1000}"#,
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    assert_eq!(
        sheet_of(&mut world, "bob")[&(0, 0)].0,
        1,
        "bob saw the write"
    );

    world.raise(
        "bob",
        "sheet.edit",
        r#"{"row":0,"col":0,"value":2,"now":10}"#,
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // Bob's clock reads 10, but he had received a write stamped 1000, so his
    // stamp sorts after it, and his number is the one every copy holds.
    let sheet = agreed(&mut world);
    assert_eq!(sheet[&(0, 0)].0, 2);
    assert_eq!(sheet, oracle(&world));
}

#[test]
fn edits_made_offline_are_merged_when_the_link_is_back() {
    let mut world = sheet(5, 20);
    world.partition("alice", "server");
    type_into(&mut world, "alice", 1, 1, 100);
    type_into(&mut world, "alice", 1, 2, 101);
    // What alice's endpoint could not send is kept until the link is back, with no
    // timer running, so there is nothing left for the world to do: it is quiet.
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    // Bob types into one of the cells alice did, later by the world's clock.
    world.advance_ms(500);
    type_into(&mut world, "bob", 1, 2, 202);
    type_into(&mut world, "bob", 2, 2, 203);
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    // While alice is cut off, bob and the hub agree and alice holds only her own.
    assert_eq!(sheet_of(&mut world, "alice").len(), 2);
    assert_eq!(sheet_of(&mut world, "server").len(), 2);

    world.heal("alice", "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let sheet = agreed(&mut world);
    assert_eq!(sheet.len(), 3);
    assert_eq!(sheet[&(1, 1)].0, 100, "alice's own cell reached the others");
    assert_eq!(sheet[&(1, 2)].0, 202, "bob typed over alice's number later");
    assert_eq!(sheet[&(2, 2)].0, 203);
    assert_eq!(sheet, oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// Many people-minutes in a few milliseconds: random edits to a small sheet at
/// random times, links cut and restored, delays spread out and envelopes delivered
/// twice, and for every seed the three copies end alike and as the oracle says.
#[test]
fn a_random_afternoon_of_edits_ends_with_one_sheet() {
    for seed in 0..40 {
        let mut rng = Rng::new(seed);
        let mut world = sheet(seed, 5);
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        let mut cut = [false, false];
        for _ in 0..60 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            type_into(
                &mut world,
                who,
                u32::try_from(rng.below(4)).unwrap(),
                u32::try_from(rng.below(4)).unwrap(),
                i32::try_from(rng.below(1_000)).unwrap(),
            );
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

        let sheet = agreed(&mut world);
        assert_eq!(sheet, oracle(&world), "seed {seed}");
        assert!(world.host_errors().is_empty(), "seed {seed}");
        assert!(!sheet.is_empty(), "seed {seed}");
    }
}
