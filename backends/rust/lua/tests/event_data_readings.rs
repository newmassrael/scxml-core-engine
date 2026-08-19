// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-B-2-8-1: which reading an arriving payload gets.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/event_data_readings.json`, one payload per case with the
// sentence of the clause that decides it, and the two C++ engines, the two
// Kotlin engines, the Go binding and the Python binding read the same file.
// A per-backend copy drifts toward the backend that reads it, which is the
// blindness that let eight engines give four different answers to one clause.
//
// This binding's answer, measured 2026-08-19 before the repair: a payload that
// opens with `<` and is not a well-formed document arrived as nil. The leading
// `<` sent it down the DOM rung and the failed parse ended there, so the
// clause's closing sentence — "Otherwise, the Processor MUST treat the content
// as a space-normalized string literal" — was never reached.
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

struct Case {
    payload: String,
    lua: String,
    clause: String,
    expect: serde_json::Value,
}

fn cases() -> Vec<Case> {
    let path = repo_root().join("tests/ecmascript/event_data_readings.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read the shared table at {}: {e}", path.display()));
    let table: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    table
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("the table has a `cases` array")
        .iter()
        .map(|case| Case {
            payload: case
                .get("payload")
                .and_then(|p| p.as_str())
                .unwrap_or_else(|| panic!("every case carries a payload: {case}"))
                .to_string(),
            lua: case
                .get("lua")
                .and_then(|l| l.as_str())
                .expect("every case has a `lua` spelling")
                .to_string(),
            clause: case
                .get("clause")
                .and_then(|c| c.as_str())
                .expect("every case names a clause")
                .to_string(),
            expect: case
                .get("expect")
                .expect("every case has an expectation")
                .clone(),
        })
        .collect()
}

/// Whether the engine's answer is the one the table names.
///
/// A whole number may arrive as `Int` or `Double` — Lua 5.4 has both, and
/// which one holds a decoded JSON number is not part of the clause.
fn matches(actual: &ScriptValue, expect: &serde_json::Value) -> bool {
    if let Some(number) = expect.get("number").and_then(|n| n.as_f64()) {
        return match actual {
            ScriptValue::Int(value) => *value as f64 == number,
            ScriptValue::Double(value) => *value == number,
            _ => false,
        };
    }
    if let Some(text) = expect.get("string").and_then(|s| s.as_str()) {
        return matches!(actual, ScriptValue::String(value) if value == text);
    }
    if let Some(boolean) = expect.get("bool").and_then(|b| b.as_bool()) {
        return matches!(actual, ScriptValue::Bool(value) if *value == boolean);
    }
    if expect.get("empty").is_some() {
        return matches!(actual, ScriptValue::Null | ScriptValue::Undefined);
    }
    false
}

#[test]
fn the_binding_reads_every_payload_the_clause_names() {
    let table = cases();
    // A floor, not an equality: adding a case must not have to touch this
    // number, but a table that stopped being read must not pass either.
    assert!(
        table.len() >= 8,
        "the shared reading table produced only {} case(s), so this is not \
         measuring the surface it claims to",
        table.len()
    );

    let engine = LuaEngine::new();
    let mut failures = Vec::new();
    for (index, case) in table.iter().enumerate() {
        let session = format!("event_data_reading_{index}");
        engine.create_session(&session);
        engine
            .set_current_event(
                &session,
                SetCurrentEventArgs {
                    event_name: "brief",
                    event_data: &case.payload,
                    event_type: "external",
                    send_id: "",
                    origin: "",
                    origin_type: "",
                    invoke_id: "",
                },
            )
            .expect("set_current_event");
        match engine.evaluate_expression(&session, &case.lua) {
            Ok(answered) if matches(&answered, &case.expect) => {}
            Ok(answered) => failures.push(format!(
                "payload {:?}: [{}] answered {answered:?}, {} says {}",
                case.payload, case.lua, case.clause, case.expect
            )),
            Err(error) => failures.push(format!(
                "payload {:?}: [{}] did not evaluate: {error} ({})",
                case.payload, case.lua, case.clause
            )),
        }
        engine.destroy_session(&session);
    }

    // Every case is reported, not just the first: an engine that drops the
    // fall-through is a different defect from one that runs the payload, and
    // one failure cannot separate them.
    assert!(
        failures.is_empty(),
        "{} of {} readings disagree with §scxml-B-2-8-1:\n{}",
        failures.len(),
        table.len(),
        failures.join("\n")
    );
}

/// The sharper half of the expression case, which the shared table cannot ask
/// because the side effect is spelled in the receiver's own language.
#[test]
fn a_payload_that_is_a_call_leaves_the_session_alone() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    engine
        .execute_script("s", "breached = false")
        .expect("execute_script");

    engine
        .set_current_event(
            "s",
            SetCurrentEventArgs {
                event_name: "brief",
                event_data: "(function() breached = true return 'x' end)()",
                event_type: "external",
                send_id: "",
                origin: "",
                origin_type: "",
                invoke_id: "",
            },
        )
        .expect("set_current_event");

    assert_eq!(
        engine.get_variable("s", "breached").expect("get_variable"),
        ScriptValue::Bool(false),
        "the payload ran: a host, a peer session or an HTTP sender could write \
         this session's globals by naming them in event data"
    );
}
