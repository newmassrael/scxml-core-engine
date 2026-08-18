// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-B-2-1 / §scxml-B-2-8-1: XML in the data model is a DOM structure,
// not three method names.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/dom_read_surface.json`, one claim per case with the DOM
// clause that backs it, and the two C++ engines, the three Kotlin engines,
// the Go binding, the Python binding and the frontend read the same file — a
// per-backend copy drifts toward the backend that reads it, which is the
// blindness that let seven bindings disagree with one specification.
// Measured 2026-08-18, every read in it answered nil here: what this binding
// carried was `getElementsByTagName`, `getAttribute` and `getTagName`, which
// are the two names the W3C IRP suite reads plus one.
//
// The `lua` spelling is what this backend is asked, and it is all it ever
// sees: the frontend lowered the author's ECMAScript at build time, so a
// member this binding does not answer is a nil index in Lua rather than an
// error in Rust. `sce-build/tests/dom_read_surface_table.rs` binds the two
// spellings to each other by lowering `source` and asserting it IS `lua`.

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::{IScriptEngine, ScriptValue};
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
    xml: String,
    lua: String,
    clause: String,
    expect: serde_json::Value,
}

fn cases() -> Vec<Case> {
    let path = repo_root().join("tests/ecmascript/dom_read_surface.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read the shared table at {}: {e}", path.display()));
    let table: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let documents = table
        .get("documents")
        .and_then(|d| d.as_object())
        .expect("the table has a `documents` object");
    table
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("the table has a `cases` array")
        .iter()
        .map(|case| {
            let named = case
                .get("document")
                .and_then(|d| d.as_str())
                .unwrap_or_else(|| panic!("every case names a document: {case}"));
            Case {
                xml: documents
                    .get(named)
                    .and_then(|d| d.as_str())
                    .unwrap_or_else(|| panic!("the table lacks document `{named}`"))
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
            }
        })
        .collect()
}

/// Whether the engine's answer is the one the table names.
///
/// A whole number may arrive as `Int` or `Double` — Lua 5.4 has both and
/// which one holds a `nodeType` is not part of the DOM contract — so the
/// numeric arm reads either.
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
fn the_binding_answers_dom_level_1_core() {
    let table = cases();
    // A floor, not an equality: adding a case must not have to touch this
    // number, but a table that stopped being read must not pass either.
    assert!(
        table.len() >= 30,
        "the shared DOM table produced only {} case(s), so this is not \
         measuring the surface it claims to",
        table.len()
    );

    let engine = LuaEngine::new();
    let mut failures = Vec::new();
    for (index, case) in table.iter().enumerate() {
        let session = format!("dom_surface_{index}");
        engine.create_session(&session);
        engine
            .set_variable_as_dom(&session, "var1", &case.xml)
            .expect("the document binds as a DOM");
        match engine.evaluate_expression(&session, &case.lua) {
            Ok(answered) if matches(&answered, &case.expect) => {}
            Ok(answered) => failures.push(format!(
                "[{}] answered {answered:?}, {} says {}",
                case.lua, case.clause, case.expect
            )),
            Err(error) => failures.push(format!(
                "[{}] did not evaluate: {error} ({})",
                case.lua, case.clause
            )),
        }
        engine.destroy_session(&session);
    }

    // Every case is reported, not just the first: a binding that answers the
    // methods and none of the properties is a different defect from one that
    // cannot parse the document, and one failure cannot separate them.
    assert!(
        failures.is_empty(),
        "{} of {} reads disagree with DOM Level 1 Core:\n{}",
        failures.len(),
        table.len(),
        failures.join("\n")
    );
}
