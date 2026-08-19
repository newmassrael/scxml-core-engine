// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: `_event.data` is read, not run.
//
// The clause gives the payload three readings and no fourth: XML becomes a
// DOM, JSON becomes the value, and anything else becomes a space-normalized
// string. This engine had a rung above all three — `load("return " .. payload)`
// — so the payload was Lua source until it failed to be. Measured 2026-08-17,
// that decided:
//
//   * `2 + 3` from a host arrived as the number 5 here and as the string
//     "2 + 3" on the cpp engine and Rhino, which read the clause. One
//     payload, two answers, and no test asked either of them.
//   * a payload that is a function call RAN, in the session's own globals.
//
// It survived because it was load-bearing in the other direction: `<send>`
// shipped `_scxml_params({...})` — Lua source — so the rung was the
// deserializer for every param a document sent. The sender now ships JSON
// (§scxml-B-2-9), which is what cpp always shipped, so the reading below is
// the clause's and nothing has to run to reach it.
//
// The sibling that proves the same thing end to end, through a document
// rather than through this API, is `integration_resources/
// event_data_arrives_as_sent` in all seven channels.

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::{IScriptEngine, ScriptValue, SetCurrentEventArgs};

fn deliver(engine: &LuaEngine, session: &str, data: &str) {
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

fn read(engine: &LuaEngine, session: &str, expr: &str) -> ScriptValue {
    engine
        .execute_script(session, &format!("held = {expr}"))
        .expect("execute_script");
    engine.get_variable(session, "held").expect("get_variable")
}

/// A payload that happens to be a valid expression is still text. This is the
/// case the eval rung got wrong in the direction nobody notices: it produced a
/// plausible value, so the only symptom was that two backends disagreed.
#[test]
fn an_expression_shaped_payload_arrives_as_the_text_it_is() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    deliver(&engine, "s", "2 + 3");

    assert_eq!(
        read(&engine, "s", "_event.data"),
        ScriptValue::String("2 + 3".to_string()),
        "the payload was evaluated instead of read; §scxml-B-2-8-1 makes text \
         that is neither XML nor JSON a space-normalized string"
    );
}

/// The same claim where it bites: a payload is the one field a document takes
/// from outside itself, so running it hands the sender the session.
#[test]
fn a_payload_that_is_a_call_does_not_execute() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    engine
        .execute_script("s", "breached = false")
        .expect("execute_script");

    deliver(
        &engine,
        "s",
        "(function() breached = true return 'x' end)()",
    );

    assert_eq!(
        engine.get_variable("s", "breached").expect("get_variable"),
        ScriptValue::Bool(false),
        "the payload ran: a host, a peer session or an HTTP sender could write \
         this session's globals by naming them in event data"
    );
}

/// Whitespace normalization is the rung the text lands on, and it is a
/// *reading* rather than a passthrough — pinned so "not evaluated" cannot be
/// satisfied by handing the bytes over untouched.
#[test]
fn text_that_is_not_a_value_is_space_normalized() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    deliver(&engine, "s", "hold   the\n\tline");

    assert_eq!(
        read(&engine, "s", "_event.data"),
        ScriptValue::String("hold the line".to_string()),
    );
}

/// The reading that must NOT change: JSON is still decoded into a value with
/// its types. This is the rung `<send>` now writes to, so a regression here
/// silently empties every document's `_event.data`.
#[test]
fn a_json_payload_is_still_decoded_with_its_types() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    deliver(&engine, "s", r#"{"milestone":"refined","turns":2}"#);

    assert_eq!(
        read(&engine, "s", "_event.data.milestone"),
        ScriptValue::String("refined".to_string()),
    );
    assert_eq!(
        read(&engine, "s", "_event.data.turns"),
        ScriptValue::Int(2),
        "a number that arrived as a string would read false against `=== 2`"
    );
}

/// A repeated `<param>` name arrives as an Array (W3C test178), which is the
/// shape the sending helper produces — asked here so the two halves of the
/// wire are pinned in one place.
#[test]
fn a_repeated_name_arrives_as_an_array() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    deliver(&engine, "s", r#"{"n":[1,2]}"#);

    assert_eq!(read(&engine, "s", "_event.data.n[1]"), ScriptValue::Int(1));
    assert_eq!(read(&engine, "s", "_event.data.n[2]"), ScriptValue::Int(2));
}

/// ECMA-262 15.12.2: the author-facing `JSON.parse` refuses what is not JSON.
///
/// The same claim one layer down. ⚠ On THIS engine `JSON.parse` is a native
/// override installed over the shared `json_builtins.lua` — measured, by a
/// mutation to that file that these cases did not notice — so what is pinned
/// here is the Rust decoder. The shared Lua reader is asked the same question
/// by `sce-build/tests/shared_json_reader_is_a_parser.rs`, which loads it raw.
/// Saying which is which is the point: the shared file used to rewrite its
/// argument into Lua source and `load` it, so `2 + 3` "parsed" to 5, and the
/// C11 backend decodes an arriving payload through exactly that.
#[test]
fn json_parse_refuses_text_that_is_not_json() {
    let engine = LuaEngine::new();
    engine.create_session("s");

    assert_eq!(
        read(&engine, "s", "JSON.parse('2 + 3') == nil"),
        ScriptValue::Bool(true),
        "`2 + 3` is a Lua expression, not a JSON document"
    );
    assert_eq!(
        read(&engine, "s", "tostring(JSON.parse('{\"a\":1} trailing'))"),
        ScriptValue::String("nil".to_string()),
        "RFC 8259 is one value and nothing after it"
    );
    assert_eq!(
        read(&engine, "s", "JSON.parse('hold the line') == nil"),
        ScriptValue::Bool(true),
    );
}

/// The other direction, so "refuses everything" cannot pass as correct.
#[test]
fn json_parse_still_reads_json() {
    let engine = LuaEngine::new();
    engine.create_session("s");

    assert_eq!(
        read(&engine, "s", "JSON.parse('{\"a\":[1,2],\"b\":\"x\"}').a[2]"),
        ScriptValue::Int(2),
    );
    assert_eq!(
        read(&engine, "s", "JSON.parse('\"plain\"')"),
        ScriptValue::String("plain".to_string()),
        "a JSON document may be a bare string (W3C test294 sends one)"
    );
    // RFC 8259 §6's exponent and fraction forms, which a stricter number
    // scanner is the easiest thing to drop. The first arrives as an integer
    // because the Lua bridge narrows an integral float; the second cannot,
    // so between them they pin both halves of the grammar.
    assert_eq!(
        read(&engine, "s", "JSON.parse('-1.5e2')"),
        ScriptValue::Int(-150),
    );
    assert_eq!(
        read(&engine, "s", "JSON.parse('1.25')"),
        ScriptValue::Double(1.25),
    );
    assert_eq!(
        read(&engine, "s", "JSON.parse('true')"),
        ScriptValue::Bool(true),
    );
}

/// §scxml-B-2-8-1's first rung, unaffected: XML is still read into a DOM by
/// the receiver (W3C test561).
#[test]
fn an_xml_payload_is_still_read_into_a_dom() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    deliver(
        &engine,
        "s",
        "<books><book title=\"x\"><title>t</title></book></books>",
    );

    assert_eq!(
        read(
            &engine,
            "s",
            "_event.data:getElementsByTagName('book')[1]:getAttribute('title')"
        ),
        ScriptValue::String("x".to_string()),
        "the payload reached the datamodel as text instead of a document"
    );
}

/// The rung above is conditioned on the content BEING XML, and the clause
/// spells out what happens when it is not: "if the Processor can interpret the
/// content as a valid XML document, it MUST create the corresponding DOM
/// structure... Otherwise, the Processor MUST treat the content as a
/// space-normalized string literal". A leading `<` is a guess about which
/// reading applies, not the reading itself, and a guess that turns out wrong
/// has to fall through rather than answer nil.
///
/// This is not a corner: every `error.*` message this engine raises names the
/// SCXML construct that failed, so every one of them opens with `<`. The
/// repository filled that field in at 192 sites and the documents receiving it
/// read nil, on this backend and on two others.
#[test]
fn text_that_merely_opens_like_xml_is_still_read() {
    let engine = LuaEngine::new();
    engine.create_session("s");
    deliver(&engine, "s", "<assign>  to\n\tdetail failed");

    assert_eq!(
        read(&engine, "s", "_event.data"),
        ScriptValue::String("<assign> to detail failed".to_string()),
        "`<assign>  to\\n\\tdetail failed` is not a valid XML document, so \
         §scxml-B-2-8-1's final sentence applies and the reading is the \
         space-normalized string. Answering nil drops the payload entirely"
    );
}
