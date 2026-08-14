// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2 / 5.10: an event payload reaches `_event.data` unchanged.
//
// `set_current_event` tries to read the payload as a Lua expression first and
// only falls back to the JSON→Lua rewrite when that fails. A consumer sending
// Lua table syntax therefore never touches the rewrite, and every in-tree
// fixture that carries JSON carries ASCII — so the rewrite's handling of
// multi-byte text had no reader at all. These tests are that reader.
//
// The payload path is the only thing under test: each case also puts the same
// text through a `<data>`-style assignment, so a failure says which of the two
// seams moved rather than just "the string differs".

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::{IScriptEngine, ScriptValue, SetCurrentEventArgs};

fn event(engine: &LuaEngine, session: &str, data: &str) {
    engine
        .set_current_event(
            session,
            SetCurrentEventArgs {
                event_name: "brief",
                event_data: data,
                event_type: "external",
                send_id: "",
                origin: "",
                origin_type: "",
                invoke_id: "",
            },
        )
        .expect("set_current_event");
}

/// Korean prose in a JSON event payload — the shape a caller that did not edit
/// the document uses to fill in a loop's goal.
#[test]
fn json_event_data_carries_a_non_ascii_string_whole() {
    let sent = "북극성 — ship it";
    let engine = LuaEngine::new();
    engine.create_session("s");
    event(&engine, "s", &format!(r#"{{"north_star": "{sent}"}}"#));
    engine
        .execute_script("s", "held = _event.data.north_star")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::String(sent.to_string()),
    );
}

/// The same text through the other seam. Keeping both in one file is what makes
/// a failure attributable: if this one passes and the one above fails, the
/// Lua→Rust conversion in `get_variable` is exonerated and the payload rewrite
/// is not.
#[test]
fn a_non_ascii_string_assigned_directly_is_unaffected() {
    let sent = "북극성 — ship it";
    let engine = LuaEngine::new();
    engine.create_session("s");
    engine
        .execute_script("s", &format!("held = '{sent}'"))
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::String(sent.to_string()),
    );
}

/// A JSON *key* is rewritten to `["key"]`, which is a different branch of the
/// same scan from the value branch above.
#[test]
fn a_non_ascii_json_key_survives_the_rewrite() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    event(&engine, "s", r#"{"북극성": "ok"}"#);
    engine
        .execute_script("s", "held = _event.data['북극성']")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::String("ok".to_string()),
    );
}

/// Nesting puts multi-byte text on both sides of the `:` the scan is looking
/// for, and behind a second level of braces.
#[test]
fn nested_json_keeps_non_ascii_on_both_sides_of_the_colon() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    event(
        &engine,
        "s",
        r#"{"outer": {"목표": "비용 무시하고 가장 옳은 것", "n": 3}}"#,
    );
    engine
        .execute_script("s", "held = _event.data.outer['목표']")
        .expect("execute_script");
    engine
        .execute_script("s", "n = _event.data.outer.n")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::String("비용 무시하고 가장 옳은 것".to_string()),
    );
    assert_eq!(
        engine.get_variable("s", "n").expect("get_variable"),
        ScriptValue::Int(3),
    );
}

/// Names the corruption rather than only detecting it. Widening each byte with
/// `as char` is a Latin-1 decode, and its output is derivable — pinning that
/// exact string stops a different kind of damage from wearing this symptom.
#[test]
fn a_mangled_payload_would_be_a_latin1_widening_and_is_not() {
    let sent = "북극성 — ship it";
    let widened: String = sent.bytes().map(char::from).collect();
    assert_ne!(widened, sent, "the case must be able to tell the two apart");

    let engine = LuaEngine::new();
    engine.create_session("s");
    event(&engine, "s", &format!(r#"{{"north_star": "{sent}"}}"#));
    engine
        .execute_script("s", "held = _event.data.north_star")
        .expect("execute_script");

    let got = engine.get_variable("s", "held").expect("get_variable");
    assert_ne!(
        got,
        ScriptValue::String(widened),
        "payload was widened byte-by-byte into Latin-1"
    );
    assert_eq!(got, ScriptValue::String(sent.to_string()));
}

/// An escape immediately before multi-byte text: the scan consumes the byte
/// after a backslash without looking at it, which is a third site that has to
/// stay on character boundaries.
#[test]
fn an_escape_before_non_ascii_text_does_not_split_it() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    event(&engine, "s", r#"{"q": "\"북\" and \"극\""}"#);
    engine
        .execute_script("s", "held = _event.data.q")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::String("\"북\" and \"극\"".to_string()),
    );
}
