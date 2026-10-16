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
use sce_rust_world::{Fate, Outcome, Rng, World};
use serde_json::Value;

include!(concat!(env!("OUT_DIR"), "/machines.rs"));

pub use sheet_peer::{SheetPeerPersist, SheetPeerPolicy as Peer};
pub use todo_peer::{TodoPeerPersist, TodoPeerPolicy as TodoPeer};

pub mod calendar;
pub mod crowd;
pub mod kotlin;
pub mod presence;
pub mod resync;

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

/// The sheet's world over ordered bindings, with every copy keeping a record of what
/// it was given, held and sent ([`World::record_of`]).
pub fn recorded_sheet(seed: u64, latency_ms: i64) -> World {
    sheet_with(seed, latency_ms, true, true)
}

fn sheet_over(seed: u64, latency_ms: i64, ordered: bool) -> World {
    sheet_with(seed, latency_ms, ordered, false)
}

/// The text a copy of the sheet saves of itself.
fn saved_sheet(engine: &sce_rust_runtime::Engine<Peer>) -> String {
    engine
        .save_at(0)
        .expect("a running machine saves")
        .to_json()
}

/// The text a copy of the to-do list saves of itself.
fn saved_todos(engine: &sce_rust_runtime::Engine<TodoPeer>) -> String {
    engine
        .save_at(0)
        .expect("a running machine saves")
        .to_json()
}

fn sheet_with(seed: u64, latency_ms: i64, ordered: bool, recorded: bool) -> World {
    let mut world = World::new(seed);
    for (who, router) in [
        ("alice", person("alice", ordered)),
        ("bob", person("bob", ordered)),
        ("server", hub(ordered)),
    ] {
        if recorded {
            world.add_read(who, Peer::new(), router, saved_sheet);
        } else {
            world.add(who, Peer::new(), router);
        }
    }
    world.link("alice", "server", latency_ms);
    world.link("bob", "server", latency_ms);
    world.raise("alice", "sheet.join", r#"{"node":1,"hub":false}"#);
    world.raise("bob", "sheet.join", r#"{"node":2,"hub":false}"#);
    world.raise("server", "sheet.join", r#"{"node":100,"hub":true}"#);
    world
}

/// An integer as a copy's variables and the wire carry it: a JSON number, or, for a
/// 64-bit integer, the text of its digits (docs/SCE_ACCEPTED_SUBSET.md §2.15).
pub fn whole(value: &Value) -> i128 {
    match value {
        Value::Number(n) => n
            .as_i64()
            .map(i128::from)
            .or_else(|| n.as_u64().map(i128::from))
            .unwrap_or_else(|| panic!("{n} is not a whole number")),
        Value::String(digits) => digits
            .parse()
            .unwrap_or_else(|_| panic!("{digits:?} is not the digits of an integer")),
        other => panic!("{other} is not a number"),
    }
}

/// The node id a person's stamps carry.
pub fn node_of(who: &str) -> u64 {
    match who {
        "alice" => 1,
        "bob" => 2,
        other => panic!("{other} is not a person of the sheet"),
    }
}

/// A person types `value` into the cell at (`row`, `col`), when the host's clock
/// reads the world's time.
pub fn type_into(world: &mut World, who: &str, row: u64, col: u64, value: i64) {
    let now = world.now_ms();
    type_into_at(world, who, row, col, value, now);
}

/// The same, when the host's clock reads `now` and not the world's time: a clock that
/// reads past what a double holds exactly is how a 64-bit stamp is exercised.
pub fn type_into_at(world: &mut World, who: &str, row: u64, col: u64, value: i64, now: i64) {
    // A 64-bit integer is written as the text of its digits, as it is on the wire.
    world.raise(
        who,
        "sheet.edit",
        &format!(r#"{{"row":"{row}","col":"{col}","value":"{value}","now":"{now}"}}"#),
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
        let number = |field: &str| whole(&cell[field]);
        let key = (number("row") as u64, number("col") as u64);
        let stamp = (
            number("wallTime") as u64,
            number("counter") as u64,
            number("nodeId") as u64,
        );
        assert!(
            cells.insert(key, (number("value") as i64, stamp)).is_none(),
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
        let number = |field: &str| whole(&write[field]);
        let key = (number("row") as u64, number("col") as u64);
        let stamp = (
            number("wallTime") as u64,
            number("counter") as u64,
            number("nodeId") as u64,
        );
        let held = (number("value") as i64, stamp);
        match expected.get(&key) {
            Some((_, latest)) if *latest >= stamp => {}
            _ => {
                expected.insert(key, held);
            }
        }
    }
    expected
}

// ── Formulas ────────────────────────────────────────────────────────────────

/// One side of a formula: a number, or the cell at a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Number(i64),
    Cell(u64, u64),
}

/// What a cell holds: a number, or a formula, which is an operation (`op`: the
/// constants below) over two operands. An operand an operation does not read is
/// held as the number 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
    Number(i64),
    Formula { op: u8, a: Operand, b: Operand },
}

/// The operations a formula names (sce:std/merge/lww_cell).
pub const COPY: u8 = 1;
pub const ADD: u8 = 2;
pub const SUBTRACT: u8 = 3;
pub const MULTIPLY: u8 = 4;
pub const DIVIDE: u8 = 5;
pub const SUM: u8 = 6;

/// The faults a cell shows instead of a number (sce:std/merge/sheet_settle).
pub const CYCLE: u8 = 1;
pub const DIVIDED_BY_ZERO: u8 = 2;
pub const OVERFLOWED: u8 = 3;
pub const BAD_RANGE: u8 = 4;
pub const UNKNOWN_OPERATION: u8 = 5;

/// What a cell shows: the number, and the fault (0 for none) it shows instead.
pub type Shown = (i64, u8);

/// A formula as the host's edit event writes it.
fn operand_fields(side: &str, operand: Operand) -> String {
    match operand {
        Operand::Number(number) => {
            format!(r#""{side}Ref":false,"{side}Row":"0","{side}Col":"0","{side}Val":"{number}""#)
        }
        Operand::Cell(row, col) => {
            format!(r#""{side}Ref":true,"{side}Row":"{row}","{side}Col":"{col}","{side}Val":"0""#)
        }
    }
}

/// A person types a formula into the cell at (`row`, `col`), when the host's clock
/// reads the world's time.
pub fn type_formula(
    world: &mut World,
    who: &str,
    (row, col): (u64, u64),
    op: u8,
    a: Operand,
    b: Operand,
) {
    let now = world.now_ms();
    world.raise(
        who,
        "sheet.formula",
        &format!(
            r#"{{"row":"{row}","col":"{col}","op":{op},{},{},"now":"{now}"}}"#,
            operand_fields("a", a),
            operand_fields("b", b)
        ),
    );
}

fn operand_of(cell: &Value, side: &str) -> Operand {
    let field = |name: &str| cell[format!("{side}{name}")].clone();
    if field("Ref").as_bool().expect("a flag") {
        Operand::Cell(whole(&field("Row")) as u64, whole(&field("Col")) as u64)
    } else {
        Operand::Number(whole(&field("Val")) as i64)
    }
}

/// What a cell as a copy holds it, or a write to it as it travels, holds.
fn content_of(cell: &Value) -> Content {
    let op = whole(&cell["op"]) as u8;
    if op == 0 {
        Content::Number(whole(&cell["value"]) as i64)
    } else {
        Content::Formula {
            op,
            a: operand_of(cell, "a"),
            b: operand_of(cell, "b"),
        }
    }
}

/// The cells a copy holds, by place, each with what it holds.
pub fn contents_of(world: &mut World, who: &str) -> BTreeMap<(u64, u64), Content> {
    let variables = variables_of(world, who);
    variables["cells"]
        .as_array()
        .expect("the cells are a list")
        .iter()
        .map(|cell| {
            (
                (whole(&cell["row"]) as u64, whole(&cell["col"]) as u64),
                content_of(cell),
            )
        })
        .collect()
}

/// What a copy shows, by place.
pub fn shown_of(world: &mut World, who: &str) -> BTreeMap<(u64, u64), Shown> {
    let variables = variables_of(world, who);
    let mut shown = BTreeMap::new();
    let mut previous = None;
    for record in variables["shown"].as_array().expect("shown is a list") {
        let place = (whole(&record["row"]) as u64, whole(&record["col"]) as u64);
        assert!(
            previous < Some(place),
            "{who} shows its places out of order or twice: {place:?} after {previous:?}"
        );
        previous = Some(place);
        shown.insert(
            place,
            (
                whole(&record["value"]) as i64,
                whole(&record["fault"]) as u8,
            ),
        );
    }
    shown
}

/// The three copies show one sheet; which.
pub fn agreed_shown(world: &mut World) -> BTreeMap<(u64, u64), Shown> {
    let server = shown_of(world, "server");
    for who in ["alice", "bob"] {
        assert_eq!(
            shown_of(world, who),
            server,
            "{who} and the hub show different sheets"
        );
    }
    server
}

/// What the cells should hold, read from the network: of every write a person sent
/// the hub, the one with the latest stamp wins its cell, formula and all.
pub fn oracle_contents(world: &World) -> BTreeMap<(u64, u64), Content> {
    // A cell's content and the stamp of the write that gave it.
    type Stamped = ((u64, u64, u64), Content);
    let mut latest: BTreeMap<(u64, u64), Stamped> = BTreeMap::new();
    for (event, write) in sent_to_the_hub(world) {
        if event != "sheet.cell" {
            continue;
        }
        let number = |field: &str| whole(&write[field]);
        let place = (number("row") as u64, number("col") as u64);
        let stamp = (
            number("wallTime") as u64,
            number("counter") as u64,
            number("nodeId") as u64,
        );
        match latest.get(&place) {
            Some((held, _)) if *held >= stamp => {}
            _ => {
                latest.insert(place, (stamp, content_of(&write)));
            }
        }
    }
    latest
        .into_iter()
        .map(|(place, (_, content))| (place, content))
        .collect()
}

/// What an operation gives for two numbers, in int64: the exact result, and the
/// fault when it is not one (division by zero, a result outside int64, an operation
/// there is none of). Written in i128, which cannot overflow, as a second statement
/// of sheet_apply.
fn apply(op: u8, a: i64, b: i64) -> Shown {
    let (a, b) = (i128::from(a), i128::from(b));
    let exact = match op {
        ADD => a + b,
        SUBTRACT => a - b,
        MULTIPLY => a * b,
        DIVIDE if b == 0 => return (0, DIVIDED_BY_ZERO),
        DIVIDE => a / b,
        _ => return (0, UNKNOWN_OPERATION),
    };
    i64::try_from(exact).map_or((0, OVERFLOWED), |number| (number, 0))
}

/// What the sheet shows, worked out from the cells and the orders of the rows and
/// columns by following each cell's references depth first, as a person would, and
/// not by the passes the library makes: a second statement of what a formula means
/// (sce:std/merge/sheet_settle), held to the run by the tests of this file.
/// `rows` and `cols` are the orders, removed ones included, and `gone` and
/// `gone_cols` the ids removed.
pub struct Sheet<'a> {
    pub cells: &'a BTreeMap<(u64, u64), Content>,
    pub rows: &'a [u64],
    pub gone: &'a [u64],
    pub cols: &'a [u64],
    pub gone_cols: &'a [u64],
}

impl Sheet<'_> {
    pub fn shown(&self) -> BTreeMap<(u64, u64), Shown> {
        let mut memo = BTreeMap::new();
        let mut busy = Vec::new();
        for place in self.cells.keys() {
            self.solve(*place, &mut memo, &mut busy);
        }
        // A cell that reads itself, however far round, shows a cycle.
        self.cells
            .keys()
            .map(|place| (*place, memo[place].unwrap_or((0, CYCLE))))
            .collect()
    }

    /// What the cell at `place` shows, `None` when it reads itself.
    fn solve(
        &self,
        place: (u64, u64),
        memo: &mut BTreeMap<(u64, u64), Option<Shown>>,
        busy: &mut Vec<(u64, u64)>,
    ) -> Option<Shown> {
        if let Some(known) = memo.get(&place) {
            return *known;
        }
        if busy.contains(&place) {
            return None;
        }
        busy.push(place);
        let answer = match self.cells[&place] {
            Content::Number(number) => Some((number, 0)),
            Content::Formula { op, a, b } => self.formula(op, a, b, memo, busy),
        };
        busy.pop();
        memo.insert(place, answer);
        answer
    }

    fn read(
        &self,
        operand: Operand,
        memo: &mut BTreeMap<(u64, u64), Option<Shown>>,
        busy: &mut Vec<(u64, u64)>,
    ) -> Option<Shown> {
        match operand {
            Operand::Number(number) => Some((number, 0)),
            Operand::Cell(row, col) if self.cells.contains_key(&(row, col)) => {
                self.solve((row, col), memo, busy)
            }
            // A place no cell holds reads 0.
            Operand::Cell(..) => Some((0, 0)),
        }
    }

    fn formula(
        &self,
        op: u8,
        a: Operand,
        b: Operand,
        memo: &mut BTreeMap<(u64, u64), Option<Shown>>,
        busy: &mut Vec<(u64, u64)>,
    ) -> Option<Shown> {
        match op {
            COPY => self.read(a, memo, busy),
            ADD..=DIVIDE => {
                let (left, right) = (self.read(a, memo, busy)?, self.read(b, memo, busy)?);
                Some(match (left.1, right.1) {
                    (0, 0) => apply(op, left.0, right.0),
                    (0, fault) | (fault, 0) => (0, fault),
                    (one, other) => (0, one.min(other)),
                })
            }
            SUM => self.sum(a, b, memo, busy),
            _ => Some((0, UNKNOWN_OPERATION)),
        }
    }

    /// The sum of the cells in the rectangle whose corners are the cells a and b
    /// name, in the orders of the rows and columns: those in a row and a column that
    /// are not removed, between the corners' places.
    fn sum(
        &self,
        a: Operand,
        b: Operand,
        memo: &mut BTreeMap<(u64, u64), Option<Shown>>,
        busy: &mut Vec<(u64, u64)>,
    ) -> Option<Shown> {
        let (Operand::Cell(a_row, a_col), Operand::Cell(b_row, b_col)) = (a, b) else {
            return Some((0, BAD_RANGE));
        };
        let place = |order: &[u64], id: u64| order.iter().position(|&held| held == id);
        let (Some(a_at), Some(b_at), Some(c_at), Some(d_at)) = (
            place(self.rows, a_row),
            place(self.rows, b_row),
            place(self.cols, a_col),
            place(self.cols, b_col),
        ) else {
            return Some((0, BAD_RANGE));
        };
        let (rows, cols) = (
            a_at.min(b_at)..=a_at.max(b_at),
            c_at.min(d_at)..=c_at.max(d_at),
        );
        let (mut above, mut below, mut fault) = (0_i128, 0_i128, 0_u8);
        for &(row, col) in self.cells.keys() {
            let (Some(r), Some(c)) = (place(self.rows, row), place(self.cols, col)) else {
                continue;
            };
            if self.gone.contains(&row)
                || self.gone_cols.contains(&col)
                || !rows.contains(&r)
                || !cols.contains(&c)
            {
                continue;
            }
            let (number, cell_fault) = self.solve((row, col), memo, busy)?;
            match cell_fault {
                0 if number > 0 => above += i128::from(number),
                0 => below += i128::from(number),
                other => fault = if fault == 0 { other } else { fault.min(other) },
            }
        }
        Some(if fault != 0 {
            (0, fault)
        } else if above > i128::from(i64::MAX) || below < i128::from(i64::MIN) {
            (0, OVERFLOWED)
        } else {
            (i64::try_from(above + below).expect("the signs differ"), 0)
        })
    }
}

/// The three copies show what the cells they agree on say, which the oracle works out
/// from the network and the orders the people made: the sheet's shown values; which.
pub fn expected_shown(world: &World) -> BTreeMap<(u64, u64), Shown> {
    let (rows, gone) = oracle_order(world, "sheet.row", "sheet.gone");
    let (cols, gone_cols) = oracle_order(world, "sheet.col", "sheet.gone_col");
    Sheet {
        cells: &oracle_contents(world),
        rows: &rows,
        gone: &gone,
        cols: &cols,
        gone_cols: &gone_cols,
    }
    .shown()
}

/// The id the next row a person adds will have: one more than the largest counter
/// their copy has seen, with their node below it.
pub fn next_row_key(world: &mut World, who: &str) -> u64 {
    let lamport = whole(&variables_of(world, who)["lamport"]) as u64;
    (lamport + 1) * 256 + node_of(who)
}

/// A person adds a row after the row `after` (0 for the top), and the id it gets.
/// What their copy does with it at once is run before this returns, so the next row
/// they add is issued the id after this one's and not the same.
pub fn add_row(world: &mut World, who: &str, after: u64) -> u64 {
    let key = next_row_key(world, who);
    world.raise(who, "sheet.add_row", &format!(r#"{{"after":"{after}"}}"#));
    world.advance_ms(0);
    key
}

/// A person removes the row `row`.
pub fn drop_row(world: &mut World, who: &str, row: u64) {
    world.raise(who, "sheet.drop_row", &format!(r#"{{"row":"{row}"}}"#));
}

/// A person adds a column after the column `after` (0 for the left edge), and the id
/// it gets: ids are issued from one counter, whichever list they go in.
pub fn add_col(world: &mut World, who: &str, after: u64) -> u64 {
    let key = next_row_key(world, who);
    world.raise(who, "sheet.add_col", &format!(r#"{{"after":"{after}"}}"#));
    world.advance_ms(0);
    key
}

/// A person removes the column `col`.
pub fn drop_col(world: &mut World, who: &str, col: u64) {
    world.raise(who, "sheet.drop_col", &format!(r#"{{"col":"{col}"}}"#));
}

/// Every row a copy holds in the order it holds them, removed ones included, and the
/// ids of the removed.
pub fn rows_of(world: &mut World, who: &str) -> (Vec<u64>, Vec<u64>) {
    order_in(world, who, "rows", "gone")
}

/// The same for the columns: every column a copy holds in its order, and the ids of
/// the removed.
pub fn cols_of(world: &mut World, who: &str) -> (Vec<u64>, Vec<u64>) {
    order_in(world, who, "cols", "goneCols")
}

fn order_in(world: &mut World, who: &str, order: &str, marks: &str) -> (Vec<u64>, Vec<u64>) {
    let variables = variables_of(world, who);
    let keys = |name: &str| -> Vec<u64> {
        variables[name]
            .as_array()
            .unwrap_or_else(|| panic!("{name} is a list"))
            .iter()
            .map(|node| whole(&node["key"]) as u64)
            .collect()
    };
    (keys(order), keys(marks))
}

/// The rows a copy shows: its rows in order, without the removed.
pub fn shown_by(world: &mut World, who: &str) -> Vec<u64> {
    let (rows, mut gone) = rows_of(world, who);
    gone.sort_unstable();
    rows.into_iter()
        .filter(|key| gone.binary_search(key).is_err())
        .collect()
}

/// The columns a copy shows: its columns in order, without the removed.
pub fn columns_shown_by(world: &mut World, who: &str) -> Vec<u64> {
    let (cols, mut gone) = cols_of(world, who);
    gone.sort_unstable();
    cols.into_iter()
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
            if envelope.event_type == "sheet.row" && whole(&data["key"]) == i128::from(*key) {
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

/// The rule of a replicated growable array, over a vector: an element goes directly
/// after the element it names, except that it steps over the elements already there
/// with a larger id. An element already there is not put in again. Written here
/// beside the library's algorithm so that a run is held to a second statement of it.
pub fn place(order: &mut Vec<u64>, key: u64, after: u64) {
    if order.contains(&key) {
        return;
    }
    let mut at = if after == 0 {
        0
    } else {
        order
            .iter()
            .position(|&held| held == after)
            .unwrap_or_else(|| panic!("{after} is not there to be inserted after"))
            + 1
    };
    while at < order.len() && order[at] > key {
        at += 1;
    }
    order.insert(at, key);
}

/// The order the people's inserts make by that rule, and the ids they removed, read
/// from the network and not from any copy: `insert` names the event that puts an
/// element in and `remove` the one that takes it out.
pub fn oracle_order(world: &World, insert: &str, remove: &str) -> (Vec<u64>, Vec<u64>) {
    let mut order = Vec::new();
    let mut gone = Vec::new();
    for (event, data) in sent_to_the_hub(world) {
        let number = |field: &str| whole(&data[field]) as u64;
        if event == insert {
            place(&mut order, number("key"), number("after"));
        } else if event == remove && !gone.contains(&number("key")) {
            gone.push(number("key"));
        }
    }
    gone.sort_unstable();
    (order, gone)
}

/// The three copies hold one order of rows and one set of removed rows; which.
pub fn agreed_rows(world: &mut World) -> (Vec<u64>, Vec<u64>) {
    agreed_order(world, rows_of, "row")
}

/// The three copies hold one order of columns and one set of removed columns; which.
pub fn agreed_cols(world: &mut World) -> (Vec<u64>, Vec<u64>) {
    agreed_order(world, cols_of, "column")
}

/// How one copy's order is read: every element in the order it holds them, and the
/// ids of the removed.
type OrderOf = fn(&mut World, &str) -> (Vec<u64>, Vec<u64>);

fn agreed_order(world: &mut World, order_of: OrderOf, what: &str) -> (Vec<u64>, Vec<u64>) {
    let (order, mut gone) = order_of(world, "server");
    gone.sort_unstable();
    for who in ["alice", "bob"] {
        let (theirs, mut their_gone) = order_of(world, who);
        their_gone.sort_unstable();
        assert_eq!(
            theirs, order,
            "{who} and the hub hold different {what} orders"
        );
        assert_eq!(
            their_gone, gone,
            "{who} and the hub removed different {what}s"
        );
    }
    (order, gone)
}

// ── The shared to-do list (the fixture `todo_peer.scxml`) ───────────────────

/// The world of the to-do list: alice (node 1) and bob (node 2), each `latency_ms` from
/// the hub, over links that may reorder what they carry. Writes to a field do not depend
/// on the ones before it, so no binding orders.
pub fn todo_world(seed: u64, latency_ms: i64) -> World {
    todo_world_with(seed, latency_ms, false)
}

/// The world of the to-do list, with every copy keeping a record of what it was
/// given, held and sent ([`World::record_of`]).
pub fn recorded_todo_world(seed: u64, latency_ms: i64) -> World {
    todo_world_with(seed, latency_ms, true)
}

fn todo_world_with(seed: u64, latency_ms: i64, recorded: bool) -> World {
    todo_world_of(seed, latency_ms, |world, who, router| {
        if recorded {
            world.add_read(who, TodoPeer::new(), router, saved_todos);
        } else {
            world.add(who, TodoPeer::new(), router);
        }
    })
}

/// The world of the to-do list with each of its three actors added by `add`, which is
/// given the world, the actor's name and its router: what differs between a world of
/// Rust copies and one with a copy that is another process's is only how a copy is added.
pub fn todo_world_of(seed: u64, latency_ms: i64, add: impl Fn(&mut World, &str, Router)) -> World {
    let mut world = World::new(seed);
    for (who, router) in [
        ("alice", person("alice", false)),
        ("bob", person("bob", false)),
        ("server", hub(false)),
    ] {
        add(&mut world, who, router);
    }
    world.link("alice", "server", latency_ms);
    world.link("bob", "server", latency_ms);
    world.raise("alice", "todo.join", r#"{"node":"1","hub":false}"#);
    world.raise("bob", "todo.join", r#"{"node":"2","hub":false}"#);
    world.raise("server", "todo.join", r#"{"node":"100","hub":true}"#);
    world
}

/// The fields of a to-do, by the numbers the host gives them.
pub const TITLE: u64 = 1;
pub const DONE: u64 = 2;
pub const DUE: u64 = 3;
pub const REMOVED: u64 = 9;

/// Quote `text` as a JSON string, escaping what JSON requires.
fn json_text(text: &str) -> String {
    serde_json::to_string(text).expect("a string is JSON")
}

/// A person changes the number of a field of a to-do, when the host's clock reads the
/// world's time.
pub fn todo_set(world: &mut World, who: &str, id: u64, field: u64, value: i64) {
    let now = world.now_ms();
    todo_edit_at(world, who, id, field, value, "", now);
}

/// A person gives a to-do a title.
pub fn todo_title(world: &mut World, who: &str, id: u64, title: &str) {
    let now = world.now_ms();
    todo_edit_at(world, who, id, TITLE, 0, title, now);
}

/// The general form: the host's clock reads `now`.
pub fn todo_edit_at(
    world: &mut World,
    who: &str,
    id: u64,
    field: u64,
    value: i64,
    text: &str,
    now: i64,
) {
    world.raise(
        who,
        "todo.edit",
        &format!(
            r#"{{"id":{id},"field":{field},"value":"{value}","text":{},"now":"{now}"}}"#,
            json_text(text)
        ),
    );
}

/// What a copy saved of itself, for the to-do list.
///
/// A copy that is another process's saves through its process; the text is the same one,
/// which every backend saves byte for byte.
pub fn todo_variables_of(world: &mut World, who: &str) -> Value {
    let saved = match world.external_saved(who) {
        Some(saved) => saved,
        None => world
            .actor::<TodoPeer>(who)
            .engine()
            .save_at(0)
            .expect("a running machine saves")
            .to_json(),
    };
    let saved: Value = serde_json::from_str(&saved).expect("a saved state is JSON");
    saved["variables"].clone()
}

/// One value a field of a to-do holds: its number, its text when it is the title, and
/// the stamp of the write that gave it, which is the dot it is known by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoValue {
    pub value: i64,
    pub text: Option<String>,
    pub stamp: (u64, u64, u64),
}

/// One field of one to-do as a copy holds it. A field holds a value for every write
/// nobody has written over, so `live` is more than one value when two people changed
/// it at the same time; `value`, `text` and `stamp` are those of the latest of them,
/// which is the one a reader that needs a single value takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub value: i64,
    pub text: Option<String>,
    pub stamp: (u64, u64, u64),
    pub live: Vec<TodoValue>,
}

impl Field {
    /// A field from the values it holds, latest stamp last once they are sorted.
    fn of(mut live: Vec<TodoValue>) -> Self {
        live.sort_by_key(|held| held.stamp);
        let latest = live.last().expect("a field holds a value").clone();
        Field {
            value: latest.value,
            text: latest.text,
            stamp: latest.stamp,
            live,
        }
    }
}

/// The fields a copy holds, by (to-do, field).
pub fn todos_of(world: &mut World, who: &str) -> BTreeMap<(u64, u64), Field> {
    let variables = todo_variables_of(world, who);
    let mut held: BTreeMap<(u64, u64), Vec<TodoValue>> = BTreeMap::new();
    for value in variables["values"].as_array().expect("values are a list") {
        let number = |name: &str| whole(&value[name]);
        let key = (number("id") as u64, number("field") as u64);
        let one = TodoValue {
            value: number("value") as i64,
            text: (key.1 == TITLE).then(|| value["text"].as_str().expect("a text").to_string()),
            stamp: (
                number("wallTime") as u64,
                number("counter") as u64,
                number("nodeId") as u64,
            ),
        };
        let held = held.entry(key).or_default();
        assert!(
            held.iter().all(|other| other.stamp != one.stamp),
            "{who} holds the write {:?} of {key:?} twice",
            one.stamp
        );
        held.push(one);
    }
    held.into_iter()
        .map(|(key, live)| (key, Field::of(live)))
        .collect()
}

/// The three copies hold one list of to-dos; which.
pub fn agreed_todos(world: &mut World) -> BTreeMap<(u64, u64), Field> {
    let server = todos_of(world, "server");
    for who in ["alice", "bob"] {
        assert_eq!(
            todos_of(world, who),
            server,
            "{who} and the hub hold different to-do lists"
        );
    }
    server
}

/// A write someone wrote over: the field it was written to, and its stamp.
type Superseded = ((u64, u64), (u64, u64, u64));

/// What the list should hold, read from the network and not from the copies: of every
/// write a person sent the hub, the ones no person sent a supersede for are the values
/// of its field, and the latest of them is the one a single reader takes.
pub fn todo_oracle(world: &World) -> BTreeMap<(u64, u64), Field> {
    let mut written: BTreeMap<(u64, u64), Vec<TodoValue>> = BTreeMap::new();
    let mut superseded: Vec<Superseded> = Vec::new();
    for (event, write) in sent_to_the_hub(world) {
        // The digest's nodes a resync walks are not changes.
        if event != "todo.write" && event != "todo.supersede" {
            continue;
        }
        let number = |name: &str| whole(&write[name]);
        let key = (number("id") as u64, number("field") as u64);
        let stamp = (
            number("wallTime") as u64,
            number("counter") as u64,
            number("nodeId") as u64,
        );
        match event.as_str() {
            "todo.write" => {
                let one = TodoValue {
                    value: number("value") as i64,
                    text: (key.1 == TITLE)
                        .then(|| write["text"].as_str().expect("a text").to_string()),
                    stamp,
                };
                let held = written.entry(key).or_default();
                if held.iter().all(|other| other.stamp != stamp) {
                    held.push(one);
                }
            }
            "todo.supersede" => superseded.push((key, stamp)),
            _ => {}
        }
    }
    written
        .into_iter()
        .filter_map(|(key, held)| {
            let live: Vec<TodoValue> = held
                .into_iter()
                .filter(|one| !superseded.contains(&(key, one.stamp)))
                .collect();
            (!live.is_empty()).then(|| (key, Field::of(live)))
        })
        .collect()
}

/// An afternoon of changes to a few to-dos at random times, links cut and restored,
/// delays spread out and envelopes delivered twice, run in `world` (a to-do world of
/// latency 5, [`todo_world`]) from `seed`: at the end the three copies hold one list and
/// it is the list the oracle says. With `restarts`, copies are now and then killed and
/// started again by `restart`, which is given the world and the copy's name.
pub fn todo_afternoon(
    world: &mut World,
    seed: u64,
    restarts: bool,
    mut restart: impl FnMut(&mut World, &str),
) {
    const TITLES: [&str; 5] = ["milk", "post office", "\u{d560}\u{c77c}", "tax", "gym at 7"];
    let mut rng = Rng::new(seed);
    // The copies take the join the host gave them before anything can kill one: a
    // process killed before it has read its first event comes back without it.
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet, "seed {seed}");
    world.jitter("alice", "server", 40);
    world.jitter("bob", "server", 40);
    world.reorder("alice", "server");
    world.reorder("bob", "server");
    world.degrade("alice", "server", 0, 20);
    world.degrade("bob", "server", 0, 20);
    let mut cut = [false, false];
    for _ in 0..60 {
        let who = if rng.below(2) == 0 { "alice" } else { "bob" };
        let id = 1 + rng.below(4);
        match rng.below(4) {
            0 => {
                let title = TITLES[usize::try_from(rng.below(5)).unwrap()];
                todo_title(world, who, id, title);
            }
            1 => todo_set(world, who, id, DONE, i64::try_from(rng.below(2)).unwrap()),
            2 => todo_set(world, who, id, DUE, i64::try_from(rng.below(365)).unwrap()),
            _ => todo_set(
                world,
                who,
                id,
                REMOVED,
                i64::try_from(rng.below(2)).unwrap(),
            ),
        }
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
        // Now and then a copy is killed and comes back from its saved state.
        if restarts && rng.below(10) == 0 {
            let who = ["alice", "bob", "server"][usize::try_from(rng.below(3)).unwrap()];
            restart(world, who);
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

    // A copy that refused a write is the first thing to say, before the copies are
    // compared: a list that differs because a block failed is not a merge fault.
    for who in ["alice", "bob", "server"] {
        assert_eq!(
            todo_variables_of(world, who)["refusals"],
            0,
            "seed {seed}: {who} refused a write"
        );
    }
    for who in ["alice", "bob"] {
        assert_eq!(
            todos_of(world, who),
            todos_of(world, "server"),
            "seed {seed}: {who} and the hub hold different to-do lists"
        );
    }
    let list = agreed_todos(world);
    assert_eq!(list, todo_oracle(world), "seed {seed}");
    assert!(world.host_errors().is_empty(), "seed {seed}");
    assert!(!list.is_empty(), "seed {seed}");
}
