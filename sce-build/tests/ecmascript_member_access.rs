// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Reaching a member — by either spelling, on the one receiver whose
// members the specification names.
//
// `ecmascript_property_calls` closed the call on a name that holds a
// value: `t.length()`, `Math.PI()`, `_sessionid()`. It closed them one
// syntax at a time, and two gaps were left in the shape of that syntax.
//
// The first is the receiver the specification describes and the rules
// did not. W3C SCXML 5.10.1 fills seven fields of `_event` with values,
// so `cond="_event.name() == 'go'"` calls a string — and it generated
// cleanly on all six backends, `check --lint` answered exit 0 with no
// record on any stream, and the machine died evaluating the guard.
//
// The second is that every one of those rules was written about `.name`
// and none of them about `['name']`, which ECMA-262 11.2.1 defines as
// the same operation. `t['length']` reached a nil where `t.length` is
// measured, `Math['PI']` indexed a table this datamodel does not
// install, and `_event['name']()` generated cleanly a token away from a
// refusal — so the repair for one spelling was reachable by writing the
// other.
//
// The counterweight this file carries throughout is W3C test178, which
// reads `_event.raw`: a field the specification never mentions, which
// this repository generates and runs as a registered fixture. Refusing
// what 5.10.1 does not name would reject it. Membership is a floor.
//
// Claims about what a lowering *means* are made by running it on
// `sce-rust-lua`, the engine a generated Rust machine uses, rather than
// by matching emitted text.

use sce_build::ecmascript::builtins::{system_value_fields, EVENT_FIELDS, VALUE_GLOBALS};
use sce_build::ecmascript::{to_lua_value, DocumentScope, ExprError};
use sce_rust_lua::LuaEngine;
use sce_rust_runtime::scripting::{IScriptEngine, ScriptValue};
use std::collections::BTreeSet;

/// The receivers these probes hang a member on.
///
/// `handlers` is the counterweight: an author's own object, whose fields
/// carry the ordinary words `name` and `data` and whose method call has
/// to survive every rule this file asserts.
fn probes() -> DocumentScope {
    DocumentScope::declaring(["arr", "handlers", "t"])
}

/// The refusal `source` produced, or a panic naming what came back
/// instead — a probe that lowered cleanly is the defect this file exists
/// for, so it must not read as a pass.
fn refusal(source: &str) -> ExprError {
    match to_lua_value(source, &probes()) {
        Err(err) => err,
        Ok(lua) => panic!("{source} was accepted and lowered to {lua}"),
    }
}

/// The name a `PropertyNotCallable` refusal offers as the repair.
fn property_refused(source: &str) -> String {
    match refusal(source) {
        ExprError::PropertyNotCallable { name, .. } => name,
        other => panic!("{source} was refused, but as {other:?} rather than a property call"),
    }
}

/// What `source` lowers to, or a panic carrying the refusal.
fn lowered(source: &str) -> String {
    to_lua_value(source, &probes()).unwrap_or_else(|err| panic!("{source} was refused: {err}"))
}

// ── The fields W3C SCXML 5.10.1 names ────────────────────────────

/// Every field the clause obliges an event to carry is refused when it
/// is called.
///
/// The clause types six of them outright — `name` and `type` are
/// character strings, `origin` is a URI — and the seventh, `data`, is
/// whatever the sender included, which crosses into the datamodel as a
/// `ScriptValue`. `no_event_field_can_hold_a_function` below is what
/// makes that seventh a fact rather than an assumption.
///
/// The names are written out from 5.10.1 rather than read from the
/// frontend, and the two are then compared: a list that lost a field
/// would otherwise shrink this loop and pass.
#[test]
fn every_field_the_specification_names_is_refused_when_called() {
    // W3C SCXML 5.10.1, in the clause's order.
    let clause = [
        "name",
        "type",
        "sendid",
        "origin",
        "origintype",
        "invokeid",
        "data",
    ];
    assert_eq!(
        clause.iter().copied().collect::<BTreeSet<_>>(),
        EVENT_FIELDS.iter().copied().collect::<BTreeSet<_>>(),
        "the fields this test runs and the fields the frontend lists are \
         not one set"
    );
    for field in clause {
        let source = format!("_event.{field}()");
        assert_eq!(
            property_refused(&source),
            format!("_event.{field}"),
            "{source} was not answered as a property call"
        );
    }
}

/// The value a field holds cannot be a function, whatever raised the
/// event.
///
/// This is the whole justification for refusing `_event.data()`, and it
/// is a property of the boundary rather than of the sender: event data
/// reaches a datamodel as `ScriptValue`, and the match below is
/// exhaustive, so a variant added for a callable value stops this file
/// compiling instead of quietly making the refusal false.
#[test]
fn no_event_field_can_hold_a_function() {
    /// What ECMA-262 11.4.3 answers for a value that crossed the
    /// boundary. The match is exhaustive on purpose: a variant added for
    /// something callable stops this file compiling, which is the only
    /// way a claim about a union stays true as the union grows.
    fn ecmascript_typeof(value: &ScriptValue) -> &'static str {
        match value {
            // `typeof null` is "object" — the language's own quirk, kept
            // here so the mapping is ECMA-262's rather than convenient.
            ScriptValue::Null => "object",
            ScriptValue::Undefined => "undefined",
            ScriptValue::Bool(_) => "boolean",
            ScriptValue::Int(_) | ScriptValue::Double(_) => "number",
            ScriptValue::String(_) => "string",
            ScriptValue::Array(_) | ScriptValue::Object(_) | ScriptValue::Dom(_) => "object",
        }
    }
    for value in [
        ScriptValue::Null,
        ScriptValue::Undefined,
        ScriptValue::Bool(true),
        ScriptValue::Int(1),
        ScriptValue::Double(1.5),
        ScriptValue::String("s".into()),
        ScriptValue::Array(vec![]),
        ScriptValue::Object(Default::default()),
        ScriptValue::Dom("<e/>".into()),
    ] {
        assert_ne!(
            ecmascript_typeof(&value),
            "function",
            "{value:?} can cross into the datamodel as a callable, so \
             refusing _event.data() is not sound"
        );
    }
}

/// A field the specification does not name is left to whoever supplied
/// it — read *and* called.
///
/// W3C test178 is the fixture that makes this more than a preference:
/// its one executable expression is `_event.raw`, and this repository
/// registers it in `tests/w3c/conformance/fixtures.json`, generates it
/// and runs it. A closed `_event` — the shape `Math` has — would refuse
/// a conformance document.
#[test]
fn a_field_the_specification_does_not_name_is_left_to_the_processor() {
    // The expression W3C test178 writes, verbatim.
    assert_eq!(lowered("_event.raw"), "_event.raw");
    // And the call form: outside the clause's list, so it is an ordinary
    // field call, exactly as an author's own object gets.
    assert_eq!(lowered("_event.raw()"), "_event.raw()");
}

/// The author's shape underneath `data` stays open one level down.
///
/// `_event.data` is refused as a call and its *members* are the sender's
/// own, so a method the sender put there is still a method. A rule that
/// reached one node further would take the payload with it.
#[test]
fn the_payload_underneath_the_field_is_still_the_authors() {
    for source in [
        "_event.data.retry()",
        "_event.data.items.push(1)",
        "_event.data.name()",
        "_event.data.type()",
    ] {
        to_lua_value(source, &probes())
            .unwrap_or_else(|err| panic!("{source} is the sender's own and was refused: {err}"));
    }
}

/// An author's object may hold a function under any of those seven
/// words, and calling it is legal.
///
/// The rule consults the receiver for exactly this reason: `name`,
/// `type` and `data` are ordinary English, and a rule decided from the
/// property alone — the shape `.length` needs — would refuse a document
/// that never mentioned an event.
#[test]
fn the_same_words_on_an_authors_object_are_still_callable() {
    for &field in EVENT_FIELDS {
        let source = format!("handlers.{field}()");
        to_lua_value(&source, &probes())
            .unwrap_or_else(|err| panic!("{source} is the author's own and was refused: {err}"));
    }
}

/// Every field still reads, and reads the value the event carries.
///
/// The refusal is one token from the read, so a rule that reached too
/// far would take `_event.name` — the expression 5.10.1 exists for —
/// with it. Run rather than matched: a read that lowered to the wrong
/// Lua would satisfy a text assertion.
#[test]
fn every_field_the_specification_names_still_reads() {
    let engine = engine_with(&[(
        "_event",
        "{ name = 'go', type = 'external', sendid = 's1', origin = 'o', \
          origintype = 'ot', invokeid = 'i1', data = { items = { 1, 2 } }, \
          raw = 'POST / HTTP/1.1' }",
    )]);
    for (source, expected) in [
        ("_event.name", "go"),
        ("_event.type", "external"),
        ("_event.sendid", "s1"),
        ("_event.origin", "o"),
        ("_event.origintype", "ot"),
        ("_event.invokeid", "i1"),
        ("_event.raw", "POST / HTTP/1.1"),
    ] {
        assert_eq!(
            evaluate_string(&engine, source),
            expected,
            "{source} did not read what the event carries"
        );
    }
    assert_eq!(evaluate_number(&engine, "_event.data.items.length"), 2.0);
}

/// `_event` is the only system variable with a field list, and the list
/// is sorted and free of duplicates.
///
/// The other four hold a string, a platform's own root, or a set keyed
/// by whichever Event I/O Processors a deployment supports — none of
/// them a set a producer can state as a fact. Pinning the emptiness is
/// what keeps a later reader from filling one in by analogy.
#[test]
fn only_the_variable_the_clause_enumerates_has_a_field_list() {
    for &global in VALUE_GLOBALS {
        let fields = system_value_fields(global);
        if global == "_event" {
            assert_eq!(fields, EVENT_FIELDS);
        } else {
            assert!(
                fields.is_empty(),
                "{global} was given a field list the specification does not enumerate: {fields:?}"
            );
        }
    }
    let mut sorted = EVENT_FIELDS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted,
        EVENT_FIELDS.to_vec(),
        "EVENT_FIELDS is not sorted or has a duplicate"
    );
}

// ── The two spellings of a member ────────────────────────────────

/// A member reached with a literal key is the member reached with a dot.
///
/// ECMA-262 11.2.1 defines the dot form as the bracket form with a
/// string literal, so every rule this datamodel states about a property
/// has to be reachable both ways. Before the parser folded the two, each
/// pair below disagreed: the left side was measured, lowered or refused
/// and the right side became a field access that reached a nil.
///
/// Refusals are compared by their rendered message rather than by
/// variant, so a pair that refuses for two *different* reasons still
/// fails here.
#[test]
fn the_two_spellings_of_a_member_are_one_expression() {
    let pairs = [
        ("t.length", "t['length']"),
        ("arr.length", "arr[\"length\"]"),
        ("_event.data.items.length", "_event.data.items['length']"),
        ("Math.PI", "Math['PI']"),
        ("Math.abs(1)", "Math['abs'](1)"),
        ("Math.tanh(1)", "Math['tanh'](1)"),
        ("JSON.stringify(arr)", "JSON['stringify'](arr)"),
        ("JSON.serialize(arr)", "JSON['serialize'](arr)"),
        ("Object.keys(handlers)", "Object['keys'](handlers)"),
        ("t.length()", "t['length']()"),
        ("_event.name", "_event['name']"),
        ("_event.name()", "_event['name']()"),
        ("_event.raw", "_event['raw']"),
        ("arr.map(handlers.retry)", "arr['map'](handlers.retry)"),
        ("handlers.retry()", "handlers['retry']()"),
        ("t.charAt(0)", "t['charAt'](0)"),
        ("handlers.count", "handlers['count']"),
    ];
    // A comparison gate passes on an empty set, and this one covers each
    // rule the fold has to carry — a property, a namespace member of
    // either kind, an unknown member, an event field, a platform field,
    // an unimplemented method and an author's own. A set that lost them
    // would compare less and still read as a pass.
    assert!(
        pairs.len() >= 17,
        "the spellings are compared over only {} pair(s)",
        pairs.len()
    );
    for (dotted, bracketed) in pairs {
        let one = to_lua_value(dotted, &probes()).map_err(|err| err.to_string());
        let other = to_lua_value(bracketed, &probes()).map_err(|err| err.to_string());
        assert_eq!(
            one, other,
            "{dotted} and {bracketed} are one operation in ECMA-262 11.2.1 \
             and this datamodel answered them differently"
        );
    }
}

/// A refusal is reached through the literal key too, and names the same
/// repair.
///
/// Stated without reference to the other spelling, because the pairing
/// test above cannot state it: a comparison rewritten to compare one
/// side with itself passes, and then the only assertion that a *rule*
/// survives the fold would be gone. Here the expected answers are
/// written out.
#[test]
fn a_refusal_is_reached_through_the_literal_key() {
    assert_eq!(property_refused("t['length']()"), ".length");
    assert_eq!(property_refused("_event['name']()"), "_event.name");
    assert_eq!(property_refused("Math['PI']()"), "Math.PI");
    match refusal("arr['map'](handlers.retry)") {
        ExprError::UnsupportedBuiltin { name, .. } => assert_eq!(name, ".map()"),
        other => panic!("arr['map'](...) was refused as {other:?}"),
    }
    match refusal("JSON['serialize'](arr)") {
        ExprError::UnsupportedBuiltin { name, .. } => assert_eq!(name, "JSON.serialize"),
        other => panic!("JSON['serialize'](...) was refused as {other:?}"),
    }
}

/// A literal key this datamodel has no rule for is still an ordinary
/// lookup, and the key is still encoded the way it was.
///
/// The folding is a change of *shape*, and it must not become a change
/// of meaning: a key with a space in it, or one that spells a Lua
/// keyword, cannot be written after a dot at all and stays a bracket
/// index through the same encoder. Where the key is a name Lua can spell,
/// the emitter writes Lua's own dot form — `t.k` and `t["k"]` are one
/// lookup in Lua as they are in ECMAScript, which is what
/// `the_bracket_spelling_measures_the_same_value` runs.
#[test]
fn a_key_this_datamodel_has_no_rule_for_stays_an_ordinary_lookup() {
    for (source, expected) in [
        ("handlers['some key']", "handlers[\"some key\"]"),
        ("handlers['end']", "handlers[\"end\"]"),
        ("handlers['a\"b']", "handlers[\"a\\\"b\"]"),
        // The shape the W3C corpus writes for an Event I/O Processor —
        // `_ioprocessors['scxml']['location']` in tests 500, 501 and 569.
        (
            "_ioprocessors['scxml']['location']",
            "_ioprocessors.scxml.location",
        ),
        ("arr[0]", "arr[1]"),
        ("handlers.count", "handlers.count"),
    ] {
        assert_eq!(lowered(source), expected, "{source} lowered unexpectedly");
    }
}

/// A key that is not a literal is still decided at runtime.
///
/// Whether `a[i]` addresses an array element or an object property
/// depends on what `a` and `i` hold, so the folding must not reach a
/// computed key: `_scxml_index` is the only thing that can answer one.
#[test]
fn a_computed_key_is_still_left_to_the_runtime() {
    assert_eq!(lowered("arr[t]"), "_scxml_index(arr, t)");
    assert_eq!(
        lowered("arr[handlers.count]"),
        "_scxml_index(arr, handlers.count)"
    );
}

/// The bracket spelling measures what the dot spelling measures.
///
/// The pairing test above proves the two lower alike; this one proves
/// what they lower *to* is right, on the engine that runs it. Before the
/// fold, `t['length']` evaluated to nil — a value ECMAScript says is 5.
#[test]
fn the_bracket_spelling_measures_the_same_value() {
    let engine = engine_with(&[("t", "'hello'"), ("arr", "{'a', 'b', 'c'}")]);
    assert_eq!(evaluate_number(&engine, "t['length']"), 5.0);
    assert_eq!(evaluate_number(&engine, "arr['length']"), 3.0);
    assert_eq!(evaluate_number(&engine, "Math['PI']"), std::f64::consts::PI);
}

// ── Running the lowered expression ───────────────────────────────

/// A session on the engine a generated Rust machine runs, with the named
/// variables already assigned.
fn engine_with(variables: &[(&str, &str)]) -> (LuaEngine, String) {
    let engine = LuaEngine::new();
    assert!(engine.initialize(), "the Lua engine must start");
    let session = "member_access".to_string();
    engine.create_session(&session);
    for (name, lua) in variables {
        engine
            .execute_script(&session, &format!("{name} = {lua}"))
            .unwrap_or_else(|err| panic!("could not establish {name}: {err}"));
    }
    (engine, session)
}

/// The value `source` evaluates to once lowered — the production path,
/// not a reimplementation of it.
fn evaluate((engine, session): &(LuaEngine, String), source: &str) -> ScriptValue {
    let lua = lowered(source);
    engine
        .evaluate_expression(session, &lua)
        .unwrap_or_else(|err| panic!("{source} lowered to {lua} and did not evaluate: {err}"))
}

fn evaluate_string(engine: &(LuaEngine, String), source: &str) -> String {
    match evaluate(engine, source) {
        ScriptValue::String(s) => s,
        other => panic!("{source} evaluated to {other:?} rather than a string"),
    }
}

fn evaluate_number(engine: &(LuaEngine, String), source: &str) -> f64 {
    match evaluate(engine, source) {
        // Lua's `#` yields an integer and its `math` library a float, so
        // both spellings of a number are the same answer here.
        ScriptValue::Int(n) => n as f64,
        ScriptValue::Double(n) => n,
        other => panic!("{source} evaluated to {other:?} rather than a number"),
    }
}
