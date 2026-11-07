// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.10 and §scxml-B-2-8: which writes to a system variable a script
// is refused.
//
// "The Processor MUST cause any attempt to change the value of a system
// variable to fail and MUST place the error 'error.execution' on the internal
// event queue." A refused write is a script that fails, which is what this
// engine reports and what the Processor turns into `error.execution`.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/system_variable_writes.json`, one write per case with the
// sentence that decides it, and every engine reads the same file. Each case
// runs in a fresh session bound to the table's `session` and `event`: the
// write must fail exactly when the table says it is refused, and the case's
// `after` condition must hold either way, so a write that is reported as
// failed but landed anyway is caught too.
//
// This binding's answer before the repair, read from the code on 2026-11-08:
// `_event` was a plain table and `_sessionid`, `_name` and `_ioprocessors`
// plain globals, so every write in the table went through.
//
// The `lua` spelling is what this backend is asked and all it ever sees: the
// frontend lowered the author's ECMAScript at build time.

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::helpers::io_processors::IoProcessorDescriptor;
use sce_rust_runtime::{IScriptEngine, ScriptError, ScriptValue, SetCurrentEventArgs};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // backends/rust/lua → repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the crate sits three levels under the repository root")
        .to_path_buf()
}

/// A text member of `owner`; an absent member, or one that is not a string,
/// is refused rather than read as blank.
fn text_member(owner: &serde_json::Value, key: &str) -> String {
    owner
        .get(key)
        .unwrap_or_else(|| panic!("`{key}` is missing from {owner}"))
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string in {owner}"))
        .to_string()
}

/// The session every case runs in.
struct Session {
    id: String,
    name: String,
    io_processors: Vec<IoProcessorDescriptor>,
}

/// The current event every case runs with; `data` is the payload text.
struct Event {
    name: String,
    event_type: String,
    send_id: String,
    origin: String,
    origin_type: String,
    invoke_id: String,
    data: String,
}

/// One case of the shared table.
struct Case {
    group: String,
    write: String,
    refused: bool,
    after: String,
    clause: String,
}

/// The shared table read whole, and how many cases it holds.
struct Table {
    session: Session,
    event: Event,
    held: usize,
    cases: Vec<Case>,
}

fn table() -> Table {
    let path = repo_root().join("tests/ecmascript/system_variable_writes.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read the shared table at {}: {e}", path.display()));
    let table: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    let session = table.get("session").expect("the table has a `session`");
    let io_processors = session
        .get("ioprocessors")
        .and_then(|p| p.as_object())
        .expect("the table's `session` has an `ioprocessors` object")
        .iter()
        .map(|(name, location)| IoProcessorDescriptor {
            name: name.clone(),
            location: location
                .as_str()
                .unwrap_or_else(|| panic!("the location of `{name}` is not a string"))
                .to_string(),
        })
        .collect();
    let session = Session {
        id: text_member(session, "sessionid"),
        name: text_member(session, "name"),
        io_processors,
    };

    let event = table.get("event").expect("the table has an `event`");
    let event = Event {
        name: text_member(event, "name"),
        event_type: text_member(event, "type"),
        send_id: text_member(event, "sendid"),
        origin: text_member(event, "origin"),
        origin_type: text_member(event, "origintype"),
        invoke_id: text_member(event, "invokeid"),
        data: text_member(event, "data"),
    };

    let held = table
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("the table has a `cases` array");
    let cases = held
        .iter()
        .map(|case| Case {
            group: text_member(case, "group"),
            write: text_member(case, "write_lua"),
            // An expectation this reader cannot read is refused, not
            // skipped: a skipped case is a case that passes.
            refused: case
                .get("refused")
                .and_then(|r| r.as_bool())
                .unwrap_or_else(|| panic!("no readable `refused` in {case}")),
            after: text_member(case, "after_lua"),
            clause: text_member(case, "clause"),
        })
        .collect();
    Table {
        session,
        event,
        held: held.len(),
        cases,
    }
}

#[test]
fn a_write_to_a_system_variable_fails() {
    let Table {
        session,
        event,
        held,
        cases,
    } = table();
    // A floor, not an equality: adding a case must not have to touch this
    // number, but a table that stopped being read must not pass either.
    assert!(
        held >= 18,
        "the shared system-variable table holds only {held} case(s), so this \
         is not measuring the surface it claims to"
    );

    let engine = LuaEngine::new();
    let mut failures = Vec::new();
    let mut read = 0usize;
    for (index, case) in cases.iter().enumerate() {
        // A fresh session per case: a write that landed must not decide the
        // cases after it.
        engine.create_session(&session.id);
        engine
            .setup_system_variables(&session.id, &session.name, &session.io_processors)
            .expect("setup_system_variables");
        engine
            .set_current_event(
                &session.id,
                SetCurrentEventArgs {
                    event_name: &event.name,
                    event_data: &event.data,
                    event_type: &event.event_type,
                    send_id: &event.send_id,
                    origin: &event.origin,
                    origin_type: &event.origin_type,
                    invoke_id: &event.invoke_id,
                },
            )
            .expect("set_current_event");

        match (
            engine.execute_script(&session.id, &case.write),
            case.refused,
        ) {
            (Ok(_), false) => {}
            // A write that does not compile was never attempted, so its
            // failure is not a refusal.
            (Err(ScriptError::SyntaxError(error)), _) => failures.push(format!(
                "case {index} ({}): [{}] did not compile: {error}",
                case.group, case.write
            )),
            (Err(_), true) => {}
            (Ok(_), true) => failures.push(format!(
                "case {index} ({}): [{}] went through, {} says it is refused",
                case.group, case.write, case.clause
            )),
            (Err(error), false) => failures.push(format!(
                "case {index} ({}): [{}] was refused ({error}), {} says it is not",
                case.group, case.write, case.clause
            )),
        }
        match engine.evaluate_expression(&session.id, &case.after) {
            Ok(ScriptValue::Bool(true)) => {}
            Ok(answered) => failures.push(format!(
                "case {index} ({}): after [{}], [{}] answered {answered:?} ({})",
                case.group, case.write, case.after, case.clause
            )),
            Err(error) => failures.push(format!(
                "case {index} ({}): after [{}], [{}] did not evaluate: {error} ({})",
                case.group, case.write, case.after, case.clause
            )),
        }
        engine.destroy_session(&session.id);
        read += 1;
    }

    assert_eq!(
        read, held,
        "read {read} of the {held} case(s) the table holds"
    );
    // Every case is reported, not just the first: a binding that guards the
    // variables but not `_event`'s fields fails different cases from one that
    // guards nothing.
    assert!(
        failures.is_empty(),
        "{} of {} system-variable writes disagree with §scxml-5.10:\n{}",
        failures.len(),
        held,
        failures.join("\n")
    );
}
