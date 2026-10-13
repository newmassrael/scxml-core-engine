// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the tests of a shared sheet have in common: the machines, the world they run
// in, and the ways of reading a copy of the sheet back. A test file declares
// `mod common;` and takes what it needs.

#![allow(dead_code)]

use std::collections::BTreeMap;

use sce_forge_runtime::codec::SceCursor;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::outbound::{OutboundBuffer, RetryPolicy};
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_world::{Fate, World};
use serde_json::Value;

include!(concat!(env!("OUT_DIR"), "/machines.rs"));

pub use sheet_peer::{SheetPeerPersist, SheetPeerPolicy as Peer};

/// How the binding to a peer is configured: envelopes delivered once, with those
/// written while the link is down kept until it is back. `ordered` asks the Mesh
/// core to number what it sends and to release what it receives in that order
/// (SCE_MESH.md, ordering), which a transport that does not order needs for a
/// document whose writes depend on the ones before them.
pub fn peer(responders: &'static [&'static str], ordered: bool) -> PeerConfig {
    PeerConfig {
        transport: "wss",
        buffer: Some(OutboundBuffer {
            max_pending: 1_024,
            max_age_ms: 0,
        }),
        retry: Some(RetryPolicy {
            max_retries: 50,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1_000,
            jitter_pct: 0,
        }),
        stamp_sequence: ordered,
        delivery: Delivery {
            dedup: true,
            ordered,
        },
        responders,
        deadline_ms: None,
        reply_events: &[],
    }
}

/// The longest an out-of-order envelope waits for the one before it, when the
/// binding orders: longer than any delay a test gives a link.
const GAP_TIMEOUT_MS: i64 = 5_000;

pub fn person(name: &str, ordered: bool) -> Router {
    let mut router = Router::new(name, 256, GAP_TIMEOUT_MS).expect("a window the core can hold");
    router.add_peer("server", peer(&["server"], ordered));
    router
}

pub fn hub(ordered: bool) -> Router {
    let mut router =
        Router::new("server", 256, GAP_TIMEOUT_MS).expect("a window the core can hold");
    router.add_peer("alice", peer(&["alice"], ordered));
    router.add_peer("bob", peer(&["bob"], ordered));
    router
}

/// The sheet's world: alice (node 1) and bob (node 2), each `latency_ms` from the
/// hub, over links that may reorder what they carry.
pub fn sheet(seed: u64, latency_ms: i64) -> World {
    sheet_over(seed, latency_ms, false)
}

/// The sheet's world over bindings that order what they carry, for the parts of a
/// sheet whose writes depend on the ones before them: a row is inserted after a row
/// that has to be there already.
pub fn ordered_sheet(seed: u64, latency_ms: i64) -> World {
    sheet_over(seed, latency_ms, true)
}

fn sheet_over(seed: u64, latency_ms: i64, ordered: bool) -> World {
    let mut world = World::new(seed);
    world.add("alice", Peer::new(), person("alice", ordered));
    world.add("bob", Peer::new(), person("bob", ordered));
    world.add("server", Peer::new(), hub(ordered));
    world.link("alice", "server", latency_ms);
    world.link("bob", "server", latency_ms);
    world.raise("alice", "sheet.join", r#"{"node":1,"hub":false}"#);
    world.raise("bob", "sheet.join", r#"{"node":2,"hub":false}"#);
    world.raise("server", "sheet.join", r#"{"node":100,"hub":true}"#);
    world
}

/// The node id a person's stamps carry.
pub fn node_of(who: &str) -> u32 {
    match who {
        "alice" => 1,
        "bob" => 2,
        other => panic!("{other} is not a person of the sheet"),
    }
}

/// A person types `value` into the cell at (`row`, `col`), when the host's clock
/// reads the world's time.
pub fn type_into(world: &mut World, who: &str, row: u32, col: u32, value: i32) {
    let now = world.now_ms();
    world.raise(
        who,
        "sheet.edit",
        &format!(r#"{{"row":{row},"col":{col},"value":{value},"now":{now}}}"#),
    );
}

/// What a copy saved of itself: its variables, by name.
pub fn variables_of(world: &mut World, who: &str) -> Value {
    let saved = world
        .actor::<Peer>(who)
        .engine()
        .save_at(0)
        .expect("a running machine saves");
    let saved: Value = serde_json::from_str(&saved.to_json()).expect("a saved state is JSON");
    saved["variables"].clone()
}

/// One cell as a copy holds it: the number, and the stamp that put it there.
pub type Held = (i64, (u64, u64, u64));

/// The cells a copy holds, by place.
pub fn sheet_of(world: &mut World, who: &str) -> BTreeMap<(u64, u64), Held> {
    let variables = variables_of(world, who);
    let mut cells = BTreeMap::new();
    for cell in variables["cells"].as_array().expect("the cells are a list") {
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

/// The three copies hold one set of cells; which.
pub fn agreed(world: &mut World) -> BTreeMap<(u64, u64), Held> {
    let alice = sheet_of(world, "alice");
    let bob = sheet_of(world, "bob");
    let server = sheet_of(world, "server");
    assert_eq!(alice, server, "alice and the hub hold different sheets");
    assert_eq!(bob, server, "bob and the hub hold different sheets");
    server
}

/// What a person sent the hub, in the order they sent it: the name of the event and
/// the data it carried, read off the envelopes the network recorded rather than off
/// any copy.
pub fn sent_to_the_hub(world: &World) -> Vec<(String, Value)> {
    let mut sent = Vec::new();
    for line in world.trace() {
        if !matches!(line.fate, Fate::Sent { .. }) || line.to != "server" {
            continue;
        }
        let envelope = Envelope::decode(&mut SceCursor::new(&line.bytes)).expect("an envelope");
        let data: Value = serde_json::from_slice(envelope.data).expect("an event's data is JSON");
        sent.push((envelope.event_type.to_string(), data));
    }
    sent
}

/// What the cells should hold, read from the network and not from the copies: of
/// every write a person sent the hub, the one with the latest stamp wins its cell.
pub fn oracle(world: &World) -> BTreeMap<(u64, u64), Held> {
    let mut expected: BTreeMap<(u64, u64), Held> = BTreeMap::new();
    for (event, write) in sent_to_the_hub(world) {
        if event != "sheet.cell" {
            continue;
        }
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

/// The id the next row a person adds will have: one more than the largest counter
/// their copy has seen, with their node below it.
pub fn next_row_key(world: &mut World, who: &str) -> u32 {
    let lamport = variables_of(world, who)["lamport"]
        .as_u64()
        .expect("the counter is a number") as u32;
    (lamport + 1) * 256 + node_of(who)
}

/// A person adds a row after the row `after` (0 for the top), and the id it gets.
/// What their copy does with it at once is run before this returns, so the next row
/// they add is issued the id after this one's and not the same.
pub fn add_row(world: &mut World, who: &str, after: u32) -> u32 {
    let key = next_row_key(world, who);
    world.raise(who, "sheet.add_row", &format!(r#"{{"after":{after}}}"#));
    world.advance_ms(0);
    key
}

/// A person removes the row `row`.
pub fn drop_row(world: &mut World, who: &str, row: u32) {
    world.raise(who, "sheet.drop_row", &format!(r#"{{"row":{row}}}"#));
}

/// Every row a copy holds in the order it holds them, removed ones included, and the
/// ids of the removed.
pub fn rows_of(world: &mut World, who: &str) -> (Vec<u32>, Vec<u32>) {
    let variables = variables_of(world, who);
    let keys = |name: &str| -> Vec<u32> {
        variables[name]
            .as_array()
            .unwrap_or_else(|| panic!("{name} is a list"))
            .iter()
            .map(|row| row["key"].as_u64().expect("a key") as u32)
            .collect()
    };
    (keys("rows"), keys("gone"))
}

/// The rows a copy shows: its rows in order, without the removed.
pub fn shown_by(world: &mut World, who: &str) -> Vec<u32> {
    let (rows, mut gone) = rows_of(world, who);
    gone.sort_unstable();
    rows.into_iter()
        .filter(|key| gone.binary_search(key).is_err())
        .collect()
}

/// What a failed run was doing, in a line: how many envelopes met each fate on the
/// network, and what each copy counted of its own refusals and of rows it could not
/// place. A run that diverges is read from this before it is read from anything else.
pub fn diagnosis(world: &mut World) -> String {
    let (mut sent, mut delivered, mut cut, mut lost, mut duplicated) = (0, 0, 0, 0, 0);
    for line in world.trace() {
        match line.fate {
            Fate::Sent { .. } => sent += 1,
            Fate::Delivered => delivered += 1,
            Fate::Cut => cut += 1,
            Fate::Lost => lost += 1,
            Fate::Duplicated { .. } => duplicated += 1,
        }
    }
    // Whether an envelope that was cut in flight ever arrived: the same bytes, from
    // the same sender to the same receiver, delivered after the cut.
    let trace = world.trace();
    let never_arrived = trace
        .iter()
        .enumerate()
        .filter(|(_, line)| line.fate == Fate::Cut)
        .filter(|(at, cut)| {
            !trace[*at..].iter().any(|later| {
                later.fate == Fate::Delivered
                    && later.from == cut.from
                    && later.to == cut.to
                    && later.bytes == cut.bytes
            })
        })
        .count();
    let mut copies = String::new();
    for who in ["alice", "bob", "server"] {
        let variables = variables_of(world, who);
        copies.push_str(&format!(
            " {who}(refusals={} orphans={} rows={})",
            variables["refusals"],
            variables["orphans"],
            variables["rows"].as_array().map_or(0, Vec::len)
        ));
    }
    // The rows one copy holds and another lacks, and what became of the envelopes
    // that carried them.
    let held_by = |world: &mut World, who: &str| rows_of(world, who).0;
    let (theirs, ours) = (held_by(world, "bob"), held_by(world, "server"));
    let mut missing = String::new();
    for key in theirs.iter().filter(|key| !ours.contains(key)) {
        missing.push_str(&format!("\n  row {key} (node {}):", key % 256));
        for line in &trace {
            let Ok(envelope) = Envelope::decode(&mut SceCursor::new(&line.bytes)) else {
                continue;
            };
            let Ok(data) = serde_json::from_slice::<Value>(envelope.data) else {
                continue;
            };
            if envelope.event_type == "sheet.row" && data["key"] == *key {
                missing.push_str(&format!(
                    " [t={} {}->{} seq={:?} {:?}]",
                    line.at_ms, line.from, line.to, envelope.sequence_no, line.fate
                ));
            }
        }
    }
    format!(
        "sent={sent} delivered={delivered} cut={cut} (never arrived: {never_arrived}) lost={lost} duplicated={duplicated};{copies}; host errors: {:?}; held by bob and not by the hub:{missing}",
        world.host_errors()
    )
}

/// The three copies hold one order of rows and one set of removed rows; which.
pub fn agreed_rows(world: &mut World) -> (Vec<u32>, Vec<u32>) {
    let (rows, mut gone) = rows_of(world, "server");
    gone.sort_unstable();
    for who in ["alice", "bob"] {
        let (theirs, mut their_gone) = rows_of(world, who);
        their_gone.sort_unstable();
        assert_eq!(theirs, rows, "{who} and the hub hold different row orders");
        assert_eq!(their_gone, gone, "{who} and the hub removed different rows");
    }
    (rows, gone)
}
