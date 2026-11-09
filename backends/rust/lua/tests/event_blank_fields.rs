// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.10.1 and §scxml-B-2-8: what an `_event` field the Processor left
// blank reads as.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/event_blank_fields.json`, one event per case with the
// sentence that decides it, and every engine reads the same file, beside its
// reader of `event_data_readings.json`. An absent member of a case's `event`
// is a field the Processor left blank, handed to the binding as the empty
// string, which is how the engine interface carries it.
//
// This binding's answer before the repair, read from the code on 2026-11-07:
// `sendid` and `invokeid` were left unset when empty and so read nil, but
// `origin` and `origintype` were bound as the empty string for every event, so
// `_event.origin == nil` answered false for an internal or a platform event.
//
// The `lua` spelling is what this backend is asked and all it ever sees: the
// frontend lowered the author's ECMAScript at build time.

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::{IScriptEngine, ScriptValue, SetCurrentEventArgs};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // backends/rust/lua → repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the crate sits three levels under the repository root")
        .to_path_buf()
}

/// One case of the shared table, with every blank field as the empty string.
struct Case {
    group: String,
    name: String,
    event_type: String,
    send_id: String,
    origin: String,
    origin_type: String,
    invoke_id: String,
    lua: String,
    clause: String,
    expect: bool,
}

/// A text member of a case: `required` refuses an absent one, and a member
/// that is present but not a string is refused either way, rather than being
/// read as blank.
fn text_member(owner: &serde_json::Value, key: &str, required: bool) -> String {
    match owner.get(key) {
        None if required => panic!("a case lacks its `{key}`: {owner}"),
        None => String::new(),
        Some(value) => value
            .as_str()
            .unwrap_or_else(|| panic!("`{key}` is not a string in {owner}"))
            .to_string(),
    }
}

/// The table's cases, and how many the table holds.
fn cases() -> (usize, Vec<Case>) {
    let path = repo_root().join("tests/ecmascript/event_blank_fields.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read the shared table at {}: {e}", path.display()));
    let table: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let held = table
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("the table has a `cases` array");
    let read = held
        .iter()
        .map(|case| {
            let event = case
                .get("event")
                .unwrap_or_else(|| panic!("every case names its event: {case}"));
            Case {
                group: text_member(case, "group", true),
                name: text_member(event, "name", true),
                event_type: text_member(event, "type", true),
                send_id: text_member(event, "sendid", false),
                origin: text_member(event, "origin", false),
                origin_type: text_member(event, "origintype", false),
                invoke_id: text_member(event, "invokeid", false),
                lua: text_member(case, "lua", true),
                clause: text_member(case, "clause", true),
                // An expectation this reader cannot read is refused, not
                // skipped: a skipped case is a case that passes.
                expect: case
                    .get("expect")
                    .and_then(|e| e.get("bool"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or_else(|| panic!("no readable `expect.bool` in {case}")),
            }
        })
        .collect();
    (held.len(), read)
}

#[test]
fn a_blank_event_field_reads_as_nil() {
    let (held, table) = cases();
    // A floor, not an equality: adding a case must not have to touch this
    // number, but a table that stopped being read must not pass either.
    assert!(
        held >= 11,
        "the shared blank-field table holds only {held} case(s), so this is \
         not measuring the surface it claims to"
    );

    let engine = LuaEngine::new();
    let mut failures = Vec::new();
    let mut read = 0usize;
    for (index, case) in table.iter().enumerate() {
        let session = format!("event_blank_field_{index}");
        engine
            .create_session(&session)
            .expect("the session is created");
        engine
            .set_current_event(
                &session,
                SetCurrentEventArgs {
                    event_name: &case.name,
                    event_data: "",
                    event_type: &case.event_type,
                    send_id: &case.send_id,
                    origin: &case.origin,
                    origin_type: &case.origin_type,
                    invoke_id: &case.invoke_id,
                },
            )
            .expect("set_current_event");
        match engine.evaluate_expression(&session, &case.lua) {
            Ok(ScriptValue::Bool(value)) if value == case.expect => {}
            Ok(answered) => failures.push(format!(
                "case {index} ({}): [{}] answered {answered:?}, {} says {}",
                case.group, case.lua, case.clause, case.expect
            )),
            Err(error) => failures.push(format!(
                "case {index} ({}): [{}] did not evaluate: {error} ({})",
                case.group, case.lua, case.clause
            )),
        }
        engine.destroy_session(&session);
        read += 1;
    }

    assert_eq!(
        read, held,
        "read {read} of the {held} case(s) the table holds"
    );
    // Every case is reported, not just the first: a binding that drops
    // `origin` always and one that binds it as "" fail different cases.
    assert!(
        failures.is_empty(),
        "{} of {} blank-field readings disagree with §scxml-B-2-8:\n{}",
        failures.len(),
        held,
        failures.join("\n")
    );
}
