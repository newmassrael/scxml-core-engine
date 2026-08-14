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

/// JSON's `\uXXXX` escape. Lua 5.4 spells the same thing `\u{XXXX}`, so a
/// payload that is perfectly good JSON produces Lua source that does not
/// compile — and the failure is not reported, it falls through to the
/// whitespace-normalised *string* fallback, which leaves `_event.data` a string
/// instead of a table and every field read on it nil. ASCII-escaping is the
/// standard way to put non-ASCII text through a transport that is not 8-bit
/// clean, so this is not an exotic input.
#[test]
fn a_unicode_escape_still_yields_a_table_not_a_string() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    // Built from the backslash character so this source carries no escape of
    // its own: the payload is the literal text a JSON encoder emits for
    // "A북" when asked to stay ASCII-only.
    let b = '\\';
    let payload = format!("{{\"north_star\": \"{b}u0041{b}ubd81\"}}");
    assert!(
        payload.contains("u0041"),
        "the escape must reach the engine"
    );
    event(&engine, "s", &payload);
    engine
        .execute_script("s", "held = _event.data.north_star")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::String("A북".to_string()),
    );
}

/// `\/` is a legal JSON escape for `/` and is not a Lua escape at all — same
/// failure mode as the case above.
#[test]
fn an_escaped_solidus_still_yields_a_table_not_a_string() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    let b = '\\';
    let payload = format!("{{\"path\": \"a{b}/b\"}}");
    event(&engine, "s", &payload);
    engine
        .execute_script("s", "held = _event.data.path")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::String("a/b".to_string()),
    );
}

/// A JSON array. Lua writes every table with braces, so `[` and `]` have to
/// become `{` and `}` — carried across verbatim they are Lua's *index*
/// syntax, which does not parse in value position.
#[test]
fn a_json_array_arrives_as_an_indexable_table() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    event(&engine, "s", r#"{"codes": [200, 201]}"#);
    engine
        .execute_script("s", "first = _event.data.codes[1]")
        .expect("execute_script");
    engine
        .execute_script("s", "second = _event.data.codes[2]")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "first").expect("get_variable"),
        ScriptValue::Int(200),
    );
    assert_eq!(
        engine.get_variable("s", "second").expect("get_variable"),
        ScriptValue::Int(201),
    );
}

/// A top-level JSON array, which reaches `_event.data` as the whole payload
/// rather than as a field of one.
#[test]
fn a_top_level_json_array_arrives_as_a_table() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    event(&engine, "s", r#"["a", "b"]"#);
    engine
        .execute_script("s", "first = _event.data[1]")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "first").expect("get_variable"),
        ScriptValue::String("a".to_string()),
    );
}

/// The author-facing `JSON.parse` and the `_event.data` payload path are two
/// entry points to one contract, so a document has to mean the same thing
/// through either. It did not: the payload path decodes and the shared
/// `json_builtins.lua` rewrites, and only the rewrite mistranslates JSON's
/// escapes. A script and its own event data disagreeing about one string is
/// the kind of thing nobody looks for.
#[test]
fn json_parse_and_event_data_agree_on_the_same_document() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    let b = '\\';
    let doc = format!("{{\"north_star\": \"{b}u0041{b}ubd81\", \"codes\": [200, 201]}}");

    event(&engine, "s", &doc);
    engine
        .execute_script("s", "from_event = _event.data.north_star")
        .expect("execute_script");
    engine
        .execute_script(
            "s",
            &format!("from_parse = JSON.parse([==[{doc}]==]).north_star"),
        )
        .expect("execute_script");

    let from_event = engine
        .get_variable("s", "from_event")
        .expect("get_variable");
    let from_parse = engine
        .get_variable("s", "from_parse")
        .expect("get_variable");
    assert_eq!(from_event, ScriptValue::String("A북".to_string()));
    assert_eq!(from_parse, from_event, "the two entry points disagree");
}

/// `JSON.parse` on an array, which the rewrite handled by brace substitution
/// and the decoder handles by construction. Pinned so the override cannot
/// regress the one case the shared implementation did get right.
#[test]
fn json_parse_builds_a_one_based_sequence() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    engine
        .execute_script("s", "held = JSON.parse('[200, 201]')[1]")
        .expect("execute_script");

    assert_eq!(
        engine.get_variable("s", "held").expect("get_variable"),
        ScriptValue::Int(200),
    );
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
