// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of this world, written out as the scenarios every backend replays.
//
// The world runs the Rust machines and nothing else, so on its own it says that three
// Rust copies of a shared document agree. Whether the Kotlin, Python, Go, C++ and C
// machines and the Interpreter would have done what the Rust ones did is asked here:
// each copy keeps a record of every event it was given, by its host or by a peer, in
// the order it took them, with what it held and what it sent its peers after each
// (`World::record_of`), and that record is a scenario of
// `sce-build/tests/fixtures/static_datamodel/scenarios/` (docs/SCE_ACCEPTED_SUBSET.md
// §2.15) that the seven drivers replay against their own machine of the same
// document. A machine is a function of the events it is given, so an engine that
// holds and sends at every step what the Rust one did would have stood in for it in
// the run.
//
// The scenarios are committed, and this test holds them to the run: a change to a
// machine, a standard algorithm or the run itself that moves a step fails here until
// the files are written again (`SCE_WORLD_SCENARIOS=write`), and the diff of the
// files is then what every other backend has to follow. What the copies hold is
// observed from the Rust engine, not derived from the documents by hand; what makes it
// the right answer is checked in the same test, by the oracles the other tests of
// this world use, which read the network and not the copies.

mod common;

use std::path::PathBuf;

use common::*;
use sce_rust_world::{Outcome, Taken, World};
use serde_json::{json, Map, Value};

/// The variables a copy of the to-do list publishes (`sce:direction="out"`).
const TODO_PUBLISHED: &[&str] = &[
    "node",
    "hub",
    "clock",
    "adds",
    "tombstones",
    "values",
    "digest",
    "refusals",
];

/// The variables a copy of the presence list publishes.
const PRESENCE_PUBLISHED: &[&str] = &[
    "node", "hub", "ttl", "lease", "online", "serial", "lost", "refusals",
];

/// The variables a copy of the sheet publishes.
const SHEET_PUBLISHED: &[&str] = &[
    "node", "hub", "clock", "cells", "rows", "gone", "cols", "goneCols", "shown", "lamport",
    "orphans", "refusals",
];

fn scenario_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sce-build/tests/fixtures/static_datamodel/scenarios")
        .join(format!("{name}.json"))
}

/// JSON text as a value; nothing for the empty text an event without data carries.
fn data_of(text: &str) -> Option<Value> {
    (!text.is_empty()).then(|| serde_json::from_str(text).expect("an event's data is JSON"))
}

/// One step of a record as a scenario states it.
fn step_of(taken: &Taken, published: &[&str]) -> Value {
    let saved: Value = serde_json::from_str(&taken.saved).expect("a saved state is JSON");
    let mut variables = Map::new();
    for name in published {
        let held = saved["variables"]
            .get(*name)
            .unwrap_or_else(|| panic!("the machine saves no variable `{name}`"));
        variables.insert((*name).to_string(), held.clone());
    }
    let sent: Vec<Value> = taken
        .sent
        .iter()
        .map(|sent| {
            let mut entry = Map::new();
            entry.insert("target".into(), json!(sent.target));
            entry.insert("event".into(), json!(sent.event));
            if let Some(data) = data_of(&sent.data) {
                entry.insert("data".into(), data);
            }
            Value::Object(entry)
        })
        .collect();
    let mut step = Map::new();
    if let Some((event, data)) = &taken.event {
        step.insert("event".into(), json!(event));
        if let Some(data) = data_of(data) {
            step.insert("data".into(), data);
        }
        if taken.dropped {
            step.insert("dropped".into(), json!(true));
        }
    }
    if taken.advance_ms > 0 {
        step.insert("advance_ms".into(), json!(taken.advance_ms));
    }
    step.insert(
        "expect".into(),
        json!({ "state": saved["current"], "variables": variables, "sent": sent }),
    );
    Value::Object(step)
}

/// A record as the text of a scenario: a step to a line, so a step that moved is a
/// line of the diff.
fn scenario_of(machine: &str, about: &str, record: &[Taken], published: &[&str]) -> String {
    let mut text = String::from("{\n");
    text.push_str(&format!("  \"machine\": {},\n", json!(machine)));
    text.push_str(&format!("  \"about\": {},\n", json!(about)));
    text.push_str("  \"steps\": [\n");
    for (n, taken) in record.iter().enumerate() {
        text.push_str("    ");
        text.push_str(&step_of(taken, published).to_string());
        text.push_str(if n + 1 < record.len() { ",\n" } else { "\n" });
    }
    text.push_str("  ]\n}\n");
    text
}

/// Hold the committed scenario `name` to `text`, or write it when the run is asked to.
fn hold(name: &str, text: &str) {
    let path = scenario_path(name);
    if std::env::var("SCE_WORLD_SCENARIOS").as_deref() == Ok("write") {
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}; write the scenarios with SCE_WORLD_SCENARIOS=write",
            path.display()
        )
    });
    assert!(
        committed == text,
        "{} is not what the run does now: write the scenarios again with SCE_WORLD_SCENARIOS=write, and read the diff as what every backend now has to do",
        path.display()
    );
}

fn about(what: &str, who: &str) -> String {
    format!(
        "{what}, as {who} took part in it: every event {who} was given, by its host or by a peer, in the order it took them, with what it held and what it sent its peers after each. Written by backends/rust/world/tests/a_world_run_is_what_every_backend_replays.rs from a run of the Rust machines (SCE_WORLD_SCENARIOS=write) and not by hand: the expected values are what the Rust engine did, which that test holds to oracles that read the network, and every other backend is held to them here."
    )
}

/// An afternoon of the shared to-do list: changes to different fields of one to-do at
/// once, two titles written at the same instant, a title past what a title holds,
/// changes made while a link is down, and a to-do removed by one while the other
/// renames it; then changes a link dropped for good, found again by the copies telling
/// each other their link is back (`todo.resync`).
fn todo_afternoon() -> World {
    let mut world = recorded_todo_world(11, 20);
    todo_title(&mut world, "alice", 7, "buy milk");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    todo_title(&mut world, "alice", 7, "buy oat milk");
    todo_set(&mut world, "bob", 7, DONE, 1);
    todo_set(&mut world, "bob", 7, DUE, 20_000);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    todo_title(&mut world, "alice", 3, "call dentist");
    todo_title(&mut world, "bob", 3, "call the dentist");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // 33 bytes: one past what a title holds. Refused whole, and nothing sent.
    todo_title(&mut world, "bob", 4, "a title that is one byte too long");
    // Text that is more than one byte to a character.
    todo_title(&mut world, "alice", 5, "caf\u{e9} at 3");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    world.partition("alice", "server");
    world.advance_ms(50);
    todo_set(&mut world, "alice", 5, DUE, 20_001);
    todo_set(&mut world, "bob", 5, REMOVED, 1);
    world.advance_ms(300);
    todo_title(&mut world, "alice", 5, "tea at 4");
    world.advance_ms(300);
    world.heal("alice", "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    // Two minutes later alice's link drops what it carries without a word, in both
    // directions, so the hub and bob never get what alice writes and alice never gets
    // what bob writes: changes lost for good, which no retry repairs.
    world.advance_ms(120_000);
    world.degrade("alice", "server", 100, 0);
    todo_set(&mut world, "alice", 7, DUE, 20_002);
    todo_title(&mut world, "alice", 3, "call Dr Han");
    world.advance_ms(60_000);
    todo_set(&mut world, "bob", 5, DONE, 1);
    todo_title(&mut world, "bob", 3, "call the clinic");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    world.degrade("alice", "server", 0, 0);

    // The link is back: each copy tells its peers, in turn, to look for where they differ.
    for who in ["alice", "server", "bob"] {
        world.raise(who, "todo.resync", "");
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    }
    world
}

#[test]
fn an_afternoon_of_the_shared_todo_list_is_three_scenarios() {
    let mut world = todo_afternoon();
    let list = agreed_todos(&mut world);
    assert_eq!(list, todo_oracle(&world));
    assert_eq!(list[&(7, TITLE)].text.as_deref(), Some("buy oat milk"));
    // Retitled by both while alice's link dropped everything: neither saw the other's, so
    // both titles stay, and the later stamp is the one a single reader takes.
    assert_eq!(list[&(3, TITLE)].live.len(), 2);
    assert_eq!(list[&(3, TITLE)].text.as_deref(), Some("call the clinic"));
    assert_eq!(list[&(7, DUE)].value, 20_002);
    assert_eq!(list[&(5, DONE)].value, 1);
    assert_eq!(list[&(5, TITLE)].text.as_deref(), Some("tea at 4"));
    assert_eq!(list[&(5, REMOVED)].value, 1);
    assert!(
        !list.contains_key(&(4, TITLE)),
        "the long title was refused"
    );
    assert_eq!(todo_variables_of(&mut world, "bob")["refusals"], json!(1));
    assert!(world.host_errors().is_empty());

    for who in ["alice", "bob", "server"] {
        let record = world.record_of::<TodoPeer>(who);
        assert!(
            record.len() > 5,
            "{who} took part in the afternoon: {} steps",
            record.len()
        );
        hold(
            &format!("todo_peer_{who}"),
            &scenario_of(
                "todo_peer",
                &about("An afternoon of a shared to-do list", who),
                &record,
                TODO_PUBLISHED,
            ),
        );
    }
}

/// A morning of the shared sheet: rows and columns added by both people at once, cells
/// typed into, the same cell typed into by both, a row removed while the other types
/// into it, and edits made while a link is down.
fn sheet_morning() -> World {
    let mut world = recorded_sheet(5, 20);
    let first = add_row(&mut world, "alice", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // Two rows after the same row at once, and a column from each.
    let second = add_row(&mut world, "alice", first);
    let third = add_row(&mut world, "bob", first);
    let left = add_col(&mut world, "alice", 0);
    let right = add_col(&mut world, "bob", 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    type_into(&mut world, "alice", first, left, 10);
    type_into(&mut world, "bob", first, right, -4);
    // The same cell, at the same instant.
    type_into(&mut world, "alice", second, left, 1);
    type_into(&mut world, "bob", second, left, 2);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // One removes a row while the other types into it, and removes it again.
    drop_row(&mut world, "alice", third);
    type_into(&mut world, "bob", third, right, 99);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    drop_row(&mut world, "bob", third);
    drop_col(&mut world, "bob", right);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    world.partition("bob", "server");
    world.advance_ms(50);
    let offline = add_row(&mut world, "bob", second);
    type_into(&mut world, "bob", offline, left, 7);
    let typed_at = world.now_ms();
    type_into(&mut world, "alice", first, left, 11);
    world.advance_ms(400);
    world.heal("bob", "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    // A hub that reaches many copies by one topic sends a write back to its writer.
    // Alice is given the write she typed above once more: she holds it already, and
    // sends nothing, for only an edit sends a write to the hub.
    world.raise(
        "alice",
        "sheet.cell",
        &format!(
            r#"{{"row":"{first}","col":"{left}","value":"11","wallTime":"{typed_at}","counter":0,"nodeId":"1","op":0,"aRef":false,"aRow":"0","aCol":"0","aVal":"0","bRef":false,"bRow":"0","bCol":"0","bVal":"0"}}"#
        ),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // Formulas over what is there: a sum down the first column from the first row to
    // the one bob added offline (the removed row between is not summed), a chain
    // that reads the sum, and then two formulas written while apart that read each
    // other, which every copy shows as a cycle.
    let total = add_col(&mut world, "alice", left);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    type_formula(
        &mut world,
        "alice",
        (first, total),
        SUM,
        Operand::Cell(first, left),
        Operand::Cell(offline, left),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    type_formula(
        &mut world,
        "bob",
        (second, total),
        MULTIPLY,
        Operand::Cell(first, total),
        Operand::Number(2),
    );
    type_formula(
        &mut world,
        "bob",
        (offline, total),
        DIVIDE,
        Operand::Number(100),
        Operand::Cell(offline, right),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.partition("bob", "server");
    world.advance_ms(50);
    type_formula(
        &mut world,
        "alice",
        (third, total),
        COPY,
        Operand::Cell(second, total),
        Operand::Number(0),
    );
    type_formula(
        &mut world,
        "bob",
        (second, total),
        COPY,
        Operand::Cell(third, total),
        Operand::Number(0),
    );
    world.advance_ms(400);
    world.heal("bob", "server");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    world
}

#[test]
fn a_morning_of_the_shared_sheet_is_three_scenarios() {
    let mut world = sheet_morning();
    assert_eq!(agreed(&mut world), oracle(&world));
    let shown = agreed_shown(&mut world);
    assert_eq!(shown, expected_shown(&world));
    assert!(
        shown.values().any(|&(_, fault)| fault == CYCLE),
        "the two formulas that read each other are a cycle"
    );
    assert!(
        shown.values().any(|&(_, fault)| fault == DIVIDED_BY_ZERO),
        "a cell that was never typed into reads 0"
    );
    assert!(shown
        .values()
        .any(|&(number, fault)| fault == 0 && number == 20));
    assert_eq!(
        agreed_rows(&mut world),
        oracle_order(&world, "sheet.row", "sheet.gone")
    );
    assert_eq!(
        agreed_cols(&mut world),
        oracle_order(&world, "sheet.col", "sheet.gone_col")
    );
    assert_eq!(shown_by(&mut world, "server").len(), 3);
    // The column the formulas sit in joined the one left standing.
    assert_eq!(columns_shown_by(&mut world, "server").len(), 2);
    for who in ["alice", "bob", "server"] {
        let variables = variables_of(&mut world, who);
        assert_eq!(variables["refusals"], json!(0), "{who} refused nothing");
        assert_eq!(variables["orphans"], json!(0), "{who} placed every row");
    }
    assert!(world.host_errors().is_empty());

    for who in ["alice", "bob", "server"] {
        let record = world.record_of::<Peer>(who);
        assert!(
            record.len() > 5,
            "{who} took part in the morning: {} steps",
            record.len()
        );
        hold(
            &format!("sheet_peer_{who}"),
            &scenario_of(
                "sheet_peer",
                &about("A morning of a shared sheet", who),
                &record,
                SHEET_PUBLISHED,
            ),
        );
    }
}

/// How long the hub keeps an entry in the afternoon, and how much longer a copy does.
/// The times of the afternoon are chosen against these: nothing the hub's own timer
/// changes is due within half a second of a step that reads it, which the Interpreter,
/// whose scheduler runs on the wall clock, needs of a scenario.
const AFTERNOON_LEASE: presence::Lease = presence::Lease {
    ttl_ms: 2_000,
    slack_ms: 1_000,
};

/// An afternoon of the presence list: two people say they are here, one goes on for a
/// while and one does not, both are let go when their heartbeats run out, and one comes
/// back. The world's time is what moves the hub's expiries, so each copy's record holds
/// the waits as well as the events.
fn presence_afternoon() -> World {
    let names = ["alice", "bob"];
    let mut world = presence::presence_world(3, &names, 20, AFTERNOON_LEASE, true);
    let holds = |world: &World| -> Vec<u32> {
        presence::who_the_hub_holds(world, 2_000)
            .into_keys()
            .collect()
    };
    let (alice, bob) = (1, 2);

    presence::beat(&mut world, "alice");
    presence::beat(&mut world, "bob");
    world.advance_ms(700);
    presence::beat(&mut world, "alice");
    world.advance_ms(700);
    presence::beat(&mut world, "alice");
    world.advance_ms(100);
    assert_eq!(holds(&world), [alice, bob]);

    // Bob's heartbeat reached the hub at 20 ms, so he is let go at 2_020.
    world.advance_ms(600);
    assert_eq!(holds(&world), [alice]);
    // And alice, whose last reached it at 1_420, at 3_420.
    world.advance_ms(1_400);
    assert!(holds(&world).is_empty());

    world.advance_ms(500);
    presence::beat(&mut world, "bob");
    world.advance_ms(500);
    assert_eq!(holds(&world), [bob]);
    world.advance_ms(3_100);
    assert!(holds(&world).is_empty());
    world
}

#[test]
fn an_afternoon_of_the_presence_list_is_three_scenarios() {
    let mut world = presence_afternoon();
    // What the hub holds is what its heartbeats and the clock say, at the end as in the
    // middle (above), and every copy agrees that nobody is left.
    for who in ["alice", "bob", "server"] {
        assert!(presence::entries_of(&mut world, who).is_empty(), "{who}");
        let variables = presence::variables_of(&mut world, who);
        assert_eq!(variables["refusals"], json!(0), "{who} refused nothing");
        assert_eq!(variables["lost"], json!(0), "{who} lost nothing");
    }
    assert_eq!(
        presence::renewals_at_the_hub(&world, 2_000).len(),
        5,
        "every heartbeat the hub was given renewed an entry"
    );
    assert!(world.host_errors().is_empty());

    for who in ["alice", "bob", "server"] {
        let record = world.record_of::<presence::Presence>(who);
        assert!(
            record.len() > 5,
            "{who} took part in the afternoon: {} steps",
            record.len()
        );
        if who == "server" {
            // What the hub's own timers change is half a second or more from every
            // step. A copy's timers are all for entries that the hub's word has ended
            // by the time they are due, so they change nothing wherever they fall.
            for taken in &record {
                assert!(
                    taken.next_due_ms.unwrap_or(u64::MAX) >= 500,
                    "a step of the hub stands within half a second of a timer: {taken:?}"
                );
            }
        }
        hold(
            &format!("presence_peer_{who}"),
            &scenario_of(
                "presence_peer",
                &about("An afternoon of a presence list", who),
                &record,
                PRESENCE_PUBLISHED,
            ),
        );
    }
}
